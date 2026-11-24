//! `cad.parcels.report` v1 and `cad.parcels.table` v1 (docs/adr/9306): Parsel alan çizelgesi, a cadastral area
//! computation table. `report` reads the parcels and writes nothing: each parcel's rings, their corners in order with a
//! point number and their place, the edges' lengths, the area computed from the corners' coordinates (on the map's
//! plane, arcs and holes included) and, apart, the area its record gives; and the table's words. `table` writes that
//! table into the drawing, its top left corner at `at` (one undo step “Parsel alan çizelgesi tablosu”).
//!
//! The rules (each in the ADR, which the independent reference follows):
//!
//! - Scope: the areas `uids` names, in its order, else every area on the parcel layer `parsel` in the drawing's order.
//! - Rings: each part's outer ring, then its holes; every ring listed clockwise as the map shows it, from its
//!   northernmost corner (of two as far north, the western one).
//! - Corner names: with `names: points` a corner takes the name of the first named point object of the drawing
//!   standing on it (1 µm); every other corner, and with `names: numbers` every corner, takes a new number: `prefix`
//!   and a whole number from `start` up by one, past every name the drawing's points have and the report has given. A
//!   place two corners share (1 µm) has one name.
//! - Areas: the ring's area by its corners' coordinates (the Gauss formula) and each arc's segment; a parcel's is its
//!   outer rings' less its holes'. The record (`field`, as written) is never overwritten: a plain decimal is read, its
//!   difference and the 2022 regulation's Article 8(2) tolerance for archive-approved geometry are given beside it.
//!
//! The shared cases are `fixtures/commands/v1/cad.parcels.report.json` and `cad.parcels.table.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::Vec2;

pub const CAD_PARCELS_REPORT: &str = "cad.parcels.report";
pub const CAD_PARCELS_REPORT_VERSION: u32 = 1;
pub const CAD_PARCELS_TABLE: &str = "cad.parcels.table";
pub const CAD_PARCELS_TABLE_VERSION: u32 = 1;

/// The most letters a new number's prefix may have.
pub const MAX_CORNER_PREFIX: usize = 12;
/// The most letters the record's attribute name may have.
pub const MAX_RECORD_FIELD: usize = 120;

/// How the corners are named: by the point standing on each, a new number where none stands (`points`); or each by a
/// new number (`numbers`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CornerNames {
    #[default]
    Points,
    Numbers,
}

/// Where a corner's name came from: the point standing on it, or a new number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CornerNamed {
    Point,
    Number,
}

/// Input of `cad.parcels.report` v1: the parcels (none: every area on the parcel layer `parsel`), how their corners are
/// named (none: `points`), a new number's prefix (none: no prefix; at most 12 letters) and first number (none: 1),
/// whether the edges' lengths are listed (none: yes), and the attribute that holds the recorded area (none or blank:
/// `Tapu alanı (m²)`; at most 120 letters).
///
/// Refusals: `invalid_input` (`prefix`, `field`), `invalid_uid` and `entity_not_found` (`uids[i]`), `not_a_parcel`
/// (`uids[i]`: not an area).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsReport {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub names: Option<CornerNames>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub start: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lengths: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
}

/// A corner as listed: its name and where the name came from, its place (x east, y north), and the edge from it to the
/// ring's next corner: the straight distance between the two (the chord), and for an arc its radius and its length
/// along the arc.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReportCorner {
    pub name: String,
    pub named: CornerNamed,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub arc: Option<f64>,
}

/// A ring as listed: its part (0: the first) and, for a hole, its number in the part (0: the first; none: the outer
/// ring), its corners clockwise from the northernmost, and the area it encloses (square metres, arcs included).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReportRing {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub part: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub hole: Option<u32>,
    pub corners: Vec<ReportCorner>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
}

/// A parcel as listed: its object, its identity's parts as written (empty when missing), the heading the table gives
/// it, its rings, its area computed from the coordinates (square metres), its record's text as written and its value
/// when it reads, the difference (computed − recorded), the tolerance from the computed area, and the status:
/// `within`, `outside`, `noRecord` or `invalidRecord`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelReported {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    pub il: String,
    pub ilce: String,
    pub mahalle: String,
    pub ada: String,
    pub parsel: String,
    pub heading: String,
    pub rings: Vec<ReportRing>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub recorded_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub recorded: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub difference: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub tolerance: f64,
    pub status: String,
}

/// Output of `cad.parcels.report` v1: the project's coordinate system (none without one), the tolerance's rule, the
/// record's attribute, the parcels, their areas' sum, how many corner places there are and how many were named by a
/// point or numbered, and the table's words row by row (as `cad.parcels.table` writes them).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsReported {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    pub rule: String,
    pub field: String,
    pub parcels: Vec<ParcelReported>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub places: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub named: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub numbered: u32,
    pub cells: Vec<Vec<String>>,
}

/// Input of `cad.parcels.table` v1: the report's input, and the table's top left corner, its layer (none: the active
/// one), its words' height (metres; none: the project's table height at its plot scale) and the expected revision.
///
/// Refusals: as `cad.parcels.report`'s, `no_objects` (no parcel to write), `not_finite` (`at`), `invalid_table`
/// (`height`, or a table past a table's limits), `layer_not_found`, `not_a_layer`, `layer_locked`, `invalid_revision`,
/// `revision_conflict`. Warning: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsTable {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub names: Option<CornerNames>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub start: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lengths: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

impl ParcelsTable {
    /// The report the table writes.
    pub fn report(&self) -> ParcelsReport {
        ParcelsReport {
            uids: self.uids.clone(),
            names: self.names,
            prefix: self.prefix.clone(),
            start: self.start,
            lengths: self.lengths,
            field: self.field.clone(),
        }
    }
}

/// What `cad.parcels.table` would write: the table, how many parcels and rows it has and how many corners it numbers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsTablePlan {
    pub table: crate::entity::Entity,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub parcels: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub numbered: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Output of `cad.parcels.table` v1: the table's persistent id, how many parcels and rows it has and how many corners
/// it numbered.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsTabled {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub parcels: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub numbered: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
