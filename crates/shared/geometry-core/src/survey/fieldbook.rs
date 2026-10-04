//! The field book (karne, docs/adr/0169 §2–§3): the stations an instrument
//! was set up on, each with its observations as the instrument wrote them,
//! and their reduction as a Turkish field book is reduced: the two faces of
//! a target paired and averaged (the horizontal reading's difference, the
//! vertical index error), the slope distance turned into the horizontal
//! distance and the height difference with the earth's curvature and
//! refraction (k), each pair checked against the project's tolerances. The
//! independent reference is `scripts/fixtures/field_reduce_cases.py`
//! (mpmath, 50 digits).

use super::Unit;
use crate::api::Op;
use crate::jsmath::{cos, sin};
use crate::op;

/// The earth's mean radius for the curvature term (m).
pub const EARTH_RADIUS: f64 = 6_371_000.0;

/// The earth's curvature and refraction for a horizontal distance with the
/// refraction coefficient k: (1 − k)·D²/2R (docs/adr/0169 §3).
pub fn curvature(horizontal: f64, k: f64) -> f64 {
    (1.0 - k) * horizontal * horizontal / (2.0 * EARTH_RADIUS)
}

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

/// The project's tolerances of a pair (docs/adr/0169 §3), radians and
/// metres; an absent one is not checked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tolerances {
    pub face_hz: Option<f64>,
    pub index: Option<f64>,
    pub face_slope: Option<f64>,
}

crate::json_struct!(Tolerances {
    face_hz => "faceHz",
    index,
    face_slope => "faceSlope"
});

/// A target reduced: one face or two (the observations it came from), the
/// reading and zenith in face I, the slope distance, the faces' differences
/// (the reading's, the index error, the distance's), the target height, the
/// horizontal distance and the height difference station mark → target mark,
/// and the tolerances its differences are above (`faceHz`, `index`, `faceSlope`).
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
    pub over: Vec<&'static str>,
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
    dh,
    over
});

/// An observation that is no observation: its place and why (a zenith of
/// 0, half or a full turn, or out of a turn).
#[derive(Clone, Debug, PartialEq)]
pub struct Unread {
    pub observation: usize,
    pub problem: &'static str,
}

crate::json_struct!(out Unread { observation, problem });

/// A station's reduction: its targets in order, the observations left out,
/// and each observation's face (1, 2, 0 for a direction only, none for no
/// observation).
#[derive(Clone, Debug, PartialEq)]
pub struct Reduction {
    pub rows: Vec<Reduced>,
    pub problems: Vec<Unread>,
    pub faces: Vec<Option<usize>>,
}

crate::json_struct!(out Reduction {
    rows,
    problems,
    faces
});

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
/// coefficient `k`, each pair checked against `tolerances` (docs/adr/0169 §3).
pub fn reduce(station: &Station, unit: Unit, k: f64, tolerances: &Tolerances) -> Reduction {
    let full = unit.full();
    let half = full / 2.0;
    let mut problems = Vec::new();
    let mut faced = Vec::new();
    let mut faces = Vec::with_capacity(station.observations.len());
    for (i, o) in station.observations.iter().enumerate() {
        let f = face(o, full);
        faces.push(f);
        match f {
            Some(f) => faced.push((i, f)),
            None => problems.push(Unread {
                observation: i,
                problem: "zenith",
            }),
        }
    }
    // The angle tolerances in the book's unit.
    let angle = |t: Option<f64>| t.map(|t| unit.of(t));
    let (tol_hz, tol_index) = (angle(tolerances.face_hz), angle(tolerances.index));
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
                let checks = [
                    ("faceHz", Some(d), tol_hz),
                    ("index", Some(index), tol_index),
                    ("faceSlope", slope_diff, tolerances.face_slope),
                ];
                let over = checks
                    .into_iter()
                    .filter(|(_, v, t)| matches!((v, t), (Some(v), Some(t)) if v.abs() > *t))
                    .map(|(key, _, _)| key)
                    .collect();
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
                    over,
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
                over: Vec::new(),
            },
        };
        if let (Some(s), Some(z)) = (row.slope, row.zenith) {
            let z = unit.rad(z);
            let hd = s * sin(z);
            let ih = station.instrument_height.unwrap_or(0.0);
            let th = row.target_height.unwrap_or(0.0);
            row.horizontal = Some(hd);
            row.dh = Some(s * cos(z) + curvature(hd, k) + ih - th);
        }
        rows.push(row);
    }
    Reduction {
        rows,
        problems,
        faces,
    }
}

/// A station reduced in the named unit (`"grad"` or `"deg"`), its pairs
/// checked against the tolerances (none: none checked).
fn reduce_named(
    station: &Station,
    unit: &str,
    k: f64,
    tolerances: Option<Tolerances>,
) -> Result<Reduction, String> {
    Ok(reduce(
        station,
        Unit::parse(unit)?,
        k,
        &tolerances.unwrap_or_default(),
    ))
}

pub(crate) static OPS: &[Op] = &[op!(
    "fieldReduce",
    |station: Station, unit: String, k: f64, tolerances: Option<Tolerances>| {
        reduce_named(&station, &unit, k, tolerances)
    }
)];
