// Hub panels: Garage, Shop, Daily, River Road. Everything is earned by playing.

import { api, ApiError, type CatalogBoat, type Profile } from "../net/api";
import { store } from "../store";
import { icon } from "./icons";
import { h, toast } from "./dom";

export type HubPanel = "garage" | "shop" | "daily" | "road";

export const PANEL_TITLES: Record<HubPanel, string> = {
  garage: "Garage",
  shop: "Shop",
  daily: "Daily reward",
  road: "River Road",
};

export async function loadProfile(): Promise<Profile | null> {
  try {
    const p = await api.profile();
    store.set({ profile: p });
    return p;
  } catch {
    return null;
  }
}

const STAT_NAMES: [keyof CatalogBoat["stats"], string][] = [
  ["speed", "Speed"],
  ["net", "Net"],
  ["cutter", "Cutter"],
  ["boost", "Boost"],
];

function boatChip(b: CatalogBoat) {
  return h(
    "div",
    { class: "boat-chip" },
    h("div", { class: "boat-pic", style: `--glow:${b.colour}` }, h("img", { src: `${import.meta.env.BASE_URL}models/boats/previews/${b.model}.png`, alt: "", loading: "lazy" })),
    h("div", { class: "boat-name" }, b.name),
    h("div", { class: "boat-blurb" }, b.blurb),
    h(
      "div",
      { class: "boat-stats" },
      STAT_NAMES.map(([k, label]) =>
        h(
          "div",
          { class: "stat-row", title: `${label} ${b.stats[k]} / 5` },
          h("span", null, label),
          h("span", { class: "pips5" }, [1, 2, 3, 4, 5].map((i) => h("i", { class: i <= b.stats[k] ? "on" : "" }))),
        ),
      ),
    ),
  );
}

function howToGet(b: CatalogBoat, p: Profile): string {
  if (b.source === "shop") return `Shop · ${b.price} Seeds`;
  const m = p.road.find((r) => r.item === b.key);
  return m ? `River Road · ${m.runs} ${m.runs === 1 ? "run" : "runs"}` : "Locked";
}

async function act(fn: () => Promise<unknown>, ok: string, rerender: () => void) {
  try {
    await fn();
    toast(ok, "good");
  } catch (e) {
    toast(e instanceof ApiError ? e.message : String(e), "error");
  }
  await loadProfile();
  rerender();
}

/** Builds a panel that redraws itself after every action. */
export function hubPanel(kind: HubPanel): HTMLElement {
  const root = h("div", { class: "hub-panel" });
  const draw = () => {
    const p = store.state.profile;
    if (!p) {
      root.replaceChildren(h("p", null, "Loading…"));
      return;
    }
    root.replaceChildren(h("div", { class: "seeds-line" }, h("span", { class: "seed-ico" }, icon("sprout")), h("strong", null, String(p.seeds)), " Seeds"), ...content(kind, p, draw));
  };
  draw();
  void loadProfile().then(draw);
  return root;
}

function content(kind: HubPanel, p: Profile, redraw: () => void): HTMLElement[] {
  const owned = new Set(p.owned);
  switch (kind) {
    case "garage":
      return [
        h("p", { class: "muted" }, "Every boat has its own speed, net, cutter and boost. Pick one to sail — your crew sees it in the run."),
        h(
          "div",
          { class: "boat-grid" },
          // Owned boats first; the rest shown locked with how to get them.
          [...p.catalog]
            .sort((a, b) => Number(owned.has(b.key)) - Number(owned.has(a.key)))
            .map((b) =>
              h(
                "div",
                { class: "boat-card" + (p.equipped === b.key ? " equipped" : "") + (owned.has(b.key) ? "" : " locked") },
                boatChip(b),
                !owned.has(b.key)
                  ? h("span", { class: "chip" }, icon("lock"), howToGet(b, p))
                  : p.equipped === b.key
                    ? h("span", { class: "chip good" }, icon("check"), "In use")
                    : h("button", { class: "btn", onclick: () => act(() => api.equip(b.key), `${b.name} is ready to sail.`, redraw) }, "Use"),
              ),
            ),
        ),
      ];
    case "shop":
      return [
        h("p", { class: "muted" }, `Better boats for the Seeds you earn in runs. Changed your mind? Undo within ${p.rules.refund_minutes} minutes.`),
        h(
          "div",
          { class: "boat-grid" },
          p.catalog
            .filter((b) => b.source === "shop")
            .map((b) => {
              const has = owned.has(b.key);
              const canUndo = p.refundable.includes(b.key);
              return h(
                "div",
                { class: "boat-card" },
                boatChip(b),
                has
                  ? canUndo
                    ? h("button", { class: "btn ghost", onclick: () => act(() => api.undo(b.key), `Refunded ${b.price} Seeds.`, redraw) }, "↶ Undo")
                    : h("span", { class: "chip good" }, "Owned")
                  : h(
                      "button",
                      {
                        class: "btn " + (p.seeds >= b.price ? "primary" : ""),
                        disabled: p.seeds < b.price,
                        onclick: () => {
                          if (confirm(`Buy ${b.name} for ${b.price} Seeds?`)) void act(() => api.buy(b.key), `${b.name} added to your Garage.`, redraw);
                        },
                      },
                      h("span", { class: "price" }, icon("sprout"), String(b.price)),
                    ),
              );
            }),
        ),
      ];
    case "daily": {
      const day = p.daily.claims % 7;
      return [
        h("p", { class: "muted" }, "One reward a day. Missed a day? Your track waits for you — it never resets."),
        h(
          "div",
          { class: "daily-track" },
          p.daily.track.map((r, i) =>
            h(
              "div",
              { class: "daily-day" + (i < day || (p.daily.claimed_today && i === day - 1) ? " done" : "") + (i === day && !p.daily.claimed_today ? " today" : "") },
              h("span", { class: "daily-n" }, `Day ${i + 1}`),
              h("span", { class: "daily-r" }, icon("sprout"), String(r)),
            ),
          ),
        ),
        p.daily.claimed_today
          ? h("p", { class: "center-text" }, "✔ Collected today — see you tomorrow.")
          : h("button", { class: "btn primary big wide", onclick: () => act(() => api.claimDaily(), `+${p.daily.next} Seeds!`, redraw) }, icon("gift"), `Collect ${p.daily.next} Seeds`),
      ];
    }
    case "road":
      return [
        h("p", { class: "muted" }, `Every run you finish moves you along the river. Runs so far: ${p.runs}.`),
        h(
          "ol",
          { class: "road" },
          p.road.map((m) => {
            const item = m.item ? p.catalog.find((b) => b.key === m.item) : null;
            return h(
              "li",
              { class: "road-step" + (m.claimed ? " claimed" : m.reached ? " reached" : "") },
              h("span", { class: "road-runs" }, `${m.runs}`),
              h("span", { class: "road-reward" }, icon("sprout"), String(m.seeds), item ? h("span", { class: "road-item" }, " + ", item.name, " boat") : null),
              m.claimed
                ? h("span", { class: "chip good" }, "✔")
                : m.reached
                  ? h("button", { class: "btn primary", onclick: () => act(() => api.claimRoad(m.runs), "Milestone collected!", redraw) }, "Collect")
                  : h("span", { class: "chip" }, `${Math.max(0, m.runs - p.runs)} to go`),
            );
          }),
        ),
      ];
  }
}
