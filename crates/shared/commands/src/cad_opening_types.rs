//! The door and window type commands (docs/adr/0772): the project's types made, changed, renamed, removed, taken from an
//! office library and written to one (`cad.openingtypes.edit`), and the openings read with their types and widths
//! (`cad.openings.query`, read-only). A type's change goes to every opening following it in the same undo step; one
//! keeping its own value of a field keeps it, one on a locked layer stays at the older version, one the change does not
//! fit is refused (all or none) or, asked, left behind. The report says which.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::{
    OpeningBuild, OpeningForm, OpeningOperation, OpeningSizeBy, OpeningSizes, OpeningType,
    OpeningTypeField,
};

pub const CAD_OPENING_TYPES_EDIT: &str = "cad.openingtypes.edit";
pub const CAD_OPENING_TYPES_EDIT_VERSION: u32 = 1;
pub const CAD_OPENINGS_QUERY: &str = "cad.openings.query";
pub const CAD_OPENINGS_QUERY_VERSION: u32 = 1;

/// A type's nominal size: the width it names and that width's value; the rough opening follows from it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningNominal {
    pub by: OpeningSizeBy,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub value: f64,
}

/// A type's fields to change; absent ones stay. A nullable field's null takes it away.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningTypeChange {
    /// The rough opening, metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: Option<f64>,
    /// Its nominal size instead of the rough opening: the rough opening follows from it, its fitting gap and frame
    /// (after them, when they change too), and `build.sizeBy` becomes the width it names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub nominal: Option<OpeningNominal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 2), extend("x-unit" = "count")))]
    pub leaves: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub operation: Option<OpeningOperation>,
    /// Degrees; null: 90.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>", extend("x-unit" = "deg")))]
    pub swing: Option<Option<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub show_swing: Option<bool>,
    /// The frame's width at each jamb, metres; null: none.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>", extend("x-unit" = "length")))]
    pub frame: Option<Option<f64>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>", extend("x-unit" = "elevation")))]
    pub sill: Option<Option<f64>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>", extend("x-unit" = "elevation")))]
    pub head: Option<Option<f64>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>", extend("x-unit" = "length")))]
    pub depth: Option<Option<f64>>,
    /// The construction's fields given (each set; the others stay).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub build: Option<OpeningBuild>,
    /// The construction's fields taken away.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<OpeningTypeField>>", optional))]
    pub unset: Vec<OpeningTypeField>,
}

impl OpeningTypeChange {
    pub fn is_empty(&self) -> bool {
        self == &OpeningTypeChange::default()
    }
}

/// When a type's change does not fit an opening following it (its wall gone, off its wall, into another).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningTypeRefused {
    /// Nothing is written (`opening_type_refused`).
    #[default]
    Stop,
    /// The others take it; those it does not fit stay at the older version and are reported.
    Skip,
}

/// What a type taken from an office library does when the project has one of its id or name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningTypeConflict {
    /// Refused (`opening_type_exists`).
    #[default]
    Refuse,
    /// The project's stays; the library's is not taken.
    Skip,
    /// One of the same id and source becomes the library's (a new version; its openings follow); another source's is
    /// still refused.
    Replace,
    /// Taken beside it under a new id and name (“ (2)”).
    Rename,
}

/// What `cad.openingtypes.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningTypesOperation {
    /// Yeni tip: a new type `name` (its id: `id`, else from its name), its values those of `copyOf` (a project type's,
    /// else KentOS's, by id or name; absent: the form's first KentOS type) changed by `change`; version 1.
    Define {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        copy_of: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        form: Option<OpeningForm>,
        #[serde(default)]
        change: OpeningTypeChange,
    },
    /// Tipi değiştir: the type `type` (its id or name) changed by `change`; its version goes up by one and its openings
    /// follow.
    Update {
        #[serde(rename = "type")]
        kind: String,
        change: OpeningTypeChange,
    },
    /// Yeniden adlandır: the type's name; its id and its openings stay.
    Rename {
        #[serde(rename = "type")]
        kind: String,
        name: String,
    },
    /// Sil: the type; its openings refuse it (`opening_type_in_use`) unless `detach`, which leaves them with their
    /// values, following nothing.
    Remove {
        #[serde(rename = "type")]
        kind: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        detach: bool,
    },
    /// Kitaplıktan al: an office library's types (`kentos.opening-types` 1, a file's text as its host read it), their
    /// source the library's name; `onConflict` for a type the project has.
    Import {
        library: crate::OpeningTypesFile,
        #[serde(default)]
        on_conflict: OpeningTypeConflict,
    },
    /// Kitaplığa yaz: the types named (none: all) as an office library `library`, given back as the file's content
    /// (its host writes it). The drawing does not change.
    Export {
        library: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
        types: Vec<String>,
    },
}

/// Input of `cad.openingtypes.edit` v1: the project's door and window types changed as one undo step “Açıklık tipi”,
/// with the openings following them. Its plan is the preview: the types and every opening's fate, nothing written.
///
/// Refusals: `invalid_opening_type`, `opening_type_not_found`, `opening_type_exists`, `opening_type_in_use`,
/// `nothing_to_set`, `not_finite`, `opening_type_refused`, `import_unreadable`, `invalid_revision`,
/// `revision_conflict`. Warnings: `opening_type_kept`, `opening_type_skipped`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningTypesEdit {
    pub operation: OpeningTypesOperation,
    /// When the change does not fit an opening following the type; absent: stop.
    #[serde(default)]
    pub on_refused: OpeningTypeRefused,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// An opening a type's change kept its own value for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningKept {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// The changed fields it keeps its own value of.
    pub fields: Vec<OpeningTypeField>,
}

/// Why an opening following a type did not take its change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningLeftWhy {
    /// On a locked layer.
    Locked,
    /// Its wall is gone.
    HostGone,
    /// It would run off its wall.
    OffWall,
    /// It would run into another opening.
    Overlap,
    /// Its fields would make none.
    Invalid,
}

/// An opening that did not take a type's change: it stays at the older version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningLeft {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    pub why: OpeningLeftWhy,
    pub message: String,
}

/// Every opening's fate in a type's change: those that took it, those that kept a changed field of their own, those
/// left at the older version (`skipped`: a locked layer; `refused`: it did not fit).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningTypeReport {
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub applied: Vec<String>,
    pub kept: Vec<OpeningKept>,
    pub skipped: Vec<OpeningLeft>,
    pub refused: Vec<OpeningLeft>,
    /// The fields the change changed.
    pub fields: Vec<OpeningTypeField>,
}

/// Output of `cad.openingtypes.edit` v1 (its plan's too: the preview, nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningTypesEdited {
    /// The project's types after it.
    pub types: Vec<OpeningType>,
    pub report: OpeningTypeReport,
    /// The office library (Export): the file's content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub library: Option<crate::OpeningTypesFile>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.openings.query` v1 (read-only): the drawing's doors, windows and other openings with their types and
/// widths, filtered: the openings named (a selection), of a type (its id or name), on a level (its id; null: on none),
/// of a form. “Aynı tiptekileri seç” asks with the type and a scope; the net clear widths come with every row.
///
/// Refusals: `invalid_uid`, `entity_not_found`, `not_an_opening`, `opening_type_not_found`, `level_not_found`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningsQuery {
    /// Only these (a selection's scope); absent: every opening.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// Only those following, or made with, this type (its id or name).
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<String>,
    /// Only those on this level (their wall's); null: on none.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub level: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub form: Option<OpeningForm>,
}

/// One opening as `cad.openings.query` reads it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningQueryRow {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    pub form: OpeningForm,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub number: Option<String>,
    /// Its type's id (its `style`) and name.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub type_name: Option<String>,
    /// The version it took (it follows the type); absent: it follows none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub type_version: Option<u32>,
    /// It took an older version than its type's.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub stale: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<OpeningTypeField>>", optional))]
    pub overrides: Vec<OpeningTypeField>,
    /// Its wall's level; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    pub host: String,
    pub sizes: OpeningSizes,
}

/// A project type and how many openings follow it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningTypeUse {
    #[serde(rename = "type")]
    pub kind: OpeningType,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    /// Of them, those behind its version.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stale: u32,
    /// Of them, those keeping a field of their own.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub exceptions: u32,
}

/// Output of `cad.openings.query` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningsQueried {
    pub openings: Vec<OpeningQueryRow>,
    /// The project's types, each with its openings counted over the whole drawing.
    pub types: Vec<OpeningTypeUse>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
