//! KentOS Yol's mass haul commands (plan/road.md Y9, docs/adr/0907): `road.hauls.edit` writes or removes a mass haul
//! (Brückner) diagram (an earthwork volume's mass ordinate against a balance line, with a free haul distance and
//! borrow and waste sites); `road.hauls.query` reads every one, or one, now: its balance points, its loops with their
//! balanced volume, haul moment, average haul and overhaul, and the volume left to waste or borrow. The geometry is
//! `kentos-road`'s; a diagram is a project setting kept in the undo history, its reading is computed on every read.
//! The desktop's handlers are `crates/native/application/src/road_hauls*.rs`; they and the Python SDK pass the shared
//! cases in `fixtures/commands/v1/road.hauls.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{HaulBarrierDef, HaulCostDef, HaulSiteDef, HaulUnsuitableDef, RoadEditOperation};

pub const ROAD_HAULS_EDIT: &str = "road.hauls.edit";
pub const ROAD_HAULS_EDIT_VERSION: u32 = 1;
pub const ROAD_HAULS_QUERY: &str = "road.hauls.query";
pub const ROAD_HAULS_QUERY_VERSION: u32 = 1;

/// Input of `road.hauls.edit` v1: the diagram by its id or name (absent: a new one). A new one needs `name` and
/// `volume`. A diagram follows its volume: it is not rebound to another.
///
/// Refusals, in this order: `not_a_road_project`, `invalid_haul` (rebind), `unknown_haul` (remove one not in the
/// project), `unknown_volume`, `invalid_haul` (another volume, a new one without its fields, set with nothing to set,
/// the definition by `road_problem`: a free haul not in (0, 100 km], a site without a name or with a volume not above
/// 0, a cost without a currency, a price below 0, a payload not in (0, 1000] m³; a volume that does not compute or a
/// site off its stations), `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulsEdit {
    pub operation: RoadEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub haul: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub volume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub balance: Option<f64>,
    /// The free haul distance; null takes it off.
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub free: Option<Option<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sites: Option<Vec<HaulSiteDef>>,
    /// The prices and the truck's payload; null takes them off.
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<HaulCostDef>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<HaulCostDef>"))]
    pub cost: Option<Option<HaulCostDef>>,
    /// The cut unsuitable for fill by stations (docs/adr/0944); an empty list takes them off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unsuitable: Option<Vec<HaulUnsuitableDef>>,
    /// What the haul road crosses (docs/adr/0944); an empty list takes them off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub barriers: Option<Vec<HaulBarrierDef>>,
    /// The haul road network its sites off the road are reached by (docs/adr/0955); null takes it off.
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<crate::HaulNetworkDef>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<crate::HaulNetworkDef>"))]
    pub network: Option<Option<crate::HaulNetworkDef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A loop between two balance points, or an end stretch that does not close: its stations and km, its direction, the
/// volume balanced in it, the haul moment, the average haul (none where nothing is balanced), the overhaul's volume and
/// moment beyond the free haul distance, and the volume with no destination (waste forward, borrow backward).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulLoop {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
    pub forward: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub volume: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume-m")))]
    pub moment: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub average: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub over_volume: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume-m")))]
    pub over_moment: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub unbalanced: f64,
}

/// A diagram's cost as priced (docs/adr/0907 §5): the currency, the overhaul (its moment in m³·km times its price), the
/// waste and the borrow (their volume times their price), each where priced, the total of those priced, and the truck
/// trips (everything moved, hauled, wasted and borrowed, over the payload, rounded up) where a payload is given.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulCostReport {
    pub currency: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money")))]
    pub overhaul: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money")))]
    pub waste: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money")))]
    pub borrow: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money")))]
    pub total: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub trips: Option<f64>,
}

/// A mass haul diagram as it reads now: its sources, whether its volume's corridor is stale, the ordinate's points
/// (station, m³; with `points`), the balance points, the loops, the totals; why it does not read when it does not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulReport {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub volume: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub balance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub free: Option<f64>,
    pub stale: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[f64; 2]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub points: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub balance_points: Vec<f64>,
    pub loops: Vec<HaulLoop>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub hauled: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume-m")))]
    pub moment: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub average: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub over_volume: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume-m")))]
    pub over_moment: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub waste: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub borrow: f64,
    /// How the haul is assigned: `layers`, the mass diagram read layer by layer (the balance point method), not an
    /// optimisation (docs/adr/0907 §5).
    pub method: String,
    /// Its cost, where priced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cost: Option<HaulCostReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<String>,
}

/// A mass haul diagram in a list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulListed {
    pub id: String,
    pub name: String,
    pub volume: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
}

/// Output of `road.hauls.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulsWritten {
    /// The diagram written, read; absent when removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub haul: Option<HaulReport>,
    pub hauls: Vec<HaulListed>,
    pub revision: String,
}

/// Input of `road.hauls.query` v1: every diagram read, or one (`haul`: its id or name) with its ordinate's points.
///
/// Refusals: `not_a_road_project`, `unknown_haul`, `invalid_haul` (`points` without a diagram; the one asked does not
/// read). Warnings: `corridor_stale`, `volume_gap`, `haul_unbalanced`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub haul: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub points: bool,
}

/// Output of `road.hauls.query` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulsQueried {
    pub hauls: Vec<HaulReport>,
}

pub const ROAD_HAULS_PLAN: &str = "road.hauls.plan";
pub const ROAD_HAULS_PLAN_VERSION: u32 = 1;

/// Input of `road.hauls.plan` v1 (docs/adr/0924; ROAD-HAUL-02, ROAD-HAUL-07): a mass haul diagram's (its id or name)
/// earthwork moved at the least haul moment: each stretch's net at its middle, its borrow and waste sites taking up to
/// their volume (here a capacity). Every cut placed, every fill met; least cost flow by successive shortest paths,
/// exact. Reads only.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_haul`, `invalid_haul` (the volume does not
/// compute or has no stretch, a site off its stations). Warnings: `haul_plan_infeasible`, `corridor_stale`,
/// `volume_gap`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulsPlan {
    pub haul: String,
    /// What the plan makes least (docs/adr/0925): the haul moment (absent), or the cost with the diagram's free haul
    /// and prices, and among plans of that cost the moment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub objective: Option<HaulObjective>,
}

/// What a haul plan makes least (docs/adr/0925).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HaulObjective {
    /// The haul moment (m³·m).
    Moment,
    /// The cost: the overhaul beyond the free haul, the waste and the borrow at the diagram's prices.
    Cost,
}

/// An economic haul plan's cost (docs/adr/0925): the currency, the moment beyond the free haul (m³·m), the overhaul,
/// the waste and the borrow priced, and their total.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulPlanCost {
    pub currency: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume-m")))]
    pub over_moment: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money")))]
    pub overhaul: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money")))]
    pub waste: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money")))]
    pub borrow: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money")))]
    pub total: f64,
    /// The limit of economic haul (m): the free haul and the waste and borrow prices (those given) over the overhaul's
    /// per metre; beyond it wasting the cut and borrowing the fill costs less than hauling. None where neither price
    /// is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub economic: Option<f64>,
}

/// A site in a haul plan: its name, kind, station, capacity and the volume the plan gives or takes there.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulPlanSite {
    pub name: String,
    pub kind: crate::HaulSiteKind,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub capacity: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub used: f64,
    /// Of a waste site's, the unsuitable cut (docs/adr/0944); none where the plan does not sort its material.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub used_unsuitable: Option<f64>,
    /// Of a site off the road (docs/adr/0955): its access road's length from the alignment (m); `station` is then
    /// where the access meets the alignment. None: on the road.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub access: Option<f64>,
    /// A site off the road the network does not reach: it is left out of the plan (used 0).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub unconnected: bool,
}

/// A segment between neighbouring places of a haul plan: its stations and the flow (m³; forward positive).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulSegment {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub flow: f64,
    /// The unsuitable cut's part of the flow (docs/adr/0944); none where the plan does not sort its material.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub unsuitable_flow: Option<f64>,
}

/// A barrier in a haul plan (docs/adr/0944): its name, station, detour (none: closed) and what crosses it (m³, both
/// ways).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulPlanBarrier {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub detour: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub crossed: f64,
}

/// Output of `road.hauls.plan` v1: the diagram's id, whether everything is placed, the cut more than the fill and the
/// fill more than the cut over the stretches, what moves, the haul moment (m³·m) and its average (m; none where nothing
/// moves), the sites as used, the segments' flows (none when infeasible), and where infeasible the fill no borrow meets
/// and the cut no waste takes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulPlanned {
    pub haul: String,
    pub feasible: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub moved: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume-m")))]
    pub moment: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub average: Option<f64>,
    pub sites: Vec<HaulPlanSite>,
    pub segments: Vec<HaulSegment>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub shortage: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub surplus: f64,
    pub objective: HaulObjective,
    /// The free haul the cost read (m); none for the moment, or where the diagram has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub free: Option<f64>,
    /// The cost, for the cost objective.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cost: Option<HaulPlanCost>,
    /// Where the diagram has unsuitable cut or barriers (docs/adr/0944): the unsuitable cut in all, the part of it no
    /// waste site takes, and its part of the moment (m³·m); none otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub unsuitable: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub stranded: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume-m")))]
    pub unsuitable_moment: Option<f64>,
    /// The plan as approved against this one, where the diagram has an approved plan (docs/adr/0954).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub approval: Option<HaulPlanApproval>,
    /// The barriers and what crosses each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<HaulPlanBarrier>>", optional))]
    pub barriers: Vec<HaulPlanBarrier>,
    /// The moment on the sites' access roads (m³·m; docs/adr/0955), part of `moment`; none where no site is off the
    /// road.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume-m")))]
    pub access_moment: Option<f64>,
}

pub const ROAD_HAULS_TRUCKS: &str = "road.hauls.trucks";
pub const ROAD_HAULS_TRUCKS_VERSION: u32 = 1;

/// Where the trucks of a haul fleet drive (docs/adr/0936 §2): on the corridor's profile (the road as designed), or on
/// the ground along its alignment (the corridor's surface, as it is before the earthwork).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HaulTrack {
    Profile,
    Ground,
}

/// A truck of a haul fleet (docs/adr/0936 §1), its numbers the user's: its masses empty and loaded (t), its engine's power
/// (kW) and the share of it reaching the wheels, its top speed and its speed down a grade that pushes it (km/h; none:
/// the top speed), the grip of its driven wheels (the friction coefficient and the share of the weight on them) and the
/// deceleration it stops with (m/s²).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulTruckDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "t")))]
    pub empty: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "t")))]
    pub gross: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "kW")))]
    pub power: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub efficiency: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "km/h")))]
    pub top: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "km/h")))]
    pub downhill: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub traction: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub drive: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/s2")))]
    pub brake: f64,
}

/// Input of `road.hauls.trucks` v1 (docs/adr/0936 §2; ROAD-HAUL-10): a haul fleet on a mass haul diagram's (its id or
/// name) haul plan (`road.hauls.plan`, by `objective`): the plan's flows split into hauls, each driven by the truck on
/// the road (`track`) loaded and back empty, its cycle (loading, the run, dumping, spotting), the trips (the volume over
/// `load`, m³ a truck carries, rounded up) and the loader's balance number; with `trucks` the fleet's production
/// deterministically and as the finite-source queue and the working hours (`hour`: the minutes worked in an hour); with
/// `fuel` (L per kWh at the engine) the fuel, `idle` (L/h) adding the time the engine does not pull, `co2` (kg/L) the
/// carbon; `trace` a haul (its index) whose speeds are given. Every number is the user's; KentOS fills in none. Reads only.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_haul`, `invalid_haul` (as the plan), `invalid_haul_trucks`
/// (a number not positive or out of its range, the profile or the ground not covering a haul), `haul_truck_stalls` (the
/// truck cannot climb a grade: its grip less than the grade's resistance). Warnings: `haul_plan_infeasible`,
/// `corridor_stale`, `volume_gap`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulsTrucks {
    pub haul: String,
    /// The plan's objective, as `road.hauls.plan`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub objective: Option<HaulObjective>,
    pub track: HaulTrack,
    pub truck: HaulTruckDef,
    /// The haul road's rolling resistance (percent of the weight).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub rolling: f64,
    /// The volume a truck carries, in the diagram's volume (m³).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub load: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "min")))]
    pub loading: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "min")))]
    pub dumping: f64,
    /// Manoeuvring and waiting at both ends, beyond the loader's queue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "min")))]
    pub spotting: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub trucks: Option<u32>,
    /// The minutes worked in an hour (none: 60).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "min")))]
    pub hour: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "L/kWh")))]
    pub fuel: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "L/h")))]
    pub idle: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "kg/L")))]
    pub co2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub trace: Option<u32>,
}

/// A haul of a fleet: its stations and their km, the volume, the horizontal distance, the rise (the height at its end
/// less at its start), the run loaded and back empty, the cycle (with the fixed times), the trips, the balance number
/// (the cycle over the loading: the trucks that keep the loader busy) and the fuel of a trip (with `fuel`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TruckHaul {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub volume: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub distance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub rise: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub loaded: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub empty: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub cycle: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub trips: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub balance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "L")))]
    pub fuel: Option<f64>,
}

/// A fleet of `trucks` at one loader over the hauls in turn: the match factor (of the mean cycle), the production per
/// working hour deterministically and as the finite-source queue (the volume over the hours), the loader busy in the
/// queue, a truck's mean wait at the loader per cycle, and the working hours both ways.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TruckFleet {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub trucks: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub match_factor: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume/h")))]
    pub steady: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume/h")))]
    pub queued: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub busy: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub wait: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "h")))]
    pub hours_steady: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "h")))]
    pub hours_queued: f64,
}

/// A haul's speeds along the way, loaded and back empty: `[station, km/h]`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TruckTrace {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub haul: u32,
    /// `[station (m), speed (km/h)]`.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub loaded: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub empty: Vec<[f64; 2]>,
}

/// Output of `road.hauls.trucks` v1: the diagram's id, where the trucks drove, the hauls, the trips, the mean cycle and
/// balance number (by trips; none where nothing moves), the energy at the engines (kWh), the fleet (with `trucks`), the
/// fuel (with `fuel`; the queue's waits with `trucks` too), the carbon (with `co2`), the speeds of `trace`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulTrucked {
    pub haul: String,
    pub track: HaulTrack,
    pub hauls: Vec<TruckHaul>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub trips: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub cycle: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub balance: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "kWh")))]
    pub energy: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fleet: Option<TruckFleet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "L")))]
    pub fuel: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "kg")))]
    pub co2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub trace: Option<TruckTrace>,
}

/// A borrow or waste site whose capacity or use differs from the approved plan's (docs/adr/0954): its name, the
/// approved capacity and use (none: a new site), today's (none: a site gone).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulSiteChange {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub approved_capacity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub capacity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub approved_used: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub used: Option<f64>,
}

/// The approved plan against the plan made now with the approved objective (docs/adr/0954): the approval, whether it
/// went stale, why (in words: the diagram's, the volume's or its approval's revision; a site's capacity; the plan's own
/// figures), the moment's change and the sites that changed. None of today's plan where it does not compute.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulPlanApproval {
    pub approved: crate::HaulApprovalDef,
    pub stale: bool,
    pub reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume-m")))]
    pub moment_change: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<HaulSiteChange>>", optional))]
    pub sites: Vec<HaulSiteChange>,
}

pub const ROAD_HAULS_APPROVE: &str = "road.hauls.approve";
pub const ROAD_HAULS_APPROVE_VERSION: u32 = 1;

/// Input of `road.hauls.approve` v1 (plan/road.md Y48, docs/adr/0954; ROAD-HAUL-08): a mass haul diagram's plan, as
/// planned now with `objective` (absent: the least moment), approved on the day `at` with a note; or, with `revoke`,
/// its approval taken off (`at` and `note` the day and why). An approval that ends goes to the diagram's history as it
/// was. One undo step “Taşıma planı onayı”.
///
/// Refusals: `not_a_road_project`, `unknown_haul`, `haul_not_approved` (revoke one not approved), `invalid_haul` (no
/// day, a plan that cannot place everything, one that does not compute, a day or note the rule refuses),
/// `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulsApprove {
    pub haul: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub revoke: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub objective: Option<HaulObjective>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

pub const ROAD_HAULS_NETWORK: &str = "road.hauls.network";
pub const ROAD_HAULS_NETWORK_VERSION: u32 = 1;

/// Input of `road.hauls.network` v1 (plan/road.md Y49, docs/adr/0955; ROAD-HAUL-06): a mass haul diagram's haul road
/// network read: its roads, where they meet the alignment, how they join, what does not join, and how each site off the
/// road is reached. Reads only.
///
/// Refusals: `not_a_road_project`, `unknown_haul`, `invalid_haul` (no network, a layer not in the drawing, a volume
/// or alignment that does not compute).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulsNetwork {
    pub haul: String,
}

/// A haul road: its object, part, layer and length in plan (m).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulRoadRead {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub entity: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub part: u32,
    pub layer: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

/// Where a road meets the alignment: the road (its index in `roads`), which end (true: its end), the station, km and
/// offset.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulJunction {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub road: u32,
    pub end: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    pub km: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: f64,
}

/// What does not join: two roads crossing along their lengths, a road crossing the alignment along its length, an end
/// that joins nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HaulNetworkIssue {
    Crossing,
    CrossesAlignment,
    Loose,
}

/// A thing in the network that does not join, in words, where, and the roads it is about (indices in `roads`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulNetworkProblem {
    pub kind: HaulNetworkIssue,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub roads: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    /// Of a loose end: how far the nearest road or the alignment lies (m).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub gap: Option<f64>,
    pub message: String,
}

/// How a site off the road is reached: joined with its access length, the station, km and offset where it meets the
/// alignment and the roads on the way (indices in `roads`, from the site); or why not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulSiteAccess {
    pub name: String,
    pub kind: crate::HaulSiteKind,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    pub joined: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub km: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub roads: Vec<u32>,
    /// Why it is not joined, in words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub why: Option<String>,
}

/// Output of `road.hauls.network` v1: the diagram, its tolerances, roads, junctions, joins, problems and sites.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulNetworkRead {
    pub haul: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub connect: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub reach: f64,
    pub roads: Vec<HaulRoadRead>,
    pub junctions: Vec<HaulJunction>,
    /// How many ends join another road.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub joins: u32,
    pub problems: Vec<HaulNetworkProblem>,
    pub sites: Vec<HaulSiteAccess>,
}
