//! Convenient paper operations over the designer's same core (ADR 0592).
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_SHEETS_CREATE: &str = "cad.sheets.create";
pub const CAD_SHEETS_CREATE_VERSION: u32 = 1;
pub const CAD_SHEETS_FIT: &str = "cad.sheets.fit";
pub const CAD_SHEETS_FIT_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SheetTemplateSource {
    System {
        id: String,
    },
    Custom {
        #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
        #[cfg_attr(feature="schema",schemars(with="kentos_sheet::template::Template", extend("x-raw-record"=true)))]
        template: serde_json::Value,
    },
}
fn empty_options() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetsCreate {
    /// New sheet id. Item and master ids are derived from it deterministically.
    pub id: String,
    pub source: SheetTemplateSource,
    /// Page choice, name, centre in metres, scale denominator and variable answers.
    #[serde(default = "empty_options")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    #[cfg_attr(feature="schema",schemars(with="kentos_sheet::template::InstanceOptions", extend("x-raw-record"=true)))]
    pub options: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="index")))]
    pub index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(regex(pattern=crate::cad::REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}
fn five() -> u8 {
    5
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetsFit {
    pub item: String,
    /// Object ids; absent together with bounds: all model objects. An empty list is refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    /// Explicit ground box [min east,min north,max east,max north], canonical metres. Mutually exclusive with uids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="m")))]
    pub bounds: Option<[f64; 4]>,
    /// Extra ground on each side as a percent of its size (0–100).
    #[serde(default = "five")]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="percent")))]
    pub margin_pct: u8,
    #[serde(default)]
    pub standard_scales: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(regex(pattern=crate::cad::REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}
