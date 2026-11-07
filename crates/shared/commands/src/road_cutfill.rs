//! `road.terrains.cutfill` v1 (plan/map-cad.md MAP-15, docs/adr/1840): Kazı ve dolgu bölgeleri, where a comparison
//! surface lies below a base (cut) and above it (fill), written on a layer: each zone a solid hatch (its fill) and a
//! closed area with its holes (its data), its plan area and volume in their attributes, the cut red and the fill blue. With an interval the difference is
//! banded from zero, each band its own zones (a cut and fill map by depth, as Civil 3D's banding from a datum). The
//! zones' volumes are `road.terrains.compare`'s (its exact TIN–TIN method). One undo step “Kazı ve dolgu bölgeleri”;
//! the drawing is kept with both surfaces' stamps, so either surface changing makes it stale (`road.project.rebuild`).
//! The shared cases are `fixtures/commands/v1/road.terrains.cutfill.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const ROAD_TERRAINS_CUTFILL: &str = "road.terrains.cutfill";
pub const ROAD_TERRAINS_CUTFILL_VERSION: u32 = 1;

/// The most bands an interval may give over the two surfaces' differences.
pub const MOST_CUTFILL_BANDS: u64 = 1000;

/// Input of `road.terrains.cutfill` v1: the zones of `comparison` over `base` (each a surface's id or name), over the
/// part of the comparison's plan the base covers, written on `layerId`. `interval` (m, above 0) bands the difference
/// `comparison − base` from zero: band k holds `[k·interval, (k+1)·interval)`; none: one band of cut and one of fill.
///
/// Refusals, in this order: `not_a_road_project`, `layer_not_found`, `not_a_layer`, `layer_locked`, `invalid_revision`,
/// `revision_conflict`, `invalid_analysis` (the interval not finite or not over 0), `unknown_terrain`,
/// `invalid_analysis` (the same surface twice), `terrain_unbuildable`, `invalid_analysis` (more than 1000 bands),
/// `no_cut_fill` (the surfaces do not overlap, or coincide wherever they do). Warning: `layer_hidden`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsCutFill {
    pub base: String,
    pub comparison: String,
    /// The layer the zones go on.
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub interval: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Whether a zone is cut (the comparison below the base) or fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CutFillKind {
    Cut,
    Fill,
}

/// A zone as written: cut or fill; with an interval its band of the difference, `[from, to)` (m); its plan area, its
/// volume (m³, the cut's and the fill's alike above 0); its outer ring (counterclockwise, from its smallest place, x
/// then y) and its holes (clockwise, each from its smallest place, ordered by it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainCutFillZone {
    pub kind: CutFillKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub volume: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub ring: Vec<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Vec<[f64; 2]>>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub holes: Vec<Vec<[f64; 2]>>,
}

/// Output of `road.terrains.cutfill` v1, and its plan: the surfaces' ids, the interval asked, the zones (by band, cut
/// first, then by their ring's first place); the cut and fill volumes (the zones' sums, `road.terrains.compare`'s
/// exact ones), the net (fill − cut), their plan areas (a coincident place in neither); the area where the surfaces
/// coincide (no zone there), the area both cover and the comparison's area off the base; the least and the largest
/// difference; the objects written, each zone's hatch then its closed area (a plan: none), and the revision after it (a
/// plan: before).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsCutFilled {
    pub base: String,
    pub comparison: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub interval: Option<f64>,
    pub zones: Vec<TerrainCutFillZone>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub net: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub cut_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub fill_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub coincident: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub covered: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub uncovered: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub largest: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
