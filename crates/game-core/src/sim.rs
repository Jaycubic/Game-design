//! The level simulator: applies player actions, resolves rounds through the
//! Environmental Pressure Engine, evaluates objectives and awards Leaf Marks.

use crate::board::{Board, Terrain, Tile};
use crate::connectivity::connected;
use crate::habitat;
use crate::level::{ChallengeRule, EventDef, EventKind, LevelDef, LevelKind, Mode, Objective};
use crate::pos::Pos;
use crate::rules::{ActionKind, Ruleset};
use crate::species::Species;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Plant {
        at: Pos,
    },
    Restore {
        at: Pos,
    },
    Protect {
        at: Pos,
    },
    Bridge {
        at: Pos,
    },
    Grove {
        at: Pos,
    },
    PlaceSpecies {
        species: Species,
        at: Pos,
    },
    RemoveSpecies {
        species: Species,
    },
    /// Puzzles only: take back an intervention before checking the plan (refunds it).
    Revert {
        at: Pos,
    },
    Answer {
        candidate: String,
    },
    /// Expedition: finish the round. Puzzle: check the plan.
    EndRound,
}

impl Action {
    pub fn kind(&self) -> Option<ActionKind> {
        Some(match self {
            Action::Plant { .. } => ActionKind::Plant,
            Action::Restore { .. } => ActionKind::Restore,
            Action::Protect { .. } => ActionKind::Protect,
            Action::Bridge { .. } => ActionKind::Bridge,
            Action::Grove { .. } => ActionKind::Grove,
            _ => return None,
        })
    }

    pub fn at(&self) -> Option<Pos> {
        match self {
            Action::Plant { at }
            | Action::Restore { at }
            | Action::Protect { at }
            | Action::Bridge { at }
            | Action::Grove { at }
            | Action::PlaceSpecies { at, .. }
            | Action::Revert { at } => Some(*at),
            _ => None,
        }
    }

    fn with_kind(kind: ActionKind, at: Pos) -> Action {
        match kind {
            ActionKind::Plant => Action::Plant { at },
            ActionKind::Restore => Action::Restore { at },
            ActionKind::Protect => Action::Protect { at },
            ActionKind::Bridge => Action::Bridge { at },
            ActionKind::Grove => Action::Grove { at },
        }
    }
}

/// Why an action was rejected. Serialised with a stable `code` for clients.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum RuleError {
    LevelOver,
    OffMap,
    NotASite,
    WrongTerrain { needs: String },
    NotAllowedByCharter,
    NotEnoughCapacity { need: u8, have: u8 },
    NotThreatened,
    AlreadyProtected,
    NotASlot,
    SlotTaken,
    NoSuchToken,
    TokenNotPlaced,
    NothingToRevert,
    WrongLevelKind,
    UnknownCandidate,
}

impl RuleError {
    pub fn message(&self) -> String {
        match self {
            RuleError::LevelOver => "This level has already ended.".into(),
            RuleError::OffMap => "That spot is off the map.".into(),
            RuleError::NotASite => "No intervention is possible at that spot in this level.".into(),
            RuleError::WrongTerrain { needs } => format!("That action needs {needs}."),
            RuleError::NotAllowedByCharter => "The Living Charter does not allow that action yet.".into(),
            RuleError::NotEnoughCapacity { need, have } => {
                format!("Not enough Restoration Capacity: needs {need}, the team has {have} left.")
            }
            RuleError::NotThreatened => "Lantana is not threatening that spot this round.".into(),
            RuleError::AlreadyProtected => "That spot is already cleared of Lantana.".into(),
            RuleError::NotASlot => "Species can only be placed on the marked habitat spots.".into(),
            RuleError::SlotTaken => "Another species is already on that spot.".into(),
            RuleError::NoSuchToken => "That species is not part of this puzzle.".into(),
            RuleError::TokenNotPlaced => "That species has not been placed.".into(),
            RuleError::NothingToRevert => "There is nothing to take back at that spot.".into(),
            RuleError::WrongLevelKind => "That action is not used in this level.".into(),
            RuleError::UnknownCandidate => "That is not one of the possible answers.".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Placement {
    pub species: Species,
    pub at: Pos,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Applied {
    pub action: Action,
    pub cost: u8,
    pub before: Vec<(Pos, Tile)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Status {
    InProgress,
    Won { round: u8 },
    Lost { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub board: Board,
    pub round: u8,
    pub capacity_left: u8,
    pub spent: u16,
    pub protected: Vec<Pos>,
    pub placements: Vec<Placement>,
    /// Failed plan checks (puzzles) or wrong answers (deduction).
    pub attempts: u8,
    pub status: Status,
    /// Interventions made this round (puzzles use it for take-backs).
    pub log: Vec<Applied>,
}

impl State {
    pub fn placement(&self, s: Species) -> Option<Pos> {
        self.placements.iter().find(|p| p.species == s).map(|p| p.at)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectiveStatus {
    pub label: String,
    pub met: bool,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evaluation {
    pub objectives: Vec<ObjectiveStatus>,
    pub all_met: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundReport {
    pub round: u8,
    pub events: Vec<String>,
    pub degraded: Vec<Pos>,
    pub eroded: Vec<Pos>,
    pub regrown: Vec<Pos>,
    pub evaluation: Evaluation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum Outcome {
    Applied {
        cost: u8,
    },
    Reverted {
        refund: u8,
    },
    Placed,
    RoundResolved {
        report: RoundReport,
    },
    /// Puzzle plan check failed; the board is unchanged so players can adjust.
    CheckFailed {
        report: RoundReport,
    },
    Answered {
        correct: bool,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeafMarks {
    pub objective: bool,
    pub efficient: bool,
    pub challenge: bool,
}

impl LeafMarks {
    pub fn count(&self) -> u8 {
        self.objective as u8 + self.efficient as u8 + self.challenge as u8
    }
}

pub struct Sim<'a> {
    pub level: &'a LevelDef,
    pub rules: Ruleset,
    initial: Board,
    answer: Option<usize>,
    /// Lantana cells per round (index = round).
    threats: Vec<Vec<Pos>>,
    /// Cells touched by any pressure event; always open to intervention.
    event_cells: Vec<Pos>,
}

impl<'a> Sim<'a> {
    pub fn new(level: &'a LevelDef, rules: Ruleset) -> Result<Self, String> {
        level.validate()?;
        let initial = level.board()?;
        let answer = level.deduction.as_ref().and_then(|d| d.answer(&initial, &rules));
        let mut threats = vec![Vec::new(); level.rounds as usize + 1];
        let mut event_cells = Vec::new();
        for e in &level.events {
            match &e.kind {
                EventKind::Lantana { cells } => {
                    threats[e.round as usize].extend(cells);
                    event_cells.extend(cells);
                }
                EventKind::Erosion { cells } => event_cells.extend(cells),
                EventKind::DrySpell { .. } => {}
            }
        }
        for t in &mut threats {
            t.sort();
            t.dedup();
        }
        event_cells.sort();
        event_cells.dedup();
        Ok(Sim { level, rules, initial, answer, threats, event_cells })
    }

    /// Interventions are allowed on level sites and on any cell a pressure event touches.
    pub fn is_site(&self, p: Pos) -> bool {
        self.level.is_site(p) || self.event_cells.binary_search(&p).is_ok()
    }

    /// Total capacity still available: this round's remainder plus all later rounds.
    pub fn remaining_capacity(&self, state: &State) -> u16 {
        state.capacity_left as u16
            + ((state.round + 1)..=self.level.rounds).map(|r| self.round_capacity(r) as u16).sum::<u16>()
    }

    pub fn initial_board(&self) -> &Board {
        &self.initial
    }

    /// Index of the correct deduction candidate under the active rules.
    pub fn answer(&self) -> Option<usize> {
        self.answer
    }

    pub fn initial_state(&self) -> State {
        State {
            board: self.initial.clone(),
            round: 1,
            capacity_left: self.round_capacity(1),
            spent: 0,
            protected: Vec::new(),
            placements: Vec::new(),
            attempts: 0,
            status: Status::InProgress,
            log: Vec::new(),
        }
    }

    pub fn events_in(&self, round: u8) -> impl Iterator<Item = &EventDef> {
        self.level.events.iter().filter(move |e| e.round == round)
    }

    pub fn round_capacity(&self, round: u8) -> u8 {
        let penalty: u8 = self
            .events_in(round)
            .map(|e| match e.kind {
                EventKind::DrySpell { capacity_penalty } => capacity_penalty,
                _ => 0,
            })
            .sum();
        self.level.capacity.saturating_sub(penalty)
    }

    /// Cells that Lantana will hit at the end of this round unless cleared.
    pub fn threatened(&self, round: u8) -> &[Pos] {
        self.threats.get(round as usize).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Validate an intervention and return its capacity cost.
    pub fn check(&self, state: &State, action: &Action) -> Result<u8, RuleError> {
        if state.status != Status::InProgress {
            return Err(RuleError::LevelOver);
        }
        let Some(kind) = action.kind() else { return Ok(0) };
        if self.level.kind == LevelKind::Deduction {
            return Err(RuleError::WrongLevelKind);
        }
        let at = action.at().expect("interventions have a cell");
        if !state.board.in_bounds(at) {
            return Err(RuleError::OffMap);
        }
        let cost = self.rules.cost(kind).ok_or(RuleError::NotAllowedByCharter)?;
        let tile = state.board.tile(at);
        let needs = |t: Terrain| -> Result<(), RuleError> {
            if tile.terrain == t {
                Ok(())
            } else {
                Err(RuleError::WrongTerrain { needs: t.label().into() })
            }
        };
        if kind == ActionKind::Protect {
            if self.threatened(state.round).binary_search(&at).is_err() {
                return Err(RuleError::NotThreatened);
            }
            if state.protected.contains(&at) {
                return Err(RuleError::AlreadyProtected);
            }
            needs(Terrain::Forest)?;
        } else {
            if !self.is_site(at) {
                return Err(RuleError::NotASite);
            }
            match kind {
                ActionKind::Plant | ActionKind::Grove => needs(Terrain::Open)?,
                ActionKind::Restore => needs(Terrain::Degraded)?,
                ActionKind::Bridge => needs(Terrain::Road)?,
                ActionKind::Protect => unreachable!(),
            }
        }
        if cost > state.capacity_left {
            return Err(RuleError::NotEnoughCapacity { need: cost, have: state.capacity_left });
        }
        Ok(cost)
    }

    pub fn apply(&self, state: &mut State, action: &Action) -> Result<Outcome, RuleError> {
        if state.status != Status::InProgress {
            return Err(RuleError::LevelOver);
        }
        match action {
            Action::Plant { .. }
            | Action::Restore { .. }
            | Action::Protect { .. }
            | Action::Bridge { .. }
            | Action::Grove { .. } => {
                let cost = self.check(state, action)?;
                let at = action.at().unwrap();
                let mut before = vec![(at, state.board.tile(at))];
                match action {
                    Action::Plant { .. } => state.board.set_terrain(at, Terrain::Forest),
                    Action::Grove { .. } => state.board.set_terrain(at, Terrain::Grove),
                    Action::Bridge { .. } => state.board.set_terrain(at, Terrain::Bridge),
                    Action::Restore { .. } => {
                        state.board.set_terrain(at, Terrain::Forest);
                        if self.rules.recovery == crate::rules::Recovery::Active {
                            let around: Vec<Pos> = state.board.neighbours(at).collect();
                            for q in around {
                                let t = state.board.tile(q);
                                if t.terrain == Terrain::Degraded {
                                    before.push((q, t));
                                    state.board.set_terrain(q, Terrain::Forest);
                                }
                            }
                        }
                    }
                    Action::Protect { .. } => {
                        before.clear();
                        state.protected.push(at);
                        state.protected.sort();
                    }
                    _ => unreachable!(),
                }
                state.capacity_left -= cost;
                state.spent += cost as u16;
                state.log.push(Applied { action: action.clone(), cost, before });
                Ok(Outcome::Applied { cost })
            }
            Action::PlaceSpecies { species, at } => {
                if !self.level.tokens.contains(species) {
                    return Err(RuleError::NoSuchToken);
                }
                if !self.level.slots.contains(at) {
                    return Err(RuleError::NotASlot);
                }
                if state.placements.iter().any(|p| p.at == *at && p.species != *species) {
                    return Err(RuleError::SlotTaken);
                }
                state.placements.retain(|p| p.species != *species);
                state.placements.push(Placement { species: *species, at: *at });
                state.placements.sort_by_key(|p| p.species);
                Ok(Outcome::Placed)
            }
            Action::RemoveSpecies { species } => {
                let n = state.placements.len();
                state.placements.retain(|p| p.species != *species);
                if state.placements.len() == n {
                    return Err(RuleError::TokenNotPlaced);
                }
                Ok(Outcome::Placed)
            }
            Action::Revert { at } => {
                if self.level.mode != Mode::Puzzle {
                    return Err(RuleError::WrongLevelKind);
                }
                let idx = state
                    .log
                    .iter()
                    .rposition(|a| a.action.at() == Some(*at) || a.before.iter().any(|(p, _)| p == at))
                    .ok_or(RuleError::NothingToRevert)?;
                let entry = state.log.remove(idx);
                for (p, t) in entry.before {
                    state.board.set(p, t);
                }
                if let Action::Protect { at } = entry.action {
                    state.protected.retain(|p| *p != at);
                }
                state.capacity_left += entry.cost;
                state.spent -= entry.cost as u16;
                Ok(Outcome::Reverted { refund: entry.cost })
            }
            Action::Answer { candidate } => {
                let d = self.level.deduction.as_ref().ok_or(RuleError::WrongLevelKind)?;
                let idx = d.candidates.iter().position(|c| &c.id == candidate).ok_or(RuleError::UnknownCandidate)?;
                let correct = Some(idx) == self.answer;
                if correct {
                    state.status = Status::Won { round: state.round };
                } else {
                    state.attempts += 1;
                    if state.attempts >= self.level.max_attempts {
                        state.status = Status::Lost { reason: "The team ran out of answers to try.".into() };
                    }
                }
                Ok(Outcome::Answered { correct })
            }
            Action::EndRound => {
                if self.level.kind == LevelKind::Deduction {
                    return Err(RuleError::WrongLevelKind);
                }
                let (mut next, report) = self.resolve(state);
                if self.level.mode == Mode::Puzzle {
                    if report.evaluation.all_met {
                        next.status = Status::Won { round: state.round };
                        *state = next;
                        return Ok(Outcome::RoundResolved { report });
                    }
                    state.attempts += 1;
                    if state.attempts >= self.level.max_attempts {
                        state.status = Status::Lost { reason: "The plan was checked too many times.".into() };
                    }
                    return Ok(Outcome::CheckFailed { report });
                }
                let degraded = next.board.count_terrain(Terrain::Degraded) as u16;
                next.status = if report.evaluation.all_met {
                    Status::Won { round: next.round }
                } else if self.level.critical_degraded.is_some_and(|c| degraded >= c) {
                    Status::Lost {
                        reason: format!("{degraded} cells degraded — the forest reached a critical state.")
                    }
                } else if next.round >= self.level.rounds {
                    Status::Lost { reason: "The mission ran out of rounds before the objective was met.".into() }
                } else {
                    Status::InProgress
                };
                if next.status == Status::InProgress {
                    next.round += 1;
                    next.capacity_left = self.round_capacity(next.round);
                }
                *state = next;
                Ok(Outcome::RoundResolved { report })
            }
        }
    }

    /// End-of-round resolution: pressure events, erosion, then natural regrowth.
    pub fn resolve(&self, state: &State) -> (State, RoundReport) {
        let mut s = state.clone();
        let round = s.round;
        let (mut degraded, mut eroded, mut regrown, mut events) = (vec![], vec![], vec![], vec![]);
        let mut dry = false;
        for e in self.events_in(round) {
            events.push(e.title.clone());
            match &e.kind {
                EventKind::Lantana { cells } => {
                    for &c in cells {
                        if !s.protected.contains(&c) && s.board.tile(c).is_forest() {
                            s.board.set_terrain(c, Terrain::Degraded);
                            degraded.push(c);
                        }
                    }
                }
                EventKind::DrySpell { .. } => dry = true,
                EventKind::Erosion { .. } => {}
            }
        }
        for e in self.events_in(round) {
            if let EventKind::Erosion { cells } = &e.kind {
                for &c in cells {
                    let t = s.board.tile(c);
                    if t.terrain == Terrain::Degraded {
                        s.board.set(c, Tile::new(Terrain::Open));
                        eroded.push(c);
                    }
                }
            }
        }
        if self.rules.natural_regrowth() && !dry {
            let snapshot = s.board.clone();
            for p in snapshot.positions() {
                if snapshot.tile(p).terrain == Terrain::Degraded
                    && !degraded.contains(&p)
                    && snapshot.forest_neighbours(p) >= crate::rules::SUCCESSION_NEIGHBOURS
                {
                    s.board.set_terrain(p, Terrain::Forest);
                    regrown.push(p);
                }
            }
        }
        s.protected.clear();
        s.log.clear();
        let evaluation = self.evaluate(&s.board, &s.placements);
        (s, RoundReport { round, events, degraded, eroded, regrown, evaluation })
    }

    pub fn evaluate(&self, board: &Board, placements: &[Placement]) -> Evaluation {
        let objectives: Vec<ObjectiveStatus> = self
            .level
            .objectives
            .iter()
            .map(|o| {
                let (met, detail) = match o {
                    Objective::Connect { species, from, to, .. } => {
                        let ok = connected(board, &self.rules, *species, *from, *to);
                        (ok, (!ok).then(|| format!("No route yet for the {}.", species.common_name().to_lowercase())))
                    }
                    Objective::Habitat { species, at, .. } => {
                        let target = at.or_else(|| placements.iter().find(|p| p.species == *species).map(|p| p.at));
                        match target {
                            None => (false, Some(format!("Place the {} on a habitat spot.", species.common_name()))),
                            Some(p) => match habitat::check(board, &self.rules, *species, p) {
                                Ok(()) => (true, None),
                                Err(issue) => (false, Some(issue.explain(*species))),
                            },
                        }
                    }
                    Objective::ForestCover { min, .. } => {
                        let have = board.count_terrain(Terrain::Forest) as u16;
                        (have >= *min, Some(format!("{have} of {min} healthy forest cells")))
                    }
                    Objective::Restore { cells, .. } => {
                        let done = cells.iter().filter(|c| board.tile(**c).is_forest()).count();
                        (done == cells.len(), Some(format!("{done} of {} restored", cells.len())))
                    }
                };
                ObjectiveStatus { label: o.label().to_string(), met, detail }
            })
            .collect();
        let all_met = !objectives.is_empty() && objectives.iter().all(|o| o.met);
        Evaluation { objectives, all_met }
    }

    /// Every intervention that could ever be legal in this level, in a fixed order.
    /// The solver and the hint system enumerate exactly this list.
    pub fn options(&self) -> Vec<Action> {
        let mut out = Vec::new();
        if self.level.kind == LevelKind::Deduction {
            if let Some(d) = &self.level.deduction {
                out.extend(d.candidates.iter().map(|c| Action::Answer { candidate: c.id.clone() }));
            }
            return out;
        }
        let mut lantana: Vec<Pos> = Vec::new();
        let mut erosion: Vec<Pos> = Vec::new();
        for e in &self.level.events {
            match &e.kind {
                EventKind::Lantana { cells } => lantana.extend(cells),
                EventKind::Erosion { cells } => erosion.extend(cells),
                EventKind::DrySpell { .. } => {}
            }
        }
        for p in self.initial.positions() {
            if !self.is_site(p) {
                continue;
            }
            let t = self.initial.tile(p).terrain;
            let may_open = t == Terrain::Open || erosion.contains(&p);
            let may_degrade = t == Terrain::Degraded || lantana.contains(&p);
            let mut kinds = Vec::new();
            if may_open {
                kinds.push(ActionKind::Plant);
                kinds.push(ActionKind::Grove);
            }
            if may_degrade {
                kinds.push(ActionKind::Restore);
            }
            if t == Terrain::Road {
                kinds.push(ActionKind::Bridge);
            }
            for k in kinds {
                if self.rules.cost(k).is_some() {
                    out.push(Action::with_kind(k, p));
                }
            }
        }
        lantana.sort();
        lantana.dedup();
        out.extend(lantana.into_iter().map(|at| Action::Protect { at }));
        for &species in &self.level.tokens {
            for &at in &self.level.slots {
                out.push(Action::PlaceSpecies { species, at });
            }
        }
        out
    }

    /// Puzzle helper: an assignment of species tokens to slots that meets every
    /// objective on `board`, if one exists (tries each slot permutation).
    pub fn assign_tokens(&self, board: &Board) -> Option<Vec<Placement>> {
        fn rec(
            sim: &Sim,
            board: &Board,
            i: usize,
            used: &mut Vec<bool>,
            cur: &mut Vec<Placement>,
        ) -> Option<Vec<Placement>> {
            let tokens = &sim.level.tokens;
            if i == tokens.len() {
                let mut placed = cur.clone();
                placed.sort_by_key(|p| p.species);
                return sim.evaluate(board, &placed).all_met.then_some(placed);
            }
            for (j, &slot) in sim.level.slots.iter().enumerate() {
                if used[j] || habitat::check(board, &sim.rules, tokens[i], slot).is_err() {
                    continue;
                }
                used[j] = true;
                cur.push(Placement { species: tokens[i], at: slot });
                if let Some(found) = rec(sim, board, i + 1, used, cur) {
                    return Some(found);
                }
                cur.pop();
                used[j] = false;
            }
            None
        }
        rec(self, board, 0, &mut vec![false; self.level.slots.len()], &mut Vec::new())
    }

    pub fn leaf_marks(&self, state: &State) -> LeafMarks {
        let Status::Won { round } = state.status else { return LeafMarks::default() };
        let efficient = if self.level.kind == LevelKind::Deduction {
            state.attempts <= 1
        } else {
            self.level.par_for(&self.rules).is_some_and(|par| state.spent <= par + self.level.par_slack)
        };
        let challenge = match &self.level.challenge.rule {
            ChallengeRule::ForestAt { cells } => cells.iter().all(|c| state.board.tile(*c).is_forest()),
            ChallengeRule::FinishByRound { round: r } => round <= *r,
            ChallengeRule::FirstTry => state.attempts == 0,
            ChallengeRule::MaxCost { cost } => state.spent <= *cost,
            ChallengeRule::Habitat { species, at } => habitat::check(&state.board, &self.rules, *species, *at).is_ok(),
        };
        LeafMarks { objective: true, efficient, challenge }
    }
}
