use crate::pos::{Pos, DIRS};
use serde::{Deserialize, Serialize};

/// What a cell of the landscape is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terrain {
    /// Healthy evergreen / moist-deciduous forest with closed canopy.
    Forest,
    /// Forest damaged by invasive Lantana, logging or edge effects.
    Degraded,
    /// Cleared land: abandoned plantation edge, scrub or a gap.
    Open,
    /// A road. It cannot be planted.
    Road,
    /// A stream or river channel.
    River,
    /// Montane shola grassland.
    Grassland,
    /// Rocky cliff / outcrop.
    Rock,
    /// A deliberately planted stepping-stone grove (Stepping-Stone Networks only).
    Grove,
    /// A rope canopy bridge over a road (Continuous Corridors only).
    Bridge,
}

impl Terrain {
    pub fn from_char(c: char) -> Option<(Terrain, bool, bool)> {
        // (terrain, fig, nest)
        Some(match c {
            'F' => (Terrain::Forest, false, false),
            'f' => (Terrain::Forest, true, false),
            'n' => (Terrain::Forest, false, true),
            'h' => (Terrain::Forest, true, true),
            'D' => (Terrain::Degraded, false, false),
            'd' => (Terrain::Degraded, true, false),
            'm' => (Terrain::Degraded, false, true),
            '.' => (Terrain::Open, false, false),
            '=' => (Terrain::Road, false, false),
            '~' => (Terrain::River, false, false),
            'g' => (Terrain::Grassland, false, false),
            '^' => (Terrain::Rock, false, false),
            'G' => (Terrain::Grove, false, false),
            'B' => (Terrain::Bridge, false, false),
            _ => return None,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            Terrain::Forest => "forest",
            Terrain::Degraded => "degraded forest",
            Terrain::Open => "open ground",
            Terrain::Road => "road",
            Terrain::River => "stream",
            Terrain::Grassland => "shola grassland",
            Terrain::Rock => "rocky cliff",
            Terrain::Grove => "stepping-stone grove",
            Terrain::Bridge => "canopy bridge",
        }
    }
}

/// One cell. `fig` and `nest` are features that only function while the cell is forest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tile {
    pub terrain: Terrain,
    #[serde(default)]
    pub fig: bool,
    #[serde(default)]
    pub nest: bool,
}

impl Tile {
    pub const fn new(terrain: Terrain) -> Self {
        Tile { terrain, fig: false, nest: false }
    }

    pub fn is_forest(&self) -> bool {
        self.terrain == Terrain::Forest
    }

    /// A fruiting fig only feeds wildlife while its tile is healthy forest.
    pub fn live_fig(&self) -> bool {
        self.fig && self.is_forest()
    }

    pub fn live_nest(&self) -> bool {
        self.nest && self.is_forest()
    }

    pub fn with_terrain(mut self, t: Terrain) -> Self {
        self.terrain = t;
        self
    }
}

pub const MAX_CELLS: usize = 160;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Board {
    pub width: i16,
    pub height: i16,
    pub tiles: Vec<Tile>,
}

impl Board {
    pub fn parse(rows: &[String]) -> Result<Board, String> {
        if rows.is_empty() {
            return Err("map has no rows".into());
        }
        let width = rows[0].chars().count();
        if width == 0 {
            return Err("map rows are empty".into());
        }
        let mut tiles = Vec::with_capacity(width * rows.len());
        for (y, row) in rows.iter().enumerate() {
            if row.chars().count() != width {
                return Err(format!("map row {y} has {} cells, expected {width}", row.chars().count()));
            }
            for (x, c) in row.chars().enumerate() {
                let (terrain, fig, nest) =
                    Terrain::from_char(c).ok_or_else(|| format!("unknown map symbol '{c}' at ({x},{y})"))?;
                tiles.push(Tile { terrain, fig, nest });
            }
        }
        if tiles.len() > MAX_CELLS {
            return Err(format!("map has {} cells; maximum is {MAX_CELLS}", tiles.len()));
        }
        Ok(Board { width: width as i16, height: rows.len() as i16, tiles })
    }

    pub fn in_bounds(&self, p: Pos) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.width && p.y < self.height
    }

    pub fn index(&self, p: Pos) -> usize {
        debug_assert!(self.in_bounds(p));
        (p.y as usize) * (self.width as usize) + p.x as usize
    }

    pub fn pos_of(&self, i: usize) -> Pos {
        Pos::new((i % self.width as usize) as i16, (i / self.width as usize) as i16)
    }

    pub fn get(&self, p: Pos) -> Option<Tile> {
        if self.in_bounds(p) {
            Some(self.tiles[self.index(p)])
        } else {
            None
        }
    }

    /// Panics when out of bounds; callers validate positions first.
    pub fn tile(&self, p: Pos) -> Tile {
        self.tiles[self.index(p)]
    }

    pub fn set(&mut self, p: Pos, t: Tile) {
        let i = self.index(p);
        self.tiles[i] = t;
    }

    pub fn set_terrain(&mut self, p: Pos, terrain: Terrain) {
        let i = self.index(p);
        self.tiles[i].terrain = terrain;
    }

    pub fn positions(&self) -> impl Iterator<Item = Pos> + '_ {
        (0..self.tiles.len()).map(move |i| self.pos_of(i))
    }

    pub fn neighbours(&self, p: Pos) -> impl Iterator<Item = Pos> + '_ {
        DIRS.iter().map(move |&(dx, dy)| p.offset(dx, dy)).filter(move |q| self.in_bounds(*q))
    }

    pub fn count_terrain(&self, t: Terrain) -> usize {
        self.tiles.iter().filter(|x| x.terrain == t).count()
    }

    pub fn adjacent_to(&self, p: Pos, t: Terrain) -> bool {
        self.neighbours(p).any(|q| self.tile(q).terrain == t)
    }

    pub fn forest_neighbours(&self, p: Pos) -> usize {
        self.neighbours(p).filter(|q| self.tile(*q).is_forest()).count()
    }

    /// Render back to map symbols — used by the classifier report and tests.
    pub fn to_rows(&self) -> Vec<String> {
        (0..self.height)
            .map(|y| {
                (0..self.width)
                    .map(|x| {
                        let t = self.tile(Pos::new(x, y));
                        match (t.terrain, t.fig, t.nest) {
                            (Terrain::Forest, true, true) => 'h',
                            (Terrain::Forest, true, false) => 'f',
                            (Terrain::Forest, false, true) => 'n',
                            (Terrain::Forest, _, _) => 'F',
                            (Terrain::Degraded, true, _) => 'd',
                            (Terrain::Degraded, false, true) => 'm',
                            (Terrain::Degraded, _, _) => 'D',
                            (Terrain::Open, _, _) => '.',
                            (Terrain::Road, _, _) => '=',
                            (Terrain::River, _, _) => '~',
                            (Terrain::Grassland, _, _) => 'g',
                            (Terrain::Rock, _, _) => '^',
                            (Terrain::Grove, _, _) => 'G',
                            (Terrain::Bridge, _, _) => 'B',
                        }
                    })
                    .collect()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips() {
        let rows: Vec<String> = ["Ffn.", "D=~g", "^GBh"].iter().map(|s| s.to_string()).collect();
        let b = Board::parse(&rows).unwrap();
        assert_eq!(b.width, 4);
        assert_eq!(b.to_rows(), rows);
        assert!(b.tile(Pos::new(1, 0)).live_fig());
    }

    #[test]
    fn rejects_ragged_and_unknown() {
        assert!(Board::parse(&["FF".into(), "F".into()]).is_err());
        assert!(Board::parse(&["FX".into()]).is_err());
    }
}
