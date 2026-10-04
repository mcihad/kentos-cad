//! Points between the registry's coordinate systems (docs/adr/0167 §3): a
//! grid to latitude and longitude (`geodesy`'s transverse Mercator, the
//! spherical Pseudo-Mercator), the datum shift through Earth-centred
//! coordinates with EPSG's seven-parameter Helmert transformations (the
//! ones PROJ picks by default for Türkiye), and back to the target's grid.
//! Each answer carries its accuracy and the EPSG operations it rests on.
//! Latitudes and longitudes are written and read in degrees, minutes and
//! seconds or in decimal degrees (§1). The reference is PROJ itself
//! (`scripts/fixtures/crs_transform_cases.py`); `libm` keeps native and WASM
//! bit for bit equal. Heights are 0 on both sides: 2D, as the ADR says.

pub mod measure;

use crate::api::Op;
use crate::api::json::{FromJson, Json, ToJson};
use crate::display::fixed;
use crate::geodesy::{Tm, tm_forward, tm_inverse};
use crate::jsmath::PI;
use crate::op;
use crate::vec2::Vec2;

/// A datum of the registry, with its ellipsoid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Datum {
    /// TUREF (ITRF96), GRS80.
    Turef,
    /// ED50, International 1924.
    Ed50,
    /// WGS 84, its own ellipsoid.
    Wgs84,
}

impl Datum {
    /// The ellipsoid: semi-major axis (m) and inverse flattening.
    pub fn ellipsoid(self) -> (f64, f64) {
        match self {
            Datum::Turef => (6_378_137.0, 298.257_222_101),
            Datum::Ed50 => (6_378_388.0, 297.0),
            Datum::Wgs84 => (6_378_137.0, 298.257_223_563),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Datum::Turef => "TUREF",
            Datum::Ed50 => "ED50",
            Datum::Wgs84 => "WGS84",
        }
    }
}

impl FromJson for Datum {
    fn from_json(v: &Json) -> Result<Datum, String> {
        match String::from_json(v)?.as_str() {
            "TUREF" => Ok(Datum::Turef),
            "ED50" => Ok(Datum::Ed50),
            "WGS84" => Ok(Datum::Wgs84),
            other => Err(format!("bilinmeyen datum: {other}")),
        }
    }
}

impl ToJson for Datum {
    fn write_json(&self, out: &mut String) {
        self.name().to_owned().write_json(out);
    }
}

/// A coordinate system as the transforms read it: the registry's entry
/// without its names. A point is (east, north) on a grid, (longitude,
/// latitude) in degrees on a geographic system: the core's x and y.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum System {
    Geographic {
        datum: Datum,
    },
    /// A transverse Mercator grid: the TM3 zones, UTM.
    Tm {
        datum: Datum,
        central_meridian: f64,
        scale_factor: f64,
        false_easting: f64,
        false_northing: f64,
    },
    /// WGS 84 / Pseudo-Mercator (EPSG:3857): the WGS 84 latitude and
    /// longitude on a sphere of the ellipsoid's semi-major axis.
    Mercator {},
}

crate::json_tagged!(
    System,
    "kind",
    Geographic => "geographic" { datum },
    Tm => "tm" {
        datum,
        central_meridian => "centralMeridian",
        scale_factor => "scaleFactor",
        false_easting => "falseEasting",
        false_northing => "falseNorthing"
    },
    Mercator => "mercator" {},
);

impl System {
    fn datum(&self) -> Datum {
        match *self {
            System::Geographic { datum } | System::Tm { datum, .. } => datum,
            System::Mercator {} => Datum::Wgs84,
        }
    }

    fn tm(&self) -> Option<Tm> {
        let System::Tm {
            datum,
            central_meridian,
            scale_factor,
            false_easting,
            false_northing,
        } = *self
        else {
            return None;
        };
        let (semi_major, inverse_flattening) = datum.ellipsoid();
        Some(Tm {
            central_meridian,
            scale_factor,
            false_easting,
            false_northing,
            semi_major,
            inverse_flattening,
        })
    }

    /// The point's latitude and longitude (degrees) on the system's datum.
    fn unproject(&self, p: Vec2) -> Option<(f64, f64)> {
        match self {
            System::Geographic { .. } => {
                (p.y.abs() <= 90.0 && p.x.is_finite()).then_some((p.y, p.x))
            }
            System::Tm { .. } => tm_inverse(&self.tm()?, p.x, p.y),
            System::Mercator {} => {
                let r = Datum::Wgs84.ellipsoid().0;
                let lat = (2.0 * libm::atan(libm::exp(p.y / r)) - PI / 2.0) * 180.0 / PI;
                let lon = p.x / r * 180.0 / PI;
                (lat.is_finite() && lon.is_finite()).then_some((lat, lon))
            }
        }
    }

    /// The system's point at a latitude and longitude on its datum.
    fn project(&self, lat: f64, lon: f64) -> Option<Vec2> {
        match self {
            System::Geographic { .. } => Some(Vec2::new(lon, lat)),
            System::Tm { .. } => tm_forward(&self.tm()?, lat, lon),
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
        }
    }
}

/// A seven-parameter Helmert transformation, EPSG's position vector
/// convention (9606): translations in metres, rotations in arc-seconds, the
/// scale difference in ppm.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Helmert {
    t: [f64; 3],
    r: [f64; 3],
    ds: f64,
}

/// ED50 to WGS 84 (30), EPSG:1784, and ED50 to ETRS89 (9), EPSG:1783: the
/// same parameters, 2 m.
const ED50: Helmert = Helmert {
    t: [-84.1, -101.8, -129.7],
    r: [0.0, 0.0, 0.468],
    ds: 1.05,
};

/// TUREF to ETRS89 (1), EPSG:5260: 0.1 m.
const TUREF_ETRS89: Helmert = Helmert {
    t: [0.023, 0.036, -0.068],
    r: [0.00176, 0.00912, -0.01136],
    ds: 0.00439,
};

impl Helmert {
    /// The small-angle rotation (PROJ's, its position vector form) and the scale.
    fn matrix(&self) -> ([[f64; 3]; 3], f64) {
        let sec = PI / (180.0 * 3600.0);
        let (rx, ry, rz) = (self.r[0] * sec, self.r[1] * sec, self.r[2] * sec);
        (
            [[1.0, -rz, ry], [rz, 1.0, -rx], [-ry, rx, 1.0]],
            1.0 + self.ds * 1e-6,
        )
    }

    fn forward(&self, x: [f64; 3]) -> [f64; 3] {
        let (m, s) = self.matrix();
        let mut out = [0.0; 3];
        for i in 0..3 {
            out[i] = self.t[i] + s * (m[i][0] * x[0] + m[i][1] * x[1] + m[i][2] * x[2]);
        }
        out
    }

    /// PROJ's reverse: the translation off, the scale out, the rotation transposed.
    fn reverse(&self, x: [f64; 3]) -> [f64; 3] {
        let (m, s) = self.matrix();
        let d = [
            (x[0] - self.t[0]) / s,
            (x[1] - self.t[1]) / s,
            (x[2] - self.t[2]) / s,
        ];
        let mut out = [0.0; 3];
        for i in 0..3 {
            out[i] = m[0][i] * d[0] + m[1][i] * d[1] + m[2][i] * d[2];
        }
        out
    }
}

/// Earth-centred coordinates of a latitude and longitude (degrees) at height 0.
fn to_geocentric(datum: Datum, lat: f64, lon: f64) -> [f64; 3] {
    let (a, inv_f) = datum.ellipsoid();
    let f = 1.0 / inv_f;
    let e2 = f * (2.0 - f);
    let rad = PI / 180.0;
    let (sp, cp) = (libm::sin(lat * rad), libm::cos(lat * rad));
    let (sl, cl) = (libm::sin(lon * rad), libm::cos(lon * rad));
    let n = a / libm::sqrt(1.0 - e2 * sp * sp);
    [n * cp * cl, n * cp * sl, n * (1.0 - e2) * sp]
}

/// The latitude and longitude (degrees) of Earth-centred coordinates; the
/// height is dropped (2D). Iterated on the latitude from the spheroid's
/// first guess: a handful of steps reach the double's last bit near the
/// surface.
fn from_geocentric(datum: Datum, x: [f64; 3]) -> (f64, f64) {
    let (a, inv_f) = datum.ellipsoid();
    let f = 1.0 / inv_f;
    let e2 = f * (2.0 - f);
    let p = libm::hypot(x[0], x[1]);
    let lon = libm::atan2(x[1], x[0]);
    let mut lat = libm::atan2(x[2], p * (1.0 - e2));
    for _ in 0..10 {
        let s = libm::sin(lat);
        let n = a / libm::sqrt(1.0 - e2 * s * s);
        let next = libm::atan2(x[2] + e2 * n * s, p);
        let done = (next - lat).abs() < 1e-15;
        lat = next;
        if done {
            break;
        }
    }
    let deg = 180.0 / PI;
    (lat * deg, lon * deg)
}

/// A latitude and longitude moved from one datum to another the way PROJ
/// does by default for Türkiye; the accuracy (m) and the EPSG operations.
fn shift(from: Datum, to: Datum, lat: f64, lon: f64) -> ((f64, f64), f64, &'static str) {
    match (from, to) {
        (a, b) if a == b => ((lat, lon), 0.0, ""),
        // TUREF to WGS 84 (1): a null transformation on latitude and longitude.
        (Datum::Turef, Datum::Wgs84) | (Datum::Wgs84, Datum::Turef) => {
            ((lat, lon), 1.0, "EPSG:5261")
        }
        (Datum::Ed50, Datum::Wgs84) => {
            let x = ED50.forward(to_geocentric(Datum::Ed50, lat, lon));
            (from_geocentric(Datum::Wgs84, x), 2.0, "EPSG:1784")
        }
        (Datum::Wgs84, Datum::Ed50) => {
            let x = ED50.reverse(to_geocentric(Datum::Wgs84, lat, lon));
            (from_geocentric(Datum::Ed50, x), 2.0, "EPSG:1784")
        }
        // Through ETRS89: ED50 to ETRS89 (9), then TUREF to ETRS89 (1) reversed.
        (Datum::Ed50, Datum::Turef) => {
            let x = TUREF_ETRS89.reverse(ED50.forward(to_geocentric(Datum::Ed50, lat, lon)));
            (
                from_geocentric(Datum::Turef, x),
                2.1,
                "EPSG:1783 + EPSG:5260",
            )
        }
        (Datum::Turef, Datum::Ed50) => {
            let x = ED50.reverse(TUREF_ETRS89.forward(to_geocentric(Datum::Turef, lat, lon)));
            (
                from_geocentric(Datum::Ed50, x),
                2.1,
                "EPSG:1783 + EPSG:5260",
            )
        }
        _ => unreachable!("every pair of the three datums is above"),
    }
}

/// A point moved to another system: where it falls, how far that can be
/// off (m; 0 where only the projection changed) and what it rests on.
#[derive(Clone, Debug, PartialEq)]
pub struct Transformed {
    pub point: Vec2,
    pub accuracy: f64,
    /// The EPSG operations of the datum shift; empty when the datum stays.
    pub via: String,
}

crate::json_struct!(out Transformed { point, accuracy, via });

/// `p` of the system `from` in the system `to`; none where a projection
/// cannot take or give the point.
pub fn transform(from: &System, to: &System, p: Vec2) -> Option<Transformed> {
    let (lat, lon) = from.unproject(p)?;
    let ((lat, lon), accuracy, via) = shift(from.datum(), to.datum(), lat, lon);
    Some(Transformed {
        point: to.project(lat, lon)?,
        accuracy,
        via: via.to_owned(),
    })
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
        assert_eq!((there.accuracy, there.via.as_str()), (0.0, ""));
        let back = transform(&geo, &TM30, there.point).expect("back");
        assert!(
            (back.point.x - p.x).abs() < 1e-6 && (back.point.y - p.y).abs() < 1e-6,
            "{back:?}"
        );
    }

    const ED50_TM30: System = System::Tm {
        datum: Datum::Ed50,
        central_meridian: 30.0,
        scale_factor: 1.0,
        false_easting: 500_000.0,
        false_northing: 0.0,
    };

    #[test]
    fn the_datum_shift_says_what_it_rests_on() {
        let t = transform(&TM30, &ED50_TM30, Vec2::new(412_345.678, 4_512_345.678)).expect("ED50");
        assert_eq!((t.accuracy, t.via.as_str()), (2.1, "EPSG:1783 + EPSG:5260"));
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
