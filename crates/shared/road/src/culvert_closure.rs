//! How long a flood closes the road over a culvert (docs/adr/0948, plan/road.md Y42; ROAD-DRAIN-08): from a hydrograph,
//! the time the road is under water (Austroads' time of submergence) and the time it is unsafe for each vehicle class
//! and at each hazard class; with ranges of the flow and of the weir's coefficient, the shortest and longest of each.
//!
//! - **The flow over the road rises with the total flow.** The road carries Cd·L·h^1.5, so its share rises with the head
//!   h, and h with the flow. On the crest the flow passes critical depth, so its depth, speed and depth × speed rise with
//!   the flow per metre. Each criterion is a limit on them that, once passed, stays passed as the flow rises: it holds
//!   from one least total flow up — its threshold. It is found on the head over the crest by bisection on the criterion
//!   itself (the shared cases' reference turns each criterion into a limit on the flow per metre instead).
//! - **And with Cd.** At a given flow a larger Cd passes the flow over at a lower head, the culvert carries less and the
//!   road more: every threshold falls. The overtopping's own (the culvert's flow at the crest) does not depend on Cd.
//! - **The hydrograph** is linear between its points: the time above a threshold is found exactly on each piece.
//! - **The range** is exact, not sampled: the shortest time is at the least flow factor and the least Cd, the longest
//!   at the largest of both, since every time rises with both.
//! - **Vehicles** (Shand et al. 2011, ARR Project 10's interim criteria, confirmed by UNSW WRL 2017/07): unsafe where
//!   the depth, the speed or depth × speed passes the class' limit.
//! - Not modelled: the water stored before the culvert (its routing would lower the peak) and a tailwater rising with the
//!   flow.

use crate::culvert_hydraulics::{Culvert, G, Performance, Weir, hazard, perform};

/// A vehicle class' limits for a stationary vehicle: depth (m), speed (m/s) and depth × speed (m²/s).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vehicle {
    pub depth: f64,
    pub speed: f64,
    pub product: f64,
}

/// Shand et al. (2011)'s classes: small passenger, large passenger, large 4WD.
pub const VEHICLES: [(&str, Vehicle); 3] = [
    ("smallPassenger", Vehicle { depth: 0.3, speed: 3.0, product: 0.3 }),
    ("largePassenger", Vehicle { depth: 0.4, speed: 3.0, product: 0.45 }),
    ("large4wd", Vehicle { depth: 0.5, speed: 3.0, product: 0.6 }),
];

impl Vehicle {
    /// Whether water this deep and fast unsettles the vehicle.
    pub fn unsafe_at(&self, depth: f64, speed: f64) -> bool {
        depth > self.depth || speed > self.speed || depth * speed > self.product
    }
}

/// What a criterion asks of the flow on the crest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Criterion {
    /// Any water over the road.
    Submerged,
    /// Unsafe for the vehicle.
    Vehicle(Vehicle),
    /// The hazard class at least this.
    Hazard(u8),
}

impl Criterion {
    /// Whether it holds at a flow's performance.
    pub fn holds(&self, p: &Performance) -> bool {
        match *self {
            Criterion::Submerged => p.overtopping > 0.0,
            Criterion::Vehicle(v) => p.overtopping > 0.0 && v.unsafe_at(p.depth, p.speed),
            Criterion::Hazard(k) => p.overtopping > 0.0 && hazard(p.depth, p.speed) >= k,
        }
    }
}

/// Whether `criterion` holds with `h` (m) of water over the crest of `weir`: the road carries Cd·h^1.5 per metre at
/// critical depth on the crest.
fn holds_over(criterion: Criterion, weir: &Weir, h: f64) -> bool {
    if h <= 0.0 {
        return false;
    }
    let q = weir.cd * h.powf(1.5);
    let depth = (q * q / G).cbrt();
    let speed = (G * depth).sqrt();
    match criterion {
        Criterion::Submerged => true,
        Criterion::Vehicle(v) => v.unsafe_at(depth, speed),
        Criterion::Hazard(k) => hazard(depth, speed) >= k,
    }
}

/// The least total flow (m³/s) at which `criterion` holds, none where it does not up to the flow of `top`, the
/// performance at the flood's peak. The overtopping's is
/// the culvert's flow with the water at the crest. The others are found on the head over the crest, which rises with
/// the flow: by bisection on the criterion itself to the least head h, the flow the culvert's at the crest + h plus
/// the road's (bisecting the total flow, each step a performance, took minutes).
pub fn threshold(culvert: &Culvert, weir: &Weir, tw: f64, criterion: Criterion, top: &Performance) -> Option<f64> {
    let upto = top.culvert + top.overtopping;
    let start = culvert.flow_at(weir.crest, tw);
    if matches!(criterion, Criterion::Submerged) {
        return (start < upto).then_some(start);
    }
    if !criterion.holds(top) {
        return None;
    }
    let (mut lo, mut hi) = (0.0, top.head);
    for _ in 0..200 {
        let m = 0.5 * (lo + hi);
        if holds_over(criterion, weir, m) {
            hi = m;
        } else {
            lo = m;
        }
        if hi - lo <= 1e-15 * hi.max(1e-3) {
            break;
        }
    }
    Some(culvert.flow_at(weir.crest + hi, tw) + weir.cd * weir.length * hi.powf(1.5))
}

/// The spans (hours) where the linear hydrograph `points` (time, flow) is above `q`, and their whole length.
pub fn above(points: &[(f64, f64)], q: f64) -> (f64, Vec<(f64, f64)>) {
    let mut spans: Vec<(f64, f64)> = Vec::new();
    for w in points.windows(2) {
        let ((t0, q0), (t1, q1)) = (w[0], w[1]);
        let cross = |a: f64, b: f64| t0 + (t1 - t0) * (q - a) / (b - a);
        let piece = match (q0 > q, q1 > q) {
            (true, true) => Some((t0, t1)),
            (true, false) => Some((t0, cross(q0, q1))),
            (false, true) => Some((cross(q0, q1), t1)),
            (false, false) => None,
        };
        if let Some((a, b)) = piece.filter(|(a, b)| b > a) {
            match spans.last_mut() {
                Some(last) if (last.1 - a).abs() <= 1e-12 * a.abs().max(1.0) => last.1 = b,
                _ => spans.push((a, b)),
            }
        }
    }
    (spans.iter().map(|(a, b)| b - a).sum(), spans)
}

/// A criterion through a flood: its threshold flow, its hours and when.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub flow: Option<f64>,
    pub hours: f64,
    pub spans: Vec<(f64, f64)>,
}

/// A flood through a culvert at one flow factor and Cd: the peak's performance, then the submergence, the vehicles'
/// closures (in [`VEHICLES`]' order) and the hazard classes H2 to H6.
#[derive(Clone, Debug, PartialEq)]
pub struct Flood {
    pub factor: f64,
    pub cd: f64,
    pub peak_flow: f64,
    pub peak: Performance,
    pub submerged: Span,
    pub vehicles: Vec<Span>,
    pub hazards: Vec<Span>,
}

/// The hydrograph `points` (hours, m³/s) times `factor` through `culvert` and the road `weir` with its Cd `cd`.
pub fn flood(culvert: &Culvert, weir: &Weir, tw: f64, points: &[(f64, f64)], factor: f64, cd: f64) -> Flood {
    let scaled: Vec<(f64, f64)> = points.iter().map(|&(t, q)| (t, q * factor)).collect();
    let weir = Weir { cd, ..*weir };
    let peak_flow = scaled.iter().map(|p| p.1).fold(0.0, f64::max);
    let peak = perform(culvert, &weir, peak_flow, tw);
    let span = |c: Criterion| {
        let flow = threshold(culvert, &weir, tw, c, &peak);
        let (hours, spans) = flow.map_or((0.0, Vec::new()), |q| above(&scaled, q));
        Span { flow, hours, spans }
    };
    Flood {
        factor,
        cd,
        peak_flow,
        peak,
        submerged: span(Criterion::Submerged),
        vehicles: VEHICLES.iter().map(|(_, v)| span(Criterion::Vehicle(*v))).collect(),
        hazards: (2..=6).map(|k| span(Criterion::Hazard(k))).collect(),
    }
}
