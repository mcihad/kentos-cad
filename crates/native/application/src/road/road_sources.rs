//! What a road answer was computed from, each as it is now (docs/adr/1038, 1039, 1040): a source by its id and
//! revision, the terrain surface by the digest of what it is made of now and the one it was last accepted from, and a
//! corridor's chain (the corridor, its alignment and profile, its templates, its superelevation schedule, its surface).
//! The stakeout table, the section sets and the earthwork volumes say theirs with these.

use kentos_commands::{CorridorDef, RoadSource, RoadSurfaceSource, VolumeSources};
use kentos_domain::Document;

/// The terrain surface `id`: the digest of what it is made of now (absent where the project has no such surface) and
/// the one it was last accepted from.
pub(crate) fn surface(doc: &Document, id: &str) -> RoadSurfaceSource {
    let def = crate::road_terrains::surfaces(doc).iter().find(|s| s.id == id);
    RoadSurfaceSource {
        id: id.to_owned(),
        digest: def.map(|d| crate::road_terrains::sources_digest(doc, d)),
        built: def.and_then(|d| d.built.clone()),
    }
}

/// What a corridor's results are computed from, each as it is now: the corridor, its alignment and profile, its
/// regions' templates in order and once each, its superelevation schedule and its terrain surface.
pub(crate) fn corridor(doc: &Document, def: &CorridorDef) -> VolumeSources {
    let alignment = doc.road_alignments().iter().find(|a| a.id == def.alignment).map_or(0, |a| a.revision);
    let profile = doc.road_profiles().iter().find(|p| p.id == def.profile).map_or(0, |p| p.revision);
    let mut templates: Vec<RoadSource> = Vec::new();
    for r in &def.regions {
        if !templates.iter().any(|t| t.id == r.template) {
            let revision = doc.road_templates().iter().find(|t| t.id == r.template).map_or(0, |t| t.revision);
            templates.push(RoadSource { id: r.template.clone(), revision });
        }
    }
    VolumeSources {
        corridor: RoadSource { id: def.id.clone(), revision: def.revision },
        alignment: RoadSource { id: def.alignment.clone(), revision: alignment },
        profile: RoadSource { id: def.profile.clone(), revision: profile },
        templates,
        superelevation: def.superelevation.as_ref().map(|id| RoadSource {
            id: id.clone(),
            revision: doc.road_superelevations().iter().find(|s| s.id == *id).map_or(0, |s| s.revision),
        }),
        surface: surface(doc, &def.surface),
    }
}

/// Whether the surface was accepted from what it is made of now, in words: "kabul edilmedi" (never), "kabul edildi",
/// "kabulden sonra değişti" (the drawing changed after it was accepted: the answer is the drawing's of now).
pub fn acceptance_words(s: &RoadSurfaceSource) -> &'static str {
    match (&s.built, &s.digest) {
        (None, _) => "kabul edilmedi",
        (Some(built), Some(now)) if built == now => "kabul edildi",
        (Some(_), _) => "kabulden sonra değişti",
    }
}

/// The surface's digest of now and its acceptance, as a sheet's cell; "projede yok" without the surface.
pub fn surface_words(s: &RoadSurfaceSource) -> String {
    match &s.digest {
        Some(digest) => format!("{digest} ({})", acceptance_words(s)),
        None => "projede yok".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_surface_says_whether_it_was_accepted_from_what_it_is_made_of_now() {
        let of = |digest: Option<&str>, built: Option<&str>| RoadSurfaceSource {
            id: "t-1".into(),
            digest: digest.map(str::to_owned),
            built: built.map(str::to_owned),
        };
        let (a, b) = ("a".repeat(64), "b".repeat(64));
        assert_eq!(acceptance_words(&of(Some(&a), None)), "kabul edilmedi");
        assert_eq!(acceptance_words(&of(Some(&a), Some(&a))), "kabul edildi");
        assert_eq!(acceptance_words(&of(Some(&a), Some(&b))), "kabulden sonra değişti");
        assert_eq!(surface_words(&of(Some(&a), Some(&a))), format!("{a} (kabul edildi)"));
        assert_eq!(surface_words(&of(None, None)), "projede yok");
    }
}
