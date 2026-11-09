//! A surface's contour lines, slope and aspect (plan/road.md Y2.5, ROAD-TIN-15/16): the lines where it crosses each
//! level of an interval from a base (every n-th a major line), as Civil 3D extracts a surface's major and minor
//! contours and Netcad draws a model's eş yükselti eğrileri; and the plane's steepness and the direction it falls
//! towards at a place.
//!
//! A level crosses a kept triangle's edge whose corners lie on its two sides, a corner on the level counting as above
//! it (so a line through a corner is drawn once and a flat triangle on the level makes none). Where it crosses is the
//! edge's own interpolation, written from its lower-numbered corner, so the two triangles of an edge give the same
//! place bit for bit and the pieces join by their edges into lines. A line is given from a fixed place: an open one
//! from its smaller end (x, then y), a closed one from its smallest place, towards the smaller of its two neighbours.

use std::collections::HashMap;

use serde::Serialize;

use super::Surface;

/// The most levels one call draws.
pub const MAX_LEVELS: usize = 10_000;

/// A contour line: its level, whether it is a major one, whether it closes, its places.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContourLine {
    pub z: f64,
    pub major: bool,
    pub closed: bool,
    pub points: Vec<[f64; 2]>,
}

/// A place's slope (rise over run) and aspect (the azimuth it falls towards, radians clockwise from north; none on a
/// flat triangle).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Slope {
    pub slope: f64,
    pub aspect: Option<f64>,
}

/// Why no contours are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContourError {
    /// The interval is not a number above zero, or the base or a level is not finite.
    Interval,
    /// More than [`MAX_LEVELS`] levels cross the surface.
    TooMany,
}

impl Surface {
    /// The slope and aspect at (x, y): none outside, in a void, or on an edge or a corner (two planes meet there).
    pub fn slope_at(&self, x: f64, y: f64) -> Option<Slope> {
        self.strictly_in(x, y).and_then(|t| self.slope_of(t))
    }

    /// The kept triangle (x, y) lies strictly inside (exact signs): none outside, in a void, or on an edge or a corner,
    /// where two planes meet.
    pub fn strictly_in(&self, x: f64, y: f64) -> Option<u32> {
        let t = self.height(x, y).triangle?;
        let [a, b, c] = self.triangles[t as usize].map(|i| {
            let v = self.vertices[i as usize];
            kentos_geometry_core::Vec2::new(v[0], v[1])
        });
        let q = kentos_geometry_core::Vec2::new(x, y);
        let orient = kentos_geometry_core::predicates::orient2d;
        let inside = [orient(a, b, q), orient(b, c, q), orient(c, a, q)].iter().all(|s| *s > 0.0);
        inside.then_some(t)
    }

    /// The slope and aspect of a kept triangle (its index in [`Surface::triangles`]).
    pub fn slope_of(&self, triangle: u32) -> Option<Slope> {
        let t = self.triangles.get(triangle as usize)?;
        let [a, b, c] = t.map(|i| self.vertices[i as usize]);
        let (ux, uy, uz) = (b[0] - a[0], b[1] - a[1], b[2] - a[2]);
        let (vx, vy, vz) = (c[0] - a[0], c[1] - a[1], c[2] - a[2]);
        let d = ux * vy - uy * vx;
        if d == 0.0 {
            return None;
        }
        // z = a + gx·(x − ax) + gy·(y − ay)
        let gx = (uz * vy - uy * vz) / d;
        let gy = (ux * vz - uz * vx) / d;
        let slope = gx.hypot(gy);
        let aspect = (slope > 0.0).then(|| {
            let az = (-gx).atan2(-gy);
            if az < 0.0 { az + std::f64::consts::TAU } else { az }
        });
        Some(Slope { slope, aspect })
    }

    /// The contour lines every `interval` metres from `base`, every `major`-th level (counted from the base) a major
    /// one (0: none).
    pub fn contours(&self, interval: f64, base: f64, major: u32) -> Result<Vec<ContourLine>, ContourError> {
        Ok(self.contours_told(interval, base, major, &|_, _| false)?.unwrap_or_default())
    }

    /// [`Surface::contours`] telling `tick` how many levels it has drawn of all before each level (docs/adr/4040, as
    /// GDAL's contour generator reports its progress and is interrupted through its callback); `tick` answering true
    /// stops it there: none.
    pub fn contours_told(
        &self,
        interval: f64,
        base: f64,
        major: u32,
        tick: &dyn Fn(u64, u64) -> bool,
    ) -> Result<Option<Vec<ContourLine>>, ContourError> {
        if !(interval.is_finite() && interval > 0.0 && base.is_finite()) {
            return Err(ContourError::Interval);
        }
        let used: Vec<f64> = self.triangles.iter().flatten().map(|&i| self.vertices[i as usize][2]).collect();
        let (Some(lo), Some(hi)) = (used.iter().copied().reduce(f64::min), used.iter().copied().reduce(f64::max))
        else {
            return Ok(Some(Vec::new()));
        };
        let first = ((lo - base) / interval).ceil();
        let last = ((hi - base) / interval).floor();
        if !(first.is_finite() && last.is_finite()) {
            return Err(ContourError::Interval);
        }
        if last - first + 1.0 > MAX_LEVELS as f64 {
            return Err(ContourError::TooMany);
        }
        let mut out = Vec::new();
        let levels = (last - first + 1.0).max(0.0) as u64;
        let mut k = first;
        while k <= last {
            if tick((k - first) as u64, levels) {
                return Ok(None);
            }
            let z = base + k * interval;
            let is_major = major > 0 && (k as i64).rem_euclid(i64::from(major)) == 0;
            out.extend(
                self.level_lines(z, &|_| true).into_iter().map(|(points, closed)| ContourLine { z, major: is_major, closed, points }),
            );
            k += 1.0;
        }
        Ok(Some(out))
    }

    /// The lines of the level `z` over the kept triangles `keep` takes (by their index in [`Surface::triangles`]),
    /// each given from its fixed place, in the order of their first places: a line breaks where a triangle is not
    /// taken. With every triangle taken they are [`Surface::contours`]' lines of the level.
    pub fn level_lines(&self, z: f64, keep: &dyn Fn(usize) -> bool) -> Vec<(Vec<[f64; 2]>, bool)> {
        let mut lines: Vec<(Vec<[f64; 2]>, bool)> = self
            .level(z, keep)
            .into_iter()
            .filter_map(|(mut points, closed)| {
                // A line through a corner on the level meets it twice: one place.
                points.dedup();
                if closed && points.len() > 1 && points.first() == points.last() {
                    points.pop();
                }
                // A level touching the surface at a corner only draws no line.
                if points.len() < if closed { 3 } else { 2 } {
                    return None;
                }
                let points = if closed { start_closed(points) } else { start_open(points) };
                Some((points, closed))
            })
            .collect();
        lines.sort_by(|a, b| {
            let (p, q) = (a.0[0], b.0[0]);
            p[0].total_cmp(&q[0]).then(p[1].total_cmp(&q[1]))
        });
        lines
    }

    /// The lines of one level over the triangles `keep` takes, each a chain of places (a closed one ends where it
    /// starts) and whether it closes.
    fn level(&self, z: f64, keep: &dyn Fn(usize) -> bool) -> Vec<(Vec<[f64; 2]>, bool)> {
        let above = |i: u32| self.vertices[i as usize][2] >= z;
        let cross = |a: u32, b: u32| {
            let (a, b) = if a < b { (a, b) } else { (b, a) };
            let (p, q) = (self.vertices[a as usize], self.vertices[b as usize]);
            let t = (z - p[2]) / (q[2] - p[2]);
            // An end on the level is the end itself.
            if t == 0.0 {
                return [p[0], p[1]];
            }
            if t == 1.0 {
                return [q[0], q[1]];
            }
            [p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]
        };
        // Each piece by its two edges; each edge's pieces.
        let mut pieces: Vec<[[u32; 2]; 2]> = Vec::new();
        for (ti, t) in self.triangles.iter().enumerate() {
            if !keep(ti) {
                continue;
            }
            let crossed: Vec<[u32; 2]> = [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])]
                .into_iter()
                .filter(|&(a, b)| above(a) != above(b))
                .map(|(a, b)| if a < b { [a, b] } else { [b, a] })
                .collect();
            if let [e, f] = crossed[..] {
                pieces.push([e, f]);
            }
        }
        let mut at: HashMap<[u32; 2], Vec<usize>> = HashMap::new();
        for (i, p) in pieces.iter().enumerate() {
            for e in p {
                at.entry(*e).or_default().push(i);
            }
        }
        let mut used = vec![false; pieces.len()];
        let mut lines = Vec::new();
        // Open chains start at an edge of one piece (the surface's border); then the closed ones.
        let mut order: Vec<usize> = (0..pieces.len()).collect();
        order.sort_by_key(|&i| pieces[i].iter().map(|e| at[e].len()).min());
        for start in order {
            if used[start] {
                continue;
            }
            let ends = pieces[start];
            let from = if at[&ends[0]].len() == 1 { ends[0] } else if at[&ends[1]].len() == 1 { ends[1] } else { ends[0] };
            let mut chain = vec![from];
            let (mut piece, mut edge) = (start, from);
            loop {
                used[piece] = true;
                let next_edge = if pieces[piece][0] == edge { pieces[piece][1] } else { pieces[piece][0] };
                chain.push(next_edge);
                edge = next_edge;
                match at[&edge].iter().copied().find(|&i| !used[i]) {
                    Some(i) => piece = i,
                    None => break,
                }
            }
            let closed = chain.len() > 2 && chain.first() == chain.last();
            lines.push((chain.iter().map(|e| cross(e[0], e[1])).collect::<Vec<_>>(), closed));
        }
        lines
    }
}

fn less(a: &[f64; 2], b: &[f64; 2]) -> bool {
    a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])).is_lt()
}

fn start_open(mut p: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    if let (Some(a), Some(b)) = (p.first(), p.last())
        && less(b, a)
    {
        p.reverse();
    }
    p
}

fn start_closed(p: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    let n = p.len();
    let Some(s) = (0..n).min_by(|&i, &j| p[i][0].total_cmp(&p[j][0]).then(p[i][1].total_cmp(&p[j][1]))) else {
        return p;
    };
    let (next, prev) = (p[(s + 1) % n], p[(s + n - 1) % n]);
    let forward = less(&next, &prev);
    (0..n).map(|k| if forward { p[(s + k) % n] } else { p[(s + n - k) % n] }).collect()
}

#[cfg(test)]
mod told_tests {
    use std::cell::{Cell, RefCell};

    use crate::surface::{SpotPoint, SurfaceInput, build};

    /// A cone of points: 0 m at the rim, 10 m at the top; levels every metre cross it from 0 to 10.
    fn cone() -> crate::surface::Surface {
        let mut points = vec![SpotPoint { x: 0.0, y: 0.0, z: 10.0, source: None }];
        for k in 0..12 {
            let a = f64::from(k) * std::f64::consts::TAU / 12.0;
            points.push(SpotPoint { x: 20.0 * a.cos(), y: 20.0 * a.sin(), z: 0.0, source: None });
        }
        build(&SurfaceInput { points, ..SurfaceInput::default() }).expect("builds")
    }

    /// Docs/adr/4040: the levels are told one at a time within one whole; a stop asked at the fourth draws no level after
    /// it; told without a stop the lines are the plain call's.
    #[test]
    fn the_levels_are_told_and_a_stop_is_heard_between_them() {
        let s = cone();
        let seen = RefCell::new(Vec::new());
        let lines = s
            .contours_told(1.0, 0.0, 5, &|done, total| {
                seen.borrow_mut().push((done, total));
                false
            })
            .expect("draws")
            .expect("not stopped");
        assert_eq!(lines, s.contours(1.0, 0.0, 5).expect("draws"));
        assert_eq!(seen.into_inner(), (0..11).map(|k| (k, 11)).collect::<Vec<_>>());
        let asked = Cell::new(0);
        let stopped = s.contours_told(1.0, 0.0, 5, &|_, _| {
            asked.set(asked.get() + 1);
            asked.get() > 3
        });
        assert_eq!(stopped, Ok(None));
        assert_eq!(asked.get(), 4);
    }
}
