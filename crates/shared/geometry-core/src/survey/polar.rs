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
    grid
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
    /// line's scale and height factor that took it there.
    pub grid: Option<f64>,
    pub scale: Option<f64>,
    pub height_factor: Option<f64>,
}

crate::json_struct!(out PolarPoint {
    p,
    z,
    bearing,
    horizontal,
    dz,
    grid,
    scale,
    height_factor => "heightFactor"
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
            let reduced = match &input.grid {
                Some(g) => Some(to_grid(g, input.station, t, horizontal)?),
                None => None,
            };
            let length = reduced.map_or(horizontal, |(l, _)| l);
            Ok(PolarPoint {
                p: from_bearing(input.station, t, length),
                z: dz.zip(input.station_z).map(|(d, h)| h + d),
                bearing: unit.of(t),
                horizontal,
                dz,
                grid: reduced.map(|(l, _)| l),
                scale: reduced.map(|(_, f)| f.scale),
                height_factor: reduced.map(|(_, f)| f.height_factor),
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
}

crate::json_struct!(StakeoutInput {
    unit,
    station,
    back,
    targets,
    grid
});

#[derive(Clone, Debug, PartialEq)]
pub struct Stake {
    /// Bearing station → target and its horizontal distance (the second fundamental task).
    pub bearing: f64,
    pub distance: f64,
    /// Angle to turn clockwise from the back point, in [0, a turn): with the back point read as 0.
    pub angle: Option<f64>,
    /// With the project's grid: the distance on the ground to set out, and
    /// the line's scale and height factor.
    pub ground: Option<f64>,
    pub scale: Option<f64>,
    pub height_factor: Option<f64>,
}

crate::json_struct!(out Stake {
    bearing,
    distance,
    angle,
    ground,
    scale,
    height_factor => "heightFactor"
});

/// Bearings, distances and turning angles from a station to the targets.
pub fn stakeout(input: &StakeoutInput) -> Result<Vec<Stake>, String> {
    let unit = Unit::parse(&input.unit)?;
    let orient = match input.back {
        Some(b) => {
            distinct(input.station, b, "Durulan nokta ile bakılan nokta")?;
            Some(bearing(input.station, b))
        }
        None => None,
    };
    input
        .targets
        .iter()
        .map(|&p| {
            let t = bearing(input.station, p);
            let d = distance(input.station, p);
            let factor = match &input.grid {
                Some(g) if d > 0.0 => Some(grid_factor(g, input.station, p)?),
                _ => None,
            };
            Ok(Stake {
                bearing: unit.of(t),
                distance: d,
                angle: orient.map(|o| unit.of(positive(t - o))),
                ground: factor.map(|f| d / f.combined()),
                scale: factor.map(|f| f.scale),
                height_factor: factor.map(|f| f.height_factor),
            })
        })
        .collect()
}

pub(crate) const POLAR_OP: Op = op!("surveyPolar", |input: PolarInput| polar_survey(&input));
pub(crate) const STAKEOUT_OP: Op = op!("surveyStakeout", |input: StakeoutInput| stakeout(&input));
