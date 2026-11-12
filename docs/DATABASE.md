# Clearwater Run — Database

The game uses PostgreSQL 14 or newer and was developed against version 17.

Migrations live in `database/migrations/` and are **embedded in the server** through sqlx. They run automatically
at startup, or explicitly with `make db-migrate` (which runs `natures-thread-server --migrate-only`). Applied
versions are recorded in `_sqlx_migrations`, so re-running is safe, and a failing migration stops the server with an
error.

**Never edit a migration that has already been applied.** sqlx checks each file's checksum. Add a new numbered file
instead.

| Migration | Contents |
|---|---|
| `0001_initial_schema.sql` | Accounts, sessions, rooms, membership, plus the earlier board game's campaign tables |
| `0002_hub_economy.sql` | Seeds wallet, owned boats, daily claims, River Road claims, run rewards |

The run itself is never written row by row. It lives in server memory for its 3 minutes, and only the reward for each
player is stored when it ends.

## Tables

### Accounts and rooms

| Table | Purpose | Key constraints |
|---|---|---|
| `players` | Accounts | UUID primary key; unique `handle` (`^[a-z0-9_]{3,24}$`); stores only the Argon2id `password_hash` |
| `player_sessions` | One row per issued token (`jti`) | Foreign key to the player; `expires_at`, `revoked_at` (logout revokes) |
| `rooms` | Shared rooms | Join code unique **among open rooms** (partial unique index); `status` check; `min_players`/`max_players` between 2 and 4 |
| `room_members` | Membership and seats | Primary key (room, player); unique active seat per room; a trigger refuses more active members than `max_players`; `left_at` records departures |
| `room_state_snapshots` | Room state by `version` | Primary key (campaign, version); last 20 kept; used for restart recovery |
| `room_events` | Ordered room event log | Unique (room, seq) |

### Hub economy

| Table | Purpose | Key constraints |
|---|---|---|
| `player_wallets` | Seeds, runs, wins, best score, equipped boat | `seeds >= 0`, `runs >= 0`, `wins >= 0`; `equipped` defaults to `boat_sun` |
| `player_items` | Boats a player owns | Primary key (player, item); `source` is one of `default`, `shop` or `road`; `price` is kept for undo refunds |
| `daily_claims` | One daily reward per UTC day | Primary key (player, day): a second claim the same day is impossible |
| `road_claims` | River Road milestones collected | Primary key (player, milestone): each milestone is paid once |
| `run_rewards` | Every finished run's reward per player | `seeds >= 0`; indexed by (player, time) for the daily cap on the win bonus |

The boat catalog, daily track and River Road milestones are constants in `crates/game-server/src/meta.rs`. Boat stats
live in `crates/river-core` (`boat_stats`). None of them are stored as rows.

### Earlier board game

These tables belong to the earlier board game. They remain in the schema, but the river game doesn't write to them:
- `regional_campaigns`
- `charter_amendments`
- `player_clues`
- `level_results`
- `player_progression`
- `field_journal_entries`

### Triggers

- `room_members_capacity` — never allows more active members than the room allows. It is race-safe because it locks
  the room row.
- `rooms_start_check` — a room may only move from `waiting` to `in_progress` with 2–4 active members.

## Transactions and concurrency

- **Room changes:** each accepted command is written in one transaction. The in-memory state only becomes live after
  the commit. If the database is unavailable the change is rejected (`database_unavailable`).
- **Economy:** every action (reward, daily claim, milestone, purchase, undo, equip) is one transaction:
  - balances are checked inside it;
  - the primary keys above make double claims impossible, even when two requests arrive at once.

## Recovery

- On startup the server loads the latest snapshot of every open room, then marks all members disconnected.
- Players reconnect to the same seat.
- A run that was in progress when the server stopped is lost and pays nothing.
- Rooms idle longer than `ROOM_IDLE_EXPIRY_HOURS` become `expired`.

## Commands

```bash
make db-check
```

```bash
make db-migrate
```

```bash
make db-reset CONFIRM=yes
```

- `db-check` tests the connection; credentials are never printed.
- `db-migrate` applies migrations and stops on the first error.
- `db-reset` is for development only: it drops and recreates the schema.

Local setup with peer authentication needs no password in any file:

```bash
createdb natures_thread
```

```
# .env
DATABASE_URL=postgres:///natures_thread?host=/var/run/postgresql&user=<your_os_user>
```

All timestamps are `TIMESTAMPTZ` (UTC). No secrets are stored in SQL files or seeds.
