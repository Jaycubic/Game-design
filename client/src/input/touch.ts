// On-screen controls for a run: thumb buttons and/or swipes on the river.
// Left thumb steers (◀ ▶), right thumb hops and cuts, signals sit on the right edge.
// Swipes anywhere on the 3D view do the same: ←/→ steer, ↑ hop, ↓ cut / plug.
// Players pick the scheme in Settings → Controls, like most mobile games:
// buttons + swipes (the default on phones and tablets), buttons only, swipes
// only, or off (the default on PCs, where the keyboard and gamepad are used).
// Keyboard and gamepad always work whatever is chosen.

import { h } from "../ui/dom";
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

export type TouchMode = "both" | "buttons" | "swipe" | "off";
export type TouchSize = "s" | "m" | "l";
export interface TouchPrefs {
  mode: TouchMode;
  size: TouchSize;
  /** Left-handed: steering on the right, hop and cut on the left. */
  swap: boolean;
}
const PREFS_KEY = "nt.touch";

function defaults(): TouchPrefs {
  return { mode: isTouch() ? "both" : "off", size: "m", swap: false };
}

export const touchPrefs = {
  get(): TouchPrefs {
    try {
      return { ...defaults(), ...JSON.parse(localStorage.getItem(PREFS_KEY) ?? "{}") };
    } catch {
      return defaults();
    }
  },
  set(p: Partial<TouchPrefs>) {
    try {
      localStorage.setItem(PREFS_KEY, JSON.stringify({ ...this.get(), ...p }));
    } catch {
      /* session only */
    }
  },
};

/** Whether this player uses on-screen controls (buttons and/or swipes). */
export function usesTouch(): boolean {
  return touchPrefs.get().mode !== "off";
}

/** How to do the action, in the player's chosen scheme: "tap ✂", "swipe down" or "press ▼". */
export function actionVerb(key = "▼"): string {
  const m = touchPrefs.get().mode;
  return m === "both" || m === "buttons" ? "tap ✂" : m === "swipe" ? "swipe down" : `press ${key}`;
}

export class TouchControls {
  readonly root: HTMLElement;
  private start: { x: number; y: number; id: number } | null = null;
  private prefs = touchPrefs.get();
  private down = (e: PointerEvent) => {
    if (e.pointerType === "mouse" || (this.prefs.mode !== "both" && this.prefs.mode !== "swipe")) return;
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
    const p = this.prefs;
    const buttons = p.mode === "both" || p.mode === "buttons";
    this.root = h(
      "div",
      { class: `touch-controls tc-size-${p.size}` + (p.swap ? " tc-swap" : "") + (buttons ? "" : " tc-swipe-only") },
      h("div", { class: "tc-left", "data-tut": "steer" }, btn("tc-steer", "◀", { type: "steer", dir: -1 }, "Steer left"), btn("tc-steer", "▶", { type: "steer", dir: 1 }, "Steer right")),
      h("div", { class: "tc-right" }, btn("tc-act tut-action", "✂", { type: "action" }, "Cut weed or plug pipe"), btn("tc-hop tut-hop", "▲", { type: "hop" }, "Hop")),
      p.mode === "swipe" ? h("div", { class: "tc-swipe-hint" }, "Swipe ← → to steer · ↑ hop · ↓ cut / plug") : null,
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
