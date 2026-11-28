//! A point or vector in world units (x = east, y = north; CLAUDE.md §5).

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

crate::json_struct!(Vec2 { x, y });

impl Vec2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A point's `[x, y]`: what the tests written for any point type ask of it (geometry.rs `point_in_ring`).
impl From<Vec2> for [f64; 2] {
    fn from(p: Vec2) -> [f64; 2] {
        [p.x, p.y]
    }
}
