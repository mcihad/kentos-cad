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

/// Where an atlas reads its coverage layer's objects from (docs/adr/0638 §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AtlasCoverage {
    /// The drawing's objects: a bound layer's working copy, the pinned state.
    #[default]
    Drawing,
    /// The bound layer's table whole, read from its source a part at a time; each page's maps read a bound part
    /// layer's rows in their reach from its source too, so a page outside the part has its content.
    Source,
}

/// What a run read from a bound layer's source (docs/adr/0638 §3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AtlasRead {
    pub layer: String,
    pub table: String,
    /// The rows read that the filter, the time and the box keep: the coverage table's once, a map layer's page after
    /// page (a row two pages reach counted twice).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u64,
    /// The pages a map layer was read for; none for the coverage table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub pages: Option<u32>,
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

/// Input of `file.export.atlas` v1. Refusals, in order: `invalid_file_path`, `file_exists`, `no_previous` (`previous`:
/// not a record of this sheet's atlas, docs/adr/4000; or none with `changedOnly`, docs/adr/4400), `no_sheets`,
/// `unknown_sheet` (`sheet`), `no_atlas`, `atlas_no_layer`, `coverage_not_bound` (`coverage`: `source` for a layer that is
/// not bound), `crs_unsupported` (`crs`: a project without a system, a code the registry does not have, a system in
/// degrees), `unknown_theme`, `invalid_time`,
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
    /// Source drawing path for resolving relative linked images; the native host fills its own file when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
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
    /// The coordinate system the pages are drawn in (the EPSG code; absent: the project's). Another than the project's
    /// is a registry's system in metres the project's moves to (docs/adr/0638 §1): the pages' maps, grids and GeoPDF are
    /// in it, each page's object fitted on its ground.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub crs: Option<u32>,
    /// The state a plan pinned; the run refuses when the drawing or a bound file is not it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pin: Option<AtlasPin>,
    /// Where the coverage layer's objects come from: the drawing (the default) or its bound source, whole.
    #[serde(default, skip_serializing_if = "is_drawing")]
    #[cfg_attr(feature = "ts", ts(as = "Option<AtlasCoverage>", optional))]
    pub coverage: AtlasCoverage,
    /// A previous atlas's record (docs/adr/4000; the `.kaynaklar.json` a run wrote beside its PDF), an absolute path:
    /// the answer then says which pages changed since, and why (ADR 1320's rule).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub previous: Option<String>,
    /// The update's PDF (docs/adr/4400): only the pages changed or new since `previous`, in the atlas's order, each
    /// numbered as the atlas numbers it (its own number and the atlas's count on it, the PDF's page labels the same).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub changed_only: bool,
}

/// The format of an atlas's record (docs/adr/4000).
pub const ATLAS_RECORD_FORMAT: &str = "kentos.atlas-sources/1";

/// An atlas's record, written beside its PDF as `<name>.kaynaklar.json` (docs/adr/4000): the PDF's file name and
/// SHA-256, the sheet, the day, the state the pages were made from, each page with its sources and each listed
/// object's digest: what a later run is told its changes from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AtlasRecord {
    pub format: String,
    pub pdf: String,
    pub sha256: String,
    pub sheet: String,
    pub date: String,
    pub pin: AtlasPin,
    pub pages: Vec<AtlasPageInfo>,
    #[cfg_attr(feature = "ts", ts(as = "std::collections::BTreeMap<String, String>"))]
    pub objects: std::collections::BTreeMap<String, String>,
    /// An update's PDF (docs/adr/4400): the pages it holds, by their index; absent: every page. The pages and objects
    /// above are still the whole atlas's, what the next update is told from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub written: Option<Vec<u32>>,
}

/// How a page stands against the previous atlas (docs/adr/4000).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AtlasPageState {
    /// Its object was a page then too, and nothing it lists, nor the project's parts, changed.
    Same,
    Changed,
    /// Its object was no page then.
    New,
}

/// Whether a changed page also looks other (docs/adr/4820): its look's digest against the previous atlas's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AtlasLook {
    /// It draws the same: only what it does not draw changed (an attribute no text, table or label shows).
    Same,
    /// It draws otherwise.
    Changed,
}

/// A page against the previous atlas (docs/adr/4000), ADR 1320's rule: the objects it listed then and lists now
/// whose digest changed (`changed`), the ones it lists now and not then (`added`), then and not now (`removed`), by
/// persistent id in the order listed; `changed` too when the project's parts the pages read changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AtlasPageChange {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub index: u32,
    pub feature: String,
    pub state: AtlasPageState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub changed: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub added: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub removed: Vec<String>,
    /// A changed page's look against the previous atlas's (docs/adr/4820), when both runs made the page and knew its
    /// look; none otherwise (a previous record without looks, a page the previous update did not make).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub look: Option<AtlasLook>,
}

/// The changes since a previous atlas (docs/adr/4000): its PDF's name, SHA-256 and day; the project's parts the pages
/// read that changed (`settings`, `layers`, `styles`, `sheets`: every page then changed); each page now; the previous
/// pages whose object is no page now (`gone`: name and object).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AtlasChanges {
    pub pdf: String,
    pub sha256: String,
    pub date: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub parts: Vec<String>,
    pub pages: Vec<AtlasPageChange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AtlasGonePage>>", optional))]
    pub gone: Vec<AtlasGonePage>,
}

/// A previous page whose object is no page now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AtlasGonePage {
    pub name: String,
    pub feature: String,
}

fn is_drawing(c: &AtlasCoverage) -> bool {
    *c == AtlasCoverage::Drawing
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
    /// The page's sources (docs/adr/3600; ADR 1320 §2's rule a page at a time): its own object first, then the objects
    /// its maps draw (its atlas maps on its own ground) or its tables read, by persistent id in drawing order; empty
    /// when they are not listed (a check that makes no pages).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub sources: Vec<String>,
    /// The page's look (docs/adr/4820): the SHA-256 of what it draws, as made for the PDF: its sheet's display list
    /// with the publication's own stamps set aside (the day, the user, the drawing state's note KentOS writes into an
    /// empty `kaynak`), its maps' paths and texts, its tables, coordinate lists, legends and survey notes, and the maps
    /// that stand still on every page. None where the pages were not made (a check), and for a page an update did not
    /// make whose previous record had none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub look: Option<String>,
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
    /// The coordinate system the pages are drawn in: the project's, or the one asked (docs/adr/0638 §1).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: u32,
    pub crs_name: String,
    /// Whether the PDF carries geographic viewports (GeoPDF): a transverse Mercator system.
    pub geo: bool,
    /// The project's system, when the pages are drawn in another: the drawing moved there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub project_srid: Option<u32>,
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
    /// What was read from bound sources (`coverage: source`): the coverage table at the plan, the map layers' rows
    /// as the pages were made.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AtlasRead>>", optional))]
    pub read: Vec<AtlasRead>,
    /// The content digest of each object a page lists (docs/adr/3600; `kentos.object-digest/1`, ADR 1320 §2: its
    /// snapshot form without its slot `id`, keys sorted, SHA-256), by persistent id: what a later run is told from.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<std::collections::BTreeMap<String, String>>", optional))]
    pub objects: std::collections::BTreeMap<String, String>,
    /// The record written (planned) beside the PDF (docs/adr/4000): `<name>.kaynaklar.json`; none for a check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub record: Option<String>,
    /// Asked with a previous record (docs/adr/4000): the changes since it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub changes: Option<AtlasChanges>,
    /// Asked `changedOnly` (docs/adr/4400): the pages the update's PDF holds, by their index (the changed and new).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub written: Vec<u32>,
}

/// The record's path beside an atlas's PDF (docs/adr/4000): `atlas.pdf` → `atlas.kaynaklar.json`; a name without the
/// `.pdf` ending takes `.kaynaklar.json` after it.
pub fn atlas_record_path(pdf: &str) -> String {
    let lower = pdf.to_lowercase();
    match lower.strip_suffix(".pdf") {
        Some(_) => format!("{}.kaynaklar.json", &pdf[..pdf.len() - 4]),
        None => format!("{pdf}.kaynaklar.json"),
    }
}
