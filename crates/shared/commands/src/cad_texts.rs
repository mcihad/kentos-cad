//! The text commands (docs/adr/0325 §3): `cad.fields.edit` v1 puts fields into a text, takes them out, formats them
//! and writes them again; `cad.texts.edit` v1 replaces letters (Bul ve değiştir), numbers texts (AutoCAD's TCOUNT),
//! combines texts into one (TXT2MTXT) and splits a text into its paragraphs; `cad.texts.check` v1, read only, finds
//! double spaces, repeated words, unbalanced brackets and quotes and broken fields.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::Entity;
use crate::text_field::{FieldFormat, FieldSource};

pub const CAD_FIELDS_EDIT: &str = "cad.fields.edit";
pub const CAD_FIELDS_EDIT_VERSION: u32 = 1;

/// One edit of `cad.fields.edit` (docs/adr/0325 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FieldEdit {
    /// Alan ekle: a field in place of text `uid`'s letters `start..end` (equal: put in at that letter), its value
    /// written at once.
    Insert {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        start: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        end: u32,
        source: FieldSource,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        format: Option<FieldFormat>,
    },
    /// Alanı kaldır: text `uid`'s field `index` (from 0) taken out; its letters stay as text.
    Remove {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        index: u32,
    },
    /// Text `uid`'s field `index` written with `format` (null: the project's decimals and unit).
    Format {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        index: u32,
        format: Option<FieldFormat>,
    },
    /// Alanları yenile: the fields of `uids` (empty: every text with fields) written again now.
    Update {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
}

/// Input of `cad.fields.edit` v1. Refusals: `invalid_uid`, `entity_not_found`, `not_a_text`, `layer_locked`,
/// `invalid_text_field`, `field_not_found`. A field that cannot be read is written `####` with a `field_broken`
/// warning.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldsEdit {
    pub edit: FieldEdit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.fields.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldsEdited {
    /// The texts that changed, by their persistent ids.
    pub changed: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.fields.edit` and `cad.texts.edit` would write (plan mode).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextsPlan {
    /// The texts as execute would write them: those changed, then those added.
    pub texts: Vec<Entity>,
    /// The texts it would take away, by their persistent ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub removed: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

pub const CAD_TEXTS_EDIT: &str = "cad.texts.edit";
pub const CAD_TEXTS_EDIT_VERSION: u32 = 1;

/// Where Numarala writes a number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NumberPlace {
    /// The text becomes the number.
    #[default]
    Replace,
    /// Before the text.
    Prefix,
    /// After the text.
    Suffix,
}

/// One edit of `cad.texts.edit` (docs/adr/0325 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TextEdit {
    /// Bul ve değiştir: every `find` in the texts `uids` (empty: every shown text) replaced by `with`; a text left
    /// blank, or on a locked layer, is left as it was and said.
    #[serde(rename_all = "camelCase")]
    Replace {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        find: String,
        #[serde(default)]
        with: String,
        /// Big and small letters apart (Turkish İ and ı); absent: alike.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        match_case: bool,
        /// Only a whole word.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        whole_word: bool,
    },
    /// Numarala (TCOUNT): the texts `uids`, in their order, numbered `start`, `start + step` …, each written with at
    /// least `digits` digits (zeros before), `prefix` and `suffix` around it, in place of the text, before or after.
    #[serde(rename_all = "camelCase")]
    Number {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        #[serde(default = "one")]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        start: i64,
        #[serde(default = "one")]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        step: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(range(max = 12)))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        digits: Option<u8>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
        prefix: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
        suffix: String,
        #[serde(default)]
        place: NumberPlace,
    },
    /// Yazıları birleştir (TXT2MTXT): the texts `uids` as one text, from the highest down (then from the left), at
    /// the top of the highest, with its look; the others taken away.
    Combine {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
    /// Satırlara ayır: text `uid`'s paragraphs (its lines between line breaks) as texts of their own, each where it
    /// stood, with its formats and fields; the text taken away.
    Split {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
    },
    /// Yazıyı eğriye oturt (docs/adr/0196 §4): each one-line text of `uids` not linked to an object put on the curve
    /// `curve` where its box's middle falls, its letters' middle there; the others are left and counted.
    Fit {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        curve: String,
    },
    /// Doğrultuya döndür: each straight text of `uids` turned about its point to the direction of the edge of `edge`
    /// nearest `at`, readable; the curved ones are left and counted.
    Turn {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        edge: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: crate::Vec2,
    },
    /// Düzleştir: each curved text of `uids` made straight at its first letter, along its direction; the straight
    /// ones are left and counted.
    Straighten {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
}

fn one() -> i64 {
    1
}

/// Input of `cad.texts.edit` v1. Refusals: `invalid_uid`, `entity_not_found`, `not_a_text`, `layer_locked`,
/// `no_entities`, `empty_text` (an empty `find`), `too_few_objects` (combining one text), `not_splittable` (one
/// paragraph, columns, a curve), `invalid_number`, `not_a_path` (a curve or an edge that is none), `not_finite`,
/// `text_not_taken` (no given text the edit takes).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextsEdit {
    pub edit: TextEdit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.texts.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextsEdited {
    /// The texts that changed, by their persistent ids.
    pub changed: Vec<String>,
    /// The texts it added (Satırlara ayır), in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub added: Vec<String>,
    /// The texts it took away.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub removed: Vec<String>,
    /// How many places Bul ve değiştir replaced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub replaced: Option<u32>,
    /// Eğriye oturt, Doğrultuya döndür, Düzleştir: the given texts the edit does not take.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: Option<u32>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

pub const CAD_TEXTS_CHECK: &str = "cad.texts.check";
pub const CAD_TEXTS_CHECK_VERSION: u32 = 1;

/// Input of `cad.texts.check` v1, read only. Refusals: `invalid_uid`, `entity_not_found`, `not_a_text`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextsCheck {
    /// The texts to check; empty: every shown text.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
}

/// What a text's finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TextFindingKind {
    /// Two spaces or more in a row inside a line.
    DoubleSpace,
    /// A word written twice in a row (“ve ve”).
    RepeatedWord,
    /// A bracket or a quote that is not closed or not opened.
    Unbalanced,
    /// A field that cannot be read.
    BrokenField,
}

/// One finding of `cad.texts.check`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextFinding {
    pub uid: String,
    pub kind: TextFindingKind,
    /// The letters it is about, counting Unicode scalar values.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub start: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub end: u32,
    /// What it is, for the user.
    pub words: String,
}

/// Output of `cad.texts.check` v1: the findings, text by text in the drawing's order, each text's by place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextsChecked {
    pub findings: Vec<TextFinding>,
    /// How many texts were read.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub texts: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
