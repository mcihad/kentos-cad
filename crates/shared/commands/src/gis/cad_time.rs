//! A temporal layer's state at a time, as it was valid and as it was known (docs/adr/0537; plan/gis.md `CBS-33`).
//!
//! `cad.time.query` answers which objects of a temporal layer (ADR 0210) show for a valid time and a day known.
//! - The valid time is an instant (`at`) or a period [`from`, `to`), read by the layer time's rules.
//! - The day known (`knownAt`) keeps only the versions recorded by then and not yet superseded, by the layer's
//!   `recordedStart` and `recordedEnd` attributes (`.kcad` schema 90).
//!
//! “The zoning valid on a day” and “what the system knew on a day” are told apart. A moment's text is a date, or a date
//! and a time with an optional zone (`Z`, `+03:00`); without a zone it is UTC. The answer writes moments in UTC. It
//! writes nothing.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_TIME_QUERY: &str = "cad.time.query";
pub const CAD_TIME_QUERY_VERSION: u32 = 1;

/// Input of `cad.time.query` v1. Refusals: `no_such_step`, `invalid_file_path`, `file_not_found`, `import_unreadable`
/// (the state asked), `layer_not_found`, `not_a_layer`, `not_temporal`, `invalid_moment`, `invalid_period`,
/// `not_recorded`; the conflict `state_changed` (docs/adr/0646 §1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TimeQuery {
    /// The temporal layer's id.
    pub layer: String,
    /// The valid time's instant; with `from` and `to` refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub at: Option<String>,
    /// The valid time's period [from, to): both or neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub to: Option<String>,
    /// The day known: only the versions recorded by then and not yet superseded (a layer that records them).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub known_at: Option<String>,
    /// The state asked (docs/adr/0646 §1): the drawing some undo steps back or a drawing file; none, the drawing as
    /// it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub revision: Option<crate::CompareAgainst>,
    /// The layer's digest a former answer gave (`stamp`): the state must hold the layer as it did, else
    /// `state_changed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_stamp: Option<String>,
}

/// Output of `cad.time.query` v1: the objects shown (persistent ids, in the drawing's order) and why the others are
/// not; the moments read, in UTC.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TimeQueried {
    pub layer: String,
    /// The instant read (`YYYY-MM-DDTHH:MM:SSZ`, a midnight `YYYY-MM-DD`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub known_at: Option<String>,
    pub shown: Vec<String>,
    /// Objects with a valid time the window does not show.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub not_valid: u32,
    /// Objects whose version was not known on the day (recorded later, superseded by then, or never recorded).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub not_known: u32,
    /// Objects without a valid time (shown: a timeless object shows at every time).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub timeless: u32,
    /// Objects without a recorded start (with `knownAt` not shown).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unrecorded: u32,
    /// Values of the layer's time attributes that are not moments.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unreadable: u32,
    /// The state asked: “Şimdiki hâl”, “1 adım geri, “Taşı” öncesi” or the file's name.
    pub state: String,
    /// The layer's digest in that state (the SHA-256 of its objects' kinds, geometries and attributes by lasting
    /// id): given back as `expectedStamp`, the same question asks the same data.
    pub stamp: String,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}
