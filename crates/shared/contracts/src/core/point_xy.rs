//! A point's `[x, y]` (docs/adr/9401 §9): the geometry core's tests take any point type that gives it
//! (`point_in_ring`, `point_in_rings`), so no crate keeps a copy of them for the contracts' `Vec2`. The two crates do
//! not know each other; an array of two numbers is what they share.

use crate::Vec2;

impl From<Vec2> for [f64; 2] {
    fn from(p: Vec2) -> [f64; 2] {
        [p.x, p.y]
    }
}
