//! A layer's look, or the whole map's, in and out as a MapLibre GL style (docs/adr/0560, plan/gis.md CBS-17): a
//! style version 8 with `fill`, `line`, `circle` and `symbol` layers over a GeoJSON file written beside it or over
//! a source URL given. What one side has and the other has not is named (`export_losses`, `import_losses`), never
//! dropped silently.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// A layer's look, or the visible layers', written as a MapLibre style.
pub const FILE_EXPORT_MAPLIBRE: &str = "file.export.maplibre";
pub const FILE_EXPORT_MAPLIBRE_VERSION: u32 = 1;
/// A MapLibre style's look made the look of the layers it is for.
pub const FILE_IMPORT_MAPLIBRE: &str = "file.import.maplibre";
pub const FILE_IMPORT_MAPLIBRE_VERSION: u32 = 1;

/// What kind of source `sourceUrl` names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MaplibreSourceKind {
    /// A GeoJSON file at the URL (`type: geojson`, `data`).
    Geojson,
    /// Vector tiles: a URL with `{z}/{x}/{y}` is the tile template (`tiles`), any other a TileJSON (`url`); each
    /// KentOS layer is a tile layer of its name (`source-layer`).
    Vector,
}

/// Input of `file.export.maplibre` v1: the look of `layer` (or of every visible layer, the one drawn last on top,
/// when `layer` is not given) as a MapLibre style at `path`, a `.json`. Without `sourceUrl` each layer's objects
/// are written as a WGS 84 GeoJSON beside the style (`<name>.geojson` for one layer, `<name>.<layer id>.geojson`
/// for several) and the style's source names that file; a project with no coordinate system cannot be written so
/// and `sourceUrl` is asked for. Refusals, in order: `invalid_file_path`, `file_exists`, `layer_not_found`,
/// `not_a_layer`, `no_system`, `nothing_to_export` (no class or rule MapLibre's layers here can draw),
/// `file_unwritable`. Warnings: `export_losses`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportMaplibre {
    /// An absolute path ending `.json`.
    pub path: String,
    /// The layer's id; absent, the visible layers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// Write over files that are there.
    #[serde(default)]
    pub overwrite: bool,
    /// The source's URL, instead of GeoJSON files written beside the style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source_url: Option<String>,
    /// What the URL is; absent, vector tiles when it has `{z}` or ends `.pbf` or `.mvt`, else GeoJSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source_kind: Option<MaplibreSourceKind>,
    /// The glyph server's URL for the labels' fonts (`{fontstack}` and `{range}` in it); absent, MapLibre's demo
    /// server. Only a style with labels has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub glyphs: Option<String>,
}

/// A GeoJSON file `file.export.maplibre` wrote (or with `plan` would write) beside the style.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MaplibreData {
    pub layer: String,
    pub path: String,
    /// Objects written.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub features: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
}

/// What `file.export.maplibre` wrote (or with `plan` would write).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MaplibreExported {
    pub path: String,
    /// The ids of the layers written, the bottom one first.
    pub layers: Vec<String>,
    /// The style's layers.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub style_layers: u32,
    /// Classes and rules written.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rules: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    /// The GeoJSON files written beside the style.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<MaplibreData>>", optional))]
    pub data: Vec<MaplibreData>,
    /// What the style could not say, a sentence each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub lost: Vec<String>,
}

/// Input of `file.import.maplibre` v1: the look of the style at `path` (or its `text`) made the renderer of the
/// layers it is for, one undo step “Katman stili”. A style layer is for a KentOS layer when its `metadata`
/// `kentos:layer` is the layer's id (a style KentOS wrote), else its `source-layer` or its `id` is the layer's id
/// or name, else, when `from` is given, its `source-layer` or `id` is `from`. With `layer` the one layer's;
/// without, every layer the style has layers for. MapLibre's `match` over a field becomes a categorized
/// renderer, `step` a graduated one, filters rules, and a style with none a single symbol; sizes stay in pixels and
/// widths that grow with the zoom as metres. Refusals, in order: `invalid_input` (neither or both of `path` and
/// `text`), `file_unreadable`, `invalid_maplibre`, `layer_not_found`, `not_a_layer`, `service_layer`,
/// `style_layer_not_found`, `revision_conflict`. Warnings: `import_losses`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportMaplibre {
    /// The layer's id; absent, every layer the style is for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// The style's `source-layer` or layer `id` to take for `layer`, when the style does not name it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from: Option<String>,
    /// An absolute path to a `.json`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// The style itself, instead of a path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = crate::REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// One layer's look `file.import.maplibre` made.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MaplibreLayerImported {
    pub layer: String,
    /// The style layers taken, by their ids.
    pub from: Vec<String>,
    /// The renderer's type: `single`, `categorized`, `graduated` or `rules`.
    pub kind: String,
    /// Classes and rules taken.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rules: u32,
    /// The renderer as the layer now has it.
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub renderer: serde_json::Value,
    /// Whether the layer's look changed.
    pub changed: bool,
}

/// What `file.import.maplibre` made of the layers' looks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MaplibreImported {
    pub layers: Vec<MaplibreLayerImported>,
    /// Whether any layer's look changed (an undo step was written).
    pub changed: bool,
    /// What KentOS could not take, a sentence each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub lost: Vec<String>,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = crate::REVISION_TEXT)))]
    pub revision: String,
}
