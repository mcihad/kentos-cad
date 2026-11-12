//! Alt ağlar (docs/adr/0557 §2; plan/gis.md CBS-27): the parts of a utility network its sources feed. The network's
//! sources (its Kaynak junction layers) are the subnetwork controllers; a subnetwork is what a controller reaches along
//! the network's rules (its direction, its closed edges and junctions, its devices' terminals), not past another
//! controller. `cad.subnetworks.find` says, without changing anything, which subnetworks there are, what each holds,
//! what no controller reaches, what two reach, and what the attribute the network names for them says differently
//! (stale, missing or left over); `cad.subnetworks.update` writes the subnetworks' names into that attribute on every
//! object of the network, in one undo step.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::SubnetworkReach;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_SUBNETWORKS_FIND: &str = "cad.subnetworks.find";
pub const CAD_SUBNETWORKS_FIND_VERSION: u32 = 1;
pub const CAD_SUBNETWORKS_UPDATE: &str = "cad.subnetworks.update";
pub const CAD_SUBNETWORKS_UPDATE_VERSION: u32 = 1;

/// The most object ids a list of an answer names; its count says how many there are.
pub const SUBNETWORK_UIDS_LISTED: usize = 1000;

/// Input of `cad.subnetworks.find` v1.
///
/// Refusals (`CommandError.code`): `unknown_network` (the project has no network of that id), `no_subnetworks` (the
/// network defines no subnetworks), `no_controllers` (it has no source in the drawing).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SubnetworksFind {
    /// The network's id.
    pub network: String,
    /// Also name each subnetwork's objects (their persistent ids, at most 1000 each).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub members: bool,
}

/// Input of `cad.subnetworks.update` v1: writes the network's attribute for its subnetworks (what
/// `cad.subnetworks.find` reports) on every object of the network, in one undo step “Alt ağları güncelle”.
///
/// Refusals, in order: `unknown_network`, `no_subnetworks`, `no_controllers`; then `invalid_revision` and
/// `revision_conflict` (status `conflict`). Layers locked, or typed without the attribute's field, are left as they
/// are and said (`subnetwork_layer_skipped`); a value the attribute's field refuses is left and said
/// (`values_skipped`). With `plan`, nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SubnetworksUpdate {
    /// The network's id.
    pub network: String,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A subnetwork: one name, the controllers that carry it (the same name is one subnetwork), and what they reach.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Subnetwork {
    pub name: String,
    /// The controllers' persistent ids, in the drawing's order.
    pub controllers: Vec<String>,
    /// Edge objects reached, and junction objects reached (the controllers included).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub edges: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub junctions: u32,
    /// The length of the network's pieces reached.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    /// Connection problems of the network's check (near misses, crossings, …) that touch what it reached: a subnetwork
    /// over them is not one to rely on (docs/adr/0525 §2).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unverified: u32,
    /// With `members`: the objects' persistent ids (edges, then junctions), at most 1000.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub members: Vec<String>,
}

/// A group of the network's objects: how many, and the first persistent ids (at most 1000).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SubnetworkObjects {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
}

/// What the attribute holds that the subnetworks do not say: an object's stored value against the one the network's
/// subnetworks give it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SubnetworkStale {
    /// The object has a value that is not its subnetworks' names (another name, or one of several).
    pub changed: SubnetworkObjects,
    /// The object is in a subnetwork and has no value.
    pub missing: SubnetworkObjects,
    /// The object is in no subnetwork and has a value.
    pub left_over: SubnetworkObjects,
}

/// Input's answer: the network's subnetworks and what the attribute says of them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SubnetworksFound {
    /// The network's id.
    pub network: String,
    pub attribute: String,
    pub reach: SubnetworkReach,
    /// Controllers in the drawing: the network's source junctions.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub controllers: u32,
    /// The subnetworks in the order their first controller comes in the drawing.
    pub subnetworks: Vec<Subnetwork>,
    /// Objects of the network no controller reaches.
    pub orphans: SubnetworkObjects,
    /// Objects two or more controllers of different names reach (they hold both names).
    pub shared: SubnetworkObjects,
    /// The attribute against what the subnetworks say. All empty: it is up to date.
    pub stale: SubnetworkStale,
    pub revision: String,
}

/// What `cad.subnetworks.update` wrote, or with `plan` would write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SubnetworksUpdated {
    /// What `cad.subnetworks.find` says before the write (its `stale` says what the write changes); its `revision` is
    /// the output's.
    pub found: SubnetworksFound,
    /// Objects whose value is written, and objects left as they are (locked layer, a typed layer without the field, a
    /// value its field refuses).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub written: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub cleared: u32,
    pub revision: String,
}
