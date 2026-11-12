//! `cad.asbuilt.check` v1 (plan/map-cad.md MAP-19, docs/adr/1700): Uygulama kontrolü, read only. The points measured
//! in the field (a layer of points with their names and heights) set against the design: the design's points (matched
//! by name, else, when a search radius is given, by the nearest design point within it, said as a proposal; two within
//! it are ambiguous and matched to neither), the design's surface (the height difference at every measured point) and
//! the design's lines (the signed normal offset to the nearest, left positive, for a point no design point matches
//! and, a radius given, within it of a line: a kerb's points, not the stake points beside it). Every limit is the user's (the
//! administration's or the contract's acceptance profile): a limit not given is not checked. A measured point's own
//! quality (its σ in an attribute) beyond the limit is reported apart from its difference: a poor solution is not a
//! construction fault.
//!
//! The design's digest (its points' names, places and heights) is in the answer. A check given the digest the field
//! record was made against says when the design has changed since, and still compares, so a record of an older design
//! is never matched to a newer one silently. Given the field package itself (docs/adr/2420), the check reads the digest,
//! the acceptance limits and the system from it; asked (`packageDesign`, docs/adr/2820), it compares the record with
//! the package's own design, the one the field had, whatever the drawing's design is now. Nothing is written. The shared
//! cases are `fixtures/commands/v1/cad.asbuilt.check.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_ASBUILT_CHECK: &str = "cad.asbuilt.check";
pub const CAD_ASBUILT_CHECK_VERSION: u32 = 1;

/// The most measured points a check takes.
pub const ASBUILT_LIMIT: usize = 100_000;

/// A measured point's flags, in the order they are listed.
pub const ASBUILT_FLAGS: [&str; 8] = [
    "proposed",
    "outsideHorizontal",
    "outsideVertical",
    "outsideSurface",
    "outsideLine",
    "lowQuality",
    "noHeight",
    "offSurface",
];

/// Input of `cad.asbuilt.check` v1. `measured` is a layer's id; `design` and `lines` too (at least one of `design`,
/// `surface`, `lines`); `surface` a terrain surface's id or name (Harita and Yol projects); `field` the attribute the
/// names are in (none: `Ad`); `search` the radius of a match by place (metres; none: by name only); the tolerances in
/// metres; `qualityField` the attribute of a point's σ (metres) and `quality` its limit; `expectedDesign` the design
/// digest the field record was made against.
///
/// Refusals, in this order: `invalid_asbuilt_check` (no design at all, a limit or radius not above zero, an empty
/// attribute's name, `quality` without `qualityField`, `packageDesign` without `package`; docs/adr/3620: a monitoring
/// job without the earlier epoch's layer or with a surface, lines, a radius or a package, control points outside a
/// monitoring job, an empty control point's name; docs/adr/4420: a series or a day outside a monitoring job, a day that
/// is no `YYYY-MM-DD` day; docs/adr/4840: a trend or thresholds without a series, a confidence not between 0 and 1, a
/// threshold not above zero, a warning not below its alarm); the package's (docs/adr/2420;
/// at `package`): `invalid_file_path` (empty, relative, not `.zip`), `file_not_found`, `invalid_asbuilt_check` (not a
/// field package; at `expectedDesign`: another digest than the package's); `layer_not_found` (`measured`, `design`,
/// `lines`, an epoch's `epochs[k].layer`); `invalid_asbuilt_check` (at `package`: its design files not read or not its digest's, docs/adr/2820; its
/// surface file not there, not read or not its SHA-256's, docs/adr/3220),
/// `not_a_road_project` and the surface's own (`unknown_terrain`, `terrain_unbuildable`), `invalid_asbuilt_check`
/// (more than 100 000 measured points).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltCheck {
    pub measured: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub design: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lines: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub search: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub vertical: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub quality_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub quality: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_design: Option<String>,
    /// The field package (`cad.field.package`'s `.zip`, an absolute path) the record was made with (docs/adr/2420): its
    /// design digest is the one expected, its acceptance limits apply where this input gives none (its quality limit
    /// only with `qualityField`), its system, epoch and heights are set against the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub package: Option<String>,
    /// The design compared is the package's (docs/adr/2820): its points (`noktalar.csv`) and lines (`cizgiler.csv`),
    /// read back to its digest; `design` and `lines`, given, are the drawing's design now, read only to say whether it
    /// changed since. Its surface too when it carries one (`yuzey.xml`, docs/adr/3220), read back to its SHA-256, given
    /// `surface` or not: `surface`, given, is then the drawing's surface now, read only to say whether it changed since
    /// (another surface, or its sources' digest not the package's). Needs `package`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub package_design: bool,
    /// The job's kind (docs/adr/3620): `asBuilt` (none), `stakeout` or `monitoring`. A monitoring job sets the points
    /// measured against an earlier epoch's (`design`: its layer), by name alone: no surface, line, place or package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<AsbuiltKind>,
    /// A monitoring job's control points by name (docs/adr/3620): marks taken as unmoved between the epochs; one beyond
    /// the limits says the two epochs do not rest on one datum (or the mark moved).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub control: Vec<String>,
    /// What the surface's limit (`vertical`) is held to (docs/adr/4020): the height difference (`vertical`, none) or the
    /// nearest distance to the surface (`nearest`, Trimble Access's Measure to surface).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface_by: Option<AsbuiltSurfaceBy>,
    /// A monitoring job's earlier epochs, oldest first (docs/adr/4420; Trimble Business Center's reference epoch the
    /// first): each a layer of points and its day. With them the answer tracks each point measured now through every
    /// epoch, its movement from the first epoch it was measured in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AsbuiltEpoch>>", optional))]
    pub epochs: Vec<AsbuiltEpoch>,
    /// This epoch's day, `YYYY-MM-DD` (docs/adr/4420): with the day of a point's first epoch, its mean rate of movement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date: Option<String>,
    /// A line fitted through each point's dated epochs (docs/adr/4840): its velocity, their standard deviations and
    /// which components move at the confidence asked. Needs a series (`epochs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub trend: Option<AsbuiltTrendAsk>,
    /// Two thresholds, warning and alarm (docs/adr/4840; Trimble's yellow and red): each point's state, the highest
    /// level it goes beyond. Needs a series (`epochs`). No threshold is the program's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub thresholds: Option<AsbuiltThresholds>,
}

/// The line fitted through a point's epochs (docs/adr/4840): the confidence its velocity's test is made at.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltTrendAsk {
    /// The confidence, between 0 and 1 (0.95: a component moves when its velocity is more than Student's t at 0.975
    /// with n − 2 degrees of freedom times its standard deviation). The user's: no regulation gives one for monitoring.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub confidence: f64,
}

/// A threshold level's limits; a limit not given is not checked at that level.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltLevel {
    /// The horizontal movement from the point's reference at this epoch (m).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub horizontal: Option<f64>,
    /// The height movement from the point's reference at this epoch, either way (m).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub vertical: Option<f64>,
    /// The horizontal speed (m a year): the fitted line's when there is one, else the mean rate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
    pub horizontal_rate: Option<f64>,
    /// The vertical speed, either way (m a year): the fitted line's when there is one, else the mean rate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
    pub vertical_rate: Option<f64>,
}

/// The two thresholds (docs/adr/4840); each limit given in both must be smaller at the warning.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltThresholds {
    /// The warning level (Trimble's yellow).
    pub warning: AsbuiltLevel,
    /// The alarm level (Trimble's red).
    pub alarm: AsbuiltLevel,
}

/// A point's state against the thresholds (docs/adr/4840): the highest level any of its values goes beyond.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AsbuiltState {
    Normal,
    Warning,
    Alarm,
}

/// The line fitted through a point's dated epochs (docs/adr/4840): by least squares, its movement from its reference
/// against the years since its first dated epoch, east, north and (with both heights) up apart.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltFit {
    /// The dated epochs the point was measured in (3 or more).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub epochs: u32,
    /// The velocity east and north (m a year).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
    pub east: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
    pub north: f64,
    /// The velocity up (m a year), when 3 dated epochs or more have both heights.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
    pub up: Option<f64>,
    /// Their standard deviations from the residuals: s / √Σ(t − t̄)², s² = Σr² / (n − 2) (m a year).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
    pub sigma_east: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
    pub sigma_north: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
    pub sigma_up: Option<f64>,
    /// The horizontal speed, √(east² + north²) (m a year).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
    pub speed: f64,
    /// The components whose velocity is more than Student's t times its standard deviation (`east`, `north`, `up`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub moving: Vec<String>,
}

/// An earlier epoch of a monitoring job's series (docs/adr/4420): its points' layer and its day (`YYYY-MM-DD`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltEpoch {
    pub layer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date: Option<String>,
}

/// A point measured now tracked through a monitoring job's series (docs/adr/4420): its name; `from`, the epoch it was
/// first measured in (its reference: the series is the earlier epochs, oldest first, then this one); at each epoch
/// its movement from the reference, none where it was not measured; with the days of its reference and of this epoch,
/// its mean rates (m a year: its last horizontal and vertical movement over the years between).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltTrack {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub from: u32,
    pub steps: Vec<Option<AsbuiltStep>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub rate_horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub rate_vertical: Option<f64>,
    /// The line fitted through its dated epochs (docs/adr/4840), asked and with 3 of them or more.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fit: Option<AsbuiltFit>,
    /// Its state against the thresholds (docs/adr/4840), asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub state: Option<AsbuiltState>,
    /// The limits of its state's level its values go beyond (`horizontal`, `vertical`, `horizontalRate`,
    /// `verticalRate`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub exceeded: Vec<String>,
}

/// A point's movement at an epoch from its reference (docs/adr/4420): east, north, height (none without both heights)
/// and horizontal; `outside` when it is beyond the job's horizontal or vertical limit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltStep {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub de: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub dn: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub dz: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub horizontal: f64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub outside: bool,
}

/// What a point's surface difference is measured as (docs/adr/4020).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AsbuiltSurfaceBy {
    /// Its height less the surface's under it.
    #[default]
    Vertical,
    /// Its distance to the surface's nearest place.
    Nearest,
}

/// What a check is for (docs/adr/3620; plan/map-cad.md MAP-19: as-built, stakeout check and periodic deformation
/// measurement as their own jobs): what was built against the design, stakes against their design points, or the same
/// marks at two epochs (physical movement, never a design change).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AsbuiltKind {
    #[default]
    AsBuilt,
    Stakeout,
    Monitoring,
}

/// A monitoring job's control points as found (docs/adr/3620): those matched at both epochs, those of them beyond a
/// limit, and those not measured at both.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltControl {
    pub checked: Vec<String>,
    pub moved: Vec<String>,
    pub missing: Vec<String>,
}

/// The field package a check was given (docs/adr/2420): the file, its stamp, the drawing's revision and the design
/// digest it was written with; its system (`EPSG:5256`), epoch and heights in words and whether each is the project's
/// now; its acceptance limits (metres) and which of them the check took (`horizontal`, `vertical`, `quality`: those the
/// input did not give).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltPackage {
    pub path: String,
    pub stamp: String,
    pub revision: String,
    pub digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub epsg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub epoch: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub heights: Option<String>,
    pub same_system: bool,
    pub same_epoch: bool,
    pub same_heights: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub vertical: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub quality: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub taken: Vec<String>,
    /// The design compared was the package's own (docs/adr/2820).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub design: bool,
    /// The surface compared was the package's own (`yuzey.xml`, docs/adr/3220).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub surface: bool,
}

/// A measured point as checked: its object and name; the design point it is matched to (`by`: `name`, `place` or
/// none) with the differences measured − design (east, north, height) and the horizontal distance; the difference to
/// the surface (measured − surface) and its nearest distance to the surface (docs/adr/4020: in space, signed as the
/// difference, with the nearest place); the signed normal offset to the nearest design line (left of its direction
/// positive) and that line (a line point only); its σ; its status and flags (of [`ASBUILT_FLAGS`]). The status:
/// `ambiguous`; `unmatched` when neither a design point nor a line takes it (a surface alone: every point is checked
/// against it); else `outside` when a limit is broken, `within`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltPoint {
    pub uid: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub design_uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub design_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub de: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dn: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dh: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub surface: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub nearest: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub nearest_at: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub normal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub sigma: Option<f64>,
    pub status: String,
    pub flags: Vec<String>,
}

/// A design point no measured point matched.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltMissing {
    pub uid: String,
    pub name: String,
}

/// Output of `cad.asbuilt.check` v1: the design's digest and whether it differs from the one expected; every measured
/// point in the drawing's order; the design points not measured; the counts by status and of the low-quality points;
/// the root mean square and the greatest of the matched points' horizontal distances and height differences.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltChecked {
    pub design_digest: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub design_changed: bool,
    /// docs/adr/3220: the drawing's surface (`surface`, given) is not the one the package names: another surface, or its
    /// sources changed since the package was written.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub surface_changed: bool,
    pub points: Vec<AsbuiltPoint>,
    pub missing: Vec<AsbuiltMissing>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub within: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outside: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unmatched: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub ambiguous: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub low_quality: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rms_horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rms_vertical: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_vertical: Option<f64>,
    /// The field package given (docs/adr/2420).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub package: Option<AsbuiltPackage>,
    /// The job's kind, as asked (docs/adr/3620).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<AsbuiltKind>,
    /// A monitoring job's control points (docs/adr/3620).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub control: Option<AsbuiltControl>,
    /// A monitoring job's series (docs/adr/4420): each point measured now and in another epoch, in this epoch's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AsbuiltTrack>>", optional))]
    pub tracks: Vec<AsbuiltTrack>,
}
