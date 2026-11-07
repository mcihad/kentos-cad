//! `cad.pipes.check` v1 (plan/map-cad.md MAP-16, docs/adr/1640): Boru hattı denetimi, read only. The pipes of a
//! project's utility network (`cad.network.define`, the edges its layers and filters take) checked as a gravity line:
//! each pipe's inverts at its ends (two attributes, or its vertices' heights), its slope the way it flows, its diameter
//! (an attribute in a unit, against a catalogue of sizes), its cover over the crown from a terrain surface (measured at
//! every place the pipe crosses a triangle's edge, so the least cover is exact) or from the structures' cover levels,
//! and how it meets the others: detached parts, an end without a structure, an outlet higher than the lowest inlet.
//! A value that would fit read in another unit is reported as a likely unit mistake, never corrected.
//!
//! No limit is the program's: the least and the greatest cover and slope are the user's (the regulation's annexes,
//! the administration's specification or the project's own), and with none given only what is wrong in any project is
//! reported (a rising pipe, a crown above the ground, a pipe not connected). Nothing is written. The shared cases are
//! `fixtures/commands/v1/cad.pipes.check.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_PIPES_CHECK: &str = "cad.pipes.check";
pub const CAD_PIPES_CHECK_VERSION: u32 = 1;

/// The most pipes checked, and the most findings listed (the counts go on).
pub const PIPE_LIMIT: usize = 100_000;
pub const PIPE_FINDING_LIMIT: usize = 5000;
/// The most sizes a catalogue holds.
pub const PIPE_CATALOG_LIMIT: usize = 200;

/// The findings' kinds, in the order a pipe's are listed.
pub const PIPE_FINDINGS: [&str; 19] = [
    "detached",
    "noStructure",
    "noInvert",
    "invertUnit",
    "invertIsDepth",
    "reverseSlope",
    "flat",
    "lowSlope",
    "steepSlope",
    "outletAbove",
    "noDiameter",
    "diameterUnit",
    "notInCatalog",
    "noGround",
    "offSurface",
    "aboveGround",
    "shallow",
    "deep",
    "curved",
];

/// Where a pipe's inverts (the inside bottom's heights, metres) are read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "from", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PipeInverts {
    /// Two attributes: the invert at the pipe's drawn start and at its drawn end; between them it is linear along the
    /// pipe.
    Fields { start: String, end: String },
    /// The pipe's vertices' heights (a line's two, a polyline's every one): linear between them.
    Heights,
}

/// A length's unit as an attribute is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PipeUnit {
    Mm,
    Cm,
    M,
}

impl PipeUnit {
    /// Metres in one.
    pub fn metres(self) -> f64 {
        match self {
            Self::Mm => 0.001,
            Self::Cm => 0.01,
            Self::M => 1.0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Mm => "mm",
            Self::Cm => "cm",
            Self::M => "m",
        }
    }

    pub const ALL: [PipeUnit; 3] = [Self::Mm, Self::Cm, Self::M];
}

/// The pipes' inner diameter: the attribute, its unit and, optionally, the sizes a pipe may have (in that unit).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipeDiameter {
    pub field: String,
    pub unit: PipeUnit,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub catalog: Vec<f64>,
}

/// Where the ground over a pipe is read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "from", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PipeGround {
    /// A terrain surface of a Harita or Yol project (its id or name): the cover all along the pipe.
    Surface { surface: String },
    /// The cover level attribute of the structure (a junction of the network) at each of the pipe's ends: the cover at
    /// the ends only.
    Structures { field: String },
}

/// Input of `cad.pipes.check` v1.
///
/// Refusals, in this order: `invalid_pipe_check` (an empty attribute's name, a limit not above zero, a least above a
/// greatest, a catalogue of more than 200 sizes or a size not above zero); `unknown_network` (no network of the id or
/// name), `not_a_pipe_network` (a road network); `not_a_road_project` (a surface asked outside Harita and Yol), the
/// surface's own (`unknown_terrain`, `terrain_unbuildable`); `invalid_network` (the network does not build),
/// `tools_unavailable`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipesCheck {
    /// The network's id or name.
    pub network: String,
    pub inverts: PipeInverts,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub diameter: Option<PipeDiameter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ground: Option<PipeGround>,
    /// The least and the greatest cover over the crown (metres).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub min_cover: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_cover: Option<f64>,
    /// The least and the greatest slope, metres a metre (0.003 is 3 ‰).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub min_slope: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub max_slope: Option<f64>,
}

/// A finding: its kind (one of [`PIPE_FINDINGS`]), the pipe, where, its measure (a gap in height, a cover, a slope, a
/// part's length) and, for a unit mistake, the unit the value would fit in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipeFinding {
    pub kind: String,
    pub uid: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub at: [f64; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<String>,
}

/// A pipe as checked: its layer, length (metres, along it), whether it flows against its drawing (the network's
/// direction field), the inverts upstream and downstream, the slope the way it flows (falling: above zero), its
/// diameter (metres), the least cover and where, the greatest cover, and its findings' kinds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipeCheck {
    pub uid: String,
    pub layer: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub reversed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub upstream: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub downstream: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub slope: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub diameter: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub min_cover: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub min_cover_at: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_cover: Option<f64>,
    pub findings: Vec<String>,
}

/// A kind's count.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipeCount {
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
}

/// Output of `cad.pipes.check` v1: the network's id, what the cover is measured to (`crown` with a diameter, else
/// `invert`), the pipes in the network's order, the findings (at most 5000, pipe by pipe, each pipe's in
/// [`PIPE_FINDINGS`]' order), the counts of the kinds found, the connected parts and the total length.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipesChecked {
    pub network: String,
    pub cover_to: String,
    pub pipes: Vec<PipeCheck>,
    pub findings: Vec<PipeFinding>,
    pub counts: Vec<PipeCount>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub parts: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
}
