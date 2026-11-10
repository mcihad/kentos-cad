//! `cad.parcels.record` v1 and `cad.parcels.records` v1 (plan/map-cad.md MAP-08, docs/adr/4220): Parsel kayıtları, the
//! drawing's parcel register (`settings.parcelRegister`, kentos_contracts::parcel_register). After ArcGIS's parcel
//! fabric records:
//!
//! - `cad.parcels.record` writes one document (a deed, a plan, a survey: its name, unique in the drawing, kind, date as
//!   written, note). The document makes parcels from areas (identity İl/İlçe/Mahalle/Ada/Parsel from the area's
//!   attributes, the stated area as written, the area's rings as a chain of boundaries) and closes earlier parcels,
//!   which stay as history. A ring edge with the same two corners (as stored) and bulge as a current boundary is that
//!   boundary: two parcels share it by its number. A boundary no current parcel uses any more is closed by the
//!   document. Docs/adr/4620: an earlier straight boundary a current neighbour uses, inside which the document's new
//!   corners lie and whose pieces are the document's boundaries, is split: the neighbour's rings take the pieces (as
//!   ArcGIS splits a bounding line so that the current parcel has coincident current lines). The areas are not touched:
//!   the register is apart from the drawing. One undo step, “Parsel kaydı”.
//! - `cad.parcels.records` reads the register against the drawing: each current parcel's area now beside its recorded
//!   area and the area its document states (a moved boundary changes the first, never the others), its neighbours by
//!   shared boundaries, the parcels before and after it through its documents, and whether each boundary is still
//!   drawn where the record has it.
//!
//! The shared cases are `fixtures/commands/v1/cad.parcels.record.json` and `cad.parcels.records.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_PARCELS_RECORD: &str = "cad.parcels.record";
pub const CAD_PARCELS_RECORD_VERSION: u32 = 1;
pub const CAD_PARCELS_RECORDS: &str = "cad.parcels.records";
pub const CAD_PARCELS_RECORDS_VERSION: u32 = 1;

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

/// A parcel the document makes: the area that draws it, and the area the document states, a plain decimal in square
/// metres as written (none: the area's `Tapu alanı (m²)` when it has one; empty: the document states none).
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
/// `invalid_input` (the register would pass its limits: 10 000 documents, 100 000 parcels, 1 000 000 boundaries).
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
    pub revision: String,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
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

/// A document as listed: its fields and the parcels it made and closed.
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub created: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub closed: Vec<u32>,
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
}

fn is_straight(bulge: &f64) -> bool {
    *bulge == 0.0
}

/// Output of `cad.parcels.records` v1: the documents, the parcels and the boundaries (current ones, or all with
/// `history`), and the counts: current and closed parcels, current parcels drawn otherwise and without their area,
/// current boundaries no longer drawn as recorded.
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
}
