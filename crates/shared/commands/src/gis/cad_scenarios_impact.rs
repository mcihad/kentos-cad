//! `cad.scenarios.impact` v1 (plan/map-cad.md MAP-10, docs/adr/1440): Senaryo ölçüleri (“Alternatif etkisi” until
//! 2026-11-07), read only. A scenario (a design alternative: a group whose layers stand for base layers, docs/adr/0210 §9) set beside the field state it
//! replaces: each measure with its unit and its method, on both sides and their difference; the areas by use; every
//! decision that changed, matched by its key (added, removed, its geometry, its decision, both), with its area and
//! capacity before and after; the gaps in the data that leave a measure short; the assumptions. No single score picks
//! a “best” plan. The base layers' stamp says what the comparison was made against: given the stamp of an earlier
//! comparison, the answer says whether the field state has changed since (the alternative is then stale, not
//! updated). The shared cases are `fixtures/commands/v1/cad.scenarios.impact.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_SCENARIOS_IMPACT: &str = "cad.scenarios.impact";
pub const CAD_SCENARIOS_IMPACT_VERSION: u32 = 1;

/// Input of `cad.scenarios.impact` v1: the scenario group's id; the field that names a decision's use (none:
/// `Kullanim`), the key that matches a decision across the two sides (none: `KararKimlik`), and the stamp of an
/// earlier comparison (`since`) to say whether the base has changed since. Two measures more, each only when asked,
/// their values the user's (docs/adr/2040; KentOS gives none): `impervious`, the impervious share (0–1) of each use's
/// area; `access`, the area of the `from` uses whose centroid lies within `distance` metres of an area of the `to` uses;
/// `infrastructure` (docs/adr/2660), the area whose centroid lies within `distance` metres of an object of a layer;
/// `slope` (docs/adr/4260), the measured areas' slope on a terrain surface of the project.
///
/// Refusals, in this order: `invalid_input` (`impervious.<use>`: a share not between 0 and 1; `access.distance`,
/// `infrastructure.distance`: not above zero; `access.from`, `access.to`: no use given; `slope.classes` (docs/adr/4660):
/// a break not finite or not above zero, not rising, or more than 20; `slope.threshold`: not above
/// zero), `scenario_not_found` (`scenario`: no such node, or not a scenario group), `nothing_to_compare` (`scenario`:
/// none of its layers stands for a base layer), `invalid_input` (`infrastructure.layer`: no such layer),
/// `not_a_road_project` (`slope` in a project without terrain surfaces), `unknown_terrain` (`slope.surface`),
/// `terrain_unbuildable` (the surface cannot be made). Along a network (docs/adr/5060): `invalid_input`
/// (`access.connect`: none or not above zero with a network), `unknown_network` (`access.network`), `invalid_network`
/// (`access.network`: the network cannot be built), `tools_unavailable` (no host here builds networks).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScenariosImpact {
    pub scenario: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub since: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub impervious: Option<std::collections::BTreeMap<String, f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub access: Option<ImpactAccess>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub infrastructure: Option<ImpactInfrastructure>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub slope: Option<ImpactSlope>,
}

/// Arazi eğimi (docs/adr/4260): a terrain surface of the project (by id or name), the slope from which an area is
/// steep (percent, the rise over the run), and the uses measured (none or empty: every area). Each triangle of the
/// surface has one slope, from its plane (its normal, the cross product of two edges); an area's part on a triangle
/// takes that slope. The surface is the same on both sides.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactSlope {
    pub surface: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub threshold: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub from: Vec<String>,
    /// Docs/adr/4660: the user's slope class breaks, percent, rising (none: no classes; at most 20):
    /// the classes run from 0 to the first break, between two breaks and from the last one up,
    /// each holding its lower break.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<f64>>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub classes: Vec<f64>,
    /// Docs/adr/4660: the areas by aspect, the direction a triangle's slope faces (clockwise from north), in eight
    /// directions (north 337.5°–22.5°, north-east 22.5°–67.5° …, each holding its lower bound) and
    /// flat (a level triangle, no aspect).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub aspect: bool,
    /// Docs/adr/5060: the slope map: each measured area's parts on the surface's triangles with their slope class and
    /// aspect (`slopeMap`), so a host can colour them; their sizes are the class and aspect measures' own parts.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub map: bool,
}

/// The most slope class breaks `cad.scenarios.impact` takes (docs/adr/4660).
pub const MAX_SLOPE_CLASSES: usize = 20;

/// Altyapıya yakınlık (docs/adr/2660): an infrastructure layer (a water or sewer line's, by id), the straight distance,
/// metres, from an area's centroid to the nearest object of that layer, and the uses measured (none or empty: every
/// area). A layer the scenario stands in for is each side's own (the base's on the field state's side, the scenario's
/// on the alternative's); any other is the same on both. Nearness is not a line's capacity nor a right to connect.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactInfrastructure {
    pub layer: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub distance: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub from: Vec<String>,
}

/// Erişim (docs/adr/2040): the uses reached from (`from`, e.g. `konut`), the uses reached (`to`, e.g. `park`, `yesil`)
/// and the straight distance, metres, from a `from` area's centroid to the nearest `to` area (0 inside it).
///
/// Ağ üzerinden erişim (docs/adr/5060): with `network` (a network of the project, by id or name) the distance runs along
/// it instead: each area's centroid joins the network at its nearest place (a leg no longer than `connect` metres, the
/// user's; farther, the area is off the network), and the distance is the `from` area's leg, the way along the network
/// as its definition travels it, and the `to` area's leg. A network layer the scenario stands in for is each side's own.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactAccess {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub distance: f64,
    pub from: Vec<String>,
    pub to: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub network: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub connect: Option<f64>,
}

/// A base layer and the scenario's layer standing for it, by id and name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactLayer {
    pub base: String,
    pub layer: String,
    pub name: String,
}

/// A measure: its id (`objects`, `area`, `length`, `capacity`), what it is called, its unit (`count`, `m2`, `m`), its
/// value on the field state's side and on the alternative's, the difference (alternative − base), and how it is
/// computed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactMeasure {
    pub id: String,
    pub label: String,
    pub unit: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub base: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub alternative: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub difference: f64,
    pub method: String,
}

/// The areas of one use (the field's value; empty for none) on both sides, square metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactUse {
    pub code: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub base: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub alternative: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub difference: f64,
}

/// A decision that differs: its key, what changed (`added`, `removed`, `geometry`, `decision`, `both`), its use, area
/// and capacity on each side that has it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactChange {
    pub key: String,
    pub change: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub base_use: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub alternative_use: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub base_area: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub alternative_area: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub base_capacity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub alternative_capacity: Option<f64>,
}

/// Which areas the access measure reached (docs/adr/2260), so a host can show them: on each side, by persistent id in
/// the drawing's order, the `from` areas reached, those not reached, and the `to` areas.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactReach {
    pub base_reached: Vec<String>,
    pub base_unreached: Vec<String>,
    pub base_targets: Vec<String>,
    pub alternative_reached: Vec<String>,
    pub alternative_unreached: Vec<String>,
    pub alternative_targets: Vec<String>,
    /// Along a network (docs/adr/5060): each side's `from` areas on the network, in the drawing's order, with the
    /// nearest `to` area they reach along it, however far; none off the network or with no way to a `to` area.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ImpactRoute>>"))]
    pub base_routes: Vec<ImpactRoute>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ImpactRoute>>"))]
    pub alternative_routes: Vec<ImpactRoute>,
    /// The `from` areas off the network (their centroid farther from it than `connect`), by persistent id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub base_off: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub alternative_off: Vec<String>,
}

/// A `from` area's way to its nearest `to` area along the network (docs/adr/5060): both by persistent id, the
/// distance in all, the two legs (the `from` centroid to the network, the network to the `to` centroid), and the way
/// as points: the `from` centroid, its place on the network, the network's way, the `to` area's place on it, its
/// centroid.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactRoute {
    pub area: String,
    pub target: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub legs: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub path: Vec<[f64; 2]>,
}

/// The slope map (docs/adr/5060): each side's measured areas cut by the surface's triangles, the areas in the drawing's
/// order, an area's parts by their first vertex (south first, then west).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactSlopeMap {
    pub base: Vec<ImpactSlopePiece>,
    pub alternative: Vec<ImpactSlopePiece>,
}

/// A measured area's part on one triangle of the surface (docs/adr/5060): the area's persistent id, the slope class
/// (0 the first, below the first break; 0 with no breaks) and the aspect (0 flat, 1–8 north … north-west), the
/// triangle's slope (percent), the part's size (m²) and its outer ring (counter-clockwise, from its lowest vertex, the
/// westmost of equals) and holes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactSlopePiece {
    pub area: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub class: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub aspect: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub slope: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub size: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub ring: Vec<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<Vec<[f64; 2]>>>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub holes: Vec<Vec<[f64; 2]>>,
}

/// A comparison written in the drawing (docs/adr/5060, `cad.scenarios.record`): the table's persistent id, the base's
/// and the scenario's stamps it was written with, and whether each has changed since (the table is then stale: its
/// numbers are not the drawing's now and are not updated by themselves).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactRecord {
    pub table: String,
    pub stamp: String,
    pub scenario_stamp: String,
    pub base_changed: bool,
    pub scenario_changed: bool,
}

/// Which areas the infrastructure measure found near (docs/adr/3060), so a host can show them: on each side, by
/// persistent id in the drawing's order, the measured areas whose centroid lies within the distance, those beyond it,
/// and the side's infrastructure layer's objects. Nearness only: not a line's capacity or a right to connect.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactNear {
    pub base_near: Vec<String>,
    pub base_far: Vec<String>,
    pub base_lines: Vec<String>,
    pub alternative_near: Vec<String>,
    pub alternative_far: Vec<String>,
    pub alternative_lines: Vec<String>,
    /// Each side's measured areas joined to their nearest object of the layer (docs/adr/3460), in the drawing's order;
    /// none when the side's layer has no object.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ImpactLink>>"))]
    pub base_links: Vec<ImpactLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ImpactLink>>"))]
    pub alternative_links: Vec<ImpactLink>,
}

/// A measured area joined to the nearest object of the infrastructure layer (docs/adr/3460): the area's and the
/// object's persistent ids, the area's centroid, the object's nearest place to it (the centroid itself when the
/// object reaches it) and the distance between them. Nearness only: not the line's capacity or a right to connect.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactLink {
    pub area: String,
    pub line: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: f64,
}

/// Output of `cad.scenarios.impact` v1: the scenario, its pairs, the field and key used, the measures, the uses by
/// code, the decisions that changed (by key), the data's gaps, the assumptions, the base's stamp now and whether it
/// changed since the stamp given (none when none was given), which areas the access reached (only when `access` is
/// asked, docs/adr/2260), and the drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScenariosImpacted {
    pub scenario: String,
    pub name: String,
    pub layers: Vec<ImpactLayer>,
    pub field: String,
    pub key: String,
    pub measures: Vec<ImpactMeasure>,
    pub uses: Vec<ImpactUse>,
    pub changes: Vec<ImpactChange>,
    pub gaps: Vec<String>,
    pub assumptions: Vec<String>,
    pub stamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stale: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reach: Option<ImpactReach>,
    /// Which areas the infrastructure measure found near (only when `infrastructure` is asked, docs/adr/3060).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub near: Option<ImpactNear>,
    /// The slope map (only when `slope.map` is asked, docs/adr/5060).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub slope_map: Option<ImpactSlopeMap>,
    /// The scenario layers' stamp now, made as the base's (docs/adr/5060): what a table written in the drawing is
    /// checked against besides the base's.
    pub scenario_stamp: String,
    /// The comparisons of this scenario written in the drawing (`cad.scenarios.record`, docs/adr/5060), in the drawing's
    /// order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ImpactRecord>>"))]
    pub records: Vec<ImpactRecord>,
    pub revision: String,
}
