//! A processing tool run (`RunJob` in `apps/web/src/processing/job.ts`): what an
//! executor needs, with no reference to the page. The server executor will
//! take snapshot and selection references instead of copied objects
//! (CLAUDE.md §19).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::document::AngleUnit;

/// Project units a tool's defaults and summaries use.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DefaultsContext {
    pub length_decimals: u32,
    pub area_decimals: u32,
    pub angle_unit: AngleUnit,
    pub plot_scale: f64,
    pub active_layer: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RunJob {
    pub tool_id: String,
    /// Parameter values as entered (features as references, layers as targets).
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    pub values: BTreeMap<String, serde_json::Value>,
    pub units: DefaultsContext,
    pub selection: Vec<u32>,
    /// Layer id → name, for expressions and summaries.
    pub layers: Vec<(String, String)>,
}
