//! Points between coordinate systems as a product command (docs/adr/0519 §3): `cad.crs.transform` moves points from
//! one system to another with their heights and says how: the operation's identity and steps, a PROJ pipeline that
//! does the same, the engine, the grids by name and SHA-256, the area of use, the stated accuracy; and keeps apart the
//! source's own uncertainty, the transformation's accuracy, the project's topology tolerance and the shown decimals.
//! A vertical or an epoch transformation KentOS has no engine for is refused by name, never done by a weaker
//! fallback. It writes nothing.
//!
//! A side is the project's own system unless it names one: an EPSG code of the registry, with its heights and epoch.
//! Points are the side's own units: metres east and north on a grid, degrees longitude and latitude on a
//! geographic system; heights in the side's vertical unit.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::heights::HeightReference;

/// Moves points between coordinate systems.
pub const CAD_CRS_TRANSFORM: &str = "cad.crs.transform";
pub const CAD_CRS_TRANSFORM_VERSION: u32 = 1;

/// The most points one call takes.
pub const MAX_CRS_POINTS: usize = 100_000;

/// The engine that answers, and the version of its rules (`Operation.engine`).
pub const CRS_ENGINE: &str = "KentOS geometry-core (kentos.crs/1)";

/// A point in a side's own units; `z` a height when there is one.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CrsPoint {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub z: Option<f64>,
}

/// One side of a transformation: a system of the registry by its EPSG code, or the project's own (absent `srid`);
/// what its heights are and its coordinate epoch. Absent `heights` and `epoch` take the project's when the system is
/// the project's, none otherwise.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CrsSide {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub heights: Option<HeightReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub epoch: Option<f64>,
}

/// The source's own uncertainty (a standard deviation the caller knows, metres): horizontal and vertical.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceUncertainty {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub vertical: Option<f64>,
}

/// Input of `cad.crs.transform` v1.
///
/// Refusals (`CommandError.code`), checked in this order: `nothing_to_transform` (none, or more than 100 000), `not_finite`
/// (a coordinate or height not finite), `unknown_srid` (a side's EPSG code not in the registry), `no_system` (a side
/// is the project's and the project has none), `invalid_heights` (a side's heights against their rules),
/// `invalid_epoch` (an epoch outside 1900–2100), `invalid_uncertainty` (not 0 or more); then the way:
/// `no_datum_link` (a datum of the project's without a way to WGS 84), `grid_missing` (the project's datum choice is
/// a grid this device does not have); `heights_unknown` (heights given and one side says what they are, the other
/// not), `geoid_required` (between ellipsoidal and orthometric heights: KentOS has no geoid engine),
/// `vertical_unsupported` (between two vertical datums), `epoch_unsupported` (two different epochs: KentOS has no
/// time-dependent transformation). Warnings: `heights_unchanged` (heights given, neither side says what they are:
/// they stay as given), `epoch_not_applied` (an epoch given, the operation is not time-dependent),
/// `unofficial_operation` (an EPSG operation of ED50, not the official transformation), `points_outside` (some
/// points have no value in the target: outside a projection or a grid).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CrsTransform {
    pub points: Vec<CrsPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from: Option<CrsSide>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub to: Option<CrsSide>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uncertainty: Option<SourceUncertainty>,
}

/// What a step of an operation is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StepKind {
    /// A projection's inverse or forward, or a local system's plane transform: exact.
    Conversion,
    /// Seven parameters on Earth-centred coordinates.
    Helmert,
    /// A null transformation (TUREF to WGS 84): coordinates stay.
    Null,
    /// An NTv2 grid.
    Grid,
    /// Heights from one unit into another within one vertical datum: exact.
    Unit,
}

/// One step of an operation: what it is, its identity (an EPSG code, the project's choice's name, the system it
/// converts), whether it runs reversed, its stated accuracy (m).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OperationStep {
    pub kind: StepKind,
    pub name: String,
    pub reverse: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub accuracy: Option<f64>,
}

/// A grid an operation reads: its file's name and SHA-256.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GridUse {
    pub name: String,
    pub sha256: String,
}

/// Where an operation may be used: its name and box (degrees, west, south, east, north).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AreaOfUse {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub west: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub south: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub east: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub north: f64,
}

/// How heights went: not at all (none given, or neither side says what they are), the same vertical reference,
/// a unit's change, with the ellipsoid through the datum's shift.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HeightMethod {
    None,
    Unchanged,
    Same,
    Unit,
    Ellipsoidal,
}

/// The operation a call ran, as a record: its sides, its identity (the datum steps' EPSG codes or the project's
/// names joined by “ + ”; empty when the datum stays), its steps, a PROJ pipeline that runs the same, the engine, the
/// grids, the area of use (the datum step's; none when only a projection changes or the step is the project's), the
/// stated accuracy (none when a step's is unknown), how heights went and their accuracy, the two epochs and whether
/// the operation took them into account (never, in this engine), and whether an EPSG operation of ED50 was used.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CrsOperation {
    pub from: String,
    pub to: String,
    pub id: String,
    pub steps: Vec<OperationStep>,
    pub pipeline: String,
    pub engine: String,
    pub grids: Vec<GridUse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub area: Option<AreaOfUse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub accuracy: Option<f64>,
    pub heights: HeightMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height_accuracy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub from_epoch: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub to_epoch: Option<f64>,
    pub epoch_applied: bool,
    pub unofficial: bool,
}

/// The four numbers a calculation's report keeps apart (docs/adr/0519 §4): the source's uncertainty (what the caller
/// gave), the transformation's stated accuracy, their root sum of squares when both are known (an estimate), the
/// project's topology tolerance (when points count as one, not an error) and the decimals and step lengths are shown
/// with (not an accuracy). Metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AccuracyReport {
    pub source: SourceUncertainty,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub transformation: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub transformation_vertical: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub combined: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub combined_vertical: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub topology_tolerance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub display_decimals: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub display_step: f64,
}

/// A point the target has no value for: its index and why (`outside`: a projection does not take it; `outsideGrid`:
/// the grid does not reach it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnreachedPoint {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub index: u32,
    pub reason: String,
}

/// Output of `cad.crs.transform` v1: the points in the target's units (none where unreached), the unreached ones,
/// the operation's record and the report.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CrsTransformed {
    pub points: Vec<Option<CrsPoint>>,
    pub unreached: Vec<UnreachedPoint>,
    pub operation: CrsOperation,
    pub report: AccuracyReport,
}
