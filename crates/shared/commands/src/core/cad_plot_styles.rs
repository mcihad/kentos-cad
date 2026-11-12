//! `cad.plotstyles.import` v1 (“Baskı stili tablosu içe aktar”, docs/adr/1130): an AutoCAD `.ctb` (colour-dependent,
//! 255 pens) or `.stb` (named) plot style table read into a plot profile of the project's sheet book, every field mapped
//! explicitly and what KentOS plots otherwise reported (lost, approximate, note); the profile keeps the file's format,
//! SHA-256 and name. Optionally the pens are assigned to layers or paper items and the profile is applied to sheets, so
//! publishing plots with it. One change of the book, one undo, as `cad.sheets.edit`; the file is only read.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_PLOTSTYLES_IMPORT: &str = "cad.plotstyles.import";
pub const CAD_PLOTSTYLES_IMPORT_VERSION: u32 = 1;

/// Input of `cad.plotstyles.import` v1.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `invalid_input` (an empty path or name),
/// `file_not_found`, `file_unreadable`, then the reader's (`plot_style_file_too_large`, `unsupported_plot_style_file`,
/// `bad_plot_style_file`, `plot_style_encoding`, `plot_style_kind_mismatch`, `duplicate_plot_style`,
/// `plot_profile_too_large`), `book_unreadable`, `duplicate_plot_profile` (a profile of that name, or the same file
/// already imported), `layer_not_found` / `unknown_item` (an assignment's target), `bad_plot_assignment` (a target
/// twice, or a pen or style the table lacks), `unknown_sheet`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlotStylesImport {
    /// The `.ctb` or `.stb` file: as given when absolute, else from `base`.
    pub path: String,
    /// The folder a relative path is read from (the drawing's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub base: Option<String>,
    /// The profile's name; absent: the file's name without its extension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// Pens or styles given to model layers (`{target:{type:"modelLayer", id}, key:{type:"aci", index}}`) or paper
    /// items; a CTB pen is never guessed from a layer's RGB colour (docs/adr/0609).
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "Array<Record<string, unknown>>"))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Vec<kentos_sheet::plot_profile::Assignment>")
    )]
    pub assignments: Vec<serde_json::Value>,
    /// The sheets the profile is applied to (their plot settings name it and hold its copy), by id.
    #[serde(default)]
    pub sheets: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.plotstyles.import` v1 (its plan's too): the book change, the profile and what the reading said.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlotStylesImported {
    #[serde(flatten)]
    pub edited: crate::SheetsEdited,
    /// The profile written to the book, with its source.
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "kentos_sheet::plot_profile::PlotProfile")
    )]
    pub profile: serde_json::Value,
    /// `ctb` or `stb`.
    pub format: String,
    /// The table's own description.
    pub description: String,
    /// Styles in the file (a CTB's 255 pens; a STB's, Normal among them).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub styles: u32,
    /// Styles carried without a lost or approximate field.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub exact: u32,
    /// What KentOS plots otherwise, by kind: its severity, words, the styles it touches and the file's values.
    #[cfg_attr(feature = "ts", ts(type = "Array<Record<string, unknown>>"))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Vec<kentos_sheet::plot_profile::acad::Loss>")
    )]
    pub losses: Vec<serde_json::Value>,
    /// The sheets the profile was applied to.
    pub applied: Vec<String>,
}
