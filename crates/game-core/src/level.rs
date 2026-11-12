//! Level definitions (loaded from `content/levels/*.json`).

use crate::board::{Board, Terrain};
use crate::deduction::DeductionDef;
use crate::pos::Pos;
use crate::rules::Ruleset;
use crate::species::Species;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Focused top-down diorama board ("2D" puzzle presentation).
    Puzzle,
    /// Free-camera 3D restoration mission.
    Expedition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LevelKind {
    Connection,
    NicheMatch,
    Deduction,
    EcosystemLogic,
    Blueprint,
    Expedition,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    /// Invasive Lantana spreads: listed forest cells degrade at round end unless cleared.
    Lantana { cells: Vec<Pos> },
    /// A dry spell: less capacity this round and no natural regrowth.
    DrySpell { capacity_penalty: u8 },
    /// Monsoon run-off: listed cells that are still degraded at round end wash out to open ground.
    Erosion { cells: Vec<Pos> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDef {
    /// Round (1-based) in which the event is revealed and resolves.
    pub round: u8,
    pub title: String,
    pub text: String,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Objective {
    /// A habitat route for `species` between two anchor cells.
    Connect { species: Species, from: Pos, to: Pos, label: String },
    /// The species' needs are met — at a fixed cell, or wherever its token was placed.
    Habitat {
        species: Species,
        #[serde(default)]
        at: Option<Pos>,
        label: String,
    },
    /// At least `min` healthy forest cells on the board.
    ForestCover { min: u16, label: String },
    /// Every listed cell is healthy forest.
    Restore { cells: Vec<Pos>, label: String },
}

impl Objective {
    pub fn label(&self) -> &str {
        match self {
            Objective::Connect { label, .. }
            | Objective::Habitat { label, .. }
            | Objective::ForestCover { label, .. }
            | Objective::Restore { label, .. } => label,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChallengeRule {
    /// Every listed cell is forest at the end.
    ForestAt { cells: Vec<Pos> },
    /// Win no later than this round.
    FinishByRound { round: u8 },
    /// No wrong answer submitted (deduction) / no failed check (puzzles).
    FirstTry,
    /// Spend at most this much Restoration Capacity in total.
    MaxCost { cost: u16 },
    /// A species' needs are also met at a cell.
    Habitat { species: Species, at: Pos },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Challenge {
    pub text: String,
    #[serde(flatten)]
    pub rule: ChallengeRule,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reward {
    /// Field Journal entry keys unlocked on first completion.
    #[serde(default)]
    pub journal: Vec<String>,
    /// Insight carried into a later level (the knowledge loop between modes).
    #[serde(default)]
    pub insight: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelDef {
    pub id: String,
    pub number: u8,
    pub mode: Mode,
    pub kind: LevelKind,
    pub title: String,
    /// Regional area this level takes place in (e.g. "central_clearing").
    pub area: String,
    /// One-line plain-language instruction shown on the HUD.
    pub instruction: String,
    /// Short scene-setting text shown before play.
    pub intro: String,
    pub map: Vec<String>,
    /// Cells where interventions are possible. `None` = any cell.
    #[serde(default)]
    pub sites: Option<Vec<Pos>>,
    /// Niche puzzles: cells where species tokens may be placed.
    #[serde(default)]
    pub slots: Vec<Pos>,
    /// Niche puzzles: the species tokens to place.
    #[serde(default)]
    pub tokens: Vec<Species>,
    pub rounds: u8,
    /// Restoration Capacity per round (puzzles: the move budget).
    pub capacity: u8,
    #[serde(default)]
    pub events: Vec<EventDef>,
    pub objectives: Vec<Objective>,
    /// Lose immediately when this many cells are degraded at a round end.
    #[serde(default)]
    pub critical_degraded: Option<u16>,
    #[serde(default = "default_attempts")]
    pub max_attempts: u8,
    /// Extra capacity allowed above the solver's optimum for the efficiency Leaf Mark.
    #[serde(default)]
    pub par_slack: u16,
    pub challenge: Challenge,
    #[serde(default)]
    pub deduction: Option<DeductionDef>,
    /// Solver-verified minimum cost per ruleset key (checked by tests).
    #[serde(default)]
    pub par: BTreeMap<String, u16>,
    #[serde(default)]
    pub reward: Reward,
    /// Insight from an earlier level that this level highlights, with the cells it marks.
    #[serde(default)]
    pub uses_insight: Option<InsightUse>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InsightUse {
    pub insight: String,
    pub cells: Vec<Pos>,
    pub text: String,
}

fn default_attempts() -> u8 {
    3
}

impl LevelDef {
    pub fn board(&self) -> Result<Board, String> {
        Board::parse(&self.map)
    }

    pub fn par_for(&self, rules: &Ruleset) -> Option<u16> {
        self.par.get(&rules.key()).copied()
    }

    pub fn is_site(&self, p: Pos) -> bool {
        self.sites.as_ref().is_none_or(|s| s.contains(&p))
    }

    /// Structural validation: bounds, consistency and counts. Solvability is the solver's job.
    pub fn validate(&self) -> Result<(), String> {
        let board = self.board()?;
        let inb = |p: &Pos, what: &str| -> Result<(), String> {
            if board.in_bounds(*p) {
                Ok(())
            } else {
                Err(format!("{}: {what} {p} is off the map", self.id))
            }
        };
        if self.rounds == 0 {
            return Err(format!("{}: rounds must be at least 1", self.id));
        }
        if self.mode == Mode::Puzzle && self.rounds != 1 {
            return Err(format!("{}: puzzles resolve in exactly one round", self.id));
        }
        if (self.mode == Mode::Expedition) != (self.kind == LevelKind::Expedition) {
            return Err(format!("{}: mode and kind disagree", self.id));
        }
        if self.kind == LevelKind::Deduction {
            let d = self.deduction.as_ref().ok_or_else(|| format!("{}: deduction data missing", self.id))?;
            for c in &d.candidates {
                inb(&c.at, "candidate")?;
            }
            if d.clues.len() < 4 {
                return Err(format!("{}: deduction needs at least 4 clues for 2–4 players", self.id));
            }
        } else if self.objectives.is_empty() {
            return Err(format!("{}: no objectives", self.id));
        }
        for s in self.sites.iter().flatten() {
            inb(s, "site")?;
        }
        for s in &self.slots {
            inb(s, "slot")?;
        }
        if !self.tokens.is_empty() && self.slots.len() < self.tokens.len() {
            return Err(format!("{}: fewer slots than tokens", self.id));
        }
        for o in &self.objectives {
            match o {
                Objective::Connect { species, from, to, .. } => {
                    inb(from, "route start")?;
                    inb(to, "route end")?;
                    if species.flies() {
                        return Err(format!("{}: flying species do not need routes", self.id));
                    }
                }
                Objective::Habitat { species, at, .. } => match at {
                    Some(p) => inb(p, "habitat cell")?,
                    None => {
                        if !self.tokens.contains(species) {
                            return Err(format!("{}: habitat objective for {species:?} has no token", self.id));
                        }
                    }
                },
                Objective::Restore { cells, .. } => {
                    for c in cells {
                        inb(c, "restore cell")?;
                    }
                }
                Objective::ForestCover { .. } => {}
            }
        }
        for e in &self.events {
            if e.round == 0 || e.round > self.rounds {
                return Err(format!("{}: event '{}' in round {} of {}", self.id, e.title, e.round, self.rounds));
            }
            match &e.kind {
                EventKind::Lantana { cells } | EventKind::Erosion { cells } => {
                    for c in cells {
                        inb(c, "event cell")?;
                    }
                }
                EventKind::DrySpell { capacity_penalty } => {
                    if *capacity_penalty >= self.capacity {
                        return Err(format!("{}: dry spell removes all capacity", self.id));
                    }
                }
            }
        }
        if let ChallengeRule::ForestAt { cells } = &self.challenge.rule {
            for c in cells {
                inb(c, "challenge cell")?;
            }
        }
        if let Some(u) = &self.uses_insight {
            for c in &u.cells {
                inb(c, "insight cell")?;
            }
        }
        // Rivers and roads are the only terrain that may never become habitat; keep at least one forest cell.
        if board.count_terrain(Terrain::Forest) == 0 && board.count_terrain(Terrain::Grassland) == 0 {
            return Err(format!("{}: no habitat on the map", self.id));
        }
        Ok(())
    }
}
