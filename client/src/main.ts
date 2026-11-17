import "./styles.css";
import "./river/run.css";
import "./ui/menus.css";
import "./ui/theme.css";
import "./ui/mobile.css";
import { api, auth, ApiError } from "./net/api";
import { RoomSocket } from "./net/socket";
import { store } from "./store";
import type { Envelope, RoomView } from "./types";
import { createEngine, detectQuality, loadQuality, perf, qualitySettings, saveQuality, type EngineInfo, type HardwareGuess, type Level, type Quality } from "./render/engine";
import { clear, h, ModalStack, toast } from "./ui/dom";
import { authScreen, hubScreen, lobbyScreen, titleScreen, type UiActions } from "./ui/screens";
import { icon } from "./ui/icons";
import { howToPlay, settings } from "./ui/panels";
import { MenuBackdrop } from "./river/backdrop";
import { RunMode } from "./river/mode";
import { hubPanel, loadProfile, PANEL_TITLES } from "./ui/hub";
import { selectedRiver } from "./ui/rivers";
import { sound } from "./audio/sound";

const uiRoot = document.getElementById("ui")!;
const canvas = document.getElementById("scene") as HTMLCanvasElement;
const modals = new ModalStack(document.getElementById("modals")!);
const banner = document.getElementById("banner")!;

let engineInfo: EngineInfo;
let quality: Quality = loadQuality();
/** What "auto" picked on this device, and why. */
let guess: HardwareGuess | null = null;
let backdrop: MenuBackdrop | null = null;
let runMode: RunMode | null = null;
let runNames: Record<string, string> = {};
let socket: RoomSocket | null = null;
let myRooms: { room_id: string; code: string; status: string }[] = [];

sound.install();
sound.music("hub");
let started = false;
let authError: string | null = null;
let pendingPractice = false;
let pendingEndless = false;
let pendingTutorial = false;
let runWasTutorial = false;
const TUTORIAL_KEY = "nt.tutorialDone";
const tutorialDone = () => {
  try {
    return localStorage.getItem(TUTORIAL_KEY) === "1";
  } catch {
    return false;
  }
};
const markTutorialDone = () => {
  try {
    localStorage.setItem(TUTORIAL_KEY, "1");
  } catch {
    /* ignore */
  }
};

// ── server messages ──────────────────────────────────────────────────────

function onMessage(m: Envelope) {
  const p = m.payload ?? {};
  if (m.type.startsWith("run.")) return handleRun(m.type, p);
  if (p.state) store.set({ room: p.state as RoomView });
  if (m.type === "room.joined" && pendingEndless) {
    pendingEndless = false;
    send("run.start", { endless: true, river: selectedRiver() });
  }
  if (m.type === "room.joined" && pendingPractice) {
    pendingPractice = false;
    send("run.start", { practice: true, river: selectedRiver() });
  }
  if (m.type === "room.joined" && pendingTutorial) {
    pendingTutorial = false;
    send("run.start", { tutorial: true });
  }
  for (const ev of (p.events ?? []) as any[]) {
    if (ev.event === "room.member_joined") toast(`${ev.name} joined the crew.`, "good");
    if (ev.event === "room.member_left") toast("A player left the room.");
  }
  if (m.type === "error") toast(p.message ?? "That did not work.", "error");
  if (m.type === "room.expired") {
    toast(p.message, "error", 8000);
    leaveToHub();
  }
}

function handleRun(type: string, p: any) {
  if (type === "run.started") {
    runNames = p.names ?? {};
    runWasTutorial = !!p.tutorial;
    openRun();
    runMode?.setSkins(p.skins);
  } else if (type === "run.snapshot") {
    if (!runMode) openRun();
    runMode?.setSkins(p.skins);
    runMode?.onSnapshot(p.snapshot);
  } else if (type === "run.rewards") {
    runMode?.rewards(p.seeds ?? 0, !!p.practice);
    if (p.river) runMode?.riverResult(p.river);
    if (p.missions || p.rank) runMode?.runSummary(p.missions ?? [], p.rank ?? null);
    void loadProfile();
  } else if (type === "run.events") runMode?.onEvents(p.events ?? []);
}

function openRun() {
  if (runMode) return;
  const room = store.state.room;
  const names = Object.keys(runNames).length ? runNames : Object.fromEntries((room?.members ?? []).map((x) => [String(x.seat), x.name]));
  modals.close("howto");
  backdrop?.dispose();
  backdrop = null;
  uiRoot.classList.add("hidden");
  sound.music("race");
  sound.river(true);
  sound.preload(["pickup", "hop", "cut", "capsize", "countdown", "go"]);
  runMode = new RunMode(
    engineInfo.engine,
    room?.my_seat ?? 0,
    names,
    { input: (i) => void socket?.sendRaw("run.input", i), choose: (option) => send("run.choose", { option }), pause: (paused) => send("run.pause", { paused }) },
    closeRun,
    quitRun,
    !!room?.members.find((x) => x.you)?.host,
  );
}

/** Quit a run early: leave the room and go back to the hub. The others keep playing. */
function quitRun() {
  send("room.leave");
  setTimeout(leaveToHub, 150);
}

function closeRun() {
  if (runWasTutorial) {
    runWasTutorial = false;
    markTutorialDone();
    send("room.leave");
    setTimeout(leaveToHub, 150);
  }
  void loadProfile();
  runMode?.dispose();
  runMode = null;
  sound.river(false);
  sound.music("hub");
  backdrop = new MenuBackdrop(engineInfo.engine);
  uiRoot.classList.remove("hidden");
  render(true);
}

// ── actions ──────────────────────────────────────────────────────────────

function send(type: string, payload: Record<string, unknown> = {}) {
  if (!socket || !socket.send(type, payload)) toast("Not connected to the room yet — reconnecting…", "error");
}

function enterRoom(roomId: string) {
  socket?.close();
  store.set({ room: null });
  socket = new RoomSocket(roomId, auth.token!, {
    onMessage,
    onStatus: (s, detail) => {
      store.set({ socket: s, socketDetail: detail ?? null });
      if (s === "closed" && detail) {
        toast(detail, "error", 7000);
        leaveToHub();
      }
    },
  });
  socket.connect();
  try {
    history.replaceState(null, "", `/?room=${roomId}`);
  } catch {
    /* ignore */
  }
}

function leaveToHub() {
  socket?.close();
  socket = null;
  if (runMode) closeRun();
  store.set({ room: null, socket: "closed" });
  try {
    history.replaceState(null, "", "/");
  } catch {
    /* ignore */
  }
  void refreshRooms();
}

async function refreshRooms() {
  if (!auth.token) return;
  try {
    myRooms = (await api.myRooms()).rooms;
  } catch {
    myRooms = [];
  }
  render(true);
}

const ui: UiActions = {
  async login(handle, pw) {
    const r = await api.login(handle, pw);
    auth.set(r.token);
    store.set({ player: r.player });
    await afterLogin();
  },
  async register(handle, name, pw) {
    const r = await api.register(handle, name, pw);
    auth.set(r.token);
    store.set({ player: r.player });
    await afterLogin();
  },
  logout() {
    api.logout().catch(() => undefined);
    auth.set(null);
    socket?.close();
    store.set({ player: null, room: null });
  },
  async play() {
    const r = await api.createRoom();
    enterRoom(r.room_id);
  },
  async practice() {
    const r = await api.createRoom();
    pendingPractice = true;
    enterRoom(r.room_id);
  },
  async endless() {
    const r = await api.createRoom();
    pendingEndless = true;
    enterRoom(r.room_id);
  },
  async joinRoom(code) {
    const r = await api.joinRoom(code.trim());
    enterRoom(r.room_id);
  },
  openRoom: (id) => enterRoom(id),
  async tutorial() {
    modals.close("welcome");
    const r = await api.createRoom();
    pendingTutorial = true;
    enterRoom(r.room_id);
  },
  leaveRoom() {
    send("room.leave");
    setTimeout(leaveToHub, 150);
  },
  ready: (v) => send("room.ready", { ready: v }),
  startRun: (practice) => send("run.start", { practice, river: selectedRiver() }),
  startEndless: () => send("run.start", { endless: true, river: selectedRiver() }),
  openHowTo: () => modals.push("howto", "How to play", howToPlay()),
  openPanel: (kind) => modals.push("panel-" + kind, PANEL_TITLES[kind], hubPanel(kind), { wide: true, onClose: () => render(true) }),
  openSettings: () =>
    modals.push(
      "settings",
      "Settings",
      settings(
        quality,
        engineInfo.backend,
        engineInfo.reason,
        (q) => {
          quality = q;
          saveQuality(q);
          location.reload();
        },
        guess ? `${guess.level === "low" ? "Low" : "Medium"} here: ${guess.why}` : "",
      ),
    ),
};

async function afterLogin() {
  authError = null;
  void loadProfile();
  if (!tutorialDone() && !new URLSearchParams(location.search).has("room")) offerTutorial();
  await refreshRooms();
  const params = new URLSearchParams(location.search);
  const join = params.get("join");
  const room = params.get("room");
  if (join) {
    try {
      await ui.joinRoom(join);
    } catch (e) {
      toast(e instanceof Error ? e.message : String(e), "error");
    }
  } else if (room && myRooms.some((r) => r.room_id === room)) enterRoom(room);
}

function offerTutorial() {
  modals.push(
    "welcome",
    "Welcome aboard!",
    h(
      "div",
      { class: "welcome" },
      h("p", { class: "lede" }, "New to the river? Take the 60-second tutorial — it teaches every control by doing."),
      h(
        "div",
        { class: "row" },
        h("button", { class: "btn primary big", "data-autofocus": true, onclick: () => void ui.tutorial() }, icon("cap"), "Quick tutorial"),
        h(
          "button",
          {
            class: "btn ghost",
            onclick: () => {
              markTutorialDone();
              modals.close("welcome");
            },
          },
          "Skip",
        ),
      ),
    ),
  );
}

// ── DOM ──────────────────────────────────────────────────────────────────

let lastKey = "";
function render(force = false) {
  const s = store.state;
  const key = JSON.stringify([started, s.player?.id, s.room?.version, s.room?.phase, myRooms.length, s.profile?.seeds, s.profile?.daily.claimed_today, s.profile?.runs]);
  if (!force && key === lastKey) return;
  lastKey = key;
  clear(uiRoot);
  if (!started) {
    uiRoot.append(
      titleScreen(() => {
        if (started) return;
        started = true;
        render(true);
      }),
    );
  } else if (!s.player) uiRoot.append(authScreen(ui, authError));
  else if (!s.room) uiRoot.append(hubScreen(ui, s, myRooms));
  else uiRoot.append(lobbyScreen(s.room, ui));
  uiRoot.querySelector<HTMLElement>("[data-autofocus]")?.focus({ preventScroll: true });
}

function renderBanner() {
  const s = store.state;
  banner.hidden = !(s.room && (s.socket === "reconnecting" || s.socket === "connecting"));
  banner.textContent = s.socket === "connecting" ? "Connecting…" : `Connection lost — ${s.socketDetail ?? "reconnecting…"}`;
}

// ── frame-rate watch (Auto quality) ─────────────────────────────────────────

/**
 * In Auto quality, measure the frame rate during runs. If it stays under 40 fps
 * for a 4-second window, render at a lower resolution (up to half), without a
 * reload. The first window of each run is skipped (models and shaders load then).
 */
function watchFrameRate() {
  const engine = engineInfo.engine;
  let start = 0;
  let frames = 0;
  let windows = 0;
  let told = false;
  engine.onEndFrameObservable.add(() => {
    if (!runMode || document.hidden) {
      start = 0;
      windows = 0;
      return;
    }
    const now = performance.now();
    if (!start) {
      start = now;
      frames = 0;
      return;
    }
    frames++;
    if (now - start < 4000) return;
    const fps = (frames * 1000) / (now - start);
    start = now;
    frames = 0;
    if (windows++ === 0) return; // warm-up window
    const scale = engine.getHardwareScalingLevel();
    if (fps < 40 && scale < 2) {
      engine.setHardwareScalingLevel(Math.min(2, scale + 0.25));
      perf.low = true;
      if (!told) {
        told = true;
        toast("Graphics lowered automatically for a smoother game.");
      }
    }
  });
}

// ── boot ─────────────────────────────────────────────────────────────────

async function boot() {
  // Graphics level: the player's choice, or "auto" from the hardware.
  guess = await detectQuality();
  const level: Level = quality === "auto" ? guess.level : quality;
  perf.low = level === "low";
  engineInfo = await createEngine(canvas, true, !perf.low);
  engineInfo.engine.setHardwareScalingLevel(qualitySettings(level).scaling);
  if (quality === "auto") watchFrameRate();
  window.addEventListener("resize", () => engineInfo.engine.resize());
  backdrop = new MenuBackdrop(engineInfo.engine);
  (window as any).__nt = { store, engineInfo };
  document.getElementById("renderer")!.textContent = engineInfo.backend === "webgpu" ? "WebGPU" : `WebGL2${engineInfo.reason ? " · " + engineInfo.reason : ""}`;
  store.subscribe(() => {
    render();
    renderBanner();
  });
  const params = new URLSearchParams(location.search);
  if (params.has("room") || params.has("join")) started = true; // returning players skip the title
  render(true);
  if (auth.token) {
    try {
      store.set({ player: await api.me() });
      await afterLogin();
    } catch (e) {
      if (e instanceof ApiError && e.code === "offline") {
        uiRoot.replaceChildren(h("main", { class: "screen center" }, h("div", { class: "panel card-narrow" }, h("h2", null, "Can't reach the game server"), h("p", null, e.message), h("button", { class: "btn primary", onclick: () => location.reload() }, "Try again"))));
        return;
      }
      if (e instanceof ApiError && e.status !== 401) authError = e.message;
      store.set({ player: null });
    }
  }
  render(true);
}

void boot();
