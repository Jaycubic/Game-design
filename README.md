# Nature's Thread

*Reconnect the Living World* — a cooperative 3D game for **2–4 players on their own devices**, about
reconnecting a fragmented Western Ghats rainforest.

## 1. What it is

Players are Nature Guardians restoring the Anamalai Forest Realm of India's Western Ghats. The environmental
theme is **biodiversity loss from habitat degradation and fragmentation**: roads, clearings and invasive Lantana
cut the forest into pieces that lion-tailed macaques, giant squirrels, hornbills, gliding frogs and Nilgiri tahr
cannot cross. There is no hidden saboteur — the opponent is the **Environmental Pressure Engine** (Lantana
spread, dry spells, monsoon run-off) and the scarcity of shared effort.

## 2. Two game modes

- **Puzzle levels (Discover)** — a top-down diorama board: plan corridors, match species to habitats, or deduce
  the right site from clues only the whole team holds. One plan, up to 3 checks.
- **Expedition levels (Restore)** — a free 3D camera over the same diorama, played in rounds with shared
  Restoration Capacity, seat-order turns and a pressure event each round.

Both modes share one landscape and rules; puzzles unlock Field Journal knowledge and insights used in
Expeditions, and Expeditions permanently change the regional map.

## 3. Shared rooms (2–4 devices)

One player signs in and **creates a room**; the game shows a six-character **invitation code** and a link.
Others open the game on their own device (same Wi-Fi/LAN), sign in, and enter the code. Everyone presses
**Ready**; the host starts. The server referees everything; each device renders the same state.

## 4. Gameplay loop

Level briefing → act (plant, restore, clear Lantana, bridges/groves) → check the plan / pass the turn →
pressure resolves → objective checked → result with up to three **Leaf Marks** (objective, efficiency against
the solver-proven optimum, optional ecological challenge) → next level. A failed level is replayed with no loss
of progress.

## 5. The Living Charter (exactly two amendments per region)

| After level | Amendment | Option A | Option B |
|---|---|---|---|
| 2 | Connectivity | **Continuous Corridors** — unbroken routes; unlocks canopy bridges over roads | **Stepping-Stone Networks** — a route may hop one gap beside a planted grove |
| 4 | Recovery | **Natural Succession** — degraded forest with 3+ healthy neighbours regrows free each round | **Active Restoration** — no regrowth; restoring costs 2 but heals the whole adjacent cluster |

Majority vote; a tie gets one re-vote, then the rotating tie-break player decides. The choice changes every later
Puzzle and Expedition (proven by the test suite). Full rules: [docs/RULE_BOOK.md](docs/RULE_BOOK.md).

## 6. Architecture

```
Browser (Babylon.js 9, WebGPU → WebGL2 fallback)  ──HTTPS/WSS──  Rust server (Actix)  ──  PostgreSQL
  client/  rendering, UI, input only                 crates/game-server  rooms, auth,      database/migrations
                                                      persistence, broadcast
                                                     crates/game-core   pure rules engine,
                                                      level solver, campaign, votes
```

| Folder | Contents |
|---|---|
| `crates/game-core` | Deterministic rules: board, species needs, connectivity under the Charter, pressure events, simulator, **A\* level solver/classifier**, campaign & votes, room roster rules. `bin/classify` prints the solver report. |
| `crates/game-server` | Actix HTTP + WebSocket on one port, Argon2id accounts, revocable tokens, rate limiting, room hub, SQL persistence, restart recovery, optional TLS. |
| `client/` | Vite + TypeScript + Babylon.js. Procedural diorama, vegetation and animals (no downloaded assets), HUD, screens. |
| `content/` | Region, the ten levels, Field Journal (with sources). Compiled into the server. |
| `database/migrations/` | Versioned SQL schema with integrity constraints and triggers. |
| `docs/` | Game design, rule book, API, database. |
| `scripts/` | Certificates, packaging, content formatting, remote dev helper. |
| `tests/e2e/` | Live two-client shared-room test. |

## 7. Tested environment

- Server: Ubuntu 22.04, Rust 1.94, PostgreSQL 17, Node 22, Python 3.10, OpenSSL 3.
- Client development: Windows 11, Node 24, Chromium-based browser with WebGPU (Intel UHD 620 laptop GPU).
- Players: any current Chrome/Edge (WebGPU or WebGL2) or Firefox/Safari (WebGL2).

## 8. Prerequisites

| Tool | Version |
|---|---|
| Rust (cargo) | 1.82 or newer |
| Node.js / npm | 22 or newer |
| PostgreSQL server + `psql` | 14 or newer |
| Python 3 | 3.8+ (packaging) |
| OpenSSL, curl, GNU make, bash | any recent |

`make check-tools` verifies them.

## 9. Get the code and install dependencies

```bash
git clone <your-repo-url> natures-thread   # or unzip Natures-Thread.zip
cd natures-thread
make setup
```

## 10. Configure `.env`

```bash
cp .env.example .env
```

Edit `.env`: set `DATABASE_URL` and a generated `JWT_SECRET`:

```bash
openssl rand -hex 48
```

The server refuses to start with a missing, short or placeholder secret. `.env` is git-ignored and never packaged.

## 11. Database

```bash
createdb natures_thread
make db-check
make db-migrate
```

With local peer authentication you need no password: `DATABASE_URL=postgres:///natures_thread?host=/var/run/postgresql&user=<os_user>`.
See [docs/DATABASE.md](docs/DATABASE.md).

## 12. Run the Rust server

```bash
make server-dev
```

or the release build that also serves the built client:

```bash
make run
```

## 13. Run the Babylon.js client

```bash
make client-dev
```

Opens on <http://localhost:5173> and proxies `/api` and `/ws` to the server (`GAME_SERVER`, default
`http://localhost:8070`). `make dev` runs server and client together.

## 14. Play on several devices (LAN)

1. Build and run on the server machine: `make build` then `make run`.
2. **HTTPS for WebGPU** — browsers only enable WebGPU on secure pages:
   ```bash
   make certs
   ```
   This creates `certs/` with a local CA. Put `TLS_CERT=certs/server.crt` and `TLS_KEY=certs/server.key` in
   `.env`, restart, and give each player `certs/natures-thread-ca.crt` to import once as a trusted root
   (Windows: double-click → Install → Trusted Root Certification Authorities; Android/iOS: install profile).
   Without it the game still works over plain HTTP using WebGL2.
3. Open the firewall port once: `sudo ufw allow 8070/tcp`.
4. Players open `https://<server-ip>:8070`, create accounts, and join with the code.
5. Test with two browser tabs on one machine: each tab keeps its own sign-in, so two tabs are two players.

## 15. Quality checks and builds

```bash
make fmt
make lint
make check
make test
make test-room
make classify
make build
```

`make test` runs the Rust unit tests, the solver proofs for all ten levels under every Charter combination
(≈4 minutes), the server room-logic tests and the client tests. `make test-room` runs 39 live checks with two
clients (starts the server if it is not running).

## 16. Package the final ZIP

```bash
make package
make verify-package
```

`package` writes `dist/Natures-Thread.zip` and refuses to include `.env`, keys, certificates, `target/`,
`node_modules/`, `.git/` or build output. `verify-package` extracts the actual ZIP into a temporary folder,
checks every required file, then builds and tests from that clean copy.

## 17. Troubleshooting

| Problem | Fix |
|---|---|
| `missing required variable(s): DATABASE_URL, JWT_SECRET` | Create `.env` from `.env.example`. |
| `cannot connect to PostgreSQL` / `/ready` 503 | Start PostgreSQL; check `DATABASE_URL`; run `make db-check`. |
| `database migration failed` | Read the message; fix the database; the migration is not marked applied, so re-run `make db-migrate`. |
| Port already in use | Change `HTTP_PORT` in `.env` (client dev proxy: `GAME_SERVER`). |
| Other devices cannot connect | Same network? Firewall: `sudo ufw allow <port>/tcp`. Use the server's LAN IP. |
| Corner label shows "WebGL2 · WebGPU needs HTTPS" | Use HTTPS (`make certs`) and import the CA on that device. |
| Browser warns about the certificate | The CA was not imported on that device, or the IP changed — re-run `make certs`. |
| "Connection lost — reconnecting" | The client retries automatically; your seat, turn and vote are kept. |
| "The game client has not been built" | `make build-client`. |
| Slow frame rate | Settings → Graphics quality → Low. |
| `npm ci` fails | Node 22+ required; delete `node_modules/` and retry. |

## 18. Known limitations

- Balance values (costs, rounds, thresholds) are solver-verified as fair and winnable but **not yet playtested**;
  in Level 6, Natural Succession is notably cheaper than Active Restoration.
- One region (Forest Realm, ten levels) is implemented; the 1,000-level structure exists as data layout only.
- The level 10 finale uses its own map rather than literally inheriting every earlier level's board; earlier
  results are kept per level and shown on the campaign map.
- The solver is exhaustive for the shipped levels; much larger custom levels may exceed its search budget
  (it then reports "unknown" rather than guessing).
- No single-player mode (the brief marks it optional); no audio yet.
- Animals are procedural low-poly models; no third-party CC0 models are bundled.

## 19. Security

Never commit real secrets. `.env`, `certs/`, keys and certificates are git-ignored and excluded from the ZIP
(the packager also scans for secrets). Passwords are stored only as Argon2id hashes; tokens are signed with
`JWT_SECRET` from the environment and can be revoked; the server has no fallback secret. CORS is limited to
`ALLOWED_ORIGINS`. Keep the local CA private key (`certs/natures-thread-ca.key`) on the server only.
