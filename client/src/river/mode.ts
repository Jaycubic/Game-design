import "./run.css";
// Glue for a river run: input → server, snapshots → smoothed 3D poses, events → juice.

import { AbstractEngine, Color4, Vector3 } from "@babylonjs/core";
import { RiverScene, type BoatPose } from "./scene";
import { RunHud } from "./hud";
import { Coach } from "./coach";
import { GamepadInput, intentForKey, type Intent } from "../input/controls";
import { TouchControls, actionVerb, usesTouch } from "../input/touch";
import { sound } from "../audio/sound";
import { feel } from "../ui/feel";
import type { RunEvent, RunResults, Snapshot } from "./types";
import type { MissionView, RankView, RiverResult } from "../net/api";

const RIVER_TITLES: Record<string, string> = { canal: "🏘️ Town Canal", mangrove: "🌿 Mangrove Maze", gorge: "⛰️ Rapids Gorge" };

export interface RunSender {
  input(i: { steer?: number; hop?: boolean; action?: boolean; glyph?: number }): void;
  choose(option: number): void;
}

interface Smooth {
  x: number;
  z: number;
}

const HOP_TIME = 0.75;
const CAPSIZE_TIME = 1.2;
const HIT_STOP_MS = 90;

export class RunMode {
  private scene: RiverScene;
  private hud: RunHud;
  private snap: Snapshot | null = null;
  private snapAt = 0;
  private smooth = new Map<number, Smooth>();
  private predictedLane: number | null = null;
  private predictedUntil = 0;
  private keyHandler: (e: KeyboardEvent) => void;
  private finished = false;
  private last = performance.now();
  private pad = new GamepadInput();
  private coach: Coach | null = null;
  private touch: TouchControls | null = null;
  private lastBeat = "";
  /** Client-only freeze-frame after your own capsize; the server never pauses. */
  private freezeUntil = 0;

  constructor(
    private engine: AbstractEngine,
    private mySeat: number,
    names: Record<string, string>,
    private send: RunSender,
    private onLeave: () => void,
    private isHost: boolean,
  ) {
    this.scene = new RiverScene(engine);
    this.hud = new RunHud(document.getElementById("run-ui")!, mySeat, names, {
      choose: (o) => send.choose(o),
      glyph: (g) => send.input({ glyph: g }),
      leave: () => this.onLeave(),
    });
    this.keyHandler = (e) => this.onKey(e);
    window.addEventListener("keydown", this.keyHandler);
    if (usesTouch()) this.touch = new TouchControls(document.getElementById("run-ui")!, document.getElementById("scene")!, (it) => this.apply(it));
    engine.runRenderLoop(() => this.frame());
  }

  private onKey(e: KeyboardEvent) {
    if (e.repeat || (e.target as HTMLElement).closest?.("input, textarea")) return;
    const s = this.snap;
    if (!s) return;
    const it = intentForKey(e.key, s.phase.phase === "choosing");
    if (!it) return;
    e.preventDefault();
    this.apply(it);
  }

  private apply(it: Intent) {
    switch (it.type) {
      case "choose":
        this.send.choose(it.option);
        break;
      case "steer":
        this.steer(it.dir);
        break;
      case "hop":
        this.send.input({ hop: true });
        sound.play("hop", { volume: 0.7 });
        break;
      case "action":
        this.send.input({ action: true });
        break;
      case "glyph":
        this.send.input({ glyph: it.g });
        sound.play("ping", { volume: 0.6 });
        break;
    }
    this.coach?.input(it);
  }

  private steer(dir: number) {
    const me = this.snap?.boats.find((b) => b.seat === this.mySeat);
    if (!me || !this.snap) return;
    const lanes = this.snap.lanes;
    const base = this.predictedLane ?? me.lane;
    // Server's left is -x; the chase camera looks down +z, so screen-left is world -x.
    this.predictedLane = Math.max(lanes[0], Math.min(lanes[lanes.length - 1], base + dir));
    this.predictedUntil = performance.now() + 350;
    this.send.input({ steer: dir });
  }

  setSkins(skins: Record<string, string> | undefined) {
    if (skins) this.scene.setSkins(skins);
  }

  /** Stars and best score for the river, sent with the rewards after a run. */
  riverResult(r: RiverResult) {
    if (r.new_stars.some(Boolean)) sound.play("win", { volume: 0.6 });
    this.hud.riverResult(r);
  }

  /** Missions and rank after a run, under the stars on the results card. */
  runSummary(missions: MissionView[], rank: RankView | null) {
    if (missions.some((m) => m.just_done) || (rank?.change ?? 0) > 0) sound.play("boost", { volume: 0.6 });
    this.hud.runSummary(missions, rank);
  }

  rewards(seeds: number, practice: boolean) {
    this.hud.banner(`+${seeds} Seeds${practice ? " (practice)" : ""} 🌱`, "good");
  }

  onSnapshot(s: Snapshot) {
    const first = !this.snap;
    this.snap = s;
    this.snapAt = performance.now();
    this.scene.setHealth(s.health);
    this.scene.setLanes(s.lane_count ?? 3);
    if (s.river) this.scene.setRiver(s.river);
    if (first && s.river && s.river !== "canal" && !s.tutorial) this.hud.banner(RIVER_TITLES[s.river], "go");
    this.beats(s);
    if (first) for (const b of s.boats) this.smooth.set(b.seat, { x: this.scene.laneX(b.lane), z: b.z });
    const me = s.boats.find((b) => b.seat === this.mySeat);
    if (me && this.predictedLane !== null && (me.lane === this.predictedLane || performance.now() > this.predictedUntil)) this.predictedLane = null;
    if (s.tutorial && !this.coach) this.coach = new Coach(document.getElementById("run-ui")!, () => this.onLeave());
    this.coach?.snapshot(s, this.mySeat);
    if (!this.finished) this.hud.update(s);
    if (s.results && !this.finished && !s.tutorial) this.finish(s.results);
  }

  onEvents(events: RunEvent[]) {
    const s = this.snap;
    if (!s) return;
    this.coach?.events(events, this.mySeat);
    for (const e of events) {
      const seat = "seat" in e ? e.seat : -1;
      const pose = this.smooth.get(seat);
      const scr = pose ? this.scene.worldToScreen(new Vector3(pose.x, 1.2, pose.z)) : null;
      const mine = seat === this.mySeat;
      switch (e.type) {
        case "collected":
          if (mine) sound.play("pickup", { volume: 0.8 });
          if (pose) this.scene.burst(pose.x, pose.z + 0.8, mine ? 22 : 10);
          if (mine && scr?.visible) this.hud.popup(scr.x, scr.y - 30, s.twist === "volunteer_day" ? "+2 ♻" : "+1 ♻");
          break;
        case "cut":
          sound.play("cut", { volume: mine ? 1 : 0.4 });
          if (pose) this.scene.burst(pose.x, pose.z + 2, 30, new Color4(0.6, 0.9, 0.5, 0.9));
          if (mine && scr?.visible) this.hud.popup(scr.x, scr.y - 40, "Bank greened +5 🌿", "green");
          break;
        case "capsized":
          sound.play("capsize", { volume: mine ? 1 : 0.45 });
          if (pose) this.scene.burst(pose.x, pose.z, mine ? 60 : 30);
          if (mine) {
            this.freezeUntil = performance.now() + HIT_STOP_MS;
            this.scene.kick(0.75);
            this.hud.hit(feel.prefs.reduceFlash);
            if (e.dropped && scr?.visible) this.hud.popup(scr.x, scr.y - 30, `−${e.dropped} ♻`, "bad");
            this.hud.banner(s.endless ? "Capsized — run over!" : e.dropped ? "Capsized! Net your plastic back" : "Capsized!", "bad");
          }
          break;
        case "slowed":
          if (mine) sound.play("slowed");
          if (mine) this.hud.banner(`Tangled in weed — ${actionVerb()} to cut`, "warn");
          break;
        case "boost":
          if (mine) sound.play("boost");
          if (mine) this.hud.banner("BOOST!", "good");
          break;
        case "plugged":
          sound.play("plug", { volume: mine ? 1 : 0.6 });
          if (pose) this.scene.burst(pose.x * 1.3, pose.z, 50, new Color4(0.5, 1, 0.7, 1));
          this.hud.banner(mine ? "Pipe plugged! Score ×1.5 more" : "A pipe was plugged — the river breathes", "good");
          if (mine) this.scene.kick(0.25);
          break;
        case "plug_failed":
          if (mine) sound.play("plugFail");
          if (mine) this.hud.banner("Wrong code — the pipe is still pouring", "bad");
          break;
        case "ping":
          if (!mine) sound.play("ping", { volume: 0.5 });
          if (scr?.visible) this.hud.bubble(scr.x, scr.y - 60, e.glyph, e.seat);
          break;
        case "wave_incoming":
          sound.play("wave");
          this.hud.banner("🌊 Debris wave ahead — find the gap!", "warn");
          break;
        case "choked":
          sound.play("lose", { volume: 0.7 });
          this.hud.banner("The river is choking — everyone's score will be halved", "bad");
          break;
        case "rule_changed":
          sound.play("rule");
          break;
        case "net_hit":
          sound.play("cut", { volume: mine ? 1 : 0.4 });
          if (pose) this.scene.burst(pose.x, pose.z + 2, 20, new Color4(0.9, 0.6, 0.3, 0.9));
          if (mine && e.left > 0 && scr?.visible) this.hud.popup(scr.x, scr.y - 40, "Cut once more!", "green");
          break;
        case "net_freed":
          sound.play("plug", { volume: mine ? 0.8 : 0.5 });
          if (pose) this.scene.burst(pose.x, pose.z + 3, 50, new Color4(0.5, 1, 0.7, 1));
          if (scr?.visible) this.hud.popup(scr.x, scr.y - 40, mine ? "Net cut free! Grab the plastic ♻" : "Ghost net freed", "green");
          break;
        case "pushed":
          if (mine) {
            sound.play("wave", { volume: 0.45 });
            this.scene.kick(0.2);
            if (scr?.visible) this.hud.popup(scr.x, scr.y - 30, e.dir > 0 ? "Current ▶" : "◀ Current");
          }
          break;
        case "bumped":
          if (mine) sound.play("bump", { volume: 0.7 });
          if (mine) this.scene.kick(0.15);
          break;
      }
    }
  }

  /** Countdown ticks and "go", once per change. */
  private beats(s: Snapshot) {
    const p = s.phase;
    const beat = p.phase === "countdown" ? `c${Math.ceil(p.left)}` : p.phase === "warning" ? `w${Math.ceil(p.left)}` : p.phase;
    if (beat === this.lastBeat) return;
    const prev = this.lastBeat;
    this.lastBeat = beat;
    if (p.phase === "countdown" || (p.phase === "warning" && p.left <= 3.5)) sound.play("countdown");
    else if (p.phase === "running" && prev.startsWith("c")) sound.play("go");
  }

  private finish(r: RunResults) {
    this.finished = true;
    const me = this.snap?.boats.find((b) => b.seat === this.mySeat);
    const won = r.winner !== null ? r.winner === me?.crew : r.revived;
    sound.play(won ? "win" : "lose");
    this.hud.results(r, this.snap, this.isHost);
  }

  private frame() {
    const now = performance.now();
    const dt = Math.min(0.1, (now - this.last) / 1000);
    this.last = now;
    const s = this.snap;
    if (!s) {
      this.scene.scene.render();
      return;
    }
    for (const it of this.pad.poll(s.phase.phase === "choosing")) this.apply(it);
    if (now < this.freezeUntil) {
      // Hit-stop: hold the picture for a moment; input still goes to the server.
      this.scene.scene.render();
      return;
    }
    this.scene.shakeScale = feel.prefs.shake;
    const paused = ["countdown", "choosing", "reveal", "finished"].includes(s.phase.phase);
    const since = paused ? 0 : Math.min(0.25, (now - this.snapAt) / 1000);
    const poses: BoatPose[] = [];
    let me: BoatPose | null = null;
    for (const b of s.boats) {
      const lane = b.seat === this.mySeat && this.predictedLane !== null ? this.predictedLane : b.lane;
      const targetX = this.scene.laneX(lane);
      const targetZ = b.z + b.speed * since;
      const sm = this.smooth.get(b.seat) ?? { x: targetX, z: targetZ };
      sm.x += (targetX - sm.x) * Math.min(1, dt * 12);
      sm.z += (targetZ - sm.z) * Math.min(1, dt * 15);
      this.smooth.set(b.seat, sm);
      const hopLeft = Math.max(0, b.hop - since);
      const y = hopLeft > 0 ? Math.sin(Math.PI * (1 - hopLeft / HOP_TIME)) * 1.1 : 0;
      const capLeft = Math.max(0, b.capsize - since);
      // Capsize: the boat rolls all the way over and rights itself.
      const cp = capLeft > 0 ? 1 - capLeft / CAPSIZE_TIME : 0;
      const roll = capLeft > 0 ? Math.PI * 2 * cp * cp * (3 - 2 * cp) : (targetX - sm.x) * -0.12;
      const ghost = capLeft <= 0 && Math.max(0, (b.immune ?? 0) - since) > 0 && !feel.prefs.reduceFlash;
      const pose: BoatPose = { seat: b.seat, x: sm.x, y, z: sm.z, roll, yaw: (targetX - sm.x) * 0.08, boosting: b.boosting > 0, ghost };
      poses.push(pose);
      if (b.seat === this.mySeat) me = pose;
    }
    this.scene.frame(dt, poses, me, s.lanes.length);
    const mine = s.boats.find((b) => b.seat === this.mySeat);
    const viewer = mine && me ? { lane: this.predictedLane ?? mine.lane, z: me.z, speed: mine.speed || 14 } : null;
    this.scene.renderObjects(s.objs, s.lanes, viewer);
    this.scene.scene.render();
    for (const p of poses) {
      const scr = this.scene.worldToScreen(new Vector3(p.x, p.y + 1.6, p.z));
      this.hud.tag(p.seat, scr.x, scr.y, scr.visible && !(p.seat === this.mySeat && poses.length === 1));
    }
  }

  dispose() {
    window.removeEventListener("keydown", this.keyHandler);
    this.engine.stopRenderLoop();
    this.hud.dispose();
    this.coach?.dispose();
    this.touch?.dispose();
    this.scene.dispose();
  }
}
