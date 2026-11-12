// One consistent icon set (24px grid, 2px rounded strokes) so menus don't
// rely on emoji, which render differently on every device.

const P: Record<string, string> = {
  boat: '<path d="M3 15h18l-2.5 4.5a2 2 0 0 1-1.8 1H7.3a2 2 0 0 1-1.8-1Z"/><path d="M6 15V9h7l3 6"/><path d="M9 9V5"/>',
  shop: '<path d="M4 9h16l-1.2 10a2 2 0 0 1-2 1.8H7.2a2 2 0 0 1-2-1.8Z"/><path d="M9 9V7a3 3 0 0 1 6 0v2"/>',
  calendar: '<rect x="3.5" y="5" width="17" height="15.5" rx="2.5"/><path d="M3.5 10h17M8 3v4M16 3v4"/><path d="m9.5 15 2 2 3.5-3.5"/>',
  road: '<path d="M4 20c4-1 2-6 6-7s6 2 10-9"/><circle cx="20" cy="4" r="1.5"/><circle cx="4" cy="20" r="1.5"/>',
  cap: '<path d="m2 9.5 10-5 10 5-10 5Z"/><path d="M6 11.5V16c3 2.5 9 2.5 12 0v-4.5M22 9.5V15"/>',
  help: '<circle cx="12" cy="12" r="9"/><path d="M9.5 9.5a2.5 2.5 0 1 1 3.3 2.4c-.5.2-.8.7-.8 1.2V14"/><circle cx="12" cy="17.2" r=".6" fill="currentColor"/>',
  gear: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1Z"/>',
  power: '<path d="M12 3v8"/><path d="M6.3 7.3a8 8 0 1 0 11.4 0"/>',
  sprout: '<path d="M12 21v-9"/><path d="M12 12C12 7.5 9 5 4 5c0 4.5 3 7 8 7Z"/><path d="M12 14c0-3.5 2.5-6 7-6 0 3.5-2.5 6-7 6Z"/>',
  close: '<path d="M6 6l12 12M18 6 6 18"/>',
  play: '<path d="M7 4.5v15a1 1 0 0 0 1.5.9l12-7.5a1 1 0 0 0 0-1.8l-12-7.5A1 1 0 0 0 7 4.5Z" fill="currentColor"/>',
  users: '<circle cx="9" cy="8" r="3.5"/><path d="M2.5 20a6.5 6.5 0 0 1 13 0"/><path d="M16 4.6a3.5 3.5 0 0 1 0 6.8M18 14a6.5 6.5 0 0 1 3.5 6"/>',
  solo: '<circle cx="12" cy="8" r="3.5"/><path d="M5.5 20a6.5 6.5 0 0 1 13 0"/>',
  back: '<path d="M15 5l-7 7 7 7"/>',
  wave: '<path d="M2 15c2.5 0 2.5-2 5-2s2.5 2 5 2 2.5-2 5-2 2.5 2 5 2"/><path d="M2 19c2.5 0 2.5-2 5-2s2.5 2 5 2 2.5-2 5-2 2.5 2 5 2"/><path d="M12 11C8 11 7 7 9 3c1 2 3 2.5 4 4.5S14 11 12 11Z"/>',
  trophy: '<path d="M8 4h8v5a4 4 0 0 1-8 0Z"/><path d="M8 6H4.5a3 3 0 0 0 3.5 4M16 6h3.5a3 3 0 0 1-3.5 4M12 13v4M8 20h8"/>',
  copy: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/>',
  lock: '<rect x="5" y="11" width="14" height="9.5" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>',
  check: '<path d="m5 12.5 4.5 4.5L19 7.5"/>',
  gift: '<rect x="3.5" y="8" width="17" height="4.5" rx="1"/><path d="M5 12.5V20h14v-7.5M12 8v12M12 8c-2-4-6-4-6-1.5S12 8 12 8Zm0 0c2-4 6-4 6-1.5S12 8 12 8Z"/>',
};

export type IconName = keyof typeof P;

export function icon(name: IconName, cls = ""): SVGSVGElement {
  const wrap = document.createElement("span");
  wrap.innerHTML = `<svg class="ico ${cls}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${P[name]}</svg>`;
  return wrap.firstElementChild as SVGSVGElement;
}

/** The game's mark: a leaf riding a wave, in a honey roundel. */
export function logoMark(): HTMLElement {
  const el = document.createElement("div");
  el.className = "logo-mark";
  el.setAttribute("aria-hidden", "true");
  el.innerHTML = `<svg viewBox="0 0 96 96"><defs><linearGradient id="lm-bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffdf8a"/><stop offset=".6" stop-color="#ffc94a"/><stop offset="1" stop-color="#eda92e"/></linearGradient><linearGradient id="lm-leaf" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#b4e880"/><stop offset="1" stop-color="#4f8a38"/></linearGradient></defs><circle cx="48" cy="50" r="42" fill="#9a6f1a"/><circle cx="48" cy="46" r="42" fill="url(#lm-bg)"/><circle cx="48" cy="46" r="35" fill="#24391f"/><path d="M48 54c-13 0-17-13-9-27 3 7 12 8 15 15s3 12-6 12Z" fill="url(#lm-leaf)"/><path d="M48 54c-2-8-4-13-9-27" stroke="#24391f" stroke-width="2" fill="none" stroke-linecap="round"/><path d="M18 62c5 0 5-4 10-4s5 4 10 4 5-4 10-4 5 4 10 4 5-4 10-4 5 4 10 4" stroke="#6cbce8" stroke-width="4.5" fill="none" stroke-linecap="round"/><path d="M24 71c4 0 4-3 8-3s4 3 8 3 4-3 8-3 4 3 8 3 4-3 8-3 4 3 8 3" stroke="#3f9ad0" stroke-width="4" fill="none" stroke-linecap="round"/></svg>`;
  return el;
}
