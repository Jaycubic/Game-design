// How-to-play and Settings overlays.

import { h } from "./dom";
import type { Quality } from "../render/engine";

export function howToPlay(): HTMLElement {
  const row = (icon: string, keys: string, text: string) => h("li", null, h("span", { class: "htp-icon" }, icon), h("kbd", null, keys), h("span", null, text));
  return h(
    "div",
    { class: "htp" },
    h(
      "ul",
      { class: "htp-list" },
      row("🚤", "◀ ▶", "Switch lanes — your boat moves on its own"),
      row("🦘", "▲", "Hop over logs and debris waves"),
      row("✂️", "▼", "Cut weed mats · plug sewage pipes"),
      row("♻️", "", "Grab plastic — it fills your boost"),
      row("🔧", "1–6", "Type gate codes · signal your crew"),
      row("🌊", "", "River Health is shared — if it chokes, everyone loses half"),
      row("⚡", "", "Twice per run, players change the rules"),
    ),
  );
}

export function settings(current: Quality, backend: string, reason: string, onQuality: (q: Quality) => void): HTMLElement {
  return h(
    "div",
    { class: "settings" },
    h("p", null, "Renderer: ", h("strong", null, backend.toUpperCase()), reason && backend !== "webgpu" ? h("span", { class: "muted" }, ` — ${reason}`) : null),
    h(
      "fieldset",
      null,
      h("legend", null, "Graphics quality"),
      (["low", "medium", "high"] as Quality[]).map((q) =>
        h(
          "label",
          { class: "radio" },
          h("input", { type: "radio", name: "quality", value: q, checked: q === current, onchange: () => onQuality(q) }),
          ` ${q[0].toUpperCase() + q.slice(1)}`,
          h("small", { class: "muted" }, q === "low" ? " — lower resolution (older laptops)" : q === "medium" ? " — recommended" : " — full resolution"),
        ),
      ),
    ),
    h("p", { class: "muted small" }, "Changing quality reloads the game. Your room is kept on the server."),
  );
}
