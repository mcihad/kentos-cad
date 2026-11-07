//! `cad.field.package` v1 (plan/map-cad.md MAP-19, docs/adr/1920): Saha paketi, what goes from the design to the field as
//! one file and never as a list of nameless coordinates. A `.zip` holds `saha-paketi.json` (the project, the drawing's
//! revision, the design's digest, the coordinate system and epoch, the heights' system, the acceptance limits and the
//! quality notes, every item's persistent id), `noktalar.csv` (the design points: name, Y east, X north, height, id) and
//! `cizgiler.csv` (the design lines' vertices: id, order, Y, X, bulge). The design is read as `cad.asbuilt.check`
//! reads it, and the digest is its own (`designDigest`): the field record made with the package checks against it
//! (`expectedDesign`), so a record of an older design is never matched to a newer one silently. A surface named goes in
//! by its name, id and sources' digest. The drawing is not changed. The shared cases are
//! `fixtures/commands/v1/cad.field.package.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_FIELD_PACKAGE: &str = "cad.field.package";
pub const CAD_FIELD_PACKAGE_VERSION: u32 = 1;

/// The package's own format name in `saha-paketi.json`.
pub const FIELD_PACKAGE_FORMAT: &str = "kentos.field-package";
/// The files in a package, in order.
pub const FIELD_PACKAGE_FILES: [&str; 3] = ["saha-paketi.json", "noktalar.csv", "cizgiler.csv"];
/// The most quality notes a package carries, and the most characters of one.
pub const FIELD_NOTES_LIMIT: usize = 20;
pub const FIELD_NOTE_LENGTH: usize = 500;

/// Input of `cad.field.package` v1: the design's points (`design`, a layer's id; their names in `field`, none: `Ad`),
/// its lines (`lines`, a layer's id), at least one of them; a terrain surface (`surface`, its id or name; Harita and
/// Yol); the acceptance limits in metres (`horizontal`, `vertical`, `quality` for a measured point's σ; none given: none
/// is written, KentOS gives none); the quality notes; the `.zip` written (`path`, absolute; `overwrite` an existing
/// one); `stamp` the date and time written (`YYYY-MM-DDThh:mm:ss`; none: now).
///
/// Refusals, in this order: `invalid_field_package` (no design, a limit not above zero, an empty attribute's name, more
/// than 20 notes or a note over 500 characters, a stamp that is no date and time), `layer_not_found` (`design`,
/// `lines`), `not_a_road_project` and the surface's own (`unknown_terrain`, `terrain_unbuildable`), `invalid_file_path`
/// (empty, relative, not `.zip`, its folder missing), `file_exists`, `file_unwritable`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldPackage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub design: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lines: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub vertical: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub quality: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub notes: Vec<String>,
    pub path: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub overwrite: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stamp: Option<String>,
}

/// A design point as the package writes it: its name, place (east, north) and height, and its persistent id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PackagePoint {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: Option<f64>,
    pub uid: String,
}

/// The surface a package names: its id, name and the digest of what it is made of.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PackageSurface {
    pub id: String,
    pub name: String,
    pub digest: String,
}

/// Output of `cad.field.package` v1: the file written and its size and SHA-256; the design's digest
/// (`cad.asbuilt.check`'s `designDigest`) and the drawing's revision; the points as written, the lines' count (and their
/// vertices'); the surface; the system (`EPSG:5256`, its name; none: a local project), the epoch and the heights'
/// system in words; the limits and the notes as written; the stamp.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldPackaged {
    pub path: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    pub digest: String,
    pub revision: String,
    pub points: Vec<PackagePoint>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub lines: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub vertices: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<PackageSurface>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub epsg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub system: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub epoch: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub heights: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub vertical: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub quality: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub notes: Vec<String>,
    pub stamp: String,
}
