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

/// What an IFC file's bodies are, its header's view definition (docs/adr/8071 §4): `designTransfer` (the first value)
/// writes each body whole and its openings' bodies for the reader to take out of their walls; `reference` writes each
/// body already cut, its openings there as 'Reference' shapes, not taken out again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum IfcView {
    #[default]
    DesignTransfer,
    Reference,
}

impl IfcView {
    pub fn is_design_transfer(&self) -> bool {
        *self == IfcView::DesignTransfer
    }
}

/// Input of `cad.arch.ifc` v1: the file's path (absolute, ending `.ifc`); `stamp` the file's date and time
/// (`YYYY-MM-DDThh:mm:ss`; absent: now, UTC); `overwrite` writes over a file that is there; `view` what its bodies
/// are (absent: `designTransfer`).
///
/// Refusals, in this order: `invalid_file_path` (empty, relative, not `.ifc`, its folder not there), `file_exists`
/// (without `overwrite`), `invalid_ifc` (a stamp that does not read; nothing to write: no wall, slab, column, beam,
/// roof, railing, stair or room with a body). Warnings: `export_losses` (an object of the building the file does not
/// hold: one the section model gives no body, an opening without its outline through a written wall, a room without
/// an area, grid axes all in one direction on their storey; a curved stair written as straight pieces, or a curved
/// object whose arcs the model could not take back); `ifc_open_body` (a body written that is not a closed, simple
/// solid, with the object's name and why).
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
    #[serde(default, skip_serializing_if = "IfcView::is_design_transfer")]
    #[cfg_attr(feature = "ts", ts(as = "Option<IfcView>", optional))]
    pub view: IfcView,
    /// One of the building program's buildings, by its id (docs/adr/8072 §2): its storeys (a shared level where it
    /// counts) and the objects on them, at its ±0.00, its code in the building's id; none: the project as one building.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub building: Option<String>,
}

/// Output of `cad.arch.ifc` v1, and its plan: the file, its schema, what its bodies are (`view`), how many of each
/// class it holds (the openings are the `IfcOpeningElement`s, the doors and the windows filling them among them), its
/// size and its SHA-256.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcBuildingExported {
    pub path: String,
    pub schema: String,
    #[serde(default)]
    pub view: IfcView,
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
    // Its ramps, chimneys (the flues in walls and the roof's stack over them one each), curtain walls, members (posts,
    // rails and the curtain walls' mullions and transoms), fixtures (the library's blocks that are building elements:
    // lifts, sanitary terminals, furniture) and grid axes (docs/adr/8070 §9).
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub ramps: u32,
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub chimneys: u32,
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub curtain_walls: u32,
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub members: u32,
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub fixtures: u32,
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub grid_axes: u32,
    /// Its coverings (docs/adr/8070 §10): the slabs that are one; `slabs` does not count them.
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub coverings: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
}
