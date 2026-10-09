//! Points between coordinate systems (docs/adr/0167 §3, docs/adr/0168): a
//! grid to latitude and longitude (`geodesy`'s transverse Mercator, of any
//! origin; the spherical Pseudo-Mercator; a local system's plane transform
//! to its base), the datum shift (`datum`: EPSG's seven-parameter Helmert
//! transformations, the ones PROJ picks by default for Türkiye; a datum of
//! the project's through WGS 84; the project's own choices), and back to the
//! target's grid. Each answer carries its accuracy and what it rests on, or
//! why there is none. Latitudes and longitudes are written and read in
//! degrees, minutes and seconds or in decimal degrees (0167 §1). The
//! references are PROJ itself (`scripts/fixtures/crs_transform_cases.py`,
//! `scripts/fixtures/crs_custom_cases.py`); `libm` keeps native and WASM bit
//! for bit equal. Heights are 0 on both sides: 2D, as the ADRs say.

mod datum;
pub mod ground;
pub mod measure;
pub mod ntv2;
mod plane;
pub mod text;
pub mod wkt;

pub use datum::{Choice, Convention, CustomDatum, Datum, Ellipsoid, Helmert, Method};
pub use plane::Plane;

use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::display::fixed;
use crate::geodesy::{Tm, tm_forward, tm_inverse};
use crate::jsmath::PI;
use crate::op;
use crate::vec2::Vec2;

/// A coordinate system as the transforms read it: the registry's entry
/// without its names, or the project's definition (docs/adr/0168 §1). A
/// point is (east, north) on a grid, (longitude, latitude) in degrees on a
/// geographic system: the core's x and y.
#[derive(Clone, Debug, PartialEq)]
pub enum System {
    Geographic {
        datum: Datum,
    },
    /// A transverse Mercator grid: the TM3 zones, UTM, or the project's own
    /// (its origin's latitude 0 when not given).
    Tm {
        datum: Datum,
        latitude_of_origin: Option<f64>,
        central_meridian: f64,
        scale_factor: f64,
        false_easting: f64,
        false_northing: f64,
    },
    /// WGS 84 / Pseudo-Mercator (EPSG:3857): the WGS 84 latitude and
    /// longitude on a sphere of the ellipsoid's semi-major axis.
    Mercator {},
    /// A local system: its coordinates become its base's by a plane transform.
    Local {
        base: Box<System>,
        plane: Plane,
    },
}

crate::json_tagged!(
    System,
    "kind",
    Geographic => "geographic" { datum },
    Tm => "tm" {
        datum,
        latitude_of_origin => "latitudeOfOrigin",
        central_meridian => "centralMeridian",
        scale_factor => "scaleFactor",
        false_easting => "falseEasting",
        false_northing => "falseNorthing"
    },
    Mercator => "mercator" {},
    Local => "local" { base, plane },
);

impl System {
    fn datum(&self) -> Datum {
        match self {
            System::Geographic { datum } | System::Tm { datum, .. } => datum.clone(),
            System::Mercator {} => Datum::Wgs84,
            System::Local { base, .. } => base.datum(),
        }
    }

    fn tm(&self) -> Option<Tm> {
        let System::Tm {
            datum,
            central_meridian,
            scale_factor,
            false_easting,
            false_northing,
            ..
        } = self
        else {
            return None;
        };
        let (semi_major, inverse_flattening) = datum.ellipsoid();
        Some(Tm {
            central_meridian: *central_meridian,
            scale_factor: *scale_factor,
            false_easting: *false_easting,
            false_northing: *false_northing,
            semi_major,
            inverse_flattening,
        })
    }

    /// How far north of the equator's the origin's latitude puts the grid
    /// (m): the grid's northing there, without the false northing; 0 for the
    /// equator.
    fn origin_northing(&self, tm: &Tm) -> Option<f64> {
        match self {
            System::Tm {
                latitude_of_origin: Some(lat0),
                central_meridian,
                ..
            } if *lat0 != 0.0 => {
                Some(tm_forward(tm, *lat0, *central_meridian)?.y - tm.false_northing)
            }
            _ => Some(0.0),
        }
    }

    /// The point's latitude and longitude (degrees) on the system's own
    /// datum: what a raster's insolation takes its rows' latitudes from
    /// (docs/adr/0231 §8); none outside the system.
    pub fn geographic_of(&self, p: Vec2) -> Option<(f64, f64)> {
        self.unproject(p)
    }

    /// Whether the system's coordinates are degrees (a local system's are its
    /// plane's, whatever its base).
    pub fn is_geographic(&self) -> bool {
        matches!(self, System::Geographic { .. })
    }

    /// The point's latitude and longitude (degrees) on the system's datum.
    fn unproject(&self, p: Vec2) -> Option<(f64, f64)> {
        match self {
            System::Geographic { .. } => {
                (p.y.abs() <= 90.0 && p.x.is_finite()).then_some((p.y, p.x))
            }
            System::Tm { .. } => {
                let tm = self.tm()?;
                let y0 = self.origin_northing(&tm)?;
                tm_inverse(&tm, p.x, p.y + y0)
            }
            System::Mercator {} => {
                let r = Datum::Wgs84.ellipsoid().0;
                let lat = (2.0 * libm::atan(libm::exp(p.y / r)) - PI / 2.0) * 180.0 / PI;
                let lon = p.x / r * 180.0 / PI;
                (lat.is_finite() && lon.is_finite()).then_some((lat, lon))
            }
            System::Local { base, plane } => base.unproject(plane.forward(p)),
        }
    }

    /// The system's point at a latitude and longitude on its datum.
    fn project(&self, lat: f64, lon: f64) -> Option<Vec2> {
        match self {
            System::Geographic { .. } => Some(Vec2::new(lon, lat)),
            System::Tm { .. } => {
                let tm = self.tm()?;
                let y0 = self.origin_northing(&tm)?;
                let p = tm_forward(&tm, lat, lon)?;
                Some(Vec2::new(p.x, p.y - y0))
            }
            System::Mercator {} => {
                if lat.abs() >= 90.0 {
                    return None;
                }
                let r = Datum::Wgs84.ellipsoid().0;
                let rad = PI / 180.0;
                let y = r * libm::log(libm::tan(PI / 4.0 + lat * rad / 2.0));
                let p = Vec2::new(r * lon * rad, y);
                (p.x.is_finite() && p.y.is_finite()).then_some(p)
            }
            System::Local { base, plane } => plane.inverse(base.project(lat, lon)?),
        }
    }
}

/// A point moved to another system: where it falls, how far that can be
/// off (m; 0 where only the projection changed; none when a step's is not
/// known), what it rests on, and whether an EPSG operation of ED50 was used
/// (its values are not the official transformation's, docs/adr/0167 §5).
#[derive(Clone, Debug, PartialEq)]
pub struct Transformed {
    pub point: Vec2,
    pub accuracy: Option<f64>,
    /// The datum shift's operations and the project's names, joined by “ + ”;
    /// empty when the datum stays.
    pub via: String,
    pub unofficial: bool,
}

crate::json_struct!(out Transformed { point, accuracy, via, unofficial });

/// Why a point has no value in another system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unreached {
    /// A projection cannot take or give the point (or a local system's plane
    /// transform folds).
    Outside,
    /// A datum of the project's with no way to WGS 84 stands between them.
    NoLink,
    /// The project's datum choice is a grid this process does not have.
    NoGrid,
    /// The point is outside the project's grid.
    OutsideGrid,
}

impl Unreached {
    /// As the references write it.
    pub fn as_str(self) -> &'static str {
        match self {
            Unreached::Outside => "outside",
            Unreached::NoLink => "noLink",
            Unreached::NoGrid => "noGrid",
            Unreached::OutsideGrid => "outsideGrid",
        }
    }
}

/// `p` of the system `from` in the system `to`, the project's datum choices
/// taken where they apply.
pub fn transform_in(
    from: &System,
    to: &System,
    p: Vec2,
    choices: &[Choice],
) -> Result<Transformed, Unreached> {
    let (a, b) = (from.datum(), to.datum());
    let way = datum::path(&a, &b, choices).map_err(|gap| match gap {
        datum::Gap::NoLink => Unreached::NoLink,
        datum::Gap::NoGrid => Unreached::NoGrid,
    })?;
    let (lat, lon) = from.unproject(p).ok_or(Unreached::Outside)?;
    let (lat, lon) = way
        .run(a.ellipsoid(), lat, lon)
        .ok_or(Unreached::OutsideGrid)?;
    Ok(Transformed {
        point: to.project(lat, lon).ok_or(Unreached::Outside)?,
        accuracy: way.accuracy,
        via: way.via.join(" + "),
        unofficial: way.unofficial,
    })
}

/// `p` of the system `from` in the system `to` by the registry's ways; none
/// where it has no value there.
pub fn transform(from: &System, to: &System, p: Vec2) -> Option<Transformed> {
    transform_in(from, to, p, &[]).ok()
}

/// The op's answer: the moved point, or why there is none.
struct Moved(Result<Transformed, Unreached>);

impl ToJson for Moved {
    fn write_json(&self, out: &mut String) {
        match &self.0 {
            Ok(t) => t.write_json(out),
            Err(why) => {
                out.push('{');
                let mut first = true;
                field(out, &mut first, "error", why.as_str());
                out.push('}');
            }
        }
    }
}

/// A latitude or a longitude in degrees, minutes and seconds: `40°45′12.3456″K`.
/// The seconds take `decimals` places (rounded half away as the display
/// rule rounds, carried into the minutes and degrees); K or G for a
/// latitude, D or B for a longitude.
pub fn format_dms(deg: f64, latitude: bool, decimals: usize) -> String {
    let hemisphere = match (latitude, deg < 0.0) {
        (true, false) => 'K',
        (true, true) => 'G',
        (false, false) => 'D',
        (false, true) => 'B',
    };
    let scale = libm::pow(10.0, decimals as f64);
    // Whole units of the last second's place, so the carry is exact.
    let units = libm::round(deg.abs() * 3600.0 * scale);
    let per_minute = 60.0 * scale;
    let per_degree = 3600.0 * scale;
    let d = libm::floor(units / per_degree);
    let m = libm::floor((units - d * per_degree) / per_minute);
    let s = (units - d * per_degree - m * per_minute) / scale;
    let seconds = fixed(s, decimals);
    // Two figures before the point: 05.1234.
    let seconds = if s < 10.0 {
        format!("0{seconds}")
    } else {
        seconds
    };
    format!("{}°{:02}′{}″{}", d as u64, m as u64, seconds, hemisphere)
}

/// A latitude or a longitude in decimal degrees: `40.7534293°K`.
pub fn format_dd(deg: f64, latitude: bool, decimals: usize) -> String {
    let hemisphere = match (latitude, deg < 0.0) {
        (true, false) => 'K',
        (true, true) => 'G',
        (false, false) => 'D',
        (false, true) => 'B',
    };
    format!("{}°{}", fixed(deg.abs(), decimals), hemisphere)
}

/// A typed latitude or longitude in degrees: decimal degrees (`40.7534293`,
/// `-29.5`), degrees and minutes (`40 45.2`), or degrees, minutes and
/// seconds with spaces or the signs ° ′ ″ (also ' and "): `40 45 12.3456`,
/// `40°45'12.3456"`. A hemisphere letter (K, G, D, B, or N, S, E, W) may
/// close it; G, B, S and W turn it south or west. Minutes and seconds are
/// below 60; none for anything else.
pub fn parse_angle(text: &str) -> Option<f64> {
    let t = text.trim();
    let (body, negative_letter) = match t.chars().last() {
        Some(c) if "KGDBNSEWkgdbnsew".contains(c) => (
            &t[..t.len() - c.len_utf8()],
            matches!(c.to_ascii_uppercase(), 'G' | 'B' | 'S' | 'W'),
        ),
        _ => (t, false),
    };
    let body = body.trim();
    let (body, sign) = match body.strip_prefix('-') {
        Some(rest) => (rest, -1.0),
        None => (body.strip_prefix('+').unwrap_or(body), 1.0),
    };
    let parts: Vec<&str> = body
        .split(|c: char| c.is_whitespace() || "°′″'\"".contains(c))
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    let number = |s: &str| -> Option<f64> {
        let ok = !s.is_empty()
            && s.chars().all(|c| c.is_ascii_digit() || c == '.')
            && s.chars().filter(|&c| c == '.').count() <= 1
            && !s.starts_with('.')
            && !s.ends_with('.');
        if ok { s.parse::<f64>().ok() } else { None }
    };
    let values: Vec<f64> = parts.iter().map(|p| number(p)).collect::<Option<_>>()?;
    // Only the last part may have a fraction; minutes and seconds are below 60.
    if values[..values.len() - 1].iter().any(|v| v.fract() != 0.0) {
        return None;
    }
    if values.iter().skip(1).any(|&v| v >= 60.0) {
        return None;
    }
    let deg = values[0]
        + values.get(1).copied().unwrap_or(0.0) / 60.0
        + values.get(2).copied().unwrap_or(0.0) / 3600.0;
    let sign = if negative_letter { -sign } else { sign };
    Some(sign * deg)
}

pub(crate) static OPS: &[Op] = &[
    op!("crsTransform", |from: System, to: System, p: Vec2| {
        transform(&from, &to, p)
    }),
    op!(
        "crsTransformIn",
        |from: System, to: System, p: Vec2, choices: Option<Vec<Choice>>| {
            Moved(transform_in(
                &from,
                &to,
                p,
                choices.as_deref().unwrap_or_default(),
            ))
        }
    ),
    op!("formatDms", |deg: f64, latitude: bool, decimals: f64| {
        format_dms(deg, latitude, decimals as usize)
    }),
    op!("formatDd", |deg: f64, latitude: bool, decimals: f64| {
        format_dd(deg, latitude, decimals as usize)
    }),
    op!("parseAngle", |text: String| parse_angle(&text)),
];

#[cfg(test)]
mod tests {
    use super::*;

    const TM30: System = System::Tm {
        datum: Datum::Turef,
        latitude_of_origin: None,
        central_meridian: 30.0,
        scale_factor: 1.0,
        false_easting: 500_000.0,
        false_northing: 0.0,
    };

    #[test]
    fn a_grid_point_goes_there_and_back() {
        let p = Vec2::new(412_345.678, 4_512_345.678);
        let geo = System::Geographic {
            datum: Datum::Turef,
        };
        let there = transform(&TM30, &geo, p).expect("geographic");
        assert_eq!((there.accuracy, there.via.as_str()), (Some(0.0), ""));
        let back = transform(&geo, &TM30, there.point).expect("back");
        assert!(
            (back.point.x - p.x).abs() < 1e-6 && (back.point.y - p.y).abs() < 1e-6,
            "{back:?}"
        );
    }

    const ED50_TM30: System = System::Tm {
        datum: Datum::Ed50,
        latitude_of_origin: None,
        central_meridian: 30.0,
        scale_factor: 1.0,
        false_easting: 500_000.0,
        false_northing: 0.0,
    };

    #[test]
    fn the_datum_shift_says_what_it_rests_on() {
        let t = transform(&TM30, &ED50_TM30, Vec2::new(412_345.678, 4_512_345.678)).expect("ED50");
        assert_eq!(
            (t.accuracy, t.via.as_str()),
            (Some(2.1), "EPSG:1783 + EPSG:5260")
        );
        assert!(t.unofficial);
        // PROJ 9.7: EPSG:5254 to EPSG:2320, its default path.
        let proj = Vec2::new(412_379.976_700_991, 4_512_531.675_571_951);
        assert!(
            (t.point.x - proj.x).abs() < 1e-6 && (t.point.y - proj.y).abs() < 1e-6,
            "{t:?}"
        );
    }

    #[test]
    fn degrees_minutes_and_seconds_carry_and_read_back() {
        assert_eq!(format_dms(40.753_429_3, true, 4), "40°45′12.3455″K");
        assert_eq!(format_dms(29.999_999_99, false, 4), "30°00′00.0000″D");
        assert_eq!(format_dms(-0.5, true, 2), "0°30′00.00″G");
        assert_eq!(format_dd(40.753_429_31, true, 7), "40.7534293°K");
        assert_eq!(
            parse_angle("40 45 12.3456"),
            Some(40.0 + 45.0 / 60.0 + 12.3456 / 3600.0)
        );
        assert_eq!(
            parse_angle("40°45'12.3456\"K"),
            parse_angle("40 45 12.3456")
        );
        assert_eq!(parse_angle("29.5 B"), Some(-29.5));
        assert_eq!(parse_angle("40 60"), None);
        assert_eq!(parse_angle("40.5 30"), None);
        assert_eq!(parse_angle(""), None);
    }
}
