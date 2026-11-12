//! Portable drawing-set identity, shared by the book and command query results.
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// An issued sheet's identity is separate from its mutable tab name and document revision.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperDrawingInfo"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DrawingInfo {
    #[serde(default)]
    pub number: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub revision: String,
    /// A project's own workflow label (Taslak, Kontrol, Yayın...), not a hidden authorization rule.
    #[serde(default)]
    pub status: String,
    /// Hierarchy segments, e.g. ["Mimari", "Kat planları"]. An empty list is the root.
    #[serde(default)]
    pub subset: Vec<String>,
}
