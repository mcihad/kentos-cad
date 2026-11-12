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

mod alignment;
mod corridor;
mod profile;
mod structures;
mod terrain;

pub use alignment::*;
pub use corridor::*;
pub use profile::*;
pub use structures::*;
pub use terrain::*;

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
    /// Drawing again, by hand from the project tree or by itself, draws each drawing whole (docs/adr/4740); unless
    /// chosen, only what changed is written (docs/adr/4320). Since `.kcad` schema 238.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub rebuild_whole: bool,
}

/// The road commands that draw (docs/adr/0911 §1): what a drawing's `command` is.
pub const ROAD_DRAWING_COMMANDS: [&str; 14] = [
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
    // A zero line between two surfaces (docs/adr/1620), kept since `.kcad` schema 198 (docs/adr/1720).
    "road.terrains.zeroline",
    // Two surfaces' cut and fill zones (docs/adr/1840), kept since `.kcad` schema 205.
    "road.terrains.cutfill",
    // A utility network's pipe run profile (docs/adr/1860), kept with its network since `.kcad` schema 212
    // (docs/adr/3560).
    "cad.pipes.profile",
    // A sheet's contour lines (docs/adr/1820), kept with their surface and the layers they read since `.kcad` schema 246
    // (docs/adr/4740).
    "road.terrains.sheetcontours",
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
    /// A utility network (docs/adr/3560): its definition and the objects of its layers.
    Network,
    /// A layer of the drawing (docs/adr/4740): the geometries of its objects in the drawing's order. A sheet's
    /// contour lines read their details, slopes, chosen points, built-up areas and roads from layers.
    Layer,
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
/// made from (its main source first), the objects it wrote in order and the stamp of those objects as written; since
/// schema 209 (docs/adr/2840) the command's version that drew it and the drawing's revision its write made.
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
    /// The version of the command that drew it (docs/adr/2840); none: a record written before schema 209.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: Option<u32>,
    /// The drawing's revision once its objects were written, as the commands say it (decimal digits; docs/adr/2840);
    /// none: a record written before schema 209.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub revision: Option<String>,
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
    /// Whether it holds terrain surfaces only, as a map project's may (`.kcad` schema 195, docs/adr/1520).
    pub fn surfaces_only(&self) -> bool {
        Self { surfaces: Vec::new(), ..self.clone() }.is_empty()
    }

    /// Whether it holds only what a map project may: terrain surfaces and graded pads (`.kcad` schema 197,
    /// docs/adr/1580).
    pub fn map_part_only(&self) -> bool {
        Self { surfaces: Vec::new(), platforms: Vec::new(), ..self.clone() }.is_empty()
    }

    /// Whether it holds only what a map project may since `.kcad` schema 198 (docs/adr/1720): terrain surfaces,
    /// graded pads, what the commands drew from them (`drawings`: contours, a pad in plan, a zero line) and whether the
    /// stale ones are drawn again by themselves (`autoRebuild`), so that a map project's drawing knows when its sources
    /// changed.
    pub fn map_drawn_only(&self) -> bool {
        Self {
            surfaces: Vec::new(),
            platforms: Vec::new(),
            drawings: Vec::new(),
            auto_rebuild: false,
            rebuild_whole: false,
            ..self.clone()
        }
        .is_empty()
    }

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
            && !self.rebuild_whole
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
        if s.edits.len() > MOST_SURFACE_EDITS {
            return fail(at("edits"), "bir yüzeyin en çok 10 000 düzenlemesi olur");
        }
        if let Some((k, words)) = s.edits.iter().enumerate().find_map(|(k, e)| e.problem().map(|w| (k, w))) {
            return fail(at(&format!("edits[{k}]")), words);
        }
        // Docs/adr/3920: a surface does not paste itself (another surface pasting it back is said when it is made).
        if let Some(k) = s.edits.iter().position(|e| matches!(e, SurfaceEdit::Paste { terrain } if *terrain == s.id)) {
            return fail(at(&format!("edits[{k}]")), "yüzey kendini yapıştıramaz");
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
            // A layer's id is the drawing's own (docs/adr/4740): it may hold spaces.
            let id_ok = if s.kind == RoadSourceKind::Layer { !s.id.is_empty() && s.id.chars().count() <= 256 } else { word_ok(&s.id) };
            if !id_ok || !stamp_ok(&s.digest) || !seen.insert((s.kind, s.id.as_str())) {
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
        // Docs/adr/2840: a command's version is 1 or more; a revision is a whole number as the commands write it.
        if d.version == Some(0) {
            return fail(at("version"), "çizimin işlem sürümü 1 ya da daha büyük olur");
        }
        if d.revision.as_deref().is_some_and(|r| r.is_empty() || r.len() > 20 || !r.bytes().all(|b| b.is_ascii_digit()) || r.parse::<u64>().is_err()) {
            return fail(at("revision"), "çizimin çıktı revizyonu bir doğal sayı olur (en çok 20 hane)");
        }
    }
    None
}

/// What is wrong with the earthwork volumes: each of a corridor in the project, its factors positive and over
/// stretches in increasing stations that do not overlap.
/// At most so many mass haul diagrams, and so many sites on one.
pub const MAX_SHEET_SETS: usize = 256;
/// The highest design speed a road takes (km/h, docs/adr/0985): a bound of sense, not a standard's. Design speed
/// tables end at 130 km/h (AASHTO Green Book, RAA); Türkiye's highest legal limit is 140 km/h on toll motorways.
pub const MAX_DESIGN_SPEED: f64 = 200.0;

fn speed_ok(v: f64) -> bool {
    v.is_finite() && v > 0.0 && v <= MAX_DESIGN_SPEED
}

fn word_ok(s: &str) -> bool {
    !s.is_empty() && s.chars().count() <= 64 && !s.chars().any(char::is_whitespace)
}

/// A digest as the road project writes it: 64 lower-case hex digits.
fn digest_ok(d: &str) -> bool {
    d.len() == 64 && d.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// An algorithm by name and version (docs/adr/1040): `kentos.end-area/1`; lower-case letters, digits, dots and dashes,
/// a slash and the version's digits, at most 64 characters.
fn algorithm_ok(t: &str) -> bool {
    let Some((name, version)) = t.split_once('/') else { return false };
    t.len() <= 64
        && !name.is_empty()
        && name.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-'))
        && !version.is_empty()
        && version.bytes().all(|b| b.is_ascii_digit())
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

    /// Docs/adr/1041: an approval's sources and algorithm as a file keeps them: ids are words, the templates at most a
    /// corridor's regions, digests 64 lower-case hex digits, the algorithm `name/version`.
    #[test]
    fn an_approvals_sources_and_algorithm_are_judged() {
        for good in ["kentos.end-area/1", "kentos.tin-overlay/12"] {
            assert!(algorithm_ok(good), "{good}");
        }
        let long = format!("{}/1", "a".repeat(63));
        for bad in ["", "kentos.end-area", "kentos.end-area/", "/1", "Kentos.end-area/1", "kentos.end area/1", "kentos.end-area/1a", &long] {
            assert!(!algorithm_ok(bad), "{bad}");
        }
        let source = |id: &str| RoadSource { id: id.into(), revision: 1 };
        let ok = VolumeSources {
            corridor: source("c-1"),
            alignment: source("g-1"),
            profile: source("b-1"),
            templates: vec![source("tk-1")],
            superelevation: None,
            surface: RoadSurfaceSource { id: "t-1".into(), digest: Some("a".repeat(64)), built: None },
        };
        assert_eq!(volume_sources_problem(&ok), None);
        let mut spaced = ok.clone();
        spaced.templates.push(source("iki sözcük"));
        assert!(volume_sources_problem(&spaced).is_some());
        let mut upper = ok.clone();
        upper.surface.built = Some("A".repeat(64));
        assert!(volume_sources_problem(&upper).is_some());
        let mut many = ok.clone();
        many.templates = vec![source("tk"); MAX_CORRIDOR_REGIONS + 1];
        assert!(volume_sources_problem(&many).is_some());
    }

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
            edits: Vec::new(),
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
            version: Some(1),
            revision: Some("12".into()),
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
        // Docs/adr/2840: the version and the revision a record keeps, or none (an older file's).
        let mut d = drawing("cz-1");
        d.version = Some(0);
        assert_eq!(of(vec![d]).as_deref(), Some("drawings[0].version"));
        for bad in ["", "1a", "-3", "123456789012345678901"] {
            let mut d = drawing("cz-1");
            d.revision = Some(bad.into());
            assert_eq!(of(vec![d]).as_deref(), Some("drawings[0].revision"), "{bad:?}");
        }
        let mut d = drawing("cz-1");
        (d.version, d.revision) = (None, None);
        assert_eq!(of(vec![d]), None);
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
            start_place: None,
            end_place: None,
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
