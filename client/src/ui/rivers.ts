// River choice: three rivers, each with its own mechanic, three stars and a
// best score. The pick is remembered per browser; the host's pick is sent
// when a run starts (the server checks it is unlocked).

import type { Profile, RiverCard } from "../net/api";
import { store } from "../store";
import { h } from "./dom";
import { icon } from "./icons";

const KEY = "nt.river";
export const RIVER_ICONS: Record<string, string> = { canal: "🏘️", mangrove: "🌿", gorge: "⛰️" };

/** The chosen river, if it is unlocked for this player (otherwise the Town Canal). */
export function selectedRiver(p: Profile | null = store.state.profile): string {
  let k = "canal";
  try {
    k = localStorage.getItem(KEY) ?? "canal";
  } catch {
    /* default */
  }
  const card = p?.rivers?.find((r) => r.key === k);
  return card?.unlocked ? k : "canal";
}

export function setRiver(key: string) {
  try {
    localStorage.setItem(KEY, key);
  } catch {
    /* session only */
  }
}

export function starRow(stars: boolean[], cls = "") {
  return h(
    "span",
    { class: "star-row " + cls, "aria-label": `${stars.filter(Boolean).length} of 3 stars` },
    stars.map((s) => h("span", { class: "star" + (s ? " on" : "") }, "★")),
  );
}

/** Compact button showing the current river and its stars. */
export function riverChip(p: Profile | null, onOpen: () => void, label = "River") {
  const card = p?.rivers?.find((r) => r.key === selectedRiver(p));
  return h(
    "button",
    { class: "river-chip", onclick: onOpen, title: "Choose the river" },
    h("span", { class: "river-chip-ico" }, RIVER_ICONS[card?.key ?? "canal"]),
    h("span", { class: "river-chip-text" }, h("small", null, label), h("b", null, card?.name ?? "Town Canal")),
    card ? starRow(card.stars) : null,
    h("span", { class: "river-chip-more" }, "▸"),
  );
}

function need(c: RiverCard) {
  return `${c.need_runs} runs or ${c.need_stars} ★`;
}

/** The river cards for the Rivers panel. */
export function riverCards(p: Profile, redraw: () => void): HTMLElement[] {
  const chosen = selectedRiver(p);
  return [
    h("p", { class: "muted" }, `Each river adds something new. Earn its three stars to unlock the next. You have ${p.stars ?? 0} ★.`),
    h(
      "div",
      { class: "river-grid" },
      (p.rivers ?? []).map((c) =>
        h(
          "div",
          { class: "river-card" + (c.key === chosen ? " chosen" : "") + (c.unlocked ? "" : " locked") },
          h("div", { class: "river-card-head" }, h("span", { class: "river-ico" }, RIVER_ICONS[c.key]), h("b", null, c.name), starRow(c.stars, "big")),
          h("p", { class: "river-line" }, c.line),
          h(
            "ul",
            { class: "river-goals" },
            c.goals.map((g, i) => h("li", { class: c.stars[i] ? "done" : "" }, h("span", { class: "star" + (c.stars[i] ? " on" : "") }, "★"), g)),
          ),
          h(
            "div",
            { class: "river-best" },
            "Best score ",
            h("b", null, c.best ? String(c.best) : "—"),
            " · Endless ",
            h("b", null, c.best_endless ? String(c.best_endless) : "—"),
          ),
          !c.unlocked
            ? h("span", { class: "chip" }, icon("lock"), `Unlocks at ${need(c)}`)
            : c.key === chosen
              ? h("span", { class: "chip good" }, icon("check"), "Chosen")
              : h("button", { class: "btn", onclick: () => (setRiver(c.key), redraw()) }, "Sail here"),
        ),
      ),
    ),
  ];
}
