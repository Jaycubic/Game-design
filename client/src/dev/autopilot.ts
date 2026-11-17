// Dev builds only: a simple autopilot for multiplayer play-tests. It steers to the
// next open pipe, opens the lock, types the keys (its own from memory), answers
// teammates' calls with its key, votes a set option, and samples the layout audit
// every second. Start it with `__autopilot(vote)`; read `__report()`.
import { audit } from "./layout-audit";

interface Boat { seat: number; lane: number; z: number; plugging?: boolean }
interface Obj { kind: string; lane: number; z: number; plugged: boolean; alive: boolean }
interface Plug { pipe: number; team?: boolean; slots?: [number, boolean, number | null][]; typed?: number }
interface Snap {
  t: number;
  phase: { phase: string; change?: string };
  boats: Boat[];
  objs: Obj[];
  lanes: number[];
  plugs: [number, Plug][];
  my_key?: number | null;
  votes: [number, number][];
  twist: string | null;
  finale: string | null;
}
interface Run {
  snap: Snap | null;
  mySeat: number;
  send: { input(i: object): void; choose(o: number): void };
  steer(dir: number): void;
}

export function autopilot(getRun: () => unknown, vote: number) {
  const log: string[] = [];
  const overlaps = new Set<string>();
  let key: number | null = null;
  let voted = "";
  let lastSent = 0;
  let lastAudit = 0;
  const say = (m: string) => log.push(`${Math.round(performance.now() / 1000)}s ${m}`);
  const timer = setInterval(() => {
    const run = getRun() as Run | null;
    const s = run?.snap;
    if (!run || !s) return;
    const now = performance.now();
    if (now - lastAudit > 1000) {
      lastAudit = now;
      const a = audit();
      for (const o of [...a.overlaps, ...a.offscreen]) if (!overlaps.has(o)) { overlaps.add(o); say(`LAYOUT ${a.size}: ${o}`); }
    }
    if (s.my_key !== null && s.my_key !== undefined && s.my_key !== key) { key = s.my_key; say(`my key ${key}`); }
    const me = s.boats.find((b) => b.seat === run.mySeat);
    if (!me) return;
    // Votes.
    if (s.phase.phase === "choosing" && voted !== s.phase.change && !s.votes.some(([x]) => x === run.mySeat)) {
      voted = s.phase.change ?? "";
      run.send.choose(vote);
      say(`voted ${vote} for ${voted}`);
    }
    if (s.phase.phase !== "running") return;
    if (now - lastSent < 150) return;
    // Answer a teammate's lock.
    for (const [fixer, pv] of s.plugs) {
      if (fixer === run.mySeat || !pv.team) continue;
      const mine = pv.slots?.find(([x]) => x === run.mySeat);
      if (mine && !mine[1] && key !== null) { run.send.input({ glyph: key }); lastSent = now; say(`sent key ${key} to seat ${fixer}`); return; }
    }
    // My own lock: type the slots left to right.
    const own = s.plugs.find(([x]) => x === run.mySeat)?.[1];
    if (own?.team && own.slots) {
      const i = own.typed ?? 0;
      const slot = own.slots[i];
      if (!slot) return;
      const k = slot[0] === run.mySeat ? key : slot[2];
      if (k !== null && k !== undefined) { run.send.input({ glyph: k }); lastSent = now; say(`typed slot ${i}: ${k}`); }
      return;
    }
    // Steer to the next open pipe and plug it.
    const pipe = s.objs.filter((o) => o.kind === "pipe" && !o.plugged && o.alive !== false && o.z > me.z - 3 && o.z < me.z + 80).sort((a, b) => a.z - b.z)[0];
    if (!pipe) return;
    const edge = pipe.lane < 0 ? s.lanes[0] : s.lanes[s.lanes.length - 1];
    if (me.lane !== edge) { run.steer(Math.sign(edge - me.lane)); lastSent = now; return; }
    if (pipe.z - me.z < 2.5 && !me.plugging) { run.send.input({ action: true }); lastSent = now; say(`opened lock at pipe z=${Math.round(pipe.z)}`); }
  }, 100);
  return {
    stop: () => clearInterval(timer),
    report: () => ({ log: log.slice(-40), overlaps: [...overlaps] }),
  };
}
