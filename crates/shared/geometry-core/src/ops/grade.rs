//! Eğimle kot ver (plan/map-cad.md MAP-06, docs/adr/1290): the elevations of a path's vertices from `from` to `to`
//! set along a constant grade, as Civil 3D's Set Grade/Slope Between Points: the start's elevation, then either the
//! end's (the grade between them follows) or the grade (rise over run; the end follows). Each vertex between takes the
//! start's elevation plus the grade times its horizontal distance along the path from the start: an arc edge measured
//! along its arc. Vertices outside the range keep theirs.

use crate::geom::bulge::{bulge_arc, bulge_at};
use crate::jsmath::js_hypot;
use crate::vec2::Vec2;

/// What the range's end is given by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    /// The end vertex's elevation (m).
    End(f64),
    /// The grade, rise over run (0.02 is 2 %).
    Grade(f64),
}

/// The range graded: each vertex's new elevation from `from` to `to`, the horizontal length between them, the end's
/// elevation and the grade.
#[derive(Clone, Debug, PartialEq)]
pub struct Graded {
    pub zs: Vec<f64>,
    pub length: f64,
    pub end: f64,
    pub grade: f64,
}

/// The horizontal length of the edge from vertex `i` to the next (an arc's along its arc).
fn edge_length(pts: &[Vec2], bulges: Option<&[f64]>, i: usize) -> f64 {
    let n = pts.len();
    let (a, b) = (pts[i], pts[(i + 1) % n]);
    match bulge_arc(a, b, bulge_at(bulges, i)) {
        Some(arc) => arc.r * arc.sweep.abs(),
        None => js_hypot(b.x - a.x, b.y - a.y),
    }
}

/// The range's elevations, or why not: `from` < `to` within the path's vertices, a range of positive length, finite
/// numbers.
pub fn grade(pts: &[Vec2], bulges: Option<&[f64]>, from: usize, to: usize, start: f64, target: Target) -> Result<Graded, &'static str> {
    if !(from < to && to < pts.len()) {
        return Err("range");
    }
    let value = match target {
        Target::End(z) | Target::Grade(z) => z,
    };
    if !(start.is_finite() && value.is_finite()) {
        return Err("number");
    }
    let mut along = Vec::with_capacity(to - from + 1);
    let mut s = 0.0;
    along.push(0.0);
    for i in from..to {
        s += edge_length(pts, bulges, i);
        along.push(s);
    }
    if !(s > 0.0) {
        return Err("length");
    }
    let g = match target {
        Target::End(z) => (z - start) / s,
        Target::Grade(g) => g,
    };
    let zs: Vec<f64> = along.iter().map(|d| start + g * d).collect();
    let end = match target {
        Target::End(z) => z,
        Target::Grade(_) => zs[zs.len() - 1],
    };
    let mut zs = zs;
    // The end as given, not as summed: no last-digit drift.
    if let Some(last) = zs.last_mut() {
        *last = end;
    }
    Ok(Graded { zs, length: s, end, grade: g })
}
