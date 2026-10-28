//! A kitchen run's leg frame (docs/adr/0706, 0727): where a point of a leg lies, along its line and into the run's side.
//! How a run lays its modules is the document's rule (`kentos_domain::casework`).

use crate::jsmath::{atan2, js_hypot};
use crate::vec2::Vec2;

/// A line shorter than this has no length.
const SLACK: f64 = 1e-9;

/// A run's frame: its line's start, its unit direction, the unit normal toward its side and its length; none for a
/// line with no length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub from: Vec2,
    pub along: Vec2,
    pub into: Vec2,
    pub length: f64,
}

impl Frame {
    /// The frame of the line `from`→`to`, its side the left of it (`left`) or the right.
    pub fn of(from: Vec2, to: Vec2, left: bool) -> Option<Frame> {
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let length = js_hypot(dx, dy);
        if !(length > SLACK) {
            return None;
        }
        let along = Vec2::new(dx / length, dy / length);
        let into = if left {
            Vec2::new(-along.y, along.x)
        } else {
            Vec2::new(along.y, -along.x)
        };
        Some(Frame {
            from,
            along,
            into,
            length,
        })
    }

    /// The point `a` along the line and `b` into the run's side.
    pub fn at(&self, a: f64, b: f64) -> Vec2 {
        Vec2::new(
            self.from.x + self.along.x * a + self.into.x * b,
            self.from.y + self.along.y * a + self.into.y * b,
        )
    }

    /// The turn of a block whose x axis runs along the line: radians counter-clockwise from east.
    pub fn rotation(&self) -> f64 {
        atan2(self.along.y, self.along.x)
    }
}
