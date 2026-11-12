//! `cad.records.page` (docs/adr/0532 §3): a layer's records, or the records a relation ties to one record, read a page
//! at a time, with the version of the source they come from. What a lookup list or a related-records box reads of
//! another layer, and what an agent reads of one: the total counted after the project's layer filters (an object a
//! filter leaves out is not counted either), the page's records by lasting id, the next page's cursor, and for a
//! bound layer the revision of its source and whether the layer has changes the source lacks. The shared cases are
//! `fixtures/commands/v1/cad.records.page.json`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::sources::SourceKind;

pub const CAD_RECORDS_PAGE: &str = "cad.records.page";
pub const CAD_RECORDS_PAGE_VERSION: u32 = 1;

/// The most a page holds, and how many when none is said.
pub const MOST_PAGED: u32 = 500;
pub const PAGED: u32 = 50;

/// Which side of a relation the record asked about is on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RelationSide {
    /// `of` is a parent: its children (the child layer's records whose field holds it).
    #[default]
    Children,
    /// `of` is a child: its parents (the parent layer's records its field holds, in the field's order).
    Parents,
}

/// Input of `cad.records.page` v1: the records of `layer`, or those `relation` ties to `of`.
///
/// Refusals, checked in this order: `invalid_count` (a limit not 1–500), `invalid_cursor` (not a cursor of this
/// listing), `invalid_input` (neither `layer` nor `relation`, both, `of` without `relation`, `side` without it),
/// `layer_not_found`, `unknown_relation`, `entity_not_found` (`of`), `form_layer_mismatch` (`of` is not on the
/// relation's side's layer), `field_not_found` (a field to return); then `revision_conflict` (status `conflict`: the
/// document is not at `expectedRevision`) and `source_changed` (status `conflict`: the layer's source is not at
/// `expectedSource`), so a page after the first is of the same data.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordsPage {
    /// The layer to read (a layer's id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// A project relation's id: read the records it ties to `of`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub relation: Option<String>,
    /// The record asked about (lowercase UUID text with hyphens).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<RelationSide>", optional))]
    pub side: Option<RelationSide>,
    /// Words a record's attribute values hold, all of them (Turkish folded); none: every record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub query: Option<String>,
    /// The attributes returned, by the layer's fields' names; none: all of them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub fields: Vec<String>,
    /// How many records a page holds, 1–500; none: 50.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub limit: Option<u32>,
    /// The page after the one whose `next` this is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cursor: Option<String>,
    /// The document revision the first page was read at (decimal text).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
    /// The source revision the first page said (`source.revision`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_source: Option<String>,
}

/// A record of a page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PagedRecord {
    pub uid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    pub attrs: BTreeMap<String, String>,
}

/// The source a page's layer is bound to, and its version.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PagedSource {
    /// The bound layer's source id (`kaynak-N`).
    pub id: String,
    pub kind: SourceKind,
    pub table: String,
    /// The source's revision when the layer last matched it (`SourceLink.revision`): the version the records are of.
    pub revision: String,
    /// The layer has changes the source lacks: the page is the working copy's, not the source's.
    pub edited: bool,
}

/// Output of `cad.records.page` v1, and its plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordsPaged {
    /// The layer the records are on (a relation's side's layer).
    pub layer: String,
    /// How many records match, over every page, after the layer filters.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub total: u32,
    pub records: Vec<PagedRecord>,
    /// The cursor of the page after this one; none on the last.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub next: Option<String>,
    /// The layer's source, when it is bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<PagedSource>,
    pub revision: String,
}
