//! A straight kitchen run's modules (docs/adr/0706 §1): base cabinets of one width along a line from its start, the
//! sink's module its own width, what is left of the line a filler at its end; and where a point of the run lies.

use crate::jsmath::{atan2, js_hypot};
use crate::vec2::Vec2;

/// The slack of the module count: a line a hair short of a whole module still holds it.
const SLACK: f64 = 1e-9;

/// How many modules a line of `length` holds: base cabinets `module` wide, one of them `sink` wide when a sink is
/// asked for; 0 when not even one fits.
pub fn module_count(length: f64, module: f64, sink: Option<f64>) -> usize {
    if !(length.is_finite() && module > 0.0) {
        return 0;
    }
    match sink {
        Some(s) if length + SLACK < s => 0,
        Some(s) => ((length - s) / module + SLACK).floor() as usize + 1,
        None => (length / module + SLACK).floor() as usize,
    }
}

/// The modules' starts and widths along the line: `count` of them, the `sink`-th (1-based) `sink`'s width.
pub fn module_spans(count: usize, module: f64, sink: Option<(usize, f64)>) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(count);
    let mut at = 0.0;
    for i in 1..=count {
        let w = match sink {
            Some((k, width)) if k == i => width,
            _ => module,
        };
        out.push((at, w));
        at += w;
    }
    out
}

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
