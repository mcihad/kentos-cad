//! An alignment built from its PIs (docs/adr/0901 §2–§4): the start, the PIs between, the end. A PI with a radius
//! gets a curve, with a clothoid in and out when their lengths are given; one without a radius, or where the
//! direction does not change, is a kink. The curve's centre is where the two tangents offset inwards by R + p1 and
//! R + p2 meet (p, the clothoid's shift, from its end): so unequal clothoids are placed without a formula's sign to
//! trust. The chain is integrated element by element from the first point; where a curve ends, the integral's end is
//! held against the place the PI's geometry gives, and the larger gap is the residual (ROAD-ALIGN-05).

use super::element::{Element, Kind, integral};

/// A PI as given: its persistent id, place, and the curve on it (radius 0: none; clothoid lengths 0: none). `ove`: the
/// curve runs into the next PI's curve through an ove clothoid (docs/adr/0979): both turn the same way, neither has a
/// clothoid on the side they share, and the clothoid's A is solved, not given.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PiInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub spiral_in: f64,
    pub spiral_out: f64,
    pub ove: bool,
}

/// What a curve on a PI came to.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveReport {
    pub pi: String,
    /// The deflection, left turns positive (radians).
    pub delta: f64,
    pub radius: f64,
    pub spiral_in: f64,
    pub spiral_out: f64,
    /// The tangent lengths, PI to TS and PI to ST.
    pub t_in: f64,
    pub t_out: f64,
    /// The circular part's length.
    pub arc: f64,
    /// The clothoids' shifts.
    pub p_in: f64,
    pub p_out: f64,
    /// The stations of TS, SC, CS and ST (TS = SC and CS = ST without clothoids).
    pub stations: [f64; 4],
    pub centre: [f64; 2],
}

/// Two curves with no tangent between them.
#[derive(Clone, Debug, PartialEq)]
pub struct Join {
    pub between: [String; 2],
    /// Turning the other way (a reverse curve, S) or the same way (a compound curve).
    pub reverse: bool,
    /// The ove clothoid between them, when the first PI asks for one (docs/adr/0979).
    pub ove: Option<Ove>,
}

/// An ove clothoid between two curves turning the same way (docs/adr/0979): its parameter A (solved), its length
/// A²·|1/R2 − 1/R1|, where it starts, and the least gap between the two circles (R_large − R_small − the centres'
/// distance; Stoer 1982: an ove exists only while it is above 0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ove {
    pub a: f64,
    pub length: f64,
    pub station: f64,
    pub gap: f64,
}

/// The largest gaps where the integral meets the PIs' geometry: place (metres) and direction (radians).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Residual {
    pub place: f64,
    pub direction: f64,
}

/// Where two elements meet (docs/adr/0967; ROAD-ALIGN-05): the later one's start, its station, and how far the two
/// part there in place (m), direction (rad) and curvature (1/m), G0, G1 and G2 in CAD's terms. The chain starts each
/// element where the one before ends, so the place and direction are 0 by construction; a clothoid meets its curve
/// and its tangent at their curvatures (0), a circular curve without one meets the tangent with a jump of 1/R.
#[derive(Clone, Debug, PartialEq)]
pub struct Seam {
    pub station: f64,
    pub before: String,
    pub after: String,
    pub place: f64,
    pub direction: f64,
    pub curvature: f64,
}

impl Alignment {
    /// Every join of two elements, in order.
    pub fn seams(&self) -> Vec<Seam> {
        self.elements
            .windows(2)
            .map(|w| {
                let (a, b) = (&w[0], &w[1]);
                let end = a.end();
                Seam {
                    station: b.station,
                    before: a.id.clone(),
                    after: b.id.clone(),
                    place: (end[0] - b.start[0]).hypot(end[1] - b.start[1]),
                    direction: wrap(a.theta(a.length) - b.theta0).abs(),
                    curvature: (a.curvature(a.length) - b.k0).abs(),
                }
            })
            .collect()
    }
}

/// An alignment: its elements in order, its length (the last station), its curves and joins, the residual.
#[derive(Clone, Debug, PartialEq)]
pub struct Alignment {
    pub elements: Vec<Element>,
    pub length: f64,
    pub curves: Vec<CurveReport>,
    pub joins: Vec<Join>,
    pub residual: Residual,
}

/// Why PIs make no alignment.
#[derive(Clone, Debug, PartialEq)]
pub enum ChainError {
    TooFewPoints,
    DuplicateId { pi: String },
    NotFinite { pi: String },
    /// A PI where the one before it is.
    SamePlace { pi: String },
    /// A negative radius or clothoid length, or a curve on the start or the end.
    BadCurve { pi: String },
    /// The direction turns back (180°).
    TurnTooSharp { pi: String },
    /// The clothoids turn more than the deflection.
    SpiralsLong { pi: String, turn: f64, delta: f64 },
    /// Two curves' tangent lengths are longer than the tangent between them.
    CurvesOverlap { a: String, b: String, excess: f64 },
    /// An ove asked where it cannot be: on the start's or the end's neighbour, between curves turning opposite ways or
    /// of one radius, or beside a clothoid on the shared side (`why` says which; docs/adr/0979).
    BadOve { pi: String, why: OveWhy },
    /// No ove joins the two curves on the PIs as they are: even its longest (the second curve's arc used up) leaves
    /// the middle tangent `excess` metres too long.
    OveTooLong { a: String, b: String, excess: f64 },
    /// More than one ove joins them (two A's): which one is meant is the user's to say by moving a PI.
    OveAmbiguous { a: String, b: String, parameters: Vec<f64> },
}

/// Why an ove cannot be asked on a PI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OveWhy {
    /// The PI or the next has no curve, or the next is the end.
    NoNextCurve,
    /// The two curves turn opposite ways: a reverse curve, not an ove.
    Reverse,
    /// The two radii are equal: the clothoid would have no length.
    EqualRadii,
    /// The first curve has an exit clothoid or the second an entry one: the ove takes their place.
    SpiralOnSharedSide,
    /// The next PI asks for an ove too: one curve between two oves is not built yet.
    Chained,
}

/// How far two curves' tangent lengths may pass each other and still meet (1 mm): a tangent this short is none.
pub const MEET: f64 = 1e-3;

fn unit(a: f64) -> [f64; 2] {
    [a.cos(), a.sin()]
}

/// An angle into (−π, π].
pub fn wrap(a: f64) -> f64 {
    let t = std::f64::consts::TAU;
    let mut w = a % t;
    if w <= -std::f64::consts::PI {
        w += t;
    } else if w > std::f64::consts::PI {
        w -= t;
    }
    w
}

/// A clothoid from straight to radius `r` over `l`: its end (x, y) in its own frame, and p and k.
fn spiral_end(r: f64, l: f64) -> ([f64; 2], f64, f64) {
    if l == 0.0 {
        return ([0.0, 0.0], 0.0, 0.0);
    }
    let tau = l / (2.0 * r);
    let e = integral(0.0, 0.0, 1.0 / (r * l), l);
    (e, e[1] - r * (1.0 - tau.cos()), e[0] - r * tau.sin())
}

/// A curve's place, before it is chained.
#[derive(Clone, Copy)]
struct Planned {
    index: usize,
    sign: f64,
    delta: f64,
    radius: f64,
    l_in: f64,
    l_out: f64,
    t_in: f64,
    t_out: f64,
    p_in: f64,
    p_out: f64,
    k_in: f64,
    k_out: f64,
    centre: [f64; 2],
    /// An ove leaves the curve (it is the first of the pair) or enters it (the second).
    ove_out: Option<OvePart>,
    ove_in: Option<OvePart>,
}

/// An ove as one of its curves sees it: its A, whether the radius falls along it (closing: the first curve is the
/// larger) and where it starts and ends, filled in when the chain is integrated.
#[derive(Clone, Copy, Debug)]
struct OvePart {
    a: f64,
    closing: bool,
    gap: f64,
}

/// A curve's centre: on the inward normals of both tangents through `pi`, `oi` from the entry one and `oo` from the
/// exit one; and its feet, `t` along the entry tangent and `u` along the exit tangent from the PI.
fn centre_of(pi: [f64; 2], din: f64, dout: f64, sign: f64, oi: f64, oo: f64) -> ([f64; 2], f64, f64) {
    let (ui, uo) = (unit(din), unit(dout));
    let (ni, no) = ([-ui[1] * sign, ui[0] * sign], [-uo[1] * sign, uo[0] * sign]);
    // centre = PI + oi·ni + t·ui = PI + oo·no + u·uo: solve for t along the entry tangent.
    let d = [oo * no[0] - oi * ni[0], oo * no[1] - oi * ni[1]];
    let det = ui[0] * (-uo[1]) - ui[1] * (-uo[0]);
    let t = (d[0] * (-uo[1]) - d[1] * (-uo[0])) / det;
    let u = (ui[0] * d[1] - ui[1] * d[0]) / det;
    ([pi[0] + oi * ni[0] + t * ui[0], pi[1] + oi * ni[1] + t * ui[1]], t, u)
}

fn plan(pis: &[PiInput], i: usize) -> Result<Option<Planned>, ChainError> {
    let p = &pis[i];
    let (a, b, c) = (&pis[i - 1], p, &pis[i + 1]);
    let din = (b.y - a.y).atan2(b.x - a.x);
    let dout = (c.y - b.y).atan2(c.x - b.x);
    let delta = wrap(dout - din);
    if (delta.abs() - std::f64::consts::PI).abs() < 1e-12 {
        return Err(ChainError::TurnTooSharp { pi: p.id.clone() });
    }
    if p.radius == 0.0 || delta == 0.0 {
        return Ok(None);
    }
    let (r, sign) = (p.radius, delta.signum());
    let turn = (p.spiral_in + p.spiral_out) / (2.0 * r);
    if turn > delta.abs() + 1e-12 {
        return Err(ChainError::SpiralsLong {
            pi: p.id.clone(),
            turn,
            delta: delta.abs(),
        });
    }
    let (_, p_in, k_in) = spiral_end(r, p.spiral_in);
    let (_, p_out, k_out) = spiral_end(r, p.spiral_out);
    let (centre, t, u) = centre_of([b.x, b.y], din, dout, sign, r + p_in, r + p_out);
    // The foot on the entry tangent is t behind the PI's place along it (t ≤ 0); TS is k further back.
    Ok(Some(Planned {
        index: i,
        sign,
        delta,
        radius: r,
        l_in: p.spiral_in,
        l_out: p.spiral_out,
        t_in: -t + k_in,
        t_out: u + k_out,
        p_in,
        p_out,
        k_in,
        k_out,
        centre,
        ove_out: None,
        ove_in: None,
    }))
}

/// How many places an ove's A is sampled at to find where the middle tangent closes (docs/adr/0979 §2).
const OVE_SAMPLES: usize = 512;

/// The ove between curves `c1` (on PI `i`) and `c2` (on PI `i + 1`), solved on the PIs (docs/adr/0979 §2): the
/// middle tangent is the clothoid's origin tangent (curvature 0 at O). Each circle sits where a curve with a clothoid
/// of Λ = A²/R from O would sit, R + p(R, Λ) off the middle tangent; O is x_M(R, Λ) from its foot, before both feet
/// when the radius falls (closing), after both when it grows. The one condition, both O's the same, is a function of
/// A alone, g(A) = |BC| + t2 − u1 + s·(x_M2 − x_M1), s = −1 closing, +1 opening; g(0) is the plain gap between the
/// circles' feet. A is sampled up to where an arc would go negative and each sign change bisected.
fn solve_ove(pis: &[PiInput], i: usize, c1: &Planned, c2: &Planned) -> Result<(Planned, Planned), ChainError> {
    let (a0, b, c, d) = (&pis[i - 1], &pis[i], &pis[i + 1], &pis[i + 2]);
    let din = (b.y - a0.y).atan2(b.x - a0.x);
    let dmid = (c.y - b.y).atan2(c.x - b.x);
    let dout = (d.y - c.y).atan2(d.x - c.x);
    let bc = (c.x - b.x).hypot(c.y - b.y);
    let (r1, r2, sign) = (c1.radius, c2.radius, c1.sign);
    let closing = r1 > r2;
    let s = if closing { -1.0 } else { 1.0 };
    let parts = |a: f64| {
        let (l1, l2) = (a * a / r1, a * a / r2);
        let (_, p1, x1) = spiral_end(r1, l1);
        let (_, p2, x2) = spiral_end(r2, l2);
        let first = centre_of([b.x, b.y], din, dmid, sign, r1 + c1.p_in, r1 + p1);
        let second = centre_of([c.x, c.y], dmid, dout, sign, r2 + p2, r2 + c2.p_out);
        (first, second, x1, x2)
    };
    let g = |a: f64| {
        let ((_, _, u1), (_, t2, _), x1, x2) = parts(a);
        bc + t2 - u1 + s * (x2 - x1)
    };
    // The longest A: the second curve's arc used up when closing, the first's when opening.
    let most = if closing {
        r2 * (2.0 * (c2.delta.abs() - c2.l_out / (2.0 * r2))).max(0.0).sqrt()
    } else {
        r1 * (2.0 * (c1.delta.abs() - c1.l_in / (2.0 * r1))).max(0.0).sqrt()
    };
    let g0 = g(0.0);
    let samples: Vec<(f64, f64)> =
        (0..=OVE_SAMPLES).map(|k| most * k as f64 / OVE_SAMPLES as f64).map(|a| (a, if a == 0.0 { g0 } else { g(a) })).collect();
    let mut roots = Vec::new();
    // Where the plain circles' feet meet (|g(0)| within MEET) the root at A = 0 is no ove: the scan starts past it.
    let from = usize::from(g0.abs() <= MEET);
    for w in samples[from..].windows(2) {
        let ((mut lo, glo), (mut hi, ghi)) = (w[0], w[1]);
        if (glo > 0.0) == (ghi > 0.0) {
            continue;
        }
        let up = glo < ghi;
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if mid <= lo || mid >= hi {
                break;
            }
            if (g(mid) < 0.0) == up {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        roots.push(0.5 * (lo + hi));
    }
    match roots.len() {
        0 => {
            // No sign change: every middle tangent too long (the circles too far apart), or every one overlapping.
            let least = samples.iter().map(|&(_, v)| v).fold(f64::INFINITY, f64::min);
            let most = samples.iter().map(|&(_, v)| v).fold(f64::NEG_INFINITY, f64::max);
            if least > 0.0 {
                Err(ChainError::OveTooLong { a: b.id.clone(), b: c.id.clone(), excess: least })
            } else {
                Err(ChainError::CurvesOverlap { a: b.id.clone(), b: c.id.clone(), excess: -most })
            }
        }
        1 => {
            let a = roots[0];
            let ((centre1, t1, u1), (centre2, _, u2), x1, _) = parts(a);
            let gap = (r1 - r2).abs() - (centre1[0] - centre2[0]).hypot(centre1[1] - centre2[1]);
            let part = OvePart { a, closing, gap };
            let o = u1 + s * x1;
            let first = Planned { t_in: -t1 + c1.k_in, t_out: o, centre: centre1, ove_out: Some(part), ..*c1 };
            let second = Planned { t_in: bc - o, t_out: u2 + c2.k_out, centre: centre2, ove_in: Some(part), ..*c2 };
            Ok((first, second))
        }
        _ => Err(ChainError::OveAmbiguous { a: b.id.clone(), b: c.id.clone(), parameters: roots }),
    }
}

/// Builds the alignment through `pis` (start, PIs, end).
pub fn build(pis: &[PiInput]) -> Result<Alignment, ChainError> {
    if pis.len() < 2 {
        return Err(ChainError::TooFewPoints);
    }
    let mut seen = std::collections::BTreeSet::new();
    for (i, p) in pis.iter().enumerate() {
        if !seen.insert(p.id.as_str()) || p.id.trim().is_empty() {
            return Err(ChainError::DuplicateId { pi: p.id.clone() });
        }
        if ![p.x, p.y, p.radius, p.spiral_in, p.spiral_out].iter().all(|v| v.is_finite()) {
            return Err(ChainError::NotFinite { pi: p.id.clone() });
        }
        let end = i == 0 || i == pis.len() - 1;
        let curved = p.radius != 0.0 || p.spiral_in != 0.0 || p.spiral_out != 0.0;
        if p.radius < 0.0 || p.spiral_in < 0.0 || p.spiral_out < 0.0 || (end && curved) || (p.radius == 0.0 && curved)
        {
            return Err(ChainError::BadCurve { pi: p.id.clone() });
        }
        if i > 0 && (p.x - pis[i - 1].x).hypot(p.y - pis[i - 1].y) == 0.0 {
            return Err(ChainError::SamePlace { pi: p.id.clone() });
        }
    }
    let mut planned: Vec<Option<Planned>> =
        (0..pis.len()).map(|i| if i == 0 || i == pis.len() - 1 { Ok(None) } else { plan(pis, i) }).collect::<Result<_, _>>()?;
    // The oves (docs/adr/0979): each between a PI's curve and the next one's, solved on the PIs.
    for i in 0..pis.len() {
        if !pis[i].ove {
            continue;
        }
        let bad = |why| Err(ChainError::BadOve { pi: pis[i].id.clone(), why });
        let (Some(c1), Some(c2)) = (planned[i], planned.get(i + 1).copied().flatten()) else {
            return bad(OveWhy::NoNextCurve);
        };
        if i + 2 >= pis.len() || i == 0 {
            return bad(OveWhy::NoNextCurve);
        }
        if c1.sign != c2.sign {
            return bad(OveWhy::Reverse);
        }
        if (c1.radius - c2.radius).abs() <= 1e-9 * c1.radius.max(c2.radius) {
            return bad(OveWhy::EqualRadii);
        }
        if c1.l_out != 0.0 || c2.l_in != 0.0 {
            return bad(OveWhy::SpiralOnSharedSide);
        }
        if pis[i + 1].ove || c1.ove_in.is_some() {
            return bad(OveWhy::Chained);
        }
        let (first, second) = solve_ove(pis, i, &c1, &c2)?;
        planned[i] = Some(first);
        planned[i + 1] = Some(second);
    }
    let t_in = |i: usize| planned[i].as_ref().map_or(0.0, |p| p.t_in);
    let t_out = |i: usize| planned[i].as_ref().map_or(0.0, |p| p.t_out);
    // The tangents between, with the overlap refused.
    let mut tangents = Vec::with_capacity(pis.len() - 1);
    for i in 0..pis.len() - 1 {
        let (a, b) = (&pis[i], &pis[i + 1]);
        if planned[i].is_some_and(|c| c.ove_out.is_some()) {
            // Solved to meet: the ove is where the tangent would be.
            tangents.push(0.0);
            continue;
        }
        let len = (b.x - a.x).hypot(b.y - a.y) - t_out(i) - t_in(i + 1);
        if len < -MEET {
            return Err(ChainError::CurvesOverlap {
                a: a.id.clone(),
                b: b.id.clone(),
                excess: -len,
            });
        }
        tangents.push(if len <= MEET { 0.0 } else { len });
    }
    let mut out = Alignment {
        elements: Vec::new(),
        length: 0.0,
        curves: Vec::new(),
        joins: Vec::new(),
        residual: Residual::default(),
    };
    let mut station = 0.0;
    fn push(out: &mut Alignment, station: &mut f64, e: Element) {
        *station += e.length;
        out.elements.push(e);
    }
    for i in 0..pis.len() - 1 {
        let (a, b) = (&pis[i], &pis[i + 1]);
        let dir = (b.y - a.y).atan2(b.x - a.x);
        if tangents[i] > 0.0 {
            let u = unit(dir);
            let start = [a.x + t_out(i) * u[0], a.y + t_out(i) * u[1]];
            let e = Element {
                id: format!("{}/aliyman", a.id),
                kind: Kind::Tangent,
                start,
                theta0: dir,
                length: tangents[i],
                k0: 0.0,
                k1: 0.0,
                station,
            };
            push(&mut out, &mut station, e);
        }
        let Some(c) = planned[i + 1].as_ref() else {
            continue;
        };
        let u = unit(dir);
        let ts = [b.x - c.t_in * u[0], b.y - c.t_in * u[1]];
        let k = c.sign / c.radius;
        let mut stations = [station; 4];
        let mut place = ts;
        let mut theta = dir;
        if c.ove_in.is_some()
            && let Some(last) = out.elements.last()
        {
            // Entered by the ove the curve before wrote: the curve starts where it ends, TS at its start.
            place = last.end();
            theta = last.theta(last.length);
            stations[0] = last.station;
        }
        if c.l_in > 0.0 {
            let e = Element {
                id: format!("{}/giris", b.id),
                kind: Kind::Spiral,
                start: place,
                theta0: theta,
                length: c.l_in,
                k0: 0.0,
                k1: k,
                station,
            };
            place = e.end();
            theta = e.theta(e.length);
            push(&mut out, &mut station, e);
        }
        stations[1] = station;
        // An ove's circle runs on past the middle direction by τ = A²/(2R²) before a closing ove leaves it, and ends that
        // much short after an opening one; the second circle the other way round (docs/adr/0979 §2).
        let tau = |o: &OvePart| o.a * o.a / (2.0 * c.radius * c.radius);
        let more = c.ove_out.map_or(0.0, |o| if o.closing { tau(&o) } else { -tau(&o) })
            + c.ove_in.map_or(0.0, |o| if o.closing { -tau(&o) } else { tau(&o) });
        let arc = c.radius * (c.delta.abs() - (c.l_in + c.l_out) / (2.0 * c.radius) + more);
        if arc > 0.0 {
            let e = Element {
                id: format!("{}/kurp", b.id),
                kind: Kind::Arc,
                start: place,
                theta0: theta,
                length: arc,
                k0: k,
                k1: k,
                station,
            };
            place = e.end();
            theta = e.theta(e.length);
            push(&mut out, &mut station, e);
        }
        stations[2] = station;
        if c.l_out > 0.0 {
            let e = Element {
                id: format!("{}/cikis", b.id),
                kind: Kind::Spiral,
                start: place,
                theta0: theta,
                length: c.l_out,
                k0: k,
                k1: 0.0,
                station,
            };
            place = e.end();
            theta = e.theta(e.length);
            push(&mut out, &mut station, e);
        }
        if let (Some(o), Some(next)) = (c.ove_out, planned.get(c.index + 1).copied().flatten()) {
            let e = Element {
                id: format!("{}/ove", b.id),
                kind: Kind::Spiral,
                start: place,
                theta0: theta,
                length: o.a * o.a * (1.0 / next.radius - 1.0 / c.radius).abs(),
                k0: k,
                k1: next.sign / next.radius,
                station,
            };
            place = e.end();
            theta = e.theta(e.length);
            push(&mut out, &mut station, e);
        }
        stations[3] = station;
        // Where the PI's geometry says the curve ends; an ove's first curve ends inside the next, which checks it.
        if c.ove_out.is_none() {
            let n = &pis[c.index + 1];
            let dout = (n.y - b.y).atan2(n.x - b.x);
            let st = [b.x + c.t_out * dout.cos(), b.y + c.t_out * dout.sin()];
            out.residual.place = out.residual.place.max((place[0] - st[0]).hypot(place[1] - st[1]));
            out.residual.direction = out.residual.direction.max(wrap(theta - dout).abs());
        }
        out.curves.push(CurveReport {
            pi: b.id.clone(),
            delta: c.delta,
            radius: c.radius,
            spiral_in: c.l_in,
            spiral_out: c.l_out,
            t_in: c.t_in,
            t_out: c.t_out,
            arc,
            p_in: c.p_in,
            p_out: c.p_out,
            stations,
            centre: c.centre,
        });
    }
    // Two curves with no tangent between them.
    for i in 1..pis.len().saturating_sub(2) {
        if let (Some(a), Some(b)) = (&planned[i], &planned[i + 1])
            && tangents[i] == 0.0
        {
            let ove = a.ove_out.map(|o| {
                let length = o.a * o.a * (1.0 / b.radius - 1.0 / a.radius).abs();
                let station = out.curves.iter().find(|c| c.pi == pis[a.index].id).map_or(0.0, |c| c.stations[3] - length);
                Ove { a: o.a, length, station, gap: o.gap }
            });
            out.joins.push(Join {
                between: [pis[a.index].id.clone(), pis[b.index].id.clone()],
                reverse: a.sign != b.sign,
                ove,
            });
        }
    }
    out.length = station;
    Ok(out)
}
