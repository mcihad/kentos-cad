//! A layer's look in and out as OGC SLD 1.1 / SE 1.1 (docs/adr/0516, plan/gis.md CBS-17): the subset QGIS 4.2,
//! GeoServer and MapServer share. What one side has and the other has not is named (`export_losses`,
//! `import_losses`), never dropped silently.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// A layer's look written as an SLD file.
pub const FILE_EXPORT_SLD: &str = "file.export.sld";
pub const FILE_EXPORT_SLD_VERSION: u32 = 1;
/// An SLD file's look made a layer's.
pub const FILE_IMPORT_SLD: &str = "file.import.sld";
pub const FILE_IMPORT_SLD_VERSION: u32 = 1;

/// Input of `file.export.sld` v1: the layer's look (its renderer, or its simple look) at `path`, an `.sld`. The
/// symbolizers are written for the geometry its objects have (all three when it has none). Refusals, in order:
/// `invalid_file_path`, `file_exists`, `layer_not_found`, `not_a_layer`, `nothing_to_export` (no rule SE can say),
/// `file_unwritable`. Warnings: `export_losses`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportSld {
    /// An absolute path ending `.sld`.
    pub path: String,
    /// The layer's id.
    pub layer: String,
    /// Write over a file that is there.
    #[serde(default)]
    pub overwrite: bool,
}

/// What `file.export.sld` wrote (or with `plan` would write).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SldExported {
    pub path: String,
    pub layer: String,
    /// SE's rules written.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rules: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    /// What SE could not say, a sentence each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub lost: Vec<String>,
}

/// Input of `file.import.sld` v1: the look of the SLD at `path` (or its `text`) made the layer's renderer, one undo
/// step “Katman stili”. Rules of one field's values become a categorized renderer, of one field's ranges a graduated
/// one, one rule with no filter a single symbol, others rules with KentOS's expressions. Refusals, in order:
/// `invalid_input` (neither or both of `path` and `text`), `file_unreadable`, `invalid_sld`, `layer_not_found`,
/// `not_a_layer`, `service_layer`, `revision_conflict`. Warnings: `import_losses`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportSld {
    /// The layer's id.
    pub layer: String,
    /// An absolute path to an `.sld`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// The SLD itself, instead of a path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = crate::REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What `file.import.sld` made the layer's look.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SldImported {
    pub layer: String,
    /// The renderer's type: `single`, `categorized`, `graduated` or `rules`.
    pub kind: String,
    /// SE's rules taken.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rules: u32,
    /// The renderer as the layer now has it.
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub renderer: serde_json::Value,
    /// Whether the layer's look changed (an undo step was written).
    pub changed: bool,
    /// What KentOS could not take, a sentence each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub lost: Vec<String>,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = crate::REVISION_TEXT)))]
    pub revision: String,
}
