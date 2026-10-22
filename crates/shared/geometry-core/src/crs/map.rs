//! The map's system (docs/adr/0583 §3): the project's drawing seen in another system. Nothing in the drawing moves;
//! what is drawn is moved forward, a pointer on the map back. A straight edge of the project's system is a curve of
//! the map's: it is split where its moved middle strays from the middle of its moved ends.

use super::{Choice, System, transform_in};
use crate::Vec2;

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
