import { icon } from "./icons";

// Minimal DOM helpers: h() builds elements; modal stack with focus trapping.

type Child = Node | string | number | null | undefined | false | Child[];
type Attrs = Record<string, unknown> & { class?: string; style?: string };

export function h<K extends keyof HTMLElementTagNameMap>(tag: K, attrs: Attrs | null = null, ...children: Child[]): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  if (attrs) {
    for (const [k, v] of Object.entries(attrs)) {
      if (v === undefined || v === null || v === false) continue;
      if (k.startsWith("on") && typeof v === "function") el.addEventListener(k.slice(2).toLowerCase(), v as EventListener);
      else if (k === "class") el.className = String(v);
      else if (k === "style") el.setAttribute("style", String(v));
      else if (k in el && typeof v !== "string") (el as any)[k] = v;
      else el.setAttribute(k, v === true ? "" : String(v));
    }
  }
  append(el, children);
  return el;
}

function append(el: Node, children: Child[]) {
  for (const c of children) {
    if (c === null || c === undefined || c === false) continue;
    if (Array.isArray(c)) append(el, c);
    else el.appendChild(typeof c === "string" || typeof c === "number" ? document.createTextNode(String(c)) : c);
  }
}

export function clear(el: Element) {
  while (el.firstChild) el.removeChild(el.firstChild);
}

const FOCUSABLE = 'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea, [tabindex]:not([tabindex="-1"])';

export function focusFirst(root: Element) {
  const el = root.querySelector<HTMLElement>("[data-autofocus]") ?? root.querySelector<HTMLElement>(FOCUSABLE);
  el?.focus({ preventScroll: true });
}

/** Modal stack: the top modal owns focus; Escape pops it; focus returns below. */
export class ModalStack {
  private stack: { el: HTMLElement; onClose?: () => void; returnTo: Element | null }[] = [];

  constructor(private host: HTMLElement) {
    document.addEventListener("keydown", (e) => {
      const top = this.stack[this.stack.length - 1];
      if (!top) return;
      if (e.key === "Escape") {
        e.preventDefault();
        this.pop();
      } else if (e.key === "Tab") {
        const items = Array.from(top.el.querySelectorAll<HTMLElement>(FOCUSABLE));
        if (!items.length) return;
        const first = items[0],
          last = items[items.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    });
  }

  get open(): boolean {
    return this.stack.length > 0;
  }

  has(id: string): boolean {
    return this.stack.some((s) => s.el.dataset.id === id);
  }

  push(id: string, title: string, body: Node, opts: { onClose?: () => void; wide?: boolean; closable?: boolean } = {}) {
    if (this.has(id)) return;
    const closable = opts.closable ?? true;
    const el = h(
      "div",
      { class: "modal-backdrop", "data-id": id, onclick: (e: Event) => closable && e.target === el && this.pop() },
      h(
        "section",
        { class: "modal panel" + (opts.wide ? " wide" : ""), role: "dialog", "aria-modal": "true", "aria-label": title },
        h("header", { class: "modal-head" }, h("h2", null, title), closable ? h("button", { class: "icon-btn", "aria-label": "Close", onclick: () => this.pop() }, icon("close")) : null),
        h("div", { class: "modal-body" }, body),
      ),
    );
    this.stack.push({ el, onClose: opts.onClose, returnTo: document.activeElement });
    this.host.appendChild(el);
    focusFirst(el);
  }

  pop() {
    const top = this.stack.pop();
    if (!top) return;
    top.el.remove();
    top.onClose?.();
    (top.returnTo as HTMLElement | null)?.focus?.({ preventScroll: true });
  }

  close(id: string) {
    while (this.has(id)) {
      const top = this.stack[this.stack.length - 1];
      this.pop();
      if (top.el.dataset.id === id) break;
    }
  }
}

export function toast(text: string, kind: "info" | "error" | "good" = "info", ms = 4200) {
  const host = document.getElementById("toasts");
  if (!host) return;
  const icon = kind === "error" ? "⚠" : kind === "good" ? "✔" : "ℹ";
  const el = h("div", { class: "toast " + kind, role: kind === "error" ? "alert" : "status" }, h("span", { class: "toast-icon", "aria-hidden": "true" }, icon), text);
  host.appendChild(el);
  setTimeout(() => el.classList.add("leaving"), ms - 400);
  setTimeout(() => el.remove(), ms);
}
