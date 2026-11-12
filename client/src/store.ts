import type { Content, PlayerInfo, RoomView, ActionKind, SpeciesId } from "./types";
import type { SocketStatus } from "./net/socket";
import type { Profile } from "./net/api";

/** Local UI selection — never sent to the server as state. */
export type Tool =
  | { kind: "action"; action: ActionKind }
  | { kind: "species"; species: SpeciesId }
  | { kind: "inspect" };

export interface AppState {
  content: Content | null;
  player: PlayerInfo | null;
  room: RoomView | null;
  socket: SocketStatus;
  socketDetail: string | null;
  tool: Tool;
  hover: [number, number] | null;
  showPreview: boolean;
  hint: { action: any; message?: string } | null;
  clues: string[];
  profile: Profile | null;
}

type Listener = (s: AppState, prev: AppState) => void;

export class Store {
  private s: AppState;
  private listeners = new Set<Listener>();

  constructor(initial: AppState) {
    this.s = initial;
  }

  get state(): AppState {
    return this.s;
  }

  set(patch: Partial<AppState>) {
    const prev = this.s;
    this.s = { ...prev, ...patch };
    for (const l of this.listeners) l(this.s, prev);
  }

  subscribe(l: Listener): () => void {
    this.listeners.add(l);
    return () => this.listeners.delete(l);
  }
}

export const store = new Store({
  content: null,
  player: null,
  room: null,
  socket: "closed",
  socketDetail: null,
  tool: { kind: "inspect" },
  hover: null,
  showPreview: true,
  hint: null,
  clues: [],
  profile: null,
});
