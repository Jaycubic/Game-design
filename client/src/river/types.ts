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
}

export type RunPhase =
  | { phase: "countdown"; left: number }
  | { phase: "running" }
  | { phase: "warning"; change: ChangeKind; left: number }
  | { phase: "choosing"; change: ChangeKind; left: number; pickers: number | null }
  | { phase: "reveal"; change: ChangeKind; left: number; spin: boolean }
  | { phase: "finished" };

export interface PlugView {
  pipe: number;
  code: [number, number, number];
  clear: boolean;
  /** No crewmate: a memory puzzle (shown clearly at first, then hidden). */
  memory?: boolean;
  /** Memory puzzle after the code is hidden: `code` carries nothing. */
  hidden?: boolean;
  entered: number;
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
  | { type: "ping"; seat: number; glyph: number }
  | { type: "wave_incoming"; id: number }
  | { type: "rule_changed"; change: ChangeKind; choice: string }
  | { type: "choked" }
  | { type: "net_hit"; seat: number; id: number; left: number }
  | { type: "net_freed"; seat: number; id: number }
  | { type: "pushed"; seat: number; dir: number };

export const RUN_SECONDS = 180;
export const LANE_W = 2.4;

export const GLYPHS = ["🍃", "💧", "🐟", "🌉", "➜", "⭐"];
export const GLYPH_NAMES = ["leaf", "drop", "fish", "bridge", "arrow", "star"];

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
