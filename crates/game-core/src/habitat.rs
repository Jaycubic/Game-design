//! Species habitat requirements, evaluated on the current board and rules.

use crate::board::{Board, Terrain};
use crate::connectivity::patch;
use crate::pos::Pos;
use crate::rules::Ruleset;
use crate::species::Species;
use serde::{Deserialize, Serialize};

/// Minimum number of healthy forest cells in a patch for a group to persist.
pub const MACAQUE_MIN_PATCH: usize = 6;
pub const SQUIRREL_MIN_PATCH: usize = 4;
/// How far (in cells) a hornbill will forage from its nest for figs.
pub const HORNBILL_FIG_RANGE: i16 = 3;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "issue", rename_all = "snake_case")]
pub enum HabitatIssue {
    OutOfBounds,
    NotForest,
    NoFigInPatch,
    PatchTooSmall { have: usize, need: usize },
    NoNestTree,
    NoFigInRange,
    NotBesideStream,
    NotGrassland,
    NoRockyRefuge,
}

impl HabitatIssue {
    pub fn explain(&self, species: Species) -> String {
        let name = species.common_name();
        match self {
            HabitatIssue::OutOfBounds => "That spot is off the map.".into(),
            HabitatIssue::NotForest => format!("{name} needs healthy forest here."),
            HabitatIssue::NoFigInPatch => format!("{name} needs a fruiting fig within its connected forest."),
            HabitatIssue::PatchTooSmall { have, need } => {
                format!("{name} needs at least {need} connected forest cells; this patch has {have}.")
            }
            HabitatIssue::NoNestTree => format!("{name} needs a tall old nest tree."),
            HabitatIssue::NoFigInRange => {
                format!("{name} needs a fruiting fig within {HORNBILL_FIG_RANGE} cells of the nest.")
            }
            HabitatIssue::NotBesideStream => format!("{name} needs forest right beside a stream."),
            HabitatIssue::NotGrassland => format!("{name} needs open shola grassland."),
            HabitatIssue::NoRockyRefuge => format!("{name} needs grassland linked to a rocky cliff."),
        }
    }
}

/// Does `at` meet the species' needs? Returns the first unmet need.
pub fn check(board: &Board, rules: &Ruleset, species: Species, at: Pos) -> Result<(), HabitatIssue> {
    let Some(tile) = board.get(at) else { return Err(HabitatIssue::OutOfBounds) };
    let forest_in = |cells: &[Pos]| cells.iter().filter(|p| board.tile(**p).is_forest()).count();
    match species {
        Species::LionTailedMacaque => {
            if !tile.is_forest() {
                return Err(HabitatIssue::NotForest);
            }
            let cells = patch(board, rules, species, at);
            if !cells.iter().any(|p| board.tile(*p).live_fig()) {
                return Err(HabitatIssue::NoFigInPatch);
            }
            let have = forest_in(&cells);
            if have < MACAQUE_MIN_PATCH {
                return Err(HabitatIssue::PatchTooSmall { have, need: MACAQUE_MIN_PATCH });
            }
            Ok(())
        }
        Species::MalabarGiantSquirrel => {
            if !tile.is_forest() {
                return Err(HabitatIssue::NotForest);
            }
            let have = forest_in(&patch(board, rules, species, at));
            if have < SQUIRREL_MIN_PATCH {
                return Err(HabitatIssue::PatchTooSmall { have, need: SQUIRREL_MIN_PATCH });
            }
            Ok(())
        }
        Species::GreatHornbill => {
            if !tile.is_forest() {
                return Err(HabitatIssue::NotForest);
            }
            if !tile.nest {
                return Err(HabitatIssue::NoNestTree);
            }
            let fig_near = board.positions().any(|p| p.manhattan(at) <= HORNBILL_FIG_RANGE && board.tile(p).live_fig());
            if !fig_near {
                return Err(HabitatIssue::NoFigInRange);
            }
            Ok(())
        }
        Species::MalabarGlidingFrog => {
            if !tile.is_forest() {
                return Err(HabitatIssue::NotForest);
            }
            if !board.adjacent_to(at, Terrain::River) {
                return Err(HabitatIssue::NotBesideStream);
            }
            Ok(())
        }
        Species::NilgiriTahr => {
            if tile.terrain != Terrain::Grassland {
                return Err(HabitatIssue::NotGrassland);
            }
            let cells = patch(board, rules, species, at);
            if !cells.iter().any(|p| board.tile(*p).terrain == Terrain::Rock) {
                return Err(HabitatIssue::NoRockyRefuge);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{Connectivity, Recovery};

    fn b(rows: &[&str]) -> Board {
        Board::parse(&rows.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
    }
    const R: Ruleset = Ruleset::new(Connectivity::Continuous, Recovery::Base);

    #[test]
    fn macaque_needs_fig_and_size() {
        let board = b(&["FFF.fF"]);
        assert_eq!(check(&board, &R, Species::LionTailedMacaque, Pos::new(0, 0)), Err(HabitatIssue::NoFigInPatch));
        let board = b(&["FFfFFF"]);
        assert_eq!(check(&board, &R, Species::LionTailedMacaque, Pos::new(0, 0)), Ok(()));
        let board = b(&["FfF..."]);
        assert!(matches!(
            check(&board, &R, Species::LionTailedMacaque, Pos::new(0, 0)),
            Err(HabitatIssue::PatchTooSmall { have: 3, need: 6 })
        ));
    }

    #[test]
    fn degraded_fig_does_not_feed() {
        let board = b(&["n..d"]);
        assert_eq!(check(&board, &R, Species::GreatHornbill, Pos::new(0, 0)), Err(HabitatIssue::NoFigInRange));
        let board = b(&["n..f"]);
        assert_eq!(check(&board, &R, Species::GreatHornbill, Pos::new(0, 0)), Ok(()));
    }

    #[test]
    fn frog_and_tahr() {
        let board = b(&["F~", "gg", "g^"]);
        assert_eq!(check(&board, &R, Species::MalabarGlidingFrog, Pos::new(0, 0)), Ok(()));
        assert_eq!(check(&board, &R, Species::NilgiriTahr, Pos::new(0, 1)), Ok(()));
        let board = b(&["gg.^"]);
        assert_eq!(check(&board, &R, Species::NilgiriTahr, Pos::new(0, 0)), Err(HabitatIssue::NoRockyRefuge));
    }
}
