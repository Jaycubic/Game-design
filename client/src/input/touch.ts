// Phone and tablet controls for a run: thumb buttons plus swipes on the river.
// Left thumb steers (◀ ▶), right thumb hops and cuts, signals sit on the right edge.
// Swipes anywhere on the 3D view do the same: ←/→ steer, ↑ hop, ↓ cut / plug.

import { h } from "../ui/dom";
import { GLYPHS } from "../river/types";
import { signalName } from "../ui/panels";
import type { Intent } from "./controls";

/** True on phones/tablets (coarse pointer or any touch points). */
export function isTouch(): boolean {
  try {
    return matchMedia("(pointer: coarse)").matches || navigator.maxTouchPoints > 0;
  } catch {
    return false;
  }
}

const SWIPE = 36; // px before a drag counts as a swipe

export class TouchControls {
  readonly root: HTMLElement;
  private start: { x: number; y: number; id: number } | null = null;
  private down = (e: PointerEvent) => {
    if (e.pointerType === "mouse") return;
    this.start = { x: e.clientX, y: e.clientY, id: e.pointerId };
  };
  private up = (e: PointerEvent) => {
    const s = this.start;
    if (!s || s.id !== e.pointerId) return;
    this.start = null;
    const dx = e.clientX - s.x;
    const dy = e.clientY - s.y;
    if (Math.max(Math.abs(dx), Math.abs(dy)) < SWIPE) return;
    if (Math.abs(dx) > Math.abs(dy)) this.emit({ type: "steer", dir: dx < 0 ? -1 : 1 });
    else this.emit(dy < 0 ? { type: "hop" } : { type: "action" });
  };

  constructor(
    host: HTMLElement,
    private surface: HTMLElement,
    private emit: (it: Intent) => void,
  ) {
    const btn = (cls: string, label: string | Node[], it: Intent, aria: string) =>
      h(
        "button",
        {
          class: "tc-btn " + cls,
          "aria-label": aria,
          // pointerdown, not click: no 300 ms delay and works while the other thumb is held
          onpointerdown: (e: PointerEvent) => {
            e.preventDefault();
            e.stopPropagation();
            navigator.vibrate?.(8);
            this.emit(it);
          },
          oncontextmenu: (e: Event) => e.preventDefault(),
        },
        label,
      );
    this.root = h(
      "div",
      { class: "touch-controls" },
      h("div", { class: "tc-left", "data-tut": "steer" }, btn("tc-steer", "◀", { type: "steer", dir: -1 }, "Steer left"), btn("tc-steer", "▶", { type: "steer", dir: 1 }, "Steer right")),
      h("div", { class: "tc-right" }, btn("tc-act tut-action", "✂", { type: "action" }, "Cut weed or plug pipe"), btn("tc-hop tut-hop", "▲", { type: "hop" }, "Hop")),
      h(
        "div",
        { class: "tc-glyphs", role: "group", "aria-label": "Signals and gate code", "data-tut": "signals" },
        GLYPHS.map((g, i) =>
          btn("tc-glyph", [h("b", null, g), h("small", null, signalName(i)), h("i", null, String(i + 1))], { type: "glyph", g: i }, `Signal ${i + 1}: ${signalName(i)}`),
        ),
      ),
      h("div", { class: "rotate-hint" }, "↻ Turn your phone sideways for a wider river"),
    );
    host.appendChild(this.root);
    document.body.classList.add("touch-run");
    surface.addEventListener("pointerdown", this.down);
    window.addEventListener("pointerup", this.up);
    window.addEventListener("pointercancel", this.up);
  }

  dispose() {
    this.surface.removeEventListener("pointerdown", this.down);
    window.removeEventListener("pointerup", this.up);
    window.removeEventListener("pointercancel", this.up);
    document.body.classList.remove("touch-run");
    this.root.remove();
  }
}
