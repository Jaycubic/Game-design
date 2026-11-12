# Nature's Thread — HTTP & WebSocket API

One listener serves everything (default port **8070**; HTTPS when `TLS_CERT`/`TLS_KEY` are set).
All bodies are JSON. Errors are `{ "code": "<stable_code>", "message": "<safe explanation>" }`.

## HTTP

| Method & path | Auth | Purpose |
|---|---|---|
| `GET /health` | – | Liveness: `{status, service, version, rooms}` |
| `GET /ready` | – | Readiness: 200 when PostgreSQL answers, 503 otherwise |
| `GET /api/content` | – | Region, ten levels, Charter options, Field Journal. **Field Report clue texts are removed** (only `clue_count`). |
| `POST /api/auth/register` | – | `{handle, display_name, password}` → 201 `{token, expires_at, player}`. Rate-limited (10/min/IP). |
| `POST /api/auth/login` | – | `{handle, password}` → `{token, expires_at, player}`. Rate-limited. |
| `POST /api/auth/logout` | Bearer | Revokes the session (204). |
| `GET /api/me` | Bearer | `{id, handle, display_name}` |
| `GET /api/me/progress` | Bearer | Leaf Mark totals, best marks per level, Field Journal discoveries |
| `GET /api/me/rooms` | Bearer | Open rooms you belong to (for reconnecting) |
| `POST /api/rooms` | Bearer | Create a room → 201 `{room_id, code, state}` |
| `POST /api/rooms/join` | Bearer | `{code}` → `{room_id, code, state}`. 404 `room_not_found`, 409 `room_full`, 409 `level_in_progress`. Rate-limited (20/min/IP). |
| `GET /api/rooms/{id}` | Bearer, member | Your view of the room (403 for non-members) |
| `GET /api/rooms/{id}/results` | Bearer, member | Level results history |

`Authorization: Bearer <token>` — HS256 token whose `jti` is a row in `player_sessions` (revocable). Tokens are never stored or logged.

## WebSocket — `GET /ws?room=<room_id>`

Envelope (both directions):

```json
{ "v": 1, "id": "<message id>", "type": "<message type>", "payload": { } }
```

Server messages also carry `room`, `seq` (the authoritative state **version**) and `ts`.

1. **Authenticate first** (within 10 s): `{"type":"auth","payload":{"token":"…"}}`. Failure → `error` `unauthorized` and close.
2. Server replies **`room.joined`** with `payload.state` (your view).
3. Every accepted change is broadcast to each member with their own view: `{ events: [...], state }`.
   The envelope `type` is the last event's name.

### Client → server commands

| type | payload | Notes |
|---|---|---|
| `room.ready` | `{ready: bool}` | Lobby only |
| `level.start` | – | Host; needs 2–4 connected players, all ready |
| `level.replay` | `{level}` | Host; any unlocked level, between levels |
| `player.action` | `{action, base_version}` | `action` is one of `plant`/`restore`/`protect`/`bridge`/`grove` `{at:[x,y]}`, `place_species {species, at}`, `remove_species {species}`, `revert {at}` (puzzles), `answer {candidate}` (deduction), `end_round` (puzzle check). Expeditions: only the seat whose turn it is. |
| `turn.pass` | `{base_version}` | Expeditions; the round resolves when every connected seat has passed or capacity reaches 0 |
| `charter.vote` | `{choice: "A"\|"B"}` | During a vote; may change until all have voted |
| `level.continue` | – | Host, after a result: next level, Charter vote, retry, or region complete |
| `hint.request` | – | Solver's next optimal step, broadcast as `game.hint` |
| `room.leave` | – | Leave permanently (host passes to the next seat) |
| `ping` | – | → `pong` |

Every command with an `id` gets an `ack` (or an `error` with `ref` = that id). A repeated `id` is answered with `{duplicate: true}` and not applied again.

**Concurrency:** commands for a room are applied one at a time. `base_version` must equal the current version, otherwise `error stale_state` and the latest state is pushed to you — so two players acting on the same version cannot both succeed. Nothing changes unless the database write succeeds (`database_unavailable` otherwise).

### Server → client events

`room.joined`, `room.member_joined`, `room.member_left`, `room.member_disconnected`, `room.member_reconnected`,
`room.ready_changed`, `level.started`, `player.action_submitted`, `turn.passed`, `environment.event_revealed`,
`game.round_resolved`, `game.check_failed`, `game.answered`, `game.state_updated`, `game.hint`,
`player.clue_assigned` (**private** — only to the clue holder), `charter.vote_opened`, `charter.vote_submitted`,
`charter.revote`, `charter.tie_break`, `charter.amendment_applied`, `level.completed`, `level.failed`,
`campaign.progress_updated`, `campaign.region_complete`, `room.expired`, `error`, `ack`, `pong`.

### Error codes (WebSocket and HTTP)

`unauthorized`, `bad_message`, `unknown_command`, `not_member`, `not_host`, `room_not_found`, `room_full`,
`not_enough_players`, `not_all_ready`, `level_in_progress`, `stale_state`, `not_your_turn`, `use_pass`,
`wrong_terrain`, `not_a_site`, `not_allowed_by_charter`, `not_enough_capacity`, `not_threatened`,
`already_protected`, `not_a_slot`, `slot_taken`, `no_such_token`, `nothing_to_revert`, `level_over`,
`no_vote_open`, `not_eligible`, `not_tie_breaker`, `too_many_amendments`, `amendment_already_applied`,
`level_locked`, `database_unavailable`, `rate_limited`, `invalid_handle`, `weak_password`, `handle_taken`,
`invalid_credentials`, `invalid_code`.

### The state view (`payload.state`)

`{ id, code, version, status, phase, my_seat, members[], min_players, max_players, campaign{current_level, connectivity, recovery, best, insights, restored, vote}, play{level, rules, state{board, round, capacity_left, spent, protected, placements, attempts, status}, turn_seat, passed, threatened, round_capacity, events, evaluation, preview, last_report, par, my_clues, clue_counts}, result }`

`play.preview` is the server's Restoration Preview: what would happen if the round ended now (pressure, erosion, regrowth). `my_clues` contains only your own Field Report clues.
