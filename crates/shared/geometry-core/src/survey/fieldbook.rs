//! The field book (karne, docs/adr/0169 §2–§3): the stations an instrument
//! was set up on, each with its observations as the instrument wrote them,
//! and their reduction as a Turkish field book is reduced: the two faces of
//! a target paired and averaged (the horizontal reading's difference, the
//! vertical index error), the slope distance turned into the horizontal
//! distance and the height difference with the earth's curvature and
//! refraction (k). The independent reference is
//! `scripts/fixtures/field_reduce_cases.py` (mpmath, 50 digits).

use super::Unit;
use crate::api::Op;
use crate::jsmath::{cos, sin};
use crate::op;

/// The earth's mean radius for the curvature term (m).
pub const EARTH_RADIUS: f64 = 6_371_000.0;

/// An observation as the instrument wrote it: the target, the horizontal
/// reading, the zenith angle (0 straight up; none: a direction only), the
/// slope distance, the target (reflector) height, a code, and the line of
/// the file it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct Observation {
    pub target: String,
    pub hz: f64,
    pub zenith: Option<f64>,
    pub slope: Option<f64>,
    pub target_height: Option<f64>,
    pub code: Option<String>,
    pub line: Option<usize>,
}

crate::json_struct!(Observation {
    target,
    hz,
    zenith,
    slope,
    target_height => "targetHeight",
    code,
    line
});

/// A station: its name, the instrument's height above it, its observations in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Station {
    pub station: String,
    pub instrument_height: Option<f64>,
    pub observations: Vec<Observation>,
}

crate::json_struct!(Station {
    station,
    instrument_height => "instrumentHeight",
    observations
});

/// A target reduced: one face or two (the observations it came from), the
/// reading and zenith in face I, the slope distance, the faces' differences
/// (the reading's, the index error, the distance's), the target height, the
/// horizontal distance and the height difference station mark → target mark.
#[derive(Clone, Debug, PartialEq)]
pub struct Reduced {
    pub target: String,
    pub faces: usize,
    pub observations: Vec<usize>,
    pub hz: f64,
    pub zenith: Option<f64>,
    pub slope: Option<f64>,
    pub hz_diff: Option<f64>,
    pub index: Option<f64>,
    pub slope_diff: Option<f64>,
    pub target_height: Option<f64>,
    pub horizontal: Option<f64>,
    pub dh: Option<f64>,
}

crate::json_struct!(out Reduced {
    target,
    faces,
    observations,
    hz,
    zenith,
    slope,
    hz_diff => "hzDiff",
    index,
    slope_diff => "slopeDiff",
    target_height => "targetHeight",
    horizontal,
    dh
});

/// An observation that is no observation: its place and why (a zenith of
/// 0, half or a full turn, or out of a turn).
#[derive(Clone, Debug, PartialEq)]
pub struct Unread {
    pub observation: usize,
    pub problem: &'static str,
}

crate::json_struct!(out Unread { observation, problem });

/// A station's reduction: its targets in order, the observations left out.
#[derive(Clone, Debug, PartialEq)]
pub struct Reduction {
    pub rows: Vec<Reduced>,
    pub problems: Vec<Unread>,
}

crate::json_struct!(out Reduction { rows, problems });

/// A value into (−half, half] of a turn.
fn wrap(v: f64, full: f64) -> f64 {
    let half = full / 2.0;
    let m = v % full;
    if m <= -half {
        m + full
    } else if m > half {
        m - full
    } else {
        m
    }
}

/// A value into [0, full).
fn positive(v: f64, full: f64) -> f64 {
    let m = v % full;
    if m < 0.0 { m + full } else { m }
}

/// The face of an observation: 1, 2, or 0 for a direction without a zenith;
/// none for a zenith that is no face.
fn face(o: &Observation, full: f64) -> Option<usize> {
    let Some(z) = o.zenith else {
        return Some(0);
    };
    let half = full / 2.0;
    if !(z > 0.0 && z < full) || z == half {
        return None;
    }
    Some(if z < half { 1 } else { 2 })
}

/// A station's observations reduced in `unit` with the refraction
/// coefficient `k` (docs/adr/0169 §3).
pub fn reduce(station: &Station, unit: Unit, k: f64) -> Reduction {
    let full = unit.full();
    let half = full / 2.0;
    let mut problems = Vec::new();
    let mut faced = Vec::new();
    for (i, o) in station.observations.iter().enumerate() {
        match face(o, full) {
            Some(f) => faced.push((i, f)),
            None => problems.push(Unread {
                observation: i,
                problem: "zenith",
            }),
        }
    }
    let obs = &station.observations;
    let mut used = vec![false; obs.len()];
    let mut rows = Vec::new();
    for (at, &(i, f)) in faced.iter().enumerate() {
        if used[i] {
            continue;
        }
        used[i] = true;
        let o = &obs[i];
        let mate = (f != 0)
            .then(|| {
                faced[at + 1..]
                    .iter()
                    .find(|&&(j, g)| !used[j] && obs[j].target == o.target && g != 0 && g != f)
                    .map(|&(j, _)| j)
            })
            .flatten();
        let mut row = match mate {
            Some(j) => {
                used[j] = true;
                let (one, two, pair) = if f == 1 {
                    (o, &obs[j], vec![i, j])
                } else {
                    (&obs[j], o, vec![j, i])
                };
                let hz1 = one.hz;
                let hz2 = positive(two.hz - half, full);
                let d = wrap(hz1 - hz2, full);
                let (z1, z2) = (one.zenith.unwrap_or(0.0), two.zenith.unwrap_or(0.0));
                let index = (z1 + z2 - full) / 2.0;
                let (slope, slope_diff) = match (one.slope, two.slope) {
                    (Some(a), Some(b)) => (Some((a + b) / 2.0), Some(a - b)),
                    (a, b) => (a.or(b), None),
                };
                Reduced {
                    target: o.target.clone(),
                    faces: 2,
                    observations: pair,
                    hz: positive(hz1 - d / 2.0, full),
                    zenith: Some(z1 - index),
                    slope,
                    hz_diff: Some(d),
                    index: Some(index),
                    slope_diff,
                    target_height: one.target_height.or(two.target_height),
                    horizontal: None,
                    dh: None,
                }
            }
            None => Reduced {
                target: o.target.clone(),
                faces: 1,
                observations: vec![i],
                hz: if f == 2 {
                    positive(o.hz - half, full)
                } else {
                    o.hz
                },
                zenith: match f {
                    0 => None,
                    1 => o.zenith,
                    _ => o.zenith.map(|z| full - z),
                },
                slope: o.slope,
                hz_diff: None,
                index: None,
                slope_diff: None,
                target_height: o.target_height,
                horizontal: None,
                dh: None,
            },
        };
        if let (Some(s), Some(z)) = (row.slope, row.zenith) {
            let z = unit.rad(z);
            let hd = s * sin(z);
            let ih = station.instrument_height.unwrap_or(0.0);
            let th = row.target_height.unwrap_or(0.0);
            row.horizontal = Some(hd);
            row.dh = Some(s * cos(z) + (1.0 - k) * hd * hd / (2.0 * EARTH_RADIUS) + ih - th);
        }
        rows.push(row);
    }
    Reduction { rows, problems }
}

/// A station reduced in the named unit (`"grad"` or `"deg"`).
fn reduce_named(station: &Station, unit: &str, k: f64) -> Result<Reduction, String> {
    Ok(reduce(station, Unit::parse(unit)?, k))
}

pub(crate) static OPS: &[Op] =
    &[op!("fieldReduce", |station: Station,
                          unit: String,
                          k: f64| {
        reduce_named(&station, &unit, k)
    })];
