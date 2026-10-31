//! The design option commands (docs/adr/0788, ADV-05c): a set of alternatives defined from the objects of one part of
//! the plan, an option copied as an independent alternative, objects moved into and out of an option, another option
//! made primary, an option accepted (the others deleted) or deleted, names changed (`cad.options.edit`); and the sets
//! read with each option's counts and areas, an object's option, and two options compared (`cad.options.query`,
//! read-only). Every write is one undo step “Tasarım seçeneği”; its plan is the preview. There is no automatic merge:
//! an option is accepted whole or not at all.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::OptionSet;

pub const CAD_OPTIONS_EDIT: &str = "cad.options.edit";
pub const CAD_OPTIONS_EDIT_VERSION: u32 = 1;
pub const CAD_OPTIONS_QUERY: &str = "cad.options.query";
pub const CAD_OPTIONS_QUERY_VERSION: u32 = 1;

/// What `cad.options.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OptionsOperation {
    /// Seçenek kümesi tanımla: the objects `uids` (in no option) become the set `name`'s first option (`option`,
    /// absent: “A”), its primary; they stay where they are. A main-model object leaning on one of them (an opening on
    /// a wall, a tag on a space) or one of them leaning on another option's object refuses it.
    Define {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        option: Option<String>,
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
    /// Seçeneği kopyala: a new option of `set`, an independent copy of the option `option`'s objects in their places
    /// (their references to each other made the copies'), named `name` (absent: the next letter); primary when asked.
    Copy {
        set: String,
        option: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        primary: bool,
    },
    /// Seçeneğe ekle: the main model's objects `uids` join the option.
    AddMembers {
        set: String,
        option: String,
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
    /// Seçenekten çıkar: the objects `uids` (of the set's options) go back to the main model.
    RemoveMembers {
        set: String,
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
    /// Birincil yap: the drawing shows `option` of the set.
    Primary { set: String, option: String },
    /// Seçeneği kabul et: `option` (absent: the primary) becomes the main model; the set's other options' objects are
    /// deleted and the set goes. An object on a locked layer refuses it.
    Accept {
        set: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        option: Option<String>,
    },
    /// Seçeneği sil: a non-primary option goes with its objects.
    Delete { set: String, option: String },
    /// Yeniden adlandır: the set (no `option`) or one of its options.
    Rename {
        set: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        option: Option<String>,
        name: String,
    },
}

/// Input of `cad.options.edit` v1.
///
/// Refusals: `invalid_input`, `no_entities`, `invalid_uid`, `entity_not_found`, `layer_locked`, `option_set_exists`,
/// `option_set_not_found`, `option_not_found`, `option_exists`, `option_member_taken`, `not_in_option`,
/// `option_leaning`, `option_primary`, `option_set_full`, `nothing_to_set`, `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OptionsEdit {
    pub operation: OptionsOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.options.edit` (and its plan): the set after, the option made or named, the objects made, deleted
/// and given back to the main model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OptionsEdited {
    /// The set after; none once accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub set: Option<OptionSet>,
    /// The option made (Tanımla, Kopyala) or the one the operation named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub option: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub created: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub deleted: Vec<String>,
    /// The objects that are the main model's now (Kabul et, Seçenekten çıkar).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub released: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Two options of one set to compare (docs/adr/0788 §2): `a` the one compared against, `b` the other; objects paired
/// by place as Veri karşılaştır pairs them (docs/adr/0179), within `search` (absent: 0.5 m in the drawing's unit) and
/// the same within `tolerance` (absent: 1 mm).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OptionsCompare {
    pub set: String,
    pub a: String,
    pub b: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub search: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
}

/// Input of `cad.options.query` v1 (read-only): the sets (one: `set`), the option an object (`uid`) is in, and two
/// options compared.
///
/// Refusals: `option_set_not_found`, `option_not_found`, `invalid_uid`, `entity_not_found`, `not_in_option`,
/// `invalid_input`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OptionsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub set: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub compare: Option<OptionsCompare>,
}

/// How many objects of a kind an option holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OptionKindCount {
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
}

/// An option as `cad.options.query` reads it: its objects counted by kind, its spaces' area and its walls' length.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OptionRow {
    pub id: String,
    pub name: String,
    pub primary: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// Its objects gone from the drawing.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub missing: u32,
    pub kinds: Vec<OptionKindCount>,
    /// Its spaces' areas summed, in the drawing's unit squared.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub space_area: f64,
    /// Its walls' base lines' lengths summed.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub wall_length: f64,
}

/// A set as `cad.options.query` reads it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OptionSetRow {
    pub id: String,
    pub name: String,
    pub primary: String,
    pub options: Vec<OptionRow>,
}

/// A pair of objects found in both options, changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OptionChange {
    pub a: String,
    pub b: String,
    /// What differs: `geometry`, `attributes` or both.
    pub what: String,
}

/// Two options compared: the objects paired as the same, those only in `b` (added) and only in `a` (removed), the
/// pairs changed, and the differences in counts and areas (b − a).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OptionsCompared {
    pub set: String,
    pub a: String,
    pub b: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub same: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub added: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub removed: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<OptionChange>>", optional))]
    pub changed: Vec<OptionChange>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub space_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub wall_length: f64,
    /// Each kind's count in `a` and in `b`, where they differ.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<OptionKindDifference>>", optional))]
    pub kinds: Vec<OptionKindDifference>,
}

/// A kind counted in both options of a comparison.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OptionKindDifference {
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub a: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub b: u32,
}

/// Output of `cad.options.query`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OptionsQueried {
    pub sets: Vec<OptionSetRow>,
    /// With `uid`: its set and option.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub set: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub option: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub compared: Option<OptionsCompared>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
