//! What the host gives the display list (design §8 `RenderInputs`): the
//! values only it knows — the project's name, the user, the date, the
//! coordinate system — and the data of legends, tables, coordinate lists
//! and the atlas page. The core lays it out; the host only paints.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::geodesy::TmParams;
use crate::kinds::GroundPoint;
use crate::model::{ItemId, VarValue, yes};
use crate::profile::Capabilities;
use crate::style::Stroke;
use crate::units::{Mdeg, Um};

/// While designing (every item) or for export (printable items only).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum RenderMode {
    #[default]
    Design,
    Export,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ProjectInfo {
    /// `@proje_adi`.
    #[serde(default)]
    pub name: String,
    /// `@kullanici`.
    #[serde(default)]
    pub user: String,
    /// `@tarih`: today, ISO (the core never reads a clock).
    #[serde(default)]
    pub date: String,
    /// `@koordinat_sistemi` when `crs` is not given.
    #[serde(default)]
    pub crs_name: String,
}

/// The project's coordinate system.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct CrsInfo {
    /// “TUREF / TM36”.
    pub name: String,
    /// A transverse Mercator's parameters: the core works the meridian convergence out from them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tm: Option<TmParams>,
}

/// A map's convergence from the host, when the core cannot work it out (no TM parameters).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MapInput {
    pub item: ItemId,
    /// Degrees; positive where grid north is east of true north.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub convergence: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum MarkerShape {
    #[default]
    Circle,
    Square,
    Triangle,
    Cross,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PatchSymbol {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stroke: Option<Stroke>,
    /// Diagonal hatching in this colour over the fill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hatch: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct LineSymbol {
    pub stroke: Stroke,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MarkerSymbol {
    #[serde(default)]
    pub shape: MarkerShape,
    pub size: Um,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stroke: Option<Stroke>,
}

/// A path of a symbol in its box, in thousandths of the box (0–1000 across and down).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SymbolPath {
    pub points: Vec<[u16; 2]>,
    #[serde(default)]
    pub closed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stroke: Option<Stroke>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PathsSymbol {
    pub paths: Vec<SymbolPath>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ImageSymbol {
    pub asset: String,
}

/// A legend row's symbol, as the host's legend engine (ADR 0093) draws it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum LegendSymbol {
    Patch(PatchSymbol),
    Line(LineSymbol),
    Marker(MarkerSymbol),
    Paths(PathsSymbol),
    Image(ImageSymbol),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct LegendEntryInput {
    pub layer: String,
    pub label: String,
    /// The heading it stands under (a layer group).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    pub symbol: LegendSymbol,
    /// Something of it is inside the map's frame.
    #[serde(default = "yes")]
    pub in_map: bool,
    /// Something of it is on the atlas page's object.
    #[serde(default = "yes")]
    pub in_atlas: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct LegendInput {
    pub item: ItemId,
    pub entries: Vec<LegendEntryInput>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Attribute {
    pub name: String,
    pub value: VarValue,
}

/// An object of a layer, for an attribute table: its fields and measures; the core filters, sorts and writes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct FeatureInput {
    pub id: String,
    #[serde(default)]
    pub attributes: Vec<Attribute>,
    /// `$alan`, m².
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub area: Option<f64>,
    /// `$uzunluk`, m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub length: Option<f64>,
    #[serde(default = "yes")]
    pub in_map: bool,
    #[serde(default = "yes")]
    pub in_atlas: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TableInput {
    pub item: ItemId,
    pub features: Vec<FeatureInput>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct CoordPoint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// East (Y), m.
    pub x: f64,
    /// North (X), m.
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub z: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct CoordinateInput {
    pub item: ItemId,
    pub points: Vec<CoordPoint>,
    /// The points close a figure (a parcel's corners).
    #[serde(default)]
    pub closed: bool,
    /// The figure's area as the drawing measures it (arcs included); none: from the points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub area: Option<f64>,
    /// How many objects the source gave, and how many of the list's own
    /// objects the drawing no longer has (docs/adr/0206 §2); none: not said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub objects: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub missing: Option<u32>,
}

/// An atlas object: its id, box and fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct AtlasFeature {
    pub id: String,
    /// min east, min north, max east, max north.
    pub bbox: [f64; 4],
    #[serde(default)]
    pub attributes: Vec<Attribute>,
}

/// A map's view on an atlas page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct AtlasMapView {
    pub item: ItemId,
    pub center: GroundPoint,
    pub scale: u32,
    pub rotation: Mdeg,
}

/// The atlas page being drawn: a page of `atlas::plan`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct AtlasPageInput {
    pub feature: AtlasFeature,
    /// 1-based.
    pub index: u32,
    pub count: u32,
    pub name: String,
    pub maps: Vec<AtlasMapView>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PageNumber {
    pub index: u32,
    pub count: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct RenderInputs {
    #[serde(default)]
    pub mode: RenderMode,
    #[serde(default)]
    pub project: ProjectInfo,
    #[serde(default)]
    pub capabilities: Capabilities,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub crs: Option<CrsInfo>,
    #[serde(default)]
    pub maps: Vec<MapInput>,
    #[serde(default)]
    pub legends: Vec<LegendInput>,
    #[serde(default)]
    pub tables: Vec<TableInput>,
    #[serde(default)]
    pub coordinates: Vec<CoordinateInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub atlas: Option<AtlasPageInput>,
    /// `@sayfa` and `@sayfa_sayisi`; none: the sheet's place in the book.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub page: Option<PageNumber>,
    /// The typefaces the host can draw; none: all of the table's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fonts: Option<Vec<String>>,
    /// The assets whose bytes the host has; none: all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub assets: Option<Vec<String>>,
    /// The export's resolution (the preflight's pictures); none: the sheet's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dpi: Option<u16>,
}
