//! Sight distance along a vertical alignment (docs/adr/0921; ROAD-PROFILE-08): from an eye `eye` metres over the
//! profile at a station, how far ahead (or behind) an object `object` metres over the profile stays in sight. The sight
//! line runs from the eye to the object in the station–elevation plane; it is blocked where the profile rises over it.
//! The obstruction of one line is exact on each piece: a grade's ends, a parabola's vertex of z − line, a circle's
//! point where its grade is the line's. The distance is stepped by the user's `step` until a line is blocked, then
//! bisected to [`RESOLUTION`]; a hidden stretch shorter than the step between two seen ones is not seen (the method's
//! sampling, said with the answer). Civil 3D's and WisDOT's profile sight distance check: the profile only, not the
//! terrain, the plan or obstacles beside the road.

use super::vertical::{Piece, Vertical};

/// The bisection's last bracket (m): the distance found is at most this short of the true one.
pub const RESOLUTION: f64 = 1e-3;

/// Why the sight stops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Until {
    /// The profile hides the object.
    Hidden,
    /// The distance asked was seen.
    Reach,
    /// The profile ends first.
    End,
}

/// The sight from one station in one direction: the distance, why it stops and, when hidden, the station where the
/// profile blocks the first hidden line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seen {
    pub distance: f64,
    pub until: Until,
    pub blocked: Option<f64>,
}

impl Piece {
    /// The highest of z − line over [lo, hi] ∩ the piece, the line z = z0 + m·(t − t0), and where.
    fn over(&self, lo: f64, hi: f64, t0: f64, z0: f64, m: f64) -> Option<(f64, f64)> {
        let (a, b) = self.range();
        let (a, b) = (a.max(lo), b.min(hi));
        if a > b {
            return None;
        }
        let f = |t: f64| self.at(t).0 - (z0 + m * (t - t0));
        let mut best = (f(a), a);
        let mut take = |t: f64| {
            if (a..=b).contains(&t) {
                let v = f(t);
                if v > best.0 {
                    best = (v, t);
                }
            }
        };
        take(b);
        match *self {
            Piece::Grade { .. } => {}
            Piece::Parabola { s0, g0, r, .. } => {
                if r != 0.0 {
                    take(s0 + (m - g0) / r);
                }
            }
            Piece::Circle { sc, radius, sag, .. } => {
                let k = m * radius / (1.0 + m * m).sqrt();
                take(if sag { sc + k } else { sc - k });
            }
        }
        Some(best)
    }
}

impl Vertical {
    /// The highest the profile rises over the line from (s0, z0) to (s1, z1) between them, and where.
    pub fn obstruction(&self, s0: f64, z0: f64, s1: f64, z1: f64) -> (f64, f64) {
        let (lo, hi) = (s0.min(s1), s0.max(s1));
        let m = (z1 - z0) / (s1 - s0);
        let from = self.pieces.partition_point(|p| p.range().1 < lo);
        let mut best = (f64::NEG_INFINITY, s0);
        for p in &self.pieces[from..] {
            if p.range().0 > hi {
                break;
            }
            if let Some(o) = p.over(lo, hi, s0, z0, m)
                && o.0 > best.0
            {
                best = o;
            }
        }
        best
    }

    /// The sight from station `s` ahead (`ahead`) or behind: the eye `eye` over the profile, the object `object` over
    /// it, looked up to `reach` metres in steps of `step`; none at the profile's end in that direction or off it.
    pub fn sight(&self, s: f64, ahead: bool, eye: f64, object: f64, reach: f64, step: f64) -> Option<Seen> {
        let (z, _, _) = self.at(s)?;
        let room = if ahead { self.end - s } else { s - self.start };
        if room <= 0.0 {
            return None;
        }
        let most = reach.min(room);
        let until = if room <= reach { Until::End } else { Until::Reach };
        let ze = z + eye;
        let sign = if ahead { 1.0 } else { -1.0 };
        let seen = |d: f64| -> (bool, f64) {
            let t = s + sign * d;
            let zo = self.at(t).map_or(f64::NAN, |p| p.0) + object;
            let (rise, at) = self.obstruction(s, ze, t, zo);
            (rise <= 0.0, at)
        };
        let mut lo = 0.0;
        let mut k = 1.0;
        loop {
            let d = (k * step).min(most);
            let (ok, _) = seen(d);
            if !ok {
                let mut hi = d;
                let mut blocked = seen(hi).1;
                while hi - lo > RESOLUTION {
                    let mid = 0.5 * (lo + hi);
                    let (ok, at) = seen(mid);
                    if ok {
                        lo = mid;
                    } else {
                        hi = mid;
                        blocked = at;
                    }
                }
                return Some(Seen { distance: lo, until: Until::Hidden, blocked: Some(blocked) });
            }
            if d >= most {
                return Some(Seen { distance: most, until, blocked: None });
            }
            lo = d;
            k += 1.0;
        }
    }
}
