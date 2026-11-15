// How-to-play and Settings overlays.

import { sound } from "../audio/sound";
import { feel } from "./feel";
import { GLYPHS, GLYPH_NAMES } from "../river/types";
import { isTouch, touchPrefs, usesTouch, type TouchMode, type TouchSize } from "../input/touch";
import { h } from "./dom";
import type { Quality } from "../render/engine";
import { BIND_LABELS, bindings, GamepadInput, keyLabel, rebind, resetBindings, type Bindable } from "../input/controls";

/** On-screen controls: scheme (like most mobile games), button size and left-handed layout. */
function touchSection(redraw: () => void): HTMLElement {
  const p = touchPrefs.get();
  const opt = (mode: TouchMode, label: string, note: string) =>
    h(
      "label",
      { class: "radio scheme" + (p.mode === mode ? " on" : "") },
      h("input", { type: "radio", name: "touch-mode", value: mode, checked: p.mode === mode, onchange: () => (touchPrefs.set({ mode }), redraw()) }),
      h("span", null, h("b", null, label), h("small", { class: "muted" }, note)),
    );
  const buttons = p.mode === "both" || p.mode === "buttons";
  return h(
    "fieldset",
    { class: "touch-settings" },
    h("legend", null, "On-screen controls"),
    h("p", { class: "muted small" }, isTouch() ? "For phones and tablets. Keyboard and gamepad always work too." : "For touch-screen laptops. Keyboard and gamepad always work."),
    h(
      "div",
      { class: "scheme-grid" },
      opt("both", "Buttons + swipes", "Tap the thumb buttons or swipe on the river"),
      opt("buttons", "Buttons only", "Thumb buttons; swipes are ignored"),
      opt("swipe", "Swipes only", "← → steer · ↑ hop · ↓ cut / plug"),
      opt("off", "Off", isTouch() ? "Keyboard or gamepad only" : "Keyboard or gamepad (default on PC)"),
    ),
    buttons
      ? h(
          "div",
          { class: "slider-row" },
          h("span", null, "Button size"),
          h(
            "div",
            { class: "seg" },
            (["s", "m", "l"] as TouchSize[]).map((size) =>
              h("button", { class: "btn tiny" + (p.size === size ? " primary" : " ghost"), onclick: () => (touchPrefs.set({ size }), redraw()) }, { s: "Small", m: "Medium", l: "Large" }[size]),
            ),
          ),
        )
      : null,
    buttons
      ? h(
          "label",
          { class: "radio" },
          h("input", { type: "checkbox", checked: p.swap, onchange: (e: Event) => (touchPrefs.set({ swap: (e.target as HTMLInputElement).checked }), redraw()) }),
          " Left-handed (steer on the right, hop and cut on the left)",
        )
      : null,
    h("p", { class: "muted small" }, "Takes effect at the start of the next run."),
  );
}

/** Keyboard rebinding (arrow keys always work) and gamepad layout. */
export function controlsSection(): HTMLElement {
  const root = h("div", { class: "controls" });
  const pad = new GamepadInput();
  const draw = () => {
    const b = bindings();
    root.replaceChildren(
      h("h3", null, "Controls"),
      touchSection(draw),
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
      row("✂️", usesTouch() ? "✂" : "▼", "Cut weed mats · plug sewage pipes"),
      row("♻️", "", "Grab plastic — it fills your boost"),
      row("🔧", usesTouch() ? "Tap" : "1–6", "Type gate codes · signal your crew"),
      row("🌊", "", "River Health is shared — if it chokes, everyone loses half"),
      row("⚡", "", "Twice per run, players change the rules"),
      row("💥", "", "Capsize and you stop for a moment and spill 3 plastic — net it back"),
      row("🗺️", "", "New rivers add ghost nets, currents and rocks — earn 3 stars on each"),
      row("∞", "", "Endless: solo, no clock, one capsize ends it — how far can you go?"),
    ),
    h("h3", null, "Signals"),
    h(
      "p",
      { class: "small" },
      "The six signals have no fixed meaning — your crew invents its own code. When a crewmate plugs a pipe, you see the real gate code and they see it scrambled: send them the symbols in order. The other crew sees your signals too, so you can bluff.",
    ),
    h(
      "div",
      { class: "htp-signals" },
      GLYPHS.map((g, i) => h("span", { class: "htp-signal" }, h("b", null, g), h("small", null, `${i + 1} ${signalName(i)}`))),
    ),
  );
}

/** "Leaf", "Drop", … */
export function signalName(i: number): string {
  const n = GLYPH_NAMES[i] ?? "";
  return n.charAt(0).toUpperCase() + n.slice(1);
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
    comfortSection(),
    controlsSection(),
  );
}

function comfortSection(): HTMLElement {
  return h(
    "fieldset",
    { class: "audio-settings" },
    h("legend", null, "Comfort"),
    h(
      "label",
      { class: "slider-row" },
      h("span", null, "Screen shake"),
      h("input", {
        type: "range",
        min: "0",
        max: "100",
        value: String(Math.round(feel.prefs.shake * 100)),
        oninput: (e: Event) => feel.set({ shake: Number((e.target as HTMLInputElement).value) / 100 }),
      }),
    ),
    h(
      "label",
      { class: "radio" },
      h("input", { type: "checkbox", checked: feel.prefs.reduceFlash, onchange: (e: Event) => feel.set({ reduceFlash: (e.target as HTMLInputElement).checked }) }),
      " Reduce flashing (no blinking boats or red flashes)",
    ),
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
