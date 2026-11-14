// Comfort settings for screen effects, saved per browser.
// Players who prefer reduced motion start with gentler defaults.

export interface FeelPrefs {
  /** Screen shake strength, 0–1. */
  shake: number;
  /** Replace blinking and flashes with steady tints. */
  reduceFlash: boolean;
}

const KEY = "nt.feel";

function defaults(): FeelPrefs {
  let reduced = false;
  try {
    reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  } catch {
    /* old browser */
  }
  return { shake: reduced ? 0.3 : 1, reduceFlash: reduced };
}

function load(): FeelPrefs {
  try {
    return { ...defaults(), ...JSON.parse(localStorage.getItem(KEY) ?? "{}") };
  } catch {
    return defaults();
  }
}

export const feel = {
  prefs: load(),
  set(p: Partial<FeelPrefs>) {
    this.prefs = { ...this.prefs, ...p };
    try {
      localStorage.setItem(KEY, JSON.stringify(this.prefs));
    } catch {
      /* session only */
    }
  },
};
