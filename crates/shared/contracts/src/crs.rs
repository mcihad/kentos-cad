//! The project's own coordinate systems and datum choices (docs/adr/0168):
//! definitions kept with the project's settings (`customCrs`, `secondCustomCrs`,
//! `datumTransforms`). The geometry core's transforms
//! (`kentos_geometry_core::crs`) take them once the project's systems are
//! resolved: here a definition's datum is one of two fields (`datum`,
//! `customDatum`) and a local system's base an EPSG code or a definition,
//! as the Python SDK's types can say them. `.kcad` schema 13 writes them
//! (docs/specs/kcad-v2.md §6.4.1).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// One of the registry's datums.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RegistryDatum {
    #[serde(rename = "TUREF")]
    Turef,
    #[serde(rename = "ED50")]
    Ed50,
    #[serde(rename = "WGS84")]
    Wgs84,
}

/// Which way a Helmert transformation's rotations turn: EPSG's position
/// vector convention (9606) or its coordinate frame convention (9607).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Convention {
    PositionVector,
    CoordinateFrame,
}

/// Seven parameters: translations (m), rotations (″) in their convention,
/// the scale difference (ppm), and the accuracy (m) when it is given.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Helmert {
    pub translation: [f64; 3],
    pub rotation: [f64; 3],
    pub scale: f64,
    pub convention: Convention,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub accuracy: Option<f64>,
}

/// An ellipsoid: its name, semi-major axis (m) and inverse flattening.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Ellipsoid {
    pub name: String,
    pub semi_major: f64,
    pub inverse_flattening: f64,
}

/// A datum the project defines: its ellipsoid and, when it has one, its
/// seven parameters to WGS 84 (without them it stands alone).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CustomDatum {
    pub name: String,
    pub ellipsoid: Ellipsoid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub to_wgs84: Option<Helmert>,
}

/// A definition's datum, whichever of its two fields holds it: the
/// registry's by name (`datum`) or the project's own (`customDatum`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DatumRef<'a> {
    Registry(RegistryDatum),
    Custom(&'a CustomDatum),
}

/// The datum of a definition's two fields; none unless exactly one is set.
fn datum_ref<'a>(
    datum: Option<RegistryDatum>,
    custom: Option<&'a CustomDatum>,
) -> Option<DatumRef<'a>> {
    match (datum, custom) {
        (Some(d), None) => Some(DatumRef::Registry(d)),
        (None, Some(c)) => Some(DatumRef::Custom(c)),
        _ => None,
    }
}

/// A transverse Mercator grid: its origin's latitude (0 when left out),
/// central meridian, scale and false origin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TmDefinition {
    /// The registry's datum by name; or
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub datum: Option<RegistryDatum>,
    /// the project's own (exactly one of the two).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub custom_datum: Option<Box<CustomDatum>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub latitude_of_origin: Option<f64>,
    pub central_meridian: f64,
    pub scale_factor: f64,
    pub false_easting: f64,
    pub false_northing: f64,
}

/// Latitude and longitude on a datum.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GeographicDefinition {
    /// The registry's datum by name; or
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub datum: Option<RegistryDatum>,
    /// the project's own (exactly one of the two).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub custom_datum: Option<Box<CustomDatum>>,
}

impl TmDefinition {
    /// Its datum; none unless exactly one of the two fields is set.
    pub fn datum(&self) -> Option<DatumRef<'_>> {
        datum_ref(self.datum, self.custom_datum.as_deref())
    }
}

impl GeographicDefinition {
    /// Its datum; none unless exactly one of the two fields is set.
    pub fn datum(&self) -> Option<DatumRef<'_>> {
        datum_ref(self.datum, self.custom_datum.as_deref())
    }
}

/// A local system's coordinates to its base's: base x = a·x + b·y + c,
/// base y = d·x + e·y + f; a similarity turns counter-clockwise (degrees),
/// scales, then shifts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CrsPlane {
    Similarity {
        east: f64,
        north: f64,
        rotation: f64,
        scale: f64,
    },
    Affine {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
    },
}

/// A local system's base: a projected system of the registry by its EPSG
/// code (`srid`), or a transverse Mercator the project defines
/// (`definition`); exactly one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CrsBase {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub srid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub definition: Option<Box<CrsDefinition>>,
}

/// A local system: its base and the plane transform from it to the base.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LocalDefinition {
    pub base: CrsBase,
    pub plane: CrsPlane,
}

/// The kind of system a definition is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CrsSystem {
    Tm(TmDefinition),
    Geographic(GeographicDefinition),
    Local(LocalDefinition),
}

/// A coordinate system the project defines (docs/adr/0168 §1): its name
/// and what it is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CrsDefinition {
    pub name: String,
    pub system: CrsSystem,
}

/// An NTv2 grid the project's datum choice names: its SHA-256 (the
/// device's grid library keeps it by that), the file's name and size, the
/// accuracy the project gives it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GridChoice {
    pub id: String,
    pub file: String,
    /// The file's size in bytes (at most the core's 256 MiB).
    // A number in TypeScript, as JSON gives it: exact up to 2^53.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub accuracy: Option<f64>,
}

/// The project's choice for a pair of the registry's datums instead of
/// EPSG's way (docs/adr/0168 §3); the pair's other way is its reverse. It
/// shifts by seven parameters (`helmert`) or an NTv2 grid (`grid`); exactly
/// one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DatumTransform {
    pub from: RegistryDatum,
    pub to: RegistryDatum,
    /// What the values rest on: “ED50 → TUREF: Bölge 7”.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub helmert: Option<Helmert>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grid: Option<GridChoice>,
}

// ── What a definition may be (docs/specs/kcad-v2.md §6.4.1) ───────────────────────────────────────────────────────

fn finite(values: &[f64]) -> bool {
    values.iter().all(|v| v.is_finite())
}

impl Helmert {
    /// What is wrong with the parameters, or none.
    pub fn problem(&self) -> Option<String> {
        if !finite(&self.translation) || !finite(&self.rotation) || !self.scale.is_finite() {
            return Some("yedi parametre sonlu sayılar olmalı".into());
        }
        match self.accuracy {
            Some(a) if !(a.is_finite() && a >= 0.0) => {
                Some("doğruluk 0 ya da büyük bir sayı olmalı".into())
            }
            _ => None,
        }
    }
}

fn custom_datum_problem(d: &CustomDatum) -> Option<String> {
    let e = &d.ellipsoid;
    if d.name.trim().is_empty() || e.name.trim().is_empty() {
        return Some("datumun ve elipsoidin adı boş olamaz".into());
    }
    let ok = e.semi_major.is_finite()
        && e.semi_major > 0.0
        && e.inverse_flattening.is_finite()
        && e.inverse_flattening > 1.0;
    if !ok {
        return Some(
            "elipsoidin büyük yarı ekseni 0'dan, ters basıklığı 1'den büyük olmalı".into(),
        );
    }
    d.to_wgs84.as_ref().and_then(Helmert::problem)
}

/// What is wrong with a definition's datum, or none: exactly one of its two
/// fields, a datum of the project's by its rules.
fn datum_problem(d: Option<DatumRef<'_>>) -> Option<String> {
    match d {
        None => Some("datum ya kayıttaki bir ad ya projenin datumu olur, yalnız biri".into()),
        Some(DatumRef::Registry(_)) => None,
        Some(DatumRef::Custom(c)) => custom_datum_problem(c),
    }
}

impl TmDefinition {
    fn problem(&self) -> Option<String> {
        let lat0 = self.latitude_of_origin.unwrap_or(0.0);
        if !finite(&[
            lat0,
            self.central_meridian,
            self.scale_factor,
            self.false_easting,
            self.false_northing,
        ]) {
            return Some("izdüşümün değerleri sonlu sayılar olmalı".into());
        }
        if self.scale_factor <= 0.0 {
            return Some("ölçek 0'dan büyük olmalı".into());
        }
        if self.central_meridian.abs() > 180.0 || lat0.abs() > 90.0 {
            return Some(
                "orta meridyen −180° ile 180°, başlangıç enlemi −90° ile 90° arasında olmalı"
                    .into(),
            );
        }
        datum_problem(self.datum())
    }
}

impl CrsPlane {
    /// What is wrong with the plane transform, or none: it must not fold the plane.
    pub fn problem(&self) -> Option<String> {
        match *self {
            CrsPlane::Similarity {
                east,
                north,
                rotation,
                scale,
            } => {
                if !finite(&[east, north, rotation, scale]) || scale <= 0.0 {
                    return Some("benzerliğin değerleri sonlu, ölçeği 0'dan büyük olmalı".into());
                }
                None
            }
            CrsPlane::Affine { a, b, c, d, e, f } => {
                if !finite(&[a, b, c, d, e, f]) || a * e - b * d == 0.0 {
                    return Some(
                        "afinin katsayıları sonlu olmalı, düzlemi katlamamalı (a·e − b·d ≠ 0)"
                            .into(),
                    );
                }
                None
            }
        }
    }
}

impl CrsDefinition {
    /// What is wrong with the definition, or none.
    pub fn problem(&self) -> Option<String> {
        self.problem_as(false)
    }

    /// A local system's base may only be a transverse Mercator.
    fn problem_as(&self, base: bool) -> Option<String> {
        if self.name.trim().is_empty() {
            return Some("tanımın adı boş olamaz".into());
        }
        match &self.system {
            CrsSystem::Tm(t) => t.problem(),
            _ if base => Some("yerel sistemin tabanı bir TM izdüşümü olmalı".into()),
            CrsSystem::Geographic(g) => datum_problem(g.datum()),
            CrsSystem::Local(l) => match (&l.base.srid, &l.base.definition) {
                (Some(0), None) => Some("yerel sistemin tabanı kayıttaki bir sistem olmalı".into()),
                (Some(_), None) => l.plane.problem(),
                (None, Some(d)) => d.problem_as(true).or_else(|| l.plane.problem()),
                _ => Some(
                    "yerel sistemin tabanı ya bir EPSG kodu ya bir tanımdır, yalnız biri".into(),
                ),
            },
        }
    }
}

impl DatumTransform {
    /// What is wrong with the choice, or none.
    pub fn problem(&self) -> Option<String> {
        if self.from == self.to {
            return Some("datum seçimi iki ayrı datum arasında olur".into());
        }
        if self.name.trim().is_empty() {
            return Some("datum seçiminin adı boş olamaz".into());
        }
        match (&self.helmert, &self.grid) {
            (Some(h), None) => h.problem(),
            (None, Some(g)) => {
                let hex = g.id.len() == 64
                    && g.id
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
                if !hex {
                    Some(
                        "ızgaranın kimliği küçük harfli 64 onaltılık rakamla SHA-256 olmalı".into(),
                    )
                } else if g.size == 0 {
                    Some("ızgaranın boyu 0 olamaz".into())
                } else {
                    match g.accuracy {
                        Some(a) if !(a.is_finite() && a >= 0.0) => {
                            Some("doğruluk 0 ya da büyük bir sayı olmalı".into())
                        }
                        _ => None,
                    }
                }
            }
            _ => Some("datum seçimi ya yedi parametre ya ızgaradır, yalnız biri".into()),
        }
    }

    /// The pair it chooses for, either way round.
    pub fn pair(&self) -> [RegistryDatum; 2] {
        let mut p = [self.from, self.to];
        p.sort_by_key(|d| *d as u8);
        p
    }
}

/// What is wrong with a project's datum choices together: each must be
/// right, and a pair has at most one.
pub fn choices_problem(list: &[DatumTransform]) -> Option<String> {
    for (i, t) in list.iter().enumerate() {
        if let Some(p) = t.problem() {
            return Some(p);
        }
        if list[..i].iter().any(|o| o.pair() == t.pair()) {
            return Some("bir datum çifti için en çok bir seçim olur".into());
        }
    }
    None
}
