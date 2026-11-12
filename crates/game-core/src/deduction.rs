//! Field Report Deduction: each player holds different clue fragments; only the
//! combined clues identify the answer. Clues may depend on the active Living
//! Charter rules, so the amendments change the correct answer.

use crate::board::{Board, Terrain};
use crate::connectivity::connected;
use crate::pos::Pos;
use crate::rules::Ruleset;
use crate::species::Species;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    North,
    South,
    East,
    West,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "test", rename_all = "snake_case")]
pub enum Predicate {
    Terrain {
        terrain: Terrain,
    },
    NotTerrain {
        terrain: Terrain,
    },
    AdjacentTo {
        terrain: Terrain,
    },
    NotAdjacentTo {
        terrain: Terrain,
    },
    HasFig,
    NoFig,
    /// Cell is in this half of the map (centre line excluded from both halves).
    Half {
        side: Side,
    },
    /// The species can already travel from `from` to the cell under the active rules.
    ReachableFrom {
        species: Species,
        from: Pos,
    },
    NotReachableFrom {
        species: Species,
        from: Pos,
    },
    /// A cell directly beside this one is reachable for the species from `from`.
    NextToReachable {
        species: Species,
        from: Pos,
    },
    NotNextToReachable {
        species: Species,
        from: Pos,
    },
    /// Under the active recovery rule, the cell would regrow on its own this round.
    WouldRegrow,
    WouldNotRegrow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clue {
    pub text: String,
    #[serde(flatten)]
    pub test: Predicate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub label: String,
    pub at: Pos,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeductionDef {
    pub question: String,
    pub candidates: Vec<Candidate>,
    pub clues: Vec<Clue>,
}

impl Predicate {
    pub fn holds(&self, board: &Board, rules: &Ruleset, p: Pos) -> bool {
        let t = board.tile(p);
        match self {
            Predicate::Terrain { terrain } => t.terrain == *terrain,
            Predicate::NotTerrain { terrain } => t.terrain != *terrain,
            Predicate::AdjacentTo { terrain } => board.adjacent_to(p, *terrain),
            Predicate::NotAdjacentTo { terrain } => !board.adjacent_to(p, *terrain),
            Predicate::HasFig => t.fig,
            Predicate::NoFig => !t.fig,
            Predicate::Half { side } => {
                let (w, h) = (board.width, board.height);
                match side {
                    Side::West => 2 * p.x + 1 < w,
                    Side::East => 2 * p.x + 1 > w,
                    Side::North => 2 * p.y + 1 < h,
                    Side::South => 2 * p.y + 1 > h,
                }
            }
            Predicate::ReachableFrom { species, from } => connected(board, rules, *species, *from, p),
            Predicate::NotReachableFrom { species, from } => !connected(board, rules, *species, *from, p),
            Predicate::NextToReachable { species, from } => next_to_reachable(board, rules, *species, *from, p),
            Predicate::NotNextToReachable { species, from } => !next_to_reachable(board, rules, *species, *from, p),
            Predicate::WouldRegrow => would_regrow(board, rules, p),
            Predicate::WouldNotRegrow => !would_regrow(board, rules, p),
        }
    }
}

fn next_to_reachable(board: &Board, rules: &Ruleset, species: Species, from: Pos, p: Pos) -> bool {
    let reached = crate::connectivity::reach(board, rules, species, from);
    board.neighbours(p).any(|q| reached[board.index(q)])
}

pub fn would_regrow(board: &Board, rules: &Ruleset, p: Pos) -> bool {
    rules.natural_regrowth()
        && board.tile(p).terrain == Terrain::Degraded
        && board.forest_neighbours(p) >= crate::rules::SUCCESSION_NEIGHBOURS
}

impl DeductionDef {
    /// Candidates consistent with the given clue indices.
    pub fn consistent(&self, board: &Board, rules: &Ruleset, clue_ids: &[usize]) -> Vec<usize> {
        (0..self.candidates.len())
            .filter(|&c| clue_ids.iter().all(|&k| self.clues[k].test.holds(board, rules, self.candidates[c].at)))
            .collect()
    }

    /// The unique answer under these rules, if the full clue set pins one down.
    pub fn answer(&self, board: &Board, rules: &Ruleset) -> Option<usize> {
        let all: Vec<usize> = (0..self.clues.len()).collect();
        match self.consistent(board, rules, &all).as_slice() {
            [one] => Some(*one),
            _ => None,
        }
    }

    /// Round-robin deal of clue indices to `players` players.
    pub fn deal(&self, players: usize) -> Vec<Vec<usize>> {
        let players = players.max(1);
        let mut hands = vec![Vec::new(); players];
        for k in 0..self.clues.len() {
            hands[k % players].push(k);
        }
        hands
    }

    /// Fairness check used by the classifier: with the clues dealt to `players`,
    /// the combined clues identify one answer and no single hand does.
    pub fn fair_for(&self, board: &Board, rules: &Ruleset, players: usize) -> Result<usize, String> {
        let answer = self.answer(board, rules).ok_or("combined clues do not identify exactly one candidate")?;
        if players >= 2 {
            for (i, hand) in self.deal(players).iter().enumerate() {
                if self.consistent(board, rules, hand).len() <= 1 {
                    return Err(format!("player {} alone can identify the answer with {players} players", i + 1));
                }
            }
        }
        Ok(answer)
    }
}
