//! What KentOS Yol's cross section commands share (plan/road.md Y5, docs/adr/0903): the templates and sample line sets
//! of the project, a set's stations by its method, its sections built by `kentos-road` (the terrain along each sample
//! line, the template's platform on the profile's elevation, its slopes tied to the terrain), and their reports. The
//! commands are [`crate::road_templates_edit`], [`crate::road_sections_edit`] and [`crate::road_sections_query`].

use kentos_commands::{
    CommandWarning, Entity, RoadSettings, SectionGround, SectionRow, SectionSetDef, SectionSetReport, SectionSide,
    SectionStations, TemplateComponentKind, TemplateDef, TemplateListed, TemplateSideDef,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::section::template::{self as tpl, Component, Ditch, Kind, Side, Template};
use kentos_road::section::{ground, sample_line};
use kentos_road::surface::Surface;

use crate::checks::Stop;
use crate::codes;
use crate::road_alignments::{self as alignments, Built};
use crate::road_terrains::{self as terrains, fail};

/// Stations closer than this (m) are one.
pub const SAME_STATION: f64 = 1e-3;

/// Only a road project has cross sections.
pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work == Some(kentos_commands::CadWork::Road) {
        Ok(())
    } else {
        Err(fail(
            codes::NOT_A_ROAD_PROJECT,
            "Enkesit yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(),
            "",
        ))
    }
}

/// The index of a definition by its id, else by its name (case aside).
fn by_name<T>(list: &[T], name: &str, id: impl Fn(&T) -> &str, words: impl Fn(&T) -> &str) -> Option<usize> {
    list.iter()
        .position(|t| id(t) == name)
        .or_else(|| list.iter().position(|t| words(t).to_lowercase() == name.to_lowercase()))
}

pub(crate) fn find_template<'a>(list: &'a [TemplateDef], name: &str, path: &str) -> Result<(usize, &'a TemplateDef), Stop> {
    by_name(list, name, |t| &t.id, |t| &t.name).map(|i| (i, &list[i])).ok_or_else(|| {
        fail(codes::UNKNOWN_ROAD_TEMPLATE, format!("“{name}” kimlikli ya da adlı tip kesit projede yok."), path)
    })
}

pub(crate) fn find_set<'a>(list: &'a [SectionSetDef], name: &str, path: &str) -> Result<(usize, &'a SectionSetDef), Stop> {
    by_name(list, name, |t| &t.id, |t| &t.name).map(|i| (i, &list[i])).ok_or_else(|| {
        fail(codes::UNKNOWN_SECTION_SET, format!("“{name}” kimlikli ya da adlı kesit takımı projede yok."), path)
    })
}

/// The road settings judged by the contract's rules; a refusal (`code`) at the field of the list item it names.
pub(crate) fn judged(road: &RoadSettings, code: &str, what: &str) -> Result<(), Stop> {
    if let Some((path, words)) = road.problem() {
        let path = path.split_once("].").map_or_else(String::new, |(_, field)| field.to_owned());
        return Err(fail(code, format!("{what} kurallarına uymuyor: {words}."), &path));
    }
    Ok(())
}

/// The project's road settings with these templates and sets.
pub(crate) fn road_with(doc: &Document, templates: Option<&[TemplateDef]>, sets: Option<&[SectionSetDef]>) -> RoadSettings {
    let mut road = doc.settings().road.clone().unwrap_or_default();
    if let Some(t) = templates {
        road.templates = t.to_vec();
    }
    if let Some(s) = sets {
        road.sections = s.to_vec();
    }
    road
}

/// The template as the core takes it: the right side the left's mirror when absent.
pub fn core_template(def: &TemplateDef) -> Template {
    let side = |s: &TemplateSideDef| Side {
        components: s
            .components
            .iter()
            .map(|c| match c.kind {
                TemplateComponentKind::Lane => Component::Lane { width: c.width, slope: c.slope },
                TemplateComponentKind::Shoulder => Component::Shoulder { width: c.width, slope: c.slope },
            })
            .collect(),
        cut: s.cut,
        fill: s.fill,
        ditch: s.ditch.as_ref().map(|d| Ditch { fore: d.fore, depth: d.depth, bottom: d.bottom }),
    };
    Template { left: side(&def.left), right: side(def.right.as_ref().unwrap_or(&def.left)) }
}

pub(crate) fn listed_templates(doc: &Document, list: &[TemplateDef]) -> Vec<TemplateListed> {
    list.iter()
        .map(|t| TemplateListed {
            id: t.id.clone(),
            name: t.name.clone(),
            revision: t.revision,
            used_by: doc.road_sections().iter().filter(|s| s.template.as_deref() == Some(&t.id)).map(|s| s.id.clone()).collect(),
        })
        .collect()
}

/// Where a path first crosses the alignment: the station of the first place its offset changes sign, found by halving.
fn crossing(built: &Built, path: &[[f64; 2]]) -> Option<f64> {
    let offset = |p: [f64; 2]| built.core.locate(p).feet.first().map(|f| (f.at.station, f.offset));
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (Some((_, oa)), Some((_, ob))) = (offset(a), offset(b)) else { continue };
        if oa == 0.0 {
            return offset(a).map(|f| f.0);
        }
        if (oa < 0.0) == (ob < 0.0) && ob != 0.0 {
            continue;
        }
        let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
        let at = |t: f64| [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
        for _ in 0..60 {
            let m = 0.5 * (lo + hi);
            match offset(at(m)) {
                Some((_, om)) if (om < 0.0) == (oa < 0.0) && om != 0.0 => lo = m,
                Some(_) => hi = m,
                None => break,
            }
        }
        return offset(at(0.5 * (lo + hi))).map(|f| f.0);
    }
    None
}

/// A set's stations by its method, in order, those within [`SAME_STATION`] one.
pub(crate) fn stations_of(doc: &Document, def: &SectionSetDef, built: &Built) -> Result<Vec<f64>, Stop> {
    let length = built.core.length;
    let off = |path: &str, words: String| fail(codes::INVALID_SECTION_SET, words, path);
    let mut out: Vec<f64> = match &def.stations {
        SectionStations::Range { from, to, every, geometry } => {
            if *from < -1e-9 {
                return Err(off("stations.from", format!("Km aralığı aksın üstünde olmalı: aks 0 ile {length:.3} m arası.")));
            }
            if *to > length + 1e-9 {
                return Err(off("stations.to", format!("Km aralığı aksın üstünde olmalı: aks 0 ile {length:.3} m arası.")));
            }
            if (to - from) / every > kentos_commands::MAX_SECTION_STATIONS as f64 {
                return Err(off(
                    "stations.every",
                    format!("Bu aralıkla {}'den çok kesit çıkar; every'yi büyütün.", kentos_commands::MAX_SECTION_STATIONS),
                ));
            }
            let mut s: Vec<f64> = alignments::tick_stations(built, *every)?
                .into_iter()
                .filter(|s| *s >= from - 1e-9 && *s <= to + 1e-9)
                .collect();
            s.extend([*from, *to]);
            if *geometry {
                s.extend(built.core.elements.iter().map(|e| e.station).filter(|s| *s > *from && *s < *to));
                if let Some(p) = def.profile.as_ref().and_then(|p| doc.road_profiles().iter().find(|q| q.id == *p)) {
                    s.extend(p.pvis.iter().map(|v| v.station).filter(|s| *s > *from && *s < *to));
                }
            }
            s
        }
        SectionStations::Stations { stations } => {
            for (i, s) in stations.iter().enumerate() {
                if *s < -1e-9 || *s > length + 1e-9 {
                    return Err(off(
                        &format!("stations.stations[{i}]"),
                        format!("{s} m aksın üstünde değil: aks 0 ile {length:.3} m arası."),
                    ));
                }
            }
            stations.clone()
        }
        SectionStations::Lines { uids } => {
            let mut out = Vec::new();
            for (i, uid) in uids.iter().enumerate() {
                let path = format!("stations.uids[{i}]");
                let entity = doc.entities_with_uids().find(|(u, _)| u.to_string() == *uid).map(|(_, e)| e);
                let pts: Vec<[f64; 2]> = match entity {
                    None => return Err(off(&path, format!("“{uid}” kimlikli nesne çizimde yok."))),
                    Some(Entity::Line(l)) => vec![[l.a.x, l.a.y], [l.b.x, l.b.y]],
                    Some(Entity::Polyline(p)) => p.pts.iter().map(|v| [v.x, v.y]).collect(),
                    Some(_) => return Err(off(&path, format!("“{uid}” bir çizgi ya da çoklu çizgi değil."))),
                };
                match crossing(built, &pts) {
                    Some(s) => out.push(s),
                    None => return Err(off(&path, format!("“{uid}” aksı kesmiyor."))),
                }
            }
            out
        }
    };
    out.sort_by(f64::total_cmp);
    let mut kept: Vec<f64> = Vec::with_capacity(out.len());
    for s in out {
        if kept.last().is_none_or(|k| s - k >= SAME_STATION) {
            kept.push(s.clamp(0.0, length));
        }
    }
    if kept.is_empty() {
        return Err(off("stations", "Kesit takımının hiç km'si yok.".into()));
    }
    Ok(kept)
}

/// The set's alignment built; its refusal at `path`.
pub(crate) fn alignment_of(doc: &Document, def: &SectionSetDef) -> Result<Built, Stop> {
    let list = alignments::alignments(doc);
    let a = list.iter().find(|a| a.id == def.alignment).ok_or_else(|| {
        fail(codes::UNKNOWN_ALIGNMENT, format!("“{}” kimlikli aks projede yok.", def.alignment), "alignment")
    })?;
    alignments::build(list, a, "")
}

pub(crate) fn stale(doc: &Document, def: &SectionSetDef) -> bool {
    doc.road_alignments().iter().find(|a| a.id == def.alignment).is_some_and(|a| a.revision != def.alignment_revision)
}

pub(crate) fn report(doc: &Document, def: &SectionSetDef, stations: Vec<f64>) -> SectionSetReport {
    SectionSetReport {
        id: def.id.clone(),
        name: def.name.clone(),
        revision: def.revision,
        alignment: def.alignment.clone(),
        profile: def.profile.clone(),
        template: def.template.clone(),
        surface: def.surface.clone(),
        stale: stale(doc, def),
        stations,
    }
}

/// Every set's report; a set whose stations no longer come out (its line erased, its alignment shortened) reports
/// none, the others still do.
pub(crate) fn reports(doc: &Document, list: &[SectionSetDef]) -> Vec<SectionSetReport> {
    list.iter()
        .map(|d| {
            let stations = alignment_of(doc, d).and_then(|b| stations_of(doc, d, &b)).unwrap_or_default();
            report(doc, d, stations)
        })
        .collect()
}

/// The set's surface made now, or the warning `section_surface_gone`; none for a set without one.
fn surface_of(doc: &Document, def: &SectionSetDef) -> Option<Result<Surface, CommandWarning>> {
    let id = def.surface.as_deref()?;
    let gone = |why: String| CommandWarning {
        code: codes::SECTION_SURFACE_GONE.into(),
        message: format!("“{}” kesit takımının yüzeyi {why}; arazi kesiti verilmedi.", def.name),
        path: Some("set".into()),
    };
    let Some(sdef) = terrains::surfaces(doc).iter().find(|s| s.id == id) else {
        return Some(Err(gone(format!("({id}) projede yok"))));
    };
    Some(terrains::make(doc, sdef).surface.map_err(|_| gone(format!("“{}” kurulamıyor", sdef.name))))
}

fn side_row(s: &tpl::SideSection) -> SectionSide {
    SectionSide {
        kind: match s.kind {
            Kind::Cut => "cut",
            Kind::Fill => "fill",
            Kind::Open => "open",
        }
        .into(),
        hinge: s.hinge,
        daylight: s.daylight,
        points: s.points.clone(),
    }
}

/// A set's sections at `stations`, and the warnings they give (the surface gone, the open sides).
pub(crate) fn sections(
    doc: &Document,
    def: &SectionSetDef,
    built: &Built,
    stations: &[f64],
) -> (Vec<SectionRow>, Vec<CommandWarning>) {
    let mut warnings = Vec::new();
    let surface = match surface_of(doc, def) {
        Some(Ok(s)) => Some(s),
        Some(Err(w)) => {
            warnings.push(w);
            None
        }
        None => None,
    };
    let vertical = def
        .profile
        .as_ref()
        .and_then(|p| doc.road_profiles().iter().find(|q| q.id == *p))
        .and_then(|p| kentos_road::profile::vertical::build(&crate::road_profiles::pvis_of(p)).ok());
    let template = def.template.as_ref().and_then(|t| doc.road_templates().iter().find(|q| q.id == *t)).map(core_template);
    let mut open: Vec<String> = Vec::new();
    let mut rows = Vec::new();
    for &s in stations {
        let Some(line) = sample_line(&built.core, s, def.left, def.right) else { continue };
        let km = built.km(s);
        let g = surface.as_ref().map(|surf| ground(surf, &line)).unwrap_or_default();
        let axis = vertical.as_ref().and_then(|v| v.at(s)).map(|r| r.0);
        let built_section = match (&template, axis, surface.is_some()) {
            (Some(t), Some(z), true) => tpl::build(t, z, &g).ok(),
            _ => None,
        };
        if let Some(sec) = &built_section {
            for (side, word) in [(&sec.left, "sol"), (&sec.right, "sağ")] {
                if side.kind == Kind::Open {
                    open.push(format!("km {} {word}", km_text(km, 3)));
                }
            }
        }
        rows.push(SectionRow {
            station: s,
            km,
            km_text: km_text(km, 3),
            ends: line.ends,
            axis,
            ground: g.iter().map(|p| SectionGround { offset: p.offset, z: p.z, gap: p.gap }).collect(),
            left: built_section.as_ref().map(|x| side_row(&x.left)),
            right: built_section.as_ref().map(|x| side_row(&x.right)),
            cut: built_section.as_ref().map(|x| x.cut),
            fill: built_section.as_ref().map(|x| x.fill),
        });
    }
    if !open.is_empty() {
        warnings.push(CommandWarning {
            code: codes::SECTION_OPEN.into(),
            message: format!(
                "Şu yanların şevi şerit genişliği içinde araziyle buluşmuyor: {}. Şeridi genişletin.",
                open.join(", ")
            ),
            path: Some("set".into()),
        });
    }
    (rows, warnings)
}

/// The set-stale warning.
pub(crate) fn stale_warning(doc: &Document, def: &SectionSetDef) -> Option<CommandWarning> {
    let now = doc.road_alignments().iter().find(|a| a.id == def.alignment).map_or(0, |a| a.revision);
    stale(doc, def).then(|| CommandWarning {
        code: codes::SECTION_SET_STALE.into(),
        message: format!(
            "“{}” kesit takımının aksı değişti (takım {}. revizyona göre kurulu, aks şimdi {now}.); km'leri denetleyip aksa yeniden bağlayın.",
            def.name, def.alignment_revision
        ),
        path: Some("set".into()),
    })
}

/// The road project's sample line sets and their sections now, for the desktop's Enkesit window (plan/road.md Y5).
pub fn made_sections(doc: &Document) -> Vec<(SectionSetDef, Result<Vec<SectionRow>, String>)> {
    if road_project(doc).is_err() {
        return Vec::new();
    }
    doc.road_sections()
        .iter()
        .map(|d| {
            let rows = alignment_of(doc, d)
                .and_then(|b| stations_of(doc, d, &b).map(|s| sections(doc, d, &b, &s).0))
                .map_err(|stop| match stop {
                    Stop::Failed(e) | Stop::Conflict(e) => e.message,
                    Stop::Cancelled => String::new(),
                });
            (d.clone(), rows)
        })
        .collect()
}
