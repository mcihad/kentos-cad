//! A stair's and a ramp's railings standing over them (docs/adr/8128). Each side's railing line is the plan's (its band's
//! side `RAILING_INSET` inside, mitred at the turns: ADR 0705 §2), cut into pieces over the stretches of its walking
//! line. Each piece has the walking plane its height is measured from: over a stair's flight the line through its
//! treads' nosings, over a ramp's run its surface, over a landing, a tread or a floor that floor. Its balusters stand
//! where a balustered railing's do (ADR 8045 §3), each with the height its foot stands at: a stair's tread, a ramp's
//! surface, a landing. The height over the plane (PAİY 38/1's 1.10 m) and the top rail's depth are the caller's.
//!
//! A balanced stair's winders and a curved stair have no railing here yet: the line of their nosings along a side is no
//! plane.

use crate::geom::arch::{Railings, StairSpec, Walk, railing_line, riser_stations, stair_landings};
use crate::geom::stair_body::winding_zones;
use crate::jsmath::{js_hypot, js_max, js_min};
use crate::vec2::Vec2;

const EPS: f64 = 1e-9;

/// A walking plane: `top + dx·x + dy·y` metres over ±0.00 at a plan point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub top: f64,
    pub dx: f64,
    pub dy: f64,
}

impl Plane {
    /// A flat one at `z`.
    pub fn flat(z: f64) -> Plane {
        Plane { top: z, dx: 0.0, dy: 0.0 }
    }

    /// Its height at plan point `p`.
    pub fn z(&self, p: Vec2) -> f64 {
        self.top + self.dx * p.x + self.dy * p.y
    }

    pub fn is_flat(&self) -> bool {
        self.dx == 0.0 && self.dy == 0.0
    }
}

/// A stretch of a walking line, `[from, to]` along it: on its segment `segment` (none: a flat one, which may turn a
/// corner), its walking plane, and the treads under it (`steps`: each from its distance along the line on, its height;
/// empty: its balusters stand on its plane).
#[derive(Clone, Debug, PartialEq)]
pub struct Stretch {
    pub from: f64,
    pub to: f64,
    pub segment: Option<usize>,
    pub plane: Plane,
    pub steps: Vec<(f64, f64)>,
}

impl Stretch {
    /// The height a baluster's foot stands at: the tread that holds distance `s`, else the plane at plan point `p`.
    fn foot(&self, s: f64, p: Vec2) -> f64 {
        let mut z = self.steps.first().map(|st| st.1);
        for &(from, h) in &self.steps {
            if s >= from - EPS {
                z = Some(h);
            } else {
                break;
            }
        }
        z.unwrap_or_else(|| self.plane.z(p))
    }
}

/// A piece of a railing over one stretch: its side (`left` as one walks up), its line from `a` to `b` (its top rail's
/// middle), the walking plane under it, and its balusters, each its plan point and the height its foot stands at.
#[derive(Clone, Debug, PartialEq)]
pub struct RailRun {
    pub left: bool,
    pub a: Vec2,
    pub b: Vec2,
    pub plane: Plane,
    pub balusters: Vec<(Vec2, f64)>,
}

/// The railings `sides` of walking line `walk`, whose band is `half` either side of it, over its `stretches`: each
/// side's line in pieces, left first, in order along it. Its balusters: one every `step` along each side's line, the
/// first half a step in; each in the piece it falls in (one on two pieces' meeting in the later one). A part of a line
/// over no stretch has no piece.
pub fn rail_runs(walk: &Walk, sides: Option<Railings>, half: f64, stretches: &[Stretch], step: f64) -> Vec<RailRun> {
    let mut out = Vec::new();
    let Some(r) = sides else { return out };
    let n = walk.pts.len();
    if n < 2 || walk.at.len() != n {
        return out;
    }
    for (left, on) in [(true, r.left()), (false, r.right())] {
        if !on {
            continue;
        }
        let line = railing_line(&walk.pts, half, left);
        if line.len() != n {
            continue;
        }
        // Its length walked before the segment in hand.
        let mut walked = 0.0;
        for k in 0..n - 1 {
            let (p, q) = (line[k], line[k + 1]);
            let l = js_hypot(q.x - p.x, q.y - p.y);
            let (o, len) = (walk.pts[k], walk.at[k + 1] - walk.at[k]);
            if len <= EPS || l <= EPS {
                walked += l;
                continue;
            }
            let dir = Vec2::new((walk.pts[k + 1].x - o.x) / len, (walk.pts[k + 1].y - o.y) / len);
            // The line's piece runs along its segment: a point's distance is the walking line's across from it.
            let s0 = walk.at[k] + (p.x - o.x) * dir.x + (p.y - o.y) * dir.y;
            let s1 = s0 + l;
            let at = |s: f64| Vec2::new(p.x + dir.x * (s - s0), p.y + dir.y * (s - s0));
            let mut pieces: Vec<(f64, f64, &Stretch)> = stretches
                .iter()
                .filter(|st| st.segment.is_none_or(|g| g == k))
                .filter_map(|st| {
                    let (lo, hi) = (js_max(st.from, s0), js_min(st.to, s1));
                    (hi - lo > EPS).then_some((lo, hi, st))
                })
                .collect();
            pieces.sort_by(|x, y| x.0.total_cmp(&y.0));
            for (lo, hi, st) in pieces {
                let mut balusters = Vec::new();
                // Its part of the line, by length along the line; the line's end is its last piece's.
                let (start, end) = (walked + (lo - s0), walked + (hi - s0));
                let closes = k + 2 == n && hi >= s1 - EPS;
                if step > 0.0 {
                    // The first baluster at or past its start, by its number along the line.
                    let mut j = js_max(((start - step / 2.0) / step - 1e-6).ceil(), 0.0);
                    loop {
                        let d = step / 2.0 + j * step;
                        if if closes { d > end + EPS } else { d >= end - EPS } {
                            break;
                        }
                        if d >= start - EPS {
                            let s = s0 + (d - walked);
                            let v = at(s);
                            balusters.push((v, st.foot(s, v)));
                        }
                        j += 1.0;
                    }
                }
                out.push(RailRun { left, a: at(lo), b: at(hi), plane: st.plane, balusters });
            }
            walked += l;
        }
    }
    out
}

/// A straight stair's railings standing over it (docs/adr/8128) over its floor `base`, a baluster every `step` along a
/// side: over each run of its treads a riser apart the line through their nosings (each riser's top at its station),
/// the balusters on its treads; over a landing, and the tread before it, flat at the landing's height; before its first
/// riser its floor, past its last its last tread. A balanced stair's winders and a curved stair have none.
pub fn stair_rails(pts: &[Vec2], spec: &StairSpec, base: f64, step: f64) -> Vec<RailRun> {
    if pts.len() < 2 || spec.bulge != 0.0 || spec.railings.is_none() || !(spec.tread > 0.0) {
        return Vec::new();
    }
    let walk = Walk::of(pts);
    let total = walk.length();
    let (stations, _) = riser_stations(&walk, spec);
    let Some(&last) = stations.last() else {
        return Vec::new();
    };
    let rh = spec.riser_height;
    let pitch = rh / spec.tread;
    let landings = stair_landings(&walk, spec);
    let winders = winding_zones(&walk, spec, &stations);
    // Each riser's top: its station and the height it lifts to.
    let nosing = |i: usize| (stations[i], base + rh * (i + 1) as f64);
    let segment_of = |a: f64, b: f64| {
        (0..walk.pts.len() - 1).find(|&k| walk.at[k] <= a + EPS && b <= walk.at[k + 1] + EPS)
    };
    let mut stretches: Vec<Stretch> = Vec::new();
    if stations[0] > EPS {
        stretches.push(Stretch {
            from: 0.0,
            to: stations[0],
            segment: None,
            plane: Plane::flat(base),
            steps: Vec::new(),
        });
    }
    let mut i = 0;
    while i + 1 < stations.len() {
        let (sa, ha) = nosing(i);
        let (sb, _) = nosing(i + 1);
        let wound = winders.iter().any(|(a, b)| sa >= a - EPS && sb <= b + EPS);
        if wound {
            i += 1;
            continue;
        }
        if landings.iter().any(|(x, y)| *x < sb - EPS && *y > sa + EPS) || sb - sa > spec.tread + EPS {
            // A landing between them: the tread and the landing at its height.
            stretches.push(Stretch { from: sa, to: sb, segment: None, plane: Plane::flat(ha), steps: Vec::new() });
            i += 1;
            continue;
        }
        // A run of treads a riser apart on one segment: the line through their nosings.
        let Some(k) = segment_of(sa, sb) else {
            i += 1;
            continue;
        };
        let mut j = i + 1;
        while j + 1 < stations.len() {
            let (s1, _) = nosing(j);
            let (s2, _) = nosing(j + 1);
            let apart = s2 - s1 <= spec.tread + EPS
                && !landings.iter().any(|(x, y)| *x < s2 - EPS && *y > s1 + EPS)
                && !winders.iter().any(|(a, b)| s1 >= a - EPS && s2 <= b + EPS);
            if !apart || segment_of(s1, s2) != Some(k) {
                break;
            }
            j += 1;
        }
        let (o, len) = (walk.pts[k], walk.at[k + 1] - walk.at[k]);
        let dir = Vec2::new((walk.pts[k + 1].x - o.x) / len, (walk.pts[k + 1].y - o.y) / len);
        // z = ha + pitch · (s − sa), s = at[k] + (p − o)·dir.
        let top = ha + pitch * (walk.at[k] - o.x * dir.x - o.y * dir.y - sa);
        stretches.push(Stretch {
            from: sa,
            to: stations[j],
            segment: Some(k),
            plane: Plane { top, dx: pitch * dir.x, dy: pitch * dir.y },
            steps: (i..j).map(nosing).collect(),
        });
        i = j;
    }
    if total - last > EPS {
        let (_, h) = nosing(stations.len() - 1);
        stretches.push(Stretch { from: last, to: total, segment: None, plane: Plane::flat(h), steps: Vec::new() });
    }
    rail_runs(&walk, spec.railings, spec.width / 2.0, &stretches, step)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(risers: f64, rh: f64, tread: f64) -> StairSpec {
        StairSpec {
            bulge: 0.0,
            width: 1.2,
            risers,
            riser_height: rh,
            tread,
            landing: None,
            cut: None,
            numbering: false,
            railings: Some(Railings::Both),
            post: None,
            upper: false,
            winders: None,
        }
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    /// A straight flight of 18 risers of 0.17 m and treads of 0.28 m from ±0.00, 1.20 m wide, its line 4.76 m (17
    /// treads), railings both sides; worked by hand. Its risers stand at 0.28·i, each lifting to 0.17·(i + 1), the last
    /// to 3.06 at the line's end: one run, the line through the nosings z = 0.17 + (0.17/0.28)·x. Each side's line 5 cm
    /// inside its band, y = ±0.55. Its balusters every 0.12 m from 0.06: 40 to 4.74 m, each on its tread, the one at
    /// 0.30 m on the second (0.34), the last on the 17th (2.89).
    #[test]
    fn a_straight_flight_s_railings_follow_the_line_through_its_nosings() {
        let pts = [Vec2::new(0.0, 0.0), Vec2::new(17.0 * 0.28, 0.0)];
        let runs = stair_rails(&pts, &spec(18.0, 0.17, 0.28), 0.0, 0.12);
        assert_eq!(runs.len(), 2, "{runs:?}");
        let pitch = 0.17 / 0.28;
        for (run, y) in runs.iter().zip([0.55, -0.55]) {
            assert!(near(run.a.x, 0.0) && near(run.a.y, y) && near(run.b.x, 4.76) && near(run.b.y, y), "{run:?}");
            for i in 0..18 {
                let x = 0.28 * i as f64;
                assert!(near(run.plane.z(Vec2::new(x, y)), 0.17 * (i + 1) as f64), "nosing {i}: {run:?}");
            }
            assert!(near(run.plane.dx, pitch) && run.plane.dy == 0.0);
            assert_eq!(run.balusters.len(), 40);
            let (first, second, last) = (run.balusters[0], run.balusters[2], run.balusters[39]);
            assert!(near(first.0.x, 0.06) && near(first.1, 0.17), "{first:?}");
            assert!(near(second.0.x, 0.30) && near(second.1, 0.34), "{second:?}");
            assert!(near(last.0.x, 4.74) && near(last.1, 2.89), "{last:?}");
        }
        assert!(runs[0].left && !runs[1].left);
    }

    /// A U stair of 18 risers of 0.17/0.28, its legs 2.84 m apart along x and 1.20 m across (as the body's test): its
    /// two corner landings meet and are one, from 2.24 to 4.64 along its line; riser 9 stands at its start (lifting to
    /// 1.53, the landing's height), riser 10 moves to its end (4.64, lifting to 1.70). Over the first leg the nosings'
    /// line from 0.17 at x = 0 reaches the landing's 1.53 at its edge; the landing is flat at 1.53 round both corners;
    /// over the last leg the line starts a riser higher (1.70 at x = 2.24) and reaches 3.06 at x = 0. The left side,
    /// inside the turns, crosses the landing in 5 cm, 10 cm and 5 cm pieces; the right side outside it.
    #[test]
    fn a_u_stair_s_railings_are_flat_over_its_landing_and_step_up_after_it() {
        let pts = [
            Vec2::new(0.0, 0.0),
            Vec2::new(8.0 * 0.28 + 0.6, 0.0),
            Vec2::new(8.0 * 0.28 + 0.6, 1.2),
            Vec2::new(0.0, 1.2),
        ];
        let runs = stair_rails(&pts, &spec(18.0, 0.17, 0.28), 0.0, 0.12);
        let left: Vec<&RailRun> = runs.iter().filter(|r| r.left).collect();
        let right: Vec<&RailRun> = runs.iter().filter(|r| !r.left).collect();
        assert_eq!((left.len(), right.len()), (5, 5), "{runs:?}");
        let pitch = 0.17 / 0.28;
        for side in [&left, &right] {
            let (first, last) = (side[0], side[4]);
            assert!(near(first.plane.z(Vec2::new(0.0, 0.0)), 0.17) && near(first.plane.dx, pitch), "{first:?}");
            assert!(near(first.plane.z(Vec2::new(2.24, 0.0)), 1.53), "{first:?}");
            assert!(near(first.b.x, 2.24), "{first:?}");
            for flat in &side[1..4] {
                assert!(flat.plane.is_flat() && near(flat.plane.top, 1.53), "{flat:?}");
                for (_, foot) in &flat.balusters {
                    assert!(near(*foot, 1.53));
                }
            }
            assert!(near(last.a.x, 2.24) && near(last.b.x, 0.0), "{last:?}");
            assert!(near(last.plane.z(Vec2::new(2.24, 1.2)), 1.70), "{last:?}");
            assert!(near(last.plane.z(Vec2::new(0.0, 1.2)), 3.06), "{last:?}");
        }
        // The inner side's pieces over the landing: (2.24, 0.55)–(2.29, 0.55), up to (2.29, 0.65), back to (2.24, 0.65).
        let lengths: Vec<f64> = left[1..4].iter().map(|r| js_hypot(r.b.x - r.a.x, r.b.y - r.a.y)).collect();
        assert!(near(lengths[0], 0.05) && near(lengths[1], 0.10) && near(lengths[2], 0.05), "{lengths:?}");
        // The outer side's: along y = −0.55 to x = 3.39, up x = 3.39 to y = 1.75, back to x = 2.24.
        assert!(near(right[1].b.x, 3.39) && near(right[1].b.y, -0.55), "{:?}", right[1]);
        assert!(near(right[2].b.x, 3.39) && near(right[2].b.y, 1.75), "{:?}", right[2]);
        assert!(near(right[3].b.x, 2.24) && near(right[3].b.y, 1.75), "{:?}", right[3]);
        // Its balusters: the inner line is 4.68 m (39 of them), the outer 9.08 m (76).
        let count = |side: &[&RailRun]| side.iter().map(|r| r.balusters.len()).sum::<usize>();
        assert_eq!((count(&left), count(&right)), (39, 76));
    }

    /// A curved stair and a stair without railings have none here.
    #[test]
    fn a_curved_stair_and_one_without_railings_have_none() {
        let pts = [Vec2::new(0.0, 0.0), Vec2::new(4.76, 0.0)];
        let mut s = spec(18.0, 0.17, 0.28);
        s.railings = None;
        assert!(stair_rails(&pts, &s, 0.0, 0.12).is_empty());
        let mut s = spec(18.0, 0.17, 0.28);
        s.bulge = 0.5;
        assert!(stair_rails(&pts, &s, 0.0, 0.12).is_empty());
    }
}
