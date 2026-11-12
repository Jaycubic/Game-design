// Game audio on Web Audio: short effects, a gapless river ambience and music
// loops, each with its own volume (saved per browser). Browsers only allow
// sound after a tap or key press, so the context starts on the first gesture.
// A missing file just stays silent, so new sounds can be added any time.

const BASE = `${import.meta.env.BASE_URL}audio/`;

/** Effect name → number of variants (sfx/<name>-<n>.mp3, n = 1…). One is picked at random. */
const SFX = {
  pickup: 3,
  hop: 3,
  capsize: 2,
  wave: 1,
  cut: 3,
  slowed: 1,
  bump: 1,
  boost: 1,
  plug: 1,
  plugFail: 1,
  ping: 1,
  click: 1,
  countdown: 1,
  go: 1,
  rule: 1,
  win: 1,
  lose: 1,
} as const;
export type Sfx = keyof typeof SFX;

/** Looping tracks: music/<name>.mp3 and the river ambience. */
export type Track = "hub" | "race";

interface Prefs {
  master: number;
  music: number;
  sfx: number;
  muted: boolean;
}
const PREFS_KEY = "nt.audio";
const DEFAULTS: Prefs = { master: 0.8, music: 0.45, sfx: 0.8, muted: false };

function loadPrefs(): Prefs {
  try {
    return { ...DEFAULTS, ...JSON.parse(localStorage.getItem(PREFS_KEY) ?? "{}") };
  } catch {
    return { ...DEFAULTS };
  }
}

class SoundSystem {
  private ctx: AudioContext | null = null;
  private master!: GainNode;
  private musicBus!: GainNode;
  private sfxBus!: GainNode;
  private buffers = new Map<string, Promise<AudioBuffer | null>>();
  private lastPlayed = new Map<string, number>();
  private wantedTrack: Track | null = null;
  private playingTrack: { name: Track; src: AudioBufferSourceNode; gain: GainNode } | null = null;
  private wantAmbience = false;
  private ambience: { src: AudioBufferSourceNode; gain: GainNode } | null = null;
  prefs: Prefs = loadPrefs();

  /** Call once at start-up: unlocks audio on the first gesture and clicks on buttons. */
  install() {
    const unlock = () => {
      this.ensure();
      void this.ctx?.resume();
    };
    for (const ev of ["pointerdown", "keydown", "touchend"]) window.addEventListener(ev, unlock, { capture: true, passive: true });
    // Every menu button gets a soft click.
    window.addEventListener(
      "pointerdown",
      (e) => {
        const t = (e.target as HTMLElement | null)?.closest?.("button, .hub-tile, [role=tab]");
        if (t && !t.classList.contains("tc-btn") && !(t as HTMLButtonElement).disabled) this.play("click", { volume: 0.5 });
      },
      { capture: true, passive: true },
    );
    // Pause everything while the tab is hidden (phones: app switcher, screen lock).
    document.addEventListener("visibilitychange", () => {
      if (!this.ctx) return;
      if (document.hidden) void this.ctx.suspend();
      else void this.ctx.resume();
    });
  }

  private ensure(): AudioContext | null {
    if (this.ctx) return this.ctx;
    const AC = window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!AC) return null;
    this.ctx = new AC();
    this.master = this.ctx.createGain();
    this.musicBus = this.ctx.createGain();
    this.sfxBus = this.ctx.createGain();
    this.musicBus.connect(this.master);
    this.sfxBus.connect(this.master);
    this.master.connect(this.ctx.destination);
    this.applyPrefs();
    // Start whatever was requested before the first gesture.
    if (this.wantedTrack) this.music(this.wantedTrack);
    if (this.wantAmbience) this.river(true);
    return this.ctx;
  }

  private load(path: string): Promise<AudioBuffer | null> {
    let p = this.buffers.get(path);
    if (!p) {
      p = fetch(BASE + path)
        .then((r) => (r.ok ? r.arrayBuffer() : Promise.reject(new Error(String(r.status)))))
        .then((b) => this.ctx!.decodeAudioData(b))
        .catch(() => null);
      this.buffers.set(path, p);
    }
    return p;
  }

  /** Warm the cache so the first pickup isn't late. */
  preload(names: Sfx[]) {
    if (!this.ensure()) return;
    for (const n of names) for (let i = 1; i <= SFX[n]; i++) void this.load(`sfx/${n}-${i}.mp3`);
  }

  play(name: Sfx, opts: { volume?: number; rate?: number } = {}) {
    const ctx = this.ctx;
    if (!ctx || this.prefs.muted || ctx.state !== "running") return;
    const now = performance.now();
    if (now - (this.lastPlayed.get(name) ?? 0) < 45) return; // no machine-gun stacking
    this.lastPlayed.set(name, now);
    const n = 1 + Math.floor(Math.random() * SFX[name]);
    void this.load(`sfx/${name}-${n}.mp3`).then((buf) => {
      if (!buf) return;
      const src = ctx.createBufferSource();
      src.buffer = buf;
      src.playbackRate.value = (opts.rate ?? 1) * (0.94 + Math.random() * 0.12); // small pitch variety
      const g = ctx.createGain();
      g.gain.value = opts.volume ?? 1;
      src.connect(g).connect(this.sfxBus);
      src.start();
    });
  }

  /** Switch music with a short crossfade; null fades it out. */
  music(name: Track | null) {
    this.wantedTrack = name;
    const ctx = this.ctx;
    if (!ctx) return;
    if (this.playingTrack?.name === name) return;
    const old = this.playingTrack;
    this.playingTrack = null;
    if (old) {
      old.gain.gain.setTargetAtTime(0, ctx.currentTime, 0.4);
      old.src.stop(ctx.currentTime + 2);
    }
    if (!name) return;
    void this.load(`music/${name}.mp3`).then((buf) => {
      if (!buf || this.wantedTrack !== name || this.playingTrack) return;
      const src = ctx.createBufferSource();
      src.buffer = buf;
      src.loop = true;
      const g = ctx.createGain();
      g.gain.value = 0;
      g.gain.setTargetAtTime(1, ctx.currentTime, 0.6);
      src.connect(g).connect(this.musicBus);
      src.start();
      this.playingTrack = { name, src, gain: g };
    });
  }

  /** The river under the boats, during runs. */
  river(on: boolean) {
    this.wantAmbience = on;
    const ctx = this.ctx;
    if (!ctx) return;
    if (!on) {
      const a = this.ambience;
      this.ambience = null;
      if (a) {
        a.gain.gain.setTargetAtTime(0, ctx.currentTime, 0.3);
        a.src.stop(ctx.currentTime + 1.5);
      }
      return;
    }
    if (this.ambience) return;
    void this.load("sfx/river-loop.mp3").then((buf) => {
      if (!buf || !this.wantAmbience || this.ambience) return;
      const src = ctx.createBufferSource();
      src.buffer = buf;
      src.loop = true;
      const g = ctx.createGain();
      g.gain.value = 0;
      g.gain.setTargetAtTime(0.55, ctx.currentTime, 0.8);
      src.connect(g).connect(this.sfxBus);
      src.start();
      this.ambience = { src, gain: g };
    });
  }

  set(p: Partial<Prefs>) {
    this.prefs = { ...this.prefs, ...p };
    try {
      localStorage.setItem(PREFS_KEY, JSON.stringify(this.prefs));
    } catch {
      /* session only */
    }
    this.applyPrefs();
  }

  private applyPrefs() {
    if (!this.ctx) return;
    const t = this.ctx.currentTime;
    this.master.gain.setTargetAtTime(this.prefs.muted ? 0 : this.prefs.master, t, 0.05);
    this.musicBus.gain.setTargetAtTime(this.prefs.music, t, 0.05);
    this.sfxBus.gain.setTargetAtTime(this.prefs.sfx, t, 0.05);
  }
}

export const sound = new SoundSystem();
