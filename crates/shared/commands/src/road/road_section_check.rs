//! `road.terrains.sectioncheck` v1 (plan/map-cad.md MAP-14, docs/adr/5120): Kesit kontrolü, BÖHHBÜY m. 93(1)'s check
//! of a sheet's contour lines. A section runs between two control points of known height (a); along it the surveyor
//! measures points at most 20 m apart by levelling (b); each point's measured height is compared with the height
//! interpolated from the contour lines the section crosses, and the RMS of the differences must be under 1.5 · KOH_Z
//! for the scale (c, KOH_Z by m. 91(2)). The limits are the rule package's (`sectionCheck`). Read only: nothing is
//! written. The shared cases are `fixtures/commands/v1/road.terrains.sectioncheck.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::ContourRules;

pub const ROAD_TERRAINS_SECTIONCHECK: &str = "road.terrains.sectioncheck";
pub const ROAD_TERRAINS_SECTIONCHECK_VERSION: u32 = 1;

/// Input of `road.terrains.sectioncheck` v1: the section's two control points (`from`, `to`: points with their own
/// height), the points measured along it (`points`: points with their own height), the layers of the contour lines
/// (`contours`: polylines whose every vertex has one same height; others are counted, not read), the scale's
/// denominator (none: the project's plot scale), how far from the section's line a point may lie (`corridor`, metres;
/// none: no limit, the distance is only said) and the rule package (none: KentOS's BÖHHBÜY 2018).
///
/// Refusals, in this order: `invalid_uid`, `entity_not_found`, `not_a_height_point` (`from`, `to`, `points[i]`: not a
/// point, or one without its own height), `invalid_input` (`to`: the same place as `from`; `points`: none, or a point
/// given twice or as an end; `corridor`: not above nought), `layer_not_found` (`contours[i]`; `contours`: none),
/// `invalid_contours` (a scale of 0; a package without `sectionCheck`). Warning `section_check_exceeded`: the RMS is not
/// under the limit; warning `section_spacing_exceeded`: two neighbouring places of the section (its ends and its points
/// on it) are farther apart than the package's spacing.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsSectionCheck {
    pub from: String,
    pub to: String,
    pub points: Vec<String>,
    pub contours: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub corridor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<ContourRules>,
}

/// A point of the section: its object, its place, how far along the section from `from` its foot is and how far off
/// the line it lies (metres), its measured height, the height read from the contour lines (none: no contour crossing on
/// one side of it, or the point is not on the section), the two contour heights it is read between, and the measured
/// less the read height. `state`: `compared`, `noContour` (no crossing on one side), `outside` (its foot before `from`
/// or after `to`), `offLine` (farther than the corridor).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionPointRow {
    pub uid: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub along: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub measured: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub contour: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub between: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub difference: Option<f64>,
    pub state: String,
}

/// Two neighbouring places of the section farther apart than the package's spacing: how far along each is, and the gap.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionGap {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
}

/// Output of `road.terrains.sectioncheck` v1: the section's length and its ends' heights, each point in order along
/// it, the contour crossings found (how far along and the height), the contour lines not read (without one same
/// height), the gaps past the spacing, the scale and the package (`id release`), and the check: how many points
/// compared, their RMS and largest difference (metres; none without a compared point), KOH_Z and the limit
/// (`factor` · KOH_Z, metres), the spacing, and whether the RMS is under the limit (none without a compared point).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsSectionChecked {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub ends: [f64; 2],
    pub points: Vec<SectionPointRow>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub crossings: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unlevelled: u32,
    pub gaps: Vec<SectionGap>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: u32,
    pub rules: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub compared: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub largest: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub koh_z: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub limit: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub within: Option<bool>,
}
