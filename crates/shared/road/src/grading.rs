//! Platform grading (plan/road.md Y14, docs/adr/0927; ROAD-GRADE-01…03, 08…10): a flat or sloping pad (a plane through
//! a base point, rising `grade` (rise over run) towards `direction`) inside a closed boundary, its cut and fill slopes run out
//! to a terrain surface, as Civil 3D's grading from a footprint (target surface, cut and fill slopes, projection at
//! corners) and Softree's grading platform (slope lines from the feature's points and between them, the daylight line
//! where they meet the target surface) make them.
//!
//! - **Slope lines:** from every corner and every `step` metres along each edge, outward square to the edge; at a
//!   convex corner a fan from one edge's square to the next's, at most `fan` degrees apart (Civil 3D's projection at
//!   corners), at a reflex corner one line along the bisector (the two edges' lines would cross).
//! - **Daylight:** the terrain along a line is linear between the places it crosses a triangle's edge
//!   ([`Surface::section`]), and so is the slope: where the terrain at the pad's edge is above the pad the line rises
//!   one per `cut` horizontal (a cut), else falls one per `fill` (a fill), and its first zero on a stretch is exact. A
//!   line off the terrain, or that meets it beyond `reach`, is open.
//! - **Design surface and volumes:** the pad's ring, the daylight ring and every slope line as breaklines, the daylight
//!   ring as the boundary ([`crate::surface::build`]): the pad and its slopes; cut and fill against the terrain by
//!   [`crate::volume::between_surfaces`]. None while a line is open.
//! - **Balance:** the pad raised or lowered (a vertical offset) until cut and fill meet within a tolerance, by
//!   bisection on a bracket the caller gives: the net (cut − fill) falls as the pad rises. Said with its iterations and
//!   whether it converged; no bracket that changes sign is no balance, never a guessed optimum (ROAD-GRADE-09).

use crate::surface::{self, Breakline, Ring, Surface, SurfaceInput};
use crate::volume::{self, Triangle, Volumes};

/// A pad and how it is graded.
#[derive(Clone, Debug, PartialEq)]
pub struct Platform {
    /// The boundary's corners, closed implicitly; either way round.
    pub corners: Vec<[f64; 2]>,
    /// The plane's reference point (x, y, z).
    pub base: [f64; 3],
    /// The plane's rise towards `direction`, a grade (rise over run).
    pub grade: f64,
    /// Where it rises: degrees counterclockwise from +x.
    pub direction: f64,
    /// The cut slope: horizontal per vertical.
    pub cut: f64,
    /// The fill slope: horizontal per vertical.
    pub fill: f64,
    /// The spacing of slope lines along an edge, m.
    pub step: f64,
    /// The widest angle between two lines of a corner's fan, degrees.
    pub fan: f64,
    /// The longest slope line in plan, m.
    pub reach: f64,
}

/// Which way a slope line goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Cut,
    Fill,
    /// Off the terrain, or no meeting within the reach.
    Open,
}

/// A slope line: where it starts on the pad's edge, its direction in plan, which way it goes, where it meets the
/// terrain, and the corner it starts at or the edge it starts on (the edge from that corner).
#[derive(Clone, Debug, PartialEq)]
pub struct SlopeLine {
    pub from: [f64; 3],
    pub direction: [f64; 2],
    pub side: Side,
    pub daylight: Option<[f64; 3]>,
    pub corner: usize,
    /// Whether it starts at the corner (else along the edge after it).
    pub at_corner: bool,
}

/// A pad graded.
#[derive(Clone, Debug)]
pub struct Graded {
    /// The pad's corners counterclockwise with their heights.
    pub top: Vec<[f64; 3]>,
    pub lines: Vec<SlopeLine>,
    /// How many lines are open.
    pub open: usize,
    /// The pad's plan area and the footprint's (the pad and its slopes), m²; the footprint none while a line is open.
    pub top_area: f64,
    pub footprint: Option<f64>,
    /// The design surface and the volumes against the terrain; none while a line is open, or why it does not build.
    pub design: Option<Surface>,
    pub volumes: Option<Volumes>,
    pub why: Option<String>,
}

/// Why a pad is not graded.
#[derive(Clone, Debug, PartialEq)]
pub enum GradingError {
    /// Fewer than three corners, or a value not finite.
    TooFewCorners,
    NotFinite,
    /// Two corners at the same place, or edges crossing.
    SamePlace { corner: usize },
    Crossing { a: usize, b: usize },
    /// No area.
    NoArea,
    /// A slope, the step, the fan or the reach not positive (or the fan above 90°).
    BadValue { what: &'static str },
}

impl std::fmt::Display for GradingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFewCorners => f.write_str("Platformun sınırı en az üç köşedir."),
            Self::NotFinite => f.write_str("Platformun bir değeri sayı değil."),
            Self::SamePlace { corner } => write!(f, "Platformun {}. köşesi bir öncekiyle aynı yerde.", corner + 1),
            Self::Crossing { a, b } => write!(f, "Platformun sınırı kendini kesiyor: {}. ve {}. kenar.", a + 1, b + 1),
            Self::NoArea => f.write_str("Platformun sınırının alanı yok."),
            Self::BadValue { what } => write!(f, "Platformun {what} sıfırdan büyük olmalı."),
        }
    }
}

/// The plane's height at (x, y), raised by `offset`.
pub fn plane_z(p: &Platform, x: f64, y: f64, offset: f64) -> f64 {
    let d = p.direction.to_radians();
    p.base[2] + offset + p.grade * ((x - p.base[0]) * d.cos() + (y - p.base[1]) * d.sin())
}

fn area2(ring: &[[f64; 2]]) -> f64 {
    let n = ring.len();
    (0..n).map(|i| {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        a[0] * b[1] - b[0] * a[1]
    })
    .sum()
}

/// Whether segments ab and cd cross or touch (sharing no end).
fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let o = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
    let (d1, d2, d3, d4) = (o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d));
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0)) && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0)) {
        return true;
    }
    let on = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
        o(p, q, r) == 0.0 && r[0] >= p[0].min(q[0]) && r[0] <= p[0].max(q[0]) && r[1] >= p[1].min(q[1]) && r[1] <= p[1].max(q[1])
    };
    on(c, d, a) || on(c, d, b) || on(a, b, c) || on(a, b, d)
}

/// The boundary checked and turned counterclockwise.
pub fn ring_of(p: &Platform) -> Result<Vec<[f64; 2]>, GradingError> {
    let n = p.corners.len();
    if n < 3 {
        return Err(GradingError::TooFewCorners);
    }
    let finite = p.corners.iter().all(|c| c[0].is_finite() && c[1].is_finite())
        && p.base.iter().all(|v| v.is_finite())
        && [p.grade, p.direction, p.cut, p.fill, p.step, p.fan, p.reach].iter().all(|v| v.is_finite());
    if !finite {
        return Err(GradingError::NotFinite);
    }
    for (what, v) in [("şev kazı eğimi", p.cut), ("şev dolgu eğimi", p.fill), ("şev çizgisi aralığı", p.step), ("köşe yelpazesi açısı", p.fan), ("şev erimi", p.reach)] {
        if v <= 0.0 {
            return Err(GradingError::BadValue { what });
        }
    }
    if p.fan > 90.0 {
        return Err(GradingError::BadValue { what: "köşe yelpazesi açısı (en çok 90°)" });
    }
    for i in 0..n {
        if p.corners[i] == p.corners[(i + 1) % n] {
            return Err(GradingError::SamePlace { corner: (i + 1) % n });
        }
    }
    for i in 0..n {
        for j in i + 1..n {
            // Neighbours share a corner.
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            if cross(p.corners[i], p.corners[(i + 1) % n], p.corners[j], p.corners[(j + 1) % n]) {
                return Err(GradingError::Crossing { a: i, b: j });
            }
        }
    }
    let a = area2(&p.corners);
    if a == 0.0 {
        return Err(GradingError::NoArea);
    }
    let mut ring = p.corners.clone();
    if a < 0.0 {
        ring.reverse();
    }
    Ok(ring)
}

/// The outward square of edge a→b of a counterclockwise ring.
fn outward(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l = dx.hypot(dy);
    [dy / l, -dx / l]
}

/// Where the line from (0, z0) on the pad's edge, along `dir`, first meets the terrain: its side and the meeting
/// (distance, height).
fn daylight(terrain: &Surface, from: [f64; 2], dir: [f64; 2], z0: f64, p: &Platform) -> (Side, Option<[f64; 2]>) {
    let to = [from[0] + p.reach * dir[0], from[1] + p.reach * dir[1]];
    let t = terrain.section(&[from, to]);
    let Some(first) = t.first().and_then(|q| q.z) else {
        return (Side::Open, None);
    };
    let (side, m) = if first > z0 { (Side::Cut, 1.0 / p.cut) } else { (Side::Fill, -1.0 / p.fill) };
    if first == z0 {
        return (side, Some([0.0, z0]));
    }
    let line = |u: f64| z0 + m * u;
    for w in t.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b.gap {
            return (Side::Open, None);
        }
        let (Some(za), Some(zb)) = (a.z, b.z) else { return (Side::Open, None) };
        let (fa, fb) = (line(a.station) - za, line(b.station) - zb);
        if fa == 0.0 {
            return (side, Some([a.station, za]));
        }
        if (fa < 0.0) != (fb < 0.0) || fb == 0.0 {
            let u = a.station + (b.station - a.station) * fa / (fa - fb);
            return (side, Some([u, line(u)]));
        }
    }
    (Side::Open, None)
}

/// The pad graded on `terrain`, raised by `offset`.
pub fn grade(p: &Platform, terrain: &Surface, offset: f64) -> Result<Graded, GradingError> {
    let ring = ring_of(p)?;
    let n = ring.len();
    let z = |q: [f64; 2]| plane_z(p, q[0], q[1], offset);
    let mut lines = Vec::new();
    let mut push = |at: [f64; 2], dir: [f64; 2], corner: usize, at_corner: bool| {
        let z0 = z(at);
        let (side, hit) = daylight(terrain, at, dir, z0, p);
        lines.push(SlopeLine {
            from: [at[0], at[1], z0],
            direction: dir,
            side,
            daylight: hit.map(|[u, h]| [at[0] + u * dir[0], at[1] + u * dir[1], h]),
            corner,
            at_corner,
        });
    };
    for i in 0..n {
        let (prev, here, next) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
        let (n1, n2) = (outward(prev, here), outward(here, next));
        let turn = (here[0] - prev[0]) * (next[1] - here[1]) - (here[1] - prev[1]) * (next[0] - here[0]);
        if turn > 0.0 {
            // Convex: a fan from the incoming edge's square to the outgoing one's, counterclockwise.
            let (a1, mut a2) = (n1[1].atan2(n1[0]), n2[1].atan2(n2[0]));
            while a2 < a1 {
                a2 += std::f64::consts::TAU;
            }
            let k = ((a2 - a1).to_degrees() / p.fan).ceil().max(1.0) as usize;
            for j in 0..=k {
                let a = a1 + (a2 - a1) * j as f64 / k as f64;
                push(here, [a.cos(), a.sin()], i, true);
            }
        } else {
            // Straight or reflex: one line along the bisector.
            let b = [n1[0] + n2[0], n1[1] + n2[1]];
            let l = b[0].hypot(b[1]);
            let d = if l > 1e-12 { [b[0] / l, b[1] / l] } else { n1 };
            push(here, d, i, true);
        }
        // Along the edge after the corner.
        let length = (next[0] - here[0]).hypot(next[1] - here[1]);
        let k = (length / p.step).ceil() as usize;
        for j in 1..k {
            let t = j as f64 / k as f64;
            push([here[0] + t * (next[0] - here[0]), here[1] + t * (next[1] - here[1])], n2, i, false);
        }
    }
    let top: Vec<[f64; 3]> = ring.iter().map(|q| [q[0], q[1], z(*q)]).collect();
    let top_area = area2(&ring) / 2.0;
    let open = lines.iter().filter(|l| l.side == Side::Open).count();
    let mut graded = Graded { top, lines, open, top_area, footprint: None, design: None, volumes: None, why: None };
    if open > 0 {
        graded.why = Some(format!("{open} şev çizgisi araziyle buluşmuyor: erimi büyütün ya da yüzeyi genişletin."));
        return Ok(graded);
    }
    // The daylight ring; a corner's fan meeting the terrain at the corner itself is one place.
    let mut ring_day: Vec<[f64; 3]> = Vec::new();
    for d in graded.lines.iter().filter_map(|l| l.daylight) {
        if ring_day.last().is_none_or(|q| q[0] != d[0] || q[1] != d[1]) {
            ring_day.push(d);
        }
    }
    while ring_day.len() > 1 && ring_day.first().map(|q| [q[0], q[1]]) == ring_day.last().map(|q| [q[0], q[1]]) {
        ring_day.pop();
    }
    let flat: Vec<[f64; 2]> = ring_day.iter().map(|q| [q[0], q[1]]).collect();
    graded.footprint = Some(area2(&flat).abs() / 2.0);
    let mut breaklines = vec![
        Breakline { points: graded.top.clone(), closed: true, source: None },
        Breakline { points: ring_day.clone(), closed: true, source: None },
    ];
    for l in &graded.lines {
        if let Some(d) = l.daylight
            && (d[0] - l.from[0]).hypot(d[1] - l.from[1]) > 1e-9
        {
            breaklines.push(Breakline { points: vec![l.from, d], closed: false, source: None });
        }
    }
    let input = SurfaceInput {
        points: Vec::new(),
        breaklines,
        boundary: Some(Ring { points: flat, source: None }),
        voids: Vec::new(),
        tolerance: None,
        edge: Default::default(),
        edges: Vec::new(),
        removed: Vec::new(),
    };
    match surface::build(&input) {
        Ok(design) => {
            let tris = |s: &Surface| -> Vec<Triangle> {
                s.triangles.iter().map(|t| t.map(|i| s.vertices[i as usize])).collect()
            };
            graded.volumes = Some(volume::between_surfaces(&tris(&design), &tris(terrain)));
            graded.design = Some(design);
        }
        Err(e) => graded.why = Some(format!("Şevli platformun yüzeyi kurulamıyor: {e}")),
    }
    Ok(graded)
}

/// A balance: the offset found, the net (cut − fill, m³) there, the iterations, whether it converged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Balance {
    pub offset: f64,
    pub net: f64,
    pub iterations: u32,
    pub converged: bool,
}

/// Why there is no balance.
#[derive(Clone, Debug, PartialEq)]
pub enum BalanceError {
    Grading(GradingError),
    /// At an end of the bracket the pad does not grade whole (an open line, a design that does not build).
    NotGraded { offset: f64, why: String },
    /// The net keeps its sign over the bracket: more cut (or fill) at both ends.
    NoSignChange { low: f64, high: f64 },
    /// The bracket is empty or not finite, or the tolerance not positive.
    BadBracket,
}

fn net_at(p: &Platform, terrain: &Surface, offset: f64) -> Result<f64, BalanceError> {
    let g = grade(p, terrain, offset).map_err(BalanceError::Grading)?;
    match g.volumes {
        Some(v) => Ok(v.cut - v.fill),
        None => Err(BalanceError::NotGraded { offset, why: g.why.unwrap_or_default() }),
    }
}

/// The offset in [`low`, `high`] where cut and fill meet within `tolerance` m³, by bisection; at most 60 steps.
pub fn balance(p: &Platform, terrain: &Surface, low: f64, high: f64, tolerance: f64) -> Result<Balance, BalanceError> {
    if !(low.is_finite() && high.is_finite() && low < high && tolerance > 0.0) {
        return Err(BalanceError::BadBracket);
    }
    let (mut lo, mut hi) = (low, high);
    let (mut flo, fhi) = (net_at(p, terrain, lo)?, net_at(p, terrain, hi)?);
    if flo == 0.0 {
        return Ok(Balance { offset: lo, net: 0.0, iterations: 0, converged: true });
    }
    if fhi == 0.0 {
        return Ok(Balance { offset: hi, net: 0.0, iterations: 0, converged: true });
    }
    if (flo > 0.0) == (fhi > 0.0) {
        return Err(BalanceError::NoSignChange { low: flo, high: fhi });
    }
    let mut best = (lo, flo);
    for k in 1..=60 {
        let mid = 0.5 * (lo + hi);
        let f = net_at(p, terrain, mid)?;
        if f.abs() < best.1.abs() {
            best = (mid, f);
        }
        if f.abs() <= tolerance {
            return Ok(Balance { offset: mid, net: f, iterations: k, converged: true });
        }
        if (f > 0.0) == (flo > 0.0) {
            lo = mid;
            flo = f;
        } else {
            hi = mid;
        }
    }
    Ok(Balance { offset: best.0, net: best.1, iterations: 60, converged: false })
}
