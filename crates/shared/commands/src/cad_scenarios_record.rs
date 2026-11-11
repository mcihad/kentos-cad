//! `cad.scenarios.record` v1 (plan/map-cad.md MAP-10, docs/adr/5060): Ölçüleri çizime yaz. A comparison of
//! `cad.scenarios.impact` written in the drawing as a table, its measures with their units on both sides and the
//! difference, the base's and the scenario's stamps in its last row and kept as its attributes: the drawing then says,
//! whenever the scenario is compared again, whether the table still holds (`ScenariosImpacted::records`). The table is
//! not updated by itself; a stale one is written again. One undo step. The shared cases are
//! `fixtures/commands/v1/cad.scenarios.record.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::ScenariosImpact;

pub const CAD_SCENARIOS_RECORD: &str = "cad.scenarios.record";
pub const CAD_SCENARIOS_RECORD_VERSION: u32 = 1;

/// The attributes the table keeps: the scenario group's id, the base's stamp and the scenario's.
pub const RECORD_SCENARIO: &str = "SenaryoKimlik";
pub const RECORD_STAMP: &str = "TabanDamgasi";
pub const RECORD_SCENARIO_STAMP: &str = "SenaryoDamgasi";

/// Input of `cad.scenarios.record` v1: the comparison as `cad.scenarios.impact` takes it, the layer the table is written
/// on (not one of the compared layers: the table would change their measures), the table's top left corner (none: two
/// table heights right of the compared layers' areas, at their top) and the revision the write expects.
///
/// Refusals, in this order: the comparison's own (their paths under `impact.`), `invalid_input` (`layerId`: no such
/// layer, a group, or a compared layer; `at`: not finite), `stale_revision`, and the write's (a locked layer).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScenariosRecord {
    pub impact: ScenariosImpact,
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.scenarios.record` v1: the table's persistent id, the stamps it keeps, its cells (the heading, a row
/// a measure, the stamps' row) and the drawing's revision after the write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScenariosRecorded {
    pub table: String,
    pub stamp: String,
    pub scenario_stamp: String,
    pub cells: Vec<Vec<String>>,
    pub revision: String,
}
