//! Coordinate systems read from WKT and PROJ strings and written as them
//! (docs/adr/0168 §5): WKT 1 (OGC and ESRI), WKT 2 and PROJ strings in; WKT
//! 1, PROJ strings and, for a local system, WKT 2 out. The reference is
//! `scripts/fixtures/crs_text_cases.py`: PROJ reads every text there, its
//! numbers and the ADR's rules give what this module must give.
//!
//! - A datum is the registry's (TUREF, ED50, WGS 84) when its name says so,
//!   its ellipsoid is that datum's, and its seven parameters to WGS 84 are
//!   absent or EPSG's; any other is the text's own, named as written (an
//!   ESRI “D_” dropped, underscores as spaces). A PROJ string names no
//!   datum: `+datum=WGS84` is WGS 84, any other its ellipsoid's own.
//! - Only the transverse Mercator (UTM too), latitude and longitude, and a
//!   local system derived by an affine transform from a projected one;
//!   metres and degrees (rotations in arc-seconds, scales in ppm);
//!   Greenwich. Anything else says what stopped it.
//! - Numbers are written in their shortest round-trip form, no exponent.

use super::wkt::{self, Item, Node, key};
use super::{Convention, CustomDatum, Datum, Ellipsoid, Helmert, Plane, System};
use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::op;

/// The degree as WKT writes it (π/180 to 15 places).
const DEGREE: f64 = 0.017_453_292_519_943_3;

/// Why a text gave no system: not a definition, something KentOS does not
/// read (the word it stopped at), a unit other than metres and degrees, a
/// prime meridian other than Greenwich.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    Syntax,
    Unsupported(String),
    Unit(String),
    Meridian(String),
}

impl Refusal {
    pub fn kind(&self) -> &'static str {
        match self {
            Refusal::Syntax => "syntax",
            Refusal::Unsupported(_) => "unsupported",
            Refusal::Unit(_) => "unit",
            Refusal::Meridian(_) => "meridian",
        }
    }

    pub fn detail(&self) -> &str {
        match self {
            Refusal::Syntax => "",
            Refusal::Unsupported(d) | Refusal::Unit(d) | Refusal::Meridian(d) => d,
        }
    }
}

/// A system read from a text: its name, the system, the EPSG code the text
/// names at its root, and the registry's system it is (`exact`) or whose
/// grid it shares on a datum of its own.
#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    pub name: String,
    pub system: System,
    pub authority: Option<u32>,
    pub registry: Option<(u32, bool)>,
}

/// The ellipsoids a PROJ string may name: its code, the core's name, the
/// semi-major axis and the inverse flattening.
const ELLIPSOIDS: [(&str, &str, f64, f64); 7] = [
    ("GRS80", "GRS 1980", 6_378_137.0, 298.257_222_101),
    ("WGS84", "WGS 84", 6_378_137.0, 298.257_223_563),
    ("intl", "International 1924", 6_378_388.0, 297.0),
    ("bessel", "Bessel 1841", 6_377_397.155, 299.152_812_8),
    ("krass", "Krasovski 1940", 6_378_245.0, 298.3),
    ("clrk80", "Clarke 1880 (mod.)", 6_378_249.145, 293.4663),
    ("helmert", "Helmert 1906", 6_378_200.0, 298.3),
];

/// A number as written: the shortest round trip, no exponent, no “.0”.
fn num(x: f64) -> String {
    format!("{x}")
}

/// A WKT name as the core keeps it: ESRI's “D_” dropped, underscores as spaces.
fn shown(name: &str) -> String {
    name.strip_prefix("D_").unwrap_or(name).replace('_', " ")
}

fn ellipsoid_named(a: f64, rf: f64) -> String {
    ELLIPSOIDS
        .iter()
        .find(|e| (e.2, e.3) == (a, rf))
        .map_or_else(
            || format!("a={}, 1/f={}", num(a), num(rf)),
            |e| e.1.to_owned(),
        )
}

fn registry_datum_named(name: &str) -> Option<Datum> {
    let k = key(name);
    let has = |words: &[&str]| words.iter().any(|w| k.contains(w));
    if has(&[
        "turef",
        "turkishnationalreferenceframe",
        "itrf96",
        "itrf1996",
        "internationalterrestrialreferenceframe1996",
    ]) {
        Some(Datum::Turef)
    } else if has(&["european1950", "europeandatum1950", "ed50"]) {
        Some(Datum::Ed50)
    } else if has(&["wgs1984", "wgs84", "worldgeodeticsystem1984"]) {
        Some(Datum::Wgs84)
    } else {
        None
    }
}

/// The registry's datum's seven parameters to WGS 84 as EPSG gives them
/// (position vector; none for WGS 84 itself).
fn epsg_towgs84(d: &Datum) -> Option<[f64; 7]> {
    match d {
        Datum::Turef => Some([0.0; 7]),
        Datum::Ed50 => Some([-84.1, -101.8, -129.7, 0.0, 0.0, 0.468, 1.05]),
        _ => None,
    }
}

fn pv(h: &Helmert) -> [f64; 7] {
    let r = match h.convention {
        Convention::PositionVector => h.rotation,
        Convention::CoordinateFrame => h.rotation.map(|r| -r),
    };
    let t = h.translation;
    [t[0], t[1], t[2], r[0], r[1], r[2], h.scale]
}

/// Seven numbers (or three translations) in the position vector convention.
fn helmert_of(n: &[f64]) -> Option<Helmert> {
    let (translation, rotation, scale) = match n {
        [x, y, z] => ([*x, *y, *z], [0.0; 3], 0.0),
        [x, y, z, rx, ry, rz, s] => ([*x, *y, *z], [*rx, *ry, *rz], *s),
        _ => return None,
    };
    Some(Helmert {
        translation,
        rotation,
        scale,
        convention: Convention::PositionVector,
        accuracy: None,
    })
}

// ── Reading WKT ──────────────────────────────────────────────────────────────────────────────────────────────────

const ROOTS: [&str; 8] = [
    "PROJCS",
    "GEOGCS",
    "PROJCRS",
    "PROJECTEDCRS",
    "GEOGCRS",
    "GEOGRAPHICCRS",
    "BOUNDCRS",
    "DERIVEDPROJCRS",
];

fn is(n: &Node, names: &[&str]) -> bool {
    names.iter().any(|w| n.name.eq_ignore_ascii_case(w))
}

/// The CRS inside a SOURCECRS or TARGETCRS node.
fn inner(n: &Node) -> Option<&Node> {
    n.items.iter().find_map(|i| match i {
        Item::Node(c) => Some(c),
        _ => None,
    })
}

/// The EPSG code the root names as its last item.
fn root_authority(root: &Node) -> Option<u32> {
    let Some(Item::Node(a)) = root.items.last() else {
        return None;
    };
    match (a.name.to_ascii_uppercase().as_str(), a.items.as_slice()) {
        ("AUTHORITY", [Item::Str(org), Item::Str(code), ..])
            if org == "EPSG" && !code.is_empty() && code.bytes().all(|b| b.is_ascii_digit()) =>
        {
            code.parse().ok()
        }
        ("ID", [Item::Str(org), Item::Num(code), ..])
            if org == "EPSG" && code.fract() == 0.0 && (0.0..4e9).contains(code) =>
        {
            Some(*code as u32)
        }
        _ => None,
    }
}

/// What every definition must pass before its parts are read: its keyword,
/// the projection, the prime meridian, the units (the reference's order).
fn first_checks(root: &Node) -> Result<(), Refusal> {
    if !is(root, &ROOTS) {
        return Err(Refusal::Unsupported(root.name.clone()));
    }
    if let Some(p) = root.find(&["PROJECTION"]) {
        let name = p.text().unwrap_or_default();
        if !key(name).contains("transversemercator") {
            return Err(Refusal::Unsupported(name.to_owned()));
        }
    }
    if let Some(pm) = root.find(&["PRIMEM"])
        && pm.number().is_some_and(|v| v != 0.0)
    {
        return Err(Refusal::Meridian(pm.text().unwrap_or_default().to_owned()));
    }
    for n in root.walk() {
        if !n.name.to_ascii_uppercase().ends_with("UNIT") {
            continue;
        }
        if let (Some(name), Some(factor)) = (n.text(), n.number()) {
            let named = matches!(
                name.to_lowercase().as_str(),
                "arc-second" | "parts per million"
            );
            if factor != 1.0 && (factor - DEGREE).abs() > 1e-15 && !named {
                return Err(Refusal::Unit(name.to_owned()));
            }
        }
    }
    Ok(())
}

/// The datum of a geographic node (WKT 1's GEOGCS, WKT 2's GEOGCRS and
/// BASEGEOGCRS): the registry's or the text's own, with the seven
/// parameters a WKT 1 TOWGS84 or a WKT 2 BOUNDCRS gives.
fn datum_of(geog: &Node, towgs84: Option<Helmert>) -> Result<Datum, Refusal> {
    let unsupported = || Refusal::Unsupported(geog.name.clone());
    let d = geog
        .child(&["DATUM", "TRF", "GEODETICDATUM", "ENSEMBLE"])
        .ok_or_else(unsupported)?;
    let e = d
        .child(&["SPHEROID", "ELLIPSOID"])
        .ok_or_else(unsupported)?;
    let (a, rf) = match e.numbers().as_slice() {
        [a, rf, ..] if *rf != 0.0 => (*a, *rf),
        _ => {
            return Err(Refusal::Unsupported(
                e.text().unwrap_or_default().to_owned(),
            ));
        }
    };
    let towgs84 = match (towgs84, d.child(&["TOWGS84"])) {
        (Some(h), _) => Some(h),
        (None, Some(t)) => {
            Some(helmert_of(&t.numbers()).ok_or_else(|| Refusal::Unsupported("TOWGS84".into()))?)
        }
        (None, None) => None,
    };
    let written = d.text().unwrap_or_default();
    if let Some(r) = registry_datum_named(written)
        && r.ellipsoid() == (a, rf)
    {
        match &towgs84 {
            None => return Ok(r),
            Some(h) if epsg_towgs84(&r).unwrap_or([0.0; 7]) == pv(h) => return Ok(r),
            Some(_) => {}
        }
    }
    Ok(Datum::Custom(Box::new(CustomDatum {
        name: shown(written),
        ellipsoid: Ellipsoid {
            name: shown(e.text().unwrap_or_default()),
            semi_major: a,
            inverse_flattening: rf,
        },
        to_wgs84: towgs84,
    })))
}

/// The seven parameters of a WKT 2 abridged transformation.
fn abridged(t: &Node) -> Result<Helmert, Refusal> {
    let method = t
        .child(&["METHOD"])
        .and_then(Node::text)
        .unwrap_or_default();
    let convention =
        if method.contains("Position Vector") || method.contains("Geocentric translations") {
            Convention::PositionVector
        } else if method.contains("Coordinate Frame") {
            Convention::CoordinateFrame
        } else {
            return Err(Refusal::Unsupported(method.to_owned()));
        };
    let value = |name: &str| {
        t.children(&["PARAMETER"])
            .find(|p| p.text() == Some(name))
            .and_then(Node::number)
            .unwrap_or(0.0)
    };
    Ok(Helmert {
        translation: [
            value("X-axis translation"),
            value("Y-axis translation"),
            value("Z-axis translation"),
        ],
        rotation: [
            value("X-axis rotation"),
            value("Y-axis rotation"),
            value("Z-axis rotation"),
        ],
        // PROJ writes and reads the scale difference here as the ratio 1 + ppm·1e-6.
        scale: t
            .children(&["PARAMETER"])
            .find(|p| p.text() == Some("Scale difference"))
            .and_then(Node::number)
            .map_or(0.0, |v| (v - 1.0) * 1e6),
        convention,
        accuracy: None,
    })
}

/// A transverse Mercator from its parameters: origin latitude, central
/// meridian, scale, false easting and northing (EPSG 8801, 8802, 8805,
/// 8806, 8807), missing ones 0 and the scale 1.
fn tm(datum: Datum, p: [Option<f64>; 5]) -> System {
    let lat0 = p[0].unwrap_or(0.0);
    System::Tm {
        datum,
        latitude_of_origin: (lat0 != 0.0).then_some(lat0),
        central_meridian: p[1].unwrap_or(0.0),
        scale_factor: p[2].unwrap_or(1.0),
        false_easting: p[3].unwrap_or(0.0),
        false_northing: p[4].unwrap_or(0.0),
    }
}

/// WKT 1's parameters by name; the last of a repeated one counts.
fn wkt1_parameters(projcs: &Node) -> [Option<f64>; 5] {
    let mut out = [None; 5];
    for p in projcs.children(&["PARAMETER"]) {
        let slot = match key(p.text().unwrap_or_default()).as_str() {
            "latitudeoforigin" => 0,
            "centralmeridian" => 1,
            "scalefactor" => 2,
            "falseeasting" => 3,
            "falsenorthing" => 4,
            _ => continue,
        };
        out[slot] = p.number();
    }
    out
}

/// WKT 2's parameters by their EPSG ids, or by name.
fn wkt2_parameters(conversion: &Node) -> [Option<f64>; 5] {
    let mut out = [None; 5];
    for p in conversion.children(&["PARAMETER"]) {
        let id = p.child(&["ID"]).and_then(|i| match i.items.as_slice() {
            [Item::Str(org), Item::Num(code), ..] if org == "EPSG" => Some(*code),
            _ => None,
        });
        let slot = match (
            id.map(|c| c as u32),
            key(p.text().unwrap_or_default()).as_str(),
        ) {
            (Some(8801), _) | (None, "latitudeofnaturalorigin") => 0,
            (Some(8802), _) | (None, "longitudeofnaturalorigin") => 1,
            (Some(8805), _) | (None, "scalefactoratnaturalorigin") => 2,
            (Some(8806), _) | (None, "falseeasting") => 3,
            (Some(8807), _) | (None, "falsenorthing") => 4,
            _ => continue,
        };
        out[slot] = p.number();
    }
    out
}

/// The plane this → base from WKT 2's deriving conversion base → this
/// (X = A0 + A1·x + A2·y, Y = B0 + B1·x + B2·y), inverted as `Plane`
/// inverts.
fn plane_from_deriving(conv: &Node) -> Plane {
    let v = |name: &str| {
        conv.children(&["PARAMETER"])
            .find(|p| p.text() == Some(name))
            .and_then(Node::number)
            .unwrap_or(0.0)
    };
    let (a, b, c, d, e, f) = (v("A1"), v("A2"), v("A0"), v("B1"), v("B2"), v("B0"));
    let det = a * e - b * d;
    let (ia, ib, id, ie) = (e / det, -b / det, -d / det, a / det);
    Plane::Affine {
        a: ia,
        b: ib,
        c: -(ia * c + ib * f),
        d: id,
        e: ie,
        f: -(id * c + ie * f),
    }
}

fn system_of(n: &Node, towgs84: Option<Helmert>) -> Result<System, Refusal> {
    let unsupported = || Refusal::Unsupported(n.name.clone());
    match n.name.to_ascii_uppercase().as_str() {
        "BOUNDCRS" => {
            let target = n
                .child(&["TARGETCRS"])
                .and_then(inner)
                .ok_or_else(unsupported)?;
            let tdatum = target
                .child(&["DATUM", "TRF", "GEODETICDATUM", "ENSEMBLE"])
                .and_then(Node::text)
                .unwrap_or_default();
            if registry_datum_named(tdatum) != Some(Datum::Wgs84) {
                return Err(Refusal::Unsupported(tdatum.to_owned()));
            }
            let h = abridged(
                n.child(&["ABRIDGEDTRANSFORMATION"])
                    .ok_or_else(unsupported)?,
            )?;
            let source = n
                .child(&["SOURCECRS"])
                .and_then(inner)
                .ok_or_else(unsupported)?;
            system_of(source, Some(h))
        }
        "GEOGCS" | "GEOGCRS" | "GEOGRAPHICCRS" => Ok(System::Geographic {
            datum: datum_of(n, towgs84)?,
        }),
        "PROJCS" => {
            let geog = n.child(&["GEOGCS"]).ok_or_else(unsupported)?;
            Ok(tm(datum_of(geog, towgs84)?, wkt1_parameters(n)))
        }
        "PROJCRS" | "PROJECTEDCRS" | "BASEPROJCRS" => {
            let conversion = n.child(&["CONVERSION"]).ok_or_else(unsupported)?;
            let method = conversion
                .child(&["METHOD"])
                .and_then(Node::text)
                .unwrap_or_default();
            if key(method) != "transversemercator" {
                return Err(Refusal::Unsupported(method.to_owned()));
            }
            let geog = n
                .child(&["BASEGEOGCRS", "BASEGEODCRS"])
                .ok_or_else(unsupported)?;
            Ok(tm(datum_of(geog, towgs84)?, wkt2_parameters(conversion)))
        }
        "DERIVEDPROJCRS" => {
            let base = system_of(n.child(&["BASEPROJCRS"]).ok_or_else(unsupported)?, towgs84)?;
            let conv = n.child(&["DERIVINGCONVERSION"]).ok_or_else(unsupported)?;
            let method = conv
                .child(&["METHOD"])
                .and_then(Node::text)
                .unwrap_or_default();
            if key(method) != "affineparametrictransformation" {
                return Err(Refusal::Unsupported(method.to_owned()));
            }
            Ok(System::Local {
                base: Box::new(base),
                plane: plane_from_deriving(conv),
            })
        }
        _ => Err(unsupported()),
    }
}

/// The first name written in the text (a WKT 2 BOUNDCRS's source's).
fn system_name(root: &Node) -> String {
    root.walk()
        .into_iter()
        .find_map(|n| match n.items.first() {
            Some(Item::Str(s)) => Some(s.replace('_', " ")),
            _ => None,
        })
        .unwrap_or_default()
}

fn read_wkt(t: &str) -> Result<(String, System, Option<u32>), Refusal> {
    let root = wkt::parse(t).ok_or(Refusal::Syntax)?;
    first_checks(&root)?;
    let system = system_of(&root, None)?;
    Ok((system_name(&root), system, root_authority(&root)))
}

// ── Reading PROJ strings ─────────────────────────────────────────────────────────────────────────────────────────

const PROJ_KEYS: [&str; 22] = [
    "proj",
    "lat_0",
    "lon_0",
    "k",
    "k_0",
    "x_0",
    "y_0",
    "zone",
    "south",
    "ellps",
    "a",
    "rf",
    "b",
    "towgs84",
    "datum",
    "units",
    "no_defs",
    "type",
    "wktext",
    "pm",
    "nadgrids",
    "geoidgrids",
];

fn read_proj(t: &str) -> Result<System, Refusal> {
    let mut params: Vec<(&str, &str)> = Vec::new();
    for token in t.split_ascii_whitespace() {
        let body = token.strip_prefix('+').ok_or(Refusal::Syntax)?;
        let (k, v) = body.split_once('=').unwrap_or((body, ""));
        match k {
            "proj"
                if !matches!(
                    v,
                    "tmerc" | "etmerc" | "utm" | "longlat" | "latlong" | "lonlat" | "latlon"
                ) =>
            {
                return Err(Refusal::Unsupported(format!("+proj={v}")));
            }
            "units" if v != "m" => return Err(Refusal::Unit(format!("+units={v}"))),
            "nadgrids" | "geoidgrids" => return Err(Refusal::Unsupported(format!("+{k}"))),
            "pm" if v != "greenwich" && v != "0" => return Err(Refusal::Meridian(v.to_owned())),
            "datum" if v != "WGS84" => return Err(Refusal::Unsupported(format!("+datum={v}"))),
            "ellps" if !ELLIPSOIDS.iter().any(|e| e.0 == v) => {
                return Err(Refusal::Unsupported(format!("+ellps={v}")));
            }
            k if !PROJ_KEYS.contains(&k) => return Err(Refusal::Unsupported(format!("+{k}"))),
            _ => {}
        }
        params.push((k, v));
    }
    let get = |k: &str| params.iter().rev().find(|p| p.0 == k).map(|p| p.1);
    let number = |k: &str| -> Result<Option<f64>, Refusal> {
        match get(k) {
            None => Ok(None),
            Some(v) => v
                .parse::<f64>()
                .ok()
                .filter(|x| x.is_finite())
                .map(Some)
                .ok_or(Refusal::Syntax),
        }
    };
    let datum = if get("datum") == Some("WGS84") {
        Datum::Wgs84
    } else {
        let (a, rf) = match (get("ellps"), number("a")?) {
            (Some(code), _) => ELLIPSOIDS
                .iter()
                .find(|e| e.0 == code)
                .map(|e| (e.2, e.3))
                .ok_or_else(|| Refusal::Unsupported(format!("+ellps={code}")))?,
            (None, Some(a)) => match (number("rf")?, number("b")?) {
                (Some(rf), _) => (a, rf),
                (None, Some(b)) => (a, a / (a - b)),
                _ => return Err(Refusal::Unsupported("+a".into())),
            },
            // PROJ's own default.
            (None, None) => (ELLIPSOIDS[0].2, ELLIPSOIDS[0].3),
        };
        let towgs84 = match get("towgs84") {
            None => None,
            Some(v) => {
                let n: Vec<f64> = v
                    .split(',')
                    .map(|x| x.trim().parse::<f64>())
                    .collect::<Result<_, _>>()
                    .map_err(|_| Refusal::Syntax)?;
                Some(helmert_of(&n).ok_or_else(|| Refusal::Unsupported("+towgs84".into()))?)
            }
        };
        let name = ellipsoid_named(a, rf);
        Datum::Custom(Box::new(CustomDatum {
            name: format!("{name} datumu"),
            ellipsoid: Ellipsoid {
                name,
                semi_major: a,
                inverse_flattening: rf,
            },
            to_wgs84: towgs84,
        }))
    };
    match get("proj").ok_or(Refusal::Syntax)? {
        "utm" => {
            let zone = number("zone")?
                .filter(|z| z.fract() == 0.0 && (1.0..=60.0).contains(z))
                .ok_or_else(|| Refusal::Unsupported("+zone".into()))?;
            let south = params.iter().any(|p| p.0 == "south");
            Ok(tm(
                datum,
                [
                    None,
                    Some(zone * 6.0 - 183.0),
                    Some(0.9996),
                    Some(500_000.0),
                    Some(if south { 10_000_000.0 } else { 0.0 }),
                ],
            ))
        }
        "tmerc" | "etmerc" => Ok(tm(
            datum,
            [
                number("lat_0")?,
                number("lon_0")?,
                number("k")?.or(number("k_0")?),
                number("x_0")?,
                number("y_0")?,
            ],
        )),
        _ => Ok(System::Geographic { datum }),
    }
}

// ── The registry ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The registry's system with this datum and grid, by the zones' rules
/// (Shapefile's reader's): the TUREF and ED50 TM3 zones, ED50 and WGS 84
/// UTM, the TUREF and WGS 84 latitude and longitude.
fn zone_srid(s: &System) -> Option<u32> {
    match s {
        System::Geographic {
            datum: Datum::Wgs84,
        } => Some(4326),
        System::Geographic {
            datum: Datum::Turef,
        } => Some(5252),
        System::Tm {
            datum,
            latitude_of_origin,
            central_meridian: cm,
            scale_factor: k,
            false_easting,
            false_northing,
        } => {
            if latitude_of_origin.unwrap_or(0.0) != 0.0
                || (*false_easting, *false_northing) != (500_000.0, 0.0)
            {
                return None;
            }
            let three = [27.0, 30.0, 33.0, 36.0, 39.0, 42.0, 45.0];
            let six = [27.0, 33.0, 39.0, 45.0];
            let zone = |step: f64| ((cm - 27.0) / step) as u32;
            match datum {
                Datum::Turef if *k == 1.0 && three.contains(cm) => Some(5253 + zone(3.0)),
                Datum::Ed50 if *k == 1.0 && three.contains(cm) => Some(2319 + zone(3.0)),
                Datum::Ed50 if *k == 0.9996 && six.contains(cm) => Some(23035 + zone(6.0)),
                Datum::Wgs84 if *k == 0.9996 && six.contains(cm) => Some(32635 + zone(6.0)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The registry's system this one is (`true`), or whose grid it shares on a
/// datum of its own whose ellipsoid is a registry datum's (`false`).
pub fn registry_match(s: &System) -> Option<(u32, bool)> {
    if let Some(srid) = zone_srid(s) {
        return Some((srid, true));
    }
    let like = |d: &Datum| -> Option<Datum> {
        let Datum::Custom(c) = d else {
            return None;
        };
        let e = (c.ellipsoid.semi_major, c.ellipsoid.inverse_flattening);
        [Datum::Turef, Datum::Ed50, Datum::Wgs84]
            .into_iter()
            .find(|r| r.ellipsoid() == e)
    };
    let alike = match s {
        System::Geographic { datum } => System::Geographic {
            datum: like(datum)?,
        },
        System::Tm { datum, .. } => {
            let mut t = s.clone();
            if let System::Tm { datum: d, .. } = &mut t {
                *d = like(datum)?;
            }
            t
        }
        _ => return None,
    };
    zone_srid(&alike).map(|srid| (srid, false))
}

/// A system from WKT or a PROJ string, or why not.
pub fn read_text(text: &str) -> Result<Read, Refusal> {
    let t = text.trim();
    let (name, system, authority) = if t.starts_with('+') {
        ("PROJ tanımı".to_owned(), read_proj(t)?, None)
    } else {
        read_wkt(t)?
    };
    let registry = registry_match(&system);
    Ok(Read {
        name,
        system,
        authority,
        registry,
    })
}

// ── Writing ──────────────────────────────────────────────────────────────────────────────────────────────────────

fn ellipsoid_numbers(d: &Datum) -> (f64, f64) {
    d.ellipsoid()
}

/// The seven numbers to WGS 84 a text writes (position vector): EPSG's for
/// the registry's datums, the project's for its own; none for WGS 84 and a
/// datum that stands alone.
fn towgs84_numbers(d: &Datum) -> Option<[f64; 7]> {
    match d {
        Datum::Custom(c) => c.to_wgs84.as_ref().map(pv),
        d => epsg_towgs84(d),
    }
}

fn joined(n: &[f64]) -> String {
    n.iter().map(|x| num(*x)).collect::<Vec<_>>().join(",")
}

/// WKT 1's GEOGCS of a datum, named `geog` or as the datum is.
fn geogcs(geog: Option<&str>, d: &Datum) -> String {
    let (a, rf) = ellipsoid_numbers(d);
    let (gname, dname, ename) = match d {
        Datum::Turef => (
            "TUREF".to_owned(),
            "Turkish_National_Reference_Frame".to_owned(),
            "GRS 1980".to_owned(),
        ),
        Datum::Ed50 => (
            "ED50".to_owned(),
            "European_Datum_1950".to_owned(),
            "International 1924".to_owned(),
        ),
        Datum::Wgs84 => (
            "WGS 84".to_owned(),
            "WGS_1984".to_owned(),
            "WGS 84".to_owned(),
        ),
        Datum::Custom(c) => (
            c.name.clone(),
            c.name.replace(' ', "_"),
            c.ellipsoid.name.clone(),
        ),
    };
    let tw = towgs84_numbers(d)
        .map(|n| format!(",TOWGS84[{}]", joined(&n)))
        .unwrap_or_default();
    format!(
        "GEOGCS[\"{}\",DATUM[\"{dname}\",SPHEROID[\"{ename}\",{},{}]{tw}],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]]",
        geog.unwrap_or(&gname),
        num(a),
        num(rf)
    )
}

fn tm_wkt1(name: &str, s: &System) -> Option<String> {
    let System::Tm {
        datum,
        latitude_of_origin,
        central_meridian,
        scale_factor,
        false_easting,
        false_northing,
    } = s
    else {
        return None;
    };
    Some(format!(
        "PROJCS[\"{name}\",{},PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",{}],PARAMETER[\"central_meridian\",{}],PARAMETER[\"scale_factor\",{}],PARAMETER[\"false_easting\",{}],PARAMETER[\"false_northing\",{}],UNIT[\"metre\",1],AXIS[\"Easting\",EAST],AXIS[\"Northing\",NORTH]]",
        geogcs(None, datum),
        num(latitude_of_origin.unwrap_or(0.0)),
        num(*central_meridian),
        num(*scale_factor),
        num(*false_easting),
        num(*false_northing)
    ))
}

fn proj_datum(d: &Datum) -> String {
    if *d == Datum::Wgs84 {
        return "+datum=WGS84".to_owned();
    }
    let (a, rf) = ellipsoid_numbers(d);
    let ell = ELLIPSOIDS
        .iter()
        .find(|e| (e.2, e.3) == (a, rf))
        .map_or_else(
            || format!("+a={} +rf={}", num(a), num(rf)),
            |e| format!("+ellps={}", e.0),
        );
    match towgs84_numbers(d) {
        Some(n) => format!("{ell} +towgs84={}", joined(&n)),
        None => ell,
    }
}

/// A system as a PROJ string; none for a local system and the Pseudo-Mercator.
pub fn write_proj(s: &System) -> Option<String> {
    match s {
        System::Geographic { datum } => Some(format!(
            "+proj=longlat {} +no_defs +type=crs",
            proj_datum(datum)
        )),
        System::Tm {
            datum,
            latitude_of_origin,
            central_meridian,
            scale_factor,
            false_easting,
            false_northing,
        } => Some(format!(
            "+proj=tmerc +lat_0={} +lon_0={} +k={} +x_0={} +y_0={} {} +units=m +no_defs +type=crs",
            num(latitude_of_origin.unwrap_or(0.0)),
            num(*central_meridian),
            num(*scale_factor),
            num(*false_easting),
            num(*false_northing),
            proj_datum(datum)
        )),
        _ => None,
    }
}

const DEG: &str = "ANGLEUNIT[\"degree\",0.0174532925199433]";
const METRE: &str = "LENGTHUNIT[\"metre\",1]";

fn basegeogcrs(d: &Datum, word: &str) -> String {
    let (a, rf) = ellipsoid_numbers(d);
    let (geog, dname, ename) = match d {
        Datum::Turef => (
            "TUREF".to_owned(),
            "Turkish National Reference Frame".to_owned(),
            "GRS 1980".to_owned(),
        ),
        Datum::Ed50 => (
            "ED50".to_owned(),
            "European Datum 1950".to_owned(),
            "International 1924".to_owned(),
        ),
        Datum::Wgs84 => (
            "WGS 84".to_owned(),
            "World Geodetic System 1984".to_owned(),
            "WGS 84".to_owned(),
        ),
        Datum::Custom(c) => (c.name.clone(), c.name.clone(), c.ellipsoid.name.clone()),
    };
    format!(
        "{word}[\"{geog}\",DATUM[\"{dname}\",ELLIPSOID[\"{ename}\",{},{},{METRE}]],PRIMEM[\"Greenwich\",0,{DEG}]]",
        num(a),
        num(rf)
    )
}

/// A local system as WKT 2: DERIVEDPROJCRS over its transverse Mercator
/// base (named `base_name`), the deriving conversion base → this the plane's
/// inverse; in a BOUNDCRS when the base's datum is the project's with a way
/// to WGS 84.
fn write_local(name: &str, base: &System, plane: &Plane, base_name: &str) -> Option<String> {
    let System::Tm {
        datum,
        latitude_of_origin,
        central_meridian,
        scale_factor,
        false_easting,
        false_northing,
    } = base
    else {
        return None;
    };
    let [a, b, c, d, e, f] = plane.coefficients();
    let det = a * e - b * d;
    let (ia, ib, id, ie) = (e / det, -b / det, -d / det, a / det);
    let v = [
        ("A0", -(ia * c + ib * f), 8623),
        ("A1", ia, 8624),
        ("A2", ib, 8625),
        ("B0", -(id * c + ie * f), 8639),
        ("B1", id, 8640),
        ("B2", ie, 8641),
    ];
    let params: Vec<String> = v
        .iter()
        .map(|(k, x, eid)| {
            let unit = if k.ends_with('0') {
                METRE
            } else {
                "SCALEUNIT[\"coefficient\",1]"
            };
            format!("PARAMETER[\"{k}\",{},{unit},ID[\"EPSG\",{eid}]]", num(*x))
        })
        .collect();
    let text = format!(
        "DERIVEDPROJCRS[\"{name}\",BASEPROJCRS[\"{base_name}\",{},CONVERSION[\"{base_name}\",METHOD[\"Transverse Mercator\",ID[\"EPSG\",9807]],PARAMETER[\"Latitude of natural origin\",{},{DEG},ID[\"EPSG\",8801]],PARAMETER[\"Longitude of natural origin\",{},{DEG},ID[\"EPSG\",8802]],PARAMETER[\"Scale factor at natural origin\",{},SCALEUNIT[\"unity\",1],ID[\"EPSG\",8805]],PARAMETER[\"False easting\",{},{METRE},ID[\"EPSG\",8806]],PARAMETER[\"False northing\",{},{METRE},ID[\"EPSG\",8807]]]],DERIVINGCONVERSION[\"{name}\",METHOD[\"Affine parametric transformation\",ID[\"EPSG\",9624]],{}],CS[Cartesian,2],AXIS[\"(E)\",east,ORDER[1],{METRE}],AXIS[\"(N)\",north,ORDER[2],{METRE}]]",
        basegeogcrs(datum, "BASEGEOGCRS"),
        num(latitude_of_origin.unwrap_or(0.0)),
        num(*central_meridian),
        num(*scale_factor),
        num(*false_easting),
        num(*false_northing),
        params.join(",")
    );
    let Datum::Custom(_) = datum else {
        return Some(text);
    };
    let Some(towgs) = towgs84_numbers(datum) else {
        return Some(text);
    };
    let names = [
        ("X-axis translation", 8605),
        ("Y-axis translation", 8606),
        ("Z-axis translation", 8607),
        ("X-axis rotation", 8608),
        ("Y-axis rotation", 8609),
        ("Z-axis rotation", 8610),
        ("Scale difference", 8611),
    ];
    // PROJ writes and reads an abridged transformation's scale difference as the ratio 1 + ppm·1e-6.
    let mut values = towgs;
    values[6] = 1.0 + towgs[6] * 1e-6;
    let tparams: Vec<String> = names
        .iter()
        .zip(values)
        .map(|((n, i), x)| format!("PARAMETER[\"{n}\",{},ID[\"EPSG\",{i}]]", num(x)))
        .collect();
    let wgs84 = basegeogcrs(&Datum::Wgs84, "GEOGCRS");
    let wgs84 = format!(
        "{},CS[ellipsoidal,2],AXIS[\"latitude\",north,ORDER[1],{DEG}],AXIS[\"longitude\",east,ORDER[2],{DEG}]]",
        &wgs84[..wgs84.len() - 1]
    );
    Some(format!(
        "BOUNDCRS[SOURCECRS[{text}],TARGETCRS[{wgs84}],ABRIDGEDTRANSFORMATION[\"{name} to WGS 84\",METHOD[\"Position Vector transformation (geog2D domain)\",ID[\"EPSG\",9606]],{}]]",
        tparams.join(",")
    ))
}

/// A system as WKT: WKT 1 for a transverse Mercator and a geographic one,
/// WKT 2 for a local system (its base named `base_name`); none for the
/// Pseudo-Mercator.
pub fn write_wkt(name: &str, s: &System, base_name: Option<&str>) -> Option<String> {
    match s {
        System::Tm { .. } => tm_wkt1(name, s),
        System::Geographic { datum } => Some(geogcs(Some(name), datum)),
        System::Local { base, plane } => write_local(name, base, plane, base_name?),
        System::Mercator {} => None,
    }
}

// ── The calls ────────────────────────────────────────────────────────────────────────────────────────────────────

/// The op's answer: what was read, or why nothing was.
struct Answer(Result<Read, Refusal>);

impl ToJson for Answer {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok(r) => {
                field(out, &mut first, "name", &r.name);
                field(out, &mut first, "system", &r.system);
                field(out, &mut first, "authority", &r.authority);
                if let Some((srid, exact)) = r.registry {
                    field(out, &mut first, "registry", &Registry { srid, exact });
                }
            }
            Err(why) => field(out, &mut first, "error", &Why(why)),
        }
        out.push('}');
    }
}

struct Registry {
    srid: u32,
    exact: bool,
}

crate::json_struct!(out Registry { srid, exact });

struct Why<'a>(&'a Refusal);

impl ToJson for Why<'_> {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "kind", self.0.kind());
        field(out, &mut first, "detail", self.0.detail());
        out.push('}');
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("crsReadText", |text: String| Answer(read_text(&text))),
    op!(
        "crsWriteWkt",
        |name: String, system: System, base_name: Option<String>| {
            write_wkt(&name, &system, base_name.as_deref())
        }
    ),
    op!("crsWriteProj", |system: System| write_proj(&system)),
];
