# Clearwater Run — HTTP & WebSocket API

One listener serves everything: the API, the WebSocket and the built client. The default port is **8070**; it uses
HTTPS when `TLS_CERT` and `TLS_KEY` are set.

All bodies are JSON. Errors look like `{ "code": "<stable_code>", "message": "<safe explanation>" }`.

## HTTP

`Bearer` below means the header `Authorization: Bearer <token>`. The token is HS256-signed, and its `jti` is a row
in `player_sessions`, so it can be revoked. Tokens are never stored in plain form or logged.

### Health, accounts and rooms

| Method & path | Auth | Purpose |
|---|---|---|
| `GET /health` | – | Liveness: `{status, service, version, rooms}` |
| `GET /ready` | – | Readiness: 200 when PostgreSQL answers, 503 otherwise |
| `POST /api/auth/register` | – | `{handle, display_name, password}` → 201 `{token, expires_at, player}`. Rate-limited to 10 per minute per IP. |
| `POST /api/auth/login` | – | `{handle, password}` → `{token, expires_at, player}`. Rate-limited. |
| `POST /api/auth/logout` | Bearer | Revokes the session (204). |
| `GET /api/me` | Bearer | `{id, handle, display_name}` |
| `GET /api/me/rooms` | Bearer | Open rooms you belong to, for reconnecting |
| `POST /api/rooms` | Bearer | Creates a room → 201 `{room_id, code, state}` |
| `POST /api/rooms/join` | Bearer | `{code}` (case-insensitive) → `{room_id, code, state}`. 404 `room_not_found`, 409 `room_full`. Rate-limited to 20 per minute per IP. |
| `GET /api/rooms/{id}` | Bearer, member | Your view of the room. Non-members get 403. |

### Hub economy (Seeds, boats, rewards)

| Method & path | Auth | Purpose |
|---|---|---|
| `GET /api/me/profile` | Bearer | Everything the hub shows (see below). |
| `POST /api/me/daily` | Bearer | Collects today's reward → `{reward}`. 409 `already_claimed`. |
| `POST /api/me/road` | Bearer | `{runs}` (the milestone) → `{seeds, item}`. Errors: `not_reached`, 409 `already_claimed`. |
| `POST /api/shop/buy` | Bearer | `{item}` → `{item, spent, seeds}`. Errors: `not_enough_seeds`, 409 `already_owned`. |
| `POST /api/shop/undo` | Bearer | `{item}` → `{refunded}`, within 10 minutes of buying. Error: `not_refundable`. |
| `POST /api/garage/equip` | Bearer | `{item}` → `{equipped}`. Error: `not_owned`. |

The profile response contains:

```json
{
  "seeds": 0, "runs": 0, "wins": 0, "best_score": 0, "equipped": "boat_sun",
  "owned": ["boat_sun"], "refundable": [],
  "catalog": [{ "key", "name", "colour", "price", "source", "model", "blurb",
                "stats": { "speed", "net", "cutter", "boost" } }],
  "daily": { "claims", "claimed_today", "track": [20,25,30,40,50,60,100], "next" },
  "road": [{ "runs", "seeds", "item", "reached", "claimed" }],
  "rules": { "win_bonus_per_day": 3, "refund_minutes": 10 },
  "stars": 0,
  "rivers": [{ "key", "name", "line", "goals": ["…", "…", "…"], "stars": [false, false, false], "best",
               "unlocked", "need_runs", "need_stars" }],
  "missions": [{ "key", "text", "progress", "goal", "reward", "done", "just_done" }],
  "rank": { "name": "Deckhand", "index": 1, "next": "Skipper", "progress": 0.67, "change": 0 }
}
```

Every economy change runs in one database transaction. Seeds can never go below zero.

### Earlier board game

These endpoints serve the earlier turn-based board game. They still work, but the client no longer uses them:
- `GET /api/content`
- `GET /api/me/progress`
- `GET /api/rooms/{id}/results`

## WebSocket: `GET /ws?room=<room_id>`

Messages in both directions use the same envelope:

```json
{ "v": 1, "id": "<message id>", "type": "<message type>", "payload": { } }
```

Server messages also carry `room`, `seq` and `ts`. In room messages `seq` is the room's state version; in run
messages it is the simulation tick.

1. **Authenticate first**, within 10 s: `{"type":"auth","payload":{"token":"…"}}`. On failure the server sends an
   `error` with `unauthorized` and closes.
2. The server replies **`room.joined`** with `payload.state`, your view of the room.
3. Every accepted room change is broadcast to each member as `{ events: [...], state }`.

Every command sent with an `id` gets an `ack`, or an `error` whose `ref` is that id. A repeated `id` is answered
with `{duplicate: true}` and not applied again. `ping` gets a `pong`.

### Room commands (client → server)

| type | payload | Notes |
|---|---|---|
| `room.ready` | `{ready: bool}` | Lobby only |
| `room.leave` | – | Leave permanently. The host role passes to the next seat. |
| `ping` | – | Answered with `pong` |

### Run commands (client → server)

| type | payload | Notes |
|---|---|---|
| `run.start` | `{practice?: bool, tutorial?: bool, river?: "canal" \| "mangrove" \| "gorge", endless?: bool}` | Host only, from the lobby. `river` defaults to `canal` and must be unlocked for the host (errors `bad_river`, `river_locked`). `endless` starts solo Endless (only with one connected player, error `endless_solo`; pays like practice). A normal run needs at least 2 connected, ready players. `practice` lets the host play alone for half Seeds; `tutorial` starts the 60-second scripted course, which pays no Seeds. Errors: `not_host`, `run_in_progress`, `not_in_lobby`, `not_enough_players`, `not_all_ready`, `no_players`. |
| `run.input` | `{steer?: -1\|0\|1, hop?: bool, action?: bool, glyph?: 0–5}` | Applied on the next tick. A `glyph` sent while plugging is a gate-code entry; otherwise it is a signal ping. |
| `run.choose` | `{option}` | Twist vote (0–2) or Finale pick (0–1). Only the trailing crew may pick the Finale. |

### Run messages (server → client)

| type | payload |
|---|---|
| `run.started` | `{seed, boats: [{seat, crew}], names: {seat: name}, skins: {seat: boat_key}, practice, tutorial, river, endless}` |
| `run.snapshot` | `{snapshot, you, skins}`, 15 times a second, filtered for each viewer (see below) |
| `run.events` | `{events: [...]}`, sent on each tick that has events |
| `run.rewards` | `{seeds, practice, river, missions, rank}` after a run finishes (not after the tutorial). `river` is `{river, name, goals, stars, new_stars, score, best, new_best}` for this player (for Endless: `{river, name, endless: true, distance, score, best, new_best}`); `missions` is today's three `{key, text, progress, goal, reward, done, just_done}`; `rank` is `{name, index, next, progress, change}` (absent after Endless, which doesn't change the rating) |

**Snapshot fields:**

| Field | Contents |
|---|---|
| Timing | `tick`, `duration` (0 in Endless, which has no time limit), `tutorial`, `t` |
| Mode | `river` (`canal`, `mangrove`, `gorge`), `endless`, `distance` (metres travelled by the leading boat) |
| `phase` | One of `countdown`, `running`, `warning`, `choosing`, `reveal` or `finished`, plus its `left` and the details of the change |
| River | `health`, `choked`, `lanes` (open lanes, numbered from 0 on the left), `lane_count` (3, 4 or 6) |
| Rule changes | `twist`, `finale`, `finale_crew`, `changes`, `votes: [[seat, option]]` |
| `boats` | Per boat: `seat`, `crew`, `lane`, `z`, `speed`, `hop`, `capsize`, `immune` (protection left after a capsize), `slow`, `boost`, `boosting`, `plastic` and `plugging` |
| `crews` | `[[id, score, plastic, pipes, banks, nets]]` (`nets`: ghost nets freed) |
| `objs` | Only the stretch of river near the boats. Each has `id`, `kind`, `lane` (for pipes: the bank side, -1 or +1), `z`, `alive`, `plugged`, and for waves on a 6-lane river `gap2`, the second gap lane. `kind` is `plastic`, `weed`, `log`, `wave`, `pipe`, `net` (spans `lane` and `lane + 1`, with `hp` cuts left), `current` (with `dir`, -1 or +1) or `rock` |
| `plugs` | Only gate codes this viewer is involved in, each with `code`, `clear`, `entered` and `left`. `clear` is true only for crewmates, or briefly for a lone boat. |
| `results` | `{scores, winner, health, choked, revived}` once the run is finished |

**Run event types:**
- collecting and missing: `collected`, `missed`, `cut`;
- boat state: `slowed`, `capsized` (with `dropped`, the plastic spilled), `bumped`, `boost`;
- pipes: `plug_started`, `plugged`, `plug_failed`;
- signals and waves: `ping`, `wave_incoming`;
- river mechanics: `net_hit {left}`, `net_freed`, `pushed {dir}`;
- river: `rule_changed {change, choice}`, `choked`.

When a run ends, every member's Ready flag resets and the room returns to the lobby.

### Room events (server → client)

- `room.joined`
- `room.member_joined`, `room.member_left`
- `room.member_disconnected`, `room.member_reconnected`
- `room.ready_changed`
- `room.expired`
- `game.state_updated`
- `error`, `ack`, `pong`

The earlier board game's commands are still accepted, but the client doesn't send them:
- `level.start`, `level.replay`, `level.continue`
- `player.action`, `turn.pass`
- `charter.vote`, `hint.request`

They produce the `level.*`, `game.*`, `charter.*` and `campaign.*` events. `tests/e2e/room.mjs` uses them as its
scripted game.

### Common error codes

| Area | Codes |
|---|---|
| Messages and access | `unauthorized`, `bad_message`, `unknown_command`, `not_member`, `not_host`, `rate_limited` |
| Rooms | `room_not_found`, `room_full`, `invalid_code`, `stale_state`, `database_unavailable` |
| Accounts | `invalid_handle`, `weak_password`, `handle_taken`, `invalid_credentials` |
| Runs | `run_in_progress`, `not_in_lobby`, `not_enough_players`, `not_all_ready`, `no_players`, `bad_river`, `river_locked`, `endless_solo` |
| Economy | `already_claimed`, `not_reached`, `not_enough_seeds`, `already_owned`, `not_refundable`, `not_owned` |
