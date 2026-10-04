//! Datums and the shifts between them (docs/adr/0167 §3, docs/adr/0168 §2–§3):
//! the registry's three with EPSG's operations (the ones PROJ picks by
//! default for Türkiye), a datum the project defines with its ellipsoid and
//! seven parameters to WGS 84, and the project's own seven parameters for a
//! pair of the registry's datums. A datum of the project's reaches the others
//! through WGS 84, as PROJ's `+towgs84` does. Shifts run as PROJ's pipelines
//! run them: consecutive Helmert steps stay on Earth-centred coordinates, the
//! null TUREF–WGS 84 step keeps latitude and longitude; heights start at 0
//! (2D) and are carried between the steps. The references are
//! `scripts/fixtures/crs_transform_cases.py` and
//! `scripts/fixtures/crs_custom_cases.py` (PROJ).

use std::sync::Arc;

use super::ntv2::{self, Grid};
use crate::api::json::{FromJson, Json, ToJson, read_field};
use crate::jsmath::{PI, js_floor};

/// A datum: one of the registry's, or one the project defines.
#[derive(Clone, Debug, PartialEq)]
pub enum Datum {
    /// TUREF (ITRF96), GRS80.
    Turef,
    /// ED50, International 1924.
    Ed50,
    /// WGS 84, its own ellipsoid.
    Wgs84,
    /// The project's (docs/adr/0168 §2).
    Custom(Box<CustomDatum>),
}

/// An ellipsoid: its name, semi-major axis (m) and inverse flattening.
#[derive(Clone, Debug, PartialEq)]
pub struct Ellipsoid {
    pub name: String,
    pub semi_major: f64,
    pub inverse_flattening: f64,
}

crate::json_struct!(Ellipsoid {
    name,
    semi_major => "semiMajor",
    inverse_flattening => "inverseFlattening"
});

/// A datum the project defines: its ellipsoid and, when it has one, its way
/// to WGS 84. Without one it stands alone: no value reaches another datum.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomDatum {
    pub name: String,
    pub ellipsoid: Ellipsoid,
    pub to_wgs84: Option<Helmert>,
}

crate::json_struct!(CustomDatum {
    name,
    ellipsoid,
    to_wgs84 => "toWgs84"
});

/// Which way a Helmert transformation's rotations turn: EPSG's position
/// vector convention (9606) or its coordinate frame convention (9607); the
/// same transformation's rotations have opposite signs in the two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Convention {
    PositionVector,
    CoordinateFrame,
}

impl FromJson for Convention {
    fn from_json(v: &Json) -> Result<Convention, String> {
        match String::from_json(v)?.as_str() {
            "positionVector" => Ok(Convention::PositionVector),
            "coordinateFrame" => Ok(Convention::CoordinateFrame),
            other => Err(format!("bilinmeyen dönüklük kuralı: {other}")),
        }
    }
}

impl ToJson for Convention {
    fn write_json(&self, out: &mut String) {
        match self {
            Convention::PositionVector => "positionVector",
            Convention::CoordinateFrame => "coordinateFrame",
        }
        .write_json(out);
    }
}

/// A seven-parameter Helmert transformation: translations (m), rotations
/// (arc-seconds) in its convention, the scale difference (ppm), and how far
/// off its answers may be (m) when that is known.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Helmert {
    pub translation: [f64; 3],
    pub rotation: [f64; 3],
    pub scale: f64,
    pub convention: Convention,
    pub accuracy: Option<f64>,
}

crate::json_struct!(Helmert {
    translation,
    rotation,
    scale,
    convention,
    accuracy
});

/// ED50 to WGS 84 (30), EPSG:1784, and ED50 to ETRS89 (9), EPSG:1783: the
/// same parameters, 2 m.
pub(crate) const ED50: Helmert = Helmert {
    translation: [-84.1, -101.8, -129.7],
    rotation: [0.0, 0.0, 0.468],
    scale: 1.05,
    convention: Convention::PositionVector,
    accuracy: Some(2.0),
};

/// TUREF to ETRS89 (1), EPSG:5260: 0.1 m.
const TUREF_ETRS89: Helmert = Helmert {
    translation: [0.023, 0.036, -0.068],
    rotation: [0.00176, 0.00912, -0.01136],
    scale: 0.00439,
    convention: Convention::PositionVector,
    accuracy: Some(0.1),
};

impl Helmert {
    /// The rotations in the position vector convention.
    fn position_vector(&self) -> [f64; 3] {
        match self.convention {
            Convention::PositionVector => self.rotation,
            Convention::CoordinateFrame => self.rotation.map(|r| -r),
        }
    }

    /// The small-angle rotation (PROJ's, its position vector form) and the scale.
    fn matrix(&self) -> ([[f64; 3]; 3], f64) {
        let sec = PI / (180.0 * 3600.0);
        let r = self.position_vector();
        let (rx, ry, rz) = (r[0] * sec, r[1] * sec, r[2] * sec);
        (
            [[1.0, -rz, ry], [rz, 1.0, -rx], [-ry, rx, 1.0]],
            1.0 + self.scale * 1e-6,
        )
    }

    fn forward(&self, x: [f64; 3]) -> [f64; 3] {
        let (m, s) = self.matrix();
        let t = self.translation;
        let mut out = [0.0; 3];
        for i in 0..3 {
            out[i] = t[i] + s * (m[i][0] * x[0] + m[i][1] * x[1] + m[i][2] * x[2]);
        }
        out
    }

    /// PROJ's reverse: the translation off, the scale out, the rotation transposed.
    fn reverse(&self, x: [f64; 3]) -> [f64; 3] {
        let (m, s) = self.matrix();
        let t = self.translation;
        let d = [(x[0] - t[0]) / s, (x[1] - t[1]) / s, (x[2] - t[2]) / s];
        let mut out = [0.0; 3];
        for i in 0..3 {
            out[i] = m[0][i] * d[0] + m[1][i] * d[1] + m[2][i] * d[2];
        }
        out
    }

    /// The same transformation, compared whatever its convention.
    fn same(&self, other: &Helmert) -> bool {
        self.translation == other.translation
            && self.position_vector() == other.position_vector()
            && self.scale == other.scale
    }
}

impl Datum {
    /// The ellipsoid: semi-major axis (m) and inverse flattening.
    pub fn ellipsoid(&self) -> (f64, f64) {
        match self {
            Datum::Turef => (6_378_137.0, 298.257_222_101),
            Datum::Ed50 => (6_378_388.0, 297.0),
            Datum::Wgs84 => (6_378_137.0, 298.257_223_563),
            Datum::Custom(d) => (d.ellipsoid.semi_major, d.ellipsoid.inverse_flattening),
        }
    }

    /// The registry's name; none for the project's.
    fn registry_name(&self) -> Option<&'static str> {
        match self {
            Datum::Turef => Some("TUREF"),
            Datum::Ed50 => Some("ED50"),
            Datum::Wgs84 => Some("WGS84"),
            Datum::Custom(_) => None,
        }
    }

    /// One datum: the registry's by name; the project's by their ellipsoid
    /// and their way to WGS 84 (in either convention), or, both without one,
    /// by their names too.
    pub fn same(&self, other: &Datum) -> bool {
        match (self, other) {
            (Datum::Custom(a), Datum::Custom(b)) => {
                self.ellipsoid() == other.ellipsoid()
                    && match (&a.to_wgs84, &b.to_wgs84) {
                        (Some(x), Some(y)) => x.same(y),
                        (None, None) => a.name == b.name,
                        _ => false,
                    }
            }
            (a, b) => a.registry_name().is_some() && a.registry_name() == b.registry_name(),
        }
    }
}

impl FromJson for Datum {
    fn from_json(v: &Json) -> Result<Datum, String> {
        match v {
            Json::Str(s) => match s.as_str() {
                "TUREF" => Ok(Datum::Turef),
                "ED50" => Ok(Datum::Ed50),
                "WGS84" => Ok(Datum::Wgs84),
                other => Err(format!("bilinmeyen datum: {other}")),
            },
            Json::Obj(_) => CustomDatum::from_json(v).map(|d| Datum::Custom(Box::new(d))),
            _ => Err("datum bir ad ya da tanım olmalı".into()),
        }
    }
}

impl ToJson for Datum {
    fn write_json(&self, out: &mut String) {
        match self {
            Datum::Custom(d) => d.write_json(out),
            d => d.registry_name().unwrap_or_default().write_json(out),
        }
    }
}

/// How the project shifts between two of the registry's datums instead of
/// EPSG's way (docs/adr/0168 §3): seven parameters, or an NTv2 grid kept
/// under `id` (`ntv2::register`) with the accuracy the project gives it.
#[derive(Clone, Debug, PartialEq)]
pub enum Method {
    Helmert(Helmert),
    Grid { id: String, accuracy: Option<f64> },
}

/// A grid choice's JSON: `{"id": …, "accuracy": …}`.
struct GridRef {
    id: String,
    accuracy: Option<f64>,
}

crate::json_struct!(GridRef { id, accuracy });

/// The project's choice for a pair of the registry's datums: from which to
/// which, its name (what values rest on) and its method. The pair's other
/// way is the same choice reversed.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub from: Datum,
    pub to: Datum,
    pub name: String,
    pub method: Method,
}

impl FromJson for Choice {
    fn from_json(v: &Json) -> Result<Choice, String> {
        let from: Datum = read_field(v, "from")?;
        let to: Datum = read_field(v, "to")?;
        if from.registry_name().is_none() || to.registry_name().is_none() || from.same(&to) {
            return Err("datum seçimi kayıttaki iki ayrı datum arasında olur".into());
        }
        let method = match (v.get("helmert"), v.get("grid")) {
            (Json::Null, Json::Null) => return Err("datum seçiminin yöntemi yok".into()),
            (Json::Null, g) => {
                let g = GridRef::from_json(g).map_err(|e| format!("“grid”: {e}"))?;
                Method::Grid {
                    id: g.id,
                    accuracy: g.accuracy,
                }
            }
            (h, _) => {
                Method::Helmert(Helmert::from_json(h).map_err(|e| format!("“helmert”: {e}"))?)
            }
        };
        Ok(Choice {
            from,
            to,
            name: read_field(v, "name")?,
            method,
        })
    }
}

impl ToJson for Choice {
    fn write_json(&self, out: &mut String) {
        use crate::api::json::field;
        out.push('{');
        let mut first = true;
        field(out, &mut first, "from", &self.from);
        field(out, &mut first, "to", &self.to);
        field(out, &mut first, "name", &self.name);
        match &self.method {
            Method::Helmert(h) => field(out, &mut first, "helmert", h),
            Method::Grid { id, accuracy } => field(
                out,
                &mut first,
                "grid",
                &GridRef {
                    id: id.clone(),
                    accuracy: *accuracy,
                },
            ),
        }
        out.push('}');
    }
}

/// Why there is no way between two datums: a datum of the project's with
/// no way to WGS 84 stands between them (docs/adr/0168 §2), or the
/// project's choice is a grid this process does not have (§4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gap {
    NoLink,
    NoGrid,
}

/// One step of a shift.
#[derive(Clone, Debug)]
enum Step {
    /// A Helmert transformation on Earth-centred coordinates, forward or reversed.
    Helmert(Helmert, bool),
    /// TUREF to WGS 84 (1), EPSG:5261: latitude and longitude stay.
    Null,
    /// An NTv2 grid on latitude and longitude, forward or reversed.
    Grid(Arc<Grid>, bool),
}

/// The way from one datum to another: each step with the ellipsoid of the
/// datum it lands on, the summed accuracy (none when a step's is unknown),
/// what the values rest on, and whether an EPSG operation of ED50 was used.
#[derive(Clone, Debug)]
pub(crate) struct Path {
    steps: Vec<(Step, (f64, f64))>,
    pub accuracy: Option<f64>,
    pub via: Vec<String>,
    pub unofficial: bool,
}

impl Path {
    fn same() -> Path {
        Path {
            steps: Vec::new(),
            accuracy: Some(0.0),
            via: Vec::new(),
            unofficial: false,
        }
    }

    /// `other` after this.
    fn then(mut self, other: Path) -> Path {
        self.steps.extend(other.steps);
        self.accuracy = self.accuracy.zip(other.accuracy).map(|(a, b)| a + b);
        self.via.extend(other.via);
        self.unofficial |= other.unofficial;
        self
    }

    /// A latitude and longitude (degrees) of the datum whose ellipsoid is
    /// `from`, on the last step's datum; none where a grid does not reach.
    pub fn run(&self, from: (f64, f64), lat: f64, lon: f64) -> Option<(f64, f64)> {
        enum State {
            Geographic(f64, f64, f64),
            Centred([f64; 3]),
        }
        let mut ellipsoid = from;
        let mut state = State::Geographic(lat, lon, 0.0);
        for (step, lands) in &self.steps {
            state = match (step, state) {
                (Step::Helmert(h, reverse), s) => {
                    let x = match s {
                        State::Geographic(lat, lon, height) => {
                            to_geocentric(ellipsoid, lat, lon, height)
                        }
                        State::Centred(x) => x,
                    };
                    State::Centred(if *reverse { h.reverse(x) } else { h.forward(x) })
                }
                (Step::Null, State::Centred(x)) => {
                    let (lat, lon, height) = from_geocentric(ellipsoid, x);
                    State::Geographic(lat, lon, height)
                }
                (Step::Null, s) => s,
                (Step::Grid(grid, reverse), s) => {
                    let (lat, lon, height) = match s {
                        State::Geographic(lat, lon, height) => (lat, lon, height),
                        State::Centred(x) => from_geocentric(ellipsoid, x),
                    };
                    // PROJ's grids work in radians.
                    let (lam, phi) =
                        grid.apply(lon * (PI / 180.0), lat * (PI / 180.0), *reverse)?;
                    State::Geographic(phi * (180.0 / PI), lam * (180.0 / PI), height)
                }
            };
            ellipsoid = *lands;
        }
        Some(match state {
            State::Geographic(lat, lon, _) => (lat, lon),
            State::Centred(x) => {
                let (lat, lon, _) = from_geocentric(ellipsoid, x);
                (lat, lon)
            }
        })
    }
}

const WGS84: (f64, f64) = (6_378_137.0, 298.257_223_563);
const GRS80: (f64, f64) = (6_378_137.0, 298.257_222_101);

fn helmert_path(h: Helmert, reverse: bool, lands: (f64, f64), via: &str, unofficial: bool) -> Path {
    Path {
        steps: vec![(Step::Helmert(h, reverse), lands)],
        accuracy: h.accuracy,
        via: vec![via.to_owned()],
        unofficial,
    }
}

/// Between two of the registry's datums: the project's choice for the pair,
/// else EPSG's way (docs/adr/0167 §3).
fn registry_path(a: &Datum, b: &Datum, choices: &[Choice]) -> Result<Path, Gap> {
    let lands = b.ellipsoid();
    for c in choices {
        let reverse = if c.from.same(a) && c.to.same(b) {
            false
        } else if c.from.same(b) && c.to.same(a) {
            true
        } else {
            continue;
        };
        return Ok(match &c.method {
            Method::Helmert(h) => helmert_path(*h, reverse, lands, &c.name, false),
            Method::Grid { id, accuracy } => Path {
                steps: vec![(
                    Step::Grid(ntv2::get(id).ok_or(Gap::NoGrid)?, reverse),
                    lands,
                )],
                accuracy: *accuracy,
                via: vec![c.name.clone()],
                unofficial: false,
            },
        });
    }
    Ok(match (a, b) {
        (Datum::Turef, Datum::Wgs84) | (Datum::Wgs84, Datum::Turef) => Path {
            steps: vec![(Step::Null, lands)],
            accuracy: Some(1.0),
            via: vec!["EPSG:5261".into()],
            unofficial: false,
        },
        (Datum::Ed50, Datum::Wgs84) => helmert_path(ED50, false, lands, "EPSG:1784", true),
        (Datum::Wgs84, Datum::Ed50) => helmert_path(ED50, true, lands, "EPSG:1784", true),
        // Through ETRS89: ED50 to ETRS89 (9), then TUREF to ETRS89 (1) reversed.
        (Datum::Ed50, Datum::Turef) => Path {
            steps: vec![
                (Step::Helmert(ED50, false), GRS80),
                (Step::Helmert(TUREF_ETRS89, true), lands),
            ],
            accuracy: Some(2.1),
            via: vec!["EPSG:1783 + EPSG:5260".into()],
            unofficial: true,
        },
        (Datum::Turef, Datum::Ed50) => Path {
            steps: vec![
                (Step::Helmert(TUREF_ETRS89, false), GRS80),
                (Step::Helmert(ED50, true), lands),
            ],
            accuracy: Some(2.1),
            via: vec!["EPSG:1783 + EPSG:5260".into()],
            unofficial: true,
        },
        _ => Path::same(),
    })
}

/// The way from datum `a` to datum `b`: none within a datum; the registry's
/// ways and the project's choices between the registry's datums; through
/// WGS 84 for a datum of the project's.
pub(crate) fn path(a: &Datum, b: &Datum, choices: &[Choice]) -> Result<Path, Gap> {
    if a.same(b) {
        return Ok(Path::same());
    }
    let custom = |d: &Datum| match d {
        Datum::Custom(c) => Some(c.as_ref().clone()),
        _ => None,
    };
    let (ca, cb) = (custom(a), custom(b));
    if ca.is_none() && cb.is_none() {
        return Ok(rounded(registry_path(a, b, choices)?));
    }
    let mut way = Path::same();
    // From a datum of the project's to WGS 84.
    let hub = match &ca {
        Some(d) => {
            let h = d.to_wgs84.ok_or(Gap::NoLink)?;
            way = way.then(helmert_path(h, false, WGS84, &d.name, false));
            Datum::Wgs84
        }
        None => a.clone(),
    };
    match &cb {
        Some(d) => {
            let h = d.to_wgs84.ok_or(Gap::NoLink)?;
            if !hub.same(&Datum::Wgs84) {
                way = way.then(registry_path(&hub, &Datum::Wgs84, choices)?);
            }
            way = way.then(helmert_path(h, true, b.ellipsoid(), &d.name, false));
        }
        None => {
            if !hub.same(b) {
                way = way.then(registry_path(&hub, b, choices)?);
            }
        }
    }
    Ok(rounded(way))
}

/// The summed accuracy to the millimetre (an estimate; 0.1 + 0.2 is 0.3).
fn rounded(mut p: Path) -> Path {
    p.accuracy = p.accuracy.map(|a| js_floor(a * 1000.0 + 0.5) / 1000.0);
    p
}

/// Earth-centred coordinates of a latitude and longitude (degrees) at a
/// height, on an ellipsoid (semi-major axis, inverse flattening).
fn to_geocentric((a, inv_f): (f64, f64), lat: f64, lon: f64, height: f64) -> [f64; 3] {
    let f = 1.0 / inv_f;
    let e2 = f * (2.0 - f);
    let rad = PI / 180.0;
    let (sp, cp) = (libm::sin(lat * rad), libm::cos(lat * rad));
    let (sl, cl) = (libm::sin(lon * rad), libm::cos(lon * rad));
    let n = a / libm::sqrt(1.0 - e2 * sp * sp);
    [
        (n + height) * cp * cl,
        (n + height) * cp * sl,
        (n * (1.0 - e2) + height) * sp,
    ]
}

/// The latitude, longitude (degrees) and height of Earth-centred
/// coordinates. Iterated on the latitude from the spheroid's first guess: a
/// handful of steps reach the double's last bit near the surface.
fn from_geocentric((a, inv_f): (f64, f64), x: [f64; 3]) -> (f64, f64, f64) {
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
    let (s, c) = (libm::sin(lat), libm::cos(lat));
    let n = a / libm::sqrt(1.0 - e2 * s * s);
    // Away from the poles by the radius, near them by the height along the axis.
    let height = if c.abs() > 0.1 {
        p / c - n
    } else {
        x[2] / s - n * (1.0 - e2)
    };
    let deg = 180.0 / PI;
    (lat * deg, lon * deg, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bessel(convention: Convention, rotation: [f64; 3]) -> Datum {
        Datum::Custom(Box::new(CustomDatum {
            name: "Bessel datumu".into(),
            ellipsoid: Ellipsoid {
                name: "Bessel 1841".into(),
                semi_major: 6_377_397.155,
                inverse_flattening: 299.152_812_8,
            },
            to_wgs84: Some(Helmert {
                translation: [598.1, 73.7, 418.2],
                rotation,
                scale: 6.7,
                convention,
                accuracy: Some(1.5),
            }),
        }))
    }

    #[test]
    fn the_two_conventions_are_one_datum() {
        let pv = bessel(Convention::PositionVector, [0.202, 0.045, -2.455]);
        let cf = bessel(Convention::CoordinateFrame, [-0.202, -0.045, 2.455]);
        assert!(pv.same(&cf));
        assert!(!pv.same(&bessel(Convention::CoordinateFrame, [0.202, 0.045, -2.455])));
        assert!(!pv.same(&Datum::Wgs84));
        assert!(Datum::Ed50.same(&Datum::Ed50));
    }

    #[test]
    fn a_datum_of_the_project_goes_through_wgs84() {
        let pv = bessel(Convention::PositionVector, [0.202, 0.045, -2.455]);
        let p = path(&pv, &Datum::Ed50, &[]).expect("a way");
        assert_eq!(p.via, ["Bessel datumu", "EPSG:1784"]);
        assert_eq!((p.accuracy, p.unofficial), (Some(3.5), true));
        let lone = Datum::Custom(Box::new(CustomDatum {
            name: "Bağsız".into(),
            ellipsoid: Ellipsoid {
                name: "Clarke 1880 (RGS)".into(),
                semi_major: 6_378_249.145,
                inverse_flattening: 293.465,
            },
            to_wgs84: None,
        }));
        assert_eq!(path(&lone, &Datum::Turef, &[]).err(), Some(Gap::NoLink));
        assert!(path(&lone, &lone.clone(), &[]).is_ok());
    }

    #[test]
    fn a_height_comes_back() {
        let international = (6_378_388.0, 297.0);
        let x = to_geocentric(international, 39.5, 32.25, 123.456);
        let (lat, lon, h) = from_geocentric(international, x);
        assert!(
            (lat - 39.5).abs() < 1e-12 && (lon - 32.25).abs() < 1e-12,
            "{lat} {lon}"
        );
        assert!((h - 123.456).abs() < 1e-6, "{h}");
    }
}
