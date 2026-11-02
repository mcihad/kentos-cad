//! Places on an alignment (docs/adr/0901 §7): a station's place, direction and curvature, and a point's station and
//! offset. A point's feet are the places where the distance to it is at a local minimum: on a tangent and a curve in
//! closed form; on a clothoid by stepping where the direction turns at most 0.05 rad, looking for f = (p − P)·T going
//! from positive to negative, and refining each root by bisection and Newton to 1e-12 m; at a kink where both sides
//! lead to it. The answer is the nearest foot; feet within 1e-9 m of the same distance go by the smaller station. The
//! offset is positive to the right, looking ahead.

use super::chain::Alignment;
use super::element::{Element, Kind};

/// A place on an alignment.
#[derive(Clone, Debug, PartialEq)]
pub struct Located {
    pub station: f64,
    pub x: f64,
    pub y: f64,
    /// The direction, radians from east counter-clockwise.
    pub direction: f64,
    /// Left turns positive.
    pub curvature: f64,
    /// The element's index.
    pub element: usize,
}

/// A foot of a point.
#[derive(Clone, Debug, PartialEq)]
pub struct Foot {
    pub at: Located,
    /// Right positive.
    pub offset: f64,
    /// At a kink between two elements.
    pub corner: bool,
}

/// Which end a point lies beyond when it has no foot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Start,
    End,
}

/// A point's station: its nearest foot, all its feet by distance, or the end it lies beyond.
#[derive(Clone, Debug, PartialEq)]
pub struct Stationed {
    pub nearest: Option<Foot>,
    pub feet: Vec<Foot>,
    pub beyond: Option<(End, Foot)>,
}

/// Equal distances.
const TIE: f64 = 1e-9;

impl Alignment {
    /// The element a station falls in: the later one at a join, the last one at the end.
    pub fn element_at(&self, station: f64) -> Option<usize> {
        if !(0.0..=self.length).contains(&station) || self.elements.is_empty() {
            return None;
        }
        let i = self.elements.partition_point(|e| e.station <= station);
        Some(i.saturating_sub(1).min(self.elements.len() - 1))
    }

    /// The place at a station; none off the alignment.
    pub fn at(&self, station: f64) -> Option<Located> {
        let i = self.element_at(station)?;
        Some(located(&self.elements[i], i, station - self.elements[i].station))
    }

    /// A point's station and offset.
    pub fn locate(&self, p: [f64; 2]) -> Stationed {
        let mut feet: Vec<Foot> = Vec::new();
        for (i, e) in self.elements.iter().enumerate() {
            for s in feet_on(e, p) {
                feet.push(foot(e, i, s, p, false));
            }
            // A kink: the distance falls into the join from both sides.
            if let Some(next) = self.elements.get(i + 1) {
                let end = e.end();
                let fl = f(e, e.length, end, p);
                let fr = (p[0] - end[0]) * next.theta0.cos() + (p[1] - end[1]) * next.theta0.sin();
                if fl > 0.0 && fr < 0.0 {
                    let mut c = foot(next, i + 1, 0.0, p, true);
                    // The side by the mean direction.
                    let mean = e.theta(e.length) + super::chain::wrap(next.theta0 - e.theta(e.length)) / 2.0;
                    c.offset = side(mean, end, p) * (p[0] - end[0]).hypot(p[1] - end[1]);
                    feet.push(c);
                }
            }
        }
        feet.sort_by(|a, b| {
            let (da, db) = (a.offset.abs(), b.offset.abs());
            if (da - db).abs() <= TIE {
                a.at.station.total_cmp(&b.at.station)
            } else {
                da.total_cmp(&db)
            }
        });
        // Feet found twice (a root at a join) once.
        feet.dedup_by(|b, a| (a.at.station - b.at.station).abs() <= TIE && (a.offset - b.offset).abs() <= TIE);
        let beyond = if feet.is_empty() && !self.elements.is_empty() {
            let first = &self.elements[0];
            let last = self.elements.len() - 1;
            let e = &self.elements[last];
            let a = foot(first, 0, 0.0, p, false);
            let b = foot(e, last, e.length, p, false);
            let da = (p[0] - a.at.x).hypot(p[1] - a.at.y);
            let db = (p[0] - b.at.x).hypot(p[1] - b.at.y);
            Some(if da <= db { (End::Start, a) } else { (End::End, b) })
        } else {
            None
        };
        Stationed {
            nearest: feet.first().cloned(),
            feet,
            beyond,
        }
    }
}

fn located(e: &Element, i: usize, s: f64) -> Located {
    let [x, y] = e.point(s);
    Located {
        station: e.station + s,
        x,
        y,
        direction: e.theta(s),
        curvature: e.curvature(s),
        element: i,
    }
}

/// +1 when `p` is right of the direction at `at`, −1 left, by the cross product.
fn side(direction: f64, at: [f64; 2], p: [f64; 2]) -> f64 {
    let (c, s) = (direction.cos(), direction.sin());
    let cross = c * (p[1] - at[1]) - s * (p[0] - at[0]);
    if cross > 0.0 { -1.0 } else { 1.0 }
}

fn foot(e: &Element, i: usize, s: f64, p: [f64; 2], corner: bool) -> Foot {
    let at = located(e, i, s);
    let d = (p[0] - at.x).hypot(p[1] - at.y);
    let offset = side(at.direction, [at.x, at.y], p) * d;
    Foot { at, offset, corner }
}

/// (p − P(s))·T(s), with P(s) given.
fn f(e: &Element, s: f64, place: [f64; 2], p: [f64; 2]) -> f64 {
    let th = e.theta(s);
    (p[0] - place[0]) * th.cos() + (p[1] - place[1]) * th.sin()
}

fn feet_on(e: &Element, p: [f64; 2]) -> Vec<f64> {
    let l = e.length;
    match e.kind {
        Kind::Tangent => {
            let s = (p[0] - e.start[0]) * e.theta0.cos() + (p[1] - e.start[1]) * e.theta0.sin();
            if (0.0..=l).contains(&s) { vec![s] } else { vec![] }
        }
        Kind::Arc => {
            let k = e.k0;
            let n = [-e.theta0.sin(), e.theta0.cos()];
            let c = [e.start[0] + n[0] / k, e.start[1] + n[1] / k];
            let r0 = [e.start[0] - c[0], e.start[1] - c[1]];
            let q = [p[0] - c[0], p[1] - c[1]];
            if q[0] == 0.0 && q[1] == 0.0 {
                return vec![];
            }
            let alpha = (r0[0] * q[1] - r0[1] * q[0]).atan2(r0[0] * q[0] + r0[1] * q[1]);
            let tau = std::f64::consts::TAU;
            [alpha, alpha - tau, alpha + tau]
                .into_iter()
                .map(|a| a / k)
                .filter(|s| (-1e-12..=l + 1e-12).contains(s))
                .map(|s| s.clamp(0.0, l))
                .collect()
        }
        Kind::Spiral => {
            let turn = e.k0.abs().max(e.k1.abs()) * l;
            let steps = ((turn / 0.05).ceil() as usize).clamp(8, 1 << 16);
            let h = l / steps as f64;
            let fs = |s: f64| f(e, s, e.point(s), p);
            let mut out = Vec::new();
            let mut a = 0.0;
            let mut fa = fs(a);
            for i in 1..=steps {
                let b = if i == steps { l } else { h * i as f64 };
                let fb = fs(b);
                if fa > 0.0 && fb <= 0.0 {
                    out.push(refine(e, p, a, b, fa));
                } else if fa == 0.0 && i == 1 {
                    out.push(0.0);
                }
                a = b;
                fa = fb;
            }
            out
        }
    }
}

/// A root of f in [a, b] (f(a) > 0 ≥ f(b)): Newton inside the bracket, bisection otherwise.
fn refine(e: &Element, p: [f64; 2], mut a: f64, mut b: f64, mut fa: f64) -> f64 {
    let mut s = 0.5 * (a + b);
    for _ in 0..200 {
        let place = e.point(s);
        let fv = f(e, s, place, p);
        if fv == 0.0 {
            return s;
        }
        if (fv > 0.0) == (fa > 0.0) {
            a = s;
            fa = fv;
        } else {
            b = s;
        }
        if b - a <= 1e-12 {
            break;
        }
        // f' = −1 + κ (p − P)·N.
        let th = e.theta(s);
        let fp = -1.0 + e.curvature(s) * (-(p[0] - place[0]) * th.sin() + (p[1] - place[1]) * th.cos());
        let next = if fp != 0.0 { s - fv / fp } else { f64::NAN };
        s = if next.is_finite() && next > a && next < b { next } else { 0.5 * (a + b) };
    }
    s
}
