//! Kutupsal alım and aplikasyon: points surveyed from a known station, and
//! the values to set out known points from one.
//!
//! In a polar survey the instrument on the station is oriented on a known
//! back point: its reading to it, subtracted from the reading to a point,
//! turned onto the bearing station → back, is the bearing to the point.
//! A distance is horizontal, or slope with its zenith angle (0 straight up,
//! a quarter turn level): then its horizontal part places the point and its
//! vertical part, with the instrument and target heights, gives its height.
//! With a refraction coefficient k the height difference has the earth's
//! curvature and refraction too, (1 − k)·D²/2R, as a field book's reduction
//! has (docs/adr/0169 §3); without one, neither.
//!
//! With the project's grid (its system and mean ellipsoidal height,
//! docs/adr/0171 §4) a measured horizontal length is on the ground: it is
//! taken to the grid with its line's scale and height factor, which depend
//! on where the point falls, so in two passes (the point by the length as
//! measured, the length reduced on that line, the point again; once more).
//! A set-out length is then given on the ground too.
//!
//! With the geoid's height above the ellipsoid at the station and at each
//! point (`undulations`; zeros for ellipsoidal heights, docs/adr/2900 §3), a
//! shot with a zenith angle from a station with a height is reduced at its
//! own line's height, the mean of its two ends' ellipsoidal heights (the
//! drawing's heights plus the undulations), not the project's mean.

use super::fieldbook::curvature;
use super::{Unit, bearing, distance, distinct, finite, from_bearing, positive};
use crate::api::Op;
use crate::crs::ground::{Grid, GridFactor, grid_factor};
use crate::jsmath::{cos, sin};
use crate::op;
use crate::vec2::Vec2;

#[derive(Clone, Debug, PartialEq)]
pub struct Shot {
    /// Horizontal direction reading to the point, in the unit.
    pub reading: f64,
    /// Horizontal distance, or slope distance when `zenith` is given.
    pub distance: f64,
    pub zenith: Option<f64>,
    /// Height of the target (reflector) above the point.
    pub target_height: Option<f64>,
}

crate::json_struct!(Shot {
    reading,
    distance,
    zenith,
    target_height => "targetHeight"
});

#[derive(Clone, Debug, PartialEq)]
pub struct PolarInput {
    pub unit: String,
    pub station: Vec2,
    pub back: Vec2,
    /// Horizontal direction reading to the back point.
    pub back_reading: f64,
    /// Height of the station, and of the instrument above it (for the points' heights).
    pub station_z: Option<f64>,
    pub instrument_height: Option<f64>,
    pub shots: Vec<Shot>,
    /// The refraction coefficient k of the heights (the project's); none:
    /// no curvature or refraction.
    pub refraction: Option<f64>,
    /// The project's grid: the lengths are on the ground and are taken to
    /// it (docs/adr/0171 §4); none: they are the grid's already.
    pub grid: Option<Grid>,
    /// The geoid's height above the ellipsoid (N, m) at the station and then
    /// at each shot's point, one more than the shots: the drawing's heights
    /// plus these are ellipsoidal (zeros when they are already; docs/adr/2900
    /// §3). With the grid, the station's height and a shot's zenith angle,
    /// that shot is reduced at its ends' mean ellipsoidal height; otherwise at
    /// the grid's height.
    pub undulations: Option<Vec<f64>>,
}

crate::json_struct!(PolarInput {
    unit,
    station,
    back,
    back_reading => "backReading",
    station_z => "stationZ",
    instrument_height => "instrumentHeight",
    shots,
    refraction,
    grid,
    undulations
});

#[derive(Clone, Debug, PartialEq)]
pub struct PolarPoint {
    pub p: Vec2,
    /// Height, when the shot has a zenith angle and the station a height.
    pub z: Option<f64>,
    /// Bearing station → point, in the unit.
    pub bearing: f64,
    pub horizontal: f64,
    /// Height difference station mark → point mark, when the shot has a zenith angle.
    pub dz: Option<f64>,
    /// With the project's grid: the horizontal length on the grid, and the
    /// line's scale and height factor that took it there, and the ellipsoidal
    /// height it was reduced at (its ends' mean, or the grid's).
    pub grid: Option<f64>,
    pub scale: Option<f64>,
    pub height_factor: Option<f64>,
    pub height: Option<f64>,
}

crate::json_struct!(out PolarPoint {
    p,
    z,
    bearing,
    horizontal,
    dz,
    grid,
    scale,
    height_factor => "heightFactor",
    height
});

/// A ground length from `station` along `t` taken to the grid: the point by
/// the length as measured, the line's factors there, the length reduced and
/// the point again, twice; the grid length and the last factors.
fn to_grid(grid: &Grid, station: Vec2, t: f64, ground: f64) -> Result<(f64, GridFactor), String> {
    let mut length = ground;
    let mut factor = grid_factor(grid, station, from_bearing(station, t, length))?;
    for _ in 0..2 {
        length = ground * factor.combined();
        factor = grid_factor(grid, station, from_bearing(station, t, length))?;
    }
    Ok((ground * factor.combined(), factor))
}

/// The points of a polar survey.
pub fn polar_survey(input: &PolarInput) -> Result<Vec<PolarPoint>, String> {
    let unit = Unit::parse(&input.unit)?;
    distinct(input.station, input.back, "Durulan nokta ile bakılan nokta")?;
    let orient = bearing(input.station, input.back)
        - unit.rad(finite(input.back_reading, "Bakılan noktanın okuması")?);
    let i_h = input.instrument_height.unwrap_or(0.0);
    if let Some(u) = &input.undulations {
        if u.len() != input.shots.len() + 1 {
            return Err(format!(
                "Geoit ayrımı durulan nokta ve her nokta için birer tane olmalı: {} nokta için {} ayrım beklenirken {} verildi.",
                input.shots.len(),
                input.shots.len() + 1,
                u.len()
            ));
        }
        if u.iter().any(|n| !n.is_finite()) {
            return Err("Geoit ayrımı sonlu bir sayı olmalı.".into());
        }
    }
    input
        .shots
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let n = k + 1;
            finite(s.reading, &format!("{n}. noktanın okuması"))?;
            if !(finite(s.distance, &format!("{n}. noktanın uzunluğu"))? > 0.0) {
                return Err(format!("{n}. noktanın uzunluğu sıfırdan büyük olmalı."));
            }
            let t = positive(orient + unit.rad(s.reading));
            let (horizontal, dz) = match s.zenith {
                Some(z) => {
                    let z = unit.rad(finite(z, &format!("{n}. noktanın başucu açısı"))?);
                    let horizontal = s.distance * sin(z);
                    let t_h = s.target_height.unwrap_or(0.0);
                    let dz = match input.refraction {
                        Some(k) => s.distance * cos(z) + curvature(horizontal, k) + i_h - t_h,
                        None => s.distance * cos(z) + i_h - t_h,
                    };
                    (horizontal, Some(dz))
                }
                None => (s.distance, None),
            };
            if !(horizontal > 0.0) {
                return Err(format!("{n}. noktanın başucu açısı yatay uzunluk bırakmıyor (0 ile yarım tur arasında olmalı)."));
            }
            // The line's own height: its ends' ellipsoidal heights' mean (docs/adr/2900 §3).
            let own = match (&input.undulations, input.station_z, dz) {
                (Some(u), Some(zs), Some(d)) => Some(((zs + u[0]) + (zs + d + u[n])) / 2.0),
                _ => None,
            };
            let reduced = match &input.grid {
                Some(g) => {
                    let line = Grid { system: g.system.clone(), height: own.unwrap_or(g.height) };
                    let (l, f) = to_grid(&line, input.station, t, horizontal)?;
                    Some((l, f, line.height))
                }
                None => None,
            };
            let length = reduced.map_or(horizontal, |(l, _, _)| l);
            Ok(PolarPoint {
                p: from_bearing(input.station, t, length),
                z: dz.zip(input.station_z).map(|(d, h)| h + d),
                bearing: unit.of(t),
                horizontal,
                dz,
                grid: reduced.map(|(l, _, _)| l),
                scale: reduced.map(|(_, f, _)| f.scale),
                height_factor: reduced.map(|(_, f, _)| f.height_factor),
                height: reduced.map(|(_, _, h)| h),
            })
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct StakeoutInput {
    pub unit: String,
    pub station: Vec2,
    /// The back point the instrument is oriented on, if any.
    pub back: Option<Vec2>,
    pub targets: Vec<Vec2>,
    /// The project's grid: each length is given on the ground too
    /// (docs/adr/0171 §4).
    pub grid: Option<Grid>,
    /// The station's height and each target's (none: a target without one;
    /// docs/adr/3300), as the drawing holds them.
    pub station_z: Option<f64>,
    pub target_zs: Option<Vec<Option<f64>>>,
    /// The geoid's height above the ellipsoid (N, m) at the station and then
    /// at each target, one more than the targets (zeros when the heights are
    /// ellipsoidal; docs/adr/2900 §3). With the grid, the station's height and
    /// a target's, that line is taken at its ends' mean ellipsoidal height;
    /// otherwise at the grid's height.
    pub undulations: Option<Vec<f64>>,
}

crate::json_struct!(StakeoutInput {
    unit,
    station,
    back,
    targets,
    grid,
    station_z => "stationZ",
    target_zs => "targetZs",
    undulations
});

#[derive(Clone, Debug, PartialEq)]
pub struct Stake {
    /// Bearing station → target and its horizontal distance (the second fundamental task).
    pub bearing: f64,
    pub distance: f64,
    /// Angle to turn clockwise from the back point, in [0, a turn): with the back point read as 0.
    pub angle: Option<f64>,
    /// With the project's grid: the distance on the ground to set out, and
    /// the line's scale and height factor, and the ellipsoidal height it was
    /// taken at (its ends' mean, docs/adr/3300, or the grid's).
    pub ground: Option<f64>,
    pub scale: Option<f64>,
    pub height_factor: Option<f64>,
    pub height: Option<f64>,
}

crate::json_struct!(out Stake {
    bearing,
    distance,
    angle,
    ground,
    scale,
    height_factor => "heightFactor",
    height
});

/// Bearings, distances and turning angles from a station to the targets.
/// With the grid, each line's factors at its own height when the station and
/// the target have heights and the undulations are given (docs/adr/3300: the
/// mean of its ends' ellipsoidal heights, as a polar shot's, docs/adr/2900),
/// else at the grid's height.
pub fn stakeout(input: &StakeoutInput) -> Result<Vec<Stake>, String> {
    let unit = Unit::parse(&input.unit)?;
    let orient = match input.back {
        Some(b) => {
            distinct(input.station, b, "Durulan nokta ile bakılan nokta")?;
            Some(bearing(input.station, b))
        }
        None => None,
    };
    if let Some(zs) = &input.target_zs
        && zs.len() != input.targets.len()
    {
        return Err(format!(
            "Hedeflerin kotları hedeflerle bire bir olmalı: {} hedef için {} kot verildi.",
            input.targets.len(),
            zs.len()
        ));
    }
    if let Some(u) = &input.undulations {
        if u.len() != input.targets.len() + 1 {
            return Err(format!(
                "Geoit ayrımı durulan nokta ve her hedef için birer tane olmalı: {} hedef için {} ayrım beklenirken {} verildi.",
                input.targets.len(),
                input.targets.len() + 1,
                u.len()
            ));
        }
        if u.iter().any(|n| !n.is_finite()) {
            return Err("Geoit ayrımı sonlu bir sayı olmalı.".into());
        }
    }
    input
        .targets
        .iter()
        .enumerate()
        .map(|(k, &p)| {
            let t = bearing(input.station, p);
            let d = distance(input.station, p);
            // The line's own height: its ends' ellipsoidal heights' mean (docs/adr/3300).
            let target_z = input.target_zs.as_ref().and_then(|zs| zs[k]);
            let own = match (&input.undulations, input.station_z, target_z) {
                (Some(u), Some(zs), Some(z)) => Some(((zs + u[0]) + (z + u[k + 1])) / 2.0),
                _ => None,
            };
            let factor = match &input.grid {
                Some(g) if d > 0.0 => {
                    let line = Grid { system: g.system.clone(), height: own.unwrap_or(g.height) };
                    Some((grid_factor(&line, input.station, p)?, line.height))
                }
                _ => None,
            };
            Ok(Stake {
                bearing: unit.of(t),
                distance: d,
                angle: orient.map(|o| unit.of(positive(t - o))),
                ground: factor.as_ref().map(|(f, _)| d / f.combined()),
                scale: factor.as_ref().map(|(f, _)| f.scale),
                height_factor: factor.as_ref().map(|(f, _)| f.height_factor),
                height: factor.as_ref().map(|(_, h)| *h),
            })
        })
        .collect()
}

pub(crate) const POLAR_OP: Op = op!("surveyPolar", |input: PolarInput| polar_survey(&input));
pub(crate) const STAKEOUT_OP: Op = op!("surveyStakeout", |input: StakeoutInput| stakeout(&input));
