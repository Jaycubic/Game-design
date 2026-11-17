# Clearwater Run

*A crew boat race to keep a river alive.* A multiplayer 3D browser game for **2–4 players on their own devices**
on the same local network. ("Clearwater Run" is a working title, set by `GAME_TITLE` in `client/src/ui/screens.ts`.)

## 1. What it is

Each player steers a small cleanup boat down one shared river for a **3-minute run**. You scoop up plastic, cut weed
mats, hop logs and plug the sewage pipes on the banks. Everyone in the room is **one crew** against the
pollution (a grumpy smog cloud sulks over every open pipe). The crew shares one score and one **River Health** bar: let it
reach zero and the river chokes (the score is halved); finish above 60 and the river is revived.

The environmental issue is **plastic and sewage pollution in rivers**, carried by the scoring rules rather than by text:
- plastic that slips past hurts the river;
- an open pipe drains it every second;
- greening the banks heals it.

The game uses no real place names; the real-world link belongs in the presentation.

- **Source code:** [github.com/Jaycubic/Game-design](https://github.com/Jaycubic/Game-design)
- **Play it:** [cleanwaterrun.work.gd:8081](https://cleanwaterrun.work.gd:8081) (when the server is running)
- **New to the game? Start here:** [docs/GAME_EXPLAINED.md](docs/GAME_EXPLAINED.md), every feature explained from
  the environmental problem up
- **Every feature and number in one document:** [docs/GAME_REFERENCE.md](docs/GAME_REFERENCE.md)
- **Rules:** [docs/RULE_BOOK.md](docs/RULE_BOOK.md)
- **Design and how the brief is met:** [docs/GAME_DESIGN.md](docs/GAME_DESIGN.md)

## 2. A run at a glance

| Time | What happens |
|---|---|
| 0:00 | 3-second countdown, then the boats move forward on their own. |
| 0:00–1:03 | Steer between 3–6 lanes (wider with more boats), collect plastic, hop logs, cut weed, plug pipes. A debris wave every 30 s. |
| ≈1:03 | **Rule change 1 – Twist:** everyone votes for one of three river modifiers (15 s). |
| ≈2:20 | **Rule change 2 – Finale:** the crew votes for one of two options (10 s). |
| 3:00 | Results: crew score, River Health (revived or choked), pipes fixed per player, Seeds earned. |

The two rule changes are **always chosen by players and happen exactly twice per run**. The controls, the scoring
formula and the win condition never change.

**Teamwork puzzle:** before each pipe, every player gets a big number (1–9) and presses it to send it. Whoever stops
at the pipe sees the numbers that were sent and enters them, with their own, smallest first. Playing alone, the lock
shows three numbers to enter smallest first.

## 3. Around the run

- **Hub:**
  - **PLAY** opens a crew room with a six-character invitation code.
  - **Practice solo** gives a run alone for half Seeds.
  - **∞ Endless** is a solo run with no clock where one capsize ends it; score is metres + plastic × 5.
  - **Join** takes a friend's code.
  - The **river chip** above PLAY picks the river.
- **Rivers:** Town Canal, Mangrove Maze (ghost nets) and Rapids Gorge (currents and rocks), unlocked by runs or
  stars. Each has three stars to earn and a best score.
- **Missions and ranks:** three daily missions that pay Seeds, and a rank from Rookie to River Guardian.
- **Tutorial:** a skippable 60-second scripted course. Each step spotlights the control it teaches (dim, ring,
  tapping hand, bubble). It is offered on first sign-in and is available later from the cap icon.
- **Phones and tablets:** on-screen buttons and/or swipes (chosen in Settings), with a layout for portrait and
  landscape. PCs default to the keyboard.
- **Seeds:** the only currency. Earned by playing, never bought.
- **Garage and Shop:** seven boats, from the starter rowboat to a jet boat and a cleanup tug. Each boat is a model
  with a person aboard, and each has four stats: speed, net, cutter and boost. Pricier boats are modestly better, at
  most 8 % faster than the starter.
- **Daily reward:** a 7-day track. A missed day pauses the track instead of resetting it.
- **River Road:** milestone rewards for runs completed, including two boats.
- **Settings:** graphics quality, sound (music, effects), comfort (screen shake, reduce flashing), on-screen controls
  (buttons + swipes, buttons only, swipes only or off; button size; left-handed), rebindable keys and the gamepad
  layout.

## 4. Architecture

```
Browser (Babylon.js 9, WebGPU → WebGL2)  ──HTTP + WebSocket──  Rust server (Actix)  ──  PostgreSQL
  client/  scene, HUD, menus, input            crates/game-server  accounts, rooms, run loop,
                                                 Seeds economy, persistence
                                               crates/river-core   deterministic run simulation
```

The server is authoritative. It steps each run at **30 Hz** and sends every player a snapshot 15 times a second.
Clients only send inputs (steer, hop, action, number, vote, pause).

| Folder | Contents |
|---|---|
| `crates/river-core` | The run: 3/4/6 lanes, obstacles, the three rivers, pipes and gate codes, River Health, scoring, stars, the two rule changes, boat stats, Endless, the tutorial course, and the in-run part of the adaptive difficulty. No I/O; seeded and deterministic; unit-tested. |
| `crates/game-server` | Actix HTTP and WebSocket on one port. Argon2id accounts, revocable tokens, rate limiting, rooms, the run loop (`river.rs`), the hub economy (`meta.rs`), rivers and stars (`rivers.rs`), missions and ranks (`missions.rs`), skill ratings (`director.rs`, see GAME_DESIGN §5.1), SQL persistence, optional TLS. |
| `crates/game-core` | The earlier turn-based board game. It is still compiled and tested, and its rooms code is reused, but the client no longer plays it. |
| `client/` | Vite + TypeScript + Babylon.js. River scene, boats and crew (`src/river/`), HUD, tutorial coach, menus, River Nature theme (`src/ui/theme.css`), icons. |
| `client/public/models/` | Kenney CC0 boat and character models, with their licence files. |
| `content/` | Data for the earlier board game. |
| `database/migrations/` | Versioned SQL schema. |
| `docs/` | Rule book, game design, API, database. |
| `scripts/` | Certificates, packaging, content formatting, remote development helper. |
| `tests/e2e/` | Live two-client shared-room test. |

Internal names still use the earlier project name: the package `natures-thread`, the server binary
`natures-thread-server`, the database `natures_thread` and the ZIP name. These are identifiers, not shown to players.

## 5. Tested environment

- Server: Ubuntu 22.04, Rust 1.94, PostgreSQL 17, Node 22, Python 3.10, OpenSSL 3.
- Client development: Windows 11, Node 24, a Chromium-based browser with WebGPU, on an Intel UHD 620 laptop GPU.
- Players: any current Chrome or Edge (WebGPU or WebGL2), or Firefox and Safari (WebGL2).

## 6. Prerequisites

| Tool | Version |
|---|---|
| Rust (cargo) | 1.82 or newer |
| Node.js / npm | 22 or newer |
| PostgreSQL server + `psql` | 14 or newer |
| Python 3 | 3.8+ (packaging) |
| OpenSSL, curl, GNU make, bash | any recent |

`make check-tools` checks for all of them.

## 7. Get the code and install dependencies

```bash
git clone https://github.com/Jaycubic/Game-design.git clearwater-run
cd clearwater-run
make setup
```

## 8. Configure `.env`

```bash
cp .env.example .env
```

Edit `.env` and set `DATABASE_URL` and a generated `JWT_SECRET`:

```bash
openssl rand -hex 48
```

The server refuses to start with a missing, short or placeholder secret. `.env` is git-ignored and never packaged.

## 9. Database

```bash
createdb natures_thread
make db-check
make db-migrate
```

Migrations also run automatically when the server starts. See [docs/DATABASE.md](docs/DATABASE.md).

## 10. Run the server and the client

```bash
make server-dev
```

```bash
make client-dev
```

The client opens on <http://localhost:5173> and proxies `/api` and `/ws` to the server. The server address comes
from `GAME_SERVER`, default `http://localhost:8070`. `make dev` runs both together.

`make run` runs the release server, which also serves the built client.

**Developing on Windows without a Rust toolchain:** `scripts/dev-remote.sh` copies the working tree to a scratch
folder on a Linux build machine and runs a command there:

```bash
DEV_REMOTE=user@host bash scripts/dev-remote.sh "make test"
```

## 11. Play on several devices (LAN)

1. Build and run on the server machine:
   ```bash
   make build
   ```
   ```bash
   make run
   ```
2. **HTTPS for WebGPU.** Browsers only enable WebGPU on secure pages:
   ```bash
   make certs
   ```
   - This creates `certs/` with a local certificate authority.
   - Add `TLS_CERT=certs/server.crt` and `TLS_KEY=certs/server.key` to `.env`, then restart the server.
   - Each player imports `certs/natures-thread-ca.crt` once as a trusted root.
   - Without HTTPS the game still works over plain HTTP, using WebGL2.
3. Open the firewall port once:
   ```bash
   sudo ufw allow 8070/tcp
   ```
4. Players open `https://<server-ip>:8070`, create accounts, and either join with the code or press PLAY.
5. To test alone, use two browser tabs. Each tab keeps its own sign-in, so two tabs are two players.

### Public deployment (own domain, HTTPS on its own port)

The game serves HTTPS itself on port **8081**, using a real certificate, so players need nothing installed. It leaves
any other web server on the machine, and ports 80/443, alone. Players open
`https://cleanwaterrun.work.gd:8081`.

One-time setup:

1. **Router:** forward external TCP port 8081 to the game server's port 8081 (the router only forwards 8081–8085; 8085 is already used on the server).
2. **On the server (sudo):**
   ```bash
   sudo chown "$USER": /opt/Game
   ```
   ```bash
   sudo ufw allow 8081/tcp
   ```
   ```bash
   sudo loginctl enable-linger "$USER"
   ```
   The last command lets the game start at boot and keep running after you log out.
3. **Certificate:** copy the full-chain certificate to `/opt/Game/certs/fullchain.cer` and its private key to
   `/opt/Game/certs/server.key`. Never put them in the repository.

Deploy, or update after a change, from a development machine:

```bash
DEPLOY_REMOTE=user@host bash scripts/deploy.sh
```

On each run the script:
- copies the source and builds it on the server;
- creates a production `.env` (fresh `JWT_SECRET`) and the database, the first time only;
- applies migrations;
- (re)starts the `clearwater-run` user service and checks `/health`.

To manage the service on the server:
- logs: `journalctl --user -u clearwater-run -f`
- restart: `systemctl --user restart clearwater-run`

**Certificate renewal.** The certificate comes from Let's Encrypt through acme.sh, installed in the server user's
`~/.acme.sh`, and lasts 90 days. The current one expires on 10 February 2027. Its DNS uses manual mode, so renewing
takes three steps on the server:

1. Request a new challenge:
   ```bash
   ~/.acme.sh/acme.sh --issue --dns -d cleanwaterrun.work.gd --keylength 2048 --force --yes-I-know-dns-manual-mode-enough-go-ahead-please
   ```
2. Add the printed `TXT value` as a TXT record `_acme-challenge` in the DNS panel. Wait until this shows it:
   ```bash
   dig +short TXT _acme-challenge.cleanwaterrun.work.gd @ns10.dnsexit.com
   ```
3. Finish, install into `certs/` and restart:
   ```bash
   ~/.acme.sh/acme.sh --renew -d cleanwaterrun.work.gd --yes-I-know-dns-manual-mode-enough-go-ahead-please
   ```
   ```bash
   ~/.acme.sh/acme.sh --install-cert -d cleanwaterrun.work.gd --fullchain-file /opt/Game/certs/fullchain.cer --key-file /opt/Game/certs/server.key
   ```
   ```bash
   systemctl --user restart clearwater-run
   ```

Don't run `--renew` before the record is visible. A failed check cancels that challenge, and you have to start
again from step 1.

## 12. Quality checks and builds

```bash
make fmt
```

```bash
make lint
```

```bash
make check
```

```bash
make test
```

```bash
make test-room
```

```bash
make build
```

`make test` runs:
- the river-core tests, covering determinism, crews, lane counts, every obstacle and river mechanic, capsizes,
  gate codes, River Health, stars, exactly two player-chosen rule changes (also in Endless), ties, the tutorial,
  boat stats, difficulty and a bot-played run;
- the server tests, including the Seeds reward rules, ratings, river unlocks, mission draws and rank rules;
- the earlier board-game tests;
- the client tests.

`make test-room` runs live checks with two WebSocket clients: room capacity, joining, membership privacy, stale and
invalid commands, and reconnection. It uses the earlier board levels as its scripted game.

## 13. Package the final ZIP

```bash
make package
```

```bash
make verify-package
```

`package` writes `dist/Natures-Thread.zip`. It refuses to include:
- `.env`, keys and certificates;
- `target/`, `node_modules/` and `.git/`;
- build output.

`verify-package` extracts the ZIP into a temporary folder, then builds and tests from that clean copy.

## 14. Troubleshooting

| Problem | Fix |
|---|---|
| `missing required variable(s): DATABASE_URL, JWT_SECRET` | Create `.env` from `.env.example`. |
| `cannot connect to PostgreSQL`, or `/ready` returns 503 | Start PostgreSQL, check `DATABASE_URL`, then run `make db-check`. |
| Port already in use | Change `HTTP_PORT` in `.env`; for the client dev proxy, change `GAME_SERVER`. |
| Other devices cannot connect | Make sure they are on the same network. Run `sudo ufw allow <port>/tcp` and use the server's LAN IP. |
| Corner label shows "WebGL2 · WebGPU needs HTTPS" | Use HTTPS (`make certs`) and import the CA on that device. |
| "Connection lost — reconnecting" | The client retries automatically, and your seat is kept. |
| "The game client has not been built" | Run `make build-client`. |
| Boats appear as simple shapes for a moment | The 3D models are still loading. A built-in stand-in is shown until they arrive. |
| Slow frame rate | Go to Settings → Graphics quality → Low. |

## 15. Known limitations

- Balance values (speeds, spawn rates, health changes, Seeds rewards) are not yet playtested.
- Gamepad input is implemented (D-pad or stick to steer, A to hop, B or X to act; the layout is listed in
  Settings), but it is untested on real pads.
- Touch controls are tested in a phone-sized browser view, not yet on a real phone.
- The puzzle in the current build is the gate code only. Pipe routing and a word hunt are planned.
- Rivers 4 and 5 (Night Delta, Harbour Port) are designed but not built.

## 16. Security

- Never commit real secrets. `.env`, `certs/`, keys and certificates are git-ignored and excluded from the ZIP, and
  the packager scans for secrets.
- Passwords are stored only as Argon2id hashes.
- Tokens are signed with `JWT_SECRET` and can be revoked.
- CORS is limited to `ALLOWED_ORIGINS`.
- The economy has no real money and no paid randomness. Every purchase can be undone for 10 minutes.
