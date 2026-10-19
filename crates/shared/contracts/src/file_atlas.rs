//! The atlas as one PDF (docs/adr/0534, plan/gis.md CBS-21): a sheet whose map follows the objects of a coverage layer,
//! written as one page for each object, made from a state of the drawing the plan pinned. The sheet core's atlas
//! planner chooses the pages (its filter, sort, names and scales); this command binds them to the drawing's data:
//! which revision of the drawing and of each bound source the pages are made from, which theme, time and coordinate
//! system they are drawn in, and which attributes they may carry.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::entity::Vec2;

/// The atlas of a sheet as a PDF with a page for each object of its coverage layer.
pub const FILE_EXPORT_ATLAS: &str = "file.export.atlas";
pub const FILE_EXPORT_ATLAS_VERSION: u32 = 1;

/// What a layer is to an atlas run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AtlasLayerRole {
    /// The layer whose objects are the pages.
    Coverage,
    /// A layer the sheet's maps draw.
    Map,
}

/// A bound layer's source file, as the run pinned it (docs/adr/0503).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AtlasPinSource {
    /// The link's id, `kaynak-N`.
    pub id: String,
    pub path: String,
    /// The file's SHA-256 when the layer last matched it: the revision the layer's objects are the copy of.
    pub revision: String,
    /// The file's SHA-256 when the run looked; none when it could not be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub current: Option<String>,
}

/// One layer's state in a pin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AtlasPinLayer {
    pub layer: String,
    pub name: String,
    pub role: AtlasLayerRole,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// The SHA-256 of the layer's objects' kinds, geometries and attributes by lasting id (`sources::layer_digest`).
    pub digest: String,
    /// The file the layer is the working copy of, when it is bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<AtlasPinSource>,
}

/// The state of the drawing an atlas is made from (docs/adr/0534 §2): the project's parts' stamps, each layer's
/// digest and each bound file's SHA-256. A run given a pin refuses (`pin_mismatch`) when what it reads is not it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AtlasPin {
    /// The document's revision counter, decimal text: told, not compared (an undo and a redo change it, the content
    /// not).
    pub revision: String,
    pub settings: String,
    pub layers: String,
    pub styles: String,
    pub sheets: String,
    pub data: Vec<AtlasPinLayer>,
    /// The SHA-256 of all the above but the revision: one word for the state.
    pub digest: String,
}

/// Input of `file.export.atlas` v1. Refusals, in order: `invalid_file_path`, `file_exists`, `no_sheets`,
/// `unknown_sheet` (`sheet`), `no_atlas`, `atlas_no_layer`, `crs_unsupported` (`crs`), `unknown_theme`, `invalid_time`,
/// `field_not_found` (`fields[i]`), `atlas_expression`, `atlas_empty`, `pin_mismatch` (`pin`; a conflict: take the plan
/// again), `source_changed` (a bound file changed while the pages were made: nothing is written, redo it),
/// `pdf_unwritable`, `file_unwritable`. Warnings: `source_stale` (a bound file is not the one its layer was read from:
/// the pages are made from the layer, the pinned revision), `source_unsaved` (a bound layer has edits not yet written
/// to its file).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportAtlas {
    /// The file to write, an absolute path.
    pub path: String,
    /// The sheet that has the atlas, by id.
    pub sheet: String,
    /// Write over a file that is there.
    #[serde(default)]
    pub overwrite: bool,
    /// The attributes the pages may carry, by name; the coverage layer's declared fields when absent (all of an
    /// undeclared layer's). A field a form marks `redact` is never carried, named or not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fields: Option<Vec<String>>,
    /// A layer state of the project (a theme) that the atlas maps draw instead of the layers they would show.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub theme: Option<String>,
    /// The moment the temporal layers are shown at (`2024-06-30` or `2024-06-30T12:00`): the coverage layer's objects and the
    /// maps' drawing are cut by it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub time: Option<String>,
    /// The coordinate system the pages are written in: the project's only (the EPSG code, or absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub crs: Option<u32>,
    /// The state a plan pinned; the run refuses when the drawing or a bound file is not it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pin: Option<AtlasPin>,
}

/// One page of the atlas as it was made.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AtlasPageInfo {
    /// From 1, the page's place in the file.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub index: u32,
    /// The page's name (the atlas's page-name expression; a repeated one is numbered).
    pub name: String,
    /// The object, by lasting id.
    pub feature: String,
    /// The first atlas map's scale, the denominator (1000 is 1/1000).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: u32,
    /// The map's centre: the middle of the object's box.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub center: Vec2,
    /// The object's box: minimum east, north, maximum east, north.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub bbox: [f64; 4],
}

/// What a run was bound to, written into the output (`Keywords`) and given back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AtlasBindings {
    /// The theme the atlas maps were drawn in; none: the layers each map shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub theme: Option<String>,
    /// The moment, as given; none: no time cut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub time: Option<String>,
    /// The project's coordinate system.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: u32,
    pub crs_name: String,
    /// Whether the PDF carries geographic viewports (GeoPDF): a transverse Mercator system.
    pub geo: bool,
}

/// What `file.export.atlas` wrote, or with `plan` would write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AtlasExported {
    pub path: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    pub replaced: bool,
    pub sheet: String,
    pub pages: Vec<AtlasPageInfo>,
    /// The state the pages are made from.
    pub pin: AtlasPin,
    pub bindings: AtlasBindings,
    /// The attributes the pages carry (what the sheet's expressions, tables and legends can read).
    pub fields: Vec<String>,
    /// The coverage layer's attribute names the pages do not carry: a form's `redact`, or not in `fields`.
    pub withheld: Vec<String>,
    /// The coverage layer's objects the layer's filter or the time left out, counted.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub filtered_out: u32,
    /// The sheet core's findings (an SVG picture drawn as a box …).
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub findings: serde_json::Value,
    /// What a host without a window cannot draw, said in words.
    pub notes: Vec<String>,
}
