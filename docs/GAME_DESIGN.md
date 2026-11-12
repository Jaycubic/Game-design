# Nature's Thread — Game Design Document

*Reconnect the Living World* · Cooperative ecological puzzle and restoration game · 2–4 players · browser, LAN

## 1. Concept

Nature's Thread is a cooperative game about **biodiversity loss caused by habitat degradation and fragmentation**, set in the **Western Ghats** of India — a UNESCO World Heritage Site and global biodiversity hotspot whose rainforests have been split by roads, plantations and clearings. Players never read a lecture: they discover *why* a lion-tailed macaque cannot cross a tea estate, *why* a hornbill needs a fig near its nest, and *why* frogs vanish when streamside trees are cut, by trying to fix a living landscape together.

The issue is addressed **indirectly**: the rules themselves embody ecology (routes, habitat needs, regrowth, invasive spread), so winning requires understanding it.

## 2. Design pillars

| Pillar | How the game delivers it |
|---|---|
| Discover | Puzzle levels teach one idea at a time; Field Journal entries unlock after play. |
| Restore | Expeditions turn knowledge into shared action on a 3D landscape. |
| Adapt | The Environmental Pressure Engine changes the situation; the Living Charter changes the rules twice. |
| Witness | Every Expedition shows current state, restoration preview and resulting state; the regional map keeps the results. |
| Progress | Ten levels reveal mechanics gradually; level 10 combines everything. |

## 3. The elements of game design

### 3.1 Players
2–4 cooperating **Nature Guardians**, each on their own device in a shared online room on the local network. No hidden roles, no sabotage, no player elimination. Seats give turn order in Expeditions and receive private clues in Field Report puzzles. *Role:* players are a team whose communication is part of the play.

### 3.2 Goals
- **Level goals:** explicit objectives such as "canopy route West ↔ East", "hornbill nest with a live fig in reach", "forest beside the stream for the frogs", "26 healthy forest cells".
- **Mastery goals:** three Leaf Marks per level — objective, efficiency (spend no more than the solver-proven optimum), optional ecological challenge.
- **Regional goal:** complete all ten levels and restore the Forest Realm.
*Role:* goals translate ecological outcomes (connectivity, habitat suitability, forest cover) into clear win conditions.

### 3.3 Rules
- Interventions: plant (1), restore (1/2), clear Lantana (1), canopy bridge (2), stepping-stone grove (2).
- Connectivity: orthogonal links through habitat suitable for the species.
- Species needs: macaque (fig + 6 connected forest), squirrel (4 connected forest), hornbill (nest tree + fig ≤ 3 cells), frog (forest beside a stream), tahr (grassland linked to cliffs).
- Pressure events resolve at round end; then natural regrowth if the rules allow.
- **Dynamic rules:** two Living Charter amendments (Connectivity after level 2, Recovery after level 4), chosen by majority vote, permanent for the region.
Full player rules: [RULE_BOOK.md](RULE_BOOK.md). *Role:* rules model ecological cause and effect simply enough to reason about.

### 3.4 Space
Three nested scales: **World map** (ecosystem families) → **Forest Realm map** (six areas: North Canopy, West Woodland, Central Clearing, East Woodland, South Shola Meadow, River Edge) → **level board**, a grid diorama of 30–110 cells. Puzzles are viewed top-down like a board game; Expeditions use a free 3D camera over the same diorama. *Role:* adjacency **is** connectivity — the spatial layout is the puzzle.

### 3.5 Time
Puzzles: a single planning phase with up to 3 checks. Expeditions: 3–5 rounds, each with reveal → discuss → act → resolve. Pressure events are scheduled per round; natural regrowth happens between rounds. *Role:* time creates urgency (erosion before the monsoon, Lantana before it spreads) and makes "wait for regrowth" a real but limited strategy.

### 3.6 Resources
One shared resource: **Restoration Capacity**, refreshed each round (reduced by dry spells). No currencies or trading economy. *Role:* scarcity forces negotiation — there is always more to fix than capacity allows.

### 3.7 Conflict
The opponent is the **Environmental Pressure Engine**: Lantana spread, dry spells, monsoon run-off, and the critical-deterioration threshold. Internal tension comes from **competing priorities** (route vs. hornbill vs. frog vs. forest cover) and from **voting** on rule changes. *Role:* conflict without violence or villains.

## 4. Social mechanics

| Mechanic | Where |
|---|---|
| Cooperation / teamwork | Shared capacity, shared objectives, one landscape. |
| Cryptic communication | Level 5 Field Reports: each player holds different clues; only the combination identifies the site. |
| Negotiation | Spending scarce capacity; Living Charter votes with revote and rotating tie-break. |
| Logical puzzles | Habitat connection, niche matching, deduction, ecosystem logic, blueprint planning. |
| Bluffing | Deliberately excluded: the design removed the hidden "Polluter" role so conflict comes from the environment. |
| Trading | Not used; one shared pool keeps the game simple. |

## 5. Level plan (Western Ghats Forest Realm)

| # | Mode | Title | New idea | Solver-proven minimum capacity |
|---|---|---|---|---|
| 1 | Puzzle | The Missing Connection | Planting a corridor | 3 |
| 2 | Expedition | Restore the Forest | Rounds, shared capacity, Lantana | 4 |
| — | Charter | Amendment 1: Connectivity | Continuous Corridors vs Stepping-Stone Networks | |
| 3 | Puzzle | Who Belongs Here? | Species needs (4 species) | 2 / 2 |
| 4 | Expedition | A Forest in Balance | Competing priorities | CC 6 / SS 5 |
| — | Charter | Amendment 2: Recovery | Natural Succession vs Active Restoration | |
| 5 | Puzzle | The Missing Clue | Private clues, cooperative deduction | answer depends on connectivity rule |
| 6 | Expedition | Under Pressure | Dry spell + two Lantana fronts | see `classify` |
| 7 | Puzzle | The Ecosystem Puzzle | All four forest species + route | see `classify` |
| 8 | Expedition | Nature Under Threat | Erosion, multi-zone | see `classify` |
| 9 | Puzzle | The Final Blueprint | Capstone plan | see `classify` |
| 10 | Expedition | A Living Forest | Full regional map, every mechanic | see `classify` |

Exact per-ruleset values live in each level file's `par` field and are re-verified by the test suite. Run `make classify` for the full report.

## 6. The level classifier

`crates/game-core` contains an exhaustive A* solver. For every level and every Living Charter combination it:

- proves the level winnable (or reports why not);
- finds the true minimum capacity — the efficiency Leaf Mark target;
- labels every possible move as *optimal*, *still winnable* or *dead end* — used for hints;
- checks that the two amendment options produce **different optimal solutions** (no cosmetic choices);
- checks Field Report clues: combined clues identify exactly one answer, and no single player's hand does, for 2, 3 and 4 players.

## 7. Emerging strategies

Corridor-first, species-first, prevention-first (clear Lantana before it lands), recovery-first (restore cells that unlock free regrowth under Succession), intervention-first (Active Restoration on clustered damage), information-first (talk through clues before guessing). The solver confirms most levels have several optimal first moves.

## 8. Loopholes considered

See the Rule Book §11. Key safeguards: marks come from objectives not clicks; only species-suitable routes count; round limits stop infinite waiting; groves only on open ground; seat-order turns; persisted votes with absent-voter removal; best-result-only rewards.

## 9. Conclusion of the game

Each level ends with explicit success or failure and a visible landscape change. The region ends when level 10 is won: the full Forest Realm map shows the restored corridors and the two rules the team chose, with the team's Leaf Marks and Journal discoveries. Failure always offers a replay with no loss of campaign progress.

## 10. Design thinking process

1. **Empathise / research:** global and local readings on fragmentation; Western Ghats species chosen for clear, real habitat needs.
2. **Define:** "make players *want* to reconnect habitat" — not "teach facts".
3. **Ideate:** early concept with a hidden Polluter was dropped in favour of an environmental opponent.
4. **Prototype:** rules engine + solver first, so every level is provably fair before art.
5. **Test:** automated (solver, unit, integration, two-client room tests) and playtests with new players (see the plan below).
6. **Refine:** solver-driven rebalancing already changed the regrowth threshold (2 → 3 neighbours) and redesigned levels 3, 5, 6, 8, 9 and 10 when the classifier found shortcuts or rule choices that did not matter.

### Playtest plan
First-time players, no briefing: can they read the level 1 goal, explain Restoration Capacity, explain what changed after each amendment, and explain why a connection was or was not suitable? Rate Puzzle and Expedition enjoyment separately. Balance values (costs, rounds, thresholds) remain open to change after playtesting.

## 11. Known balance notes
- Natural Succession is cheaper than Active Restoration on several Expeditions; Active finishes sooner on clustered damage. Both are always winnable; efficiency targets are per-rule. Confirm in playtests whether the choice feels balanced.
