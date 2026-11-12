# River run — core game design (vertical slice)

Working title **Clearwater Run** (rename later; one constant in code). Replaces the turn-based board as the main
game. Real place names stay out of the game (the issue is shown indirectly); they belong in the presentation.

## Intent
A game people want to play right now: one fast, simple activity in the first 10 seconds (Temple Run / Smash Karts
feel), with the environmental message carried by the scoring rules, not by text.

## The run (3 minutes)
- 2–4 players, each on their own device, in small cleanup boats on one shared river. Seats alternate into two
  crews (A, B). 2 players = one crew co-op against the river.
- Boats move forward automatically. Three controls: **steer** (left/right between 3 lanes), **hop** (jump
  obstacles), **action** (context: scoop plastic is automatic on contact; action cuts weed mats and plugs pipes).
- **Plastic** in lanes: collect for score and boost meter. **Weed mats**: slow you unless cut (cutting gives
  compost → greens the bank). **Logs/debris**: hop or capsize (2 s). **Sewage pipes** on the banks pour murk
  that drains the shared River Health; plugging one needs a 3-symbol **gate code** and gives the crew a lasting
  score multiplier. **Debris waves** sweep two lanes every 30 s.
- Crew score = plastic × (1 + 0.5 × pipes plugged) + 5 × banks greened.
- **Shared River Health** (both crews): falls while pipes run and plastic slips past; rises when pipes are plugged
  and banks greened. At 0 the river is "choked": everyone's result is halved. Ending above 60 = "River revived"
  bonus for all. Rivals compete on score but share the river.

## Communication and bluffing
- Gate code: the plugging boat sees it scrambled; its crewmate sees it clearly. Crewmates talk only through a
  **6-glyph ping wheel** (leaf, drop, fish, bridge, arrow, star) — visible to the rival crew too, so crews invent
  shorthand and decoys. A one-player crew sees its own code for 1.5 s (memory puzzle).

## Exactly two rule changes (player-chosen, never random)
Same ceremony both times: 5 s warning → full-screen card (icon + one sentence) → HUD badge → 3 s grace. The run
pauses during the card.
1. **Twist at 35 %** (≈63 s): 15 s vote, all players, among three river modifiers. Ties/no votes keep current rules.
   - *Monsoon Surge*: waves every 15 s, but twice as much plastic.
   - *Dry Season*: the river narrows to 2 lanes and pipes pour twice as fast.
   - *Volunteer Day*: plastic counts double, but weed mats spread.
2. **Finale at 78 %** (≈140 s): the trailing crew (or the only crew) picks one of two capped options.
   - *Second Wind*: the picking crew's boost fills and extra plastic spawns in their lanes for 20 s.
   - *Clean Sweep*: every unplugged pipe slows to half flow for the rest of the run (helps River Health for all).
Core rules (controls, scoring formula, win condition) never change.

## Puzzles
Gate codes (mental/logic, cooperative) in the slice. Pipe routing and a word hunt come later.

## Architecture (builder's call)
- `crates/river-core`: deterministic fixed-step simulation (30 Hz), seeded course generator, rule changes, scoring.
  Unit-tested; no I/O.
- Server: a room's "run" phase steps the sim on a timer, takes player inputs over the existing WebSocket, broadcasts
  snapshots at 20 Hz. Lanes are discrete, so no client prediction is needed on a LAN; the client animates lane
  changes smoothly. Existing accounts, rooms, lobby and persistence are reused.
- Client: new Babylon river scene (scrolling river, banks, procedural boats/props, chase camera), minimal HUD
  (timer, score, River Health, boost, ping wheel), rule-change cards, results. Keyboard first; gamepad later.

## Not in this slice
Hub/shop/economy, title/loading polish, market trading, word hunt, gamepad settings, multi-stretch shows.
The old board game stays in the repo, unused by the client for now.
