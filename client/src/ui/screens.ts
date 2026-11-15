// Menu screens: title → sign in → hub → crew lobby.

import { isTouch } from "../input/touch";
import { store, type AppState } from "../store";
import type { RoomView } from "../types";
import { h } from "./dom";
import { icon, logoMark, type IconName } from "./icons";
import { riverChip } from "./rivers";

export const GAME_TITLE = "Clearwater Run";
export const GAME_TAGLINE = "Keep the river alive";

export interface UiActions {
  login(handle: string, password: string): Promise<void>;
  register(handle: string, name: string, password: string): Promise<void>;
  logout(): void;
  play(): Promise<void>;
  practice(): Promise<void>;
  joinRoom(code: string): Promise<void>;
  openRoom(id: string): void;
  leaveRoom(): void;
  ready(v: boolean): void;
  startRun(practice: boolean): void;
  openHowTo(): void;
  openSettings(): void;
  openPanel(kind: "garage" | "shop" | "daily" | "road" | "rivers"): void;
  tutorial(): Promise<void>;
}

function logo() {
  return h(
    "div",
    { class: "logo-block" },
    logoMark(),
    h("h1", { class: "logo-title" }, GAME_TITLE),
    h("p", { class: "logo-tag" }, GAME_TAGLINE),
  );
}

/** First screen: the game's name over the live river, "press any key". */
export function titleScreen(onStart: () => void): HTMLElement {
  const el = h(
    "main",
    { class: "screen title-screen", tabindex: 0, onclick: onStart, onkeydown: (e: KeyboardEvent) => (e.preventDefault(), onStart()) },
    logo(),
    h("div", { class: "press-start" }, isTouch() ? "Tap to start" : "Press any key to start"),
    h("div", { class: "title-foot" }, "2–4 players · each on their own device · same Wi-Fi"),
  );
  setTimeout(() => el.focus(), 0);
  return el;
}

export function authScreen(a: UiActions, error: string | null): HTMLElement {
  let mode: "login" | "register" = "login";
  const err = h("p", { class: "form-error", role: "alert" }, error ?? "");
  const nameRow = h("label", { class: "field hidden" }, "Display name", h("input", { name: "name", maxlength: 32, autocomplete: "nickname" }));
  const submit = h("button", { class: "btn primary big", type: "submit" }, "Sign in");
  const form = h(
    "form",
    {
      class: "auth-form",
      onsubmit: async (e: Event) => {
        e.preventDefault();
        const f = new FormData(form);
        const handle = String(f.get("handle") ?? ""),
          pw = String(f.get("password") ?? "");
        submit.disabled = true;
        err.textContent = "";
        try {
          if (mode === "login") await a.login(handle, pw);
          else await a.register(handle, String(f.get("name") ?? handle), pw);
        } catch (x) {
          err.textContent = x instanceof Error ? x.message : String(x);
        } finally {
          submit.disabled = false;
        }
      },
    },
    h("label", { class: "field" }, "Handle", h("input", { name: "handle", required: true, minlength: 3, maxlength: 24, autocomplete: "username", "data-autofocus": true, pattern: "[A-Za-z0-9_]{3,24}", title: "3–24 letters, digits or underscores" })),
    nameRow,
    h("label", { class: "field" }, "Password", h("input", { name: "password", type: "password", required: true, minlength: 8, autocomplete: "current-password" })),
    err,
    submit,
  );
  const tabs = h(
    "div",
    { class: "tabs", role: "tablist" },
    h("button", { class: "tab active", type: "button", role: "tab", onclick: () => setMode("login") }, "Sign in"),
    h("button", { class: "tab", type: "button", role: "tab", onclick: () => setMode("register") }, "New player"),
  );
  function setMode(m: typeof mode) {
    mode = m;
    tabs.children[0].classList.toggle("active", m === "login");
    tabs.children[1].classList.toggle("active", m === "register");
    nameRow.classList.toggle("hidden", m === "login");
    submit.textContent = m === "login" ? "Sign in" : "Create player";
  }
  return h("main", { class: "screen center" }, h("div", { class: "panel card-narrow" }, logo(), tabs, form));
}

/** The hub: one big PLAY, quick join, and the rest of the game around it. */
export function hubScreen(a: UiActions, s: AppState, rooms: { room_id: string; code: string; status: string }[]): HTMLElement {
  const code = h("input", { class: "code-input", maxlength: 8, placeholder: "CODE", "aria-label": "Invitation code", autocomplete: "off", spellcheck: false });
  const err = h("p", { class: "form-error", role: "alert" });
  const guard = (f: () => Promise<void>) => async () => {
    err.textContent = "";
    try {
      await f();
    } catch (x) {
      err.textContent = x instanceof Error ? x.message : String(x);
    }
  };
  const p = s.profile;
  const tile = (kind: "garage" | "shop" | "daily" | "road", ic: IconName, name: string, note: string, alert = false) =>
    h(
      "button",
      { class: "hub-tile" + (alert ? " alert" : ""), onclick: () => a.openPanel(kind) },
      h("span", { class: "hub-tile-icon" }, icon(ic)),
      h("span", { class: "hub-tile-name" }, name),
      h("span", { class: "hub-tile-note" }, note),
    );
  const roadReady = p?.road.some((m) => m.reached && !m.claimed) ?? false;
  return h(
    "main",
    { class: "screen hub" },
    h(
      "header",
      { class: "hub-top" },
      h("div", { class: "hub-brand" }, logoMark(), h("span", null, GAME_TITLE)),
      h(
        "div",
        { class: "hub-player" },
        h("span", { class: "seeds-chip", title: "Seeds — earned by playing" }, icon("sprout"), h("b", null, String(s.profile?.seeds ?? 0))),
        h("span", { class: "hub-avatar" }, (s.player?.display_name ?? "?").slice(0, 1).toUpperCase()),
        h("span", null, s.player?.display_name ?? ""),
        h("button", { class: "icon-btn", title: "Tutorial", "aria-label": "Tutorial", onclick: () => void a.tutorial() }, icon("cap")),
        h("button", { class: "icon-btn", title: "How to play", "aria-label": "How to play", onclick: () => a.openHowTo() }, icon("help")),
        h("button", { class: "icon-btn", title: "Settings", "aria-label": "Settings", onclick: () => a.openSettings() }, icon("gear")),
        h("button", { class: "icon-btn", title: "Sign out", "aria-label": "Sign out", onclick: () => a.logout() }, icon("power")),
      ),
    ),
    h(
      "aside",
      { class: "hub-rail" },
      tile("garage", "boat", "Garage", "Your boats"),
      tile("shop", "shop", "Shop", "Spend your Seeds"),
      tile("daily", "calendar", "Daily", p && !p.daily.claimed_today ? `${p.daily.next} Seeds waiting!` : "Come back tomorrow", !!p && !p.daily.claimed_today),
      tile("road", "road", "River Road", roadReady ? "Reward ready!" : `${p?.runs ?? 0} runs`, roadReady),
    ),
    h(
      "section",
      { class: "hub-play" },
      riverChip(p ?? null, () => a.openPanel("rivers")),
      h("button", { class: "play-btn", "data-autofocus": true, onclick: guard(a.play) }, h("span", { class: "play-main" }, icon("play"), "PLAY"), h("span", { class: "play-sub" }, "Crew run · 2–4 players")),
      h("button", { class: "btn ghost wide", onclick: guard(a.practice) }, icon("solo"), "Practice solo"),
      h("form", { class: "join-row", onsubmit: (e: Event) => (e.preventDefault(), guard(() => a.joinRoom(code.value))()) }, code, h("button", { class: "btn", type: "submit" }, "Join")),
      err,
      rooms.length
        ? h("div", { class: "my-rooms" }, rooms.slice(0, 3).map((r) => h("button", { class: "btn ghost room-chip", onclick: () => a.openRoom(r.room_id) }, "↩ ", h("span", { class: "mono" }, r.code))))
        : null,
    ),
  );
}

function memberList(room: RoomView) {
  const seats = Array.from({ length: room.max_players }, (_, i) => room.members.find((m) => m.seat === i));
  return h(
    "ul",
    { class: "members" },
    seats.map((m, i) =>
      m
        ? h(
            "li",
            { class: "member" + (m.connected ? "" : " away") },
            h("span", { class: "seat seat-" + i }, String(i + 1)),
            h("span", { class: "name" }, m.name, m.you ? " (you)" : ""),
            m.host ? h("span", { class: "chip" }, "host") : null,
            !m.connected ? h("span", { class: "chip warn" }, "away") : null,
            h("span", { class: "chip " + (m.ready ? "good" : "") }, m.ready ? "✔ ready" : "… not ready"),
          )
        : h("li", { class: "member empty" }, h("span", { class: "seat" }, String(i + 1)), h("span", { class: "name muted" }, "Open seat")),
    ),
  );
}

export function lobbyScreen(room: RoomView, a: UiActions): HTMLElement {
  const me = room.members.find((m) => m.you);
  const host = !!me?.host;
  const present = room.members.filter((m) => m.connected);
  const allReady = present.length >= room.min_players && present.every((m) => m.ready);
  const why = present.length < room.min_players ? `Waiting for players (${present.length}/${room.min_players})` : !allReady ? "Waiting for everyone to be ready" : "";
  const link = `${location.origin}/?join=${room.code}`;
  return h(
    "main",
    { class: "screen center" },
    h(
      "section",
      { class: "panel lobby-card" },
      h("div", { class: "lobby-head" }, h("h2", null, "Your crew's dock"), h("button", { class: "icon-btn", title: "How to play", onclick: () => a.openHowTo() }, icon("help"))),
      h(
        "div",
        { class: "invite" },
        h("div", null, h("div", { class: "muted small" }, "Invitation code"), h("div", { class: "code mono" }, room.code)),
        h(
          "button",
          {
            class: "btn ghost",
            onclick: async (e: Event) => {
              try {
                await navigator.clipboard.writeText(link);
                (e.target as HTMLElement).textContent = "✔ Copied";
              } catch {
                (e.target as HTMLElement).textContent = link;
              }
            },
          },
          "Copy link",
        ),
      ),
      memberList(room),
      host
        ? riverChip(store.state.profile, () => a.openPanel("rivers"), "The river you'll race")
        : h("p", { class: "muted small" }, "The host picks the river."),
      h(
        "div",
        { class: "lobby-buttons" },
        h("button", { class: "btn " + (me?.ready ? "" : "primary"), "data-autofocus": true, onclick: () => a.ready(!me?.ready) }, me?.ready ? "Not ready" : "I'm ready"),
        host ? h("button", { class: "btn primary big", disabled: !allReady, title: why, onclick: () => a.startRun(false) }, icon("play"), "Start the run") : null,
        host ? h("button", { class: "btn ghost", onclick: () => a.startRun(true) }, "Practice solo") : null,
      ),
      h("p", { class: "muted small", role: "status" }, why || (host ? "Everyone's ready — start when you like." : "The host starts the run.")),
      h("div", { class: "footer-links" }, h("button", { class: "link", onclick: () => a.leaveRoom() }, icon("back"), "Back to hub")),
    ),
  );
}
