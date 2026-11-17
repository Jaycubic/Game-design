// Data sent by the server's river run (crates/river-core).

export type ObjKind = "plastic" | "weed" | "log" | "wave" | "pipe" | "net" | "current" | "rock";
export type River = "canal" | "mangrove" | "gorge";
export type Twist = "monsoon_surge" | "dry_season" | "volunteer_day";
export type Finale = "second_wind" | "clean_sweep";
export type ChangeKind = "twist" | "finale";

export interface RunObj {
  id: number;
  kind: ObjKind;
  lane: number;
  z: number;
  alive: boolean;
  plugged: boolean;
  /** Waves on a wide river: a second gap lane. */
  gap2?: number;
  /** Ghost nets: cuts still needed. */
  hp?: number;
  /** Currents: push direction (-1 left, +1 right). */
  dir?: number;
}

export interface RunBoat {
  seat: number;
  crew: number;
  lane: number;
  z: number;
  speed: number;
  hop: number;
  capsize: number;
  /** Seconds of protection left after a capsize (includes the capsize itself). */
  immune: number;
  slow: number;
  boost: number;
  boosting: number;
  plastic: number;
  plugging: number | null;
  /** Seconds left fixing a just-plugged pipe (the boat is stopped). */
  fixing?: number;
  /** Paused, disconnected or quit: the boat waits out of the race. */
  parked?: boolean;
  /** Pipes this boat fixed this run. */
  pipes?: number;
}

export type RunPhase =
  | { phase: "countdown"; left: number }
  | { phase: "running" }
  | { phase: "warning"; change: ChangeKind; left: number }
  | { phase: "choosing"; change: ChangeKind; left: number; pickers: number | null }
  | { phase: "reveal"; change: ChangeKind; left: number; spin: boolean; tie_by?: number }
  | { phase: "finished" };

/** One number on a lock, as this viewer may see it. */
export interface TileView {
  /** Whose number it is (crew locks). */
  seat?: number;
  /** The number (shown to the fixer only). */
  digit?: number;
  /** Sent by its player (crew locks). */
  ready: boolean;
  /** Already typed into the lock. */
  used?: boolean;
}

export interface PlugView {
  pipe: number;
  /** A crew number lock (otherwise the solo lock). */
  team: boolean;
  /** The lock's numbers: crew locks in seat order, solo in shown order. */
  tiles: TileView[];
  /** Numbers typed so far. */
  entered: number;
  /** A crew lock has locked in its numbers (on the fixer's first number). */
  locked?: boolean;
  left: number;
}

export interface RunResults {
  scores: [number, number][];
  winner: number | null;
  health: number;
  choked: boolean;
  revived: boolean;
}

export interface Snapshot {
  tick: number;
  river?: River;
  /** Solo Endless: no time limit, one capsize ends the run. */
  endless?: boolean;
  /** Endless: lives the crew has left. */
  lives?: number;
  /** Metres travelled by the leading boat. */
  distance?: number;
  duration: number;
  tutorial: boolean;
  t: number;
  phase: RunPhase;
  health: number;
  choked: boolean;
  twist: Twist | null;
  finale: Finale | null;
  finale_crew: number | null;
  changes: number;
  votes: [number, number][];
  /** Open lanes (0 = leftmost). */
  lanes: number[];
  /** Lanes in the full-width river: 3, 4 or 6. */
  lane_count: number;
  boats: RunBoat[];
  /** [crew id, score, plastic, pipes, banks, ghost nets freed] */
  crews: [number, number, number, number, number, number][];
  objs: RunObj[];
  plugs: [number, PlugView][];
  /** Your number (1–9) for the pipe ahead: while shown, while a crewmate's lock waits for it, or once sent. */
  my_key?: number | null;
  /** Seconds until your number hides (it blinks near the end). */
  key_left?: number;
  /** A crewmate is at the lock waiting for your number. */
  key_call?: boolean;
  /** Your number has been sent. */
  key_sent?: boolean;
  results: RunResults | null;
}

export type RunEvent =
  | { type: "collected"; seat: number; id: number }
  | { type: "missed"; id: number }
  | { type: "cut"; seat: number; id: number }
  | { type: "slowed"; seat: number }
  | { type: "capsized"; seat: number; dropped: number }
  | { type: "bumped"; seat: number; by: number }
  | { type: "boost"; seat: number }
  | { type: "plug_started"; seat: number; pipe: number }
  | { type: "plugged"; seat: number; pipe: number }
  | { type: "plug_failed"; seat: number; pipe: number }
  | { type: "pipe_missed"; pipe: number }
  | { type: "wrong_key"; seat: number }
  | { type: "wave_incoming"; id: number }
  | { type: "rule_changed"; change: ChangeKind; choice: string }
  | { type: "choked" }
  | { type: "net_hit"; seat: number; id: number; left: number }
  | { type: "net_freed"; seat: number; id: number }
  | { type: "pushed"; seat: number; dir: number }
  | { type: "keys_dealt"; pipe: number }
  | { type: "plug_skipped"; seat: number; pipe: number }
  | { type: "answered"; seat: number; pipe: number };

export const RUN_SECONDS = 180;
export const LANE_W = 2.4;

/** Lock numbers run 1..DIGITS. */
export const DIGITS = 9;

export const TWIST_INFO: Record<Twist, { icon: string; name: string; line: string }> = {
  monsoon_surge: { icon: "🌧️", name: "Monsoon Surge", line: "Debris waves every 15 s — but twice the plastic to collect." },
  dry_season: { icon: "☀️", name: "Dry Season", line: "The river loses a third of its lanes and pipes pour twice as fast." },
  volunteer_day: { icon: "🙌", name: "Volunteer Day", line: "Plastic counts double — but weed mats keep spreading." },
};
export const TWIST_ORDER: Twist[] = ["monsoon_surge", "dry_season", "volunteer_day"];

export const FINALE_INFO: Record<Finale, { icon: string; name: string; line: string }> = {
  second_wind: { icon: "💨", name: "Second Wind", line: "Your crew's boost fills and extra plastic flows your way for 20 s." },
  clean_sweep: { icon: "🧹", name: "Clean Sweep", line: "Every open pipe pours at half rate for the rest of the run." },
};
export const FINALE_ORDER: Finale[] = ["second_wind", "clean_sweep"];

export const SEAT_COLOURS = ["#f2c14e", "#4fb3ff", "#ff7a59", "#9b6bff"];
export const CREW_NAMES = ["Kingfishers", "Otters"];
