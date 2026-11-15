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
| `0003_river_director.sql` | Skill ratings and the River Director's run log (internal, see below) |
| `0004_rivers.sql` | Stars, best score and runs per player per river |
| `0005_missions_ranks.sql` | Daily missions; shown rank columns on `player_skill` |

The run itself is never written row by row. It lives in server memory for its 3 minutes; when it ends, each player's
reward is stored, and the River Director logs one summary row for the run and one per player.

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
| `player_missions` | Three missions per player per UTC day: mission key, progress, goal, reward, `completed_at` (paid once, when set) | Primary key (player, day, slot); `slot` 0–2; `progress >= 0` |
| `player_rivers` | Each player's progress per river: `stars` as a bitmask (1 clean river, 2 score target, 4 river goal; never lost), `best_score`, `runs` | Primary key (player, river); `river` is `canal`, `mangrove` or `gorge`; `stars` 0–7 |

The boat catalog, daily track and River Road milestones are constants in `crates/game-server/src/meta.rs`; river
names, goals and unlock rules are in `crates/game-server/src/rivers.rs`. Boat stats
live in `crates/river-core` (`boat_stats`). None of them are stored as rows.

### River Director (internal)

The adaptive-difficulty system described in `docs/GAME_DESIGN.md` §5.1. Players never see these values.

| Table | Purpose | Key constraints |
|---|---|---|
| `player_skill` | One skill rating per player (starts at 1000), its `confidence` (0–1) and the `runs` counted; `shown_rank` (0–4) with `rank_day`/`rank_day_start`, so the shown rank falls at most one step a day | Primary key player; `confidence` between 0 and 1 |
| `director_runs` | One row per finished run (tutorials excluded): boats, crews, lanes, whether the Director could adapt in-run, starting and final difficulty `d_start`/`d_end`, final River Health | Serial id; `room_id` is kept even after the room closes |
| `director_players` | Each player's part in a logged run: performance score, rating before and after, plastic collected, capsizes, pipes plugged | Primary key (run, player); indexed by (player, run) |

"Played again within 24 hours", the signal a future model would learn from, is derived rather than stored:

```sql
SELECT p.run_id, p.player_id,
       EXISTS (SELECT 1 FROM director_players q JOIN director_runs rq ON rq.id = q.run_id
               WHERE q.player_id = p.player_id AND q.run_id > p.run_id
                 AND rq.finished_at <= r.finished_at + interval '24 hours') AS returned_24h
FROM director_players p JOIN director_runs r ON r.id = p.run_id;
```

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
