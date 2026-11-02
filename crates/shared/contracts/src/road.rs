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
}

impl RoadSettings {
    /// Whether it says nothing (and is not written).
    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
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
            let got = road_problem(&RoadSettings { surfaces: vec![s] }).map(|p| p.0);
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
        };
        assert_eq!(road_problem(&two).map(|p| p.0).as_deref(), Some("surfaces[1].name"));
    }
}
