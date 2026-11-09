//! `cad.zoning.check` v1 (plan/map-cad.md MAP-09, docs/adr/1420): İmar planı denetimi, read only. The plan as typed
//! objects of the KentOS profile `imar-plani` (plan boundaries, decision areas, plan notes), not as symbols or colours:
//! a decision area without a use or with one off the list, a value the plan neither gives nor declares not given (an
//! empty value is not 0 and not "not given"), a value given and declared not given, KAKS past TAKS × floors (to
//! review), two decisions on one place, a plan boundary not covered by decisions, a decision outside its plan, a
//! plan note named but missing or a note for areas no area names, a decision whose plan is missing, an upper plan
//! missing or not of a higher level. Each finding names its objects. The check claims neither the ministry's schema
//! nor legal conformity. The shared cases are `fixtures/commands/v1/cad.zoning.check.json`.
//!
//! docs/adr/3440: a road width's and a road parcel's finding carries its regions (the pieces of the overlap with the
//! road's corridor that are counted), so that a program draws what the window hatches. Extra field: the command's
//! version stays.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_ZONING_CHECK: &str = "cad.zoning.check";
pub const CAD_ZONING_CHECK_VERSION: u32 = 1;

/// The profile the check reads.
pub const ZONING_PROFILE: &str = "imar-plani";

/// Input of `cad.zoning.check` v1: the tolerance (metres, a piece of an overlap or of a gap narrower than it is not
/// counted, docs/adr/0202 §2; none: 0.01). Refusal: `invalid_input` (`tolerance` not finite or under 0.000001).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningCheck {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
}

/// A finding: its kind (`missingLayer`, `noUse`, `unknownUse`, `missingValue`, `declaredValued`, `kaksReview`,
/// `overlap`, `uncovered`, `outside`, `noPlan`, `noteMissing`, `noteUnused`, `noLevel`, `upperMissing`,
/// `levelOrder`), its objects' persistent ids in the drawing's order, the area it measures (square metres) when it
/// measures one, and what it says.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningFinding {
    pub kind: String,
    pub uids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: Option<f64>,
    pub message: String,
    /// `roadWidth`, `roadParcel` (docs/adr/3440): the counted pieces of the overlap; none for the other kinds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ZoningRegion>>"))]
    pub regions: Vec<ZoningRegion>,
}

/// A finding's region (docs/adr/3440): its outer ring counter-clockwise and its holes clockwise, each from its lowest
/// vertex (the westmost of the lowest), the vertices on a straight run between two others left out; the regions in
/// order of their outer rings' first vertices (north, then east).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningRegion {
    pub outer: crate::RingGeometry,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<crate::RingGeometry>>"))]
    pub holes: Vec<crate::RingGeometry>,
}

/// Output of `cad.zoning.check` v1: the profile, the tolerance, how many plan boundaries, decision areas and plan
/// notes there are, the findings (by kind in the list's order, then by their first object's place in the drawing) and
/// their count by kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningChecked {
    pub profile: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub plans: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub decisions: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub notes: u32,
    pub findings: Vec<ZoningFinding>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub counts: BTreeMap<String, u32>,
}
