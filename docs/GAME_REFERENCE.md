# Clearwater Run — Complete Game Reference

This is the full description of the game as it is built today: what it is about, every system with its exact
numbers, every mode and screen, and how the game is made, run and tested. It brings together what is spread across
the other documents.

| If you want… | Read |
|---|---|
| A gentle, beginner-friendly explanation | [GAME_EXPLAINED.md](GAME_EXPLAINED.md) |
| The rules as a player needs them | [RULE_BOOK.md](RULE_BOOK.md) |
| Design reasoning and how the brief is met | [GAME_DESIGN.md](GAME_DESIGN.md) |
| Server messages and HTTP endpoints | [API.md](API.md) |
| Database tables and migrations | [DATABASE.md](DATABASE.md) |
| Everything in one place, with numbers | **this document** |

All numbers below come from the code (`crates/river-core/src/lib.rs`, `crates/game-server/src/*.rs`,
`client/src/**`). If a number changes there, this document should change too.

---

## Contents

1. [The game in brief](#1-the-game-in-brief)
2. [The environmental theme](#2-the-environmental-theme)
3. [The assignment brief, point by point](#3-the-assignment-brief-point-by-point)
4. [Players, crew and modes](#4-players-crew-and-modes)
5. [A run from start to finish](#5-a-run-from-start-to-finish)
6. [Controls](#6-controls)
7. [The river](#7-the-river)
8. [Boats: movement, boost, capsizing](#8-boats-movement-boost-capsizing)
9. [Everything on the river](#9-everything-on-the-river)
10. [Sewage pipes and the number lock](#10-sewage-pipes-and-the-number-lock)
11. [River Health](#11-river-health)
12. [Score](#12-score)
13. [The two rule changes](#13-the-two-rule-changes)
14. [The three rivers](#14-the-three-rivers)
15. [End of a run: results, stars, best scores](#15-end-of-a-run-results-stars-best-scores)
16. [Endless](#16-endless)
17. [The tutorial](#17-the-tutorial)
18. [The hub and progression](#18-the-hub-and-progression)
19. [Daily missions and ranks](#19-daily-missions-and-ranks)
20. [Adaptive difficulty: the River Director (design-side)](#20-adaptive-difficulty-the-river-director-design-side)
21. [Screens and HUD](#21-screens-and-hud)
22. [Look, sound and game feel](#22-look-sound-and-game-feel)
23. [Settings](#23-settings)
24. [Phones and tablets](#24-phones-and-tablets)
25. [Fair play](#25-fair-play)
26. [How it is built](#26-how-it-is-built)
27. [Data that is stored](#27-data-that-is-stored)
28. [Deployment](#28-deployment)
29. [Testing](#29-testing)
30. [Credits and licences](#30-credits-and-licences)
31. [Known limits and future work](#31-known-limits-and-future-work)
32. [Every tuning number in one table](#32-every-tuning-number-in-one-table)

---

## 1. The game in brief

**Clearwater Run** ("Clearwater Run" is a working title) is a multiplayer 3D boat race that runs in a web browser.

- **1–4 players**, each on their own device (computer, phone or tablet), on the same network or over the internet.
- Everyone in a room is **one crew**. Together you race small cleanup boats down a polluted river for **3 minutes**.
- Boats move forward on their own. You only **steer**, **hop** and press one **action** button.
- You **collect plastic**, **cut weed**, **hop logs** and, most importantly, **plug sewage pipes**, which only works
  with your crew's help: each player presses their own number, and the fixer enters them smallest first.
- One shared **River Health** bar decides how the run ends: keep it at 60 or more and the river is revived;
  let it reach 0 and the river chokes.
- **Exactly twice** per run, the players **vote to change the rules**.
- Between runs, a hub offers boats, rivers, stars, daily missions, ranks and an Endless mode. Everything is earned
  by playing; there is no real money anywhere.

Play it at **https://cleanwaterrun.work.gd:8081** (when the server is running). Source:
[github.com/Jaycubic/Game-design](https://github.com/Jaycubic/Game-design).

---

## 2. The environmental theme

**The issue:** plastic waste and untreated sewage choking rivers in fast-growing cities, and the *tragedy of the
commons* behind it: each person or group gains a little by ignoring a shared river, until the river fails everyone.

**The lesson the rules teach:**
1. **A shared resource only survives if someone looks after it.** River Health is shared by the whole crew.
2. **Stop pollution at its source.** Collecting plastic helps a little, but open sewage pipes keep draining the river
   every second. Plugging a pipe is the single biggest thing you can do, and it makes all other cleaning count for
   more (the score multiplier).
3. **Fixing the source is never a one-person job.** A pipe's lock needs at least one crewmate's number on top of the fixer's own.

**How it is told:** indirectly, through rules and consequences rather than text. The water turns from blue-green to
murky brown as River Health falls; missed plastic hurts the river; open pipes drain it; a grumpy brown smog cloud
sulks over every open pipe and disappears when the pipe is plugged.

**Real-world links inside the mechanics:** water hyacinth-like weed mats (fed by sewage), outfall pipes, flood
debris waves, monsoon and dry-season effects on pollution, abandoned "ghost" fishing nets, volunteer clean-up days.

**No real place names** are used in the game; the real river case belongs in the presentation.

---

## 3. The assignment brief, point by point

| Requirement | How Clearwater Run meets it |
|---|---|
| Environmental awareness, addressed indirectly | The whole score and outcome depend on a shared River Health bar; pollution is felt through lost points, murky water and choked rivers, not lectures. |
| Players change the rules dynamically, **exactly twice** | Every run has a **Twist** vote (~1:03) and a **Finale** vote (~2:20), both chosen by the players. The engine enforces exactly two changes in every mode (Endless too, at 1:00 and 2:30), and automated tests prove it. |
| Cooperation | One crew, one score, one River Health bar. |
| Communication | Pipe number locks: each player gets one number per pipe and chooses to send it; the fixer calls for numbers ("press your number!") and needs at least one. |
| Negotiation | Both rule changes are crew votes argued out loud; crews also plan who fixes which pipe. (Bluffing was optional in the brief and is not used.) |
| Puzzles | Number lock: put the crew's numbers in increasing order under time pressure, while still steering. Alone: three numbers, smallest first. |
| Simple rules | Three controls, one score formula, one shared bar, a 60-second tutorial. |
| Limited number of players | 1–4 per room, enforced by the server and the database. |
| No violence | Nobody attacks anybody. Running into a boat ahead only slows you for a moment. |

---

## 4. Players, crew and modes

### Crew

| Players in the run | Crew |
|---|---|
| 1 | You alone |
| 2–4 | Everyone in **one crew** |

The crew shares one score, one River Health bar and one result. There are no rival crews.

### Modes

| Mode | Players | Where to start it | Length | Pays |
|---|---|---|---|---|
| **Crew run** | 2–4 | Hub → PLAY → room → everyone Ready → host starts | 3 min | Full Seeds |
| **Practice solo** | 1 | Hub → Practice solo | 3 min | Half Seeds |
| **Endless** | 1–4 | Hub → ∞ Endless (alone), or room → ∞ Endless (crew) | Until lives run out | Half Seeds |
| **Tutorial** | 1 | Offered on first sign-in; cap icon in the hub | 60 s | Nothing |

### Rooms

- **PLAY** creates a room with a **6-character invitation code**. Friends join by typing it (or a `?join=CODE` link).
- Rooms hold **2–4** players for crew runs (server config `ROOM_MIN_PLAYERS=2`, `ROOM_MAX_PLAYERS=4`).
- A crew run starts when at least 2 connected players are ready; the host presses **Start the run**.
- A disconnected player keeps their seat for **120 s** (`ROOM_RECONNECT_GRACE_SECONDS`); idle rooms expire after
  **12 hours**.
- The **host** chooses the river (the river chip in the lobby).

---

## 5. A run from start to finish

| Time | What happens |
|---|---|
| 0:00 | **3-second countdown**, then the boats start moving on their own. |
| 0:00–1:03 | Race: collect plastic, cut weed, hop logs, fix pipes with number locks. A **debris wave** about every 30 s. |
| ~0:58 | **5-second warning**: "The river is about to change". Racing continues. |
| ~1:03 | **Rule change 1, Twist.** The run pauses; everyone votes (15 s). A card reveals the result (3 s). |
| ~2:15 | 5-second warning: "Final twist incoming". |
| ~2:20 | **Rule change 2, Finale.** The run pauses; everyone votes (10 s); reveal (3 s). |
| 3:00 | **Results**: crew score, River Health (revived / choked), pipes fixed per player, stars, best score, missions, rank, Seeds. |

The run clock only advances while the river is moving (pauses for votes and reveals do not count).

---

## 6. Controls

| Action | Keyboard | Gamepad | Phone / tablet |
|---|---|---|---|
| Steer one lane left / right | ◀ ▶ or A / D | D-pad or left stick | ◀ ▶ buttons, or swipe left / right |
| Hop | ▲, W or Space | A | ▲ button, or swipe up |
| Action: cut weed or a net, open a pipe lock | ▼, S or E | B or X | ✂ button, or swipe down |
| Send your pipe number / type a lock (1–9) | keys 1–9 | hold RT + A/B/X/Y/LB/RB (the 1st–6th number on screen) | tap your big number, or the lock's numbers |
| Vote in a rule change | keys 1–3 | A / B / X | tap a card |
| Leave a pipe lock | ▲ (hop) | A | ▲ |
| Pause / quit | **Esc** or **P** (opens the menu) | — | **⏸** top-left corner |

- Arrow keys always work; the letter keys can be rebound in Settings.
- Pipe numbers are plain digits **1–9**: no symbols to learn.

---

## 7. The river

### Width grows with the crew

| Boats | Lanes | Plastic spawn | Obstacle spawn |
|---|---|---|---|
| 1 | 3 | ×1.0 | ×1.0 |
| 2 | 4 | ×1.3 | ×1.15 |
| 3–4 | 6 | ×1.8 | ×1.4 |

- Lanes are numbered from 0 on the left. Boats start spread across the river.
- The course is generated from a random seed, **200 m ahead of the leading boat** as the run goes, so difficulty
  changes affect only the river still to come. A normal course is 3,300 m long; Endless never ends.
- **Debris waves** cover every lane except a gap: one gap per 3 lanes (two gaps on a 6-lane river). Gap lanes glow.
- **Dry Season** (a Twist) closes a third of the lanes on the right: 6 → 4, 4 → 3, 3 → 2; the closed lanes turn to mud.

---

## 8. Boats: movement, boost, capsizing

| Property | Value |
|---|---|
| Base speed | 14 m/s (× the boat's speed stat) |
| Hop | 0.75 s in the air (clears logs, weed, nets, waves and currents; not rocks) |
| Boost | Full meter → **×1.5 speed for 2.5 s**; weed can't slow you while boosting |
| Slowed (weed, net) | ×0.55 speed for 1 s |
| Bump (running into a boat ahead in your lane) | slowed for 0.4 s |
| **Catch-up current** | A boat more than **25 m** behind the front boat sails **15 % faster** until it rejoins |

### Capsizing

Hitting a log, a rock or a debris wave without hopping (rocks: even when hopping) **capsizes** the boat:
- it stops for **1.2 s** and **spills 3 plastic** just ahead (anyone can net it back; the crew's plastic count drops
  until then);
- then for **1.0 s** obstacles pass through it (the boat blinks);
- the boost and any open lock are cancelled.

Feedback: a short freeze-frame (client only), trauma-based camera shake, a red edge flash, the boat flips all the
way over, "−3 ♻". Shake and flashing can be reduced in Settings → Comfort. A **red shadow** appears under a log or
rock in your lane for the last 1.5 s before you reach it.

---

## 9. Everything on the river

| Object | Effect | Response |
|---|---|---|
| **Plastic** (bottles, bags) | +1 crew plastic, boost +12 % (base), River Health +0.5. Escaping plastic (passed by every boat): River Health −0.8. | Drive into it (pickup window ±0.9 m, wider with a better net). |
| **Weed mat** | Slows you (×0.55 for 1 s). | Action within ~5 m ahead cuts it: **+5 crew score** ("bank greened"), River Health +1.5. |
| **Log** | Capsizes you. | Hop. |
| **Debris wave** (~every 30 s) | Capsizes you, except in the gap lane(s). | Steer to a gap, or hop. |
| **Sewage pipe** (on a bank) | Drains River Health **0.12 per second** from when the front boat is within 30 m, until plugged. A smog cloud sulks above it. | Number lock (section 10). |
| **Ghost net** (Mangrove) | Spans two lanes; slows like weed. | Two Action presses (by anyone) free it: **4 plastic** released, River Health +3. |
| **Current** (Gorge) | Pushes you one lane sideways (once per current). | Hop to ride over it. |
| **Rock** (Gorge) | Capsizes you even when hopping. | Steer round it. |

---

## 10. Sewage pipes and the number lock

### Finding a pipe

- Pipes are large and set into the bank, with a wide brown plume and a smog cloud above.
- A floating **🔧 marker** shows the distance to the nearest open pipe ahead. With friends it also suggests whose
  turn it is ("Asha's turn?" = the player who has fixed the fewest pipes this run; a hint, not a rule).
- A steady **pipe card** under the timer says which bank, how many metres, and the next step:
  "Move ◀ left to the edge lane" → "Stay in this lane — press ▼ when you reach it" → "Press ▼ NOW to plug it!".
- You plug from the **outermost open lane** on the pipe's side, within about 7 m before to 3 m after it.

### Crew number lock (2–4 players)

1. **Numbers are dealt** when the front boat is **90 m** from the next open pipe. Each player gets a **different**
   number **1–9** and sees **only their own**, big ("YOUR NUMBER 7"), for **8 s**; it **blinks** in its last 3 s.
   Only one pipe's numbers are live at a time; the next pipe's come once the current pipe is fixed or everyone has
   passed it.
2. **Players press their number** (keys 1–9, tap the big number, or RT + a face button). The server keeps it for the
   pipe; the card becomes a calm "✔ Your 7 is in" and a ✔ pops over that boat. A wrong number is ignored (the card
   shakes). Not pressing = not taking part in this pipe.
3. **A fixer presses Action** beside the pipe. Their boat stops; the **lock** shows a tile per crewmate in seat order:
   the fixer's own number, every number sent so far, and "…" for players who haven't pressed. Those players get their
   number back on screen, blinking, with "Your crew needs it!" (and a buzz on phones) while the lock is open.
4. **The fixer types the numbers smallest first** (increasing order) within **12 s**. It needs **at least one
   crewmate's number** besides their own. The first number typed **locks in** the set (later presses don't join this
   attempt). A number out of order is ignored, with no penalty.
5. **Success:** the boat stays put **1 s** while the crew member hammers (sparks, clanks); the pipe is plugged,
   **River Health +10**, crew pipes +1 (multiplier up), the smog cloud disappears.
6. **Time out:** **River Health −5**, the pipe keeps pouring, every open lock on that pipe closes, and **new numbers**
   are dealt.
7. **▲ (hop) leaves a lock** at any time, with no penalty.
8. **Several fixers** may open locks at the same pipe. Sent numbers show on all of them; the **first** to finish wins
   and the other locks close.
9. **Away players'** (paused, disconnected, quit) numbers send themselves.

### Solo number lock (1 player, and the tutorial)

Three different numbers 1–9 show in a random order (e.g. 7 · 2 · 5) and **stay on screen**; type them **smallest
first** (2, 5, 7) within **8 s**. ▲ leaves. A number out of order is ignored; running out of time costs River
Health −5.

### Why it matters

Plugging pipes is the main teamwork moment, the biggest River Health gain and the only way to raise the score
multiplier. Racing ahead and ignoring a teammate's call costs the whole crew.

---

## 11. River Health

| Event | Change |
|---|---|
| Start | 70 / 100 |
| Plastic collected | +0.5 |
| Weed cut | +1.5 |
| Ghost net freed | +3 |
| Pipe plugged | +10 |
| Plastic escaped | −0.8 |
| Each open pipe | −0.12 per second |
| Failed lock / wrong code | −5 |

- **Choked:** River Health reaches 0 at any time → the crew's final score is **halved**.
- **Revived:** River Health is **60 or more** at the end → **+20** to the crew score, and it counts as a win.
- The water colour, fog and murkiness follow River Health.

---

## 12. Score

```
Crew score = plastic × (1 + 0.5 × pipes plugged) + 5 × banks greened
```

- Example: 40 plastic, 2 pipes, 3 weed mats cut → 40 × 2 + 15 = **95**.
- The **multiplier badge** (×1.5, ×2, ×2.5 …) sits next to the crew score once a pipe is plugged.
- Volunteer Day (a Twist) makes each plastic count double.
- End adjustments: choked → halved; revived → +20.
- Endless uses its own score (section 16).

---

## 13. The two rule changes

Every run changes its rules **exactly twice**, and the players choose both. Each change: a 5 s warning while racing
→ the run pauses → the crew votes → a full-screen card shows the result for 3 s → a HUD badge stays for the rest of
the run. A tie goes to the **first vote cast** among the tied options (the card names that player); only no votes at all means a **visible spin**. Controls, the score formula and the win condition
never change.

### Twist (~1:03, 15 s to vote)

| Twist | Upside | Downside | Real-world idea |
|---|---|---|---|
| **Monsoon Surge** | Extra plastic appears | Debris waves every 15 s | Heavy rain washes more waste into rivers |
| **Dry Season** | Fewer lanes to watch | A third of the lanes close; pipes pour twice as fast | Low water concentrates pollution |
| **Volunteer Day** | Each plastic counts double | Weed mats keep spreading | Clean-ups help, nature pushes back |

### Finale (~2:20, 10 s to vote)

| Finale | Effect |
|---|---|
| **Second Wind** | Every boat's boost fills, and extra plastic flows into the crew's lanes for 20 s |
| **Clean Sweep** | Every open pipe pours at half rate for the rest of the run |

---

## 14. The three rivers

Each river adds one mechanic on top of everything before it. The river chip above PLAY (and in the lobby, for the
host) chooses it.

| River | New mechanic | Look | Unlocks at | Star 2 target | Star 3 goal |
|---|---|---|---|---|---|
| **Town Canal** | — | Houses, steps, lamps, footbridges | Start | 120 | Plug every pipe you pass |
| **Mangrove Maze** | Ghost nets | Dense trees, dark water | 3 runs **or** 3 ★ | 150 | Free 3 ghost nets |
| **Rapids Gorge** | Currents and rocks | Rocky cliffs, boulders | 8 runs **or** 7 ★ | 180 | Finish without capsizing |

Two more rivers (Night Delta: darkness; Harbour Port: oil slicks) are designed but not built.

---

## 15. End of a run: results, stars, best scores

The **results card** shows:
- a title: "River revived! You saved it together", "The river choked", or "Run complete" ("Endless run over" in
  Endless);
- the crew score and River Health;
- **pipes fixed per player** (crew runs);
- the river's **three stars** from this run (new ones marked "NEW") and the **best score** on that river;
- **daily missions** progress ("+25 paid" when one is completed);
- the player's **rank** and progress to the next;
- Seeds earned (as a banner below the card).

During the results, all racing UI (boost bar, quit button, your number, pipe card…) is hidden.

### Stars (per player, per river, kept forever)

| Star | Rule |
|---|---|
| ★ Clean river | River Health 60 or more at the end |
| ★★ Score | Crew score at or above the river's target |
| ★★★ River goal | The river's own goal (table above) |

Stars help unlock rivers. Endless and the tutorial earn no stars.

---

## 16. Endless

- **No clock.** Alone: **one capsize ends the run.** Crew (2–4): **3 shared lives** (❤️❤️❤️ under the timer); each
  capsize uses one; the run ends at 0 lives or if the river chokes.
- Boats speed up **2 % every 15 s** (×1.02 per step).
- Still **exactly two** player-chosen rule changes: Twist at **1:00**, Finale at **2:30**.
- Pipes use the number lock (crew or alone).
- **Score = metres travelled + 5 × plastic collected.** The best Endless score is kept per player per river.
- Pays half Seeds; counts for missions; no stars; does not change the skill rating.

---

## 17. The tutorial

A scripted **60-second** course on Town Canal, skippable at any moment, paying nothing. Seven steps, each one
**spotlighting** the control or meter it teaches (the screen dims for ~2.5 s, a pulsing ring and a tapping hand
point at it, a bubble explains it):

1. **Steer** (◀ ▶): two lane changes.
2. **Grab plastic** (boost meter): three pieces.
3. **River Health** (the bar): 4.5 s.
4. **Hop** (▲): past the first log row.
5. **Cut weed** (✂): a mat cut (or both passed).
6. **Plug the pipe** (✂, then three numbers smallest first): plugged or failed (or both pipes passed).

---

## 18. The hub and progression

| Feature | Details |
|---|---|
| **Seeds** 🌱 | The only currency, earned only by playing. Per run: crew score ÷ 5 (max 60), **+20** for a win (revived river; first 3 wins a day), **+10** if revived; half in practice/Endless; nothing in the tutorial. |
| **Garage** | Choose your boat. All boats are shown; locked ones say how to get them. |
| **Shop** | Buy boats with Seeds; any purchase can be undone within **10 minutes** for a full refund. |
| **Daily reward** | Once per UTC day: 20, 25, 30, 40, 50, 60, then 100 Seeds, repeating. A missed day pauses the track; it never resets. |
| **River Road** | 1 run: 50 + Kingfisher boat · 3: 75 · 5: 100 · 8: 150 · 10: 100 + River Keeper boat · 15: 300 Seeds. |
| **Rivers** | River chip above PLAY: cards with stars, goals, best scores and unlocks. |
| **Missions** | Hub tile: today's three missions (section 19). |
| **Rank badge** | Next to your name (section 19). |

### Boats (stats 1–5; level 2 = the starter)

| Boat | How to get it | Speed | Net | Cutter | Boost |
|---|---|---|---|---|---|
| Driftwood (rowboat) | Starter | 2 | 2 | 2 | 2 |
| Kingfisher (fishing boat) | River Road, 1 run | 2 | 3 | 2 | 2 |
| Coral Skiff | Shop, 120 Seeds | 3 | 2 | 2 | 3 |
| Bluefin | Shop, 120 Seeds | 3 | 3 | 2 | 2 |
| Mangrove | Shop, 200 Seeds | 3 | 3 | 3 | 3 |
| Night Heron (jet boat) | Shop, 300 Seeds | 4 | 3 | 3 | 4 |
| River Keeper (cleanup tug) | River Road, 10 runs | 3 | 5 | 4 | 3 |

Per level above 2: **Speed** +4 % · **Net** +15 % pickup reach (level 5 also scoops the next lane) · **Cutter**
+15 % cutting reach · **Boost** +15 % charge per plastic. Upgrades are deliberately small (at most +8 % speed).

---

## 19. Daily missions and ranks

### Missions

Three per player per **UTC day**, drawn from a pool of ten with a per-player, per-day seed (at most one plastic and
one pipe mission a day). Progress counts in every run, practice and Endless included (not the tutorial). Each pays
its Seeds **automatically** the moment it is done.

| Mission | Goal | Seeds |
|---|---|---|
| Collect 60 plastic | 60 | 20 |
| Collect 150 plastic | 150 | 35 |
| Plug 2 sewage pipes | 2 | 25 |
| Fix a pipe yourself | 1 | 25 |
| Cut 6 weed mats | 6 | 20 |
| Finish 3 runs | 3 | 20 |
| Finish a run with the river revived | 1 | 30 |
| Earn a star on any river | 1 | 30 |
| Finish a run without capsizing | 1 | 30 |
| Reach a crew score of 100 in one run | 1 | 25 |

### Ranks

| Rank | From rating |
|---|---|
| 🛶 Rookie | below 900 |
| ⚓ Deckhand | 900 |
| 🧭 Skipper | 1050 |
| 🎖️ Captain | 1200 |
| 🌊 River Guardian | 1350 |

The rank is the skill rating (section 20) shown only as a name; the number is never shown. A rank rises at once and
falls **at most one step per UTC day**.

---

## 20. Adaptive difficulty: the River Director (design-side)

> Documented for the design team; deliberately **not** described in the Rule Book or shown in the game.

- **Skill rating:** every player starts at 1000. After each finished run (not the tutorial or Endless), the server
  scores the player's performance (share of passing plastic, capsizes per minute, pipes fixed, final River Health)
  on the same scale (500–1600) and moves the rating towards it: halfway after the first run, then by
  `1 / (runs + 2)`, never less than 10 % (online Elo/Glicko-style learning).
- **Room difficulty `d`** (0.8–1.2) comes from the average rating of the players: 1000 → 1.0, ±200 → ±0.2.
  Obstacles and pipes × d, debris waves d times as often, plastic × (2 − d).
- **In-run nudges:** every 10 s of run clock, the Director compares the crew with a **flow band** (netting 55–80 %
  of passing plastic, fewer than 2 capsizes per window) and changes `d` by ±0.03, at most ±0.1 from the start.
  Active in every crew and solo run; off in Endless and the tutorial.
- **Never touched:** points, the score formula, River Health rules, rule-change options, boat speed, hop length,
  rewards or the shop.
- **Logged** (tables `director_runs`, `director_players`) for tuning and a future trained model.

---

## 21. Screens and HUD

### Screens
1. **Title**: logo, animated river behind, "Press any key / Tap to start".
2. **Sign in / New player**.
3. **Welcome** (first time): offers the tutorial.
4. **Hub**: player bar (Seeds, rank, name, tutorial, how to play, settings, sign out); tiles (Garage, Shop, Daily,
   River Road, Missions); river chip; PLAY; Practice solo; ∞ Endless; join by code; recent rooms.
5. **Lobby**: invitation code and link, seats with ready marks, river chip (host), Start the run / ∞ Endless /
   Practice solo (host), I'm ready.
6. **Run** (HUD below).
7. **Results** (section 15).
8. **Panels**: Garage, Shop, Daily, River Road, Rivers, Missions, How to play, Settings.

### Run HUD

The HUD is one grid of named **slots** (`client/src/river/run.css`, "HUD layout"). Every piece belongs to exactly one
slot, and a slot stacks its pieces, so two pieces never land on top of each other on any screen size.

| Slot | What it holds |
|---|---|
| Top | ⏸ Pause · crew score · timer (or metres in Endless), rule badges, lives · **River health** |
| Notices (under the top bar) | Tutorial coach card · pipe card (bank, distance, next step) · banners (at most 2; 1 at a lock or on short screens) |
| Lower | Your big number (blinking near the end, or while a crewmate's lock waits for it; a small "✔ Your 7 is in" once sent) · the lock card |
| Bottom | Boost meter, with key hints (PC, left) |
| Over the world | Name tags, pipe marker, ✔ bubbles (a number sent), "+1 ♻" pop-ups (lowest layer) |
| Full screen | Countdown, votes, reveals, results (above the HUD; the HUD steps back while they show) |

When pieces compete for room: at a lock the lock card wins and the pipe card hides (you're there); a fixer's number card
hides too (their number is on the lock), and while your big number is calling, a crewmate's lock card steps aside.
Thumb buttons own the bottom corners on touch screens, and the HUD reserves that space. Layers, low to high: world markers
< HUD < thumb buttons < full-screen moments < pause menu < tutorial spotlight. Dev builds have `__audit()` and
`__stress()` in the browser console to list any overlap.

---

## 22. Look, sound and game feel

- **Style:** "River Nature" theme: moss-green boards in wooden frames, honey-gold buttons with a pressable edge, the
  Fredoka typeface, one consistent SVG icon set in menus.
- **3D:** Babylon.js 9 with **WebGPU** (WebGL2 fallback). Boats and crew are Kenney CC0 models; the river, banks,
  houses, plants, plastic, weed, logs, nets, currents, pipes, plumes and the smog cloud are built in code.
- **Per-river looks** (section 14); water colour follows River Health.
- **Sound:** CC0 effects for every action (pickup, hop, cut, capsize, bump, boost, plug, wrong number, number sent,
  countdown, go, rule change, win, lose), a looping river ambience, and separate hub and race music.
- **Game feel:** capsize freeze-frame, trauma-based shake, red edge flash, boat flip, warning shadows, glowing wave
  gaps, sparks and clanks while fixing, pop-ups, banners, spotlight tutorial.

---

## 23. Settings

| Section | Options |
|---|---|
| Graphics quality | Low / Medium / High (reloads; the room is kept) |
| Sound | Mute; Volume; Music; Effects |
| Comfort | Screen shake 0–100 %; Reduce flashing (no blinking or red flashes). Reduced-motion devices start gentler. |
| On-screen controls | **Buttons + swipes** (phone default) · Buttons only · Swipes only · **Off** (PC default); button size S/M/L; left-handed layout |
| Keys | One rebindable key per action (arrows always work); "Reset to W A S D" |
| Gamepad | Layout list; connection status |

Settings are saved per browser.

---

## 24. Phones and tablets

- Full-screen layout using `100dvh` and safe-area insets; separate layouts for landscape phones and portrait
  (portrait suggests turning the phone sideways).
- Thumb buttons (◀ ▶ bottom left, ✂ ▲ bottom right), the big number and lock numbers are tappable, swipe gestures on
  the river (36 px threshold), haptic buzzes.
- The ⏸ Pause button sits in the top-left corner on every device (labelled "Pause" on wide screens); the top bar moves right to make room. Solo runs really pause on the server (`run.pause`, auto-resume after 10 minutes). In crew runs the paused player's boat parks (stops, can't be hit, doesn't hold the crew back, number sends itself) while the others keep racing; on Resume it rejoins just behind the crew. Players who quit or disconnect are parked the same way, and votes don't wait for them. Reloading the page or closing the tab leaves the room (like Quit): a refresh lands on the hub, never back in the old run.
- Touch wording everywhere ("tap ✂", "swipe down") follows the chosen control scheme.

---

## 25. Fair play

| Possible loophole | How the rules stop it |
|---|---|
| Ignoring the river and only farming plastic | A choked river halves the score. |
| Nobody pressing their number | The lock can't open without a crewmate's number; a timed-out lock costs River Health −5 for the whole crew. Your number blinks until you press it. |
| One player fixing every pipe | The marker suggests whoever has fixed the fewest; results show each player's pipes. |
| Being stranded after fixing | The catch-up current. |
| Random or forced rule changes | Both are crew votes; ties go to the first vote, and only no votes spin, visibly. |
| Farming wins for Seeds | Win bonus only for the first 3 wins a day; practice and Endless pay half. |
| Paying to win | No real money; upgrades capped and small. |
| Regretted purchases | 10-minute undo. |
| Missing a day | The daily track pauses, never resets. |
| Grinding missions | Each pays once; three a day. |
| A bad day ruining a rank | Ranks fall at most one step a day. |
| Reading another player's number | Numbers are sent privately per player; only the fixer sees sent numbers. |

---

## 26. How it is built

```
Browser (Babylon.js 9, WebGPU → WebGL2)  ── HTTPS + WebSocket ──  Rust server (Actix)  ──  PostgreSQL
  client/  scenes, HUD, menus, input, audio        crates/game-server  accounts, rooms, run loop, economy,
                                                                        rivers, missions, ratings, TLS
                                                    crates/river-core   the deterministic run simulation
```

| Part | Technology | Role |
|---|---|---|
| `crates/river-core` | Rust, no I/O | The whole run: lanes, objects, rivers, capsizes, number locks, River Health, scoring, stars, rule changes, Endless, tutorial course, in-run difficulty. Fixed **30 Hz** step; seeded and deterministic. |
| `crates/game-server` | Rust, Actix-web, SQLx, rustls | HTTP API and WebSocket on one port; Argon2id accounts, revocable JWT sessions, rate limiting; rooms; the run loop (`river.rs`), economy (`meta.rs`), rivers and stars (`rivers.rs`), missions and ranks (`missions.rs`), ratings (`director.rs`); serves the built client; optional TLS. |
| `client/` | TypeScript, Vite, Babylon.js | Title, hub, lobby, run scene, HUD, tutorial coach, input (keyboard, gamepad, touch), audio, settings. |
| `database/migrations/` | PostgreSQL | Versioned schema, applied at server start. |
| `crates/game-core` | Rust | The earlier board-game prototype; still compiled and tested, not used by the client. |

**Server-authoritative:** clients only send inputs (steer, hop, action, number, vote, start, pause, quit). The server steps
each run at 30 Hz and sends every player a snapshot **15 times a second**, filtered per viewer (each player's own
number, and only the lock information they're allowed to see), plus events every tick.

Size: about 15,700 lines of Rust and TypeScript (excluding styles and tests data).

---

## 27. Data that is stored

| Table | Contents |
|---|---|
| `players`, `player_sessions` | Accounts (Argon2id hashes only) and revocable sessions |
| `rooms`, `room_members`, `room_state_snapshots`, `room_events` | Rooms, seats, recovery |
| `player_wallets`, `player_items`, `daily_claims`, `road_claims`, `run_rewards` | Seeds economy and audit |
| `player_rivers` | Stars (bitmask), best score, best Endless score, runs per river |
| `player_missions` | Three missions per player per day, progress, completion |
| `player_skill` | Skill rating, confidence, runs, shown rank and its daily floor |
| `director_runs`, `director_players` | Difficulty log per run and per player |

A run itself lives in server memory for its 3 minutes; only results, rewards and logs are written. Migrations
`0001`–`0006` (see [DATABASE.md](DATABASE.md)).

---

## 28. Deployment

- **Live:** `https://cleanwaterrun.work.gd:8081`, served by the game server itself with TLS (Let's Encrypt
  certificate via acme.sh, manual DNS renewal before February 2027).
- **Server:** Ubuntu, systemd user service `clearwater-run`, PostgreSQL database `natures_thread`.
- **One command:** `DEPLOY_REMOTE=user@host bash scripts/deploy.sh` packs the project, builds the client and the
  server on the machine, runs migrations, restarts the service and checks `/ready`.
- **Development:** Vite dev server on `localhost:5173` proxying to a dev game server; builds and Rust tests run on
  the Linux server.

---

## 29. Testing

| Suite | Count | Covers |
|---|---|---|
| `river-core` unit tests | 52 | Determinism; one crew; lane counts and spawns; every obstacle and river mechanic; capsizes and spill; crew number locks (dealing, privacy, sending, increasing order, waiting for a crewmate, time-out, skip, several fixers, away players, one pipe at a time); solo number lock; fixing stop; catch-up current; River Health; stars; exactly two rule changes (including Endless); vote ties; tutorial; boat stats; difficulty; a full bot-played run |
| `game-server` tests | 19 | Seeds rules, rating maths, river unlocks and star masks, mission draws and amounts, rank rules, room rules |
| Client tests | 2 | Input mapping |
| Live checks | — | Two-client room test; two-bot crew-lock run; browser playthroughs on desktop and phone sizes |

---

## 30. Credits and licences

- **3D models:** Kenney Watercraft Kit and Mini Characters (CC0).
- **Sound:** OpenGameArt (rubberduck, RandomMind; CC0), Kenney Interface/Impact Sounds and Music Jingles (CC0),
  Tallbeard Studios Music Loop Bundle (CC0).
- **Font:** Fredoka (SIL OFL), loaded from Google Fonts.
- **Libraries:** Babylon.js (Apache-2.0); Rust crates under MIT/Apache-2.0/ISC.
- Full list: [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).

---

## 31. Known limits and future work

- Balance numbers are first guesses; they need playtests with real groups (see section 32).
- Gamepad and touch controls are implemented but not yet tried on real devices.
- Rivers 4–5 (Night Delta, Harbour Port) are designed, not built.
- A leaderboard of crew scores is a possible next step.
- The River Director logs data for a future trained model.

---

## 32. Every tuning number in one table

| System | Number |
|---|---|
| Run length | 180 s (tutorial 60 s) |
| Simulation | 30 Hz; snapshots 15 Hz |
| Countdown / warning / Twist vote / Finale vote / reveal | 3 s / 5 s / 15 s / 10 s / 3 s |
| Twist / Finale time | 63 s / 140 s (Endless 60 s / 150 s) |
| Base speed | 14 m/s |
| Boost | ×1.5 for 2.5 s; +12 % per plastic (base) |
| Slow | ×0.55 for 1 s; bump 0.4 s |
| Hop | 0.75 s |
| Capsize | 1.2 s stop, 1.0 s protection, 3 plastic spilled |
| Catch-up current | +15 % when > 25 m behind |
| Lanes | 3 / 4 / 6 for 1 / 2 / 3–4 boats |
| Spawn multipliers | plastic ×1.0 / 1.3 / 1.8, obstacles ×1.0 / 1.15 / 1.4 |
| Debris waves | every 30 s (Monsoon 15 s), divided by difficulty; 1 gap per 3 lanes |
| Course generated ahead | 200 m; normal course 3,300 m |
| Key deal / show / lock time | 70 m / 3 s / 12 s |
| Solo code | shown 2 s, 8 s to enter |
| Fix stop | 1 s |
| Failed lock | River Health −5 |
| River Health | start 70; plastic +0.5; weed +1.5; net +3; pipe +10; missed −0.8; pipe drain −0.12/s |
| Revived / choked | ≥ 60 → +20 / 0 → score halved |
| Score | plastic × (1 + 0.5 × pipes) + 5 × banks |
| Ghost net | 2 cuts, 4 plastic |
| Star targets | 120 / 150 / 180 |
| River unlocks | 3 runs or 3 ★; 8 runs or 7 ★ |
| Endless | +2 % speed per 15 s; score = metres + 5 × plastic; crew 3 lives |
| Seeds per run | score ÷ 5 (max 60); +20 win (3/day); +10 revived; half in practice/Endless |
| Daily track | 20, 25, 30, 40, 50, 60, 100 |
| Missions | 3 a day, 20–40 Seeds |
| Ranks | 900 / 1050 / 1200 / 1350 |
| Difficulty | d 0.8–1.2; ±0.03 per 10 s; max ±0.1 in-run; flow band 55–80 % |
| Rooms | 2–4 players; 6-character codes; 120 s reconnect grace; 12 h idle expiry |
| Refund window | 10 minutes |
