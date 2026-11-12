//! The level classifier: an exhaustive, cost-ordered (A*) search over every
//! reachable game state. It proves each level solvable under every Living
//! Charter combination, finds the minimum Restoration Capacity (par), labels
//! each possible move as optimal / still winnable / dead end, and powers hints.
//!
//! Exactness: the search is A* with an admissible lower bound (`lower_bound`),
//! so the first winning state popped has minimum cost. Branches whose lower
//! bound exceeds all capacity still available are pruned — they cannot win.

use crate::board::Terrain;
use crate::level::{LevelDef, LevelKind, Mode, Objective};
use crate::pos::Pos;
use crate::rules::{Connectivity, Recovery, Ruleset};
use crate::sim::{Action, Outcome, RuleError, Sim, State, Status};
use crate::species::Species;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BinaryHeap, HashMap};
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_states: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_states: 3_000_000 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Solution {
    pub cost: u16,
    pub rounds: u8,
    pub plan: Vec<Action>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum SolveResult {
    Solved {
        solution: Solution,
        explored: usize,
    },
    Unsolvable {
        explored: usize,
    },
    /// The search budget ran out before a proof either way.
    Unknown {
        explored: usize,
    },
}

impl SolveResult {
    pub fn solution(&self) -> Option<&Solution> {
        match self {
            SolveResult::Solved { solution, .. } => Some(solution),
            _ => None,
        }
    }
}

const END_ROUND: u16 = u16::MAX;

fn key(s: &State) -> u64 {
    let mut h = DefaultHasher::new();
    s.board.tiles.hash(&mut h);
    s.round.hash(&mut h);
    s.capacity_left.hash(&mut h);
    s.protected.hash(&mut h);
    s.placements.hash(&mut h);
    std::mem::discriminant(&s.status).hash(&mut h);
    h.finish()
}

// ── Admissible lower bound ──────────────────────────────────────────────────
// Costs are in half-units so fractional per-cell bounds stay exact integers.

const INF: u32 = u32::MAX / 4;

/// Can this degraded cell ever regrow for free? It needs enough neighbours that
/// are, or could become, healthy forest.
fn could_regrow(board: &crate::board::Board, p: Pos) -> bool {
    board
        .neighbours(p)
        .filter(|q| matches!(board.tile(*q).terrain, Terrain::Forest | Terrain::Degraded | Terrain::Open))
        .count()
        >= crate::rules::SUCCESSION_NEIGHBOURS
}

/// Cheapest conceivable half-unit cost for a route to pass through a cell.
fn cell_half_cost(rules: &Ruleset, species: Species, t: Terrain, regrowable: bool) -> u32 {
    if species.route_suitable(t) {
        return 0;
    }
    let rules = &if t == Terrain::Degraded && rules.recovery == Recovery::Succession && !regrowable {
        Ruleset::new(rules.connectivity, Recovery::Base)
    } else {
        *rules
    };
    let ss = rules.connectivity == Connectivity::SteppingStone && species.route_suitable(Terrain::Grove);
    let forest_species = species.route_suitable(Terrain::Forest);
    match t {
        // A grove covers two cells for 2 capacity, and chains of groves approach 0.5 per cell.
        Terrain::Open if forest_species => {
            if ss {
                1
            } else {
                2
            }
        }
        Terrain::Degraded if forest_species => match rules.recovery {
            Recovery::Succession => 0,
            // One active restoration (2) can heal up to three cells along a path.
            Recovery::Active => 1,
            Recovery::Base => {
                if ss {
                    1
                } else {
                    2
                }
            }
        },
        Terrain::Road if species.route_suitable(Terrain::Bridge) && rules.connectivity == Connectivity::Continuous => 4,
        Terrain::Road | Terrain::Grassland | Terrain::Open | Terrain::Degraded if ss => 1,
        _ => INF,
    }
}

fn route_bound(sim: &Sim, s: &State, species: Species, from: Pos, to: Pos) -> u32 {
    let b = &s.board;
    let n = b.tiles.len();
    let mut dist = vec![INF; n];
    let cost = |p: Pos| {
        let t = b.tile(p).terrain;
        cell_half_cost(&sim.rules, species, t, t == Terrain::Degraded && could_regrow(b, p))
    };
    let start = cost(from);
    if start >= INF {
        return INF;
    }
    dist[b.index(from)] = start;
    let mut heap = BinaryHeap::from([Reverse((start, b.index(from)))]);
    while let Some(Reverse((d, i))) = heap.pop() {
        if d > dist[i] {
            continue;
        }
        let p = b.pos_of(i);
        if p == to {
            return d;
        }
        for q in b.neighbours(p) {
            let c = cost(q);
            if c >= INF {
                continue;
            }
            let j = b.index(q);
            if d + c < dist[j] {
                dist[j] = d + c;
                heap.push(Reverse((d + c, j)));
            }
        }
    }
    INF
}

fn heal_bound_half(rules: &Ruleset, t: Terrain) -> u32 {
    match t {
        Terrain::Forest => 0,
        Terrain::Open => 2,
        Terrain::Degraded => match rules.recovery {
            Recovery::Succession => 0,
            Recovery::Active => 1,
            Recovery::Base => 2,
        },
        _ => INF,
    }
}

/// A lower bound on the capacity still needed to win from `s`.
pub fn lower_bound(sim: &Sim, s: &State) -> u16 {
    let mut best = 0u32;
    for o in &sim.level.objectives {
        let half = match o {
            Objective::Connect { species, from, to, .. } => route_bound(sim, s, *species, *from, *to),
            Objective::Habitat { species, at: Some(p), .. } if !species.route_suitable(Terrain::Grassland) => {
                let h = heal_bound_half(&sim.rules, s.board.tile(*p).terrain);
                // Active restoration is at least one 2-capacity action whenever anything must heal.
                if h > 0 && sim.rules.recovery == Recovery::Active && s.board.tile(*p).terrain == Terrain::Degraded {
                    4
                } else {
                    h
                }
            }
            Objective::Habitat { .. } => 0,
            Objective::Restore { cells, .. } => {
                let sum: u32 = cells.iter().map(|c| heal_bound_half(&sim.rules, s.board.tile(*c).terrain)).sum();
                if sim.rules.recovery == Recovery::Active {
                    // each 2-capacity restoration heals at most five cells
                    let degraded =
                        cells.iter().filter(|c| s.board.tile(**c).terrain == Terrain::Degraded).count() as u32;
                    let open = cells.iter().filter(|c| s.board.tile(**c).terrain == Terrain::Open).count() as u32;
                    open * 2 + (degraded * 4).div_ceil(5)
                } else {
                    sum
                }
            }
            Objective::ForestCover { min, .. } => {
                let have = s.board.count_terrain(Terrain::Forest) as u32;
                let deficit = (*min as u32).saturating_sub(have);
                match sim.rules.recovery {
                    // Only regrowable degraded cells can become forest for free.
                    Recovery::Succession => {
                        let free = s
                            .board
                            .positions()
                            .filter(|p| s.board.tile(*p).terrain == Terrain::Degraded && could_regrow(&s.board, *p))
                            .count() as u32;
                        deficit.saturating_sub(free) * 2
                    }
                    Recovery::Active => (deficit * 4).div_ceil(5), // 0.4 capacity per forest cell, in half-units
                    Recovery::Base => deficit * 2,
                }
            }
        };
        best = best.max(half);
    }
    if best >= INF {
        return u16::MAX;
    }
    best.div_ceil(2) as u16
}

// ── Search ─────────────────────────────────────────────────────────────────

fn puzzle_check(sim: &Sim, s: &State) -> Option<State> {
    let (mut next, report) = sim.resolve(s);
    if sim.level.tokens.is_empty() {
        if !report.evaluation.all_met {
            return None;
        }
    } else {
        next.placements = sim.assign_tokens(&next.board)?;
    }
    next.status = Status::Won { round: s.round };
    Some(next)
}

/// Expand one state: every legal successor with its added cost and the option index used.
fn successors(sim: &Sim, options: &[Action], s: &State, out: &mut Vec<(State, u8, u16)>) {
    out.clear();
    for (i, a) in options.iter().enumerate() {
        if matches!(a, Action::PlaceSpecies { .. }) || sim.check(s, a).is_err() {
            continue;
        }
        let mut n = s.clone();
        if let Ok(Outcome::Applied { cost }) = sim.apply(&mut n, a) {
            n.log.clear(); // take-back history is irrelevant to the search
            out.push((n, cost, i as u16));
        }
    }
    if sim.level.mode == Mode::Puzzle {
        if let Some(n) = puzzle_check(sim, s) {
            out.push((n, 0, END_ROUND));
        }
        return;
    }
    let mut n = s.clone();
    if sim.apply(&mut n, &Action::EndRound).is_ok() && !matches!(n.status, Status::Lost { .. }) {
        out.push((n, 0, END_ROUND));
    }
}

/// Minimum-cost solution from `start` (ties broken by fewer rounds).
pub fn solve(sim: &Sim, start: &State, limits: &Limits) -> SolveResult {
    if sim.level.kind == LevelKind::Deduction {
        let d = sim.level.deduction.as_ref().expect("validated");
        return match sim.answer() {
            Some(i) => SolveResult::Solved {
                solution: Solution {
                    cost: 0,
                    rounds: 1,
                    plan: vec![Action::Answer { candidate: d.candidates[i].id.clone() }],
                },
                explored: d.candidates.len(),
            },
            None => SolveResult::Unsolvable { explored: d.candidates.len() },
        };
    }
    match start.status {
        Status::Won { round } => {
            return SolveResult::Solved { solution: Solution { cost: 0, rounds: round, plan: vec![] }, explored: 0 }
        }
        Status::Lost { .. } => return SolveResult::Unsolvable { explored: 0 },
        Status::InProgress => {}
    }

    let options = sim.options();
    let h0 = lower_bound(sim, start);
    if h0 as u32 > sim.remaining_capacity(start) as u32 {
        return SolveResult::Unsolvable { explored: 0 };
    }
    let mut slab: Vec<Option<State>> = vec![Some(start.clone())];
    let mut heap = BinaryHeap::new();
    let start_key = key(start);
    let mut best: HashMap<u64, u16> = HashMap::from([(start_key, 0)]);
    let mut parent: HashMap<u64, (u64, u16)> = HashMap::new();
    heap.push(Reverse((h0, 0u16, start.round, 0usize)));
    let mut explored = 0usize;
    let mut buf = Vec::new();
    // The bound depends only on the board, which many states share across rounds.
    let mut hcache: HashMap<u64, u16> = HashMap::new();
    let mut bound = |n: &State| -> u16 {
        let mut hh = DefaultHasher::new();
        n.board.tiles.hash(&mut hh);
        *hcache.entry(hh.finish()).or_insert_with(|| lower_bound(sim, n))
    };

    while let Some(Reverse((_, cost, _, slot))) = heap.pop() {
        let Some(s) = slab[slot].take() else { continue };
        let k = key(&s);
        if best.get(&k).is_some_and(|b| *b < cost) {
            continue;
        }
        if let Status::Won { round } = s.status {
            let mut plan = Vec::new();
            let mut cur = k;
            while let Some(&(prev, idx)) = parent.get(&cur) {
                plan.push(if idx == END_ROUND { Action::EndRound } else { options[idx as usize].clone() });
                cur = prev;
            }
            plan.reverse();
            if !s.placements.is_empty() {
                // Puzzle tokens were matched at the check; list them before the final check.
                let end = plan.pop();
                plan.extend(s.placements.iter().map(|p| Action::PlaceSpecies { species: p.species, at: p.at }));
                plan.extend(end);
            }
            return SolveResult::Solved { solution: Solution { cost, rounds: round, plan }, explored };
        }
        explored += 1;
        if explored > limits.max_states {
            return SolveResult::Unknown { explored };
        }
        successors(sim, &options, &s, &mut buf);
        for (n, c, idx) in buf.drain(..) {
            let nc = cost + c as u16;
            let h = if matches!(n.status, Status::Won { .. }) { 0 } else { bound(&n) };
            if h != 0 && h as u32 > sim.remaining_capacity(&n) as u32 {
                continue; // cannot be won with the capacity that is left
            }
            let nk = key(&n);
            if best.get(&nk).is_some_and(|b| *b <= nc) {
                continue;
            }
            best.insert(nk, nc);
            parent.insert(nk, (k, idx));
            let round = n.round;
            slab.push(Some(n));
            heap.push(Reverse((nc.saturating_add(h), nc, round, slab.len() - 1)));
        }
    }
    SolveResult::Unsolvable { explored }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum Verdict {
    /// Part of a minimum-cost solution.
    Optimal,
    /// Still winnable, but at a higher total cost.
    Winnable {
        total_cost: u16,
    },
    /// After this move the level can no longer be won.
    DeadEnd,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveVerdict {
    pub action: Action,
    pub cost: u8,
    #[serde(flatten)]
    pub verdict: Verdict,
}

/// Classify every legal intervention (and ending the round) from `state`.
pub fn classify_moves(sim: &Sim, state: &State, limits: &Limits) -> Vec<MoveVerdict> {
    let best = solve(sim, state, limits).solution().map(|s| s.cost);
    let options = sim.options();
    let mut buf = Vec::new();
    successors(sim, &options, state, &mut buf);
    buf.into_iter()
        .map(|(n, c, idx)| {
            let action = if idx == END_ROUND { Action::EndRound } else { options[idx as usize].clone() };
            let verdict = match solve(sim, &n, limits) {
                SolveResult::Solved { solution, .. } => {
                    let total = c as u16 + solution.cost;
                    if Some(total) == best {
                        Verdict::Optimal
                    } else {
                        Verdict::Winnable { total_cost: total }
                    }
                }
                SolveResult::Unsolvable { .. } => Verdict::DeadEnd,
                SolveResult::Unknown { .. } => Verdict::Unknown,
            };
            MoveVerdict { action, cost: c, verdict }
        })
        .collect()
}

/// Next step of a minimum-cost plan from here, for the in-game hint.
pub fn hint(sim: &Sim, state: &State, limits: &Limits) -> Option<Action> {
    solve(sim, state, limits).solution().and_then(|s| s.plan.first().cloned())
}

/// Replay a plan from the start. Returns the capacity spent if it wins.
pub fn replay(sim: &Sim, plan: &[Action]) -> Result<u16, String> {
    let mut s = sim.initial_state();
    for a in plan {
        sim.apply(&mut s, a).map_err(|e: RuleError| e.message())?;
    }
    match s.status {
        Status::Won { .. } => Ok(s.spent),
        other => Err(format!("plan ends in {other:?}")),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RulesetReport {
    pub rules: String,
    pub solvable: bool,
    pub min_cost: Option<u16>,
    pub rounds: Option<u8>,
    pub plan: Vec<Action>,
    pub explored: usize,
    pub legal_first_moves: usize,
    pub optimal_first_moves: usize,
    pub dead_end_first_moves: usize,
    /// Deduction: the correct candidate id under these rules.
    pub answer: Option<String>,
    pub notes: Vec<String>,
}

/// Full classification of one level under one ruleset.
pub fn analyze(level: &LevelDef, rules: Ruleset, limits: &Limits, with_moves: bool) -> Result<RulesetReport, String> {
    let sim = Sim::new(level, rules)?;
    let start = sim.initial_state();
    let mut notes = Vec::new();
    let mut answer = None;
    if let Some(d) = &level.deduction {
        for players in 2..=4 {
            if let Err(e) = d.fair_for(sim.initial_board(), &rules, players) {
                notes.push(format!("UNFAIR ({players} players): {e}"));
            }
        }
        answer = sim.answer().map(|i| d.candidates[i].id.clone());
    }
    let result = solve(&sim, &start, limits);
    let (solvable, min_cost, rounds, plan, explored) = match &result {
        SolveResult::Solved { solution, explored } => {
            (true, Some(solution.cost), Some(solution.rounds), solution.plan.clone(), *explored)
        }
        SolveResult::Unsolvable { explored } => (false, None, None, vec![], *explored),
        SolveResult::Unknown { explored } => {
            notes.push("search budget exhausted".into());
            (false, None, None, vec![], *explored)
        }
    };
    if solvable {
        if let Err(e) = replay(&sim, &plan) {
            notes.push(format!("solver plan does not replay: {e}"));
        }
    }
    let (mut legal, mut optimal, mut dead) = (0, 0, 0);
    if with_moves && level.kind != LevelKind::Deduction && solvable {
        for m in classify_moves(&sim, &start, limits) {
            if m.action == Action::EndRound {
                continue;
            }
            legal += 1;
            match m.verdict {
                Verdict::Optimal => optimal += 1,
                Verdict::DeadEnd => dead += 1,
                _ => {}
            }
        }
    }
    Ok(RulesetReport {
        rules: rules.key(),
        solvable,
        min_cost,
        rounds,
        plan,
        explored,
        legal_first_moves: legal,
        optimal_first_moves: optimal,
        dead_end_first_moves: dead,
        answer,
        notes,
    })
}
