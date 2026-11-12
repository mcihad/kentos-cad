//! The networks' command of the product command catalog (docs/adr/0209
//! §11): `cad.network.define` writes a network's definition into the
//! project's settings, or takes one away. Ağlar writes through it on the web
//! (`apps/web/src/product`) and on the desktop (`crates/native/application`);
//! both pass the shared cases in `fixtures/commands/v1`. A network is a
//! project setting, as the topology rules are: writing it makes the drawing
//! dirty and is no undo step.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::network::NetworkDef;

/// Writes or removes a network's definition.
pub const CAD_NETWORK_DEFINE: &str = "cad.network.define";
pub const CAD_NETWORK_DEFINE_VERSION: u32 = 1;

/// What `cad.network.define` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NetworkDefineOperation {
    /// The network replaces the project's of its id, or goes last.
    Set,
    /// The network of `id` goes.
    Remove,
}

/// Input of `cad.network.define` v1.
///
/// - `set`: `network`, which replaces the project's network of its id or goes
///   last;
/// - `remove`: `id`.
///
/// Refusals (`CommandError.code`), checked in this order: `no_network` (set
/// without `network`), `no_id` (remove without `id`), `invalid_network` (the
/// network, or the project's networks with it, by `networks_problem`); then
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `unknown_network` (remove: the project has no network of the id). A
/// network naming a layer the drawing does not have is written, with the
/// warning `unknown_layer`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkDefine {
    pub operation: NetworkDefineOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub network: Option<NetworkDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.network.define` v1, and its plan: the project's networks as
/// the write leaves them and the revision after it (a plan: before it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkDefined {
    pub networks: Vec<NetworkDef>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
