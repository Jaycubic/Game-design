// Dev builds only: layout checks for play-tests. `__audit()` lists HUD pieces that
// overlap each other or leave the screen; `__stress()` shows every HUD piece at once
// (pipe card, two banners, your key, a 4-player lock card) so the worst case can be checked.

const PIECES = [
  ".rh-pause", ".rh-crew", ".rh-time", ".rh-badges", ".rh-health", ".rh-pipe", ".rh-banner", ".coach",
  ".rh-key", ".rh-code > *", ".rh-boost", ".rh-keys", ".rh-signals",
  ".tc-left", ".tc-right", ".tc-glyphs", ".tc-swipe-hint", ".rotate-hint",
];

function shown(el: Element): boolean {
  for (let e: Element | null = el; e; e = e.parentElement) {
    const cs = getComputedStyle(e);
    if (cs.display === "none" || cs.visibility === "hidden") return false;
  }
  if (Number(getComputedStyle(el).opacity) === 0) return false;
  const r = el.getBoundingClientRect();
  return r.width > 2 && r.height > 2;
}

/** The part of an element you can actually see (ancestors with overflow hidden clip it). */
function clipped(el: Element): DOMRect {
  const r = el.getBoundingClientRect();
  let { left, top, right, bottom } = r;
  for (let e = el.parentElement; e; e = e.parentElement) {
    if (getComputedStyle(e).overflow === "visible") continue;
    const c = e.getBoundingClientRect();
    left = Math.max(left, c.left);
    top = Math.max(top, c.top);
    right = Math.min(right, c.right);
    bottom = Math.min(bottom, c.bottom);
  }
  return new DOMRect(left, top, Math.max(0, right - left), Math.max(0, bottom - top));
}

export function audit() {
  const items: { sel: string; r: DOMRect }[] = [];
  for (const sel of PIECES) for (const el of document.querySelectorAll(sel)) if (shown(el)) items.push({ sel, r: clipped(el) });
  const W = innerWidth;
  const H = innerHeight;
  const offscreen = items
    .filter(({ r }) => r.left < -1 || r.top < -1 || r.right > W + 1 || r.bottom > H + 1)
    .map(({ sel, r }) => `${sel} [${r.left | 0},${r.top | 0},${r.right | 0},${r.bottom | 0}]`);
  const overlaps: string[] = [];
  for (let i = 0; i < items.length; i++)
    for (let j = i + 1; j < items.length; j++) {
      const a = items[i].r;
      const b = items[j].r;
      const ox = Math.min(a.right, b.right) - Math.max(a.left, b.left);
      const oy = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
      if (ox > 2 && oy > 2) overlaps.push(`${items[i].sel} × ${items[j].sel} (${ox | 0}×${oy | 0})`);
    }
  return { size: `${W}x${H}`, shown: items.map((x) => x.sel).join(" "), overlaps, offscreen };
}

export function stress(hud: { banner(t: string, k?: string): void }) {
  hud.banner("🔧 Pipe fixed! River +10 · every plastic now ×1.5", "good big");
  hud.banner("A pipe was left open behind you! River −5", "bad");
  hud.banner("A third banner pushes out the oldest", "warn");
  const pipe = document.querySelector(".rh-pipe")!;
  if (!pipe.firstChild)
    pipe.innerHTML =
      '<div class="rh-pipe-head"><span class="rh-pipe-ico">🔧</span><b>Sewage pipe</b><span class="rh-pipe-side">RIGHT bank ▶</span><span class="rh-pipe-dist">54 m</span></div><div class="rh-pipe-step">Move right ▶ to the edge lane</div>';
  pipe.classList.add("on");
  const key = document.querySelector(".rh-key")!;
  key.innerHTML = '<div class="rh-key-title">Your key for the next pipe</div><div class="rh-key-glyph">🐟<small>Fish</small></div><div class="rh-key-hint">Remember it!</div>';
  key.classList.add("on");
  const slot = (c: string, n: string, k: string) => `<div class="rh-slot" style="--c:${c}"><span class="rh-slot-name">${n}</span><span class="rh-slot-key">${k}</span></div>`;
  document.querySelector(".rh-code")!.innerHTML =
    `<div class="rh-code-card lock call"><div class="rh-code-title">🔧 Asha is fixing a pipe — send your key! Fix the pipe — enter every key</div><div class="rh-slots">${slot("#f2c14e", "You", "🍃")}${slot("#4fb3ff", "Asha", "?")}${slot("#ff7a59", "Ravi", "?")}${slot("#9b6bff", "Mei", "?")}</div><div class="rh-code-hint">Time left 9 s</div></div>`;
}
