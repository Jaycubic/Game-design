# Nature's Thread — Database

PostgreSQL 14+ (developed against 17). Migrations live in `database/migrations/` and are **embedded in the
server** (sqlx). They run automatically at startup, or explicitly with `make db-migrate`
(`natures-thread-server --migrate-only`). Applied versions are recorded in `_sqlx_migrations`, so re-running is
safe; a failing migration stops the server with an error.

Static content — region, levels, species needs, events, Charter options, Field Journal — lives in `content/`
and is compiled into the server, so no rows are needed per tile or level.

## Tables

| Table | Purpose | Key constraints |
|---|---|---|
| `players` | Accounts | UUID PK; unique `handle` (`^[a-z0-9_]{3,24}$`); Argon2id `password_hash` only |
| `player_sessions` | One row per issued token (`jti`) | FK player; `expires_at`, `revoked_at` — logout revokes |
| `rooms` | Shared rooms | Join code unique **among open rooms** (partial unique index); `status` check; `min_players`/`max_players` 2–4 |
| `room_members` | Membership and seats | PK (room, player); unique active seat per room; trigger refuses more active members than `max_players`; `left_at` for departures |
| `regional_campaigns` | A room's ten-level run | One active campaign per room; `connectivity`/`recovery` checks; recovery only after connectivity |
| `room_state_snapshots` | Authoritative state by `version` | PK (campaign, version); last 20 kept; used for restart recovery |
| `room_events` | Ordered event log | Unique (room, seq) |
| `charter_amendments` | The two votes | PK (campaign, `amendment_number IN (1,2)`) → a third amendment or a second result is impossible; rule must match the slot; `selected` ⇔ `applied_at` |
| `player_clues` | Private Field Report fragments | PK (campaign, level, clue); FK holder |
| `level_results` | Every finished attempt | `outcome` won/lost; lost ⇒ 0 marks |
| `player_progression` | Totals and best marks | Totals grow only by improvements (no farming by replays) |
| `field_journal_entries` | Discoveries | PK (player, entry) — unlocked once |

Additional rules enforced by triggers:
- `room_members_capacity` — never more active members than the room allows (race-safe: locks the room row).
- `rooms_start_check` — a room may only move from `waiting` to `in_progress` with 2–4 active members.

## Transactions and concurrency

Each accepted command is written in one transaction: membership rows, effects (results, votes, clues,
progression), ordered events and the new snapshot. The in-memory state only becomes live after the commit; if
the database is unavailable the change is rejected (`database_unavailable`) and nobody's state changes. Clients
send the `version` they acted on; stale versions are rejected, which gives optimistic concurrency.

## Recovery

On startup the server loads the latest snapshot of every open room (`status` not `closed`/`expired`) and marks
all members disconnected; players reconnect to the same seat, level, turn and vote. Rooms idle longer than
`ROOM_IDLE_EXPIRY_HOURS` become `expired`.

## Commands

```bash
make db-check     # connectivity (credentials are never printed)
make db-migrate   # apply migrations, stop on error
make db-reset CONFIRM=yes   # development only: drops and recreates the schema
```

Local setup with peer authentication (no password in any file):

```bash
createdb natures_thread
# .env
DATABASE_URL=postgres:///natures_thread?host=/var/run/postgresql&user=<your_os_user>
```

All timestamps are `TIMESTAMPTZ` (UTC). No secrets are stored in SQL files or seeds.
