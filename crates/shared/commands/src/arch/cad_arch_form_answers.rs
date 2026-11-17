//! `cad.form.answer` v1 (“Kontrol formunun cevapları”, docs/adr/8077; plan/bim-plan.md BIM-14): the author's
//! answers to the model quality control form's questions KentOS leaves to them (Dijital Proje Yön. EK-9 Tablo 9.1: the
//! file size, the development level, the area tables), kept in the drawing (`settings.arch.formAnswers`): the printed
//! form and the BIM package's publication read them. Each change names its question: Evet or Hayır with a note, or null
//! to take the answer away. One undo step “Kontrol formunun cevapları”; the drawing's geometry never changes.
//!
//! The command keeps what the author says; which questions take it is the form's (the standards package's rule): an
//! answer to a question KentOS answers from the file is kept but not read.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::FormAnswer;

/// The command's name.
pub const CAD_FORM_ANSWER: &str = "cad.form.answer";
/// Its version.
pub const CAD_FORM_ANSWER_VERSION: u32 = 1;

/// One question's change: Evet (true) or Hayır (false), or null to take its answer, note and basis away; its note,
/// absent to keep the one it has (an empty one takes it away); the evidence it answers, likewise.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormAnswerChange {
    /// The question's number in EK-9 Tablo 9.1 (1–21).
    #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 21), extend("x-unit" = "index")))]
    pub no: u8,
    pub yes: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
    /// The form's evidence the author answered (the question's KentOS note now, a publication's `rows[].basis`);
    /// absent: the one it has kept. An answer whose evidence changes is stale (docs/adr/8077 §7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub basis: Option<String>,
}

/// Input of `cad.form.answer` v1.
///
/// Refusals, in this order: `nothing_to_set` (no change), `invalid_input` (a number outside 1–21, a question twice, a
/// note or a basis given with null, a note or a basis too long or with a control character), `invalid_revision`,
/// `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchFormAnswers {
    pub answers: Vec<FormAnswerChange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.form.answer` v1, and its plan: the drawing's answers then, by question; the revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchFormAnswersDone {
    pub answers: Vec<FormAnswer>,
    pub revision: String,
}
