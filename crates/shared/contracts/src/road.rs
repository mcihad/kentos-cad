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
}

impl RoadSettings {
    /// Whether it says nothing (and is not written).
    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty() && self.alignments.is_empty()
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
}
