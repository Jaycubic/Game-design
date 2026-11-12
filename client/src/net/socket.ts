import type { Envelope } from "../types";

/** Message id; crypto.randomUUID is unavailable on plain-HTTP LAN pages. */
export function uid(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto && window.isSecureContext) return crypto.randomUUID();
  return Date.now().toString(36) + "-" + Math.random().toString(36).slice(2, 12);
}

export type SocketStatus = "connecting" | "open" | "reconnecting" | "closed";

export interface SocketHandlers {
  onMessage(m: Envelope): void;
  onStatus(s: SocketStatus, detail?: string): void;
}

/**
 * Room WebSocket with automatic reconnection (exponential backoff) and
 * first-message authentication. Commands carry a message id for de-duplication.
 */
export class RoomSocket {
  private ws: WebSocket | null = null;
  private attempts = 0;
  private closedByUser = false;
  private timer: number | undefined;
  private pingTimer: number | undefined;

  constructor(
    private roomId: string,
    private token: string,
    private handlers: SocketHandlers,
  ) {}

  connect() {
    this.closedByUser = false;
    const scheme = location.protocol === "https:" ? "wss" : "ws";
    this.handlers.onStatus(this.attempts === 0 ? "connecting" : "reconnecting");
    const ws = new WebSocket(`${scheme}://${location.host}/ws?room=${encodeURIComponent(this.roomId)}`);
    this.ws = ws;
    ws.onopen = () => {
      ws.send(JSON.stringify({ v: 1, type: "auth", payload: { token: this.token } }));
    };
    ws.onmessage = (ev) => {
      let m: Envelope;
      try {
        m = JSON.parse(ev.data);
      } catch {
        return;
      }
      if (m.type === "room.joined") {
        this.attempts = 0;
        this.handlers.onStatus("open");
        window.clearInterval(this.pingTimer);
        this.pingTimer = window.setInterval(() => this.send("ping"), 20000);
      }
      if (m.type === "error" && (m.payload?.code === "unauthorized" || m.payload?.code === "not_member" || m.payload?.code === "room_not_found")) {
        this.closedByUser = true; // retrying cannot fix these
        this.handlers.onStatus("closed", m.payload.message);
      }
      if (m.type === "room.expired") this.closedByUser = true;
      this.handlers.onMessage(m);
    };
    ws.onclose = () => {
      window.clearInterval(this.pingTimer);
      if (this.closedByUser) {
        this.handlers.onStatus("closed");
        return;
      }
      this.attempts++;
      const delay = Math.min(10000, 500 * 2 ** Math.min(this.attempts, 5));
      this.handlers.onStatus("reconnecting", `Reconnecting in ${Math.round(delay / 1000)}s…`);
      this.timer = window.setTimeout(() => this.connect(), delay);
    };
  }

  send(type: string, payload: Record<string, unknown> = {}): string | null {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return null;
    const id = uid();
    this.ws.send(JSON.stringify({ v: 1, id, type, payload }));
    return id;
  }

  /** High-rate messages (run input): no id, so no ack or de-duplication. */
  sendRaw(type: string, payload: Record<string, unknown> = {}): boolean {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return false;
    this.ws.send(JSON.stringify({ v: 1, type, payload }));
    return true;
  }

  close() {
    this.closedByUser = true;
    window.clearTimeout(this.timer);
    window.clearInterval(this.pingTimer);
    this.ws?.close();
  }
}
