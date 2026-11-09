//! Points between coordinate systems as a product command (docs/adr/0519 §3): `cad.crs.transform` moves points from
//! one system to another with their heights and says how: the operation's identity and steps, a PROJ pipeline that
//! does the same, the engine, the grids by name and SHA-256, the area of use, the stated accuracy; and keeps apart the
//! source's own uncertainty, the transformation's accuracy, the project's topology tolerance and the shown decimals.
//! Orthometric heights go through their side's geoid model and two epochs through a velocity grid, both kept on the
//! device by their SHA-256 (docs/adr/0631); without them the call is refused by name, never done by a weaker
//! fallback. It writes nothing.
//!
//! A side is the project's own system unless it names one: an EPSG code of the registry, with its heights and epoch.
//! Points are the side's own units: metres east and north on a grid, degrees longitude and latitude on a
//! geographic system; heights in the side's vertical unit.
//!
//! The answer also carries the coordinate chain (docs/adr/1230): each side's horizontal and vertical system, axes,
//! unit and epoch with where it came from, the grids read with their extents, what each step's accuracy rests on,
//! whether the chain is complete or limited and why, and the engine's numeric budget against PROJ.

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
/// is the project's and the project has none), `unsupported_srid` (a side is the project's and its stored EPSG code
/// is not one the registry resolves, docs/adr/1230 §1), `invalid_heights` (a side's heights against their rules),
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
    /// The velocity grid that carries the points from the source's epoch to the target's (docs/adr/0631 §2): a
    /// GeoTIFF of east, north and up velocities (mm/yr) this device keeps by its SHA-256. Without it two epochs are
    /// refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub velocity: Option<VelocityModel>,
}

/// A velocity grid by its name and the SHA-256 of its file (lower-case hex), as the device's grid library keeps it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VelocityModel {
    /// “TUSAGA-Aktif hız alanı 2023”.
    pub name: String,
    pub sha256: String,
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
    /// A geoid model's grid: an orthometric height made ellipsoidal, or back (docs/adr/0631 §1).
    Geoid,
    /// A velocity grid: the point carried to another epoch (docs/adr/0631 §2).
    Velocity,
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
/// a unit's change, with the ellipsoid through the datum's shift, through a geoid model (docs/adr/0631 §1).
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
    Geoid,
}

/// The operation a call ran, as a record: its sides, its identity (the datum steps' EPSG codes or the project's
/// names joined by “ + ”; empty when the datum stays), its steps, a PROJ pipeline that runs the same, the engine, the
/// grids, the area of use (the datum step's; none when only a projection changes or the step is the project's), the
/// stated accuracy (none when a step's is unknown), how heights went and their accuracy, the two epochs and whether
/// the operation carried the points between them (by a velocity grid, docs/adr/0631 §2), and whether an EPSG operation
/// of ED50 was used.
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

/// What a transformed point may lie beyond (docs/adr/3700, as a layer's objects in docs/adr/2500 §3), in this order:
/// EPSG's area of use of the source system (read at the point on the source's datum), of the datum transformation's
/// operation (the same place; none when the datum stays or the way is the project's own), of the target system (where
/// the point lands). A box is approximate (IOGP GN 7-1 §5.7): a note warns, it does not refuse.
pub const CRS_POINT_NOTES: [&str; 3] = ["outside_source_area", "outside_operation_area", "outside_target_area"];

/// A transformed point beyond an area of use (docs/adr/3700): its place in `points` and its notes ([`CRS_POINT_NOTES`]).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CrsPointNotes {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub index: u32,
    pub notes: Vec<String>,
}

/// Output of `cad.crs.transform` v1: the points in the target's units (none where unreached), the unreached ones,
/// the operation's record, the report and the coordinate chain (docs/adr/1230); the points beyond an area of use, each
/// with its notes (docs/adr/3700: the decision point by point, the chain's stays the call's).
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
    pub chain: CrsChain,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CrsPointNotes>>", optional))]
    pub flagged: Vec<CrsPointNotes>,
}

/// The families of horizontal systems KentOS's engine resolves (docs/adr/1230 §1). A system outside them, or an EPSG
/// code the registry does not hold, is refused by name: a stored code is not assumed resolvable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CrsFamily {
    /// Latitude and longitude on a datum's ellipsoid.
    Geographic,
    /// A transverse Mercator grid: the TM3 zones, UTM, or the project's own.
    TransverseMercator,
    /// WGS 84 / Pseudo-Mercator (EPSG:3857).
    PseudoMercator,
    /// A local system bound to a base system by a plane transform (the project's definition).
    Local,
}

/// Where a side's coordinate epoch came from: the call, the project's settings, the other side (the target takes
/// the source's when it states none), or nowhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EpochSource {
    Given,
    Project,
    Carried,
    None,
}

/// What a side's heights rest on, as the chain records it: their kind, the vertical system (EPSG code, name and
/// unit; an ellipsoidal height's is the horizontal datum's ellipsoid, in metres) and the geoid model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ChainVertical {
    pub kind: crate::heights::HeightKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    pub name: String,
    /// “m”, “ft”.
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geoid: Option<crate::heights::GeoidModel>,
}

/// One side of the chain (docs/adr/1230 §1): its horizontal system (EPSG code when the registry's, name, family,
/// datum, unit, the order KentOS reads x and y in and the order EPSG gives the axes), the datum ensemble's accuracy
/// when the datum is one (WGS 84: EPSG says 2 m), what its heights rest on, and its epoch with where it came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ChainSide {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    pub name: String,
    pub family: CrsFamily,
    /// `TUREF`, `ED50`, `WGS84`, or the project's datum's name.
    pub datum: String,
    /// “m” on a grid, “deg” on a geographic system.
    pub unit: String,
    /// The order x and y are: “east,north” or “longitude,latitude” (CLAUDE.md §5; whatever EPSG's order).
    pub axes: String,
    /// The order EPSG gives the axes: “ne”, “en” or “latlon”; none for the project's definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub epsg_axes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub ensemble_accuracy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub vertical: Option<ChainVertical>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub epoch: Option<f64>,
    pub epoch_source: EpochSource,
}

/// What a grid of the chain does: an NTv2 datum shift, a geoid model, a velocity grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GridKind {
    Horizontal,
    Geoid,
    Velocity,
}

/// A box in degrees: west, south, east, north.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DegreeBox {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub west: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub south: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub east: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub north: f64,
}

/// A grid the chain read (docs/adr/1230 §2): what it does, its name, its file's SHA-256 (its version: another file
/// is another grid), the box its nodes cover (degrees; a point beyond has no value) and the accuracy the project
/// states for it (none: not stated; a geoid's and a velocity grid's are not known here).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ChainGrid {
    pub kind: GridKind,
    pub name: String,
    pub sha256: String,
    pub extent: DegreeBox,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub accuracy: Option<f64>,
}

/// What a step's accuracy rests on (docs/adr/1230 §2): exact (a conversion, a unit change), EPSG's registry, the
/// project's own statement (its Helmert or grid), or nothing (a geoid model, a velocity grid).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AccuracyBasis {
    Exact,
    Epsg,
    Project,
    Unknown,
}

/// Whether the chain is complete (every step's accuracy stated, the epochs settled, every point reached, heights
/// carried by a stated rule) or limited, and why (docs/adr/1230 §3). A limited answer is never a precise one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ChainStatus {
    Complete,
    Limited,
}

/// The engine's numeric budget for the call (docs/adr/1230 §4): how far its numbers may lie from an independent
/// engine's (PROJ) for the same operation — the target's horizontal units (metres on a grid, degrees on a geographic
/// system) and metres of height. Not an accuracy: the transformation's is the report's.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NumericBudget {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub horizontal: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub vertical: f64,
}

/// The coordinate chain of a call (docs/adr/1230): the two sides, the basis of each step's accuracy (in the
/// operation's steps' order), the grids read, the verdict and its reasons (in a fixed order: `points_outside`,
/// `accuracy_unknown`, `height_accuracy_unknown`, `heights_unchanged`, `epoch_unknown`, `datum_ensemble`,
/// `unofficial_operation`) and the numeric budget.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CrsChain {
    pub source: ChainSide,
    pub target: ChainSide,
    pub bases: Vec<AccuracyBasis>,
    pub grids: Vec<ChainGrid>,
    pub status: ChainStatus,
    pub reasons: Vec<String>,
    pub budget: NumericBudget,
}

/// The engine's horizontal budget on a grid (m), on a geographic system (degrees), and of heights (m).
pub const CRS_BUDGET_METRES: f64 = 1e-6;
pub const CRS_BUDGET_DEGREES: f64 = 1e-9;
pub const CRS_BUDGET_HEIGHT: f64 = 1e-6;

/// The datum ensembles of the registry and their accuracy (m) as EPSG states it: WGS 84's (EPSG:6326) 2 m.
pub fn ensemble_accuracy(datum: &str) -> Option<f64> {
    (datum == "WGS84").then_some(2.0)
}
