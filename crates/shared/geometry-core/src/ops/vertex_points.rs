//! Köşelere nokta (docs/adr/0152 §5): a named point at every vertex of the
//! given paths. Objects in the drawing's order, each path's vertices in
//! their order; a place two vertices share (1 µm) is one point, its
//! elevation the first of theirs that has one; a place an existing point
//! holds is passed over and counted; names run from the first by Yazı's
//! Artır (`text::edit::increment`; a name not ending in a number stays).
//! The independent reference is `scripts/fixtures/vertex_points_cases.py`.

use std::collections::HashMap;

use crate::api::Op;
use crate::jsmath::js_hypot;
use crate::op;
use crate::ops::elevation::Elevated;
use crate::text::edit::increment;
use crate::vec2::Vec2;

/// “The same place”, metres: the elevation rules' (ADR 0142).
pub const TOUCH: f64 = 1e-6;

/// A point to place: where, its elevation, its name.
#[derive(Clone, Debug, PartialEq)]
pub struct VertexPoint {
    pub p: Vec2,
    pub z: Option<f64>,
    pub name: Option<String>,
}

crate::json_struct!(VertexPoint { p, z, name });

#[derive(Clone, Debug, PartialEq)]
pub struct VertexPoints {
    pub points: Vec<VertexPoint>,
    /// The places passed over because a point is there already.
    pub skipped: usize,
    /// The name after the last point's: where the next run starts.
    pub next: Option<String>,
}

crate::json_struct!(VertexPoints {
    points,
    skipped,
    next
});

/// Places by the cell of a grid `TOUCH` wide they fall in.
#[derive(Default)]
pub(crate) struct Grid {
    cells: HashMap<(i64, i64), Vec<usize>>,
}

impl Grid {
    fn cell(p: Vec2) -> (i64, i64) {
        ((p.x / TOUCH).floor() as i64, (p.y / TOUCH).floor() as i64)
    }

    pub(crate) fn put(&mut self, p: Vec2, item: usize) {
        self.cells.entry(Self::cell(p)).or_default().push(item);
    }

    /// The first item (in the order they were put) within `TOUCH` of `p`.
    pub(crate) fn near(&self, p: Vec2, at: impl Fn(usize) -> Vec2) -> Option<usize> {
        let (cx, cy) = Self::cell(p);
        let mut best: Option<usize> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for &i in self.cells.get(&(cx + dx, cy + dy)).into_iter().flatten() {
                    let q = at(i);
                    if js_hypot(q.x - p.x, q.y - p.y) <= TOUCH && best.is_none_or(|b| i < b) {
                        best = Some(i);
                    }
                }
            }
        }
        best
    }
}

/// The points Köşelere nokta places at the vertices of `objects` (each its
/// paths with their elevations, in the drawing's order), past the places
/// `existing` points hold, named from `first` (none: no names).
pub fn vertex_points(
    objects: &[Vec<Elevated>],
    existing: &[Vec2],
    first: Option<&str>,
) -> VertexPoints {
    let mut held = Grid::default();
    for (i, p) in existing.iter().enumerate() {
        held.put(*p, i);
    }
    let mut points: Vec<VertexPoint> = Vec::new();
    let mut placed = Grid::default();
    // The places passed over, so that a shared one is counted once.
    let mut passed: Vec<Vec2> = Vec::new();
    let mut passed_grid = Grid::default();
    let mut name = first.map(str::to_owned);
    for paths in objects {
        for path in paths {
            for (k, &p) in path.pts.iter().enumerate() {
                let z = path.zs.get(k).copied().flatten();
                if let Some(i) = placed.near(p, |i| points[i].p) {
                    // A shared vertex: its first elevation, or the first given.
                    if points[i].z.is_none() {
                        points[i].z = z;
                    }
                    continue;
                }
                if held.near(p, |i| existing[i]).is_some() {
                    if passed_grid.near(p, |i| passed[i]).is_none() {
                        passed_grid.put(p, passed.len());
                        passed.push(p);
                    }
                    continue;
                }
                placed.put(p, points.len());
                let this = name.clone();
                if let Some(n) = &name {
                    name = Some(increment(n).unwrap_or_else(|| n.clone()));
                }
                points.push(VertexPoint { p, z, name: this });
            }
        }
    }
    VertexPoints {
        skipped: passed.len(),
        next: name,
        points,
    }
}

pub(crate) static OPS: &[Op] = &[op!(
    "vertexPoints",
    |objects: Vec<Vec<Elevated>>, existing: Vec<Vec2>, first: Option<String>| vertex_points(
        &objects,
        &existing,
        first.as_deref()
    )
)];
