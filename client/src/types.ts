// Shapes of the data the Rust server sends (see crates/game-server/src/room.rs).

export type Pos = [number, number];
export type Terrain = "forest" | "degraded" | "open" | "road" | "river" | "grassland" | "rock" | "grove" | "bridge";
export type SpeciesId =
  | "lion_tailed_macaque"
  | "malabar_giant_squirrel"
  | "great_hornbill"
  | "malabar_gliding_frog"
  | "nilgiri_tahr";
export type Connectivity = "base" | "continuous" | "stepping_stone";
export type Recovery = "base" | "succession" | "active";

export interface Tile {
  terrain: Terrain;
  fig: boolean;
  nest: boolean;
}
export interface Board {
  width: number;
  height: number;
  tiles: Tile[];
}

export type Action =
  | { type: "plant" | "restore" | "protect" | "bridge" | "grove" | "revert"; at: Pos }
  | { type: "place_species"; species: SpeciesId; at: Pos }
  | { type: "remove_species"; species: SpeciesId }
  | { type: "answer"; candidate: string }
  | { type: "end_round" };
export type ActionKind = "plant" | "restore" | "protect" | "bridge" | "grove";

export interface ObjectiveStatus {
  label: string;
  met: boolean;
  detail: string | null;
}
export interface Evaluation {
  objectives: ObjectiveStatus[];
  all_met: boolean;
}
export interface RoundReport {
  round: number;
  events: string[];
  degraded: Pos[];
  eroded: Pos[];
  regrown: Pos[];
  evaluation: Evaluation;
}
export type Status = { state: "in_progress" } | { state: "won"; round: number } | { state: "lost"; reason: string };

export interface SimState {
  board: Board;
  round: number;
  capacity_left: number;
  spent: number;
  protected: Pos[];
  placements: { species: SpeciesId; at: Pos }[];
  attempts: number;
  status: Status;
}

export interface EventDef {
  round: number;
  title: string;
  text: string;
  kind: "lantana" | "dry_spell" | "erosion";
  cells?: Pos[];
  capacity_penalty?: number;
}

export interface PlayView {
  level: number;
  rules: { connectivity: Connectivity; recovery: Recovery };
  rules_key: string;
  state: SimState;
  turn_seat: number | null;
  passed: number[];
  threatened: Pos[];
  round_capacity: number;
  events: EventDef[];
  evaluation: Evaluation;
  preview: RoundReport | null;
  last_report: RoundReport | null;
  par: number | null;
  my_clues: string[];
  clue_counts: Record<string, number>;
}

export interface LeafMarks {
  objective: boolean;
  efficient: boolean;
  challenge: boolean;
}
export interface LevelResult {
  level: number;
  won: boolean;
  marks: LeafMarks;
  spent: number;
  par: number | null;
  reason: string | null;
  rules: string;
  journal: string[];
  insight: string | null;
  restored_cells: number;
  final_board: Board;
}

export interface Member {
  seat: number;
  name: string;
  ready: boolean;
  connected: boolean;
  host: boolean;
  you: boolean;
}

export interface VoteView {
  amendment: number;
  ballot_round: number;
  eligible: number[];
  voted: number[];
  my_vote: "A" | "B" | null;
  tie_breaker: number | null;
}

export interface RoomView {
  id: string;
  code: string;
  version: number;
  status: string;
  phase: "lobby" | "playing" | "result" | "vote" | "complete";
  my_seat: number | null;
  members: Member[];
  min_players: number;
  max_players: number;
  campaign: {
    region: string;
    current_level: number | null;
    connectivity: Connectivity | null;
    recovery: Recovery | null;
    best: Record<string, { leaf_marks: number; spent: number; completions: number }>;
    insights: string[];
    restored: Record<string, number>;
    vote: VoteView | null;
  };
  play: PlayView | null;
  result: LevelResult | null;
}

// ── Static content (/api/content) ──────────────────────────────────────────

export interface Objective {
  kind: "connect" | "habitat" | "forest_cover" | "restore";
  label: string;
  species?: SpeciesId;
  from?: Pos;
  to?: Pos;
  at?: Pos | null;
  min?: number;
  cells?: Pos[];
}
export interface Candidate {
  id: string;
  label: string;
  at: Pos;
}
export interface LevelDef {
  id: string;
  number: number;
  mode: "puzzle" | "expedition";
  kind: "connection" | "niche_match" | "deduction" | "ecosystem_logic" | "blueprint" | "expedition";
  title: string;
  area: string;
  instruction: string;
  intro: string;
  map: string[];
  sites: Pos[] | null;
  slots: Pos[];
  tokens: SpeciesId[];
  rounds: number;
  capacity: number;
  events: EventDef[];
  objectives: Objective[];
  critical_degraded: number | null;
  max_attempts: number;
  challenge: { text: string; kind: string };
  deduction: { question: string; candidates: Candidate[]; clue_count: number } | null;
  par: Record<string, number>;
  reward: { journal: string[]; insight: string | null };
  uses_insight: { insight: string; cells: Pos[]; text: string } | null;
}
export interface AmendmentOption {
  name: string;
  summary: string;
  effects: string[];
}
export interface Amendment {
  number: number;
  after_level: number;
  title: string;
  question: string;
  option_a: AmendmentOption;
  option_b: AmendmentOption;
}
export interface JournalEntry {
  key: string;
  title: string;
  kind: string;
  text: string;
  sources: string[];
}
export interface Content {
  region: {
    id: string;
    name: string;
    ecosystem: string;
    tagline: string;
    areas: { id: string; name: string; description: string }[];
    amendments: Amendment[];
  };
  levels: LevelDef[];
  journal: { sources: { id: string; title: string; url: string }[]; entries: JournalEntry[] };
}

export interface PlayerInfo {
  id: string;
  handle: string;
  display_name: string;
}

export interface Envelope {
  v: number;
  id: string;
  type: string;
  room: string;
  seq: number;
  ts: string;
  payload: any;
}
