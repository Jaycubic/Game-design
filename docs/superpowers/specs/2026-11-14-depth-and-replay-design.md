# Clearwater Run — depth and replay (design, round 2)

Status: **approved** 2026-11-14 (with the guided tutorial added in §3.1) · steps 1–6 built

## Why

Playtesting (the owner's own play) says the core loop is fun, but:

- capsizing feels like a harsh time penalty;
- with 3–4 boats, three lanes are crowded and there isn't enough waste to go round;
- the six signal symbols are unclear;
- after a few wins there is nothing new to aim for.

The goal of this round is to keep players coming back without changing what already works: 3-minute runs, crews,
the shared River Health, gate codes and exactly two player-chosen rule changes per run.

## Goals

1. **A softer, fairer capsize** that still costs something.
2. **A wider river and more waste** when more players are on it.
3. **Signals people understand.**
4. **Adaptive difficulty** that learns each player's skill and keeps every room in "flow". It is documented here
   and in GAME_DESIGN.md, but **not shown in the rule book or the game's interface**.
5. **Long-term goals:** new rivers with new mechanics, three stars per river, missions, ranks, and best scores.
6. **Solo Endless mode** for a personal best.

## Non-goals

- No real money. No changes to the scoring formula or the win condition.
- No change to "exactly two player-chosen rule changes per run". It applies to every mode, Endless included.
- No hidden help for one crew against another (see §4).
- No trained machine-learning model yet; there's no data for one. The design logs the data a future model needs.

## 1. Capsize rework

| | Now | New |
|---|---|---|
| Time stopped | 2.0 s | **1.2 s** |
| After capsizing | can be hit again at once | **1.0 s** where obstacles pass through you (boat blinks) |
| Cost | time only | you also **drop 3 of your plastic**; it floats just ahead across nearby lanes, where you or anyone can net it again; the crew score loses it until someone does |
| Feedback | splash + banner | short freeze-frame (≈0.1 s, client only), camera shake, big splash, the boat flips and rights itself, "−3 ♻" pop |
| Warning | none | a red shadow under a log in your lane for the last 1.5 s before you reach it; the gap lane of a debris wave glows |

Hit-stop and shake are visual only; the server's simulation never pauses. A **"Reduce screen shake"** setting
(0–100 %) and **"Reduce flashing"** are added to Settings.

## 2. Lanes and waste scale with players

| Boats in the run | Lanes | Plastic spawn rate | Obstacle rate |
|---|---|---|---|
| 1 | 3 | ×1.0 | ×1.0 |
| 2 | 4 | ×1.3 | ×1.15 |
| 3–4 | 6 | ×1.8 | ×1.4 |

- **Dry Season** removes a third of the lanes (6 → 4, 4 → 3, 3 → 2) instead of exactly one.
- **Debris waves** leave one gap lane per 3 lanes, so 6 lanes have 2 gaps.
- **Starting lanes** are spread across the river.
- **Camera:** the chase camera keeps following your boat; at the current field of view the whole 6-lane river
  stays in view, so no extra zoom was needed (builder's call after a visual check).
- **Code:** lanes become a simple count (`lanes: u8`) instead of the fixed −1 / 0 / +1.

## 3. Signals people understand

The six symbols deliberately have no fixed meaning: crews invent their own code, and that is the bluffing
mechanic. The confusion comes from that never being explained. The fix:

- Each signal button shows a small number (1–6) and the symbol's name: Leaf, Drop, Fish, Bridge, Arrow, Star.
- The first time a gate code appears, a one-time tip says: "Your crewmate sees the real order. Tap the symbols to
  tell them — the other crew sees your signals too."
- The gate-code card shows each symbol with its name under it.
- How to play gets a short "Signals" section.
- Desktop gets a signal bar (bottom right) with the same six buttons, numbers and names; phones keep their
  touch buttons, now labelled too.

### 3.1 Guided tutorial (added at approval)

The owner asked for a tutorial that introduces every button in place, like the guided tours in big mobile
games. Each tutorial step now **spotlights** the control or meter it is about:

- the screen dims around it for about 2.5 s, then the dimming lifts so the river stays readable;
- a pulsing ring and a tapping hand point at it, and a bubble says what it does;
- nothing is blocked: the player uses the real control to finish the step.

| Step | Spotlight | Done when |
|---|---|---|
| 1 Steer | ◀ ▶ (touch) or the steer keys | two lane changes |
| 2 Grab plastic | Boost meter | three pieces |
| 3 River Health | River Health bar | 4.5 s |
| 4 Hop | ▲ | past the first log row |
| 5 Cut weed | ✂ | a mat cut (or both passed) |
| 6 Plug the pipe | ✂, then the gate-code card | plugged or failed (or both pipes passed) |
| 7 Signal your crew | the six signals | one signal sent (or 12 s) |

Skip stays available on every step.

## 4. Adaptive difficulty (the "River Director")

Internal name: **River Director**. It is described only in GAME_DESIGN.md and this document.

### 4.1 What it learns

Each player has a **skill rating**, stored in a new table `player_skill`:

| Column | Meaning |
|---|---|
| `rating` | starts at 1000 |
| `confidence` | how sure the estimate is |
| `runs` | runs counted |

After every finished run (not the tutorial), the server computes a **performance score** for each player, then
moves their rating towards it, more quickly while confidence is low. This is an online learning model, similar to
the Elo and Glicko ratings used in chess and ranked games. The performance score combines:

- plastic collected per minute, compared with the river's expected rate;
- capsizes per minute;
- pipes plugged;
- the change in River Health that the player's crew caused.

### 4.2 What it changes

A single **difficulty factor `d`**, limited to 0.8 – 1.2, scales spawn rates on the server:

| Value | Changes with `d` |
|---|---|
| Logs, weed mats and pipes | × d |
| Debris-wave interval | ÷ d |
| Plastic | × (2 − d), so a harder river is slightly less generous, and an easier one more |

It never changes:
- points per item or the score formula;
- River Health rules;
- the rule-change options;
- timings players react to (boat speed, hop length).

### 4.3 When it changes

- **Before a run:** `d` comes from the room's **average** skill rating. One value applies to the whole river, so
  crews compete on equal terms.
- **During a run, solo and co-op only** (one crew): every 10 s the Director compares recent performance with a
  "flow band" (e.g. collecting 55–80 % of plastic, at most one capsize per 30 s). It nudges `d` by ±0.03, up to
  ±0.1 from its starting value.
- **During a crew-vs-crew run:** `d` is fixed. Hidden help for one side would decide who wins.

### 4.4 Data for a future model

Each finished run records:
- the starting and final `d`;
- each player's performance score and rating before and after;
- whether the player played again within 24 hours.

With enough runs, this data can train a real model that predicts which settings keep players playing. That is
future work, stated as such.

### 4.5 Ethics

The Director aims for "flow", where a run is never trivially easy or hopelessly hard. It never sells anything,
never touches rewards in the shop, and treats both crews identically in competitive play. It is disclosed in the
design documentation.

## 5. Long-term goals

### 5.1 Rivers

Each river adds **one** new mechanic on top of everything before it. A river unlocks with River Road progress or
stars, whichever comes first.

| # | River | New mechanic | Unlock |
|---|---|---|---|
| 1 | **Town Canal** | (today's river) | start |
| 2 | **Mangrove Maze** | **Ghost nets**: an abandoned fishing net spans two lanes. Cutting it takes two action presses, or two crewmates one each; it frees trapped plastic | 3 runs or 3 ★ |
| 3 | **Rapids Gorge** | **Currents**: arrows on the water push your boat one lane sideways unless you hop; rocks replace logs in places | 8 runs or 7 ★ |
| 4 | **Night Delta** | **Darkness**: you see only your headlight cone and crewmates' lights; glowing plastic; signals matter more | 15 runs or 12 ★ |
| 5 | **Harbour Port** | **Oil slicks**: steering is delayed on oil; slicks spread from leaking barrels until plugged with a gate code | 25 runs or 18 ★ |

The host chooses the river in the lobby. This round builds **rivers 1–3**; 4–5 come next.

### 5.2 Stars

Each river has three stars, earned at the end of a run in that river:

| Star | How to earn it |
|---|---|
| ★ Clean river | finish with River Health ≥ 60 (revived) |
| ★★ Score | crew score at or above the river's target (e.g. Canal 120, Mangrove 150, Gorge 180) |
| ★★★ River goal | something different per river: Canal "plug every pipe", Mangrove "free 3 ghost nets", Gorge "no capsizes" |

Stars are kept per player and shown on each river's card.

### 5.3 Missions

- **Three daily missions** each day (UTC), drawn from a pool, e.g. "collect 60 plastic", "plug 2 pipes",
  "win a crew race", "earn a star".
- Each mission pays 20–40 Seeds.
- Missions count across runs and modes. Practice counts; the tutorial doesn't.

### 5.4 Ranks and scores

- **Rank:** the skill rating from §4 is shown as a rank. Seasons are out of scope.

  | Rating | Rank |
  |---|---|
  | below 900 | Rookie |
  | 900 | Deckhand |
  | 1050 | Skipper |
  | 1200 | Captain |
  | 1350 | River Guardian |

  The rank never goes down by more than one step in a day.
- **Best scores:** for each player and river, the best crew score and the best Endless distance. They're shown
  on the river card and the profile.
- **Results screen:** score, stars earned, mission progress, rank progress and "New best!".

## 6. Solo Endless mode

- Solo only, on any unlocked river.
- **One capsize ends the run.** It's the runner thrill, kept out of crew races.
- Boat speed rises 2 % every 15 s.
- **Two player-chosen rule changes** still happen: at 60 s the player picks one of 3 Twists, and at 150 s one of
  2 Finales.
- Score = distance (m) + plastic × 5.
- Pays Seeds like a practice run.
- Personal best per river.

## 7. Build order

1. Capsize rework, hit feedback, the shake and flashing settings, signal clarity.
2. Lanes and waste scaling with players.
3. The River Director: rating table, performance score, room `d`, the in-run controller for solo and co-op,
   and logging.
4. Rivers 2 and 3, the river picker in the lobby, stars, best scores.
5. Missions, ranks, and the results-screen summary.
6. Solo Endless.
7. Documentation: RULE_BOOK (everything except the Director), GAME_DESIGN (everything, including the Director),
   API, DATABASE.

Each step is tested with unit tests in `river-core`, server tests for the rating, missions and stars, and a
browser playthrough. It's deployed when it passes.

## Builder's calls (adjustable after playtesting)

All the numbers above, including:
- the spawn multipliers;
- the `d` range;
- the flow band;
- the star targets;
- the rank thresholds;
- the mission rewards;
- the Endless speed-up.
