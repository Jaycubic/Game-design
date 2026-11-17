// Dev builds only: a simple autopilot for multiplayer play-tests. It steers to the
// next open pipe, opens the lock, types the board numbers once every number is in, sends its own
// number when dealt one, votes a set option, and samples the layout audit
// every second. Start it with `__autopilot(vote)`; read `__report()`.
import { audit } from "./layout-audit";

interface Boat { seat: number; lane: number; z: number; plugging?: boolean }
interface Obj { kind: string; lane: number; z: number; plugged: boolean; alive: boolean }
interface Tile { seat?: number; digit?: number; ready: boolean; used?: boolean }
interface Plug { pipe: number; team: boolean; tiles: Tile[] }
interface Snap {
  t: number;
  phase: { phase: string; change?: string };
  boats: Boat[];
  objs: Obj[];
  lanes: number[];
  plugs: [number, Plug][];
  my_key?: number | null;
  key_sent?: boolean;
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
    key = s.my_key ?? null;
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
    // Send my number (once) while it is shown.
    if (key !== null && !s.key_sent && !s.plugs.some(([x]) => x === run.mySeat)) {
      run.send.input({ glyph: key }); lastSent = now; say(`sent number ${key}`); return;
    }
    // My own lock: once every number is on the board, tap them in board order.
    const own = s.plugs.find(([x]) => x === run.mySeat)?.[1];
    if (own) {
      const ready = own.tiles.filter((t) => t.digit !== undefined && !t.used).map((t) => t.digit!);
      if (ready.length && own.tiles.every((t) => t.ready)) { run.send.input({ glyph: ready[0] }); lastSent = now; say(`typed ${ready[0]}`); }
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
