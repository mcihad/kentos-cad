//! The commands that read files into the drawing (docs/adr/0257 §3): a DXF
//! taken in as the desktop's DXF içe aktar takes it, its unit decided by one
//! rule (§1), its layers going to the drawing's layers of the same name or to
//! new ones, its blocks and styles with them, as one undo step named after
//! the file. The desktop's window, the headless host, Python and MCP run the
//! same handler (`kentos_native_application::file_import`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::formats::{FileUnit, UnitDecision};

/// A DXF file into the drawing.
pub const FILE_IMPORT_DXF: &str = "file.import.dxf";
pub const FILE_IMPORT_DXF_VERSION: u32 = 1;

/// Input of `file.import.dxf` v1. Refusals (`CommandError.code`), in order:
/// `invalid_file_path` (empty, relative, a directory), `invalid_revision`,
/// `revision_conflict`, `file_not_found`, `import_unreadable` (not a DXF, or
/// broken), `layer_not_found` and `not_a_layer` (a target of `layers`, at
/// `layers.<name>`), `layer_locked` (a target the objects cannot go onto),
/// `nothing_to_import` (no object left to take). Warnings: `unit_assumed`,
/// `unit_ignored`, `block_units_differ`, `import_losses`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportDxf {
    /// The file to read, an absolute path.
    pub path: String,
    /// The unit the file's values are in, whatever it declares (docs/adr/0257
    /// §1). Absent: a local project takes the unit the file declares, a
    /// project with a coordinate system metres; a file that declares none is
    /// in the project's unit, its numbers kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file_unit: Option<FileUnit>,
    /// Blokları patlat: every insert opened into its objects, no block
    /// definitions taken (docs/adr/0144 §5).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub explode_blocks: bool,
    /// The file's layers (by their names in the file) whose objects are left out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub skip_layers: Vec<String>,
    /// A file's layer (its name in the file) → the drawing's layer (its id)
    /// its objects go onto. A layer not named here goes onto the drawing's
    /// layer of the same name (case and Turkish marks aside), else onto a new
    /// layer with the file's look.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub layers: BTreeMap<String, String>,
    /// The group at the top of the tree the new layers go into, found by its
    /// name or made. Absent: one named after the file; empty: the top of the tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    /// The drawing's revision the input was prepared against.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// A layer an import made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportedLayer {
    pub id: String,
    pub name: String,
}

/// A block whose name the drawing had: as the file names it, as it went in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RenamedBlock {
    pub from: String,
    pub to: String,
}

/// What `file.import.dxf` took in, or with `plan` would take.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DxfImported {
    /// The file's name, its bytes and their SHA-256 (64 hex digits): what
    /// was read, so a replay can tell the same file.
    pub file: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    /// The unit the file's lengths were read in and why (docs/adr/0257 §1).
    pub unit: UnitDecision,
    /// The objects taken.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// The file's layer (its name in the file) → the drawing's layer its
    /// objects went onto; the layers left out are not here.
    pub targets: BTreeMap<String, String>,
    /// The layers made, in the tree's order, and the group they went into.
    pub layers: Vec<ImportedLayer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    /// The block definitions taken, and the names changed on the way: as
    /// the file says, as the drawing has it (“Kapı” → “Kapı (2)”).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub blocks: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<RenamedBlock>>", optional))]
    pub renamed: Vec<RenamedBlock>,
    /// The text and dimension styles added to the project (docs/adr/0183 §7).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub text_styles: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub dimension_styles: u32,
    /// The reader's report: the file's facts, what was skipped and why.
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub report: serde_json::Value,
    /// The drawing's revision after the step (plan: before it).
    pub revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The input reads with only its path; what it leaves out is the default.
    #[test]
    fn the_least_input_is_the_path() {
        let input: ImportDxf =
            serde_json::from_value(json!({ "path": "/plan/oda.dxf" })).expect("reads");
        assert_eq!(
            (
                input.file_unit,
                input.explode_blocks,
                input.skip_layers.len(),
                input.layers.len(),
                input.group.clone(),
            ),
            (None, false, 0, 0, None)
        );
        assert_eq!(
            serde_json::to_value(&input).expect("writes"),
            json!({ "path": "/plan/oda.dxf" })
        );
        let unit: ImportDxf =
            serde_json::from_value(json!({ "path": "/a.dxf", "fileUnit": "usFt", "group": "" }))
                .expect("reads");
        assert_eq!(
            (unit.file_unit, unit.group.as_deref()),
            (Some(FileUnit::UsFt), Some(""))
        );
    }
}
