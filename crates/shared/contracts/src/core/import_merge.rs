//! A file's rows taken into a layer that holds objects already (docs/adr/0628 §5, §6): matched by a key column (or
//! by their lasting ids), the matched objects updated, the others added or left, a key a source repeats handled as
//! asked, and every row refused written with its reason to a CSV the institution can mend and load again.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// What a matched object takes from its row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MergeMatched {
    /// The row's attributes; a cell empty in the row empties the object's.
    #[default]
    Update,
    /// Only the attributes empty on the object.
    FillEmpty,
    /// Nothing: the object stays as it is.
    Keep,
}

/// What a row no object matches does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MergeUnmatched {
    #[default]
    Add,
    Skip,
}

/// What a key the source gives more than one row does (docs/adr/0439 §4's words).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MergeDuplicates {
    /// Nothing is taken: `duplicate_key`.
    #[default]
    Stop,
    /// The first row is taken, the others refused.
    First,
    /// No row of the key is taken.
    None,
}

fn yes() -> bool {
    true
}

fn is_true(v: &bool) -> bool {
    *v
}

/// How the rows meet the objects of the layer they go onto. A row goes onto a layer of the drawing as the import
/// routes it; on a layer the import makes every row is added.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportMerge {
    /// The source's column naming a row; none: the row's lasting id (taking the same file again updates).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub key: Option<String>,
    /// The layer's field (or attribute) the key meets; none: the one named as `key`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub target_key: Option<String>,
    #[serde(default)]
    pub matched: MergeMatched,
    /// Whether a matched object takes its row's geometry.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub geometry: bool,
    #[serde(default)]
    pub unmatched: MergeUnmatched,
    #[serde(default)]
    pub duplicates: MergeDuplicates,
}

impl Default for ImportMerge {
    fn default() -> Self {
        ImportMerge {
            key: None,
            target_key: None,
            matched: MergeMatched::Update,
            geometry: true,
            unmatched: MergeUnmatched::Add,
            duplicates: MergeDuplicates::Stop,
        }
    }
}

/// What the merge did on one layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MergeReport {
    /// The file's table (or layer) and the drawing's layer its rows went onto.
    pub table: String,
    pub layer: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub updated: u32,
    /// Matched objects the row changed nothing of.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unchanged: u32,
    /// Rows no object matched, left by `unmatched: skip`.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rejected: u32,
    /// Why rows were refused (the reason codes below), with how many.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub reasons: BTreeMap<String, u32>,
}

/// The CSV of the rows refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RejectsWritten {
    pub path: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
}

/// The reasons a row is refused, as `MergeReport.reasons` and the rejects file's `neden` column name them.
pub const REJECT_KEY_EMPTY: &str = "key_empty";
pub const REJECT_KEY_DUPLICATE: &str = "key_duplicate";
pub const REJECT_TARGET_AMBIGUOUS: &str = "target_ambiguous";
pub const REJECT_FIELD_VALUE: &str = "field_value";
pub const REJECT_GEOMETRY_KIND: &str = "geometry_kind";
pub const REJECT_LAYER_TYPE: &str = "layer_type";
pub const REJECT_UID_ELSEWHERE: &str = "uid_elsewhere";

/// A reason's words.
pub fn reject_words(code: &str) -> &'static str {
    match code {
        REJECT_KEY_EMPTY => "anahtarı boş",
        REJECT_KEY_DUPLICATE => "anahtarı kaynakta yineleniyor",
        REJECT_TARGET_AMBIGUOUS => "anahtarı katmanda birden çok nesnede",
        REJECT_FIELD_VALUE => "değeri alanına uymuyor",
        REJECT_GEOMETRY_KIND => "geometrisi eşleşen nesneninkiyle aynı türde değil",
        REJECT_LAYER_TYPE => "geometrisi katmanın türüne uymuyor",
        REJECT_UID_ELSEWHERE => "kalıcı kimliği başka bir katmanın nesnesinde",
        _ => "alınmadı",
    }
}
