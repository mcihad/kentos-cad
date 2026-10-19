//! `cad.joins.edit` and `cad.joins.query` (docs/adr/0527): the project's join views kept and read. A view is
//! a setting; its rows are the drawing's when read, each with the records it came from, so a write goes to the
//! record a cell belongs to (`cad.entities.set`) and the cells that cannot be written say why.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{JoinKeep, JoinView};

pub const CAD_JOINS_EDIT: &str = "cad.joins.edit";
pub const CAD_JOINS_EDIT_VERSION: u32 = 1;
pub const CAD_JOINS_QUERY: &str = "cad.joins.query";
pub const CAD_JOINS_QUERY_VERSION: u32 = 1;

/// The most rows a page gives, and how many when none is said.
pub const MOST_JOIN_ROWS: u32 = 1000;
pub const JOIN_ROWS: u32 = 100;

/// What `cad.joins.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum JoinsOp {
    /// A new view.
    Create { view: JoinView },
    /// A new view of a relation: its parent layer by `$uid`, its child layer by the relation's field.
    FromRelation {
        relation: String,
        id: String,
        name: String,
        #[serde(default, skip_serializing_if = "JoinKeep::is_default")]
        #[cfg_attr(feature = "ts", ts(as = "Option<JoinKeep>", optional))]
        keep: JoinKeep,
    },
    /// The view of the same id replaced.
    Update { view: JoinView },
    /// The view removed; the records stay as they are.
    Remove { id: String },
}

/// Input of `cad.joins.edit` v1.
///
/// Refusals, in this order: `invalid_join_view`; `invalid_revision`, `revision_conflict` (status `conflict`);
/// `unknown_join_view`, `join_view_exists`, `too_many_join_views`, `unknown_relation`, `layer_not_found`,
/// `not_a_layer`, `field_not_found`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JoinsEdit {
    pub edit: JoinsOp,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.joins.edit` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JoinsEdited {
    /// The project's views afterwards, in order.
    pub views: Vec<JoinView>,
    pub changed: bool,
    /// The document's revision afterwards (plan: the one it was made against).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.joins.query` v1: a kept view by its id, or a definition read as it is (a preview).
///
/// Refusals: `invalid_join_view` (neither or both, or a definition that does not hold), `unknown_join_view`,
/// `invalid_count` (a limit not 1–1000), `invalid_cursor`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JoinsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub view: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub definition: Option<JoinView>,
    /// The page after the one whose `next` this is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cursor: Option<String>,
    /// Rows a page gives, 1–1000; none: 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub limit: Option<u32>,
}

/// Which records a column's cells are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum JoinSide {
    /// The main layer's record (the row's `left`).
    Left,
    /// The joined layer's record (the row's `right`).
    Right,
}

/// A column of a view.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JoinColumn {
    /// The heading: the field's name; a joined one the main layer has too with “ (layer)”.
    pub name: String,
    pub side: JoinSide,
    /// The attribute the cell is (the field's name in the record's attributes).
    pub field: String,
    /// Whether a cell of a row with its record can be written (with `cad.entities.set` on that record).
    pub editable: bool,
    /// Why not, when not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reason: Option<String>,
}

/// A row: a record of the main layer and one of its matches (none when it has none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JoinRow {
    /// `left/right` of the records' persistent ids (`left/` without a match): the same row whenever read.
    pub id: String,
    pub left: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub right: Option<String>,
    /// The columns' values in order; none: no value (an attribute not written, or no match).
    pub cells: Vec<Option<String>>,
}

/// How the records met.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JoinReport {
    /// The main layer's records, those with a match, without one, and without a key.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub records: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub matched: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unmatched: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub null_keys: u32,
    /// The joined layer's records and those some record met.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub with_records: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub with_used: u32,
    /// Every row of the view (all pages).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    /// `1:1`, `1:N` (a record met several), `N:1` (a joined record met by several), `N:N`; empty: no match.
    pub cardinality: String,
}

/// A column's sum over its records, each once however many rows it is in (`kentos.statistics/1`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JoinTotal {
    /// The column's place.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub column: u32,
    /// The exact sum as text; none when too large to hold exactly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sum: Option<String>,
    /// The records whose value was a number, and those whose value was not.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
}

/// Output of `cad.joins.query` v1, and its plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JoinsAnswer {
    pub name: String,
    /// Why the view cannot be read now (its layer or key is gone): no rows, nothing of a former reading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<String>,
    pub columns: Vec<JoinColumn>,
    pub rows: Vec<JoinRow>,
    /// The next page's cursor; none: the last page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub next: Option<String>,
    pub report: JoinReport,
    /// The sums of the columns with a number among their records' values, in column order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<JoinTotal>>", optional))]
    pub totals: Vec<JoinTotal>,
    /// The document's revision the rows are of.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}
