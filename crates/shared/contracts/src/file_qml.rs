//! A layer's look in and out as a QGIS layer style file, `.qml` (docs/adr/0555, plan/gis.md CBS-17): QGIS 4.2's
//! single symbol, categorized, graduated and rule-based renderers with simple fills, lines, markers and centroid
//! fills. What one side has and the other has not is named (`export_losses`, `import_losses`), never dropped silently.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// A layer's look written as an QML file.
pub const FILE_EXPORT_QML: &str = "file.export.qml";
pub const FILE_EXPORT_QML_VERSION: u32 = 1;
/// An QML file's look made a layer's.
pub const FILE_IMPORT_QML: &str = "file.import.qml";
pub const FILE_IMPORT_QML_VERSION: u32 = 1;

/// Input of `file.export.qml` v1: the layer's look (its renderer, or its simple look) at `path`, a `.qml`. A QML is
/// for one geometry type: the layer's objects choose it (areas, then lines, then points; areas when it has none).
/// Refusals, in order: `invalid_file_path`, `file_exists`, `layer_not_found`, `not_a_layer`, `nothing_to_export` (no
/// class or rule QGIS's symbol layers here can draw), `file_unwritable`. Warnings: `export_losses`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportQml {
    /// An absolute path ending `.qml`.
    pub path: String,
    /// The layer's id.
    pub layer: String,
    /// Write over a file that is there.
    #[serde(default)]
    pub overwrite: bool,
}

/// What `file.export.qml` wrote (or with `plan` would write).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct QmlExported {
    pub path: String,
    pub layer: String,
    /// Classes and rules written.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rules: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    /// What the QML could not say, a sentence each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub lost: Vec<String>,
}

/// Input of `file.import.qml` v1: the look of the QML at `path` (or its `text`) made the layer's renderer, one undo
/// step “Katman stili”: QGIS's renderer as KentOS's of the same kind, rule filters in KentOS's expressions. Refusals,
/// in order:
/// `invalid_input` (neither or both of `path` and `text`), `file_unreadable`, `invalid_qml`, `layer_not_found`,
/// `not_a_layer`, `service_layer`, `revision_conflict`. Warnings: `import_losses`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportQml {
    /// The layer's id.
    pub layer: String,
    /// An absolute path to a `.qml`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// The QML itself, instead of a path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = crate::REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What `file.import.qml` made the layer's look.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct QmlImported {
    pub layer: String,
    /// The renderer's type: `single`, `categorized`, `graduated` or `rules`.
    pub kind: String,
    /// Classes and rules taken.
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
