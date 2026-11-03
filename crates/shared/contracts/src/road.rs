//! A road project's own settings (plan/road.md Y2.2, docs/adr/0900 §10): its terrain surfaces' definitions. A surface
//! is kept as what it is made of, not as its triangles: the layers and objects its points and breaklines come from,
//! its boundary and voids, its height tolerance and edge method. Its triangles are made again from the drawing when it
//! is read (`kentos-road`'s `kentos.cdt/1`: exact and fast), so a surface never disagrees with its sources; `built`
//! keeps the digest of the sources it was last made and accepted from, and a surface whose sources' digest differs
//! is out of date (Civil 3D's “out of date”, OpenRoads' rule). Kept with the project's settings (`road`, `.kcad`
//! schema 158, the road project's schema).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The most terrain surfaces a project holds.
pub const MAX_TERRAIN_SURFACES: usize = 64;
/// The most layers or objects one source list names.
pub const MAX_SURFACE_SOURCES: usize = 10_000;
/// The most alignments a project holds.
pub const MAX_ALIGNMENTS: usize = 256;
/// The most PIs an alignment has.
pub const MAX_ALIGNMENT_PIS: usize = 10_000;
/// The most station equations an alignment has.
pub const MAX_STATION_EQUATIONS: usize = 100;
/// The farthest a parallel axis lies from its source, m.
pub const MAX_ALIGNMENT_OFFSET: f64 = 1000.0;
/// The most vertical alignments (profiles) a project holds.
pub const MAX_PROFILES: usize = 256;
/// The most PVIs a profile has.
pub const MAX_PROFILE_PVIS: usize = 10_000;
/// The most templates a project holds, and components a template's side has.
pub const MAX_TEMPLATES: usize = 256;
pub const MAX_TEMPLATE_COMPONENTS: usize = 32;
/// The most sample line sets a project holds, and stations or lines a set names.
pub const MAX_SECTION_SETS: usize = 256;
/// The most superelevation schedules a road project holds, curves or rows one holds.
pub const MAX_SUPERELEVATIONS: usize = 256;
pub const MAX_SUPERELEVATION_ROWS: usize = 10_000;
/// The most corridors a road project holds, regions a corridor holds (docs/adr/0905).
pub const MAX_CORRIDORS: usize = 256;
pub const MAX_CORRIDOR_REGIONS: usize = 256;
/// The most earthwork volumes a road project holds, factor rows one holds (docs/adr/0906).
pub const MAX_VOLUMES: usize = 256;
pub const MAX_VOLUME_FACTORS: usize = 1000;
/// At most so many approved stretches: a corridor's stations at most (`kentos_road::corridor::MAX_STATIONS`).
pub const MAX_VOLUME_ROWS: usize = 20000;
pub const MAX_SECTION_STATIONS: usize = 10_000;
/// The widest a sample line reaches on a side, m.
pub const MAX_SWATH: f64 = 1000.0;

/// A road project's settings; absent while it has none.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadSettings {
    /// The terrain surfaces, in the order the project tree shows them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TerrainSurfaceDef>>", optional))]
    pub surfaces: Vec<TerrainSurfaceDef>,
    /// The horizontal alignments (docs/adr/0901), in the order the project tree shows them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AlignmentDef>>", optional))]
    pub alignments: Vec<AlignmentDef>,
    /// The vertical alignments (docs/adr/0902), each on an alignment, in the order the project tree shows them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProfileDef>>", optional))]
    pub profiles: Vec<ProfileDef>,
    /// The templates (docs/adr/0903 §3), in the order the project tree shows them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TemplateDef>>", optional))]
    pub templates: Vec<TemplateDef>,
    /// The sample line sets (docs/adr/0903 §2), in the order the project tree shows them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SectionSetDef>>", optional))]
    pub sections: Vec<SectionSetDef>,
    /// The superelevation schedules (docs/adr/0904), each on an alignment, in the order the project tree shows them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SuperelevationDef>>", optional))]
    pub superelevations: Vec<SuperelevationDef>,
    /// The corridors (docs/adr/0905), each on an alignment, in the order the project tree shows them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CorridorDef>>", optional))]
    pub corridors: Vec<CorridorDef>,
    /// The earthwork volumes (docs/adr/0906), each of a corridor, in the order the project tree shows them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<VolumeDef>>", optional))]
    pub volumes: Vec<VolumeDef>,
    /// The mass haul diagrams (docs/adr/0907).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<HaulDef>>", optional))]
    pub hauls: Vec<HaulDef>,
    /// The plan–profile sheet sets (docs/adr/0908 §10), each along an alignment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<RoadSheetSetDef>>", optional))]
    pub sheet_sets: Vec<RoadSheetSetDef>,
}

impl RoadSettings {
    /// Whether it says nothing (and is not written).
    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
            && self.alignments.is_empty()
            && self.profiles.is_empty()
            && self.templates.is_empty()
            && self.sections.is_empty()
            && self.superelevations.is_empty()
            && self.corridors.is_empty()
            && self.volumes.is_empty()
            && self.hauls.is_empty()
            && self.sheet_sets.is_empty()
    }

    /// What is wrong with it, by the field's path and in words; none when it is right.
    pub fn problem(&self) -> Option<(String, String)> {
        road_problem(self)
    }
}

/// Where a surface's points or breaklines come from: every object of the layers (and of the groups' layers), and the
/// objects named by their persistent ids.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurfaceSources {
    /// Layer or group ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    /// Objects' persistent ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
}

impl SurfaceSources {
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty() && self.uids.is_empty()
    }
}

/// A terrain surface's definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainSurfaceDef {
    /// Its id, unique in the project; what a profile or a section names.
    pub id: String,
    /// Its name (“Mevcut zemin”), unique in the project.
    pub name: String,
    /// The points: each point object's places with an elevation.
    #[serde(default, skip_serializing_if = "SurfaceSources::is_empty")]
    pub points: SurfaceSources,
    /// The breaklines: each line's and polyline's path, every vertex with an elevation.
    #[serde(default, skip_serializing_if = "SurfaceSources::is_empty")]
    pub breaklines: SurfaceSources,
    /// The outer boundary: a closed polyline's or an area's id; its vertices take the surface's heights.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub boundary: Option<String>,
    /// The voids: closed polylines' or areas' ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub voids: Vec<String>,
    /// Two records at one place are one vertex when their heights are this close, m; absent: 0.001.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: Option<f64>,
    /// The edge method: an outer triangle whose outer edge is longer than this is left out, m; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub max_edge: Option<f64>,
    /// The digest (64 lower-case hex digits) of the sources it was last made and accepted from; absent: never.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub built: Option<String>,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

fn is_true(v: &bool) -> bool {
    *v
}

fn yes() -> bool {
    true
}

fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

/// A PI of an alignment (docs/adr/0901 §2): its persistent id, its place, and the curve on it. The first and the last
/// are the alignment's start and end and carry no curve.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentPi {
    /// Unique in the alignment; the elements' ids come from it (`p3/kurp`).
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub y: f64,
    /// The curve's radius; absent or 0: no curve (a kink).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: f64,
    /// The entry clothoid's length; absent or 0: none.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub spiral_in: f64,
    /// The exit clothoid's length; absent or 0: none.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub spiral_out: f64,
}

/// A parallel axis's source (docs/adr/0901 §8): the alignment and how far right of it (left negative).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentOffset {
    pub alignment: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: f64,
}

/// A station equation (docs/adr/0901 §6): the km where it is (read before it), the km after it, and whether the km
/// grows after it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StationEquationDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub back: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub ahead: f64,
    /// Absent: true.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub increasing: bool,
}

/// An alignment's km: the km at its start and its equations in order.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentStationing {
    /// Absent: 0.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub start: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<StationEquationDef>>", optional))]
    pub equations: Vec<StationEquationDef>,
}

impl AlignmentStationing {
    pub fn is_empty(&self) -> bool {
        self.start == 0.0 && self.equations.is_empty()
    }
}

/// A horizontal alignment's definition (docs/adr/0901): its PIs, or the alignment it is parallel to; its km.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentDef {
    /// Unique in the project; what a profile, a corridor or a parallel axis names.
    pub id: String,
    /// Its name (“Ana aks”), unique in the project.
    pub name: String,
    /// Grows by one with every change of its geometry or km; absent: 0.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    /// Its start, PIs and end; empty for a parallel axis.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AlignmentPi>>", optional))]
    pub pis: Vec<AlignmentPi>,
    /// A parallel axis's source; absent for an alignment of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub offset_of: Option<AlignmentOffset>,
    /// Its km; absent: from 0 with no equation. A parallel axis has its source's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stationing: Option<AlignmentStationing>,
}

/// A PVI of a vertical alignment (docs/adr/0902 §2): its persistent id, its station along the alignment (its own
/// horizontal length from the start, not its km), its elevation, and the curve on it: a symmetric parabola (`length`),
/// an asymmetric one (`lengthIn` and `lengthOut` together) or a circle (`radius`); none of them: no curve. The first
/// and the last carry none.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilePvi {
    /// Unique in the profile.
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub elevation: f64,
    /// A symmetric parabola's horizontal length; absent or 0: none.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    /// An asymmetric parabola's horizontal length before the PVI; absent or 0: none.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length_in: f64,
    /// … and after it.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length_out: f64,
    /// A circular curve's radius in the station–elevation plane; absent or 0: none.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: f64,
}

/// A vertical alignment's definition (docs/adr/0902): the alignment it lies on and that alignment's revision when it
/// was last made or rebound (a newer one makes it stale), its PVIs, and the surface whose terrain profile it is read
/// against.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileDef {
    /// Unique in the project.
    pub id: String,
    /// Its name (“Proje kotu”), unique in the project.
    pub name: String,
    /// The alignment's id.
    pub alignment: String,
    /// The alignment's revision it was made or rebound against; absent: 0.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub alignment_revision: u32,
    /// Grows by one with every change; absent: 0.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub pvis: Vec<ProfilePvi>,
    /// The terrain surface's id; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
}

/// A template component's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TemplateComponentKind {
    Lane,
    Shoulder,
}

/// A component of a template's side, outward from the axis: its width and cross slope (percent, falling outward
/// negative).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TemplateComponentDef {
    pub kind: TemplateComponentKind,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub width: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
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
}

/// A curve's superelevation (docs/adr/0904 §2): its PI, the full rate (percent), the runoff length (level crown to full
/// rate), the share of the runoff before the curve (0–1) and the extra width on its inside.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperCurveDef {
    pub pi: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
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
/// slopes of the left and the right side (percent, outward; falling outward negative) and how it is had.
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub normal_left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub normal_right: f64,
    pub method: SuperelevationMethod,
}

/// A corridor's region (docs/adr/0905 §1): the alignment's stations it spans (m) and its template.
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
/// percent under a fill, steps `height` (m) high; both the project's (its regulation, its soil report).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeBenchDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub slope: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: f64,
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
}

fn is_zero_f64(v: &f64) -> bool {
    *v == 0.0
}

/// What is wrong with a road project's settings, by the field's path and in words.
pub fn road_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    if road.surfaces.len() > MAX_TERRAIN_SURFACES {
        return fail("surfaces".into(), &format!("en çok {MAX_TERRAIN_SURFACES} yüzey olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, s) in road.surfaces.iter().enumerate() {
        let at = |field: &str| format!("surfaces[{i}].{field}");
        if !word_ok(&s.id) {
            return fail(at("id"), "yüzeyin kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(s.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir yüzey var");
        }
        let name = s.name.trim();
        if name.is_empty() || name != s.name || s.name.chars().count() > 120 {
            return fail(at("name"), "yüzeyin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir yüzey var");
        }
        for (field, list) in [("points", &s.points), ("breaklines", &s.breaklines)] {
            if let Some(words) = list_problem(&list.layers).or_else(|| list_problem(&list.uids)) {
                return fail(at(field), &words);
            }
        }
        if s.points.is_empty() && s.breaklines.is_empty() {
            return fail(at("points"), "yüzeyin noktası ya da kırık hattı olmalı");
        }
        if s.boundary.as_deref().is_some_and(|b| !word_ok(b)) {
            return fail(at("boundary"), "sınır bir nesnenin kimliği olmalı");
        }
        if let Some(words) = list_problem(&s.voids) {
            return fail(at("voids"), &words);
        }
        if s.boundary.as_ref().is_some_and(|b| s.voids.contains(b)) {
            return fail(at("voids"), "sınır aynı zamanda boşluk olamaz");
        }
        if s.tolerance.is_some_and(|t| !(t.is_finite() && t > 0.0 && t <= 1.0)) {
            return fail(at("tolerance"), "kot toleransı 0'dan büyük, en çok 1 m olmalı");
        }
        if s.max_edge.is_some_and(|l| !(l.is_finite() && l > 0.0)) {
            return fail(at("maxEdge"), "en uzun dış kenar 0'dan büyük olmalı");
        }
        if s
            .built
            .as_deref()
            .is_some_and(|d| d.len() != 64 || !d.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
        {
            return fail(at("built"), "özet 64 küçük onaltılık rakam olmalı");
        }
    }
    alignments_problem(&road.alignments)
        .or_else(|| profiles_problem(&road.profiles, &road.alignments))
        .or_else(|| templates_problem(&road.templates))
        .or_else(|| sections_problem(road))
        .or_else(|| superelevations_problem(road))
        .or_else(|| corridors_problem(road))
        .or_else(|| volumes_problem(road))
        .or_else(|| hauls_problem(road))
        .or_else(|| sheet_sets_problem(road))
}

fn side_problem(s: &TemplateSideDef, at: &str) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    if s.components.len() > MAX_TEMPLATE_COMPONENTS {
        return fail(format!("{at}.components"), &format!("bir yanda en çok {MAX_TEMPLATE_COMPONENTS} bileşen olur"));
    }
    for (j, c) in s.components.iter().enumerate() {
        if !(c.width.is_finite() && c.width > 0.0) {
            return fail(format!("{at}.components[{j}].width"), "bileşenin genişliği 0'dan büyük olmalı");
        }
        if !c.slope.is_finite() || c.slope.abs() > 100.0 {
            return fail(format!("{at}.components[{j}].slope"), "enine eğim en çok %100 olur");
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
    None
}

/// What is wrong with the templates.
fn templates_problem(list: &[TemplateDef]) -> Option<(String, String)> {
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
    }
    None
}

/// What is wrong with the sample line sets: each on an alignment of its own PIs, its profile on the same alignment,
/// its template in the project.
fn sections_problem(road: &RoadSettings) -> Option<(String, String)> {
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
            SectionStations::Range { from, to, every, .. } => {
                if !(from.is_finite() && to.is_finite() && from < to) {
                    return fail(at("stations.from"), "km aralığının başı sonundan küçük olmalı");
                }
                if !(every.is_finite() && *every > 0.0) {
                    return fail(at("stations.every"), "km aralığı 0'dan büyük olmalı");
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
fn superelevations_problem(road: &RoadSettings) -> Option<(String, String)> {
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
            if !(v.is_finite() && v.abs() <= 100.0) {
                return fail(at(field), "normal enine eğim en çok %100 olur");
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

/// What is wrong with the earthwork volumes: each of a corridor in the project, its factors positive and over
/// stretches in increasing stations that do not overlap.
/// At most so many mass haul diagrams, and so many sites on one.
pub const MAX_SHEET_SETS: usize = 256;
pub const MAX_ROAD_SHEETS: usize = 500;

/// A sheet of a plan–profile set: the book's sheet, the stations it shows (internal, m) and the profile view drawn for
/// it (its frame's lower left and upper right), when the set has a profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadSheetDef {
    pub sheet: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub view: Option<[[f64; 2]; 2]>,
}

/// A plan–profile sheet set (docs/adr/0908 §10; Civil 3D's view frame group): sheets along an alignment, each showing
/// its stretch in plan at the set's scale (1:`scale`) and, with a profile, the profile view drawn for it; the sources'
/// revisions it was made from, so the sheets say when the alignment or the profile changed since.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadSheetSetDef {
    pub id: String,
    pub name: String,
    pub alignment: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub alignment_revision: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub profile_revision: u32,
    /// A section sheet set's sample line set (docs/adr/0908 §11) and its revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sections: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub sections_revision: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: u32,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub overlap: f64,
    pub sheets: Vec<RoadSheetDef>,
}

fn sheet_sets_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.sheet_sets;
    if list.len() > MAX_SHEET_SETS {
        return fail("sheetSets".into(), &format!("en çok {MAX_SHEET_SETS} pafta takımı olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, t) in list.iter().enumerate() {
        let at = |field: &str| format!("sheetSets[{i}].{field}");
        if !word_ok(&t.id) {
            return fail(at("id"), "pafta takımının kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(t.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir pafta takımı var");
        }
        let name = t.name.trim();
        if name.is_empty() || name != t.name || t.name.chars().count() > 120 {
            return fail(at("name"), "pafta takımının adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir pafta takımı var");
        }
        match road.alignments.iter().find(|a| a.id == t.alignment) {
            None => return fail(at("alignment"), "pafta takımının aksı projede yok"),
            Some(a) if a.offset_of.is_some() => return fail(at("alignment"), "pafta takımı paralel aksta olmaz"),
            Some(_) => {}
        }
        if let Some(p) = &t.profile
            && road.profiles.iter().all(|q| q.id != *p || q.alignment != t.alignment)
        {
            return fail(at("profile"), "pafta takımının boykesiti projede aynı aksta yok");
        }
        if let Some(k) = &t.sections
            && road.sections.iter().all(|q| q.id != *k || q.alignment != t.alignment)
        {
            return fail(at("sections"), "pafta takımının kesit takımı projede aynı aksta yok");
        }
        if !(t.from.is_finite() && t.to.is_finite() && t.from <= t.to) {
            return fail(at("to"), "aralığın başı sonundan sonra olamaz, ikisi sonlu olmalı");
        }
        if t.scale == 0 || t.scale > 1_000_000 {
            return fail(at("scale"), "ölçek 1 ile 1 000 000 arası olmalı");
        }
        if !(t.overlap.is_finite() && t.overlap >= 0.0) {
            return fail(at("overlap"), "bindirme 0 ya da büyük, sonlu olmalı");
        }
        if t.sheets.is_empty() || t.sheets.len() > MAX_ROAD_SHEETS {
            return fail(at("sheets"), &format!("pafta takımında 1 ile {MAX_ROAD_SHEETS} arası pafta olur"));
        }
        let mut sheets = std::collections::BTreeSet::new();
        for (j, sh) in t.sheets.iter().enumerate() {
            let path = at(&format!("sheets[{j}]"));
            if !word_ok(&sh.sheet) || !sheets.insert(sh.sheet.as_str()) {
                return fail(path, "paftanın kimliği tek, boş olmayan bir sözcük olmalı");
            }
            if !(sh.from.is_finite() && sh.to.is_finite() && sh.from <= sh.to) {
                return fail(path, "paftanın aralığının başı sonundan sonra olamaz");
            }
            if sh.view.is_some_and(|v| v.iter().flatten().any(|c| !c.is_finite())) {
                return fail(path, "boykesit görünümünün çerçevesi sonlu olmalı");
            }
        }
    }
    None
}

pub const MAX_HAULS: usize = 256;
pub const MAX_HAUL_SITES: usize = 64;

fn hauls_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.hauls;
    if list.len() > MAX_HAULS {
        return fail("hauls".into(), &format!("en çok {MAX_HAULS} Brükner olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, h) in list.iter().enumerate() {
        let at = |field: &str| format!("hauls[{i}].{field}");
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
    }
    None
}

fn volumes_problem(road: &RoadSettings) -> Option<(String, String)> {
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
        }
        let sections = v.method != VolumeMethod::Surfaces;
        if !sections && (!v.strip.is_empty() || v.bench.is_some()) {
            return fail(at("method"), "sıyırma ve diş kazısı yalnız kesit yöntemleriyle (ortalama alan, prizmoidal) hesaplanır");
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
            && !(b.slope.is_finite() && b.slope > 0.0 && b.slope <= 1000.0 && b.height.is_finite() && b.height > 0.0 && b.height <= 10.0)
        {
            return fail(at("bench"), "diş kazısının eğimi 0'dan büyük, en çok %1000; basamak yüksekliği 0'dan büyük, en çok 10 m olur");
        }
        if let Some(a) = &v.approved {
            let day = a.at.as_bytes();
            let date = day.len() == 10
                && day.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() });
            if !date {
                return fail(at("approved"), "onay günü YYYY-AA-GG biçiminde olmalı");
            }
            if a.note.as_ref().is_some_and(|n| n.chars().count() > 500) {
                return fail(at("approved"), "onay notu en çok 500 karakter olur");
            }
            let numbers = [a.cut, a.fill, a.factored_cut, a.factored_fill].into_iter().chain(a.strip).chain(a.bench);
            if a.rows.len() > MAX_VOLUME_ROWS || !numbers.chain(a.rows.iter().flatten().copied()).all(f64::is_finite) {
                return fail(at("approved"), &format!("onaylı kübajın sayıları sonlu, en çok {MAX_VOLUME_ROWS} satır olur"));
            }
        }
    }
    None
}

/// What is wrong with the corridors: each on an alignment of its own PIs, its profile on it, its schedule on it, its
/// regions' templates in the project, its numbers finite and in range. Its regions' geometry is judged where it is
/// built (docs/adr/0905 §1).
fn corridors_problem(road: &RoadSettings) -> Option<(String, String)> {
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
        }
    }
    None
}

/// What is wrong with the profiles, by the field's path and in words; each lies on an alignment of its own PIs. Their
/// geometry is judged where it is built.
fn profiles_problem(list: &[ProfileDef], alignments: &[AlignmentDef]) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    if list.len() > MAX_PROFILES {
        return fail("profiles".into(), &format!("en çok {MAX_PROFILES} boykesit olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, p) in list.iter().enumerate() {
        let at = |field: &str| format!("profiles[{i}].{field}");
        if !word_ok(&p.id) {
            return fail(at("id"), "boykesitin kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(p.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir boykesit var");
        }
        let name = p.name.trim();
        if name.is_empty() || name != p.name || p.name.chars().count() > 120 {
            return fail(at("name"), "boykesitin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir boykesit var");
        }
        match alignments.iter().find(|a| a.id == p.alignment) {
            None => return fail(at("alignment"), "boykesitin aksı projede yok"),
            Some(a) if a.offset_of.is_some() => {
                return fail(at("alignment"), "boykesit yalnız PI'larıyla kurulan aksa bağlanır");
            }
            _ => {}
        }
        if p.surface.as_deref().is_some_and(|s| !word_ok(s)) {
            return fail(at("surface"), "yüzey bir yüzeyin kimliği olmalı");
        }
        if p.pvis.len() < 2 || p.pvis.len() > MAX_PROFILE_PVIS {
            return fail(at("pvis"), &format!("boykesitin 2 ile {MAX_PROFILE_PVIS} arası PVI'si olur"));
        }
        let mut pvis = std::collections::BTreeSet::new();
        for (j, v) in p.pvis.iter().enumerate() {
            let vat = |field: &str| format!("profiles[{i}].pvis[{j}].{field}");
            if !word_ok(&v.id) || !pvis.insert(v.id.as_str()) {
                return fail(vat("id"), "PVI'nin kimliği boykesitte tek ve boşluksuz olmalı");
            }
            if ![v.station, v.elevation, v.length, v.length_in, v.length_out, v.radius].iter().all(|x| x.is_finite()) {
                return fail(vat("station"), "sayılar sonlu olmalı");
            }
            if v.length < 0.0 || v.length_in < 0.0 || v.length_out < 0.0 || v.radius < 0.0 {
                return fail(vat("length"), "kurp boyu ve yarıçap eksi olamaz");
            }
            let asymmetric = v.length_in > 0.0 || v.length_out > 0.0;
            let kinds = [v.length > 0.0, asymmetric, v.radius > 0.0].iter().filter(|k| **k).count();
            if kinds > 1 {
                return fail(vat("length"), "bir PVI'de tek kurp olur: length, lengthIn ile lengthOut ya da radius");
            }
            if asymmetric && (v.length_in <= 0.0 || v.length_out <= 0.0) {
                return fail(vat("lengthIn"), "asimetrik kurbun iki kolu birlikte verilir");
            }
        }
    }
    None
}

/// What is wrong with the alignments, by the field's path and in words; their geometry is judged where it is built.
fn alignments_problem(list: &[AlignmentDef]) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    if list.len() > MAX_ALIGNMENTS {
        return fail("alignments".into(), &format!("en çok {MAX_ALIGNMENTS} aks olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, a) in list.iter().enumerate() {
        let at = |field: &str| format!("alignments[{i}].{field}");
        if !word_ok(&a.id) {
            return fail(at("id"), "aksın kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(a.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir aks var");
        }
        let name = a.name.trim();
        if name.is_empty() || name != a.name || a.name.chars().count() > 120 {
            return fail(at("name"), "aksın adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir aks var");
        }
        match (&a.offset_of, a.pis.is_empty()) {
            (Some(o), true) => {
                if a.stationing.is_some() {
                    return fail(at("stationing"), "paralel aksın km'si kaynağınındır");
                }
                let source = list.iter().find(|b| b.id == o.alignment);
                match source {
                    None => return fail(at("offsetOf.alignment"), "kaynak aks projede yok"),
                    Some(b) if b.offset_of.is_some() || b.id == a.id => {
                        return fail(at("offsetOf.alignment"), "paralel aks yalnız PI'larıyla kurulan aksa bağlanır");
                    }
                    _ => {}
                }
                if !(o.offset.is_finite() && o.offset != 0.0 && o.offset.abs() <= MAX_ALIGNMENT_OFFSET) {
                    return fail(
                        at("offsetOf.offset"),
                        &format!("öteleme 0 olmayan, en çok {MAX_ALIGNMENT_OFFSET} m bir sayı olmalı"),
                    );
                }
            }
            (None, false) => {
                if a.pis.len() < 2 || a.pis.len() > MAX_ALIGNMENT_PIS {
                    return fail(at("pis"), &format!("aksın 2 ile {MAX_ALIGNMENT_PIS} arası noktası olur"));
                }
                let mut pis = std::collections::BTreeSet::new();
                for (j, p) in a.pis.iter().enumerate() {
                    let pat = |field: &str| format!("alignments[{i}].pis[{j}].{field}");
                    if !word_ok(&p.id) || !pis.insert(p.id.as_str()) {
                        return fail(pat("id"), "noktanın kimliği aksta tek ve boşluksuz olmalı");
                    }
                    if ![p.x, p.y, p.radius, p.spiral_in, p.spiral_out].iter().all(|v| v.is_finite()) {
                        return fail(pat("x"), "sayılar sonlu olmalı");
                    }
                    if p.radius < 0.0 || p.spiral_in < 0.0 || p.spiral_out < 0.0 {
                        return fail(pat("radius"), "yarıçap ve klotoid boyları eksi olamaz");
                    }
                }
            }
            _ => return fail(at("pis"), "aks ya PI'larıyla ya bir aksa paralel kurulur, ikisi birden değil"),
        }
        if let Some(st) = &a.stationing {
            if !st.start.is_finite() {
                return fail(at("stationing.start"), "başlangıç km'si sonlu olmalı");
            }
            if st.equations.len() > MAX_STATION_EQUATIONS {
                return fail(at("stationing.equations"), &format!("en çok {MAX_STATION_EQUATIONS} km eşitliği olur"));
            }
            if st.equations.iter().any(|q| !(q.back.is_finite() && q.ahead.is_finite())) {
                return fail(at("stationing.equations"), "km eşitliğinin değerleri sonlu olmalı");
            }
        }
    }
    None
}

fn word_ok(s: &str) -> bool {
    !s.is_empty() && s.chars().count() <= 64 && !s.chars().any(char::is_whitespace)
}

fn list_problem(list: &[String]) -> Option<String> {
    if list.len() > MAX_SURFACE_SOURCES {
        return Some(format!("en çok {MAX_SURFACE_SOURCES} kaynak olur"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for item in list {
        if !word_ok(item) {
            return Some("kaynak boş olmayan, boşluksuz bir kimlik olmalı".into());
        }
        if !seen.insert(item.as_str()) {
            return Some(format!("{item} iki kez yazılmış"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surface() -> TerrainSurfaceDef {
        TerrainSurfaceDef {
            id: "t-zemin".into(),
            name: "Mevcut zemin".into(),
            points: SurfaceSources {
                layers: vec!["L5".into()],
                uids: vec![],
            },
            breaklines: SurfaceSources::default(),
            boundary: Some("u-sinir".into()),
            voids: vec![],
            tolerance: None,
            max_edge: Some(50.0),
            built: None,
        }
    }

    #[test]
    fn a_surface_is_judged_by_its_fields() {
        let ok = RoadSettings {
            surfaces: vec![surface()],
            ..RoadSettings::default()
        };
        assert_eq!(road_problem(&ok), None);
        let json = serde_json::to_value(&ok).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"surfaces": [{"id": "t-zemin", "name": "Mevcut zemin", "points": {"layers": ["L5"]},
                "boundary": "u-sinir", "maxEdge": 50.0}]})
        );
        let bad = |f: fn(&mut TerrainSurfaceDef), path: &str| {
            let mut s = surface();
            f(&mut s);
            let got = road_problem(&RoadSettings { surfaces: vec![s], ..RoadSettings::default() }).map(|p| p.0);
            assert_eq!(got.as_deref(), Some(path));
        };
        bad(|s| s.id = String::new(), "surfaces[0].id");
        bad(|s| s.name = " a".into(), "surfaces[0].name");
        bad(|s| s.points = SurfaceSources::default(), "surfaces[0].points");
        bad(|s| s.points.uids = vec!["a".into(), "a".into()], "surfaces[0].points");
        bad(|s| s.voids = vec!["u-sinir".into()], "surfaces[0].voids");
        bad(|s| s.tolerance = Some(0.0), "surfaces[0].tolerance");
        bad(|s| s.max_edge = Some(f64::NAN), "surfaces[0].maxEdge");
        bad(|s| s.built = Some("AB".into()), "surfaces[0].built");
        let two = RoadSettings {
            surfaces: vec![surface(), TerrainSurfaceDef { id: "t-2".into(), name: "mevcut ZEMIN".into(), ..surface() }],
            ..RoadSettings::default()
        };
        assert_eq!(road_problem(&two).map(|p| p.0).as_deref(), Some("surfaces[1].name"));
    }

    #[test]
    fn a_profile_lies_on_an_alignment_of_its_own_pis() {
        let pvi = |id: &str, station: f64| ProfilePvi {
            id: id.into(),
            station,
            elevation: 100.0,
            length: 0.0,
            length_in: 0.0,
            length_out: 0.0,
            radius: 0.0,
        };
        let line = AlignmentDef {
            id: "g-1".into(),
            name: "Ana aks".into(),
            revision: 1,
            pis: vec![
                AlignmentPi { id: "a".into(), x: 0.0, y: 0.0, radius: 0.0, spiral_in: 0.0, spiral_out: 0.0 },
                AlignmentPi { id: "b".into(), x: 100.0, y: 0.0, radius: 0.0, spiral_in: 0.0, spiral_out: 0.0 },
            ],
            offset_of: None,
            stationing: None,
        };
        let side = AlignmentDef {
            id: "g-2".into(),
            name: "Kenar".into(),
            pis: vec![],
            offset_of: Some(AlignmentOffset { alignment: "g-1".into(), offset: 3.0 }),
            ..line.clone()
        };
        let profile = ProfileDef {
            id: "b-1".into(),
            name: "Proje kotu".into(),
            alignment: "g-1".into(),
            alignment_revision: 1,
            revision: 1,
            pvis: vec![pvi("a", 0.0), pvi("b", 100.0)],
            surface: None,
        };
        let judge = |f: fn(&mut ProfileDef)| {
            let mut p = profile.clone();
            f(&mut p);
            road_problem(&RoadSettings {
                alignments: vec![line.clone(), side.clone()],
                profiles: vec![p],
                ..RoadSettings::default()
            })
            .map(|p| p.0)
        };
        assert_eq!(judge(|_| {}), None);
        assert_eq!(judge(|p| p.alignment = "g-2".into()).as_deref(), Some("profiles[0].alignment"));
        assert_eq!(judge(|p| p.alignment = "yok".into()).as_deref(), Some("profiles[0].alignment"));
        assert_eq!(judge(|p| p.pvis.truncate(1)).as_deref(), Some("profiles[0].pvis"));
        assert_eq!(judge(|p| p.pvis[1].radius = -1.0).as_deref(), Some("profiles[0].pvis[1].length"));
        assert_eq!(judge(|p| p.pvis[1].length_in = 5.0).as_deref(), Some("profiles[0].pvis[1].lengthIn"));
        assert_eq!(
            judge(|p| {
                p.pvis[1].length = 5.0;
                p.pvis[1].radius = 5.0;
            })
            .as_deref(),
            Some("profiles[0].pvis[1].length")
        );
    }

}
