//! `cad.pointclouds.network` v1 (plan/map-cad.md MAP-13, docs/adr/4720): Tarama ağı dengelemesi, read only. Three or
//! more scans registered together: each link (a pair of scans) is registered as `cad.pointclouds.register` registers
//! two (docs/adr/1800, 2300, 2700), and its result is an observation of the second scan's correction relative to the
//! first, weighted by its final normal matrix. The corrections of the scans not held fixed are adjusted together by
//! least squares, as a GNSS network adjusts its baselines: one first-order (small-angle) solve about a common centre.
//! Each link's residual says how far the network moves it from its own registration; each loop of links says how far
//! its registrations fail to close. Nothing is written: the scans' matrices go to Dönüşüm uygula (docs/adr/3100).
//! The shared cases are `fixtures/commands/v1/cad.pointclouds.network.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{RegisterReject, RegisterRobust};

pub const CAD_POINTCLOUDS_NETWORK: &str = "cad.pointclouds.network";
pub const CAD_POINTCLOUDS_NETWORK_VERSION: u32 = 1;

/// Input of `cad.pointclouds.network` v1 (plan/map-cad.md MAP-13, docs/adr/4720): the scans (point clouds' persistent
/// ids, at least three), those held where they are (at least one, not all), the links (pairs of scans; none: every
/// pair whose plan boxes overlap, in the scans' order), and the pairwise registration's own options (docs/adr/1800:
/// `spacing`, `reach`, `iterations`, `tolerance`, `neighbours`; docs/adr/2300 `robust`; docs/adr/2700 `reject`), the
/// same for every link. `drawing`: where a cloud's relative file is looked for.
///
/// Refusals: `invalid_uid`, `entity_not_found`, `not_a_pointcloud` (`uids[i]`); `invalid_input` (fewer than three scans,
/// one twice; no fixed scan, every scan fixed, a fixed scan not among the scans; a link twice, of one scan, of a scan not
/// among them; a scan joined to no fixed scan by the links; a link whose registration found fewer than six pairs; a
/// network that does not fix every scan's position, as flat ground leaves a horizontal shift free); the registration's
/// own refusals at its option (`spacing` …); the files' (`file_not_found`, `import_unreadable`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsNetwork {
    pub uids: Vec<String>,
    pub fixed: Vec<String>,
    /// The links, each two scans' ids (the first the one the second is registered to); none: every pair whose plan
    /// boxes overlap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub links: Option<Vec<[String; 2]>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub reach: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub iterations: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub neighbours: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub robust: Option<RegisterRobust>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reject: Option<RegisterReject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// A scan adjusted (docs/adr/4720): its id, whether it was held, its correction about the network's centre — the
/// rotation (ω, φ, κ; R = Rz(κ)·Ry(φ)·Rx(ω)) and the translation — and the matrix taking its points where they belong
/// (row by row, p ↦ R(p − centre) + centre + translation), for Dönüşüm uygula; for a free scan its horizontal strength in
/// the network: √(σz² / σh²) of its translation's covariance (σh² the larger plan eigenvalue), Tarama kaydı's
/// `horizontal` for the network — under 0.1 the network barely fixes it in plan (flat ground).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkScan {
    pub uid: String,
    pub fixed: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub translation: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub matrix: [f64; 16],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub horizontal: Option<f64>,
}

/// A link (docs/adr/4720): its two scans, its registration's pairs, overlap share, RMS after and strength
/// (`cad.pointclouds.register`'s), the second's correction relative to the first it observed (rotation, then
/// translation about the network's centre), the network's residual of it (adjusted − observed, the same order), and the
/// shift that residual makes along the pairs' normals (the square root of rᵀ·N·r over the pairs, metres).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkLink {
    pub scans: [String; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub pairs: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub overlap: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rms_after: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub strength: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub observed: [f64; 6],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub residual: [f64; 6],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub shift: f64,
}

/// A loop of links (docs/adr/4720): the scans it passes through in order (a held scan stands for every held one, which
/// do not move against each other), and how far its observations fail to close: the rotation's length (radians) and
/// the translation's (metres) of the sum round it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkLoop {
    pub scans: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub translation: f64,
}

/// Output of `cad.pointclouds.network` v1: the network's centre (the scans' plan boxes' middles' mean, their heights'
/// mean), the scans, the links, the loops, the network's RMS shift (the square root of Σ rᵀ·N·r over every link's
/// pairs, metres), and notes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsNetworked {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub centre: [f64; 3],
    pub scans: Vec<NetworkScan>,
    pub links: Vec<NetworkLink>,
    pub loops: Vec<NetworkLoop>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub shift: f64,
    pub notes: Vec<String>,
}
