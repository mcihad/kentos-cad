//! What KentOS Yol's terrain surface commands share (plan/road.md Y2.3, docs/adr/0900 §11): a surface made from the
//! drawing by its definition, reported as the catalog says, and the project's road settings written back. The
//! commands are [`crate::road_terrains_create`], [`crate::road_terrains_edit`] and [`crate::road_terrains_query`].

use kentos_commands::{
    CadWork, CommandWarning, ProjectSettings, RoadSettings, TerrainFigures, TerrainLeft, TerrainProblem, TerrainReport,
    TerrainState, TerrainSurfaceDef,
};
use kentos_domain::Document;
use kentos_road::sources::{self, Gathered, SourceObject, SurfaceState};
use kentos_road::surface::{self, Surface};

use crate::checks::{Stop, error};
use crate::codes;

pub(crate) fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, (!path.is_empty()).then(|| path.to_owned())))
}

/// Whether a project of this work holds terrain surfaces: Harita's and Yol's (docs/adr/1520; Netcad's NetSURF is a
/// map engineer's tool as much as a road engineer's).
pub fn holds_surfaces(work: Option<CadWork>) -> bool {
    matches!(work, Some(CadWork::Road | CadWork::Map))
}

/// The terrain surfaces' commands' check: a Harita or a Yol project (the code stays `not_a_road_project`).
pub(crate) fn terrain_project(doc: &Document) -> Result<(), Stop> {
    if holds_surfaces(doc.settings().cad_work) {
        Ok(())
    } else {
        Err(fail(
            codes::NOT_A_ROAD_PROJECT,
            "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim ikisinden biri değil.".into(),
            "",
        ))
    }
}

/// The project's surfaces.
pub(crate) fn surfaces(doc: &Document) -> &[TerrainSurfaceDef] {
    doc.settings().road.as_ref().map_or(&[], |r| &r.surfaces[..])
}

/// The surfaces judged by the contract's rules; a refusal's path is the input's `field`'s.
pub(crate) fn judged(list: &[TerrainSurfaceDef], at: usize, field: &str) -> Result<(), Stop> {
    let road = RoadSettings {
        surfaces: list.to_vec(),
        ..RoadSettings::default()
    };
    if let Some((path, words)) = road.problem() {
        let prefix = format!("surfaces[{at}].");
        let rest = path.strip_prefix(&prefix);
        let path = match rest {
            Some(rest) if field.is_empty() => rest.to_owned(),
            Some(rest) => format!("{field}.{rest}"),
            None => field.to_owned(),
        };
        return Err(fail(codes::INVALID_TERRAIN, format!("Yüzey kurallarına uymuyor: {words}."), &path));
    }
    Ok(())
}

/// A surface made from the drawing now: what it read and, when one is made, the surface or why none is.
pub(crate) struct Made {
    pub gathered: Gathered,
    pub surface: Result<Surface, TerrainProblem>,
}

/// The digest of what `def` is made of now (its sources and method, `sources::gather`'s), without making it.
pub(crate) fn sources_digest(doc: &Document, def: &TerrainSurfaceDef) -> String {
    let uids: Vec<(String, &kentos_commands::Entity)> =
        doc.entities_with_uids().map(|(uid, e)| (uid.to_string(), e)).collect();
    sources::gather_among(def, surfaces(doc), doc.layers().nodes(), uids.iter().map(|(uid, entity)| SourceObject { uid, entity }))
        .digest
}

pub(crate) fn make(doc: &Document, def: &TerrainSurfaceDef) -> Made {
    let uids: Vec<(String, &kentos_commands::Entity)> =
        doc.entities_with_uids().map(|(uid, e)| (uid.to_string(), e)).collect();
    // Docs/adr/3920: a paste edit finds the surface it pastes among the project's.
    let gathered = sources::gather_among(
        def,
        surfaces(doc),
        doc.layers().nodes(),
        uids.iter().map(|(uid, entity)| SourceObject { uid, entity }),
    );
    let surface = surface::build(&gathered.input).map_err(|e| TerrainProblem {
        code: e.code.to_owned(),
        message: problem_words(e.code).to_owned(),
        at: e.at,
    });
    Made { gathered, surface }
}

impl Made {
    pub(crate) fn report(&self, def: &TerrainSurfaceDef) -> TerrainReport {
        let state = match (&self.surface, self.gathered.state(def)) {
            (Err(_), _) => TerrainState::Unbuildable,
            (Ok(_), SurfaceState::Current) => TerrainState::Current,
            (Ok(_), SurfaceState::Stale) => TerrainState::Stale,
            (Ok(_), SurfaceState::New) => TerrainState::New,
        };
        TerrainReport {
            id: def.id.clone(),
            name: def.name.clone(),
            state,
            digest: self.gathered.digest.clone(),
            built: def.built.clone(),
            left: self
                .gathered
                .left
                .iter()
                .map(|l| TerrainLeft {
                    id: l.id.clone(),
                    role: l.role.to_owned(),
                    why: l.why.to_owned(),
                })
                .chain(self.skipped_edits().map(|(id, why)| TerrainLeft {
                    id,
                    role: "edits".to_owned(),
                    why: why.to_owned(),
                }))
                .collect(),
            algorithm: self.surface.is_ok().then(|| kentos_road::surface::ALGORITHM.to_owned()),
            figures: self.surface.as_ref().ok().map(|s| {
                let f = s.figures();
                let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
                TerrainFigures {
                    vertices: count(f.vertices),
                    triangles: count(f.triangles),
                    constrained: count(f.constrained),
                    area: f.area,
                    min_z: f.min_z,
                    max_z: f.max_z,
                    merged: count(s.report.merged),
                    split: count(s.report.split),
                    trimmed: count(s.report.trimmed),
                    removed: count(s.report.removed),
                }
            }),
            problem: self.surface.as_ref().err().cloned(),
        }
    }

    /// The edits the core could not do on the surface made now (docs/adr/1560): `edits[k]` and why.
    fn skipped_edits(&self) -> impl Iterator<Item = (String, &'static str)> + '_ {
        self.surface
            .iter()
            .flat_map(|s| s.report.skipped.iter())
            .map(|s| (format!("edits[{}]", s.edit), s.why))
    }

    /// The warnings of what it could not use.
    pub(crate) fn warnings(&self) -> Vec<CommandWarning> {
        let left = self.gathered.left.iter().map(|l| (l.id.clone(), l.role, l.why));
        let skipped = self.skipped_edits().map(|(id, why)| (id, "edits", why));
        left.chain(skipped)
            .map(|(id, role, why)| CommandWarning {
                code: codes::TERRAIN_SOURCE_LEFT.into(),
                message: format!("{id} ({}) kullanılamadı: {}.", role_words(role), why_words(why)),
                path: Some(role.to_owned()),
            })
            .collect()
    }

    /// The surface, or the refusal that none is made.
    pub(crate) fn built(&self) -> Result<&Surface, Stop> {
        self.surface.as_ref().map_err(|p| {
            let place = p.at.map_or(String::new(), |[x, y]| format!(" (yer: {x}, {y})"));
            fail(
                codes::TERRAIN_UNBUILDABLE,
                format!("Kaynaklardan yüzey kurulamıyor: {}{place}.", p.message),
                "",
            )
        })
    }
}

/// Why no surface is made, by the core's code (ADR 0900 §2–§5), in the user's words.
fn problem_words(code: &str) -> &'static str {
    match code {
        "too_few_points" => "kotlu yer üçten az ya da hepsi bir doğru üstünde",
        "height_ambiguous" => "bir yerde iki farklı kot var",
        "height_conflict" => "bir nokta bir kırık hattın üstünde ama kotu hattınkinden farklı",
        "constraints_cross" => "iki kırık hat ya da bir kırık hatla bir halka birbirini kesiyor",
        "boundary_outside" => "sınırın ya da bir boşluğun köşesi noktaların dışında",
        "invalid_input" => "bir kaynağın geometrisi geçersiz (sıfır boylu parça ya da üçten az köşeli halka)",
        _ => "üçgenleme tamamlanamadı",
    }
}

fn role_words(role: &str) -> &'static str {
    match role {
        "points" => "nokta",
        "breaklines" => "kırık hat",
        "boundary" => "sınır",
        "edits" => "düzenleme",
        _ => "boşluk",
    }
}

/// Why a source or an edit could not be used, in the user's words (a report's `left[].why`).
pub fn why_words(why: &str) -> &'static str {
    match why {
        "missing" => "çizimde yok",
        "no_height" => "kotu eksik",
        "arc_edge" => "yaylı kenarı var",
        "wrong_kind" => "türü uymuyor",
        "not_closed" => "kapalı değil",
        "occupied" => "o yerde zaten bir kayıt var; kotunu Kot değiştir ile verin",
        "no_point" => "o yerde nokta ya da kırık hat köşesi yok",
        "no_corner" => "bir ucu yüzeyin köşesi değil",
        "crosses" => "bir kırık hatla, halkayla ya da önceki bir kenarla kesişiyor",
        "no_triangle" => "o yerde yüzeyin üçgeni yok",
        "no_surface" => "projede bu kimlikli yüzey yok",
        "cycle" => "yapıştırılan yüzey, doğrudan ya da başka bir yüzey üzerinden, bu yüzeyi yapıştırıyor",
        "unbuildable" => "yapıştırılan yüzey kaynaklarından kurulamıyor",
        _ => "köşesi az",
    }
}

/// The project's settings with these surfaces (none: no road settings).
pub(crate) fn with_surfaces(doc: &Document, list: Vec<TerrainSurfaceDef>) -> ProjectSettings {
    let mut road = doc.settings().road.clone().unwrap_or_default();
    road.surfaces = list;
    ProjectSettings {
        road: (!road.is_empty()).then_some(road),
        ..doc.settings().clone()
    }
}

/// One definition made from the drawing now, saved or not (the window's edits, docs/adr/1560): its report and its
/// surface (none when none is made).
pub fn made_of(doc: &Document, def: &TerrainSurfaceDef) -> (TerrainReport, Option<Surface>) {
    let made = make(doc, def);
    (made.report(def), made.surface.ok())
}

/// The road project's surfaces as their sources make them now, for the desktop's drawing and window (plan/road.md
/// Y2.4): each definition, its report and its surface (none when none is made). None outside a road project.
pub fn made_surfaces(doc: &Document) -> Vec<(TerrainSurfaceDef, TerrainReport, Option<Surface>)> {
    if !holds_surfaces(doc.settings().cad_work) {
        return Vec::new();
    }
    surfaces(doc)
        .iter()
        .map(|d| {
            let made = make(doc, d);
            let report = made.report(d);
            (d.clone(), report, made.surface.ok())
        })
        .collect()
}
