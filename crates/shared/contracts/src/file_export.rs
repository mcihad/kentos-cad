//! The commands that write files (docs/adr/0253 §10): the drawing as DXF
//! and the sheet book as PDF. They write no document, so they are no undo
//! step, and undo does not take a file back. A file is written whole or not
//! at all: under a temporary name, made durable, read back and compared,
//! then put in place in one rename; an old file goes only once the new one
//! is whole. The headless host runs them (`kentos_headless`): they need the
//! file formats and the sheet core, which the document's commands do not.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The drawing as an AutoCAD 2007 DXF file.
pub const FILE_EXPORT_DXF: &str = "file.export.dxf";
pub const FILE_EXPORT_DXF_VERSION: u32 = 1;
/// The project's sheet book as a PDF.
pub const FILE_EXPORT_PDF: &str = "file.export.pdf";
pub const FILE_EXPORT_PDF_VERSION: u32 = 1;

/// A block, or objects, as a drawing file of its own (WBLOCK, docs/adr/0316 §3).
pub const FILE_EXPORT_BLOCK: &str = "file.export.block";
pub const FILE_EXPORT_BLOCK_VERSION: u32 = 1;

/// The commands that write files, by name and version: the headless host
/// runs them beside the document's (`DESKTOP_COMMANDS`).
pub const FILE_COMMANDS: &[(&str, u32)] = &[
    (FILE_EXPORT_DXF, FILE_EXPORT_DXF_VERSION),
    (FILE_EXPORT_PDF, FILE_EXPORT_PDF_VERSION),
    (FILE_EXPORT_BLOCK, FILE_EXPORT_BLOCK_VERSION),
    (crate::FILE_EXPORT_GPKG, crate::FILE_EXPORT_GPKG_VERSION),
    (crate::FILE_EXPORT_SHP, crate::FILE_EXPORT_SHP_VERSION),
    (crate::FILE_EXPORT_SLD, crate::FILE_EXPORT_SLD_VERSION),
    (crate::FILE_EXPORT_CSV, crate::FILE_EXPORT_CSV_VERSION),
    (crate::FILE_EXPORT_GIS, crate::FILE_EXPORT_GIS_VERSION),
    (crate::FILE_PACKAGE, crate::FILE_PACKAGE_VERSION),
];

/// Input of `file.export.block` v1 (WBLOCK, docs/adr/0316 §3): a block's objects, or the objects named by their
/// persistent ids, as a new `.kcad` drawing with the layers, blocks, styles, library items and settings they use;
/// `base` (a block's own base point by default) goes to the drawing's origin. Refusals, in order:
/// `invalid_file_path`, `file_exists`, `no_block` (neither `block` nor `uids`), `unknown_block`, `invalid_uid`,
/// `entity_not_found`, `nothing_to_export` (a block without objects), `not_finite` (`base`), `file_unwritable`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportBlock {
    /// The file to write, an absolute path (`.kcad`).
    pub path: String,
    /// The block, by id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub block: Option<crate::identity::BlockId>,
    /// Or the objects, by persistent id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    /// The point that goes to the drawing's origin; a block's base point by default, none for objects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub base: Option<crate::entity::Vec2>,
    /// Write over a file that is there.
    #[serde(default)]
    pub overwrite: bool,
}

/// What `file.export.block` wrote, or with `plan` would write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlockFileExported {
    pub path: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    /// SHA-256 of the bytes, 64 hex digits.
    pub sha256: String,
    /// The drawing's objects, and the blocks it carries.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub blocks: u32,
    /// Whether a file was there (and is written over).
    pub replaced: bool,
}

/// Input of `file.export.dxf` v1. Refusals (`CommandError.code`), in order:
/// `invalid_file_path` (empty, relative, a directory, no directory to go
/// into), `file_exists` (there is a file and `overwrite` is not set),
/// `layer_not_found` (a layer named the drawing does not have, at
/// `layers[i]`), `nothing_to_export` (no object to write), `file_unwritable`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportDxf {
    /// The file to write, an absolute path.
    pub path: String,
    /// The layers whose objects to write (ids); every object when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<Vec<String>>,
    /// Write over a file that is there.
    #[serde(default)]
    pub overwrite: bool,
    /// How tables, dimensions and leaders are written (docs/adr/0327 §6); absent: native entities with KentOS's data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub annotations: Option<crate::DxfAnnotations>,
}

/// What `file.export.dxf` wrote, or with `plan` would write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DxfExported {
    pub path: String,
    /// The file's bytes: written, or the size the plan would write.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    /// SHA-256 of the bytes, 64 hex digits.
    pub sha256: String,
    /// The objects written and the layers they are on.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub layers: u32,
    /// Whether a file was there (and is written over).
    pub replaced: bool,
    /// The writer's report: what each kind became, what changed on the way.
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub report: serde_json::Value,
}

/// Input of `file.export.pdf` v1. Refusals: `invalid_file_path`,
/// `file_exists`, `no_sheets` (the project has no sheet), `unknown_sheet`
/// (at `sheets[i]`), `book_unreadable`, `pdf_unwritable`, `file_unwritable`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportPdf {
    /// The file to write, an absolute path.
    pub path: String,
    /// The sheets to print, by id, a page each; every sheet in the book's order when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sheets: Option<Vec<String>>,
    /// Write over a file that is there.
    #[serde(default)]
    pub overwrite: bool,
}

/// What `file.export.pdf` wrote, or with `plan` would write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PdfExported {
    pub path: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    /// The sheets printed, by id, in page order.
    pub sheets: Vec<String>,
    pub replaced: bool,
    /// The sheet core's findings (preflight).
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub findings: serde_json::Value,
    /// What a host without a window cannot draw, said in words.
    pub notes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_inputs_read_with_their_defaults() {
        let dxf: ExportDxf =
            serde_json::from_value(json!({ "path": "/tmp/a.dxf" })).expect("reads");
        assert!(!dxf.overwrite);
        assert!(dxf.layers.is_none());
        let pdf: ExportPdf = serde_json::from_value(
            json!({ "path": "/tmp/a.pdf", "sheets": ["s1"], "overwrite": true }),
        )
        .expect("reads");
        assert!(pdf.overwrite);
        assert_eq!(pdf.sheets.as_deref(), Some(&["s1".to_owned()][..]));
    }
}
