//! `cad.casework.run` v1 (“Mutfak dizisi”, docs/adr/0706): a straight run of kitchen base cabinets along a wall's
//! line, its sink and cooktop modules, a countertop over it and, if asked, wall cabinets above, as one undo step.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::{Entity, Vec2};

/// The command's name.
pub const CAD_CASEWORK_RUN: &str = "cad.casework.run";
/// Its version.
pub const CAD_CASEWORK_RUN_VERSION: u32 = 1;
/// A base cabinet's width when none is given (docs/adr/0706 §1): KentOS's default, the library's 60 cm.
pub const DEFAULT_CASEWORK_MODULE: f64 = 0.60;
/// The countertop's depth when none is given.
pub const DEFAULT_CASEWORK_DEPTH: f64 = 0.60;
/// The sink module's width: the library's 80 cm sink on a 90 cm base cabinet.
pub const CASEWORK_SINK_MODULE: f64 = 0.90;

/// Which side of its line `from`→`to` a run stands on, as one walks from `from` (the room's side).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CaseworkSide {
    #[default]
    Left,
    Right,
}

/// Input of `cad.casework.run` v1 (“Mutfak dizisi”, docs/adr/0706): base cabinets of `module` width from `from`
/// toward `to` along the line where their backs stand (a wall's face), on its `side`; the sink's module (`sink`,
/// 1-based) 0.90 m wide, the cooktop's (`cooktop`) the run's width; what is left at its end a filler. The countertop
/// covers the whole line, `depth` deep; `upper` adds wall cabinets over the modules. KentOS's library blocks (Alt
/// dolap, Evye, Ocak, Üst dolap) are brought in the same step when the drawing has none of their names; a block of
/// the drawing by that name is used as it is, its width parameter “Genişlik” set.
///
/// Refusals, in this order: `not_finite`, `invalid_casework` (a run with no length, a module or depth out of their
/// ranges, a run shorter than a module, a sink or cooktop module outside the run or both in one, a block without
/// its width parameter), `invalid_line_weight`, `invalid_revision`, `revision_conflict`, `layer_not_found`,
/// `not_a_layer`, `layer_locked`; on the desktop also `slots_exhausted`. Warnings: `layer_hidden`,
/// `casework_filler`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkRun {
    /// The layer the run goes on, a layer's id.
    pub layer_id: String,
    /// Where the run starts, on the line its backs stand on.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: Vec2,
    /// Where the line ends: the run is at most this long.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: Vec2,
    /// The side it stands on; absent: left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub side: Option<CaseworkSide>,
    /// A base cabinet's width (one of the cabinet block's widths: 0.30 to 1.20 m by 0.15); absent: 0.60.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub module: Option<f64>,
    /// The countertop's depth, from 0.30 to 1.20 m; absent: 0.60.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: Option<f64>,
    /// The sink's module, counted from `from` (1 the first); absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub sink: Option<u32>,
    /// The cooktop's module; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub cooktop: Option<u32>,
    /// Wall cabinets over the modules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub upper: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What a module of the run is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CaseworkModuleKind {
    Base,
    Sink,
    Cooktop,
}

/// One module of a run, along its line from `from`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkModule {
    pub kind: CaseworkModuleKind,
    /// Where it starts along the line.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub start: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
}

/// Plan of `cad.casework.run`: the objects it would write (`id` 0), the library blocks it would bring by name, its
/// modules and the filler at its end (0: none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkPlan {
    pub entities: Vec<Entity>,
    pub blocks: Vec<String>,
    pub modules: Vec<CaseworkModule>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub filler: f64,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Output of `cad.casework.run`: the objects written, in the plan's order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkBuilt {
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    pub blocks: Vec<String>,
    pub modules: Vec<CaseworkModule>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub filler: f64,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
