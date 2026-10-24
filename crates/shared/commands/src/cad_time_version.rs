//! Yeni sürüm oluştur and Sona erdir as one product command (docs/adr/0210 §7, 0622 §9, TODOS.md `UX-14`).
//!
//! The caller gives objects of ranged temporal layers (a start and an end field, ADR 0210 §2) by their persistent ids
//! and a date; the command checks every object's time holds the date and writes it as the date's end. Yeni sürüm
//! oluştur also writes beside each object a copy (its geometry, colour, weight, attributes, label and symbol), starting
//! at the date and ending where the object ended. One undo step named after the tool; the tools write through it, and Python and an agent ask the same.
//!
//! A layer whose start or end field is of the date kind (ADR 0199) takes the day alone: the date written there is that
//! day's midnight.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

/// Ends objects at a date, and writes their new versions.
pub const CAD_TIME_VERSION: &str = "cad.time.version";
pub const CAD_TIME_VERSION_VERSION: u32 = 1;

/// What is written at the date.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TimeVersionKind {
    /// Yeni sürüm oluştur: each object ends at the date and its copy starts there.
    #[default]
    Version,
    /// Sona erdir: each object ends at the date.
    End,
}

/// `cad.time.version`'s input.
///
/// Refusals, in this order: `no_entities`, `invalid_uid`, `invalid_moment`, `invalid_revision`, `revision_conflict`,
/// `entity_not_found`; then every object in the input's order: `not_ranged`, `layer_locked`, `time_unreadable`,
/// `not_versionable` (Yeni sürüm oluştur: a record or an architectural object, copied by their own commands); then
/// `not_within`, which names the first object whose time does not hold the date and counts them all; then
/// `invalid_attribute`, a typed end field (or, for the new versions, start field) that does not take the date as its
/// text. The new versions are written as `cad.entities.create` writes them, by their layer's fields and rules.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TimeVersion {
    /// The objects' persistent ids, at least one; a repeat counts once.
    pub uids: Vec<String>,
    /// The date: a date (`2024-03-05`, `05.03.2024`), or a date and a time with an optional zone (`2024-03-05T10:30`,
    /// `2024-03-05T10:30:00+03:00`; without a zone UTC), as the layers' attributes are read (ADR 0210 §3).
    pub at: String,
    #[serde(default)]
    pub kind: TimeVersionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// The date written on a layer's objects.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TimeVersionLayer {
    /// The layer's id.
    pub layer: String,
    /// The text written (`YYYY-MM-DD` at midnight, else `YYYY-MM-DDTHH:MM:SS`): the date, or its day's midnight where
    /// the layer's start or end field is of the date kind.
    pub at: String,
}

/// What `cad.time.version` wrote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TimeVersioned {
    pub kind: TimeVersionKind,
    /// The objects ended at the date, in the input's order (a repeat once).
    pub ended: Vec<String>,
    /// Their new versions' persistent ids, each in its object's place in `ended`; none for Sona erdir.
    pub created: Vec<String>,
    /// The new versions' slots in the drawing, as `created`.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    /// The date written on each layer of the objects, in the order the layers are first met.
    pub layers: Vec<TimeVersionLayer>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
