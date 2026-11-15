# Clearwater Run — Game Design Document

*A crew boat race to keep a river alive.* Multiplayer arcade racing with a cooperative twist · 2–4 players · browser,
local network · 3-minute runs.

## 1. Concept

Players race small cleanup boats down a polluted river. The game has to be fun within ten seconds: the boat moves on
its own, and there are only three controls (steer, hop, act). The environmental message lives in the **rules, not
in text**:
- every piece of plastic you miss hurts the river;
- every open sewage pipe drains it;
- the river's health decides everyone's result, whichever crew is ahead.

**Issue addressed:** plastic waste and untreated sewage choking urban rivers, and the "tragedy of the commons"
behind them: each group gains from ignoring the shared river until the river fails everyone. The game names no real
places; the real-world case is shown in the presentation.

## 2. How the assignment brief is met

| Requirement | How Clearwater Run meets it |
|---|---|
| Environmental awareness, addressed indirectly | River Health is a shared resource. Missed plastic and open pipes damage it; a choked river halves every score; a revived river rewards every crew. Players learn the lesson by losing points, not by reading. |
| Players change the rules dynamically, exactly twice | Every run has a **Twist** vote by all players at about 35 % of the run, and a **Finale** pick by the trailing crew at about 78 %. Both are player-chosen; a spin settles only ties and missing choices. The engine enforces exactly two changes in every mode (Endless too, at 60 s and 150 s), and tests prove it. |
| Cooperation | Crewmates share a score; 2 players form one co-op crew; all crews share River Health and the "revived" bonus. |
| Communication | Gate codes: the plugging player sees the symbols scrambled, and only a crewmate sees the real order. |
| Negotiation and bluffing | The Twist vote has to be argued out in the room. Signal glyphs are visible to the rival crew, so crews invent shorthand or send decoys. The Finale makes the trailing crew choose between chasing points (Second Wind) and helping everyone (Clean Sweep). |
| Puzzles | The gate code is a cooperative logic puzzle under time pressure. Solo players get a memory version: the code shows for 1.5 s. |
| Simple rules | Three controls, one score formula, one shared bar. A 60-second scripted tutorial spotlights each control in place and teaches it by doing. |
| Limited number of players | 2–4 per room, enforced by the server and the database. |
| Reasons to come back (beyond the brief) | Three rivers with new mechanics, three stars each, best scores, daily missions, ranks, solo Endless, and difficulty that adapts to each room (§5). |

## 3. The elements of game design

### 3.1 Players
- 2–4 players, each on their own device, in a shared room.
- 3–4 players split into two rival crews, alternating by seat. 2 players form one co-op crew.
- Practice solo and the tutorial are single-player.
- No player elimination: a capsize stops the boat for 1.2 s and spills 3 plastic just ahead (anyone can net it back), then gives 1 s of protection. A brief freeze-frame, camera shake and a red edge flash sell the hit; both can be reduced in Settings → Comfort.

### 3.2 Objectives
- **Crew:** a higher score than the rival crew.
- **Shared:** finish with River Health at 60 or more ("River revived", +20 for every crew).
- **Avoid:** River Health reaching 0 ("River choked", every score halved).
- **Long term:** earn Seeds to unlock and upgrade boats; complete River Road milestones.

### 3.3 Procedures
Steer between lanes (3, 4 or 6, by boat count), hop, press one context action (cut weed, cut a ghost net or
start plugging), type gate codes, send signals, and vote. Plastic is collected by contact.

### 3.4 Rules
See the [Rule Book](RULE_BOOK.md). The core rules — controls, score formula, win condition — never change. Only the
two player-chosen rule changes alter the river.

### 3.5 Resources
- **River Health:** shared, contested and visible to everyone.
- **Boost:** personal, charged by plastic.
- **Pipe multiplier:** per crew, raised by each plugged pipe.
- **Seeds:** the meta currency, earned only by playing (runs, daily reward, River Road, missions).
- **Stars and best scores:** per river, kept once earned; stars unlock rivers.

### 3.6 Conflict
- The river is the opponent: logs, weed, debris waves and pipes.
- The rival crew competes on score.
- The real tension is between scoring for your crew and spending time on pipes that protect everyone.
- There is no violence and no attacks. Running into the boat ahead only slows you for a moment.

### 3.7 Boundaries
One river course, generated from a seed, 3 minutes long. Its width depends on the boats in the run: 1 boat → 3
lanes, 2 → 4, 3–4 → 6. Wider rivers spawn plastic ×1.3 / ×1.8 and obstacles ×1.15 / ×1.4, so plastic grows
faster than danger and there is enough to go round. Dry Season closes a third of the lanes on the right. Debris
waves leave one gap per 3 lanes. Pipes are plugged from the outermost open lane on their bank.

### 3.8 Outcome
- A results screen shows crew scores, final River Health, whether the river was revived or choked, and the Seeds
  each player earned.

## 4. The two rule changes

| | Twist (~1:03) | Finale (~2:20) |
|---|---|---|
| Who chooses | Every player votes (15 s) | The trailing crew picks (10 s) |
| Options | Monsoon Surge, Dry Season, Volunteer Day | Second Wind, Clean Sweep |
| Tie or no choice | Visible spin among the tied options | Visible spin |
| Purpose | A collective, negotiated change to the river | Comeback help that can be selfish or shared |

Every option has an upside and a cost, so no choice is free. Both changes go through the same ceremony: a 5 s
warning, a pause, a full-screen card, a HUD badge, then 3 s of grace.

## 5. Meta-structure (the hub)

The hub follows the mobile pattern of a big PLAY button, side tiles and a currency chip, without the predatory parts.

| Feature | Design |
|---|---|
| Seeds | The only currency. Earned in runs (score ÷ 5, up to 60; +20 for a win, first 3 wins a day; +10 if revived). Practice pays half. |
| Garage and Shop | Seven boats with four stats each. Upgrades are capped and modest (at most +8 % speed). |
| Daily reward | A 7-day track. A missed day pauses it rather than resetting it: no streak anxiety. |
| River Road | Milestones for runs completed, unlocking two boats. |
| Fair-play rules | No real money, no loot boxes or paid randomness, a 10-minute undo on purchases, and a daily cap on the win bonus. |

### 5.0 Rivers, stars and best scores (replay value)

Three rivers, each adding one mechanic on top of everything before it, so there is always a next thing to learn:
Town Canal (the base game), **Mangrove Maze** (ghost nets across two lanes that need two cuts, which rewards
crewmates working together) and **Rapids Gorge** (currents that shove you sideways unless you hop, and rocks that
can't be hopped). A river unlocks with runs or stars, whichever comes first (3 runs / 3 ★, 8 runs / 7 ★), so
players who play well and players who play a lot both move on.

Each river has three stars (clean river, a score target, the river's own goal) that are kept once earned, and a
best score per player. The results screen shows both. Two more rivers (Night Delta, Harbour Port) are designed in the
round-2 spec for later.

### 5.0.1 Daily missions and ranks

- **Missions:** three a day per player from a pool of ten (plastic, pipes, weed, runs, a revived river, a crew win,
  a star, a capsize-free run, a 100-point run), drawn with a per-player, per-day seed and at most one plastic
  mission a day. Progress counts across runs and modes; each pays 20–40 Seeds automatically when done. Missions give
  a reason to try other parts of the game (pipes, weed, other rivers) rather than repeat one routine.
- **Ranks:** the River Director's rating shown only as a name (Rookie < 900, Deckhand 900, Skipper 1050, Captain
  1200, River Guardian 1350). The number itself is never shown. A rank rises at once but falls at most one step per
  UTC day, so a bad evening doesn't undo a week of progress.
- **Results screen:** score, stars, best score, mission progress (with Seeds paid) and rank progress.

### 5.0.2 Solo Endless

A personal-best mode for one player: no clock, one capsize ends the run, the boat speeds up 2 % every 15 s, and
score is metres + plastic × 5, kept per river. The two player-chosen rule changes still happen (Twist at 60 s,
Finale at 150 s), so the assignment's "rules change exactly twice" holds in every mode. The runner-style "one hit"
thrill is kept out of crew races on purpose, where a capsize only costs time and spilled plastic. Endless runs pay
like practice and count for missions, but don't feed the skill rating (they end on the first mistake by design).

### 5.1 River Director (adaptive difficulty, internal)

The River Director keeps each room in "flow": never trivially easy, never hopeless. It is a design-side system and
is deliberately **not** described in the rule book or anywhere in the game's interface.

**What it learns.** Every player has a skill rating (table `player_skill`, starting at 1000). After each finished
run (not the tutorial), the server scores the player's performance on the same scale, from:
- their share of the plastic that passed, against a fair share (all collected and missed plastic ÷ boats);
- capsizes per minute;
- pipes they plugged;
- the river's final health.

The rating then moves part of the way towards that score: halfway after the first run, less as runs add up, but
never less than 10 %. This is online learning in the style of the Elo and Glicko ratings used in chess. The code is
in `crates/game-server/src/director.rs`.

**What it changes.** One factor `d`, between 0.8 and 1.2:

| Value | Effect of `d` |
|---|---|
| Logs, weed mats and pipes | × d (denser rows, closer pipes) |
| Debris-wave interval | ÷ d |
| Plastic | × (2 − d): a harder river is a little less generous |

It never changes points, the score formula, River Health rules, rule-change options, boat speed or hop length.

**When.**
- *Before a run:* `d` comes from the average rating of everyone in the run (1000 → 1.0, ±200 → ±0.2). One value for
  the whole river.
- *During a solo or co-op run* (one crew): every 10 s of run clock, the Director compares the crew with a flow
  band: it should net 55–80 % of the plastic that passes, with at most one capsize per window. Below the band (or
  2+ capsizes) `d` drops by 0.03; above it with no capsizes `d` rises by 0.03. The total drift is capped at ±0.1
  from the starting value. The course is generated about 200 m ahead of the leading boat, so changes shape the
  river still to come and never pop obstacles in front of a player.
- *During a crew-vs-crew run:* `d` stays fixed. Hidden help for one side would decide who wins.

**Data for the future.** Each scored run is logged (`director_runs`, `director_players`): starting and final `d`,
each player's performance and rating before and after. Whether a player came back within 24 hours is derived from
the log (see `docs/DATABASE.md`). With enough runs this can train a model that predicts which settings keep people
playing; that is future work.

**Ethics.** The Director sells nothing, never touches rewards or the shop, and treats both crews identically in
competitive play. It is disclosed here, in the design documentation.

## 6. Look and feel

- **Theme "River Nature":**
  - moss-green boards in wooden frames;
  - honey-gold buttons with a pressable 3D edge;
  - the rounded Fredoka typeface;
  - one consistent SVG icon set (no emoji in menus).
- **3D models:** Kenney CC0 boats, each with a seated character (rowing or driving), on a code-built river and banks.
- **Rendering:** WebGPU where available, otherwise WebGL2.
- **Graphics quality setting:** keeps the game playable on an integrated laptop GPU.
- **Rivers:** each has its own look: town houses and footbridges (Canal), dense trees and dark water (Mangrove),
  rocky cliffs and boulders (Gorge).
- **Game feel:** a capsize gets a short freeze-frame, smooth trauma-based camera shake, a red edge flash and a full
  flip; logs in your lane get a red warning shadow, and wave gaps glow. Settings → Comfort scales the shake down and
  replaces flashes and blinking with steady tints.
- **Sound:** CC0 effects for every action, a river ambience and separate hub and race music, each with its own
  volume (see `THIRD_PARTY_NOTICES.md`).
- **Phones:** on-screen thumb buttons, swipes and a landscape-first layout.
- **Tutorial:** one step at a time (steer, boost meter, River Health, hop, cut weed, plug the pipe, signals). Each
  step spotlights its control or meter: the screen dims briefly, a ring and a tapping hand point at it, and a bubble
  explains it. It can be skipped at any point.

## 7. Emerging strategies

- **Pipe-first crews** take the multiplier early.
- **Plastic sweepers** fill boost and chain surges.
- **The signaller:** one crewmate reads codes while the other plugs.
- **Decoy signalling** confuses the rival crew.
- **Lane-splitting** within a crew covers more plastic.
- **Voting tactics:** Volunteer Day suits a plastic-heavy crew; Dry Season punishes crews that ignore pipes.
- **Net partners (Mangrove):** two crewmates cut a ghost net together, one press each, then split the plastic.
- **Current riding (Gorge):** let a current carry you into the plastic waiting beside it, or hop to hold your lane.

## 8. Design thinking process

1. **Empathise:** players wanted something immediately fun, like a casual racer or runner, not a "3D website".
2. **Define:** "Make players *feel* the cost of a shared, polluted river" — not "teach facts".
3. **Ideate:**
   - The first version was a turn-based cooperative board game about forest corridors (`crates/game-core`).
   - It was rigorous, with a solver-verified level set, but slow to learn and slow to play.
   - Research into casual multiplayer, runner and party games led to the river-run concept.
4. **Prototype:**
   - The deterministic simulation (`crates/river-core`) came first, so rules could be tested without graphics.
   - Then came the server loop, the 3D scene, the HUD, and the hub economy.
5. **Test:** automated tests (simulation, rule changes, rewards, client, live room test), plus the playtest plan
   below.
6. **Refine:**
   - A scripted tutorial was added.
   - Keys can be rebound.
   - The UI moved to a premium "River Nature" theme.
   - Boats became 3D models with crew and upgrade stats.
   - An airboat was replaced because its fan blocked the camera.

### Playtest plan
Give first-time players no briefing, then check:
- Can they play within 10 seconds?
- Can they explain River Health after one run?
- Do crews actually signal codes, and does anyone bluff?
- Do Twist votes cause discussion?
- Does the trailing crew ever choose Clean Sweep?

Rate fun and clarity separately for 2-player and 4-player rooms.

## 9. Known balance questions

- Spawn rates, health changes and Seeds rewards are untested with players.
- The boat upgrades are small on purpose. Confirm that new players never feel outclassed.
- Dry Season may be too harsh on crews that start far behind on pipes.
- Capsize cost (1.2 s, 3 plastic spilled, 1 s protection): check it stings without feeling unfair.
- Wide rivers (×1.8 plastic, ×1.4 obstacles on 6 lanes): check there is enough for four boats without clutter.
- River Director: the flow band (55–80 % of plastic, ≤ 1 capsize per 10 s window), the ±0.1 in-run swing and the
  rating-to-difficulty mapping are first guesses; the run log (`director_runs`) is there to tune them.
- Star targets (120 / 150 / 180), unlock thresholds (3 runs or 3 ★, 8 runs or 7 ★), mission rewards (20–40 Seeds)
  and rank thresholds are all adjustable after playtesting.
- Endless speed-up (+2 % per 15 s): check a strong player's run ends in a few minutes rather than never.
