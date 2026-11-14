// Guided-tour spotlight: dims the screen around one control, rings it, points
// at it with a tapping hand and explains it in a bubble. It never blocks input:
// the player can use the highlighted control (or anything else) while it shows.
// After a moment the dimming lifts so the river stays readable; the ring stays.

import { h } from "./dom";

export interface SpotTarget {
  /** `data-tut` name; also matches a `.tut-<name>` class. */
  name: string;
  title: string;
  text: string;
}

const DIM_MS = 2600;

export class Spotlight {
  private root = h("div", { class: "spot", "aria-live": "polite" });
  private hole = h("div", { class: "spot-hole" });
  private hand = h("div", { class: "spot-hand", "aria-hidden": "true" }, "👆");
  private tip = h("div", { class: "spot-tip" });
  private target: SpotTarget | null = null;
  private shownAt = 0;
  private raf = 0;

  constructor(private host: HTMLElement) {
    this.root.append(this.hole, this.hand, this.tip);
    host.appendChild(this.root);
    const loop = () => {
      this.place();
      this.raf = requestAnimationFrame(loop);
    };
    this.raf = requestAnimationFrame(loop);
  }

  /** Point at a control (null hides the spotlight). */
  show(t: SpotTarget | null) {
    if (t?.name === this.target?.name) return;
    this.target = t;
    this.shownAt = performance.now();
    this.root.classList.toggle("on", !!t);
    this.root.classList.add("dim");
    if (t) this.tip.replaceChildren(h("strong", null, t.title), h("span", null, t.text));
  }

  private find(name: string): HTMLElement | null {
    const all = this.host.querySelectorAll<HTMLElement>(`[data-tut="${name}"], .tut-${name}`);
    for (const el of all) {
      const r = el.getBoundingClientRect();
      if (r.width > 0 && r.height > 0) return el;
    }
    return null;
  }

  private place() {
    const t = this.target;
    if (!t) return;
    if (performance.now() - this.shownAt > DIM_MS) this.root.classList.remove("dim");
    const el = this.find(t.name);
    this.root.classList.toggle("lost", !el);
    if (!el) return;
    const r = el.getBoundingClientRect();
    const pad = 8;
    Object.assign(this.hole.style, {
      left: `${r.left - pad}px`,
      top: `${r.top - pad}px`,
      width: `${r.width + pad * 2}px`,
      height: `${r.height + pad * 2}px`,
    });
    // The hand and the bubble go on the roomier side: above a low control, below a high one.
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const above = r.top > vh / 2;
    this.hand.textContent = above ? "👇" : "👆";
    this.hand.classList.toggle("down", above);
    Object.assign(this.hand.style, { left: `${r.left + r.width / 2}px`, top: above ? `${r.top - 50}px` : `${r.bottom - 4}px` });
    const tw = Math.min(280, vw - 24);
    const left = Math.max(12, Math.min(vw - tw - 12, r.left + r.width / 2 - tw / 2));
    Object.assign(this.tip.style, {
      width: `${tw}px`,
      left: `${left}px`,
      top: above ? "" : `${r.bottom + 46}px`,
      bottom: above ? `${vh - r.top + 58}px` : "",
    });
  }

  dispose() {
    cancelAnimationFrame(this.raf);
    this.root.remove();
  }
}
