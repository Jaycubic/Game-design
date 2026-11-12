#!/usr/bin/env node
// Shared-room end-to-end test against a running server (Node 22+: global fetch + WebSocket).
//
//   BASE_URL=http://localhost:8070 CLASSIFY=./target/release/classify node tests/e2e/room.mjs
//
// Verifies with independent clients (separate accounts and sockets):
//  - both players see the same room and level; accepted actions reach everyone
//  - rejected and stale actions change nothing
//  - Expedition turn order is enforced
//  - both Living Charter amendments are applied identically for every member
//  - private Field Report clues reach only their holder
//  - a disconnected player reconnects to the same state
//  - a room cannot exceed four players; two rooms never leak state
// Solver plans (from the `classify` binary) drive optimal play through the real API.

import { execFileSync } from "node:child_process";

const BASE = (process.env.BASE_URL ?? "http://localhost:8070").replace(/\/$/, "");
const WS_BASE = BASE.replace(/^http/, "ws");
const CLASSIFY = process.env.CLASSIFY ?? "./target/release/classify";
const TIMEOUT = 15000;
const run = Math.random().toString(36).slice(2, 8);

let passed = 0;
function ok(cond, msg) {
  if (!cond) throw new Error("FAILED: " + msg);
  passed++;
  console.log("  ✔ " + msg);
}

async function api(path, { method = "GET", token, body } = {}) {
  const res = await fetch(BASE + path, {
    method,
    headers: { "content-type": "application/json", ...(token ? { authorization: "Bearer " + token } : {}) },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  return { status: res.status, body: text ? JSON.parse(text) : null };
}

async function user(tag) {
  const r = await api("/api/auth/register", {
    method: "POST",
    body: { handle: `e2e_${tag}_${run}`, display_name: `Tester ${tag}`, password: "forest-pass-123" },
  });
  if (r.status !== 201) throw new Error(`register ${tag}: ${r.status} ${JSON.stringify(r.body)}`);
  return { tag, token: r.body.token, id: r.body.player.id };
}

class Client {
  constructor(u, roomId) {
    this.u = u;
    this.roomId = roomId;
    this.messages = [];
    this.state = null;
    this.waiters = [];
  }
  connect() {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(`${WS_BASE}/ws?room=${this.roomId}`);
      this.ws.onopen = () => this.ws.send(JSON.stringify({ v: 1, type: "auth", payload: { token: this.u.token } }));
      this.ws.onerror = (e) => reject(new Error("socket error " + (e.message ?? "")));
      this.ws.onmessage = (ev) => {
        const m = JSON.parse(ev.data);
        this.messages.push(m);
        if (m.payload?.state) this.state = m.payload.state;
        if (m.type === "room.joined") resolve();
        this.waiters = this.waiters.filter((w) => !w(m));
      };
      setTimeout(() => reject(new Error("connect timeout")), TIMEOUT);
    });
  }
  send(type, payload = {}) {
    const id = crypto.randomUUID();
    this.ws.send(JSON.stringify({ v: 1, id, type, payload }));
    return id;
  }
  /** Send and resolve with the ack or error that references this message. */
  request(type, payload = {}) {
    const id = this.send(type, payload);
    return this.until((m) => m.payload?.ref === id && (m.type === "ack" || m.type === "error" || m.type === "pong"));
  }
  until(pred, timeout = TIMEOUT) {
    const found = this.messages.find(pred);
    if (found) return Promise.resolve(found);
    return new Promise((resolve, reject) => {
      const t = setTimeout(() => reject(new Error(`${this.u.tag}: timed out waiting`)), timeout);
      this.waiters.push((m) => {
        if (pred(m)) {
          clearTimeout(t);
          resolve(m);
          return true;
        }
        return false;
      });
    });
  }
  waitVersion(v) {
    return this.state?.version >= v ? Promise.resolve() : this.until(() => this.state?.version >= v);
  }
  close() {
    this.ws.close();
  }
}

const planCache = new Map();
function plan(level, rulesKey) {
  const k = `${level}|${rulesKey}`;
  if (!planCache.has(k)) {
    let out;
    try {
      out = execFileSync(CLASSIFY, ["--level", String(level), "--json"], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
    } catch (e) {
      out = e.stdout; // classify exits non-zero when a report needs attention; the JSON is still complete
    }
    for (const [n, r] of JSON.parse(out)) planCache.set(`${n}|${r.rules}`, r.plan);
  }
  const p = planCache.get(k);
  if (!p) throw new Error(`no plan for level ${level} under ${rulesKey}`);
  return p;
}

/** Play the current level optimally through the API, honouring turn order. */
async function playLevel(clients) {
  const [a] = clients;
  const play = a.state.play;
  const steps = plan(play.level, play.rules_key);
  const bySeat = (s) => clients.find((c) => c.state.my_seat === s);
  const expedition = play.turn_seat !== null && play.turn_seat !== undefined;
  for (const action of steps) {
    const st = a.state;
    if (st.phase !== "playing") break;
    if (expedition && action.type === "end_round") {
      const round = st.play.state.round;
      while (a.state.phase === "playing" && a.state.play.state.round === round) {
        const c = bySeat(a.state.play.turn_seat);
        const r = await c.request("turn.pass", { base_version: c.state.version });
        if (r.type === "error") throw new Error("pass rejected: " + JSON.stringify(r.payload));
        await a.waitVersion(c.state.version);
      }
      continue;
    }
    const actor = expedition ? bySeat(st.play.turn_seat) : a;
    const r = await actor.request("player.action", { action, base_version: actor.state.version });
    if (r.type === "error") throw new Error(`action ${JSON.stringify(action)} rejected: ${JSON.stringify(r.payload)}`);
    for (const c of clients) await c.waitVersion(actor.state.version);
  }
  await a.until(() => a.state.phase === "result");
}

async function main() {
  console.log(`Shared-room E2E against ${BASE}`);
  const health = await api("/health");
  ok(health.status === 200, "server is healthy");

  // ── capacity: a fifth player cannot join ────────────────────────────────
  const crowd = await Promise.all(["c1", "c2", "c3", "c4", "c5"].map(user));
  const r2 = await api("/api/rooms", { method: "POST", token: crowd[0].token });
  ok(r2.status === 201 && /^[A-Z2-9]{6}$/.test(r2.body.code), "room created with a 6-character invitation code");
  for (const u of crowd.slice(1, 4)) {
    const j = await api("/api/rooms/join", { method: "POST", token: u.token, body: { code: r2.body.code } });
    if (j.status !== 200) throw new Error("join failed " + JSON.stringify(j.body));
  }
  const fifth = await api("/api/rooms/join", { method: "POST", token: crowd[4].token, body: { code: r2.body.code } });
  ok(fifth.status === 409 && fifth.body.code === "room_full", "a fifth player is refused (room_full)");

  // ── main two-player room ───────────────────────────────────────────────
  const [ua, ub] = await Promise.all([user("a"), user("b")]);
  const created = await api("/api/rooms", { method: "POST", token: ua.token });
  const roomId = created.body.room_id;
  const joined = await api("/api/rooms/join", { method: "POST", token: ub.token, body: { code: created.body.code.toLowerCase() } });
  ok(joined.status === 200 && joined.body.room_id === roomId, "second player joins with the code (case-insensitive)");
  const outsider = await user("x");
  const peek = await api(`/api/rooms/${roomId}`, { token: outsider.token });
  ok(peek.status === 403, "a non-member cannot read the room");

  const A = new Client(ua, roomId);
  const B = new Client(ub, roomId);
  await A.connect();
  await B.connect();
  const other = new Client(crowd[0], r2.body.room_id);
  await other.connect();
  ok(A.state.members.length === 2 && B.state.members.length === 2, "both players see the same member list");

  await A.request("room.ready", { ready: true });
  await B.request("room.ready", { ready: true });
  const start = await A.request("level.start");
  ok(start.type === "ack", "host starts level 1");
  await B.until(() => B.state.phase === "playing");
  ok(A.state.play.level === 1 && B.state.play.level === 1, "both players see level 1");
  ok(JSON.stringify(A.state.play.state.board) === JSON.stringify(B.state.play.state.board), "both players see the same board");

  const v0 = B.state.version;
  const bad = await B.request("player.action", { action: { type: "plant", at: [0, 0] }, base_version: v0 });
  ok(bad.type === "error" && bad.payload.code === "wrong_terrain", "an invalid action is rejected with a stable code");
  ok(A.state.version === v0 && B.state.version === v0, "a rejected action changes nobody's state");
  const stale = await B.request("player.action", { action: { type: "plant", at: [2, 0] }, base_version: v0 - 1 });
  ok(stale.type === "error" && stale.payload.code === "stale_state", "a stale action is rejected");
  const garbage = await new Promise((resolve) => {
    B.ws.send("{not json");
    B.until((m) => m.type === "error" && m.payload.code === "bad_message").then(resolve);
  });
  ok(garbage && B.ws.readyState === WebSocket.OPEN, "malformed JSON is answered with an error and the socket stays open");

  const accepted = await B.request("player.action", { action: { type: "plant", at: [2, 0] }, base_version: B.state.version });
  ok(accepted.type === "ack", "player B plants a sapling");
  await A.waitVersion(B.state.version);
  ok(A.state.play.state.board.tiles[2].terrain === "forest", "B's accepted action appears for A");
  await B.request("player.action", { action: { type: "revert", at: [2, 0] }, base_version: B.state.version });
  await A.waitVersion(B.state.version);

  await playLevel([A, B]);
  ok(A.state.result.won && A.state.result.marks.objective, "level 1 won through the API");
  await A.request("level.continue");
  await A.until(() => A.state.play?.level === 2);
  ok(A.state.play.turn_seat === 0, "level 2 Expedition starts on seat 0's turn");
  const wrongTurn = await B.request("player.action", { action: { type: "plant", at: [2, 1] }, base_version: B.state.version });
  ok(wrongTurn.type === "error" && wrongTurn.payload.code === "not_your_turn", "acting out of turn is rejected");
  await playLevel([A, B]);
  ok(A.state.result.won, "level 2 Expedition won with alternating turns");

  // ── Amendment 1 ─────────────────────────────────────────────────────────
  await A.request("level.continue");
  await B.until(() => B.state.phase === "vote");
  ok(B.state.campaign.vote.amendment === 1, "the Connectivity vote opens for everyone");
  await A.request("charter.vote", { choice: "B" });
  await B.request("charter.vote", { choice: "B" });
  await A.until(() => A.state.play?.level === 3);
  await B.until(() => B.state.play?.level === 3);
  ok(A.state.campaign.connectivity === "stepping_stone" && B.state.campaign.connectivity === "stepping_stone", "amendment 1 applied identically for both players");
  for (let n = 3; n <= 4; n++) {
    await playLevel([A, B]);
    ok(A.state.result.won, `level ${n} won under Stepping-Stone Networks`);
    await A.request("level.continue");
  }

  // ── Amendment 2 with a tie → revote → tie-break ────────────────────────
  await B.until(() => B.state.phase === "vote" && B.state.campaign.vote.amendment === 2);
  await A.request("charter.vote", { choice: "A" });
  await B.request("charter.vote", { choice: "B" });
  await A.until((m) => m.type === "charter.revote");
  ok(true, "a tied vote triggers a revote");
  await A.request("charter.vote", { choice: "A" });
  await B.request("charter.vote", { choice: "B" });
  await A.until(() => A.state.campaign.vote?.tie_breaker !== null && A.state.campaign.vote?.tie_breaker !== undefined);
  const tb = A.state.campaign.vote.tie_breaker;
  const breaker = tb === A.state.my_seat ? A : B;
  await breaker.request("charter.vote", { choice: "A" });
  await A.until(() => A.state.play?.level === 5);
  await B.until(() => B.state.play?.level === 5);
  ok(A.state.campaign.recovery === "succession" && B.state.campaign.recovery === "succession", "amendment 2 decided by the tie-break player, identical for both");

  // ── Private clues ──────────────────────────────────────────────────────
  const ca = A.state.play.my_clues;
  const cb = B.state.play.my_clues;
  ok(ca.length >= 2 && cb.length >= 2, "each player holds Field Report clues");
  ok(!ca.some((c) => cb.includes(c)), "players hold different clues");
  const leaked = JSON.stringify(B.messages).includes(ca[0]);
  ok(!leaked, "B never received A's clue text in any message");
  ok(A.messages.some((m) => m.type === "player.clue_assigned"), "clues were delivered privately (player.clue_assigned)");
  const content = await api("/api/content");
  ok(!JSON.stringify(content.body).includes(ca[0]), "public content does not contain clue texts");

  // ── Reconnect ──────────────────────────────────────────────────────────
  const before = JSON.stringify({ v: B.state.version, play: B.state.play.state, c: B.state.campaign });
  B.close();
  await A.until((m) => m.type === "room.member_disconnected");
  ok(true, "A is told B disconnected");
  const B2 = new Client(ub, roomId);
  await B2.connect();
  ok(B2.state.my_seat === 1 && B2.state.play.level === 5, "B reconnects to the same seat and level");
  ok(B2.state.campaign.connectivity === "stepping_stone" && B2.state.campaign.recovery === "succession", "reconnect restores both amendments");
  ok(JSON.stringify(B2.state.play.state) === JSON.stringify(JSON.parse(before).play), "reconnect restores the level state");
  ok(JSON.stringify(B2.state.play.my_clues) === JSON.stringify(cb), "reconnect restores B's own clues only");

  const answer = plan(5, A.state.play.rules_key)[0];
  const ans = await A.request("player.action", { action: answer, base_version: A.state.version });
  ok(ans.type === "ack", "the team submits the deduced answer");
  await B2.until(() => B2.state.phase === "result" && B2.state.result.won);
  ok(true, "the Field Report puzzle is solved for both players");

  // ── Isolation between rooms ────────────────────────────────────────────
  const leakedRoom = other.messages.some((m) => m.room === roomId || JSON.stringify(m).includes(roomId));
  ok(!leakedRoom, "the other room received nothing from this room");
  const otherSeesOnlyOwn = other.state.members.length === 4 && other.state.id === r2.body.room_id;
  ok(otherSeesOnlyOwn, "the other room's state only describes itself");

  for (const c of [A, B2, other]) c.close();
  console.log(`\nAll ${passed} shared-room checks passed.`);
}

main().catch((e) => {
  console.error("\n" + e.message);
  process.exit(1);
});
