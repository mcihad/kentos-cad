//! A wall's path edited (docs/adr/0340 §4, plan/cad.md `CAD-ARCH-10`): split where a point is nearest it, reversed,
//! its end run on to another path, cut back where it crosses another path. Arcs stay arcs, cut exactly
//! (`ops::path::sub_path`); a straight last edge runs on itself, an arc's on a straight edge along its tangent. The
//! paths' bulges are one per vertex, an open path's last 0, as the document keeps a wall's.

use crate::entity::Shape;
use crate::geom::bulge::{reverse_bulge_path, segment_tangent};
use crate::geom::intersect::{Edge, intersect_edges, ray_edge};
use crate::geom::wall::path_shape;
use crate::jsmath::js_max;
use crate::ops::edges::edge_length;
use crate::ops::path::{Path, nearest_s, path_of, point_at_s, sub_path, tangent_at_s};
use crate::vec2::Vec2;

/// A wall's path.
#[derive(Clone, Debug, PartialEq)]
pub struct WallPath {
    pub pts: Vec<Vec2>,
    /// One per vertex; an open path's last 0.
    pub bulges: Vec<f64>,
    pub closed: bool,
}

impl WallPath {
    pub fn new(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool) -> WallPath {
        let n = pts.len();
        let mut b: Vec<f64> = (0..n)
            .map(|i| bulges.and_then(|b| b.get(i)).copied().unwrap_or(0.0))
            .collect();
        if !closed && let Some(last) = b.last_mut() {
            *last = 0.0;
        }
        WallPath {
            pts: pts.to_vec(),
            bulges: b,
            closed,
        }
    }

    pub fn shape(&self) -> Shape {
        path_shape(&self.pts, Some(&self.bulges), self.closed)
    }

    pub fn path(&self) -> Option<Path> {
        path_of(&self.shape())
    }

    /// Whether any edge is an arc.
    pub fn curved(&self) -> bool {
        self.bulges.iter().any(|b| *b != 0.0)
    }

    /// Its points and bulges the other way, an arc turning the other way.
    pub fn reversed(&self) -> WallPath {
        let r = reverse_bulge_path(&self.pts, Some(&self.bulges), self.closed);
        let n = r.pts.len();
        WallPath::new(
            &r.pts,
            Some(&r.bulges.unwrap_or_else(|| vec![0.0; n])),
            self.closed,
        )
    }

    /// The point at an end, and the direction travel leaves the path there (out of the wall).
    pub fn end(&self, at_end: bool) -> (Vec2, Vec2) {
        let n = self.pts.len();
        if at_end {
            (
                self.pts[n - 1],
                segment_tangent(self.pts[n - 2], self.pts[n - 1], self.bulges[n - 2], true),
            )
        } else {
            let t = segment_tangent(self.pts[0], self.pts[1], self.bulges[0], false);
            (self.pts[0], Vec2::new(-t.x, -t.y))
        }
    }
}

fn of_shape(shape: &Shape) -> Option<WallPath> {
    match shape {
        Shape::Line { a, b } => Some(WallPath::new(&[*a, *b], None, false)),
        Shape::Polyline { pts, bulges, .. } => Some(WallPath::new(pts, bulges.as_deref(), false)),
        _ => None,
    }
}

/// The stretch from `s0` to `s1` of the path (past its length round a closed one), as an open path.
pub fn stretch(p: &WallPath, path: &Path, s0: f64, s1: f64) -> Option<WallPath> {
    of_shape(&sub_path(path, s0, s1, &p.shape()).shape)
}

/// Why a path is not split.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitRefusal {
    /// The point is nearest an end of an open path.
    AtEnd,
}

/// The path split where it passes nearest `at`, and how far from it `at` lies (to the path's left positive): an open
/// path in two, a closed one opened there into one open path.
pub fn split(p: &WallPath, at: Vec2) -> Result<(WallPath, Option<WallPath>, f64), SplitRefusal> {
    let path = p.path().ok_or(SplitRefusal::AtEnd)?;
    let s = nearest_s(&path, at);
    let q = point_at_s(&path, s);
    let t = tangent_at_s(&path, s);
    let across = (at.x - q.x) * -t.y + (at.y - q.y) * t.x;
    let eps = 1e-9 * js_max(path.length, 1.0);
    if p.closed {
        let open = stretch(p, &path, s, s + path.length).ok_or(SplitRefusal::AtEnd)?;
        return Ok((open, None, across));
    }
    if s <= eps || s >= path.length - eps {
        return Err(SplitRefusal::AtEnd);
    }
    let a = stretch(p, &path, 0.0, s).ok_or(SplitRefusal::AtEnd)?;
    let b = stretch(p, &path, s, path.length).ok_or(SplitRefusal::AtEnd)?;
    Ok((a, Some(b), across))
}

/// The path's end (`at_end`, else its start) run on along its direction there to the nearest edge of `target` ahead;
/// none when none is ahead.
pub fn extend(p: &WallPath, at_end: bool, target: &[Edge]) -> Option<WallPath> {
    let (o, dir) = p.end(at_end);
    let d = target
        .iter()
        .flat_map(|e| ray_edge(o, dir, e, 1e-12))
        .min_by(f64::total_cmp)?;
    let to = Vec2::new(o.x + dir.x * d, o.y + dir.y * d);
    let mut out = if at_end { p.clone() } else { p.reversed() };
    let n = out.pts.len();
    if out.bulges[n - 2] == 0.0 {
        out.pts[n - 1] = to;
    } else {
        out.pts.push(to);
        out.bulges.push(0.0);
    }
    Some(if at_end { out } else { out.reversed() })
}

/// The arc lengths where `target` crosses the path, inside it, in order.
pub fn crossings(path: &Path, target: &[Edge]) -> Vec<f64> {
    let eps = 1e-9 * js_max(path.length, 1.0);
    let mut out: Vec<f64> = Vec::new();
    for (i, ed) in path.edges.iter().enumerate() {
        let len = edge_length(ed);
        for t in target {
            for h in intersect_edges(ed, t) {
                out.push(path.cum[i] + h.t * len);
            }
        }
    }
    out.retain(|s| *s > eps && *s < path.length - eps);
    out.sort_by(f64::total_cmp);
    out.dedup_by(|a, b| (*a - *b).abs() <= eps);
    out
}

/// The open path cut where `target` crosses it: the stretch between two crossings (or a crossing and an end) that
/// `keep` lies nearest, else the longest; none when it does not cross.
pub fn trim(p: &WallPath, target: &[Edge], keep: Option<Vec2>) -> Option<WallPath> {
    let path = p.path()?;
    let cuts = crossings(&path, target);
    if cuts.is_empty() {
        return None;
    }
    let mut bounds = vec![0.0];
    bounds.extend(&cuts);
    bounds.push(path.length);
    let stretches: Vec<(f64, f64)> = bounds.windows(2).map(|w| (w[0], w[1])).collect();
    let pick = match keep {
        Some(k) => {
            let s = nearest_s(&path, k);
            stretches
                .iter()
                .copied()
                .find(|(a, b)| s >= *a && s <= *b)
                .unwrap_or(stretches[0])
        }
        None => stretches.iter().copied().fold((0.0, 0.0), |best, x| {
            if x.1 - x.0 > best.1 - best.0 { x } else { best }
        }),
    };
    stretch(p, &path, pick.0, pick.1)
}

/// The edges of another wall's path, for `extend` and `trim`.
pub fn edges(p: &WallPath) -> Vec<Edge> {
    p.path().map(|x| x.edges).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    #[test]
    fn a_path_splits_and_a_ring_opens() {
        let p = WallPath::new(&[v(0.0, 0.0), v(6.0, 0.0)], None, false);
        let (a, b, across) = split(&p, v(2.0, 0.05)).unwrap();
        assert_eq!(a.pts, [v(0.0, 0.0), v(2.0, 0.0)]);
        assert_eq!(b.unwrap().pts, [v(2.0, 0.0), v(6.0, 0.0)]);
        assert!((across - 0.05).abs() < 1e-12);
        assert_eq!(split(&p, v(0.0, 0.0)), Err(SplitRefusal::AtEnd));
        let ring = WallPath::new(
            &[v(0.0, 0.0), v(4.0, 0.0), v(4.0, 3.0), v(0.0, 3.0)],
            None,
            true,
        );
        let (open, none, _) = split(&ring, v(2.0, 0.0)).unwrap();
        assert!(none.is_none());
        assert_eq!(open.pts.first(), Some(&v(2.0, 0.0)));
        assert_eq!(open.pts.last(), Some(&v(2.0, 0.0)));
        assert_eq!(open.pts.len(), 6);
    }

    #[test]
    fn an_end_runs_on_and_a_crossing_cuts() {
        let p = WallPath::new(&[v(0.0, 0.0), v(3.0, 0.0)], None, false);
        let host = edges(&WallPath::new(&[v(5.0, -2.0), v(5.0, 2.0)], None, false));
        assert_eq!(
            extend(&p, true, &host).unwrap().pts,
            [v(0.0, 0.0), v(5.0, 0.0)]
        );
        assert!(extend(&p, false, &host).is_none());
        let long = WallPath::new(&[v(0.0, 0.0), v(8.0, 0.0)], None, false);
        assert_eq!(
            trim(&long, &host, None).unwrap().pts,
            [v(0.0, 0.0), v(5.0, 0.0)]
        );
        assert_eq!(
            trim(&long, &host, Some(v(7.0, 0.0))).unwrap().pts,
            [v(5.0, 0.0), v(8.0, 0.0)]
        );
    }
}
