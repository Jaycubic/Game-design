// Keyboard bindings (rebindable, saved per browser) and gamepad polling.
// Arrow keys always work; each action also has one custom key.

export type Bindable = "left" | "right" | "hop" | "action";
export type Intent =
  | { type: "steer"; dir: -1 | 1 }
  | { type: "hop" }
  | { type: "action" }
  | { type: "glyph"; g: number }
  | { type: "choose"; option: number };

export const BIND_LABELS: Record<Bindable, string> = { left: "Steer left", right: "Steer right", hop: "Hop", action: "Cut / plug" };
const FIXED: Record<Bindable, string> = { left: "arrowleft", right: "arrowright", hop: "arrowup", action: "arrowdown" };
const DEFAULTS: Record<Bindable, string> = { left: "a", right: "d", hop: "w", action: "s" };
const KEY = "nt.keys";

let custom: Record<Bindable, string> = load();

function load(): Record<Bindable, string> {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? "null");
    if (v && typeof v === "object") return { ...DEFAULTS, ...v };
  } catch {
    /* defaults */
  }
  return { ...DEFAULTS };
}

export function bindings(): Record<Bindable, string> {
  return { ...custom };
}

export function rebind(action: Bindable, key: string) {
  const k = key.toLowerCase();
  // A key can only do one thing: free it from any other action first.
  for (const a of Object.keys(custom) as Bindable[]) if (custom[a] === k) custom[a] = "";
  custom[action] = k;
  try {
    localStorage.setItem(KEY, JSON.stringify(custom));
  } catch {
    /* session only */
  }
}

export function resetBindings() {
  custom = { ...DEFAULTS };
  try {
    localStorage.removeItem(KEY);
  } catch {
    /* ignore */
  }
}

export function keyLabel(k: string): string {
  const names: Record<string, string> = { arrowleft: "◀", arrowright: "▶", arrowup: "▲", arrowdown: "▼", " ": "Space", "": "—" };
  return names[k] ?? k.toUpperCase();
}

/** "◀ / A" style label for HUD hints. */
export function hint(a: Bindable): string {
  return custom[a] ? `${keyLabel(FIXED[a])} / ${keyLabel(custom[a])}` : keyLabel(FIXED[a]);
}

/** Map a key press to an intent. `choosing` turns 1–3 into vote options. */
export function intentForKey(key: string, choosing: boolean): Intent | null {
  const k = key.toLowerCase();
  if (choosing && k >= "1" && k <= "3") return { type: "choose", option: Number(k) - 1 };
  const is = (a: Bindable) => k === FIXED[a] || (custom[a] !== "" && k === custom[a]);
  if (is("left")) return { type: "steer", dir: -1 };
  if (is("right")) return { type: "steer", dir: 1 };
  if (is("hop") || k === " ") return { type: "hop" };
  if (is("action") || k === "e") return { type: "action" };
  if (k >= "1" && k <= "6") return { type: "glyph", g: Number(k) - 1 };
  return null;
}

/**
 * Standard-mapping gamepad: D-pad or left stick steers, A hops, B/X cut or plug.
 * Hold RT and press A/B/X/Y/LB/RB to send signals 1–6. While voting, A/B/X pick options 1–3.
 */
export class GamepadInput {
  private prev = new Map<number, boolean[]>();
  private stickLatch = new Map<number, number>();

  connected(): string | null {
    const pads = navigator.getGamepads?.() ?? [];
    for (const p of pads) if (p) return p.id;
    return null;
  }

  poll(choosing: boolean): Intent[] {
    const out: Intent[] = [];
    const pads = navigator.getGamepads?.() ?? [];
    for (const p of pads) {
      if (!p) continue;
      const now = p.buttons.map((b) => b.pressed);
      const before = this.prev.get(p.index) ?? [];
      const pressed = (i: number) => !!now[i] && !before[i];
      const held = (i: number) => !!now[i];
      if (choosing) {
        [0, 1, 2].forEach((b, i) => pressed(b) && out.push({ type: "choose", option: i }));
      } else if (held(7)) {
        [0, 1, 2, 3, 4, 5].forEach((b, i) => pressed(b) && out.push({ type: "glyph", g: i }));
      } else {
        if (pressed(14)) out.push({ type: "steer", dir: -1 });
        if (pressed(15)) out.push({ type: "steer", dir: 1 });
        if (pressed(0) || pressed(12)) out.push({ type: "hop" });
        if (pressed(1) || pressed(2) || pressed(13)) out.push({ type: "action" });
        // Left stick flick: one lane per push, re-armed when the stick centres.
        const x = p.axes[0] ?? 0;
        const latch = this.stickLatch.get(p.index) ?? 0;
        if (latch === 0 && Math.abs(x) > 0.6) {
          out.push({ type: "steer", dir: x < 0 ? -1 : 1 });
          this.stickLatch.set(p.index, x < 0 ? -1 : 1);
        } else if (Math.abs(x) < 0.3) this.stickLatch.set(p.index, 0);
      }
      this.prev.set(p.index, now);
    }
    return out;
  }
}
