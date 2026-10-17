//! `cad.tables.edit` v1 (docs/adr/0327): a table's cells written, its rows and columns inserted, deleted and sized,
//! ranges merged and styled, formulas and fields set; a table written again from its source (Tabloyu güncelle), linked
//! to a file or let go of it.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::Entity;
use crate::table::TableAlign;
use crate::table_cells::SourceOverrides;
use crate::text_field::{FieldFormat, FieldSource};

pub const CAD_TABLES_EDIT: &str = "cad.tables.edit";
pub const CAD_TABLES_EDIT_VERSION: u32 = 1;

/// One edit of a table's cells, rows and columns, applied in order (Tabloyu düzenle).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TableEdit {
    /// The cell's words (one line); words beginning with `=` are its formula, its words then its result. A cell
    /// inside a merged range is its top left's.
    SetCell {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        row: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        col: u32,
        words: String,
    },
    /// `count` empty rows before row `at` (the number of rows: after the last); merged ranges, styles, formulas'
    /// references and fields move with the rows.
    InsertRows {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        at: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        count: u32,
    },
    /// `count` rows from row `from` taken away; a formula reading one of them reads `#BAŞV!`.
    DeleteRows {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        from: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        count: u32,
    },
    InsertColumns {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        at: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        count: u32,
    },
    DeleteColumns {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        from: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        count: u32,
    },
    /// The range merged: its words its cells', the empty ones left out, joined by a space; ranges inside it absorbed.
    Merge {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        row: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        col: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        rows: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        cols: u32,
    },
    /// The merged range the cell is in let go of.
    Unmerge {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        row: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        col: u32,
    },
    /// A style over the range: what it gives (at least one), over the styles before it.
    Style {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        row: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        col: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        rows: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        cols: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        align: Option<TableAlign>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        bold: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        color: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        fill: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        decimals: Option<u32>,
    },
    /// The styles that meet the range taken away, whole.
    ClearStyle {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        row: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        col: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        rows: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        cols: u32,
    },
    /// The cell bound to `source` (its formula, if any, taken away): its words the value.
    Field {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        row: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        col: u32,
        source: FieldSource,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        format: Option<FieldFormat>,
    },
    /// The cell's field let go of: its words stay.
    Unfield {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        row: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        col: u32,
    },
    /// `count` rows from `from` made `height` metres high.
    RowHeight {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        from: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        count: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        height: f64,
    },
    /// `count` columns from `from` made `width` metres wide.
    ColumnWidth {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        from: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        count: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        width: f64,
    },
    /// Sığdır: `count` columns from `from` as wide as their words ask, in the drawing's typeface; with `grow` only
    /// those narrower get wider; with `rows` every row `ROW` text heights high too (Yazıya sığdır).
    Fit {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        from: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        count: u32,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        grow: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        rows: bool,
    },
    /// `count` columns from `from` aligned so (the column's, under the cells' styles).
    ColumnAlign {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        from: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        count: u32,
        align: TableAlign,
    },
    /// The first row the heading (bold, centred) or not.
    Header { on: bool },
    /// Which lines are drawn (null: all).
    Grid { grid: Option<crate::table::TableGrid> },
    /// Kalın çerçeve's width, metres (null: a line as the others).
    Frame {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        width: Option<f64>,
    },
    /// Kaynağı bırak: the table follows no source.
    Unlink,
}

/// What `cad.tables.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TablesOperation {
    /// Tabloyu düzenle: the edits in order, as one step.
    Edit {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        edits: Vec<TableEdit>,
    },
    /// Tabloyu güncelle: each table written again from its source (a schedule from its objects, a linked file
    /// from its path, read from `base`, the drawing's folder, when relative); a block schedule is the document's.
    Refresh {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        base: Option<String>,
    },
    /// Dosyaya bağla: the table's cells from the file at `path` (its sheet `sheet`, the first when none) and the table
    /// linked to it: its path, digest and copy kept, `overrides` its rule.
    Link {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        sheet: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        base: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        overrides: Option<SourceOverrides>,
    },
    /// Kaynağı bırak: the tables keep their cells and follow no source.
    Unlink {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
}

/// Input of `cad.tables.edit` v1. Refusals: `invalid_uid`, `entity_not_found`, `not_a_table`, `layer_locked`,
/// `no_entities`, `cell_out_of_range`, `invalid_table`, `not_finite`, `no_source`, `source_missing`,
/// `source_unreadable`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TablesEdit {
    pub operation: TablesOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What Tabloyu güncelle did with a table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TableRefresh {
    /// Written again.
    Updated,
    /// Its source gives what it has (a file's same bytes).
    Unchanged,
    /// Its file is not where its path says: it stays as it was.
    Missing,
    /// Its file cannot be read as a table: it stays as it was.
    Unreadable,
    /// It names a file but is linked to none: choose the file (Dosyaya bağla).
    Unlinked,
    /// A block schedule: the document keeps it at every step.
    Live,
}

/// A table Tabloyu güncelle looked at.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TableReport {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    pub status: TableRefresh,
    /// Cells changed by hand kept (their file's cell as it was).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub kept: u32,
    /// Cells changed by hand the file's words were written over (it changed there too, or the rule is replace).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub replaced: u32,
}

/// Output of `cad.tables.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TablesEdited {
    /// The tables that changed, by their persistent ids.
    pub changed: Vec<String>,
    /// What Tabloyu güncelle and Dosyaya bağla did with each table.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TableReport>>", optional))]
    pub report: Vec<TableReport>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.tables.edit` would write (plan mode).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TablesPlan {
    /// The tables as execute would write them.
    pub tables: Vec<Entity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TableReport>>", optional))]
    pub report: Vec<TableReport>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
