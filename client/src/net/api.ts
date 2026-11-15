import type { Content, PlayerInfo, RoomView } from "../types";

export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(message);
  }
}

const TOKEN_KEY = "nt.token";

function readToken(): string | null {
  try {
    return sessionStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

export const auth = {
  token: readToken(),
  set(token: string | null) {
    this.token = token;
    try {
      if (token) sessionStorage.setItem(TOKEN_KEY, token);
      else sessionStorage.removeItem(TOKEN_KEY);
    } catch {
      /* storage unavailable: session-only login */
    }
  },
};

async function request<T>(path: string, init: { method?: string; body?: unknown } = {}): Promise<T> {
  let res: Response;
  try {
    res = await fetch(path, {
      method: init.method ?? "GET",
      headers: {
        "content-type": "application/json",
        ...(auth.token ? { authorization: `Bearer ${auth.token}` } : {}),
      },
      body: init.body === undefined ? undefined : JSON.stringify(init.body),
    });
  } catch {
    throw new ApiError(0, "offline", "Cannot reach the game server. Check that it is running and that you are on the same network.");
  }
  const text = await res.text();
  let data: any = null;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    /* non-JSON body (e.g. proxy error page) */
  }
  if (!res.ok) {
    if (res.status === 401) auth.set(null);
    throw new ApiError(res.status, data?.code ?? "http_" + res.status, data?.message ?? `The server answered ${res.status}.`);
  }
  return data as T;
}

export interface AuthResponse {
  token: string;
  expires_at: string;
  player: PlayerInfo;
}
export interface CatalogBoat {
  key: string;
  name: string;
  colour: string;
  price: number;
  source: "default" | "shop" | "road";
  model: string;
  blurb: string;
  /** Upgrade levels 1–5 (2 = starter boat). */
  stats: { speed: number; net: number; cutter: number; boost: number };
}
export interface Profile {
  seeds: number;
  runs: number;
  wins: number;
  best_score: number;
  equipped: string;
  owned: string[];
  refundable: string[];
  catalog: CatalogBoat[];
  daily: { claims: number; claimed_today: boolean; track: number[]; next: number };
  road: { runs: number; seeds: number; item: string | null; reached: boolean; claimed: boolean }[];
  rules: { win_bonus_per_day: number; refund_minutes: number };
  /** Total stars across rivers. */
  stars: number;
  rivers: RiverCard[];
  missions: MissionView[];
  rank: RankView;
}
export interface MissionView {
  key: string;
  text: string;
  progress: number;
  goal: number;
  reward: number;
  done: boolean;
  /** Completed by the run that just finished. */
  just_done: boolean;
}
export interface RankView {
  name: string;
  index: number;
  next: string | null;
  /** 0–1 towards the next rank. */
  progress: number;
  /** -1, 0 or +1 compared with before the run. */
  change: number;
}
export interface RiverCard {
  key: string;
  name: string;
  line: string;
  /** The three star goals, in words. */
  goals: string[];
  stars: boolean[];
  best: number;
  unlocked: boolean;
  need_runs: number;
  need_stars: number;
}
/** What the results screen shows for the river after a run. */
export interface RiverResult {
  river: string;
  name: string;
  goals: string[];
  stars: boolean[];
  new_stars: boolean[];
  score: number;
  best: number;
  new_best: boolean;
}
export interface RoomResponse {
  room_id: string;
  code: string;
  state: RoomView;
}

export const api = {
  content: () => request<Content>("/api/content"),
  register: (handle: string, display_name: string, password: string) =>
    request<AuthResponse>("/api/auth/register", { method: "POST", body: { handle, display_name, password } }),
  login: (handle: string, password: string) => request<AuthResponse>("/api/auth/login", { method: "POST", body: { handle, password } }),
  logout: () => request<void>("/api/auth/logout", { method: "POST" }),
  me: () => request<PlayerInfo>("/api/me"),
  myRooms: () => request<{ rooms: { room_id: string; code: string; status: string }[] }>("/api/me/rooms"),
  progress: () => request<any>("/api/me/progress"),
  profile: () => request<Profile>("/api/me/profile"),
  claimDaily: () => request<{ reward: number }>("/api/me/daily", { method: "POST" }),
  claimRoad: (runs: number) => request<{ seeds: number; item: string | null }>("/api/me/road", { method: "POST", body: { runs } }),
  buy: (item: string) => request<{ seeds: number; spent: number }>("/api/shop/buy", { method: "POST", body: { item } }),
  undo: (item: string) => request<{ refunded: number }>("/api/shop/undo", { method: "POST", body: { item } }),
  equip: (item: string) => request<{ equipped: string }>("/api/garage/equip", { method: "POST", body: { item } }),
  createRoom: () => request<RoomResponse>("/api/rooms", { method: "POST" }),
  joinRoom: (code: string) => request<RoomResponse>("/api/rooms/join", { method: "POST", body: { code } }),
};
