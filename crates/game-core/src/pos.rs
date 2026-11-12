use serde::{Deserialize, Serialize};

/// A board cell. Serialised as `[x, y]` so level files stay compact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "[i16; 2]", into = "[i16; 2]")]
pub struct Pos {
    pub x: i16,
    pub y: i16,
}

impl From<[i16; 2]> for Pos {
    fn from(a: [i16; 2]) -> Self {
        Pos { x: a[0], y: a[1] }
    }
}

impl From<Pos> for [i16; 2] {
    fn from(p: Pos) -> Self {
        [p.x, p.y]
    }
}

/// Orthogonal directions. Diagonals never count as a habitat link.
pub const DIRS: [(i16, i16); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

impl Pos {
    pub const fn new(x: i16, y: i16) -> Self {
        Pos { x, y }
    }

    pub fn offset(self, dx: i16, dy: i16) -> Pos {
        Pos::new(self.x + dx, self.y + dy)
    }

    pub fn manhattan(self, other: Pos) -> i16 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

impl std::fmt::Display for Pos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}
