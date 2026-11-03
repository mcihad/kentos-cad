//! An alignment built from its PIs (docs/adr/0901 §2–§4): the start, the PIs between, the end. A PI with a radius
//! gets a curve, with a clothoid in and out when their lengths are given; one without a radius, or where the
//! direction does not change, is a kink. The curve's centre is where the two tangents offset inwards by R + p1 and
//! R + p2 meet (p, the clothoid's shift, from its end): so unequal clothoids are placed without a formula's sign to
//! trust. The chain is integrated element by element from the first point; where a curve ends, the integral's end is
//! held against the place the PI's geometry gives, and the larger gap is the residual (ROAD-ALIGN-05).

use super::element::{Element, Kind, integral};

/// A PI as given: its persistent id, place, and the curve on it (radius 0: none; clothoid lengths 0: none).
#[derive(Clone, Debug, PartialEq)]
pub struct PiInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub spiral_in: f64,
    pub spiral_out: f64,
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
}

/// The largest gaps where the integral meets the PIs' geometry: place (metres) and direction (radians).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Residual {
    pub place: f64,
    pub direction: f64,
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
    centre: [f64; 2],
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
    // The centre: on the inward normals of both tangents, R + p from each.
    let (ui, uo) = (unit(din), unit(dout));
    let (ni, no) = ([-ui[1] * sign, ui[0] * sign], [-uo[1] * sign, uo[0] * sign]);
    let (oi, oo) = (r + p_in, r + p_out);
    // centre = PI + oi·ni + t·ui = PI + oo·no + u·uo: solve for t along the entry tangent.
    let d = [oo * no[0] - oi * ni[0], oo * no[1] - oi * ni[1]];
    let det = ui[0] * (-uo[1]) - ui[1] * (-uo[0]);
    let t = (d[0] * (-uo[1]) - d[1] * (-uo[0])) / det;
    let u = (ui[0] * d[1] - ui[1] * d[0]) / det;
    let centre = [b.x + oi * ni[0] + t * ui[0], b.y + oi * ni[1] + t * ui[1]];
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
        centre,
    }))
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
    let planned: Vec<Option<Planned>> =
        (0..pis.len()).map(|i| if i == 0 || i == pis.len() - 1 { Ok(None) } else { plan(pis, i) }).collect::<Result<_, _>>()?;
    let t_in = |i: usize| planned[i].as_ref().map_or(0.0, |p| p.t_in);
    let t_out = |i: usize| planned[i].as_ref().map_or(0.0, |p| p.t_out);
    // The tangents between, with the overlap refused.
    let mut tangents = Vec::with_capacity(pis.len() - 1);
    for i in 0..pis.len() - 1 {
        let (a, b) = (&pis[i], &pis[i + 1]);
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
        let arc = c.radius * (c.delta.abs() - (c.l_in + c.l_out) / (2.0 * c.radius));
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
        stations[3] = station;
        // Where the PI's geometry says the curve ends.
        let n = &pis[c.index + 1];
        let dout = (n.y - b.y).atan2(n.x - b.x);
        let st = [b.x + c.t_out * dout.cos(), b.y + c.t_out * dout.sin()];
        out.residual.place = out.residual.place.max((place[0] - st[0]).hypot(place[1] - st[1]));
        out.residual.direction = out.residual.direction.max(wrap(theta - dout).abs());
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
            out.joins.push(Join {
                between: [pis[a.index].id.clone(), pis[b.index].id.clone()],
                reverse: a.sign != b.sign,
            });
        }
    }
    out.length = station;
    Ok(out)
}
