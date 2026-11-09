//! Eğimle kot ver (plan/map-cad.md MAP-06, docs/adr/1290): the elevations of a path's vertices from `from` to `to`
//! set along a constant grade, as Civil 3D's Set Grade/Slope Between Points: the start's elevation, then either the
//! end's (the grade between them follows) or the grade (rise over run; the end follows). Each vertex between takes the
//! start's elevation plus the grade times its horizontal distance along the path from the start: an arc edge measured
//! along its arc. Vertices outside the range keep theirs.
//!
//! docs/adr/3380 adds the high or low point (`peak`), as Civil 3D's Insert High/Low Elevation Point: the grade ahead
//! from the start and the grade back from the end (each from its own vertex into the range, rising positive) meet at
//! s = (end − start + back·L) / (ahead + back) along the path; a vertex goes there, an arc's halves keeping its circle,
//! unless an existing vertex of the range lies there (within Köşe ekle's 10⁻⁹ of the edge, `ops::vertex`); the
//! vertices up to it take the grade ahead, those after it the grade back.

use crate::geom::bulge::{bulge_arc, bulge_at, bulge_of_sweep};
use crate::jsmath::{cos, js_hypot, sin};
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

/// Where the high or low point falls (docs/adr/3380).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PeakAt {
    /// On an existing vertex of the range, neither its start nor its end: no vertex is added.
    Vertex(usize),
    /// Inside the edge from vertex `edge` to the next: a new vertex there, the edge's two halves of these bulges (0 for
    /// a straight edge; an arc's halves keep its circle).
    Edge { edge: usize, bulges: [f64; 2] },
}

/// Which the point is: both grades rise towards it (high), both fall towards it (low), or neither (a change of grade).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeakKind {
    High,
    Low,
    Break,
}

/// The range graded through its high or low point.
#[derive(Clone, Debug, PartialEq)]
pub struct Peaked {
    /// Each vertex's new elevation from `from` to `to` as the path has them now (the new vertex's is `z`).
    pub zs: Vec<f64>,
    /// The range's horizontal length (an added vertex does not change it).
    pub length: f64,
    pub at: PeakAt,
    /// The point's horizontal distance from the start along the path, its place and its elevation.
    pub along: f64,
    pub p: Vec2,
    pub z: f64,
    pub kind: PeakKind,
}

/// Why there is no high or low point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoPeak {
    /// `from` < `to` within the path's vertices it is not.
    Range,
    /// A number not finite.
    Number,
    /// The range has no length.
    Length,
    /// The two grades are parallel (ahead = −back): they never meet, or they are one line.
    Parallel,
    /// They meet `along` from the start, not inside the range of `length` (at its start or its end is not inside).
    Outside { along: f64, length: f64 },
}

/// Köşe ekle's nearness to an existing vertex (`ops::vertex::insert_vertex`), as a fraction of the edge.
const AT_VERTEX: f64 = 1e-9;

/// The range's high or low point (docs/adr/3380): the start's and end's elevations, the grade ahead from the start and
/// the grade back from the end (rise over run, each from its vertex into the range; 0.02 is 2 % up).
#[allow(clippy::too_many_arguments)]
pub fn peak(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    from: usize,
    to: usize,
    start: f64,
    end: f64,
    ahead: f64,
    back: f64,
) -> Result<Peaked, NoPeak> {
    if !(from < to && to < pts.len()) {
        return Err(NoPeak::Range);
    }
    if ![start, end, ahead, back].iter().all(|v| v.is_finite()) {
        return Err(NoPeak::Number);
    }
    let edges: Vec<f64> = (from..to).map(|i| edge_length(pts, bulges, i)).collect();
    let mut along = Vec::with_capacity(edges.len() + 1);
    along.push(0.0);
    for len in &edges {
        along.push(along[along.len() - 1] + len);
    }
    let length = along[along.len() - 1];
    if !(length > 0.0) {
        return Err(NoPeak::Length);
    }
    let sum = ahead + back;
    let s = (end - start + back * length) / sum;
    if sum == 0.0 || !s.is_finite() {
        return Err(NoPeak::Parallel);
    }
    let outside = NoPeak::Outside { along: s, length };
    if !(s > 0.0 && s < length) {
        return Err(outside);
    }
    // The edge it falls on: the first of some length whose end it does not pass.
    let Some(k) = (0..edges.len()).find(|&k| edges[k] > 0.0 && s <= along[k + 1]) else {
        return Err(outside);
    };
    let t = ((s - along[k]) / edges[k]).clamp(0.0, 1.0);
    let n = pts.len();
    let (i, j) = (from + k, (from + k + 1) % n);
    let (at, along_at, p) = if t <= AT_VERTEX || t >= 1.0 - AT_VERTEX {
        let v = if t <= AT_VERTEX { k } else { k + 1 };
        if v == 0 || v == edges.len() {
            return Err(outside);
        }
        (PeakAt::Vertex(from + v), along[v], pts[from + v])
    } else {
        let (a, b) = (pts[i], pts[j]);
        match bulge_arc(a, b, bulge_at(bulges, i)) {
            Some(arc) => {
                let angle = arc.a0 + arc.sweep * t;
                let p = Vec2::new(arc.c.x + arc.r * cos(angle), arc.c.y + arc.r * sin(angle));
                let halves = [bulge_of_sweep(arc.sweep * t), bulge_of_sweep(arc.sweep * (1.0 - t))];
                (PeakAt::Edge { edge: i, bulges: halves }, s, p)
            }
            None => (PeakAt::Edge { edge: i, bulges: [0.0, 0.0] }, s, Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)),
        }
    };
    let z = start + ahead * along_at;
    let last = along.len() - 1;
    let zs = along
        .iter()
        .enumerate()
        .map(|(v, &d)| match v {
            0 => start,
            v if v == last => end,
            _ if matches!(at, PeakAt::Vertex(at) if at == from + v) => z,
            _ if d <= along_at => start + ahead * d,
            _ => end + back * (length - d),
        })
        .collect();
    let kind = if ahead > 0.0 && back > 0.0 {
        PeakKind::High
    } else if ahead < 0.0 && back < 0.0 {
        PeakKind::Low
    } else {
        PeakKind::Break
    };
    Ok(Peaked { zs, length, at, along: along_at, p, z, kind })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    #[test]
    fn a_high_point_on_a_straight_edge_and_at_a_vertex() {
        // 0 → 30 → 70 along; start 850, end 850.625; 1/16 ahead, 1/32 back meet at 30, the middle vertex.
        let pts = [v(0.0, 0.0), v(30.0, 0.0), v(30.0, 40.0)];
        let at = peak(&pts, None, 0, 2, 850.0, 850.625, 0.0625, 0.03125).expect("a point");
        assert_eq!((at.at, at.along, at.z, at.kind), (PeakAt::Vertex(1), 30.0, 851.875, PeakKind::High));
        assert_eq!(at.zs, vec![850.0, 851.875, 850.625]);
        // A low point inside the second edge: (850.625 − 850 − 0.03125·70)/(−0.0625 − 0.03125) = 16.666…
        let low = peak(&pts, None, 0, 2, 850.0, 850.625, -0.0625, -0.03125).expect("a point");
        let s = (0.625 - 0.03125 * 70.0) / -0.09375;
        assert!(matches!(low.at, PeakAt::Edge { edge: 0, bulges: [0.0, 0.0] }) && low.kind == PeakKind::Low);
        assert!((low.along - s).abs() < 1e-12 && (low.p.x - s).abs() < 1e-12 && low.p.y == 0.0);
        assert_eq!(low.zs.len(), 3);
    }

    #[test]
    fn an_arc_keeps_its_circle_on_both_halves() {
        // A half circle of radius 10 from (0, 0) to (20, 0), bulge 1: the point a quarter along is 45° round.
        let pts = [v(0.0, 0.0), v(20.0, 0.0)];
        let length = 10.0 * std::f64::consts::PI;
        let (start, ahead, back) = (100.0, 0.03, 0.01);
        // s = L/4: end = start + ahead·s − back·(L − s).
        let s = length / 4.0;
        let end = start + ahead * s - back * (length - s);
        let at = peak(&pts, Some(&[1.0, 0.0]), 0, 1, start, end, ahead, back).expect("a point");
        let PeakAt::Edge { edge: 0, bulges: [b1, b2] } = at.at else { panic!("{:?}", at.at) };
        assert!((at.along - s).abs() < 1e-9);
        assert!((js_hypot(at.p.x - 10.0, at.p.y) - 10.0).abs() < 1e-9, "on the circle");
        // Bulge 1 sweeps π: a quarter is π/4, its bulge tan(π/16); the rest tan(3π/16).
        let pi = crate::jsmath::PI;
        assert!((b1 - crate::jsmath::tan(pi / 16.0)).abs() < 1e-12 && (b2 - crate::jsmath::tan(3.0 * pi / 16.0)).abs() < 1e-12, "{b1} {b2}");
    }

    #[test]
    fn no_point_where_the_grades_do_not_meet_inside() {
        let pts = [v(0.0, 0.0), v(30.0, 0.0), v(30.0, 40.0)];
        assert_eq!(peak(&pts, None, 0, 2, 850.0, 851.0, 0.02, -0.02), Err(NoPeak::Parallel));
        // At the end: end = start + ahead·L.
        assert_eq!(peak(&pts, None, 0, 2, 850.0, 854.375, 0.0625, 0.03125), Err(NoPeak::Outside { along: 70.0, length: 70.0 }));
        assert!(matches!(peak(&pts, None, 0, 2, 850.0, 851.0, 0.05, -0.03), Err(NoPeak::Outside { along, .. }) if (along + 55.0).abs() < 1e-9));
        assert_eq!(peak(&pts, None, 2, 1, 850.0, 851.0, 0.01, 0.01), Err(NoPeak::Range));
        assert_eq!(peak(&pts, None, 0, 2, f64::NAN, 851.0, 0.01, 0.01), Err(NoPeak::Number));
    }
}
