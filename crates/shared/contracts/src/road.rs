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
/// At most so many ended approvals in a volume's history (docs/adr/0950).
pub const MAX_VOLUME_HISTORY: usize = 200;

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
    /// The guardrails (docs/adr/0909 §1), each along a corridor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<GuardrailDef>>", optional))]
    pub guardrails: Vec<GuardrailDef>,
    /// The culverts (docs/adr/0909 §2), each across a corridor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CulvertDef>>", optional))]
    pub culverts: Vec<CulvertDef>,
    /// The retaining walls (docs/adr/0909 §3), each along a corridor's side.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<RetainingWallDef>>", optional))]
    pub walls: Vec<RetainingWallDef>,
    /// The graded pads (docs/adr/0927), each on a terrain surface.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<PlatformDef>>", optional))]
    pub platforms: Vec<PlatformDef>,
    /// The junctions (docs/adr/0910 §2), each of two alignments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<JunctionDef>>", optional))]
    pub junctions: Vec<JunctionDef>,
    /// The pay item book (docs/adr/0909 §4): the agency's items and the rules that give them KentOS's quantities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pay: Option<PayBookDef>,
    /// What the road commands drew (docs/adr/0911 §1): each drawing's command, input, sources and objects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<RoadDrawingDef>>", optional))]
    pub drawings: Vec<RoadDrawingDef>,
    /// A write that leaves drawings stale draws them again in its step (docs/adr/0911 §3); off unless chosen.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub auto_rebuild: bool,
}

/// The road commands that draw (docs/adr/0911 §1): what a drawing's `command` is.
pub const ROAD_DRAWING_COMMANDS: [&str; 10] = [
    "road.alignments.label",
    "road.profiles.draw",
    "road.sections.draw",
    "road.corridors.draw",
    "road.guardrails.draw",
    "road.culverts.draw",
    "road.walls.draw",
    "road.junctions.draw",
    "road.platforms.draw",
    "road.terrains.contours",
];
/// The most drawings a project keeps, sources a drawing names and objects it holds.
pub const MAX_ROAD_DRAWINGS: usize = 1000;
pub const MAX_DRAWING_SOURCES: usize = 256;
pub const MAX_DRAWING_OBJECTS: usize = 1_000_000;

/// What kind of road source a drawing depends on (docs/adr/0911 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RoadSourceKind {
    Surface,
    Alignment,
    Profile,
    Template,
    Sections,
    Superelevation,
    Corridor,
    Guardrail,
    Culvert,
    Wall,
    Junction,
    Platform,
}

/// A source a drawing was made from: its kind, id and the stamp of what it was (docs/adr/0911 §1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadDrawingSource {
    pub kind: RoadSourceKind,
    pub id: String,
    /// 16 lower-case hex digits: its definition's canonical JSON stamped (a surface's with its sources' digest).
    pub digest: String,
}

/// A drawing a road command made (docs/adr/0911 §1): its id, the command and its input as given, the sources it was
/// made from (its main source first), the objects it wrote in order and the stamp of those objects as written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadDrawingDef {
    pub id: String,
    pub command: String,
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    #[cfg_attr(feature = "schema", schemars(with = "std::collections::BTreeMap<String, serde_json::Value>"))]
    pub input: std::collections::BTreeMap<String, serde_json::Value>,
    pub sources: Vec<RoadDrawingSource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    /// 16 lower-case hex digits: the objects' JSON in order, stamped.
    pub digest: String,
}

impl RoadSettings {
    /// Whether a grade (rise over run) is held: a template's cross slopes, a superelevation schedule, a culvert's fall,
    /// a pad's grade or a volume's benched ground slope (docs/adr/0977; KCAD schema 191 holds them as grades).
    pub fn has_grades(&self) -> bool {
        !(self.templates.is_empty() && self.superelevations.is_empty() && self.culverts.is_empty() && self.platforms.is_empty())
            || self.volumes.iter().any(|v| v.bench.is_some())
    }

    /// A file written before schema 191 held these in percent (docs/adr/0977): each is divided by 100 as it is read,
    /// so −2.5 becomes −0.025. Nothing else is touched (a side slope is horizontal per vertical, a share is a share).
    pub fn grades_from_percent(&mut self) {
        let p = |v: &mut f64| *v /= 100.0;
        for t in &mut self.templates {
            for side in std::iter::once(&mut t.left).chain(t.right.as_mut()) {
                side.components.iter_mut().for_each(|c| p(&mut c.slope));
                for b in side.bench.iter_mut().chain(side.fill_bench.iter_mut()) {
                    p(&mut b.fall);
                }
            }
        }
        for d in &mut self.superelevations {
            p(&mut d.normal_left);
            p(&mut d.normal_right);
            match &mut d.method {
                SuperelevationMethod::Curves { curves } => curves.iter_mut().for_each(|c| p(&mut c.rate)),
                SuperelevationMethod::Stations { rows } => rows.iter_mut().for_each(|r| {
                    p(&mut r.left);
                    p(&mut r.right);
                }),
            }
        }
        self.culverts.iter_mut().for_each(|c| p(&mut c.slope));
        self.platforms.iter_mut().for_each(|x| p(&mut x.grade));
        for v in &mut self.volumes {
            if let Some(b) = &mut v.bench {
                p(&mut b.slope);
            }
        }
    }

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
            && self.guardrails.is_empty()
            && self.culverts.is_empty()
            && self.walls.is_empty()
            && self.platforms.is_empty()
            && self.pay.is_none()
            && self.junctions.is_empty()
            && self.drawings.is_empty()
            && !self.auto_rebuild
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
    /// The ground points taken from point clouds into its point layers (docs/adr/0928 §2), each layer's record.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SurfaceCloudDef>>", optional))]
    pub clouds: Vec<SurfaceCloudDef>,
    /// The point records left out of it (docs/adr/0916 §6; a repair: a spike, a shot twice): each place, exactly as the
    /// record's; the drawing's points stay as they are.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SurfaceExclusion>>", optional))]
    pub excluded: Vec<SurfaceExclusion>,
}

/// A point record left out of a surface: its place (docs/adr/0916 §6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurfaceExclusion {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
}

/// The most point records one surface leaves out.
pub const MOST_SURFACE_EXCLUSIONS: usize = 100_000;

/// Which of a pulse's returns a cloud's points are taken from (docs/adr/0928): all, the first, the last (nearest the
/// ground), or a pulse's only return.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CloudReturns {
    #[default]
    All,
    First,
    Last,
    Single,
}

/// What a surface's points taken from point clouds are (docs/adr/0928 §2): the layer they were written to, the clouds,
/// classes, returns and area they were chosen by, the thinning, and what it cost: the ground points read and kept,
/// their coverage (0–1) and density (per m²) of the area, and the vertical error of the points left out against the
/// surface made then (absent when none was measured), and how many were outside it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurfaceCloudDef {
    pub layer: String,
    pub clouds: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub classes: Vec<u8>,
    #[serde(default)]
    pub returns: CloudReturns,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub area: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cell: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub z_tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub near: Option<f64>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub read: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub kept: u64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub coverage: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub density: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub error_max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub error_rms: Option<f64>,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    #[cfg_attr(feature = "ts", ts(type = "number", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outside: u64,
}

fn is_zero_u64(v: &u64) -> bool {
    *v == 0
}

/// The most cloud records a surface keeps, and the most clouds and classes one names.
pub const MAX_SURFACE_CLOUDS: usize = 64;

/// What is wrong with a surface's cloud records (docs/adr/0928 §2): a record names its layer once and at least one
/// cloud and class; the thinning's values are positive; the counts agree and the shares are shares.
fn clouds_problem(list: &[SurfaceCloudDef]) -> Option<String> {
    if list.len() > MAX_SURFACE_CLOUDS {
        return Some(format!("en çok {MAX_SURFACE_CLOUDS} bulut kaydı olur"));
    }
    let mut layers = std::collections::BTreeSet::new();
    for (k, c) in list.iter().enumerate() {
        let at = |what: &str| format!("{}. bulut kaydının {what}", k + 1);
        if !word_ok(&c.layer) || !layers.insert(c.layer.as_str()) {
            return Some(at("katmanı bir katmanın kimliği olmalı ve bir kez geçmeli"));
        }
        if c.clouds.is_empty() || c.clouds.len() > MAX_SURFACE_CLOUDS || c.clouds.iter().any(|u| !word_ok(u)) {
            return Some(at("bulutları 1 ile 64 arası nesne kimliği olmalı"));
        }
        if c.classes.is_empty() || c.classes.len() > 256 {
            return Some(at("sınıfları en az bir sınıf olmalı"));
        }
        if c.area.as_deref().is_some_and(|a| !word_ok(a)) {
            return Some(at("alanı bir nesnenin kimliği olmalı"));
        }
        let positive = |v: f64| v.is_finite() && v > 0.0;
        if !positive(c.cell) || c.z_tolerance.is_some_and(|v| !positive(v)) || c.near.is_some_and(|v| !positive(v)) {
            return Some(at("hücresi, kot toleransı ve kırık hat erimi sıfırdan büyük olmalı"));
        }
        if c.kept > c.read
            || !(0.0..=1.0).contains(&c.coverage)
            || !(c.density.is_finite() && c.density >= 0.0)
            || [c.error_max, c.error_rms].iter().flatten().any(|v| !(v.is_finite() && *v >= 0.0))
        {
            return Some(at("sayıları tutarlı olmalı (kalan okunandan çok değil, kapsama 0–1)"));
        }
    }
    None
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
    /// The curve runs into the next PI's through an ove clothoid whose A is solved on the PIs (docs/adr/0979; KCAD
    /// schema 192): both curves turn the same way, of different radii, with no clothoid on the side they share.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub ove: bool,
    /// The design speed at this PI's curve (km/h), when it is not the alignment's (docs/adr/0985; KCAD schema 193):
    /// KGM's 2019 sample sheets write V=110 for one curve and V=50 for a junction's on the same road.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "km/h")))]
    pub speed: Option<f64>,
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
    /// The road's design speed (proje hızı, km/h; docs/adr/0985, KCAD schema 193): what its curves are designed for
    /// and the V of a curve's data on the plan (KGM 2019 şartname s. 5: “Yatay kurblar proje hızına göre
    /// seçilmelidir”; the 2024 sample sheets write `V = 80`). A PI's own `speed` stands for its curve. A parallel axis
    /// has its source's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "km/h")))]
    pub design_speed: Option<f64>,
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
    /// What the road crosses (docs/adr/0946; KCAD schema 172).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProfileCrossingDef>>", optional))]
    pub crossings: Vec<ProfileCrossingDef>,
}

/// Where a crossed structure is against the road (docs/adr/0946).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CrossingSide {
    /// Over the road (a bridge, an overpass, a line): its elevation is its underside, the road passes under it.
    Above,
    /// Under the road (a culvert, a pipe): its elevation is its top, the road passes over it.
    Below,
}

/// Something a road crosses (docs/adr/0946): its name, station on the profile's alignment, side, elevation (underside
/// or top) and the clearance under it or the cover over it (m). The values are the project's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileCrossingDef {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    pub side: CrossingSide,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub elevation: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub clearance: f64,
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
        if let Some(words) = clouds_problem(&s.clouds) {
            return fail(at("clouds"), &words);
        }
        if s.excluded.len() > MOST_SURFACE_EXCLUSIONS {
            return fail(at("excluded"), "bir yüzey en çok 100 000 noktayı dışarıda bırakır");
        }
        if s.excluded.iter().any(|e| !(e.x.is_finite() && e.y.is_finite())) {
            return fail(at("excluded"), "dışarıda bırakılan yer sonlu iki sayı olmalı");
        }
        let mut places: Vec<(u64, u64)> = s.excluded.iter().map(|e| ((e.x + 0.0).to_bits(), (e.y + 0.0).to_bits())).collect();
        places.sort_unstable();
        if places.windows(2).any(|w| w[0] == w[1]) {
            return fail(at("excluded"), "bir yer dışarıda bırakılanlarda iki kez");
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
        .or_else(|| guardrails_problem(road))
        .or_else(|| culverts_problem(road))
        .or_else(|| walls_problem(road))
        .or_else(|| platforms_problem(road))
        .or_else(|| pay_problem(road))
        .or_else(|| junctions_problem(road))
        .or_else(|| drawings_problem(road))
}

fn stamp_ok(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn drawings_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    if road.drawings.len() > MAX_ROAD_DRAWINGS {
        return fail("drawings".into(), &format!("en çok {MAX_ROAD_DRAWINGS} yol çizimi kaydedilir"));
    }
    let mut ids = std::collections::BTreeSet::new();
    for (i, d) in road.drawings.iter().enumerate() {
        let at = |field: &str| format!("drawings[{i}].{field}");
        if !word_ok(&d.id) || !ids.insert(d.id.as_str()) {
            return fail(at("id"), "çizimin kimliği boş olmayan, en çok 64 karakterlik ve tek bir sözcük olmalı");
        }
        if !ROAD_DRAWING_COMMANDS.contains(&d.command.as_str()) {
            return fail(at("command"), "çizimin komutu çizime yazan bir yol komutu olmalı");
        }
        if d.sources.is_empty() || d.sources.len() > MAX_DRAWING_SOURCES {
            return fail(at("sources"), &format!("çizimin 1 ile {MAX_DRAWING_SOURCES} arası kaynağı olur"));
        }
        let mut seen = std::collections::BTreeSet::new();
        for (k, s) in d.sources.iter().enumerate() {
            if !word_ok(&s.id) || !stamp_ok(&s.digest) || !seen.insert((s.kind, s.id.as_str())) {
                return fail(
                    format!("drawings[{i}].sources[{k}]"),
                    "kaynağın kimliği bir sözcük, özeti 16 onaltılık hane olur; aynı kaynak iki kez yazılmaz",
                );
            }
        }
        if d.uids.len() > MAX_DRAWING_OBJECTS || d.uids.iter().any(|u| !word_ok(u)) {
            return fail(at("uids"), &format!("çizimin en çok {MAX_DRAWING_OBJECTS} nesnesi olur, her biri bir kimlik"));
        }
        if !stamp_ok(&d.digest) {
            return fail(at("digest"), "nesnelerin özeti 16 onaltılık hane olur");
        }
    }
    None
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
fn pavement_problem(p: &PavementDef, at: &str) -> Option<(String, String)> {
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
/// The most guardrails a project holds, and rows a guardrail's rule holds.
pub const MAX_GUARDRAILS: usize = 256;
pub const MAX_GUARDRAIL_WARRANTS: usize = 32;

/// A row of a guardrail rule (docs/adr/0909 §1): a barrier where the fill is at least `height` high and, with `slope`,
/// steeper than 1:`slope` (its run per rise at most that).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailWarrantDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub slope: Option<f64>,
}

/// A guardrail rule (docs/adr/0909 §1): the standard's table it comes from (`source`, required: KentOS has no number of
/// its own), its rows, the lengths added before (the approach end) and after a warranted stretch, the least gap left
/// between two runs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailRuleDef {
    pub source: String,
    pub warrants: Vec<GuardrailWarrantDef>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub approach: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub leaving: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub gap: f64,
}

/// A guardrail (docs/adr/0909 §1): its corridor and the corridor's revision it was made from, its rule, the step its
/// crosses are read at, and how far outward from the hinge the rail stands.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailDef {
    pub id: String,
    pub name: String,
    pub corridor: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub corridor_revision: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub rule: GuardrailRuleDef,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: f64,
}

/// The most culverts a project holds, and barrels a culvert carries.
pub const MAX_CULVERTS: usize = 1000;
pub const MAX_CULVERT_BARRELS: u32 = 20;

/// A culvert's barrel cross (docs/adr/0909 §2): pipes side by side (`spacing` their clear gap) or a box of cells.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CulvertShapeDef {
    Pipe {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        diameter: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        thickness: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        count: u32,
        #[serde(default, skip_serializing_if = "is_zero_f64")]
        #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        spacing: f64,
    },
    Box {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        width: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        height: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        thickness: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        slab: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        cells: u32,
    },
}

/// The side a culvert's water runs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CulvertFlow {
    Right,
    Left,
}

/// A culvert across a corridor (docs/adr/0909 §2): its corridor and that corridor's revision it was made from, its
/// station and skew (degrees from the cross, positive ahead on the right), its barrel, its flow line under the axis and
/// its fall (a grade, 0.02 is 2 %; docs/adr/0977) toward `flow`, the length added beyond each toe, the least cover asked, its material and Manning
/// roughness (typed, kept).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CulvertDef {
    pub id: String,
    pub name: String,
    pub corridor: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub corridor_revision: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub skew: f64,
    pub shape: CulvertShapeDef,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub invert: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub slope: f64,
    pub flow: CulvertFlow,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub extend: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cover: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub roughness: Option<f64>,
}

fn culverts_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.culverts;
    if list.len() > MAX_CULVERTS {
        return fail("culverts".into(), &format!("en çok {MAX_CULVERTS} menfez olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    let size = |v: f64| v.is_finite() && v > 0.0 && v <= 100.0;
    for (i, c) in list.iter().enumerate() {
        let at = |field: &str| format!("culverts[{i}].{field}");
        if !word_ok(&c.id) {
            return fail(at("id"), "menfezin kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(c.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir menfez var");
        }
        let name = c.name.trim();
        if name.is_empty() || name != c.name || c.name.chars().count() > 120 {
            return fail(at("name"), "menfezin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir menfez var");
        }
        if road.corridors.iter().all(|x| x.id != c.corridor) {
            return fail(at("corridor"), "menfezin koridoru projede yok");
        }
        if !c.station.is_finite() {
            return fail(at("station"), "menfezin km'si sonlu olmalı");
        }
        if !(c.skew.is_finite() && c.skew.abs() < 90.0) {
            return fail(at("skew"), "menfezin açısı -90° ile 90° arası olur (sınırlar dışında)");
        }
        match c.shape {
            CulvertShapeDef::Pipe { diameter, thickness, count, spacing } => {
                if !size(diameter) || !size(thickness) {
                    return fail(at("shape"), "borunun çapı ve et kalınlığı 0'dan büyük, en çok 100 m olur");
                }
                if !(1..=MAX_CULVERT_BARRELS).contains(&count) {
                    return fail(at("shape.count"), &format!("boru sayısı 1 ile {MAX_CULVERT_BARRELS} arası olur"));
                }
                if !(spacing.is_finite() && (0.0..=100.0).contains(&spacing)) {
                    return fail(at("shape.spacing"), "borular arası boşluk 0 ile 100 m arası olur");
                }
            }
            CulvertShapeDef::Box { width, height, thickness, slab, cells } => {
                if !size(width) || !size(height) || !size(thickness) || !size(slab) {
                    return fail(at("shape"), "kutunun genişliği, yüksekliği, duvar ve döşeme kalınlığı 0'dan büyük, en çok 100 m olur");
                }
                if !(1..=MAX_CULVERT_BARRELS).contains(&cells) {
                    return fail(at("shape.cells"), &format!("göz sayısı 1 ile {MAX_CULVERT_BARRELS} arası olur"));
                }
            }
        }
        if !(c.invert.is_finite() && c.invert.abs() <= 10_000.0) {
            return fail(at("invert"), "akış kotu sonlu, en çok 10 000 m olur");
        }
        if !(c.slope.is_finite() && (0.0..=1.0).contains(&c.slope)) {
            return fail(at("slope"), "akış eğimi oranla 0 ile 1 (%0–100) arası olur");
        }
        if !(c.extend.is_finite() && (0.0..=100.0).contains(&c.extend)) {
            return fail(at("extend"), "topuğun ötesine boy 0 ile 100 m arası olur");
        }
        if c.cover.is_some_and(|v| !(v.is_finite() && (0.0..=100.0).contains(&v))) {
            return fail(at("cover"), "en az örtü 0 ile 100 m arası olur");
        }
        if c.material.as_ref().is_some_and(|m| m.trim().is_empty() || m.trim() != m || m.chars().count() > 120) {
            return fail(at("material"), "malzeme boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if c.roughness.is_some_and(|n| !(n.is_finite() && n > 0.0 && n <= 1.0)) {
            return fail(at("roughness"), "Manning pürüzlülüğü 0'dan büyük, en çok 1 olur");
        }
    }
    None
}

/// The most retaining walls a project holds, and stations a wall is read at.
pub const MAX_RETAINING_WALLS: usize = 1000;
pub const MAX_RETAINING_WALL_STATIONS: usize = 5000;

/// A corridor's side a wall stands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WallSide {
    Left,
    Right,
}

/// A retaining wall's cross (docs/adr/0909 §3): the stem's top thickness and its batter (m/m), the footing's width,
/// thickness and toe, the ground over the footing and what stands above the higher of the road's edge and the ground.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallShapeDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub top: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub batter: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub footing: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub base: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub toe: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cover: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub above: f64,
}

/// A retaining wall along a corridor's side (docs/adr/0909 §3): its corridor and that corridor's revision it was made
/// from, its side and stations, the step it is read at, its cross, the highest stem asked (a warning, not a refusal) and
/// its material (typed, kept).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RetainingWallDef {
    pub id: String,
    pub name: String,
    pub corridor: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub corridor_revision: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub side: WallSide,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: f64,
    pub shape: WallShapeDef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub highest: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
}

fn walls_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.walls;
    if list.len() > MAX_RETAINING_WALLS {
        return fail("walls".into(), &format!("en çok {MAX_RETAINING_WALLS} istinat duvarı olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    let size = |v: f64| v.is_finite() && v > 0.0 && v <= 100.0;
    let room = |v: f64| v.is_finite() && (0.0..=100.0).contains(&v);
    for (i, w) in list.iter().enumerate() {
        let at = |field: &str| format!("walls[{i}].{field}");
        if !word_ok(&w.id) {
            return fail(at("id"), "duvarın kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(w.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir istinat duvarı var");
        }
        let name = w.name.trim();
        if name.is_empty() || name != w.name || w.name.chars().count() > 120 {
            return fail(at("name"), "duvarın adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir istinat duvarı var");
        }
        if road.corridors.iter().all(|x| x.id != w.corridor) {
            return fail(at("corridor"), "duvarın koridoru projede yok");
        }
        if !(w.from.is_finite() && w.to.is_finite() && w.from < w.to) {
            return fail(at("to"), "duvarın bitiş km'si başlangıcından büyük olmalı");
        }
        if !(w.step.is_finite() && w.step > 0.0 && w.step <= 1000.0) {
            return fail(at("step"), "duvarın okunduğu aralık 0'dan büyük, en çok 1000 m olur");
        }
        if (w.to - w.from) / w.step > MAX_RETAINING_WALL_STATIONS as f64 {
            return fail(at("step"), &format!("duvar en çok {MAX_RETAINING_WALL_STATIONS} km'de okunur; aralığı büyütün"));
        }
        let s = &w.shape;
        if !size(s.top) || !size(s.footing) || !size(s.base) {
            return fail(at("shape"), "gövdenin üst kalınlığı, temelin genişliği ve kalınlığı 0'dan büyük, en çok 100 m olur");
        }
        if !(s.batter.is_finite() && (0.0..=10.0).contains(&s.batter)) {
            return fail(at("shape.batter"), "gövdenin eğimi 0 ile 10 arası olur (metrede yatay metre)");
        }
        if !room(s.toe) || !room(s.cover) || !room(s.above) {
            return fail(at("shape"), "temelin önü, örtü ve korkuluk payı 0 ile 100 m arası olur");
        }
        if s.toe + s.top > s.footing + 1e-9 {
            return fail(at("shape.toe"), "temelin önü ve gövdenin üst kalınlığı temelin genişliğine sığmalı");
        }
        if w.highest.is_some_and(|v| !size(v)) {
            return fail(at("highest"), "en yüksek gövde 0'dan büyük, en çok 100 m olur");
        }
        if w.material.as_ref().is_some_and(|m| m.trim().is_empty() || m.trim() != m || m.chars().count() > 120) {
            return fail(at("material"), "malzeme boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
    }
    None
}

/// The most graded pads a project holds, and corners a pad's boundary has.
pub const MAX_PLATFORMS: usize = 1000;
pub const MAX_PLATFORM_CORNERS: usize = 1000;

/// A graded pad (docs/adr/0927): its boundary's corners, the plane through `base` rising `grade` (0.02 is 2 %; docs/adr/0977) towards
/// `direction` (degrees counterclockwise from +x), its cut and fill slopes (horizontal per vertical), the spacing of
/// its slope lines along an edge, a corner's widest fan step (degrees), the longest slope line and the terrain it is
/// graded to. The slopes are the user's: no soil's default is guessed (ROAD-GRADE-02).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformDef {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub corners: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub base: [f64; 3],
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub grade: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub direction: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub fan: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub reach: f64,
    pub surface: String,
}

fn platforms_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.platforms;
    if list.len() > MAX_PLATFORMS {
        return fail("platforms".into(), &format!("en çok {MAX_PLATFORMS} platform olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, p) in list.iter().enumerate() {
        let at = |field: &str| format!("platforms[{i}].{field}");
        if !word_ok(&p.id) {
            return fail(at("id"), "platformun kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(p.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir platform var");
        }
        let name = p.name.trim();
        if name.is_empty() || name != p.name || p.name.chars().count() > 120 {
            return fail(at("name"), "platformun adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir platform var");
        }
        if p.corners.len() < 3 || p.corners.len() > MAX_PLATFORM_CORNERS {
            return fail(at("corners"), &format!("platformun sınırı 3 ile {MAX_PLATFORM_CORNERS} arası köşedir"));
        }
        if !p.corners.iter().flatten().chain(p.base.iter()).all(|v| v.is_finite()) {
            return fail(at("corners"), "köşeler ve taban sonlu sayılar olmalı");
        }
        if !(p.grade.is_finite() && p.grade.abs() <= 1.0) {
            return fail(at("grade"), "platformun eğimi oranla yazılır (0,02 %2'dir), en çok 1 (%100) olur");
        }
        if !(p.direction.is_finite() && (-360.0..=360.0).contains(&p.direction)) {
            return fail(at("direction"), "eğimin yönü −360 ile 360 derece arası olur");
        }
        for (field, v, top) in [("cut", p.cut, 100.0), ("fill", p.fill, 100.0), ("step", p.step, 1000.0), ("reach", p.reach, 10_000.0)] {
            if !(v.is_finite() && v > 0.0 && v <= top) {
                return fail(at(field), &format!("0'dan büyük, en çok {top} olur"));
            }
        }
        if !(p.fan.is_finite() && p.fan > 0.0 && p.fan <= 90.0) {
            return fail(at("fan"), "köşe yelpazesinin açısı 0'dan büyük, en çok 90 derece olur");
        }
        if road.surfaces.iter().all(|s| s.id != p.surface) {
            return fail(at("surface"), "platformun arazi yüzeyi projede yok");
        }
    }
    None
}

/// The most junctions a project holds.
pub const MAX_JUNCTIONS: usize = 1000;

/// How a junction's legs meet: at grade (joined, the curb returns built) or separated (one over the other, not joined).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum JunctionKind {
    AtGrade,
    Separated,
}

/// A junction's leg: its alignment and that alignment's revision it was made from, and its edges' offsets from the
/// alignment (left and right, metres, positive).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionLegDef {
    pub alignment: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub alignment_revision: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub right: f64,
}

/// A junction of two alignments (docs/adr/0910 §2): its legs, which of their crossings (`at`, 0 the first along the
/// first leg), how they meet, and at grade the curb returns' radius (`radius`, every quadrant) and, when given, each
/// quadrant's own (`radii`, four: first leg's right with the second's right, right with left, left with left, left with
/// right).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionDef {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub kind: JunctionKind,
    pub legs: Vec<JunctionLegDef>,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub at: u32,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radii: Option<Vec<f64>>,
}

fn junctions_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.junctions;
    if list.len() > MAX_JUNCTIONS {
        return fail("junctions".into(), &format!("en çok {MAX_JUNCTIONS} kavşak olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    let size = |v: f64| v.is_finite() && v > 0.0 && v <= 1000.0;
    for (i, j) in list.iter().enumerate() {
        let at = |field: &str| format!("junctions[{i}].{field}");
        if !word_ok(&j.id) {
            return fail(at("id"), "kavşağın kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(j.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir kavşak var");
        }
        let name = j.name.trim();
        if name.is_empty() || name != j.name || j.name.chars().count() > 120 {
            return fail(at("name"), "kavşağın adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir kavşak var");
        }
        if j.legs.len() != 2 {
            return fail(at("legs"), "kavşağın iki kolu olur");
        }
        if j.legs[0].alignment == j.legs[1].alignment {
            return fail(at("legs"), "kavşağın iki kolu ayrı akslarda olur");
        }
        for (k, leg) in j.legs.iter().enumerate() {
            match road.alignments.iter().find(|a| a.id == leg.alignment) {
                None => return fail(format!("junctions[{i}].legs[{k}].alignment"), "kolun aksı projede yok"),
                Some(a) if a.offset_of.is_some() => {
                    return fail(format!("junctions[{i}].legs[{k}].alignment"), "kolun aksı PI'larıyla kurulan bir aks olur");
                }
                _ => {}
            }
            if !(size(leg.left) && size(leg.right)) || leg.left > 100.0 || leg.right > 100.0 {
                return fail(format!("junctions[{i}].legs[{k}].left"), "kolun kenarları 0'dan büyük, en çok 100 m olur");
            }
        }
        if j.at > 64 {
            return fail(at("at"), "kesişimin sırası en çok 64 olur");
        }
        match j.kind {
            JunctionKind::AtGrade => {
                if !size(j.radius) {
                    return fail(at("radius"), "hemzemin kavşağın dönüş yarıçapı 0'dan büyük, en çok 1000 m olur");
                }
                if j.radii.as_ref().is_some_and(|r| r.len() != 4 || r.iter().any(|v| !size(*v))) {
                    return fail(at("radii"), "çeyreklerin yarıçapı dört olur, her biri 0'dan büyük, en çok 1000 m");
                }
            }
            JunctionKind::Separated => {
                if j.radius != 0.0 || j.radii.is_some() {
                    return fail(at("radius"), "katlı kavşakta dönüş yayı olmaz");
                }
            }
        }
    }
    None
}

/// The most pay items and rules a book holds.
pub const MAX_PAY_ITEMS: usize = 2000;
pub const MAX_PAY_RULES: usize = 2000;

/// A quantity KentOS reads from a road project's structures and earthwork (docs/adr/0909 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PayMeasure {
    /// A guardrail's runs, both sides (m).
    GuardrailLength,
    /// A culvert's barrel (m).
    CulvertLength,
    /// A culvert's pipes: count × barrel (m).
    CulvertPipes,
    /// A culvert's barrel concrete (m³).
    CulvertConcrete,
    /// A retaining wall's face in plan (m).
    WallLength,
    /// A retaining wall's stem face (m²).
    WallFace,
    /// A retaining wall's concrete (m³).
    WallConcrete,
    /// An earthwork volume's cut (m³).
    VolumeCut,
    /// An earthwork volume's fill (m³).
    VolumeFill,
    /// An earthwork volume's cut times its swell factors (m³).
    VolumeFactoredCut,
    /// An earthwork volume's fill times its shrink factors (m³).
    VolumeFactoredFill,
}

impl PayMeasure {
    /// Whether it reads a culvert or a wall (which have a material).
    pub fn has_material(self) -> bool {
        matches!(
            self,
            Self::CulvertLength | Self::CulvertPipes | Self::CulvertConcrete | Self::WallLength | Self::WallFace | Self::WallConcrete
        )
    }
}

/// An agency's pay item: its code (as the agency writes it), name, unit (as written) and optional unit price.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PayItemDef {
    pub code: String,
    pub name: String,
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money")))]
    pub price: Option<f64>,
}

/// A rule giving an item a quantity: KentOS's `measure` (of the culverts or walls of `material` only, when given)
/// times `factor`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PayRuleDef {
    pub measure: PayMeasure,
    pub item: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub factor: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
}

/// The project's pay item book (docs/adr/0909 §4): its source (the agency's list and year, required: KentOS gives no
/// item), its version, its currency, its items and its rules.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PayBookDef {
    pub source: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub currency: Option<String>,
    pub items: Vec<PayItemDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<PayRuleDef>>", optional))]
    pub rules: Vec<PayRuleDef>,
}

fn pay_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let Some(b) = &road.pay else { return None };
    let text = |t: &str, most: usize| !t.trim().is_empty() && t.trim() == t && t.chars().count() <= most;
    if !text(&b.source, 200) {
        return fail("pay.source".into(), "poz defterinin kaynağı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 200 karakter");
    }
    if !text(&b.version, 64) {
        return fail("pay.version".into(), "poz defterinin sürümü boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 64 karakter");
    }
    if b.currency.as_ref().is_some_and(|c| !text(c, 12)) {
        return fail("pay.currency".into(), "para birimi boş olmamalı, en çok 12 karakter");
    }
    if b.items.is_empty() || b.items.len() > MAX_PAY_ITEMS {
        return fail("pay.items".into(), &format!("poz defterinde 1 ile {MAX_PAY_ITEMS} arası poz olur"));
    }
    if b.rules.len() > MAX_PAY_RULES {
        return fail("pay.rules".into(), &format!("poz defterinde en çok {MAX_PAY_RULES} kural olur"));
    }
    let mut codes = std::collections::BTreeSet::new();
    for (i, it) in b.items.iter().enumerate() {
        let at = |field: &str| format!("pay.items[{i}].{field}");
        if !text(&it.code, 64) {
            return fail(at("code"), "pozun numarası boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 64 karakter");
        }
        if !codes.insert(it.code.as_str()) {
            return fail(at("code"), "bu numarada başka bir poz var");
        }
        if !text(&it.name, 300) {
            return fail(at("name"), "pozun adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 300 karakter");
        }
        if !text(&it.unit, 16) {
            return fail(at("unit"), "pozun birimi boş olmamalı, en çok 16 karakter");
        }
        if it.price.is_some_and(|p| !(0.0..=1e12).contains(&p)) {
            return fail(at("price"), "pozun birim fiyatı 0 ya da büyük, sonlu olmalı");
        }
    }
    for (i, r) in b.rules.iter().enumerate() {
        let at = |field: &str| format!("pay.rules[{i}].{field}");
        if !codes.contains(r.item.as_str()) {
            return fail(at("item"), "kuralın pozu defterde yok");
        }
        if !(r.factor.is_finite() && r.factor > 0.0 && r.factor <= 1e6) {
            return fail(at("factor"), "kuralın çarpanı 0'dan büyük, en çok 1 000 000 olur");
        }
        if let Some(m) = &r.material {
            if !r.measure.has_material() {
                return fail(at("material"), "malzeme yalnız menfez ve istinat duvarı ölçülerinde olur");
            }
            if !text(m, 120) {
                return fail(at("material"), "kuralın malzemesi boş olmamalı, en çok 120 karakter");
            }
        }
    }
    None
}

fn guardrails_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.guardrails;
    if list.len() > MAX_GUARDRAILS {
        return fail("guardrails".into(), &format!("en çok {MAX_GUARDRAILS} otokorkuluk olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, g) in list.iter().enumerate() {
        let at = |field: &str| format!("guardrails[{i}].{field}");
        if !word_ok(&g.id) {
            return fail(at("id"), "otokorkuluğun kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(g.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir otokorkuluk var");
        }
        let name = g.name.trim();
        if name.is_empty() || name != g.name || g.name.chars().count() > 120 {
            return fail(at("name"), "otokorkuluğun adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir otokorkuluk var");
        }
        if road.corridors.iter().all(|c| c.id != g.corridor) {
            return fail(at("corridor"), "otokorkuluğun koridoru projede yok");
        }
        let r = &g.rule;
        let source = r.source.trim();
        if source.is_empty() || source != r.source || r.source.chars().count() > 200 {
            return fail(at("rule.source"), "kuralın kaynağı boş olmamalı (standart ve tablosu), başında ve sonunda boşluk olmamalı, en çok 200 karakter");
        }
        if r.warrants.is_empty() || r.warrants.len() > MAX_GUARDRAIL_WARRANTS {
            return fail(at("rule.warrants"), &format!("kuralın 1 ile {MAX_GUARDRAIL_WARRANTS} arası satırı olur"));
        }
        for (j, w) in r.warrants.iter().enumerate() {
            if !(w.height.is_finite() && (0.0..=1000.0).contains(&w.height)) {
                return fail(at(&format!("rule.warrants[{j}].height")), "dolgu yüksekliği 0 ile 1000 m arası olur");
            }
            if w.slope.is_some_and(|s| !(s.is_finite() && s > 0.0 && s <= 1000.0)) {
                return fail(at(&format!("rule.warrants[{j}].slope")), "şev (bir düşeye yatay) 0'dan büyük, en çok 1000 olur");
            }
        }
        for (field, v) in [("approach", r.approach), ("leaving", r.leaving), ("gap", r.gap)] {
            if !(v.is_finite() && (0.0..=10_000.0).contains(&v)) {
                return fail(at(&format!("rule.{field}")), "uzunluk 0 ile 10 000 m arası olur");
            }
        }
        if !(g.step.is_finite() && g.step > 0.0 && g.step <= 1000.0) {
            return fail(at("step"), "okuma aralığı 0'dan büyük, en çok 1000 m olur");
        }
        if !(g.offset.is_finite() && g.offset.abs() <= 100.0) {
            return fail(at("offset"), "korkuluğun şev başından uzaklığı en çok 100 m olur");
        }
    }
    None
}
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
        if p.crossings.len() > MAX_PROFILE_PVIS {
            return fail(at("crossings"), &format!("en çok {MAX_PROFILE_PVIS} kesişme olur"));
        }
        for (j, c) in p.crossings.iter().enumerate() {
            let cat = format!("profiles[{i}].crossings[{j}]");
            let nm = c.name.trim();
            if nm.is_empty() || nm != c.name || c.name.chars().count() > 120 {
                return fail(cat, "kesişmenin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
            }
            if !(c.station.is_finite() && c.elevation.is_finite()) {
                return fail(cat, "kesişmenin km'si ve kotu sonlu olmalı");
            }
            if !(c.clearance.is_finite() && (0.0..=100.0).contains(&c.clearance)) {
                return fail(cat, "açıklık ya da örtü 0 ile 100 m arası olur");
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
                if a.design_speed.is_some() {
                    return fail(at("designSpeed"), "paralel aksın proje hızı kaynağınındır");
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
                    if p.speed.is_some_and(|v| !speed_ok(v)) {
                        return fail(pat("speed"), &format!("proje hızı 0'dan büyük, en çok {MAX_DESIGN_SPEED} km/sa olmalı"));
                    }
                }
            }
            _ => return fail(at("pis"), "aks ya PI'larıyla ya bir aksa paralel kurulur, ikisi birden değil"),
        }
        if a.design_speed.is_some_and(|v| !speed_ok(v)) {
            return fail(at("designSpeed"), &format!("proje hızı 0'dan büyük, en çok {MAX_DESIGN_SPEED} km/sa olmalı"));
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

/// The highest design speed a road takes (km/h, docs/adr/0985): a bound of sense, not a standard's. Design speed
/// tables end at 130 km/h (AASHTO Green Book, RAA); Türkiye's highest legal limit is 140 km/h on toll motorways.
pub const MAX_DESIGN_SPEED: f64 = 200.0;

fn speed_ok(v: f64) -> bool {
    v.is_finite() && v > 0.0 && v <= MAX_DESIGN_SPEED
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
            clouds: Vec::new(),
            excluded: Vec::new(),
        }
    }

    #[test]
    fn a_drawing_record_is_judged() {
        let drawing = |id: &str| RoadDrawingDef {
            id: id.into(),
            command: "road.corridors.draw".into(),
            input: std::collections::BTreeMap::new(),
            sources: vec![RoadDrawingSource { kind: RoadSourceKind::Corridor, id: "c-1".into(), digest: "0123456789abcdef".into() }],
            uids: vec!["u-1".into()],
            digest: "fedcba9876543210".into(),
        };
        let of = |drawings: Vec<RoadDrawingDef>| road_problem(&RoadSettings { drawings, ..RoadSettings::default() }).map(|p| p.0);
        assert_eq!(of(vec![drawing("cz-1")]), None);
        assert_eq!(of(vec![drawing("cz-1"), drawing("cz-1")]).as_deref(), Some("drawings[1].id"));
        let mut d = drawing("cz-1");
        d.command = "road.routes.draw".into();
        assert_eq!(of(vec![d]).as_deref(), Some("drawings[0].command"));
        let mut d = drawing("cz-1");
        d.sources[0].digest = "0123456789ABCDEF".into();
        assert_eq!(of(vec![d]).as_deref(), Some("drawings[0].sources[0]"));
        let mut d = drawing("cz-1");
        d.sources.push(d.sources[0].clone());
        assert_eq!(of(vec![d]).as_deref(), Some("drawings[0].sources[1]"));
        let mut d = drawing("cz-1");
        d.digest = "x".into();
        assert_eq!(of(vec![d]).as_deref(), Some("drawings[0].digest"));
        // The most a project keeps: the 1001st is refused (what road_drawings_full answers).
        let many: Vec<RoadDrawingDef> = (0..=MAX_ROAD_DRAWINGS).map(|k| drawing(&format!("cz-{k}"))).collect();
        assert_eq!(of(many).as_deref(), Some("drawings"));
        assert!(!RoadSettings { auto_rebuild: true, ..RoadSettings::default() }.is_empty());
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
                AlignmentPi { id: "a".into(), x: 0.0, y: 0.0, radius: 0.0, spiral_in: 0.0, spiral_out: 0.0, ove: false, speed: None },
                AlignmentPi { id: "b".into(), x: 100.0, y: 0.0, radius: 0.0, spiral_in: 0.0, spiral_out: 0.0, ove: false, speed: None },
            ],
            offset_of: None,
            stationing: None,
            design_speed: None,
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
            crossings: Vec::new(),
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
