//! `cad.parcels.record` v1, `cad.parcels.records` v1, `cad.parcels.approve` v1 and `cad.parcels.withdraw` v1
//! (plan/map-cad.md MAP-08, docs/adr/4220, 4620 and 5020): Parsel kayıtları, the drawing's parcel register
//! (`settings.parcelRegister`, kentos_contracts::parcel_register):
//!
//! - `cad.parcels.record` writes one document (a deed, a plan, a survey: its name, unique in the drawing, kind, date as
//!   written, note). The document makes parcels from areas (identity İl/İlçe/Mahalle/Ada/Parsel from the area's
//!   attributes, the stated area as written, the area's rings as a chain of boundaries) and closes earlier parcels,
//!   which stay as history. A ring edge with the same two corners (as stored) and bulge as a current boundary is that
//!   boundary: two parcels share it by its number. A boundary no current parcel uses any more is closed by the
//!   document. Docs/adr/4620 and 5020: an earlier boundary (straight or an arc) a current neighbour uses, inside which
//!   the document's new corners lie and whose pieces are the document's boundaries, is split: the neighbour's rings take
//!   the pieces. The document states its boundaries' lengths (`lengths`, else the area's `Kenar ölçüleri (m)`); with
//!   `draft` it is written as a draft that changes nothing current. The areas are not touched: the register is apart
//!   from the drawing. One undo step, “Parsel kaydı”.
//! - `cad.parcels.approve` writes an approval given outside (its number, date, who): a draft's approval writes the
//!   draft as a document with the next number, as drafted; a document's is kept with it. One undo step, “Belge onayı”.
//! - `cad.parcels.withdraw` removes a draft. One undo step, “Taslağı geri çek”.
//! - `cad.parcels.records` reads the register against the drawing: each current parcel's area now beside its recorded
//!   area and the area its document states (a moved boundary changes the first, never the others), its neighbours by
//!   shared boundaries, the parcels before and after it through its documents, and whether each boundary is still
//!   drawn where the record has it; each length a document states beside the boundary's length, their difference and
//!   the frontage check's limit d = 0,05 + 0,001·S m (S the stated length).
//!
//! The shared cases are `fixtures/commands/v1/cad.parcels.record.json`, `cad.parcels.records.json`,
//! `cad.parcels.approve.json` and `cad.parcels.withdraw.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{ParcelApproval, StatedLength};

pub const CAD_PARCELS_RECORD: &str = "cad.parcels.record";
pub const CAD_PARCELS_RECORD_VERSION: u32 = 1;
pub const CAD_PARCELS_RECORDS: &str = "cad.parcels.records";
pub const CAD_PARCELS_RECORDS_VERSION: u32 = 1;
pub const CAD_PARCELS_APPROVE: &str = "cad.parcels.approve";
pub const CAD_PARCELS_APPROVE_VERSION: u32 = 1;
pub const CAD_PARCELS_WITHDRAW: &str = "cad.parcels.withdraw";
pub const CAD_PARCELS_WITHDRAW_VERSION: u32 = 1;

/// The document `cad.parcels.record` writes: its name (unique in the drawing, at most 120 characters), its kind (120)
/// and date (64) as the user writes them, a note (4000).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelDocumentInput {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
}

/// A parcel the document makes: the area that draws it, the area the document states, a plain decimal in square metres
/// as written (none: the area's `Tapu alanı (m²)` when it has one; empty: the document states none), and the lengths it
/// states for the area's edges (none: the area's `Kenar ölçüleri (m)`, the outline's edges' lengths in order, separated
/// by `;`, an empty or `-` one unstated; empty: none).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelToRecord {
    pub uid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stated_area: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lengths: Option<Vec<StatedLength>>,
}

/// Input of `cad.parcels.record` v1: the document, the parcels it makes (`create`) and the recorded parcels it closes
/// (`close`, their numbers); at least one of the two.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`; `invalid_input` (the document's name empty, too
/// long or with a control character, its kind, date or note too long; nothing to make or close); `document_name_taken`
/// (`document.name`); for `close[i]`: `invalid_input` (given twice), `unknown_parcel_record`, `parcel_closed`; for
/// `create[i]`: `invalid_uid`, `entity_not_found`, `not_a_parcel` (not an area), `no_parcel_identity` (no Ada or
/// Parsel), `invalid_input` (the area given twice, an identity part past 120 characters, two vertices in one place, a
/// stated area that is not a plain decimal), `parcel_recorded` (the area draws a current parcel the document does not
/// close), `parcel_identity_taken` (a current parcel the document does not close, or another it makes, has the identity);
/// `invalid_input` for `create[i].lengths[j]` (the ring or edge not the area's, an edge given twice, not a plain decimal
/// above zero; or the area's `Kenar ölçüleri (m)` so, at `create[i].lengths`); `invalid_input` (the register would pass
/// its limits: 10 000 documents and drafts, 100 000 parcels, 1 000 000 boundaries). `document_name_taken` covers the
/// drafts' names too. With `draft` the same checks run and the answer is what approving would write today; only the
/// draft is written, numbered among the drafts.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsRecord {
    pub document: ParcelDocumentInput,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ParcelToRecord>>"))]
    pub create: Vec<ParcelToRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<u32>>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub close: Vec<u32>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub draft: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.parcels.record` v1: the document's number, the parcels it made and closed (numbers), the boundaries
/// it made, the current boundaries its parcels share with earlier ones, and those it closed; the revision after.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsRecorded {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub document: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub created: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub closed: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub boundaries_created: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub boundaries_shared: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub boundaries_closed: u32,
    /// Docs/adr/4620: the earlier boundaries split at the document's new corners, and the current parcels whose rings
    /// took the pieces (not closed: their identity, document and stated area stay).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<u32>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub boundaries_split: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<u32>>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub neighbours: Vec<u32>,
    /// Docs/adr/5020: the boundaries' lengths the document states.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<u32>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub lengths: u32,
    /// Docs/adr/5020: written as a draft; `document` is then the draft's number and the rest what approving it would
    /// write today.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub draft: bool,
    pub revision: String,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// An approval as given (docs/adr/5020): its number (at most 64 characters), date (64) and who approved (120), as
/// written outside KentOS.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelApprovalInput {
    pub reference: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub by: Option<String>,
}

/// Input of `cad.parcels.approve` v1: the draft (`draft`, its number) or the document (`document`) the approval is of,
/// one of the two; the approval.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`; `invalid_input` (neither or both of `draft` and
/// `document`; the approval's number empty, past 64 characters or with a control character, its date or approver too
/// long); `unknown_parcel_draft` (`draft`), `unknown_parcel_document` (`document`); `document_approved` (the document has
/// its approval); for a draft, what `cad.parcels.record` refuses when it writes the draft today, at `draft`:
/// `parcel_closed` (a parcel it closes was closed since), `parcel_recorded`, `parcel_identity_taken`, `invalid_input`
/// (the register's limits).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsApprove {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub draft: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub document: Option<u32>,
    pub approval: ParcelApprovalInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.parcels.approve` v1: the approved document's number and, for a draft, what writing it did (as
/// `cad.parcels.record` answers; none for a document already recorded); the revision after.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsApproved {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub document: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub recorded: Option<ParcelsRecorded>,
    pub revision: String,
}

/// Input of `cad.parcels.withdraw` v1: the draft to remove. Refusals: `invalid_revision`, `revision_conflict`,
/// `unknown_parcel_draft` (`draft`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsWithdraw {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub draft: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.parcels.withdraw` v1: the removed draft's name; the revision after.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsWithdrawn {
    pub name: String,
    pub revision: String,
}

/// Input of `cad.parcels.records` v1: with `history` the closed parcels and boundaries too (none: the current ones).
/// Every document is listed. No refusal: a drawing without a register answers empty lists.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsRecords {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub history: bool,
}

/// A document as listed: its fields, its approval as written (docs/adr/5020) and the parcels it made and closed.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelDocumentRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub number: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub date: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub approval: Option<ParcelApproval>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub created: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub closed: Vec<u32>,
}

/// A draft as listed (docs/adr/5020): its number among the drafts, its fields, the identities (`Ada/Parsel`) of the
/// parcels it would make, the recorded parcels it would close, how many lengths it states.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelDraftRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub number: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub date: String,
    pub create: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub close: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub lengths: u32,
}

/// A length a document states for a boundary against the boundary's length now (docs/adr/5020): the document, the
/// length as written, the boundary's length less it, and the frontage check's limit d = 0,05 + 0,001·S m (S the stated
/// length in metres); `within` when the difference's size is no more than the limit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BoundaryLengthRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub document: u32,
    pub length: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub difference: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub limit: f64,
    pub within: bool,
}

/// A recorded parcel against the drawing: its identity, documents and stated area as recorded; the area of its
/// recorded rings; the area that draws it and that area's geometric area now; its state (`current`: drawn as
/// recorded, `changed`: drawn otherwise, `missing`: its area is gone, `closed`); its boundaries in ring order; the
/// current parcels it shares a boundary with (a current parcel's); the parcels its document closed (`before`) and
/// those the document that closed it made (`after`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordedParcelRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub number: u32,
    pub il: String,
    pub ilce: String,
    pub mahalle: String,
    pub ada: String,
    pub parsel: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub created_by: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub closed_by: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stated_area: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub recorded_area: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: Option<f64>,
    pub state: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub boundaries: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub neighbours: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub before: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub after: Vec<u32>,
}

/// A recorded boundary against the drawing: its corners and bulge as recorded, its length (the arc's on an arc), its
/// documents, the parcels that use it (a current boundary's current ones) and whether every current one still draws
/// it as recorded (its area has an edge between the two corners with the bulge; a closed boundary: false).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordedBoundaryRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub number: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: [f64; 2],
    #[serde(default, skip_serializing_if = "is_straight")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulge: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub created_by: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub closed_by: Option<u32>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub parcels: Vec<u32>,
    pub drawn: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<BoundaryLengthRow>>"))]
    pub lengths: Vec<BoundaryLengthRow>,
}

fn is_straight(bulge: &f64) -> bool {
    *bulge == 0.0
}

/// Output of `cad.parcels.records` v1: the documents, the parcels and the boundaries (current ones, or all with
/// `history`), the drafts, and the counts: current and closed parcels, current parcels drawn otherwise and without their
/// area, current boundaries no longer drawn as recorded, current boundaries with a stated length past its limit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsRecordsListed {
    pub documents: Vec<ParcelDocumentRow>,
    pub parcels: Vec<RecordedParcelRow>,
    pub boundaries: Vec<RecordedBoundaryRow>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub current: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub closed: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changed: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub missing: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub moved: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ParcelDraftRow>>"))]
    pub drafts: Vec<ParcelDraftRow>,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<u32>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub over_limit: u32,
}
