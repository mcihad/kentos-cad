//! A vertical alignment (docs/adr/0902 §2–§4): PVIs (station along the alignment, elevation) joined by grades, and on
//! each PVI between them a curve: a symmetric parabola of horizontal length L, an asymmetric one of L1 before and L2
//! after the PVI, or a true circle of radius R in the station–elevation plane. A parabola is z = z0 + g1·t + r·t²/2,
//! r = (g2 − g1)/L. The asymmetric one is two parabolas meeting at the PVI's station with one elevation and grade:
//! r1 = (g2 − g1)·L2/(L1·(L1 + L2)), r2 = r1·L1²/L2², the grade there (L1·g1 + L2·g2)/(L1 + L2). A circle's tangent length
//! along each grade is R·tan(|θ2 − θ1|/2), θ = atan g, its centre R from the first tangent's end on its concave side.
//! Grades are ratios, lengths horizontal metres; K = L/|A| with A = 100·(g2 − g1) in percent.

/// A PVI's curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VCurve {
    None,
    Parabola { length: f64 },
    Asymmetric { length_in: f64, length_out: f64 },
    Circle { radius: f64 },
}

/// A PVI as given.
#[derive(Clone, Debug, PartialEq)]
pub struct Pvi {
    pub id: String,
    pub station: f64,
    pub elevation: f64,
    pub curve: VCurve,
}

/// A piece of the profile between two stations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Piece {
    /// z = z0 + g·(s − s0).
    Grade { s0: f64, s1: f64, z0: f64, g: f64 },
    /// z = z0 + g0·t + r·t²/2, t = s − s0.
    Parabola { s0: f64, s1: f64, z0: f64, g0: f64, r: f64 },
    /// On the circle of centre (sc, zc) and radius R; `sag`: the centre above.
    Circle { s0: f64, s1: f64, sc: f64, zc: f64, radius: f64, sag: bool },
}

impl Piece {
    pub fn range(&self) -> (f64, f64) {
        match *self {
            Piece::Grade { s0, s1, .. } | Piece::Parabola { s0, s1, .. } | Piece::Circle { s0, s1, .. } => (s0, s1),
        }
    }

    /// Elevation, grade and the second derivative at a station.
    pub fn at(&self, s: f64) -> (f64, f64, f64) {
        match *self {
            Piece::Grade { s0, z0, g, .. } => (z0 + g * (s - s0), g, 0.0),
            Piece::Parabola { s0, z0, g0, r, .. } => {
                let t = s - s0;
                (z0 + g0 * t + 0.5 * r * t * t, g0 + r * t, r)
            }
            Piece::Circle { sc, zc, radius, sag, .. } => {
                let d = s - sc;
                let h = (radius * radius - d * d).max(0.0).sqrt();
                let sign = if sag { -1.0 } else { 1.0 };
                // z = zc ∓ h; dz/ds = ±d/h; d²z/ds² = ±R²/h³.
                let z = zc + sign * h;
                let g = -sign * d / h;
                let c = -sign * radius * radius / (h * h * h);
                (z, g, c)
            }
        }
    }
}

/// What a curve on a PVI came to.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveReport {
    pub pvi: String,
    pub curve: VCurve,
    pub g_in: f64,
    pub g_out: f64,
    /// Its start and end: station and elevation.
    pub start: [f64; 2],
    pub end: [f64; 2],
    /// A = 100·(g_out − g_in), percent.
    pub a: f64,
    /// L/|A|, metres per percent; none for a curve whose grades do not change.
    pub k: Option<f64>,
    /// Its least radius as the standards that write a radius take it (docs/adr/0961; RAA's Kuppen- and
    /// Wannenhalbmesser): a parabola's 1/|z''| (L/|g_out − g_in| = 100·K when symmetric, the sharper half's when not), a
    /// circle's own; none for a curve whose grades do not change.
    pub h: Option<f64>,
    /// A crest (the grade falls) or a sag.
    pub crest: bool,
    /// Its highest (crest) or lowest (sag) point, when it falls inside the curve.
    pub extreme: Option<[f64; 2]>,
}

/// A vertical alignment: its pieces in station order, its curves.
#[derive(Clone, Debug, PartialEq)]
pub struct Vertical {
    pub pieces: Vec<Piece>,
    pub curves: Vec<CurveReport>,
    pub start: f64,
    pub end: f64,
}

/// Why PVIs make no vertical alignment.
#[derive(Clone, Debug, PartialEq)]
pub enum VerticalError {
    TooFewPoints,
    DuplicateId { pvi: String },
    NotFinite { pvi: String },
    /// A PVI not after the one before it.
    NotIncreasing { pvi: String },
    /// A curve on the first or the last PVI, a length or radius not above 0, a curve where the grade does not change.
    BadCurve { pvi: String },
    /// Two curves (or a curve and an end) overlap: by how much along the station.
    Overlap { a: String, b: String, excess: f64 },
}

/// A curve's reach (start and end stations), its pieces and its report.
type Span = (f64, f64, Vec<Piece>, CurveReport);

/// Builds the vertical alignment through `pvis`.
pub fn build(pvis: &[Pvi]) -> Result<Vertical, VerticalError> {
    if pvis.len() < 2 {
        return Err(VerticalError::TooFewPoints);
    }
    let mut seen = std::collections::BTreeSet::new();
    for (i, p) in pvis.iter().enumerate() {
        if p.id.trim().is_empty() || !seen.insert(p.id.as_str()) {
            return Err(VerticalError::DuplicateId { pvi: p.id.clone() });
        }
        let numbers: Vec<f64> = match p.curve {
            VCurve::None => vec![],
            VCurve::Parabola { length } => vec![length],
            VCurve::Asymmetric { length_in, length_out } => vec![length_in, length_out],
            VCurve::Circle { radius } => vec![radius],
        };
        if !(p.station.is_finite() && p.elevation.is_finite() && numbers.iter().all(|v| v.is_finite())) {
            return Err(VerticalError::NotFinite { pvi: p.id.clone() });
        }
        if i > 0 && p.station <= pvis[i - 1].station {
            return Err(VerticalError::NotIncreasing { pvi: p.id.clone() });
        }
        let end = i == 0 || i + 1 == pvis.len();
        let bad = match p.curve {
            VCurve::None => false,
            _ if end => true,
            VCurve::Parabola { length } => length <= 0.0,
            VCurve::Asymmetric { length_in, length_out } => length_in <= 0.0 || length_out <= 0.0,
            VCurve::Circle { radius } => radius <= 0.0,
        };
        if bad {
            return Err(VerticalError::BadCurve { pvi: p.id.clone() });
        }
    }
    let grade = |i: usize| (pvis[i + 1].elevation - pvis[i].elevation) / (pvis[i + 1].station - pvis[i].station);
    // Each PVI's curve: its pieces and its reach (start, end stations).
    let mut spans: Vec<Option<Span>> = vec![None; pvis.len()];
    for i in 1..pvis.len() - 1 {
        let p = &pvis[i];
        if p.curve == VCurve::None {
            continue;
        }
        let (g1, g2) = (grade(i - 1), grade(i));
        if g1 == g2 {
            return Err(VerticalError::BadCurve { pvi: p.id.clone() });
        }
        let (s, z) = (p.station, p.elevation);
        let (pieces, b, e) = match p.curve {
            VCurve::Parabola { length } => {
                let b = s - length / 2.0;
                (
                    vec![Piece::Parabola { s0: b, s1: s + length / 2.0, z0: z - g1 * length / 2.0, g0: g1, r: (g2 - g1) / length }],
                    b,
                    s + length / 2.0,
                )
            }
            VCurve::Asymmetric { length_in: l1, length_out: l2 } => {
                let r1 = (g2 - g1) * l2 / (l1 * (l1 + l2));
                let r2 = r1 * l1 * l1 / (l2 * l2);
                let zb = z - g1 * l1;
                let zm = zb + g1 * l1 + 0.5 * r1 * l1 * l1;
                let gm = (l1 * g1 + l2 * g2) / (l1 + l2);
                (
                    vec![
                        Piece::Parabola { s0: s - l1, s1: s, z0: zb, g0: g1, r: r1 },
                        Piece::Parabola { s0: s, s1: s + l2, z0: zm, g0: gm, r: r2 },
                    ],
                    s - l1,
                    s + l2,
                )
            }
            VCurve::Circle { radius } => {
                let (t1, t2) = (g1.atan(), g2.atan());
                let sag = t2 > t1;
                let tl = radius * ((t2 - t1).abs() / 2.0).tan();
                let (bs, bz) = (s - tl * t1.cos(), z - tl * t1.sin());
                let es = s + tl * t2.cos();
                let sign = if sag { 1.0 } else { -1.0 };
                let (sc, zc) = (bs - sign * radius * t1.sin(), bz + sign * radius * t1.cos());
                (vec![Piece::Circle { s0: bs, s1: es, sc, zc, radius, sag }], bs, es)
            }
            VCurve::None => continue,
        };
        let start_z = pieces[0].at(b).0;
        let end_z = pieces[pieces.len() - 1].at(e).0;
        let crest = g2 < g1;
        // The highest or lowest point: where the grade is 0, if inside.
        let extreme = pieces.iter().find_map(|pc| {
            let (a, z1) = pc.range();
            let s0 = match *pc {
                Piece::Parabola { s0, g0, r, .. } if r != 0.0 => s0 - g0 / r,
                Piece::Circle { sc, .. } => sc,
                _ => return None,
            };
            (s0 >= a && s0 <= z1).then(|| [s0, pc.at(s0).0])
        });
        let a_pct = 100.0 * (g2 - g1);
        let report = CurveReport {
            pvi: p.id.clone(),
            curve: p.curve,
            g_in: g1,
            g_out: g2,
            start: [b, start_z],
            end: [e, end_z],
            a: a_pct,
            k: (a_pct != 0.0).then(|| (e - b) / a_pct.abs()),
            h: pieces
                .iter()
                .filter_map(|pc| match *pc {
                    Piece::Parabola { r, .. } if r != 0.0 => Some(1.0 / r.abs()),
                    Piece::Circle { radius, .. } => Some(radius),
                    _ => None,
                })
                .reduce(f64::min)
                .filter(|_| a_pct != 0.0),
            crest,
            extreme,
        };
        spans[i] = Some((b, e, pieces, report));
    }
    // Overlaps: each curve's start after the previous reach, the ends inside the profile.
    let first = pvis[0].station;
    let last = pvis[pvis.len() - 1].station;
    let mut reach = (first, pvis[0].id.clone());
    for (i, sp) in spans.iter().enumerate() {
        if let Some((b, e, _, _)) = sp {
            if *b < reach.0 - 1e-9 {
                return Err(VerticalError::Overlap { a: reach.1.clone(), b: pvis[i].id.clone(), excess: reach.0 - b });
            }
            reach = (*e, pvis[i].id.clone());
        }
    }
    if reach.0 > last + 1e-9 {
        return Err(VerticalError::Overlap {
            a: reach.1,
            b: pvis[pvis.len() - 1].id.clone(),
            excess: reach.0 - last,
        });
    }
    // The pieces: grades between the curves.
    let mut pieces = Vec::new();
    let mut curves = Vec::new();
    let mut at = first;
    for i in 0..pvis.len() - 1 {
        // The grade from `at` (on the line of PVIs i → i+1) up to the next curve's start or the next PVI.
        let g = grade(i);
        let z_line = |s: f64| pvis[i].elevation + g * (s - pvis[i].station);
        let next = match &spans[i + 1] {
            Some((b, _, _, _)) => *b,
            None => pvis[i + 1].station,
        };
        if next > at + 1e-12 {
            pieces.push(Piece::Grade { s0: at, s1: next, z0: z_line(at), g });
        }
        if let Some((_, e, ps, report)) = &spans[i + 1] {
            pieces.extend(ps.iter().copied());
            curves.push(report.clone());
            at = *e;
        } else {
            at = pvis[i + 1].station;
        }
    }
    Ok(Vertical {
        pieces,
        curves,
        start: first,
        end: last,
    })
}

impl Vertical {
    /// Elevation, grade and the second derivative at a station; none off the profile.
    pub fn at(&self, s: f64) -> Option<(f64, f64, f64)> {
        if !(self.start..=self.end).contains(&s) {
            return None;
        }
        let i = self.pieces.partition_point(|p| p.range().0 <= s).saturating_sub(1);
        self.pieces.get(i).map(|p| p.at(s))
    }
}
