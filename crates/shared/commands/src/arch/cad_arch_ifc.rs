//! KentOS Mimari's building as IFC 4.3 (plan/architecture-advanced.md M47, docs/adr/8047): `cad.arch.ifc`
//! writes the drawing's building — its storeys, walls, slabs, columns, beams, roofs, railings, stairs, its openings with
//! their doors and windows, and its rooms — to an IFC 4.3 (`IFC4X3_ADD2`) file, its classes, names and properties as
//! Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında Yönetmelik EK-5 and EK-6 ask, whole or not
//! at all. It does not change the drawing. Every IFC rule is `kentos-ifc`'s; the desktop's handler is
//! `crates/native/application/src/arch_ifc.rs`; it and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/cad.arch.ifc.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_ARCH_IFC: &str = "cad.arch.ifc";
pub const CAD_ARCH_IFC_VERSION: u32 = 1;

/// Input of `cad.arch.ifc` v1: the file's path (absolute, ending `.ifc`); `stamp` the file's date and time
/// (`YYYY-MM-DDThh:mm:ss`; absent: now, UTC); `overwrite` writes over a file that is there.
///
/// Refusals, in this order: `invalid_file_path` (empty, relative, not `.ifc`, its folder not there), `file_exists`
/// (without `overwrite`), `invalid_ifc` (a stamp that does not read; nothing to write: no wall, slab, column, beam,
/// roof, railing, stair or room with a body). Warnings: `export_losses` (an object of the building the file does not
/// hold: one the section model gives no body, an opening without its outline through a written wall, a room without
/// an area, a ramp, not written yet; a curved wall or stair written as straight pieces).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportIfcBuilding {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stamp: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub overwrite: bool,
}

/// Output of `cad.arch.ifc` v1, and its plan: the file, its schema, how many of each class it holds (the
/// openings are the `IfcOpeningElement`s, the doors and the windows filling them among them), its size and its
/// SHA-256.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcBuildingExported {
    pub path: String,
    pub schema: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub storeys: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub walls: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub doors: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub windows: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub openings: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub spaces: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub slabs: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub columns: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub beams: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub roofs: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub railings: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stairs: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
}
