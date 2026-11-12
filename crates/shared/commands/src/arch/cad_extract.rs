//! Veri çıkarımı (docs/adr/1030, plan/cad-advanced.md `CAD-ADV-13`): `cad.extract.run` v1 works a recipe out and
//! writes nothing; `cad.extract.table` v1 writes it as a table that keeps its recipe (one undo step “Veri çıkarımı
//! tablosu”); Tabloyu güncelle (`cad.tables.edit` `refresh`) writes such a table again from its recipe, keeping the
//! cells changed by hand and the user's own columns by each row's key.
//!
//! Other drawings and the joined file are read only: their bytes are read and digested, never written, saved or
//! changed (the open drawing is not changed by `run` either).
//!
//! Refusals, checked in this order: `invalid_input` (neither or both of `recipe` and `table`), `invalid_extraction_recipe`
//! (the recipe's rules, at its field), `invalid_uid`, `entity_not_found`, `not_a_table`, `not_an_extraction`
//! (a table without a recipe), `drawing_read_twice` (a drawing named that is the open drawing's own file),
//! `extraction_source_missing` (a drawing or the joined file not where its path says), `extraction_source_unreadable` (not a drawing,
//! not a table), `join_unusable`,
//! `extraction_too_large`, `extraction_empty` (`cad.extract.table`: no row to write), `layer_locked`,
//! `invalid_table` (the table it would write breaks a table's rules: too many rows), `not_finite`,
//! `invalid_revision`, `revision_conflict`.
//!
//! Warnings: `extraction_hidden` (hidden parts left out), `xref_unloaded`, `xref_read_apart` (a reference to a
//! drawing the recipe reads itself, counted once as that drawing), `join_unmatched` (rows whose key the joined
//! table lacks), `dimension_overrides` (dimensions whose text overrides their measure: words, not summed),
//! `extraction_too_deep`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::Vec2;
use crate::extraction::{ExtractRead, ExtractionRecipe};
use crate::table_cells::SourceOverrides;

pub const CAD_EXTRACT_RUN: &str = "cad.extract.run";
pub const CAD_EXTRACT_RUN_VERSION: u32 = 1;
pub const CAD_EXTRACT_TABLE: &str = "cad.extract.table";
pub const CAD_EXTRACT_TABLE_VERSION: u32 = 1;

/// Input of `cad.extract.run` v1: a recipe, or an extraction table whose recipe is worked out (and said whether it
/// is current).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExtractRun {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub recipe: Option<ExtractionRecipe>,
    /// An extraction table by its persistent id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub table: Option<String>,
    /// The folder relative paths are read from (the drawing's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub base: Option<String>,
    /// The open drawing's own file: a recipe naming it is refused (it would be counted twice).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// One row worked out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExtractRow {
    /// Its key: a grouped row's word cells joined by U+001F, an ungrouped row's occurrence path.
    pub key: String,
    pub cells: Vec<String>,
    /// Each column's unrounded value in its unit (a quantity's); null for words or an empty cell.
    #[cfg_attr(feature = "ts", ts(type = "Array<number | null>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub values: Vec<Option<f64>>,
    /// Its occurrences' paths: the drawing (its path as the recipe names it, empty for the open one), `#`, the
    /// object's persistent id, each nested piece's local id (`/12`, an external reference's piece `/x:<id>`); an
    /// array counted whole `#dizi:<id>`.
    pub members: Vec<String>,
}

/// What an extraction left out or could not match.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExtractNotes {
    /// Hidden pieces and objects on hidden or frozen layers left out.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub hidden: u32,
    /// Of those, the pieces a dynamic block's visibility state hides (an array action's copies of them too).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub hidden_states: u32,
    /// External references not loaded, by block name.
    pub unloaded: Vec<String>,
    /// External references not looked into: the recipe reads their drawing itself.
    pub read_apart: Vec<String>,
    /// Occurrences reached twice and counted once.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub repeated: u32,
    /// Dimensions whose measure is an angle (no length).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub angles: u32,
    /// Dimensions whose text overrides their measure.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub overrides: u32,
    /// Join keys the joined table lacks.
    pub unmatched: Vec<String>,
    /// The joined table's keys no row used.
    pub unused: Vec<String>,
    /// Nested blocks past the deepest level not looked into.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub too_deep: u32,
}

/// Output of `cad.extract.run` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExtractRan {
    pub headings: Vec<String>,
    /// Which columns are quantities.
    pub quantities: Vec<bool>,
    pub rows: Vec<ExtractRow>,
    /// The Toplam row's cells, when the recipe totals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub total: Option<Vec<String>>,
    /// Its quantity columns' unrounded sums.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "Array<number | null>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub totals: Option<Vec<Option<f64>>>,
    /// How many occurrences were counted.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub occurrences: u32,
    /// The other drawings and the joined file as read now.
    pub read: Vec<ExtractRead>,
    pub notes: ExtractNotes,
    /// With `table`: whether Tabloyu güncelle would change its extracted cells (its sources changed since).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stale: Option<bool>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.extract.table` v1: a recipe written as a table at `at` (its top left corner), keeping the recipe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExtractTable {
    pub recipe: ExtractionRecipe,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// Its layer; absent: the active one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer_id: Option<String>,
    /// Its words' height, metres; absent: the project's table height at its plot scale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: Option<f64>,
    /// Its parts (Tabloyu böl, docs/adr/1030 §7): each at most `breaks.height` deep, the heading row atop each;
    /// absent: one part.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub breaks: Option<crate::table::TableBreaks>,
    /// What Tabloyu güncelle does with a cell changed by hand whose source changed too; absent: keep (refuse the
    /// refresh, saying so).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub overrides: Option<SourceOverrides>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub base: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.extract.table` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExtractTabled {
    /// The table's persistent id.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// Its data rows.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub occurrences: u32,
    pub notes: ExtractNotes,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.extract.table` would write (plan mode).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExtractTablePlan {
    pub table: crate::entity::Entity,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub occurrences: u32,
    pub notes: ExtractNotes,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
