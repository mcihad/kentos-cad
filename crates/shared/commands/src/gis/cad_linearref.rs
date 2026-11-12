//! The linear-referencing events' command of the product command catalog (docs/adr/0569): `cad.linearref.define`
//! writes an event link into the project's settings, or takes one away. A link keeps the events of one layer on the
//! routes of others: any write that changes a route or an event places the layer's events again in its own step.
//! The desktop's handler is `crates/native/application/src/linearref_define.rs`; both it and the Python SDK pass the
//! shared cases in `fixtures/commands/v1`. A link is a project setting, as a network is: writing it makes the drawing
//! dirty and is no undo step.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::EventLink;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

/// Writes or removes an event link.
pub const CAD_LINEARREF_DEFINE: &str = "cad.linearref.define";
pub const CAD_LINEARREF_DEFINE_VERSION: u32 = 1;

/// What `cad.linearref.define` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LinearRefDefineOperation {
    /// The link replaces the project's of its id, or goes last.
    Set,
    /// The link of `id` goes; the events stay where they are.
    Remove,
}

/// Input of `cad.linearref.define` v1.
///
/// - `set`: `link`, which replaces the project's link of its id or goes last;
/// - `remove`: `id`.
///
/// Refusals (`CommandError.code`), checked in this order: `no_event_link` (set without `link`), `no_id` (remove without
/// `id`), `invalid_event_link` (the link, or the project's links with it, by `event_links_problem`); then
/// `invalid_revision`, `revision_conflict` (status `conflict`), `unknown_event_link` (remove: the project has no link of
/// the id). A link naming a layer the drawing does not have is written, with the warning `unknown_layer`; the link
/// does nothing until the layer exists.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LinearRefDefine {
    pub operation: LinearRefDefineOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub link: Option<EventLink>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.linearref.define` v1, and its plan: the project's links as the write leaves them and the
/// revision after it (a plan: before it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LinearRefDefined {
    pub links: Vec<EventLink>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
