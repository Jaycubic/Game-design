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
| Players change the rules dynamically, exactly twice | Every run has a **Twist** vote by all players at about 35 % of the run, and a **Finale** pick by the trailing crew at about 78 %. Both are player-chosen; a spin settles only ties and missing choices. The engine enforces exactly two changes, and a test proves it. |
| Cooperation | Crewmates share a score; 2 players form one co-op crew; all crews share River Health and the "revived" bonus. |
| Communication | Gate codes: the plugging player sees the symbols scrambled, and only a crewmate sees the real order. |
| Negotiation and bluffing | The Twist vote has to be argued out in the room. Signal glyphs are visible to the rival crew, so crews invent shorthand or send decoys. The Finale makes the trailing crew choose between chasing points (Second Wind) and helping everyone (Clean Sweep). |
| Puzzles | The gate code is a cooperative logic puzzle under time pressure. Solo players get a memory version: the code shows for 1.5 s. |
| Simple rules | Three controls, one score formula, one shared bar. A 60-second scripted tutorial teaches everything. |
| Limited number of players | 2–4 per room, enforced by the server and the database. |

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
Steer between three lanes, hop, press one context action (cut weed or start plugging), type gate codes, send signal
glyphs, and vote. Plastic is collected by contact.

### 3.4 Rules
See the [Rule Book](RULE_BOOK.md). The core rules — controls, score formula, win condition — never change. Only the
two player-chosen rule changes alter the river.

### 3.5 Resources
- **River Health:** shared, contested and visible to everyone.
- **Boost:** personal, charged by plastic.
- **Pipe multiplier:** per crew, raised by each plugged pipe.
- **Seeds:** the meta currency, earned only by playing.

### 3.6 Conflict
- The river is the opponent: logs, weed, debris waves and pipes.
- The rival crew competes on score.
- The real tension is between scoring for your crew and spending time on pipes that protect everyone.
- There is no violence and no attacks. Running into the boat ahead only slows you for a moment.

### 3.7 Boundaries
One river course, generated from a seed, three lanes wide (two under Dry Season), 3 minutes long.

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

## 6. Look and feel

- **Theme "River Nature":**
  - moss-green boards in wooden frames;
  - honey-gold buttons with a pressable 3D edge;
  - the rounded Fredoka typeface;
  - one consistent SVG icon set (no emoji in menus).
- **3D models:** Kenney CC0 boats, each with a seated character (rowing or driving), on a code-built river and banks.
- **Rendering:** WebGPU where available, otherwise WebGL2.
- **Graphics quality setting:** keeps the game playable on an integrated laptop GPU.
- **Tutorial:** teaches one control at a time with a coach card (steer, grab plastic, hop, cut weed, plug the pipe).
  It can be skipped at any point.

## 7. Emerging strategies

- **Pipe-first crews** take the multiplier early.
- **Plastic sweepers** fill boost and chain surges.
- **The signaller:** one crewmate reads codes while the other plugs.
- **Decoy signalling** confuses the rival crew.
- **Lane-splitting** within a crew covers more plastic.
- **Voting tactics:** Volunteer Day suits a plastic-heavy crew; Dry Season punishes crews that ignore pipes.

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
