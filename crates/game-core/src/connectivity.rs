//! Habitat routes under the active Living Charter connectivity rule.

use crate::board::{Board, Terrain};
use crate::pos::{Pos, DIRS};
use crate::rules::{Connectivity, Ruleset};
use crate::species::Species;
use std::collections::VecDeque;

/// Cells a stepping-stone hop may pass over. Rivers and cliffs are never hopped.
pub fn hop_gap(t: Terrain) -> bool {
    matches!(t, Terrain::Open | Terrain::Road | Terrain::Degraded | Terrain::Grassland)
}

fn suitable(board: &Board, species: Species, p: Pos) -> bool {
    board.get(p).is_some_and(|t| species.route_suitable(t.terrain))
}

/// Push every cell directly linked to `p` for this species under `rules`.
pub fn links(board: &Board, rules: &Ruleset, species: Species, p: Pos, out: &mut Vec<Pos>) {
    for &(dx, dy) in &DIRS {
        let q = p.offset(dx, dy);
        if suitable(board, species, q) {
            out.push(q);
        }
    }
    if rules.connectivity == Connectivity::SteppingStone && species.route_suitable(Terrain::Grove) {
        let here_grove = board.tile(p).terrain == Terrain::Grove;
        for &(dx, dy) in &DIRS {
            let mid = p.offset(dx, dy);
            let q = p.offset(2 * dx, 2 * dy);
            let (Some(m), Some(t)) = (board.get(mid), board.get(q)) else { continue };
            if species.route_suitable(m.terrain) || !hop_gap(m.terrain) || !species.route_suitable(t.terrain) {
                continue;
            }
            if here_grove || t.terrain == Terrain::Grove {
                out.push(q);
            }
        }
    }
}

/// Flood fill from `start`. Returns a per-cell reached mask (all false if start is unsuitable).
pub fn reach(board: &Board, rules: &Ruleset, species: Species, start: Pos) -> Vec<bool> {
    let mut seen = vec![false; board.tiles.len()];
    if !suitable(board, species, start) {
        return seen;
    }
    let mut queue = VecDeque::from([start]);
    seen[board.index(start)] = true;
    let mut buf = Vec::with_capacity(8);
    while let Some(p) = queue.pop_front() {
        buf.clear();
        links(board, rules, species, p, &mut buf);
        for &q in &buf {
            let i = board.index(q);
            if !seen[i] {
                seen[i] = true;
                queue.push_back(q);
            }
        }
    }
    seen
}

pub fn connected(board: &Board, rules: &Ruleset, species: Species, a: Pos, b: Pos) -> bool {
    if species.flies() {
        return true;
    }
    board.in_bounds(b) && reach(board, rules, species, a)[board.index(b)]
}

/// The habitat patch (connected component) containing `start`.
pub fn patch(board: &Board, rules: &Ruleset, species: Species, start: Pos) -> Vec<Pos> {
    reach(board, rules, species, start).iter().enumerate().filter(|(_, r)| **r).map(|(i, _)| board.pos_of(i)).collect()
}

/// Shortest route (fewest links) for display: the client lights it up.
pub fn route(board: &Board, rules: &Ruleset, species: Species, a: Pos, b: Pos) -> Option<Vec<Pos>> {
    if !suitable(board, species, a) || !board.in_bounds(b) {
        return None;
    }
    let mut prev: Vec<Option<usize>> = vec![None; board.tiles.len()];
    let mut seen = vec![false; board.tiles.len()];
    let mut queue = VecDeque::from([a]);
    seen[board.index(a)] = true;
    let mut buf = Vec::with_capacity(8);
    while let Some(p) = queue.pop_front() {
        if p == b {
            let mut path = vec![b];
            let mut i = board.index(b);
            while let Some(j) = prev[i] {
                path.push(board.pos_of(j));
                i = j;
            }
            path.reverse();
            return Some(path);
        }
        buf.clear();
        links(board, rules, species, p, &mut buf);
        for &q in &buf {
            let i = board.index(q);
            if !seen[i] {
                seen[i] = true;
                prev[i] = Some(board.index(p));
                queue.push_back(q);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Recovery;

    fn b(rows: &[&str]) -> Board {
        Board::parse(&rows.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
    }
    const SQ: Species = Species::MalabarGiantSquirrel;
    const CC: Ruleset = Ruleset::new(Connectivity::Continuous, Recovery::Base);
    const SS: Ruleset = Ruleset::new(Connectivity::SteppingStone, Recovery::Base);

    #[test]
    fn continuous_needs_unbroken_forest() {
        let board = b(&["FF.FF"]);
        assert!(!connected(&board, &CC, SQ, Pos::new(0, 0), Pos::new(4, 0)));
        let board = b(&["FFFFF"]);
        assert!(connected(&board, &CC, SQ, Pos::new(0, 0), Pos::new(4, 0)));
    }

    #[test]
    fn stepping_stone_hops_one_gap_beside_a_grove() {
        // F G . F : the grove lets the route hop the single open cell.
        let board = b(&["FG.F"]);
        assert!(connected(&board, &SS, SQ, Pos::new(0, 0), Pos::new(3, 0)));
        // Same board under Continuous: groves count as habitat but there is no hop.
        assert!(!connected(&board, &CC, SQ, Pos::new(0, 0), Pos::new(3, 0)));
        // Without a grove nothing hops.
        let board = b(&["FF.F"]);
        assert!(!connected(&board, &SS, SQ, Pos::new(0, 0), Pos::new(3, 0)));
    }

    #[test]
    fn rivers_are_never_hopped_and_bridges_carry_canopy() {
        let board = b(&["FG~F"]);
        assert!(!connected(&board, &SS, SQ, Pos::new(0, 0), Pos::new(3, 0)));
        let board = b(&["FBF"]);
        assert!(connected(&board, &CC, SQ, Pos::new(0, 0), Pos::new(2, 0)));
        // Frogs cannot use a rope bridge.
        assert!(!connected(&board, &CC, Species::MalabarGlidingFrog, Pos::new(0, 0), Pos::new(2, 0)));
    }

    #[test]
    fn route_is_shortest() {
        let board = b(&["FFF", "F.F", "FFF"]);
        let r = route(&board, &CC, SQ, Pos::new(0, 0), Pos::new(2, 0)).unwrap();
        assert_eq!(r.len(), 3);
    }
}
