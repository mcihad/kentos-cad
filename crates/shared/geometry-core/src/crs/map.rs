//! The map's system (docs/adr/0583 §3): the project's drawing seen in another system. Nothing in the drawing moves;
//! what is drawn is moved forward, a pointer on the map back. A straight edge of the project's system is a curve of
//! the map's: it is split where its moved middle strays from the middle of its moved ends.

use super::{Choice, System, transform_in};
use crate::Vec2;
use crate::jsmath::{js_max, js_min};

/// The deepest an edge is halved.
const DEPTH: u32 = 8;

/// The project's system seen in the map's.
#[derive(Clone, Debug, PartialEq)]
pub struct MapProjection {
    pub project: System,
    pub map: System,
    pub choices: Vec<Choice>,
    /// How far, in the map's units, a split edge may stray from the moved curve.
    pub tolerance: f64,
}

impl MapProjection {
    /// The view of `project` in `map`; the tolerance about 5 cm in either's units.
    pub fn new(project: System, map: System, choices: Vec<Choice>) -> Self {
        let tolerance = if map.is_geographic() { 5e-7 } else { 0.05 };
        Self {
            project,
            map,
            choices,
            tolerance,
        }
    }

    /// A point of the project's system on the map; none outside the map's reach.
    pub fn forward(&self, p: Vec2) -> Option<Vec2> {
        transform_in(&self.project, &self.map, p, &self.choices)
            .ok()
            .map(|t| t.point)
    }

    /// A point of the map in the project's system; none outside the project's reach.
    pub fn inverse(&self, p: Vec2) -> Option<Vec2> {
        transform_in(&self.map, &self.project, p, &self.choices)
            .ok()
            .map(|t| t.point)
    }

    /// A path of the project's system on the map: its vertices moved, each straight edge split as deep as the
    /// tolerance asks; none when a vertex is not reached.
    pub fn path(&self, pts: &[Vec2], closed: bool) -> Option<Vec<Vec2>> {
        let moved: Vec<Vec2> = pts
            .iter()
            .map(|p| self.forward(*p))
            .collect::<Option<_>>()?;
        if pts.len() < 2 {
            return Some(moved);
        }
        let mut out = Vec::with_capacity(moved.len());
        let edges = if closed { pts.len() } else { pts.len() - 1 };
        out.push(moved[0]);
        for i in 0..edges {
            let j = (i + 1) % pts.len();
            self.split(pts[i], pts[j], moved[i], moved[j], DEPTH, &mut out);
            if closed && j == 0 {
                // A closed ring's last edge ends where it began: its end is not written twice.
                out.pop();
            }
        }
        Some(out)
    }

    /// How many times a mesh of the project's triangles `tris` is halved so that, moved node by node, its flat
    /// triangles stray from the moved surface by no more than `tolerance` (the map's units): an edge's or a
    /// triangle's middle moved strays from the mean of its moved corners by a quarter as much at each halving.
    /// At most `most`; 0 when a corner is not reached (such a triangle is not drawn).
    pub fn mesh_level(&self, tris: &[[Vec2; 3]], tolerance: f64, most: u32) -> u32 {
        let mut worst: f64 = 0.0;
        for t in tris {
            let Some(m) = t
                .iter()
                .map(|p| self.forward(*p))
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let stray = |ps: &[Vec2], ms: &[Vec2]| {
                let k = ps.len() as f64;
                let p = Vec2::new(
                    ps.iter().map(|p| p.x).sum::<f64>() / k,
                    ps.iter().map(|p| p.y).sum::<f64>() / k,
                );
                let chord = Vec2::new(
                    ms.iter().map(|p| p.x).sum::<f64>() / k,
                    ms.iter().map(|p| p.y).sum::<f64>() / k,
                );
                self.forward(p).map_or(0.0, |q| {
                    crate::jsmath::js_hypot(q.x - chord.x, q.y - chord.y)
                })
            };
            for (i, j) in [(0, 1), (1, 2), (2, 0)] {
                worst = js_max(worst, stray(&[t[i], t[j]], &[m[i], m[j]]));
            }
            worst = js_max(worst, stray(t, &m));
        }
        let mut level = 0;
        while level < most && worst > tolerance {
            worst /= 4.0;
            level += 1;
        }
        level
    }

    /// The triangle `t` of the project's system halved `level` times: each small triangle's corners, in the
    /// project's system and on the map, appended to `out` three by three; a small triangle with a corner the map
    /// does not reach is left out.
    pub fn mesh(&self, t: [Vec2; 3], level: u32, out: &mut Vec<[Vec2; 2]>) {
        let n = 1usize << level.min(12);
        let (a, b, c) = (t[0], t[1], t[2]);
        let node = |i: usize, j: usize| {
            let (u, v) = (i as f64 / n as f64, j as f64 / n as f64);
            let p = Vec2::new(
                a.x + (b.x - a.x) * u + (c.x - a.x) * v,
                a.y + (b.y - a.y) * u + (c.y - a.y) * v,
            );
            (p, self.forward(p))
        };
        // Row j of the grid, nodes (i, j) for i + j ≤ n, moved once.
        let mut below: Vec<(Vec2, Option<Vec2>)> = (0..=n).map(|i| node(i, 0)).collect();
        for j in 0..n {
            let above: Vec<(Vec2, Option<Vec2>)> = (0..n - j).map(|i| node(i, j + 1)).collect();
            let mut put = |k: [(Vec2, Option<Vec2>); 3]| {
                if let (Some(x), Some(y), Some(z)) = (k[0].1, k[1].1, k[2].1) {
                    out.extend([[k[0].0, x], [k[1].0, y], [k[2].0, z]]);
                }
            };
            for i in 0..n - j {
                put([below[i], below[i + 1], above[i]]);
                if i + 1 < n - j {
                    put([below[i + 1], above[i + 1], above[i]]);
                }
            }
            below = above;
        }
    }

    /// The cells across a quad of the project's system (corners in order round it) so that its grid, moved node
    /// by node, strays from the moved surface by no more than `tolerance`: a power of two, at most `most`.
    pub fn grid_cells(&self, corners: [Vec2; 4], tolerance: f64, most: usize) -> usize {
        let level = self.mesh_level(
            &[
                [corners[0], corners[1], corners[2]],
                [corners[0], corners[2], corners[3]],
            ],
            tolerance,
            most.max(1).ilog2(),
        );
        1 << level
    }

    /// The `n` × `n` grid of a quad of the project's system (corners in order round it, `corners[0]` the first
    /// row's first), moved: (n + 1)² nodes row by row; none when a node is not reached.
    pub fn grid(&self, corners: [Vec2; 4], n: usize) -> Option<Vec<Vec2>> {
        let n = n.max(1);
        let [p0, p1, p2, p3] = corners;
        let mut out = Vec::with_capacity((n + 1) * (n + 1));
        for j in 0..=n {
            let v = j as f64 / n as f64;
            let (l, r) = (
                Vec2::new(p0.x + (p3.x - p0.x) * v, p0.y + (p3.y - p0.y) * v),
                Vec2::new(p1.x + (p2.x - p1.x) * v, p1.y + (p2.y - p1.y) * v),
            );
            for i in 0..=n {
                let u = i as f64 / n as f64;
                out.push(self.forward(Vec2::new(l.x + (r.x - l.x) * u, l.y + (r.y - l.y) * u))?);
            }
        }
        Some(out)
    }

    /// The map near `p`: `p` moved and the linear map that takes a step from `p` (within about `h` of it) to the
    /// step on the map, `[a, b, c, d]` for (a·dx + b·dy, c·dx + d·dy); central differences over `h`.
    pub fn linear_at(&self, p: Vec2, h: f64) -> Option<(Vec2, [f64; 4])> {
        let h = if h.is_finite() && h > 0.0 { h } else { 1.0 };
        let at = |dx: f64, dy: f64| self.forward(Vec2::new(p.x + dx, p.y + dy));
        let (e, w, n, s) = (at(h, 0.0)?, at(-h, 0.0)?, at(0.0, h)?, at(0.0, -h)?);
        let k = 2.0 * h;
        Some((
            self.forward(p)?,
            [
                (e.x - w.x) / k,
                (n.x - s.x) / k,
                (e.y - w.y) / k,
                (n.y - s.y) / k,
            ],
        ))
    }

    /// The box of the project's system a box of the map shows: its edges walked at 16 points each and its middle
    /// taken back; points the project's system does not reach are left out, none when no point is reached.
    pub fn box_back(&self, b: [f64; 4]) -> Option<[f64; 4]> {
        const STEPS: usize = 16;
        let mut out = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        let mut take = |x: f64, y: f64| {
            if let Some(p) = self.inverse(Vec2::new(x, y)) {
                out = [
                    js_min(out[0], p.x),
                    js_min(out[1], p.y),
                    js_max(out[2], p.x),
                    js_max(out[3], p.y),
                ];
            }
        };
        for k in 0..=STEPS {
            let f = k as f64 / STEPS as f64;
            let (x, y) = (b[0] + (b[2] - b[0]) * f, b[1] + (b[3] - b[1]) * f);
            take(x, b[1]);
            take(x, b[3]);
            take(b[0], y);
            take(b[2], y);
        }
        take((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
        out[0].is_finite().then_some(out)
    }

    /// The edge `a`–`b` (moved `ma`–`mb`) after `ma`: its inner points and `mb`.
    fn split(&self, a: Vec2, b: Vec2, ma: Vec2, mb: Vec2, depth: u32, out: &mut Vec<Vec2>) {
        if depth > 0 {
            let mid = Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            if let Some(mm) = self.forward(mid) {
                let chord = Vec2::new((ma.x + mb.x) / 2.0, (ma.y + mb.y) / 2.0);
                if crate::jsmath::js_hypot(mm.x - chord.x, mm.y - chord.y) > self.tolerance {
                    self.split(a, mid, ma, mm, depth - 1, out);
                    self.split(mid, b, mm, mb, depth - 1, out);
                    return;
                }
            }
        }
        out.push(mb);
    }
}

/// The one copy of `p` this process keeps: views and cameras are copied freely and compared by it, so they hold a
/// reference that lives as long as the process (one per map system and project system a session uses).
pub fn interned(p: MapProjection) -> &'static MapProjection {
    use std::sync::Mutex;
    static KEPT: Mutex<Vec<&'static MapProjection>> = Mutex::new(Vec::new());
    let mut kept = KEPT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(k) = kept.iter().find(|k| ***k == p) {
        return k;
    }
    let k: &'static MapProjection = Box::leak(Box::new(p));
    kept.push(k);
    k
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crs::{Datum, System};

    fn tm(cm: f64) -> System {
        System::Tm {
            datum: Datum::Turef,
            central_meridian: cm,
            scale_factor: 1.0,
            false_easting: 500_000.0,
            false_northing: 0.0,
            latitude_of_origin: None,
        }
    }

    #[test]
    fn a_point_goes_to_the_map_and_comes_back() {
        let m = MapProjection::new(tm(36.0), System::Mercator {}, Vec::new());
        let p = Vec2::new(512_345.678, 4_420_123.456);
        let q = m.forward(p).expect("on the map");
        let back = m.inverse(q).expect("back");
        assert!(
            (back.x - p.x).abs() < 1e-6 && (back.y - p.y).abs() < 1e-6,
            "{back:?}"
        );
    }

    #[test]
    fn a_long_edge_is_split_a_short_one_is_not() {
        let m = MapProjection::new(tm(36.0), tm(33.0), Vec::new());
        let short = m
            .path(
                &[
                    Vec2::new(500_000.0, 4_400_000.0),
                    Vec2::new(500_100.0, 4_400_000.0),
                ],
                false,
            )
            .expect("moved");
        assert_eq!(short.len(), 2);
        let long = m
            .path(
                &[
                    Vec2::new(400_000.0, 4_300_000.0),
                    Vec2::new(600_000.0, 4_500_000.0),
                ],
                false,
            )
            .expect("moved");
        assert!(long.len() > 2, "{}", long.len());
        let ring = m
            .path(
                &[
                    Vec2::new(500_000.0, 4_400_000.0),
                    Vec2::new(500_100.0, 4_400_000.0),
                    Vec2::new(500_100.0, 4_400_100.0),
                ],
                true,
            )
            .expect("moved");
        assert_eq!(ring.len(), 3, "a closed ring keeps its vertices once");
    }
}
