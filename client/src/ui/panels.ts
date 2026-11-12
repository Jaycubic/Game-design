// How-to-play and Settings overlays.

import { sound } from "../audio/sound";
import { isTouch } from "../input/touch";
import { h } from "./dom";
import type { Quality } from "../render/engine";
import { BIND_LABELS, bindings, GamepadInput, keyLabel, rebind, resetBindings, type Bindable } from "../input/controls";

/** Keyboard rebinding (arrow keys always work) and gamepad layout. */
export function controlsSection(): HTMLElement {
  const root = h("div", { class: "controls" });
  const pad = new GamepadInput();
  const draw = () => {
    const b = bindings();
    root.replaceChildren(
      h("h3", null, "Controls"),
      h("p", { class: "muted small" }, "Arrow keys always work. Each action also has one key you can choose."),
      h(
        "table",
        { class: "binds" },
        (Object.keys(BIND_LABELS) as Bindable[]).map((a) =>
          h(
            "tr",
            null,
            h("td", null, BIND_LABELS[a]),
            h("td", null, h("kbd", null, keyLabel({ left: "arrowleft", right: "arrowright", hop: "arrowup", action: "arrowdown" }[a]))),
            h("td", null, h("kbd", null, keyLabel(b[a]))),
            h(
              "td",
              null,
              h(
                "button",
                {
                  class: "btn tiny",
                  onclick: (e: Event) => {
                    const btn = e.currentTarget as HTMLButtonElement;
                    btn.textContent = "Press a key…";
                    const grab = (ev: KeyboardEvent) => {
                      ev.preventDefault();
                      ev.stopPropagation();
                      window.removeEventListener("keydown", grab, true);
                      if (ev.key !== "Escape" && !/^[1-6]$/.test(ev.key) && !ev.key.startsWith("Arrow")) rebind(a, ev.key);
                      draw();
                    };
                    window.addEventListener("keydown", grab, true);
                  },
                },
                "Change",
              ),
            ),
          ),
        ),
      ),
      h("p", { class: "muted small" }, "Keys 1–6 send signals and type gate codes. While voting, 1–3 choose an option."),
      h("button", { class: "btn ghost tiny", onclick: () => (resetBindings(), draw()) }, "Reset to W A S D"),
      h("h3", null, "Gamepad"),
      h("p", { class: "small" }, pad.connected() ? `✔ Connected: ${pad.connected()}` : "No gamepad detected — connect one and press any button."),
      h(
        "ul",
        { class: "small pad-map" },
        h("li", null, "D-pad ◀ ▶ or left stick — steer"),
        h("li", null, "A (or D-pad ▲) — hop"),
        h("li", null, "B / X (or D-pad ▼) — cut / plug"),
        h("li", null, "Hold RT + A / B / X / Y / LB / RB — signals 1–6"),
        h("li", null, "While voting: A / B / X — options 1–3"),
      ),
    );
  };
  draw();
  return root;
}

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
      row("✂️", isTouch() ? "✂" : "▼", "Cut weed mats · plug sewage pipes"),
      row("♻️", "", "Grab plastic — it fills your boost"),
      row("🔧", isTouch() ? "Tap" : "1–6", "Type gate codes · signal your crew"),
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
    audioSection(),
    controlsSection(),
  );
}

function audioSection(): HTMLElement {
  const slider = (label: string, key: "master" | "music" | "sfx") =>
    h(
      "label",
      { class: "slider-row" },
      h("span", null, label),
      h("input", {
        type: "range",
        min: "0",
        max: "100",
        value: String(Math.round(sound.prefs[key] * 100)),
        oninput: (e: Event) => sound.set({ [key]: Number((e.target as HTMLInputElement).value) / 100 }),
        onchange: () => key !== "music" && sound.play("pickup"),
      }),
    );
  return h(
    "fieldset",
    { class: "audio-settings" },
    h("legend", null, "Sound"),
    h(
      "label",
      { class: "radio" },
      h("input", { type: "checkbox", checked: sound.prefs.muted, onchange: (e: Event) => sound.set({ muted: (e.target as HTMLInputElement).checked }) }),
      " Mute all sound",
    ),
    slider("Volume", "master"),
    slider("Music", "music"),
    slider("Effects", "sfx"),
  );
}
