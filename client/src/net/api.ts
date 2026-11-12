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
  createRoom: () => request<RoomResponse>("/api/rooms", { method: "POST" }),
  joinRoom: (code: string) => request<RoomResponse>("/api/rooms/join", { method: "POST", body: { code } }),
};
