//! A road's superelevation and widening (docs/adr/0904, plan/road.md Y6): the cross slope of each side along an
//! alignment as a schedule of break points, built from the curves (a rate, a runoff length and the share of the runoff
//! before the curve, rotating about the centreline) or from rows of stations; and the extra width on a curve's inside.
//! Slopes are percent, measured outward from the axis: falling outward is negative. Left and right are the alignment's.

use crate::alignment::Alignment;

/// The cross slopes of both sides at a station (percent, outward).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crossfall {
    pub left: f64,
    pub right: f64,
}

/// A row of a schedule written by station.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    pub station: f64,
    pub left: f64,
    pub right: f64,
}

/// A curve's superelevation: its PI, the full rate (percent, above 0), the runoff length (level crown to full rate),
/// the share of the runoff before the curve's start (0–1) and the extra width on its inside (0 none).
#[derive(Clone, Debug, PartialEq)]
pub struct CurveRule {
    pub pi: String,
    pub rate: f64,
    pub runoff: f64,
    pub on_tangent: f64,
    pub widening: f64,
}

/// A critical station of a curve's transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// Normal crown: the outside starts to turn.
    NormalCrown,
    /// Level crown: the outside is level.
    LevelCrown,
    /// Reverse crown: the outside slopes as the inside does.
    ReverseCrown,
    /// Full superelevation.
    Full,
}

/// A critical station and the curve it belongs to; `exit` on the way out of the curve.
#[derive(Clone, Debug, PartialEq)]
pub struct Critical {
    pub pi: String,
    pub mark: Mark,
    pub station: f64,
    pub exit: bool,
}

/// A schedule: each side's break points (station, slope) and extra width (station, metres), in station order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Schedule {
    pub left: Vec<[f64; 2]>,
    pub right: Vec<[f64; 2]>,
    pub left_widening: Vec<[f64; 2]>,
    pub right_widening: Vec<[f64; 2]>,
    /// The line the traveled way turns about (docs/adr/0931); the centreline unless set.
    pub pivot: crate::superelevation_physics::Pivot,
}

/// Why a schedule does not build.
#[derive(Clone, Debug, PartialEq)]
pub enum SuperError {
    /// A rule's PI has no curve on the alignment.
    NoCurve { pi: String },
    /// A rate, runoff, share or width out of its range.
    Bad { pi: String },
    /// The rate is below a side's normal slope: the inside would not reach it.
    RateBelowNormal { pi: String },
    /// The full rate does not fit on the curve: the two transitions meet before it is reached.
    TooShort { pi: String },
    /// Two curves' transitions overlap.
    Overlap { a: String, b: String },
    /// A transition runs off the alignment.
    OffAlignment { pi: String },
    /// A rule's PI twice.
    Twice { pi: String },
    /// Rows not in increasing station, or a slope not finite.
    BadRow { row: usize },
}

/// The value of piecewise linear break points at `s`; the end value beyond them; `none` without any.
fn along(points: &[[f64; 2]], s: f64, none: f64) -> f64 {
    let (Some(first), Some(last)) = (points.first(), points.last()) else { return none };
    if s <= first[0] {
        return first[1];
    }
    if s >= last[0] {
        return last[1];
    }
    let i = points.partition_point(|p| p[0] <= s);
    let (a, b) = (points[i - 1], points[i]);
    if b[0] == a[0] { b[1] } else { a[1] + (b[1] - a[1]) * (s - a[0]) / (b[0] - a[0]) }
}

impl Schedule {
    /// The cross slopes at `s`; `normal` where a side has no break point.
    pub fn at(&self, s: f64, normal: Crossfall) -> Crossfall {
        Crossfall { left: along(&self.left, s, normal.left), right: along(&self.right, s, normal.right) }
    }

    /// How much the axis rises over the profile grade at `s` (m) for the traveled way's `widths` (left, right) there:
    /// 0 about the centreline (docs/adr/0931 §1).
    pub fn lift(&self, s: f64, normal: Crossfall, widths: (f64, f64)) -> f64 {
        let c = self.at(s, normal);
        crate::superelevation_physics::axis_lift(self.pivot, (c.left, c.right), (normal.left, normal.right), widths)
    }

    /// The extra widths (left, right) at `s`.
    pub fn widening(&self, s: f64) -> (f64, f64) {
        (along(&self.left_widening, s, 0.0), along(&self.right_widening, s, 0.0))
    }
}

/// A schedule from rows written by station.
pub fn from_rows(rows: &[Row]) -> Result<Schedule, SuperError> {
    for (i, r) in rows.iter().enumerate() {
        let bad = !(r.station.is_finite() && r.left.is_finite() && r.right.is_finite())
            || (i > 0 && r.station <= rows[i - 1].station);
        if bad {
            return Err(SuperError::BadRow { row: i });
        }
    }
    Ok(Schedule {
        left: rows.iter().map(|r| [r.station, r.left]).collect(),
        right: rows.iter().map(|r| [r.station, r.right]).collect(),
        ..Schedule::default()
    })
}

/// One end of a curve's transition: its break points on the outside and the inside, its widening's, its critical
/// stations.
struct End {
    outside: Vec<[f64; 2]>,
    inside: Vec<[f64; 2]>,
    widening: Vec<[f64; 2]>,
    critical: Vec<Critical>,
}

/// A curve's end: `full` the full rate's station, `toward` +1 entering (stations grow toward the curve), −1 leaving.
fn end(rule: &CurveRule, n_out: f64, n_in: f64, full: f64, toward: f64, exit: bool) -> End {
    let (e, l) = (rule.rate, rule.runoff);
    // Where g(s) = e − d·e/L reaches v, d the distance back from the full rate's station.
    let at = |v: f64| full - toward * (e - v) * l / e;
    let (nc, rc) = (at(n_out), at(-n_in));
    let mut outside = vec![[nc, n_out], [full, e]];
    let mut inside = vec![[rc, n_in], [full, -e]];
    let start_w = full - toward * l;
    let mut widening = vec![[start_w, 0.0], [full, rule.widening]];
    if toward < 0.0 {
        outside.reverse();
        inside.reverse();
        widening.reverse();
    }
    let mut critical = vec![Critical { pi: rule.pi.clone(), mark: Mark::NormalCrown, station: nc, exit }];
    if n_out < 0.0 {
        critical.push(Critical { pi: rule.pi.clone(), mark: Mark::LevelCrown, station: at(0.0), exit });
    }
    if n_in < 0.0 && -n_in > n_out {
        critical.push(Critical { pi: rule.pi.clone(), mark: Mark::ReverseCrown, station: rc, exit });
    }
    critical.push(Critical { pi: rule.pi.clone(), mark: Mark::Full, station: full, exit });
    if exit {
        critical.reverse();
    }
    End { outside, inside, widening, critical }
}

/// A schedule from the curves' rules, rotating about the centreline from the `normal` slopes; and its critical
/// stations in order.
pub fn from_curves(
    al: &Alignment,
    normal: Crossfall,
    rules: &[CurveRule],
) -> Result<(Schedule, Vec<Critical>), SuperError> {
    let mut placed: Vec<(f64, f64, &CurveRule, &crate::alignment::CurveReport)> = Vec::new();
    for (i, r) in rules.iter().enumerate() {
        if rules[..i].iter().any(|q| q.pi == r.pi) {
            return Err(SuperError::Twice { pi: r.pi.clone() });
        }
        let Some(c) = al.curves.iter().find(|c| c.pi == r.pi) else {
            return Err(SuperError::NoCurve { pi: r.pi.clone() });
        };
        let ok = r.rate.is_finite()
            && r.rate > 0.0
            && r.runoff.is_finite()
            && r.runoff > 0.0
            && (0.0..=1.0).contains(&r.on_tangent)
            && r.widening.is_finite()
            && r.widening >= 0.0;
        if !ok {
            return Err(SuperError::Bad { pi: r.pi.clone() });
        }
        if r.rate < normal.left.abs() || r.rate < normal.right.abs() {
            return Err(SuperError::RateBelowNormal { pi: r.pi.clone() });
        }
        let lead = (1.0 - r.on_tangent) * r.runoff;
        let (fs_in, fs_out) = (c.stations[0] + lead, c.stations[3] - lead);
        if fs_in > fs_out {
            return Err(SuperError::TooShort { pi: r.pi.clone() });
        }
        placed.push((fs_in, fs_out, r, c));
    }
    placed.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut s = Schedule::default();
    let mut critical = Vec::new();
    let mut last: Option<(f64, &str)> = None;
    for (fs_in, fs_out, r, c) in placed {
        // Turning left (delta > 0) the right side is outside.
        let left_outside = c.delta < 0.0;
        let (n_out, n_in) = if left_outside { (normal.left, normal.right) } else { (normal.right, normal.left) };
        let a = end(r, n_out, n_in, fs_in, 1.0, false);
        let b = end(r, n_out, n_in, fs_out, -1.0, true);
        let first = a.outside[0][0].min(a.inside[0][0]).min(a.widening[0][0]);
        let tail = |v: &Vec<[f64; 2]>| v.last().map_or(f64::NEG_INFINITY, |p| p[0]);
        let end_s = tail(&b.outside).max(tail(&b.inside)).max(tail(&b.widening));
        if first < -1e-9 || end_s > al.length + 1e-9 {
            return Err(SuperError::OffAlignment { pi: r.pi.clone() });
        }
        if let Some((prev, pi)) = last
            && first < prev - 1e-9
        {
            return Err(SuperError::Overlap { a: pi.to_owned(), b: r.pi.clone() });
        }
        last = Some((end_s, r.pi.as_str()));
        let (outside, inside) = if left_outside { (&mut s.left, &mut s.right) } else { (&mut s.right, &mut s.left) };
        outside.extend(a.outside.iter().chain(b.outside.iter()));
        inside.extend(a.inside.iter().chain(b.inside.iter()));
        let wide = if left_outside { &mut s.right_widening } else { &mut s.left_widening };
        if r.widening > 0.0 {
            wide.extend(a.widening.iter().chain(b.widening.iter()));
        }
        critical.extend(a.critical);
        critical.extend(b.critical);
    }
    // A side's points between curves: the ends of one curve and the start of the next lie apart, in order.
    for side in [&mut s.left, &mut s.right] {
        side.sort_by(|a, b| a[0].total_cmp(&b[0]));
    }
    Ok((s, critical))
}
