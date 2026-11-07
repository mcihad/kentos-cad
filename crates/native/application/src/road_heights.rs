//! Whether surfaces compared rest on one vertical system (plan/road.md ROAD-NUM-04, docs/adr/1029). A comparison of
//! two grounds (a survey's progress, a terrain's difference) is a difference of heights: two surfaces whose heights
//! stand on different vertical systems differ by the systems' separation everywhere, and in Türkiye an ellipsoidal
//! height stands some 30–40 m over an orthometric one. Neither a zero separation nor a guessed geoid is assumed.
//!
//! A surface's heights come from the drawing's own points and lines, which are the project's (`settings.heights`), and
//! from the ground points taken from point clouds, which are their files' (`SurfaceCloudDef::vertical`, recorded when
//! taken). Surfaces made only of the drawing's points rest on one system whatever it is, the drawing's. Otherwise every
//! source's system must be known and the same; a source not known is refused unless the call says to take the heights
//! as one (`assumeHeights`), when the result is a draft (`heights_assumed`). Two systems known to differ are refused
//! even then: the separation is not zero by an assumption.

use std::collections::HashSet;

use kentos_commands::{
    CloudFormat, CommandWarning, HeightKind, HeightReference, PointCloudEntity, SurfaceCloudDef, TerrainSurfaceDef, VERTICAL_SYSTEMS,
    vertical_system,
};
use kentos_domain::Document;

use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::fail;

/// A vertical system: ellipsoidal heights, one of EPSG's by its code, or as a file names one KentOS does not know.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum System {
    Ellipsoidal,
    Srid(u32),
    Named(String),
}

impl System {
    /// The system a file names: `EPSG:n`, or a WKT's name, matched to EPSG's by name where KentOS knows it.
    pub(crate) fn of_file(text: &str) -> System {
        let t = text.trim();
        if let Some(n) = t.strip_prefix("EPSG:").and_then(|n| n.trim().parse::<u32>().ok()) {
            return System::Srid(n);
        }
        match VERTICAL_SYSTEMS.iter().find(|v| v.name.eq_ignore_ascii_case(t)) {
            Some(v) => System::Srid(v.srid),
            None => System::Named(t.to_owned()),
        }
    }

    /// The project's, when its settings say it.
    pub(crate) fn of_project(doc: &Document) -> Option<System> {
        System::of_ref(doc.settings().heights.as_ref()?)
    }

    /// What a height reference says, when it says a system.
    pub(crate) fn of_ref(h: &HeightReference) -> Option<System> {
        match h.kind {
            HeightKind::Ellipsoidal => Some(System::Ellipsoidal),
            HeightKind::Orthometric => h.vertical_srid.map(System::Srid),
            HeightKind::Unknown => None,
        }
    }

    /// The system a cloud record's points are in (docs/adr/1032): the project's they were written in when the call
    /// said the clouds' heights, else the files' when they named one.
    pub(crate) fn of_record(c: &SurfaceCloudDef) -> Option<System> {
        match &c.heights {
            Some(h) => System::of_ref(&h.to),
            None => c.vertical.as_deref().map(System::of_file),
        }
    }

    pub(crate) fn words(&self) -> String {
        match self {
            System::Ellipsoidal => "elipsoit yüksekliği".into(),
            System::Srid(s) => vertical_system(*s).map_or_else(|| format!("EPSG:{s}"), |v| format!("{} (EPSG:{s})", v.name)),
            System::Named(n) => format!("“{n}”"),
        }
    }
}

/// Where a surface's heights come from: the drawing's points and lines, or the ground of the clouds on a layer and the
/// system its points are in, when known.
enum Source<'a> {
    Drawing,
    Cloud { layer: &'a str, system: Option<System> },
}

fn sources(def: &TerrainSurfaceDef) -> Vec<Source<'_>> {
    let clouds: HashSet<&str> = def.clouds.iter().map(|c| c.layer.as_str()).collect();
    let mut out = Vec::new();
    if def.points.layers.iter().any(|l| !clouds.contains(l.as_str())) || !def.points.uids.is_empty() || !def.breaklines.is_empty() {
        out.push(Source::Drawing);
    }
    out.extend(def.clouds.iter().map(|c| Source::Cloud { layer: &c.layer, system: System::of_record(c) }));
    out
}

/// One side of a comparison: its name as the words say it, and its definition; none for the design (the drawing's).
pub(crate) type Side<'a> = (&'a str, Option<&'a TerrainSurfaceDef>);

/// Whether `sides` rest on one vertical system. When a source is not known and `assume`, a `heights_assumed` warning;
/// otherwise `unproven_heights` (at `path`). Systems known to differ are refused whatever `assume`.
pub(crate) fn proven(doc: &Document, sides: &[Side<'_>], assume: bool, path: &str, warnings: &mut Vec<CommandWarning>) -> Result<(), Stop> {
    let all: Vec<(&str, Source<'_>)> = sides
        .iter()
        .flat_map(|(name, def)| match def {
            Some(d) => sources(d).into_iter().map(|s| (*name, s)).collect::<Vec<_>>(),
            None => vec![(*name, Source::Drawing)],
        })
        .collect();
    if all.iter().all(|(_, s)| matches!(s, Source::Drawing)) {
        return Ok(());
    }
    let project = System::of_project(doc);
    // The drawing's own heights are one source whichever surfaces hold them: named once, with those surfaces.
    let mut drawing: Vec<&str> = Vec::new();
    let mut unknown = Vec::new();
    let mut known: Vec<(String, System)> = Vec::new();
    for (name, s) in &all {
        match s {
            Source::Drawing => {
                if !drawing.contains(name) {
                    drawing.push(name);
                }
            }
            Source::Cloud { layer, system } => match system {
                Some(v) => known.push((format!("“{name}” yüzeyinin “{layer}” katmanındaki bulut noktaları"), v.clone())),
                None => unknown.push(format!(
                    "“{name}” yüzeyinin “{layer}” katmanındaki bulut noktaları (dosya düşey sistemini söylemiyor ya da alındığında kaydedilmedi)"
                )),
            },
        }
    }
    if !drawing.is_empty() {
        let names = drawing.iter().map(|n| format!("“{n}”")).collect::<Vec<_>>().join(", ");
        match &project {
            Some(p) => known.insert(0, (format!("çizimin kendi kotları ({names})"), p.clone())),
            None => unknown.insert(0, format!("çizimin kendi kotları ({names}; projenin kotlarının düşey sistemi söylenmemiş)")),
        }
    }
    if let Some((first, sys)) = known.first()
        && let Some((other, theirs)) = known.iter().find(|(_, s)| s != sys)
    {
        return Err(fail(
            codes::UNPROVEN_HEIGHTS,
            format!(
                "Karşılaştırılan kotlar iki ayrı düşey sistemde: {first} {}, {other} {}. Aradaki ayrım sıfır sayılmaz; kotları önce aynı sisteme çevirin (Koordinat dönüştür, geoit modeliyle).",
                sys.words(),
                theirs.words()
            ),
            path,
        ));
    }
    if unknown.is_empty() {
        return Ok(());
    }
    let what = unknown.join("; ");
    if !assume {
        return Err(fail(
            codes::UNPROVEN_HEIGHTS,
            format!(
                "Karşılaştırılan kotların aynı düşey sistemde olduğu gösterilemiyor: {what}. Sıfır ayrım ya da geoit tahmini yapılmaz. Projenin kotlarını Proje ayarlarında söyleyin, ya da kotları aynı saymak için assumeHeights verin (sonuç taslak olur)."
            ),
            "assumeHeights",
        ));
    }
    warnings.push(CommandWarning {
        code: codes::HEIGHTS_ASSUMED.into(),
        message: format!(
            "Kotlar aynı düşey sistemde sayıldı (assumeHeights): {what}. Sonuç bu varsayıma dayanan bir taslaktır."
        ),
        path: Some("assumeHeights".into()),
    });
    Ok(())
}

/// The vertical system the sources of `clouds` name (docs/adr/1029), to record with the ground taken from them: the
/// files' words when every source read names one and they are one system; none when a source names none (an XYZ file
/// names none). Sources that cannot be opened are left to the scan's warning. Sources naming different systems, or one
/// differing from the project's or from the surface's other clouds', are refused (`cloud_heights_differ`).
pub(crate) fn of_clouds(
    doc: &Document,
    drawing: Option<&str>,
    clouds: &[(String, PointCloudEntity)],
    others: &[(&str, Option<System>)],
    stated: Option<&HeightReference>,
    path: &str,
) -> Result<Option<String>, Stop> {
    // Each source read: what it names (its words and the system), or none.
    let mut named: Vec<(String, String, System)> = Vec::new();
    let mut silent = false;
    for (k, (_, c)) in clouds.iter().enumerate() {
        for (si, s) in c.cloud.sources.iter().enumerate() {
            if s.format == CloudFormat::Xyz {
                silent = true;
                continue;
            }
            let Ok(cloud) = crate::pointclouds::bytes_of(doc, drawing, s).and_then(|b| crate::pointclouds::open(&b)) else {
                continue;
            };
            match crate::pointclouds::vertical(&cloud) {
                Some(v) => named.push((format!("{}. bulutun {}. kaynağı", k + 1, si + 1), v.clone(), System::of_file(&v))),
                None => silent = true,
            }
        }
    }
    let differ = |words: String| Err(fail(codes::CLOUD_HEIGHTS_DIFFER, words, path));
    // Docs/adr/1032: heights the call says are the clouds'; a file naming another is refused, and the points go into
    // the project's system (the surface's other clouds are held to it).
    if let Some(said) = stated.and_then(System::of_ref) {
        if let Some((first, _, sys)) = named.iter().find(|(_, _, s)| *s != said) {
            return differ(format!(
                "{first} kotların {} olduğunu söylüyor; çağrı {} diyor (heights). Dosyanın söylediğini verin ya da heights'ı bırakın.",
                sys.words(),
                said.words()
            ));
        }
        if let Some(p) = System::of_project(doc)
            && let Some((layer, v)) = others.iter().find_map(|(l, v)| v.clone().filter(|v| *v != p).map(|v| (l, v)))
        {
            return differ(format!(
                "Bulutun noktaları projenin kotlarına ({}) çevrilir; yüzeyin “{layer}” katmanındaki bulut noktaları {}: bir yüzeyin noktaları tek sistemde olmalı.",
                p.words(),
                v.words()
            ));
        }
        return Ok(named.first().filter(|_| !silent).map(|(_, text, _)| text.clone()));
    }
    let Some((first, text, sys)) = named.first() else { return Ok(None) };
    if let Some((other, _, theirs)) = named.iter().find(|(_, _, s)| s != sys) {
        return differ(format!(
            "Bulutların kaynakları ayrı düşey sistemlerde: {first} {}, {other} {}. Bir yüzeyin noktaları tek sistemde olmalı; kaynakları önce aynı sisteme çevirin.",
            sys.words(),
            theirs.words()
        ));
    }
    if let Some(p) = System::of_project(doc)
        && p != *sys
    {
        return differ(format!(
            "Bulutun kotları {}, projenin kotları {}: aradaki ayrım sıfır sayılmaz, bulut noktaları çevrilmeden zemine alınmaz. Bulutu projenin sistemine çevirin ya da projenin kotlarını Proje ayarlarında düzeltin.",
            sys.words(),
            p.words()
        ));
    }
    if let Some((layer, v)) = others.iter().find_map(|(l, v)| v.clone().filter(|v| v != sys).map(|v| (l, v))) {
        return differ(format!(
            "Bulutun kotları {}, yüzeyin “{layer}” katmanındaki bulut noktalarınınki {}: bir yüzeyin noktaları tek sistemde olmalı.",
            sys.words(),
            v.words()
        ));
    }
    Ok((!silent).then(|| text.clone()))
}
