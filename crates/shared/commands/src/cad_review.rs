//! `cad.review.notes.edit` and `cad.review.notes.list` (docs/adr/1000 §3, §4): review notes bound to objects, a
//! sheet or a named view, kept apart from the design. The edit writes one note change as one undo step
//! “İnceleme notu”; the list says, besides each note, whether the objects it was written for changed since.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{ReviewNote, ReviewOrigin, ReviewStatus, Vec2};

pub const CAD_REVIEW_NOTES_EDIT: &str = "cad.review.notes.edit";
pub const CAD_REVIEW_NOTES_EDIT_VERSION: u32 = 1;
pub const CAD_REVIEW_NOTES_LIST: &str = "cad.review.notes.list";
pub const CAD_REVIEW_NOTES_LIST_VERSION: u32 = 1;
/// The undo step's name.
pub const REVIEW_NOTE_STEP: &str = "İnceleme notu";

/// One change of the drawing's review notes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ReviewNoteOp {
    /// A new note, open; its anchors' stamps are the drawing's now.
    #[serde(rename_all = "camelCase")]
    Add {
        /// Lowercase, hyphenated UUID; absent: a new one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        id: Option<String>,
        text: String,
        /// Persistent ids of the objects the note is about, at most 1000, none twice.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
        anchors: Vec<String>,
        /// A point in the model, world coordinates.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Option<Vec2>,
        /// A sheet's id.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        sheet: Option<String>,
        /// A named view's id.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        view: Option<String>,
        /// The source revision label (“B”).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        revision: Option<String>,
        /// `user` (absent), `pdf` or `ai`: a note only, never a design change.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        origin: Option<ReviewOrigin>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        author: Option<String>,
        /// When it was written, as the host says it (ISO 8601); the command puts no clock in.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        created_at: Option<String>,
    },
    /// A note's text, status or revision label; at least one.
    #[serde(rename_all = "camelCase")]
    Update {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        status: Option<ReviewStatus>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        revision: Option<String>,
    },
    /// A reply after the note's last; the status changes only when given.
    #[serde(rename_all = "camelCase")]
    Reply {
        id: String,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        author: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        created_at: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        status: Option<ReviewStatus>,
    },
    /// The anchors stamped again with the drawing as it is now: the change was seen. `anchors` given: the note's
    /// anchors become exactly these; absent: all of them, again.
    #[serde(rename_all = "camelCase")]
    Rebind {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        anchors: Option<Vec<String>>,
    },
    /// The note goes.
    #[serde(rename_all = "camelCase")]
    Remove { id: String },
}

/// Input of `cad.review.notes.edit` v1. Refusals, in order: `invalid_review_note`, `invalid_uid`,
/// `review_note_not_found`, `duplicate_review_note`, `not_finite`, `invalid_review_anchors`, `entity_not_found`,
/// `unknown_sheet`, `unknown_view`, `too_many_review_replies`, `too_many_review_notes`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewNotesEdit {
    /// The one change.
    pub change: ReviewNoteOp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.review.notes.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewNotesEdited {
    /// The note the change was about.
    pub id: String,
    /// Its number (a removed note's too).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub number: u32,
    /// How many notes the drawing has after.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub notes: u32,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.review.notes.list` v1: absent filters list every note.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewNotesList {
    /// Only notes in these states.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub status: Option<Vec<ReviewStatus>>,
    /// Only stale notes.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub stale_only: bool,
}

/// How an anchor stands against the object now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ReviewAnchorState {
    /// The object is as the note saw it.
    Current,
    /// The object changed.
    Changed,
    /// The object is as it was, its block definition (or one nested in it) changed.
    BlockChanged,
    /// The object is gone.
    Removed,
}

/// An anchor's state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewAnchorCheck {
    pub uid: String,
    pub state: ReviewAnchorState,
}

/// A note and whether it is stale.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewNoteState {
    pub note: ReviewNote,
    /// An anchor is not current, or its sheet or view is gone.
    pub stale: bool,
    /// Each anchor's state, in the note's order.
    pub anchor_states: Vec<ReviewAnchorCheck>,
    pub sheet_missing: bool,
    pub view_missing: bool,
}

/// Output of `cad.review.notes.list` v1: the notes by their numbers; the counts are of every note.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewNotesListed {
    pub notes: Vec<ReviewNoteState>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub open: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub answered: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub resolved: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stale: u32,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}
