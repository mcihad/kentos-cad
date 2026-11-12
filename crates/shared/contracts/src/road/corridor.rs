//! Templates, pavements, sample lines, superelevation, corridors, earthwork volumes and mass haul. A part of the road project's settings (docs/adr/8054 §9).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[allow(unused_imports)]
use super::*;

/// A component of a template's side, outward from the axis: its width and cross slope (a grade: rise over run, −0.025
/// is −2.5 %; falling outward negative; docs/adr/0977).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TemplateComponentDef {
    pub kind: TemplateComponentKind,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub width: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub slope: f64,
}

/// A ditch in a cut: its fore slope (horizontal per one vertical), its depth under the hinge and its bottom's width.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TemplateDitchDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub fore: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub depth: f64,
    /// Absent: 0 (a V ditch).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub bottom: f64,
}

/// A template's side: its components outward from the axis and its end condition: the cut and fill slopes (horizontal
/// per one vertical, 1.5 is 3/2) and an optional ditch in the cut.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TemplateSideDef {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TemplateComponentDef>>", optional))]
    pub components: Vec<TemplateComponentDef>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub fill: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ditch: Option<TemplateDitchDef>,
    /// The cut slope's benches (berme, docs/adr/0958; KCAD schema 177).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bench: Option<TemplateBenchDef>,
    /// The fill slope's benches (docs/adr/0958; KCAD schema 177).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill_bench: Option<TemplateBenchDef>,
}

/// A slope's benches (docs/adr/0958): every `height` m up a cut slope or down a fill slope a bench `width` m wide, its
/// surface falling `fall` outward (a grade, 0.04 is 4 %; absent: level; docs/adr/0977). From the project's geotechnical report: KentOS gives none.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TemplateBenchDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub fall: f64,
}

/// A template (docs/adr/0903 §3): its left side and its right; absent right: the left's mirror.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TemplateDef {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub left: TemplateSideDef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub right: Option<TemplateSideDef>,
    /// The pavement under the finish grade, both sides alike (docs/adr/0975 §2); absent: none (an older template).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pavement: Option<PavementDef>,
}

/// A pavement layer: its name (aşınma, binder, temel …), its thickness and an optional material code (a pay item's,
/// docs/adr/0909).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PavementLayerDef {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub thickness: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
}

/// How a pavement's layers end at a side's hinge (docs/adr/0975 §1).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PavementEdge {
    /// A vertical line at the hinge.
    Vertical,
    /// A line from the hinge falling `h` horizontal per one vertical, the layers carried out to it.
    Slope {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        h: f64,
    },
}

/// A template's pavement: its layers from the top down and how they end.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PavementDef {
    pub layers: Vec<PavementLayerDef>,
    pub edge: PavementEdge,
}

/// How a sample line set's stations are had (docs/adr/0903 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "method", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SectionStations {
    /// Every round `every` metres of km from `from` to `to` (the alignment's own stations) and both of them, with the
    /// curves' points and the profile's PVIs when `geometry`.
    #[serde(rename_all = "camelCase")]
    Range {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        from: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        to: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        every: f64,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        geometry: bool,
        /// On curves and clothoids every round `curve` metres too (docs/adr/0933 §3; schema 168).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        curve: Option<f64>,
    },
    /// Where the centreline's ground breaks (it crosses a triangle's edge of the set's surface) from `from` to `to`,
    /// kept so the ground read linearly between two sections is within `tolerance` metres of the exact profile, with
    /// no gap longer than `most` metres where the profile has a break to fill it, and with the curves' points and the
    /// profile's PVIs when `geometry` (docs/adr/0933 §3; schema 168).
    #[serde(rename_all = "camelCase")]
    Breaks {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        from: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        to: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        tolerance: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        most: Option<f64>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        geometry: bool,
    },
    /// The stations written.
    Stations {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        stations: Vec<f64>,
    },
    /// Where lines and polylines of the drawing (their persistent ids) first cross the alignment.
    Lines { uids: Vec<String> },
}

/// A sample line set (docs/adr/0903 §1–§2): on an alignment of its own PIs (its revision held), with an optional profile
/// on it, template and surface; its swath widths and how its stations are had.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionSetDef {
    pub id: String,
    pub name: String,
    pub alignment: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub alignment_revision: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub template: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    /// The superelevation schedule on the same alignment its lanes take their cross slopes from (docs/adr/0904 §3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub superelevation: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub right: f64,
    pub stations: SectionStations,
    /// Lines that meet on a curve's inside are shortened to their meeting, nearest first (docs/adr/0933 §1; schema
    /// 166); otherwise they are left and said.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub clip: bool,
}

/// A curve's superelevation (docs/adr/0904 §2): its PI, the full rate (a grade, 0.07 is 7 %), the runoff length (level crown to full
/// rate), the share of the runoff before the curve (0–1) and the extra width on its inside.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperCurveDef {
    pub pi: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub rate: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub runoff: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub on_tangent: f64,
    /// Absent: none.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub widening: f64,
}

/// A row of a superelevation schedule written by station: the alignment's own station and each side's slope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperRowDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub right: f64,
}

/// How a superelevation schedule is had (docs/adr/0904 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "method", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SuperelevationMethod {
    /// From the curves, rotating about the centreline.
    Curves { curves: Vec<SuperCurveDef> },
    /// The rows written.
    Stations { rows: Vec<SuperRowDef> },
}

/// A superelevation schedule (docs/adr/0904): on an alignment of its own PIs (its revision held), the normal cross
/// slopes of the left and the right side (grades, outward; falling outward negative; docs/adr/0977) and how it is had.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperelevationDef {
    pub id: String,
    pub name: String,
    pub alignment: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub alignment_revision: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub normal_left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub normal_right: f64,
    pub method: SuperelevationMethod,
    /// The line the traveled way turns about (docs/adr/0931 §1); absent: the centreline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pivot: Option<SuperelevationPivot>,
}

/// The axis of rotation of a superelevation (docs/adr/0931 §1): the centreline, an edge of the traveled way by side,
/// or the curve's inside or outside edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SuperelevationPivot {
    Centerline,
    LeftEdge,
    RightEdge,
    InsideEdge,
    OutsideEdge,
}

impl SuperelevationPivot {
    pub fn words(self) -> &'static str {
        match self {
            SuperelevationPivot::Centerline => "Eksen",
            SuperelevationPivot::LeftEdge => "Sol kenar",
            SuperelevationPivot::RightEdge => "Sağ kenar",
            SuperelevationPivot::InsideEdge => "İç kenar",
            SuperelevationPivot::OutsideEdge => "Dış kenar",
        }
    }
}

/// A corridor's region (docs/adr/0905 §1): the alignment's stations it spans (m) and its template; and an optional
/// slope transition (docs/adr/0960, KCAD schema 178): over its first `transition` m the sides' cut and fill slopes go
/// linearly from the region before's template's to its own. The region before ends where it starts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorRegionDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub template: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub transition: Option<f64>,
}

/// A corridor (docs/adr/0905): on an alignment of its own PIs (its revision held), its profile's elevations, the
/// surface its slopes meet, an optional superelevation schedule, the swath searched on each side (m), how often it
/// is sampled (the tangent and curve intervals, an optional chord tolerance; the profile's points, the critical
/// stations and the stations written) and its regions in order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorDef {
    pub id: String,
    pub name: String,
    pub alignment: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub alignment_revision: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub profile: String,
    pub surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub superelevation: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub right: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tangent: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub curve: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sag: Option<f64>,
    /// The profile's PVIs, its vertical curves' ends and its high and low points are sampled too.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub profile_points: bool,
    /// The superelevation schedule's critical stations are sampled too.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub critical: bool,
    /// Stations the user wrote (the alignment's own, m).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub stations: Vec<f64>,
    pub regions: Vec<CorridorRegionDef>,
    /// The chord height the profile's vertical curves are divided by (m; docs/adr/0939 §1; schema 170).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub vertical_sag: Option<f64>,
    /// The sections between stations measured and the stations refined (docs/adr/0939 §2; schema 170).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub refine: Option<CorridorRefineDef>,
}

/// How a corridor's sampling is checked (docs/adr/0939 §2): the plan distance and the height a section between two
/// stations may stray from their straight join (m; absent: not checked), and the least step an interval is halved to
/// (m). A section whose kind changes (cut to fill, a slope that meets nothing, none) halves it in any case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorRefineDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub plan: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least: f64,
}

/// How an earthwork volume is computed (docs/adr/0906 §2–§3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum VolumeMethod {
    /// The average end area between consecutive sections.
    EndArea,
    /// The prismoidal formula with a section in the middle of each pair.
    Prismoidal,
    /// Between the corridor's top surface and the terrain, the triangulations laid over each other.
    Surfaces,
    /// Between the corridor's top surface and the terrain on a regular grid (docs/adr/0974; schema 179): at each cell's
    /// centre the two heights' difference times the cell's area (the midpoint rule), the cell (`cell`, m) given by
    /// the volume; KentOS gives none.
    Grid,
}

/// The factors over a stretch of stations (docs/adr/0906 §4): the cut's (bank to the volume it makes, kabarma) and the
/// fill's (compacted to the volume it needs, sıkışma), each a multiplier; absent: not converted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeFactorDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub cut: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub fill: Option<f64>,
    /// The soil's densities from the soil report (docs/adr/0957; KCAD schema 176): the factors follow from them, the
    /// cut's bank over loose, the fill's compacted over loose (both volumes in loose m³, as a truck carries them); a
    /// row with densities writes no factor of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub densities: Option<VolumeDensities>,
    /// Where its factors or densities come from (the soil report, its borehole or test), in words (docs/adr/0957).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
}

impl VolumeFactorDef {
    /// The factors as the volume uses them: the densities' where given (cut ρ_bank/ρ_loose, fill ρ_compacted/ρ_loose),
    /// else the row's own.
    pub fn effective(&self) -> (Option<f64>, Option<f64>) {
        match &self.densities {
            Some(d) => (Some(d.bank / d.loose), Some(d.compacted / d.loose)),
            None => (self.cut, self.fill),
        }
    }
}

/// A soil's densities (t/m³): in place (bank, yerinde), loosened (loose, gevşek) and compacted in the fill (sıkışık).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeDensities {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "t/m3")))]
    pub bank: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "t/m3")))]
    pub loose: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "t/m3")))]
    pub compacted: f64,
}

/// The topsoil stripped over a stretch of stations (docs/adr/0906 §6): its depth (m) under the earthwork's footprint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeStripDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub depth: f64,
}

/// The fill's foundation benched (diş kazısı, docs/adr/0906 §6): where the ground falls across steeper than `slope`
/// (a grade, 0.2 is 20 %; docs/adr/0977) under a fill, steps `height` (m) high; both the project's (its regulation, its soil report).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeBenchDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub slope: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: f64,
}

/// A source a road answer was computed from, as it is now (docs/adr/1038, 1039, 1040): its id and revision (0 where
/// the project has no such source).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadSource {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
}

/// The terrain surface a road answer was computed on (docs/adr/1038, 1039, 1040): its id, the digest of what it is
/// made of now (the drawing's points and lines it was triangulated from, with its method; absent where the project
/// has no such surface) and the digest it was last accepted from (absent: never accepted). The two differ when the
/// drawing changed after the surface was accepted: the answer is the drawing's of now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadSurfaceSource {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub built: Option<String>,
}

/// What an earthwork volume is computed from, each as it is now (docs/adr/1040; ROAD-VOL-01): its corridor, the
/// corridor's alignment and profile, its regions' templates in order and once each, its superelevation schedule and
/// its terrain surface (the original ground; the design is the corridor's).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeSources {
    pub corridor: RoadSource,
    pub alignment: RoadSource,
    pub profile: RoadSource,
    pub templates: Vec<RoadSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub superelevation: Option<RoadSource>,
    pub surface: RoadSurfaceSource,
}

/// An approved earthwork volume's figures (docs/adr/0906 §5): when and by which revision of its definition, a note,
/// the totals and each stretch's cut and fill (from, to, cut, fill; the stretches with volume) as they were.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeApprovalDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    /// The day it was approved, YYYY-MM-DD.
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub factored_cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub factored_fill: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub strip: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub bench: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub rows: Vec<[f64; 4]>,
    /// Its number among the volume's approvals, from 1 (docs/adr/0950; schema 173); 0: approved before approvals were
    /// numbered.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub number: u32,
    /// How it ended, in the volume's history (docs/adr/0950; schema 173): none while it stands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ended: Option<VolumeApprovalEnd>,
    /// What the figures were computed from when approved, each as it was then (docs/adr/1041; schema 197): none on an
    /// approval made before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sources: Option<VolumeSources>,
    /// The method's algorithm by name and version they were computed by (docs/adr/1040, 1041; schema 197): none on an
    /// approval made before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub algorithm: Option<String>,
}

/// How an approval ended (docs/adr/0950): the day (YYYY-MM-DD; none where it was not given), whether it was revoked
/// (else a later approval took its place) and why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeApprovalEnd {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub at: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub revoked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
}

/// An earthwork volume (docs/adr/0906, Netpro's kübaj hesabı): a corridor's, by a method, with the factors by stations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeDef {
    pub id: String,
    pub name: String,
    pub corridor: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub method: VolumeMethod,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<VolumeFactorDef>>", optional))]
    pub factors: Vec<VolumeFactorDef>,
    /// The topsoil stripped by stations (a section method's).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<VolumeStripDef>>", optional))]
    pub strip: Vec<VolumeStripDef>,
    /// The fill's foundation benched (a section method's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bench: Option<VolumeBenchDef>,
    /// Its figures as approved; an approved volume's definition does not change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub approved: Option<VolumeApprovalDef>,
    /// Its approvals that ended, oldest first, each as it was approved with how it ended (docs/adr/0950; schema 173):
    /// never changed nor removed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<VolumeApprovalDef>>", optional))]
    pub history: Vec<VolumeApprovalDef>,
    /// Which two lines it measures between (docs/adr/0975 §2, Netpro's names); absent: the road's top against the
    /// ground, stripped when it strips (as before the rule).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rule: Option<VolumeRule>,
    /// The volume on a curve by the sections' moments (docs/adr/0934; schema 169): each section's effective area
    /// `A + κ·Q`; the average end area method only.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub curve: bool,
    /// The grid method's cell (m, docs/adr/0974; schema 179): 0.01 to 1000; the grid method only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cell: Option<f64>,
}

/// An earthwork volume's rule (docs/adr/0975 §2): the road's top (Yol) or the formation (Formasyon) against the ground
/// (Zemin) or the ground with its topsoil stripped (Bitkisel Toprak). The topsoil stripped is its own item (Sıyırma
/// Kazısı) whichever rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum VolumeRule {
    /// Yol–Zemin.
    RoadGround,
    /// Formasyon–Zemin.
    FormationGround,
    /// Yol–Bitkisel Toprak.
    RoadTopsoil,
    /// Formasyon–Bitkisel Toprak.
    FormationTopsoil,
}

impl VolumeRule {
    /// Netpro's name.
    pub fn words(self) -> &'static str {
        match self {
            VolumeRule::RoadGround => "Yol–Zemin",
            VolumeRule::FormationGround => "Formasyon–Zemin",
            VolumeRule::RoadTopsoil => "Yol–Bitkisel Toprak",
            VolumeRule::FormationTopsoil => "Formasyon–Bitkisel Toprak",
        }
    }

    pub fn formation(self) -> bool {
        matches!(self, VolumeRule::FormationGround | VolumeRule::FormationTopsoil)
    }

    pub fn topsoil(self) -> bool {
        matches!(self, VolumeRule::RoadTopsoil | VolumeRule::FormationTopsoil)
    }
}

/// A borrow site or a waste site on a mass haul diagram (docs/adr/0907 §1): its name, kind, station and the volume
/// (m³, in the volume sheet's state) it gives (borrow) or takes (waste) there.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulSiteDef {
    pub name: String,
    pub kind: HaulSiteKind,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub volume: f64,
    /// Where it lies off the road, in plan (docs/adr/0955; KCAD schema 175): it is reached along the diagram's haul
    /// road network, which says where it meets the alignment and how far; `station` is then that station as last
    /// written. None: on the road at `station`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<crate::Vec2>,
}

/// A mass haul diagram's haul road network (docs/adr/0955; KCAD schema 175): the line and polyline objects on these
/// layers are its roads; two join where an end of one lies within `connect` (m) of the other; a road joins the
/// alignment where an end lies within `reach` (m) of its centre line (none: `connect`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulNetworkDef {
    pub layers: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub connect: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub reach: Option<f64>,
}

/// A site's kind: material brought in (borrow, ödünç) or taken away (waste, döküm).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HaulSiteKind {
    Borrow,
    Waste,
}

/// A mass haul diagram's prices (docs/adr/0907 §5): the currency, the overhaul's price per m³·km beyond the free haul,
/// the waste's and the borrow's per m³ (absent: not priced), and a truck's payload (m³, in the volume sheet's state) for
/// the trips; all the project's (its contract and price list).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulCostDef {
    pub currency: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money/volume-km")))]
    pub overhaul: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money/volume")))]
    pub waste: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money/volume")))]
    pub borrow: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub payload: Option<f64>,
}

/// A mass haul diagram (docs/adr/0907): an earthwork volume's mass ordinate against a balance line (m³), with a free
/// haul distance (m; absent: none, every haul is overhaul) and borrow and waste sites.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulDef {
    pub id: String,
    pub name: String,
    pub volume: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub balance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub free: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<HaulSiteDef>>", optional))]
    pub sites: Vec<HaulSiteDef>,
    /// Its prices and the truck's payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cost: Option<HaulCostDef>,
    /// The cut unsuitable for fill, by stations (docs/adr/0944; KCAD schema 171).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<HaulUnsuitableDef>>", optional))]
    pub unsuitable: Vec<HaulUnsuitableDef>,
    /// What the haul road crosses (docs/adr/0944; KCAD schema 171).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<HaulBarrierDef>>", optional))]
    pub barriers: Vec<HaulBarrierDef>,
    /// Its plan as approved (docs/adr/0954; KCAD schema 174): a plan read later against it says what changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub approved: Option<HaulApprovalDef>,
    /// Its approvals that ended, oldest first, as they were (docs/adr/0954): never changed nor removed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<HaulApprovalDef>>", optional))]
    pub history: Vec<HaulApprovalDef>,
    /// Its haul road network, by which its sites off the road are reached (docs/adr/0955; KCAD schema 175).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub network: Option<HaulNetworkDef>,
}

/// A haul plan as approved (docs/adr/0954): its number from 1, the day and a note; what it was made from — the
/// diagram's revision, its volume's revision and approval number (0: not approved), the objective (cost, else the least
/// moment); what it was — the moment (m³·m), the volume moved (m³), the cost in all where priced, each site's capacity
/// and use; how it ended, in the history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulApprovalDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub number: u32,
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub volume_revision: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub volume_approval: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub cost: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume-m")))]
    pub moment: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub moved: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money")))]
    pub total: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<HaulApprovedSite>>", optional))]
    pub sites: Vec<HaulApprovedSite>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ended: Option<VolumeApprovalEnd>,
}

/// A borrow or waste site as a haul plan was approved: its name, capacity and use (m³).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulApprovedSite {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub capacity: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub used: f64,
}

/// The share of the cut unsuitable for fill between two stations (docs/adr/0944): percent, from the soil report;
/// KentOS gives none. It goes to waste sites only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulUnsuitableDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub share: f64,
}

/// Something the haul road crosses (docs/adr/0944): its name, station, and the extra distance a load goes round it
/// (m); absent: closed, nothing crosses.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulBarrierDef {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub detour: Option<f64>,
}

pub(super) fn side_problem(s: &TemplateSideDef, at: &str) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    if s.components.len() > MAX_TEMPLATE_COMPONENTS {
        return fail(format!("{at}.components"), &format!("bir yanda en çok {MAX_TEMPLATE_COMPONENTS} bileşen olur"));
    }
    for (j, c) in s.components.iter().enumerate() {
        if !(c.width.is_finite() && c.width > 0.0) {
            return fail(format!("{at}.components[{j}].width"), "bileşenin genişliği 0'dan büyük olmalı");
        }
        if !c.slope.is_finite() || c.slope.abs() > 1.0 {
            return fail(format!("{at}.components[{j}].slope"), "enine eğim oranla yazılır (−0,025 %−2,5'tir), en çok 1 (%100) olur");
        }
    }
    let positive = |v: f64| v.is_finite() && v > 0.0;
    if !positive(s.cut) {
        return fail(format!("{at}.cut"), "yarma şevi 0'dan büyük olmalı (bir düşeye yatay)");
    }
    if !positive(s.fill) {
        return fail(format!("{at}.fill"), "dolgu şevi 0'dan büyük olmalı (bir düşeye yatay)");
    }
    if let Some(d) = &s.ditch
        && !(positive(d.fore) && positive(d.depth) && d.bottom.is_finite() && d.bottom >= 0.0)
    {
        return fail(format!("{at}.ditch"), "hendeğin iç şevi ve derinliği 0'dan büyük, tabanı eksi olmamalı");
    }
    for (b, path) in [(&s.bench, "bench"), (&s.fill_bench, "fillBench")] {
        if let Some(b) = b
            && !(b.height.is_finite() && b.height > 0.0 && b.height <= 100.0 && b.width.is_finite() && b.width > 0.0 && b.width <= 50.0 && b.fall.is_finite() && (0.0..=0.5).contains(&b.fall))
        {
            return fail(format!("{at}.{path}"), "basamağın yüksekliği (0, 100], genişliği (0, 50] m, eğimi oranla 0–0,5 (%0–50) olur");
        }
    }
    None
}

/// What is wrong with the templates.
pub(super) fn templates_problem(list: &[TemplateDef]) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    if list.len() > MAX_TEMPLATES {
        return fail("templates".into(), &format!("en çok {MAX_TEMPLATES} tip kesit olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, t) in list.iter().enumerate() {
        let at = |field: &str| format!("templates[{i}].{field}");
        if !word_ok(&t.id) {
            return fail(at("id"), "tip kesitin kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(t.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir tip kesit var");
        }
        let name = t.name.trim();
        if name.is_empty() || name != t.name || t.name.chars().count() > 120 {
            return fail(at("name"), "tip kesitin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir tip kesit var");
        }
        if let Some(p) = side_problem(&t.left, &at("left")) {
            return Some(p);
        }
        if let Some(p) = t.right.as_ref().and_then(|r| side_problem(r, &at("right"))) {
            return Some(p);
        }
        if let Some(p) = t.pavement.as_ref().and_then(|p| pavement_problem(p, &at("pavement"))) {
            return Some(p);
        }
    }
    None
}

/// The most layers a pavement has, and the deepest it goes (m).
pub const MAX_PAVEMENT_LAYERS: usize = 12;
pub const MAX_PAVEMENT_DEPTH: f64 = 5.0;

/// What is wrong with a pavement (docs/adr/0975 §2): at least one layer, each named and in (0, 2] m, the whole not
/// deeper than 5 m; the edge's slope in (0, 100]. Whether its edge meets the layers is the template's (its last
/// component's slope) and is said when a section is built.
pub(super) fn pavement_problem(p: &PavementDef, at: &str) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    if p.layers.is_empty() || p.layers.len() > MAX_PAVEMENT_LAYERS {
        return fail(format!("{at}.layers"), &format!("üstyapının 1 ile {MAX_PAVEMENT_LAYERS} arasında tabakası olur"));
    }
    let mut depth = 0.0;
    for (i, l) in p.layers.iter().enumerate() {
        let name = l.name.trim();
        if name.is_empty() || name != l.name || l.name.chars().count() > 60 {
            return fail(format!("{at}.layers[{i}].name"), "tabakanın adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 60 karakter");
        }
        if !(l.thickness.is_finite() && l.thickness > 0.0 && l.thickness <= 2.0) {
            return fail(format!("{at}.layers[{i}].thickness"), "tabakanın kalınlığı 0'dan büyük, en çok 2 m olmalı");
        }
        if l.material.as_ref().is_some_and(|m| m.trim().is_empty() || m.chars().count() > 64) {
            return fail(format!("{at}.layers[{i}].material"), "malzeme kodu boş olmamalı, en çok 64 karakter");
        }
        depth += l.thickness;
    }
    if depth > MAX_PAVEMENT_DEPTH {
        return fail(format!("{at}.layers"), "üstyapının toplam kalınlığı en çok 5 m olur");
    }
    if let PavementEdge::Slope { h } = p.edge
        && !(h.is_finite() && h > 0.0 && h <= 100.0)
    {
        return fail(format!("{at}.edge.h"), "kenar şevi 0'dan büyük, en çok 100 olmalı (yatay / 1 düşey)");
    }
    None
}

/// What is wrong with the sample line sets: each on an alignment of its own PIs, its profile on the same alignment,
/// its template in the project.
pub(super) fn sections_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.sections;
    if list.len() > MAX_SECTION_SETS {
        return fail("sections".into(), &format!("en çok {MAX_SECTION_SETS} kesit takımı olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, s) in list.iter().enumerate() {
        let at = |field: &str| format!("sections[{i}].{field}");
        if !word_ok(&s.id) {
            return fail(at("id"), "kesit takımının kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(s.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir kesit takımı var");
        }
        let name = s.name.trim();
        if name.is_empty() || name != s.name || s.name.chars().count() > 120 {
            return fail(at("name"), "kesit takımının adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir kesit takımı var");
        }
        match road.alignments.iter().find(|a| a.id == s.alignment) {
            None => return fail(at("alignment"), "kesit takımının aksı projede yok"),
            Some(a) if a.offset_of.is_some() => {
                return fail(at("alignment"), "kesit takımı yalnız PI'larıyla kurulan aksa bağlanır");
            }
            _ => {}
        }
        if let Some(p) = &s.profile {
            match road.profiles.iter().find(|q| q.id == *p) {
                None => return fail(at("profile"), "kesit takımının boykesiti projede yok"),
                Some(q) if q.alignment != s.alignment => {
                    return fail(at("profile"), "boykesit kesit takımının aksında değil");
                }
                _ => {}
            }
        }
        if s.template.as_ref().is_some_and(|t| road.templates.iter().all(|q| q.id != *t)) {
            return fail(at("template"), "kesit takımının tip kesiti projede yok");
        }
        if let Some(e) = &s.superelevation {
            match road.superelevations.iter().find(|q| q.id == *e) {
                None => return fail(at("superelevation"), "kesit takımının dever çizelgesi projede yok"),
                Some(q) if q.alignment != s.alignment => {
                    return fail(at("superelevation"), "dever çizelgesi kesit takımının aksında değil");
                }
                _ => {}
            }
        }
        if s.surface.as_deref().is_some_and(|v| !word_ok(v)) {
            return fail(at("surface"), "yüzey bir yüzeyin kimliği olmalı");
        }
        for (field, v) in [("left", s.left), ("right", s.right)] {
            if !(v.is_finite() && v > 0.0 && v <= MAX_SWATH) {
                return fail(at(field), &format!("şerit genişliği 0'dan büyük, en çok {MAX_SWATH} m olmalı"));
            }
        }
        match &s.stations {
            SectionStations::Range { from, to, every, curve, .. } => {
                if !(from.is_finite() && to.is_finite() && from < to) {
                    return fail(at("stations.from"), "km aralığının başı sonundan küçük olmalı");
                }
                if !(every.is_finite() && *every > 0.0) {
                    return fail(at("stations.every"), "km aralığı 0'dan büyük olmalı");
                }
                if curve.is_some_and(|c| !(c.is_finite() && c > 0.0)) {
                    return fail(at("stations.curve"), "kurptaki aralık 0'dan büyük olmalı");
                }
            }
            SectionStations::Breaks { from, to, tolerance, most, .. } => {
                if !(from.is_finite() && to.is_finite() && from < to) {
                    return fail(at("stations.from"), "km aralığının başı sonundan küçük olmalı");
                }
                if !(tolerance.is_finite() && *tolerance >= 0.0) {
                    return fail(at("stations.tolerance"), "kot toleransı eksi olmayan bir sayı olmalı");
                }
                if most.is_some_and(|m| !(m.is_finite() && m > 0.0)) {
                    return fail(at("stations.most"), "en büyük aralık 0'dan büyük olmalı");
                }
                if s.surface.is_none() {
                    return fail(at("surface"), "arazi kırıklarında kesit bir yüzey ister");
                }
            }
            SectionStations::Stations { stations } => {
                if stations.is_empty() || stations.len() > MAX_SECTION_STATIONS {
                    return fail(at("stations.stations"), &format!("1 ile {MAX_SECTION_STATIONS} arası km olur"));
                }
                if !stations.iter().all(|v| v.is_finite()) {
                    return fail(at("stations.stations"), "km'ler sonlu olmalı");
                }
            }
            SectionStations::Lines { uids } => {
                if uids.is_empty() || uids.len() > MAX_SECTION_STATIONS {
                    return fail(at("stations.uids"), &format!("1 ile {MAX_SECTION_STATIONS} arası çizgi olur"));
                }
                if let Some(words) = list_problem(uids) {
                    return fail(at("stations.uids"), &words);
                }
            }
        }
    }
    None
}

/// What is wrong with the superelevation schedules: each on an alignment of its own PIs, its normal slopes and its
/// values finite. Their geometry is judged where it is built.
pub(super) fn superelevations_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.superelevations;
    if list.len() > MAX_SUPERELEVATIONS {
        return fail("superelevations".into(), &format!("en çok {MAX_SUPERELEVATIONS} dever çizelgesi olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, d) in list.iter().enumerate() {
        let at = |field: &str| format!("superelevations[{i}].{field}");
        if !word_ok(&d.id) {
            return fail(at("id"), "dever çizelgesinin kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(d.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir dever çizelgesi var");
        }
        let name = d.name.trim();
        if name.is_empty() || name != d.name || d.name.chars().count() > 120 {
            return fail(at("name"), "dever çizelgesinin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir dever çizelgesi var");
        }
        match road.alignments.iter().find(|a| a.id == d.alignment) {
            None => return fail(at("alignment"), "dever çizelgesinin aksı projede yok"),
            Some(a) if a.offset_of.is_some() => {
                return fail(at("alignment"), "dever çizelgesi yalnız PI'larıyla kurulan aksa bağlanır");
            }
            _ => {}
        }
        for (field, v) in [("normalLeft", d.normal_left), ("normalRight", d.normal_right)] {
            if !(v.is_finite() && v.abs() <= 1.0) {
                return fail(at(field), "normal enine eğim oranla yazılır (−0,025 %−2,5'tir), en çok 1 (%100) olur");
            }
        }
        match &d.method {
            SuperelevationMethod::Curves { curves } => {
                if curves.is_empty() || curves.len() > MAX_SUPERELEVATION_ROWS {
                    return fail(at("method.curves"), &format!("1 ile {MAX_SUPERELEVATION_ROWS} arası kurp olur"));
                }
                for (j, c) in curves.iter().enumerate() {
                    let ok = [c.rate, c.runoff, c.on_tangent, c.widening].iter().all(|v| v.is_finite());
                    if !word_ok(&c.pi) || !ok {
                        return fail(at(&format!("method.curves[{j}]")), "kurbun PI'si bir sözcük, değerleri sonlu sayılar olmalı");
                    }
                }
            }
            SuperelevationMethod::Stations { rows } => {
                if rows.is_empty() || rows.len() > MAX_SUPERELEVATION_ROWS {
                    return fail(at("method.rows"), &format!("1 ile {MAX_SUPERELEVATION_ROWS} arası satır olur"));
                }
                if !rows.iter().all(|r| r.station.is_finite() && r.left.is_finite() && r.right.is_finite()) {
                    return fail(at("method.rows"), "satırların km'si ve eğimleri sonlu sayılar olmalı");
                }
            }
        }
    }
    None
}

pub const MAX_HAULS: usize = 256;
pub const MAX_HAUL_SITES: usize = 64;

pub(super) fn hauls_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.hauls;
    if list.len() > MAX_HAULS {
        return fail("hauls".into(), &format!("en çok {MAX_HAULS} Brükner olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, h) in list.iter().enumerate() {
        let at = |field: &str| format!("hauls[{i}].{field}");
        // Its approvals (docs/adr/0954): days, notes, finite figures; the standing one not ended, the history's ended,
        // the numbers rising.
        let day = |d: &str| {
            let b = d.as_bytes();
            b.len() == 10 && b.iter().enumerate().all(|(k, c)| if k == 4 || k == 7 { *c == b'-' } else { c.is_ascii_digit() })
        };
        let approval = |a: &HaulApprovalDef, path: String| -> Option<(String, String)> {
            if !day(&a.at) || a.ended.as_ref().and_then(|e| e.at.as_deref()).is_some_and(|d| !day(d)) {
                return fail(path, "onay ve bitiş günleri YYYY-AA-GG biçiminde olmalı");
            }
            if a.note.as_ref().is_some_and(|n| n.chars().count() > 500) || a.ended.as_ref().and_then(|e| e.note.as_ref()).is_some_and(|n| n.chars().count() > 500) {
                return fail(path, "onay ve bitiş notları en çok 500 karakter olur");
            }
            let figures = [a.moment, a.moved].into_iter().chain(a.total).chain(a.sites.iter().flat_map(|s| [s.capacity, s.used]));
            if a.number == 0 || a.sites.len() > 1000 || !figures.into_iter().all(f64::is_finite) {
                return fail(path, "onaylı planın numarası 1'den başlar, sayıları sonlu, en çok 1000 alanı olur");
            }
            None
        };
        if let Some(a) = &h.approved {
            if let Some(p) = approval(a, at("approved")) {
                return Some(p);
            }
            if a.ended.is_some() {
                return fail(at("approved"), "geçerli onayın bitişi olmaz; biten onay geçmişe yazılır");
            }
        }
        if h.history.len() > 200 {
            return fail(at("history"), "onay geçmişi en çok 200 onay olur");
        }
        let mut last = 0;
        for (j, a) in h.history.iter().enumerate() {
            let path = at(&format!("history[{j}]"));
            if let Some(p) = approval(a, path.clone()) {
                return Some(p);
            }
            if a.ended.is_none() || a.number <= last {
                return fail(path, "geçmişteki onayların bitişi yazılır, numaraları artarak gider");
            }
            last = a.number;
        }
        if h.approved.as_ref().is_some_and(|a| a.number <= last) {
            return fail(at("approved"), "geçerli onayın numarası geçmiştekilerden büyük olur");
        }
        if !word_ok(&h.id) {
            return fail(at("id"), "Brüknerin kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(h.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir Brükner var");
        }
        let name = h.name.trim();
        if name.is_empty() || name != h.name || h.name.chars().count() > 120 {
            return fail(at("name"), "Brüknerin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir Brükner var");
        }
        if road.volumes.iter().all(|v| v.id != h.volume) {
            return fail(at("volume"), "Brüknerin kübaj hesabı projede yok");
        }
        if !h.balance.is_finite() || h.balance.abs() > 1e12 {
            return fail(at("balance"), "denge çizgisi sonlu bir hacim olmalı");
        }
        if h.free.is_some_and(|f| !(f.is_finite() && f > 0.0 && f <= 100_000.0)) {
            return fail(at("free"), "serbest taşıma uzaklığı 0'dan büyük, en çok 100 km olur");
        }
        if h.sites.len() > MAX_HAUL_SITES {
            return fail(at("sites"), &format!("en çok {MAX_HAUL_SITES} ödünç ve döküm alanı olur"));
        }
        for (j, site) in h.sites.iter().enumerate() {
            let path = at(&format!("sites[{j}]"));
            let nm = site.name.trim();
            if nm.is_empty() || nm != site.name || site.name.chars().count() > 120 {
                return fail(path, "alanın adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
            }
            if !site.station.is_finite() {
                return fail(path, "alanın km'si sonlu olmalı");
            }
            if !(site.volume.is_finite() && site.volume > 0.0 && site.volume <= 1e12) {
                return fail(path, "alanın hacmi 0'dan büyük olmalı");
            }
            // Docs/adr/0955: a site off the road lies somewhere and is reached by the network.
            if let Some(p) = &site.at {
                if !(p.x.is_finite() && p.y.is_finite()) {
                    return fail(path, "alanın konumu sonlu olmalı");
                }
                if h.network.is_none() {
                    return fail(path, "konumu verilen alana taşıma ağıyla gidilir; önce taşıma ağının katmanlarını verin");
                }
            }
        }
        if let Some(n) = &h.network {
            if n.layers.is_empty() || n.layers.len() > 20 || n.layers.iter().any(|l| l.is_empty() || l.chars().count() > 128) {
                return fail(at("network"), "taşıma ağının 1 ile 20 arası katmanı olur");
            }
            if !(n.connect.is_finite() && n.connect > 0.0 && n.connect <= 50.0) {
                return fail(at("network"), "bağlantı toleransı 0'dan büyük, en çok 50 m olur");
            }
            if n.reach.is_some_and(|r| !(r.is_finite() && r > 0.0 && r <= 200.0)) {
                return fail(at("network"), "aksa erişim uzaklığı 0'dan büyük, en çok 200 m olur");
            }
        }
        if let Some(c) = &h.cost {
            let cur = c.currency.trim();
            if cur.is_empty() || cur != c.currency || c.currency.chars().count() > 12 {
                return fail(at("cost"), "para birimi boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 12 karakter");
            }
            if [c.overhaul, c.waste, c.borrow].into_iter().flatten().any(|p| !(p.is_finite() && (0.0..=1e9).contains(&p))) {
                return fail(at("cost"), "birim fiyat 0 ya da daha büyük, sonlu olmalı");
            }
            if c.payload.is_some_and(|p| !(p.is_finite() && p > 0.0 && p <= 1000.0)) {
                return fail(at("cost"), "kamyon yükü 0'dan büyük, en çok 1000 m³ olur");
            }
        }
        if h.unsuitable.len() > MAX_HAUL_SITES {
            return fail(at("unsuitable"), &format!("en çok {MAX_HAUL_SITES} uygunsuz malzeme satırı olur"));
        }
        for (j, u) in h.unsuitable.iter().enumerate() {
            let path = at(&format!("unsuitable[{j}]"));
            if !(u.from.is_finite() && u.to.is_finite() && u.to > u.from) {
                return fail(path, "uygunsuz malzeme satırının sonu başından sonra olmalı");
            }
            if !(u.share.is_finite() && (0.0..=100.0).contains(&u.share)) {
                return fail(path, "uygunsuz malzemenin payı yüzde 0 ile 100 arası olur");
            }
            if h.unsuitable[..j].iter().any(|o| o.from < u.to && u.from < o.to) {
                return fail(path, "uygunsuz malzeme satırları kesişmemeli");
            }
        }
        if h.barriers.len() > MAX_HAUL_SITES {
            return fail(at("barriers"), &format!("en çok {MAX_HAUL_SITES} engel olur"));
        }
        for (j, b) in h.barriers.iter().enumerate() {
            let path = at(&format!("barriers[{j}]"));
            let nm = b.name.trim();
            if nm.is_empty() || nm != b.name || b.name.chars().count() > 120 {
                return fail(path, "engelin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
            }
            if !b.station.is_finite() {
                return fail(path, "engelin km'si sonlu olmalı");
            }
            if b.detour.is_some_and(|d| !(d.is_finite() && (0.0..=100_000.0).contains(&d))) {
                return fail(path, "dolaşma uzunluğu 0 ile 100 km arası olur");
            }
        }
    }
    None
}

pub(super) fn volumes_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.volumes;
    if list.len() > MAX_VOLUMES {
        return fail("volumes".into(), &format!("en çok {MAX_VOLUMES} kübaj hesabı olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, v) in list.iter().enumerate() {
        let at = |field: &str| format!("volumes[{i}].{field}");
        if !word_ok(&v.id) {
            return fail(at("id"), "kübaj hesabının kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(v.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir kübaj hesabı var");
        }
        let name = v.name.trim();
        if name.is_empty() || name != v.name || v.name.chars().count() > 120 {
            return fail(at("name"), "kübaj hesabının adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir kübaj hesabı var");
        }
        if road.corridors.iter().all(|c| c.id != v.corridor) {
            return fail(at("corridor"), "kübaj hesabının koridoru projede yok");
        }
        if v.factors.len() > MAX_VOLUME_FACTORS {
            return fail(at("factors"), &format!("en çok {MAX_VOLUME_FACTORS} katsayı satırı olur"));
        }
        for (j, f) in v.factors.iter().enumerate() {
            let path = at(&format!("factors[{j}]"));
            if !(f.from.is_finite() && f.to.is_finite() && f.to > f.from) {
                return fail(path, "katsayı satırının sonu başından sonra olmalı");
            }
            if j > 0 && f.from < v.factors[j - 1].to {
                return fail(path, "katsayı satırları artan km'lerle, birbirine binmeden yazılır");
            }
            if [f.cut, f.fill].iter().flatten().any(|k| !(k.is_finite() && *k > 0.0 && *k <= 10.0)) {
                return fail(path, "katsayı 0'dan büyük, en çok 10 olur");
            }
            // Docs/adr/0957: densities make the factors; a row has one or the other.
            if let Some(d) = &f.densities {
                if f.cut.is_some() || f.fill.is_some() {
                    return fail(path, "yoğunluk verilen satıra katsayı yazılmaz; katsayılar yoğunluklardan çıkar");
                }
                if [d.bank, d.loose, d.compacted].iter().any(|r| !(r.is_finite() && *r >= 0.5 && *r <= 4.0)) {
                    return fail(path, "yoğunluk 0,5 ile 4 t/m³ arası olur");
                }
            }
            if f.source.as_ref().is_some_and(|t| t.trim().is_empty() || t.chars().count() > 200) {
                return fail(path, "katsayının kaynağı boş olmaz, en çok 200 karakter");
            }
        }
        let sections = matches!(v.method, VolumeMethod::EndArea | VolumeMethod::Prismoidal);
        // Docs/adr/0974: the grid's cell, the grid method's only.
        match (v.method, v.cell) {
            (VolumeMethod::Grid, Some(c)) if c.is_finite() && (0.01..=1000.0).contains(&c) => {}
            (VolumeMethod::Grid, _) => return fail(at("cell"), "ızgara yöntemi 0,01 ile 1000 m arası bir hücre boyu (cell) ister"),
            (_, Some(_)) => return fail(at("cell"), "hücre boyu yalnız ızgara yönteminin"),
            _ => {}
        }
        if v.curve && v.method != VolumeMethod::EndArea {
            return fail(at("curve"), "kurp düzeltmesi yalnız ortalama alan yönteminde olur");
        }
        if !sections && (!v.strip.is_empty() || v.bench.is_some()) {
            return fail(at("method"), "sıyırma ve diş kazısı yalnız kesit yöntemleriyle (ortalama alan, prizmoidal) hesaplanır");
        }
        if let Some(r) = v.rule {
            if !sections && r != VolumeRule::RoadGround {
                return fail(at("rule"), &format!("{} kuralı yalnız kesit yöntemleriyle (ortalama alan, prizmoidal) hesaplanır", r.words()));
            }
            if r.topsoil() && v.strip.is_empty() {
                return fail(at("rule"), &format!("{} kuralı bitkisel toprak sıyırması ister: önce sıyırma kalınlıklarını (strip) verin", r.words()));
            }
        }
        if v.strip.len() > MAX_VOLUME_FACTORS {
            return fail(at("strip"), &format!("en çok {MAX_VOLUME_FACTORS} sıyırma satırı olur"));
        }
        for (j, f) in v.strip.iter().enumerate() {
            let path = at(&format!("strip[{j}]"));
            if !(f.from.is_finite() && f.to.is_finite() && f.to > f.from) {
                return fail(path, "sıyırma satırının sonu başından sonra olmalı");
            }
            if j > 0 && f.from < v.strip[j - 1].to {
                return fail(path, "sıyırma satırları artan km'lerle, birbirine binmeden yazılır");
            }
            if !(f.depth.is_finite() && f.depth > 0.0 && f.depth <= 5.0) {
                return fail(path, "bitkisel toprak kalınlığı 0'dan büyük, en çok 5 m olur");
            }
        }
        if let Some(b) = &v.bench
            && !(b.slope.is_finite() && b.slope > 0.0 && b.slope <= 10.0 && b.height.is_finite() && b.height > 0.0 && b.height <= 10.0)
        {
            return fail(at("bench"), "diş kazısının eğimi oranla 0'dan büyük, en çok 10 (%1000); basamak yüksekliği 0'dan büyük, en çok 10 m olur");
        }
        let is_day = |d: &str| {
            let day = d.as_bytes();
            day.len() == 10 && day.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() })
        };
        let approval = |a: &VolumeApprovalDef, path: String| -> Option<(String, String)> {
            if !is_day(&a.at) {
                return fail(path, "onay günü YYYY-AA-GG biçiminde olmalı");
            }
            if a.note.as_ref().is_some_and(|n| n.chars().count() > 500) {
                return fail(path, "onay notu en çok 500 karakter olur");
            }
            let numbers = [a.cut, a.fill, a.factored_cut, a.factored_fill].into_iter().chain(a.strip).chain(a.bench);
            if a.rows.len() > MAX_VOLUME_ROWS || !numbers.chain(a.rows.iter().flatten().copied()).all(f64::is_finite) {
                return fail(path, &format!("onaylı kübajın sayıları sonlu, en çok {MAX_VOLUME_ROWS} satır olur"));
            }
            if let Some(e) = &a.ended {
                if e.at.as_deref().is_some_and(|d| !is_day(d)) {
                    return fail(path, "onayın bitiş günü YYYY-AA-GG biçiminde olmalı");
                }
                if e.note.as_ref().is_some_and(|n| n.chars().count() > 500) {
                    return fail(path, "onayın bitiş notu en çok 500 karakter olur");
                }
            }
            // Docs/adr/1041: what it was computed from and by which algorithm.
            if let Some(s) = &a.sources
                && let Some(words) = volume_sources_problem(s)
            {
                return fail(format!("{path}.sources"), &words);
            }
            if a.algorithm.as_deref().is_some_and(|t| !algorithm_ok(t)) {
                return fail(
                    format!("{path}.algorithm"),
                    "algoritma ad/sürüm biçiminde olmalı (küçük harf, rakam, nokta ve tire; en çok 64 harf)",
                );
            }
            None
        };
        if let Some(a) = &v.approved {
            if let Some(p) = approval(a, at("approved")) {
                return Some(p);
            }
            if a.ended.is_some() {
                return fail(at("approved"), "geçerli onayın bitişi olmaz; biten onay geçmişe yazılır");
            }
        }
        if v.history.len() > MAX_VOLUME_HISTORY {
            return fail(at("history"), &format!("onay geçmişi en çok {MAX_VOLUME_HISTORY} onay olur"));
        }
        let mut last = 0;
        for (j, a) in v.history.iter().enumerate() {
            let path = at(&format!("history[{j}]"));
            if let Some(p) = approval(a, path.clone()) {
                return Some(p);
            }
            if a.ended.is_none() {
                return fail(path, "geçmişteki onayın bitişi yazılır");
            }
            if a.number != 0 && a.number <= last {
                return fail(path, "geçmişteki onayların sıra numaraları artarak gider");
            }
            last = last.max(a.number);
        }
        if v.approved.as_ref().is_some_and(|a| a.number != 0 && a.number <= last) {
            return fail(at("approved"), "geçerli onayın numarası geçmiştekilerden büyük olur");
        }
    }
    None
}

/// What is wrong with the corridors: each on an alignment of its own PIs, its profile on it, its schedule on it, its
/// regions' templates in the project, its numbers finite and in range. Its regions' geometry is judged where it is
/// built (docs/adr/0905 §1).
pub(super) fn corridors_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.corridors;
    if list.len() > MAX_CORRIDORS {
        return fail("corridors".into(), &format!("en çok {MAX_CORRIDORS} koridor olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, c) in list.iter().enumerate() {
        let at = |field: &str| format!("corridors[{i}].{field}");
        if !word_ok(&c.id) {
            return fail(at("id"), "koridorun kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(c.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir koridor var");
        }
        let name = c.name.trim();
        if name.is_empty() || name != c.name || c.name.chars().count() > 120 {
            return fail(at("name"), "koridorun adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir koridor var");
        }
        match road.alignments.iter().find(|a| a.id == c.alignment) {
            None => return fail(at("alignment"), "koridorun aksı projede yok"),
            Some(a) if a.offset_of.is_some() => return fail(at("alignment"), "koridor yalnız PI'larıyla kurulan aksa bağlanır"),
            _ => {}
        }
        match road.profiles.iter().find(|q| q.id == c.profile) {
            None => return fail(at("profile"), "koridorun boykesiti projede yok"),
            Some(q) if q.alignment != c.alignment => return fail(at("profile"), "boykesit koridorun aksında değil"),
            _ => {}
        }
        if !word_ok(&c.surface) {
            return fail(at("surface"), "yüzey bir yüzeyin kimliği olmalı");
        }
        if let Some(e) = &c.superelevation {
            match road.superelevations.iter().find(|q| q.id == *e) {
                None => return fail(at("superelevation"), "koridorun dever çizelgesi projede yok"),
                Some(q) if q.alignment != c.alignment => {
                    return fail(at("superelevation"), "dever çizelgesi koridorun aksında değil");
                }
                _ => {}
            }
        }
        for (field, v) in [("left", c.left), ("right", c.right)] {
            if !(v.is_finite() && v > 0.0 && v <= MAX_SWATH) {
                return fail(at(field), &format!("arama genişliği 0'dan büyük, en çok {MAX_SWATH} m olur"));
            }
        }
        for (field, v) in [("tangent", c.tangent), ("curve", c.curve)] {
            if !(v.is_finite() && v > 0.0 && v <= 1000.0) {
                return fail(at(field), "aralık 0'dan büyük, en çok 1000 m olur");
            }
        }
        if c.sag.is_some_and(|t| !(t.is_finite() && t > 0.0 && t <= 10.0)) {
            return fail(at("sag"), "sapma toleransı 0'dan büyük, en çok 10 m olur");
        }
        if c.stations.len() > MAX_SECTION_STATIONS || !c.stations.iter().all(|s| s.is_finite()) {
            return fail(at("stations"), &format!("en çok {MAX_SECTION_STATIONS} sonlu km yazılır"));
        }
        if c.vertical_sag.is_some_and(|t| !(t.is_finite() && t > 0.0 && t <= 10.0)) {
            return fail(at("verticalSag"), "düşey kiriş yüksekliği 0'dan büyük, en çok 10 m olur");
        }
        if let Some(r) = &c.refine {
            for (field, v) in [("plan", r.plan), ("height", r.height)] {
                if v.is_some_and(|t| !(t.is_finite() && t > 0.0 && t <= 10.0)) {
                    return fail(at(&format!("refine.{field}")), "sapma toleransı 0'dan büyük, en çok 10 m olur");
                }
            }
            if !(r.least.is_finite() && r.least >= 0.01 && r.least <= 100.0) {
                return fail(at("refine.least"), "en kısa adım 0,01 ile 100 m arası olur");
            }
        }
        if c.regions.is_empty() || c.regions.len() > MAX_CORRIDOR_REGIONS {
            return fail(at("regions"), &format!("1 ile {MAX_CORRIDOR_REGIONS} arası bölge olur"));
        }
        for (j, r) in c.regions.iter().enumerate() {
            if !(r.from.is_finite() && r.to.is_finite()) {
                return fail(at(&format!("regions[{j}]")), "bölgenin başı ve sonu sonlu sayılar olmalı");
            }
            if road.templates.iter().all(|t| t.id != r.template) {
                return fail(at(&format!("regions[{j}].template")), "bölgenin tip kesiti projede yok");
            }
            if let Some(l) = r.transition {
                if !(l.is_finite() && l > 0.0 && l <= r.to - r.from) {
                    return fail(at(&format!("regions[{j}].transition")), "şev geçişi 0'dan uzun, bölgeden kısa olur");
                }
                if j == 0 || (c.regions[j - 1].to - r.from).abs() > 1e-3 {
                    return fail(at(&format!("regions[{j}].transition")), "şev geçişi ancak önceki bölgenin bittiği yerde başlayan bölgede olur");
                }
            }
        }
    }
    None
}

/// What is wrong with a volume's sources as an approval keeps them (docs/adr/1041): ids are words, the templates at
/// most a corridor's regions, the surface's digests 64 lower-case hex digits.
pub(super) fn volume_sources_problem(s: &VolumeSources) -> Option<String> {
    let named = [&s.corridor, &s.alignment, &s.profile].into_iter().chain(&s.templates).chain(&s.superelevation);
    if named.into_iter().any(|r| !word_ok(&r.id)) || !word_ok(&s.surface.id) {
        return Some("kaynağın kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı".into());
    }
    if s.templates.len() > MAX_CORRIDOR_REGIONS {
        return Some(format!("en çok {MAX_CORRIDOR_REGIONS} tip kesit olur"));
    }
    if [&s.surface.digest, &s.surface.built].into_iter().flatten().any(|d| !digest_ok(d)) {
        return Some("arazinin özeti 64 küçük onaltılık rakam olmalı".into());
    }
    None
}
