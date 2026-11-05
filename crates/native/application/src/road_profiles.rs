//! What KentOS Yol's profile commands share (plan/road.md Y4, docs/adr/0902): a vertical alignment built from its
//! definition on its alignment by `kentos-road` (`kentos.profile/1`), reported as the catalog says, its red and black
//! elevations at stations, and the profiles written back as one undo step. The commands are
//! [`crate::road_profiles_create`], [`crate::road_profiles_edit`] and [`crate::road_profiles_query`].

use kentos_commands::{
    CommandResult, CommandWarning, ProfileAt, ProfileCurveRow, ProfileDef, ProfileGradeRow, RoadProfileListed,
    ProfileReport, ProfilesWritten, RoadSettings,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::profile::{self as core, Pvi, VCurve, Vertical, VerticalError};
use kentos_road::surface::Surface;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_alignments::{self as alignments, Built as AlignmentBuilt};
use crate::road_terrains::{self as terrains, fail};

/// Only a road project has profiles.
pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work == Some(kentos_commands::CadWork::Road) {
        Ok(())
    } else {
        Err(fail(
            codes::NOT_A_ROAD_PROJECT,
            "Boykesit yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(),
            "",
        ))
    }
}

/// The project's profiles.
pub(crate) fn profiles(doc: &Document) -> &[ProfileDef] {
    doc.road_profiles()
}

/// The profile of an id, else of a name (case aside).
pub(crate) fn find<'a>(list: &'a [ProfileDef], name: &str, path: &str) -> Result<(usize, &'a ProfileDef), Stop> {
    list.iter()
        .position(|p| p.id == name)
        .or_else(|| list.iter().position(|p| p.name.to_lowercase() == name.to_lowercase()))
        .map(|i| (i, &list[i]))
        .ok_or_else(|| fail(codes::UNKNOWN_ROAD_PROFILE, format!("“{name}” kimlikli ya da adlı boykesit projede yok."), path))
}

/// The surface of an id, else of a name (case aside): its id.
pub(crate) fn surface_id(doc: &Document, name: &str, path: &str) -> Result<String, Stop> {
    let list = terrains::surfaces(doc);
    list.iter()
        .find(|s| s.id == name)
        .or_else(|| list.iter().find(|s| s.name.to_lowercase() == name.to_lowercase()))
        .map(|s| s.id.clone())
        .ok_or_else(|| fail(codes::UNKNOWN_TERRAIN, format!("“{name}” kimlikli ya da adlı yüzey projede yok."), path))
}

/// The profiles judged by the contract's rules with the project's alignments; a refusal's path is the field's.
pub(crate) fn judged(doc: &Document, list: &[ProfileDef]) -> Result<(), Stop> {
    let road = RoadSettings {
        alignments: doc.road_alignments().to_vec(),
        profiles: list.to_vec(),
        ..RoadSettings::default()
    };
    if let Some((path, words)) = road.problem() {
        // The field, whichever profile the rule caught: a name the edited one takes may be found on the one after it.
        let path = path
            .strip_prefix("profiles[")
            .and_then(|rest| rest.split_once("]."))
            .map_or_else(String::new, |(_, field)| field.to_owned());
        return Err(fail(codes::INVALID_ROAD_PROFILE, format!("Boykesit kurallarına uymuyor: {words}."), &path));
    }
    Ok(())
}

/// The PVIs as the core takes them.
pub fn pvis_of(def: &ProfileDef) -> Vec<Pvi> {
    def.pvis
        .iter()
        .map(|p| Pvi {
            id: p.id.clone(),
            station: p.station,
            elevation: p.elevation,
            curve: if p.radius > 0.0 {
                VCurve::Circle { radius: p.radius }
            } else if p.length > 0.0 {
                VCurve::Parabola { length: p.length }
            } else if p.length_in > 0.0 || p.length_out > 0.0 {
                VCurve::Asymmetric {
                    length_in: p.length_in,
                    length_out: p.length_out,
                }
            } else {
                VCurve::None
            },
        })
        .collect()
}

/// A profile as built: its vertical alignment and its alignment.
pub struct Built {
    pub vertical: Vertical,
    pub alignment: AlignmentBuilt,
}

fn vertical_stop(e: VerticalError, def: &ProfileDef) -> Stop {
    let index = |id: &str| def.pvis.iter().position(|p| p.id == id);
    let at = |id: &str| index(id).map_or_else(|| "pvis".to_owned(), |i| format!("pvis[{i}]"));
    match e {
        VerticalError::TooFewPoints => {
            fail(codes::INVALID_ROAD_PROFILE, "Boykesitin en az iki PVI'si olur.".into(), "pvis")
        }
        VerticalError::DuplicateId { pvi } => {
            fail(codes::INVALID_ROAD_PROFILE, format!("“{pvi}” kimliği boykesitte iki kez var."), &at(&pvi))
        }
        VerticalError::NotFinite { pvi } => {
            fail(codes::INVALID_ROAD_PROFILE, format!("{pvi} PVI'sinin sayıları sonlu değil."), &at(&pvi))
        }
        VerticalError::NotIncreasing { pvi } => fail(
            codes::INVALID_ROAD_PROFILE,
            format!("{pvi} PVI'sinin km'si bir öncekinden büyük olmalı."),
            &format!("{}.station", at(&pvi)),
        ),
        VerticalError::BadCurve { pvi } => fail(
            codes::INVALID_ROAD_PROFILE,
            format!(
                "{pvi} PVI'sinde kurp olamaz: başlangıçta ve bitişte kurp yok, eğimi değişmeyen PVI'de kurp yok, boy ve yarıçap 0'dan büyük olur."
            ),
            &at(&pvi),
        ),
        VerticalError::Overlap { a, b, excess } => {
            // The PVI whose curve does not fit: the later one that carries a curve.
            let carries = |id: &str| index(id).is_some_and(|i| pvis_of(def)[i].curve != VCurve::None);
            let who = if carries(&b) { b.clone() } else { a.clone() };
            fail(
                codes::PROFILE_CURVES_OVERLAP,
                format!(
                    "{a}–{b} arasına düşey kurplar sığmıyor: kolları {excess:.3} m uzun. Kurp boyunu ya da yarıçapı küçültün, PVI'ları uzaklaştırın."
                ),
                &at(&who),
            )
        }
    }
}

/// Builds `def` on its alignment; refusals' paths are the input's.
pub(crate) fn build(doc: &Document, def: &ProfileDef) -> Result<Built, Stop> {
    let list = alignments::alignments(doc);
    let adef = list.iter().find(|a| a.id == def.alignment).ok_or_else(|| {
        fail(codes::UNKNOWN_ALIGNMENT, format!("“{}” kimlikli aks projede yok.", def.alignment), "alignment")
    })?;
    let alignment = alignments::build(list, adef, "")?;
    let vertical = core::vertical::build(&pvis_of(def)).map_err(|e| vertical_stop(e, def))?;
    let length = alignment.core.length;
    for (i, p) in def.pvis.iter().enumerate() {
        if p.station < -1e-9 || p.station > length + 1e-9 {
            return Err(fail(
                codes::INVALID_ROAD_PROFILE,
                format!("{} PVI'si aksın dışında: aksın km'si 0 ile {length:.3} m arası.", p.id),
                &format!("pvis[{i}].station"),
            ));
        }
    }
    Ok(Built { vertical, alignment })
}

/// Whether the profile's alignment changed since it was made or rebound, and the alignment's present revision.
pub(crate) fn stale(doc: &Document, def: &ProfileDef) -> (bool, u32) {
    let now = doc.road_alignments().iter().find(|a| a.id == def.alignment).map_or(0, |a| a.revision);
    (now != def.alignment_revision, now)
}

/// The report of `def` as built.
pub(crate) fn report(doc: &Document, def: &ProfileDef, built: &Built) -> ProfileReport {
    let (stale, now) = stale(doc, def);
    let first = def.pvis.first().map_or(0.0, |p| p.station);
    let last = def.pvis.last().map_or(0.0, |p| p.station);
    ProfileReport {
        id: def.id.clone(),
        name: def.name.clone(),
        revision: def.revision,
        alignment: def.alignment.clone(),
        alignment_revision: def.alignment_revision,
        alignment_now: now,
        stale,
        surface: def.surface.clone(),
        stations: [first, last],
        kms: [built.alignment.km(first), built.alignment.km(last)],
        grades: def
            .pvis
            .windows(2)
            .map(|w| ProfileGradeRow {
                between: [w[0].id.clone(), w[1].id.clone()],
                stations: [w[0].station, w[1].station],
                grade: (w[1].elevation - w[0].elevation) / (w[1].station - w[0].station),
            })
            .collect(),
        curves: built
            .vertical
            .curves
            .iter()
            .map(|c| ProfileCurveRow {
                pvi: c.pvi.clone(),
                kind: match c.curve {
                    VCurve::Parabola { .. } | VCurve::None => "parabola",
                    VCurve::Asymmetric { .. } => "asymmetric",
                    VCurve::Circle { .. } => "circle",
                }
                .into(),
                g_in: c.g_in,
                g_out: c.g_out,
                start: c.start,
                end: c.end,
                length: c.end[0] - c.start[0],
                radius: match c.curve {
                    VCurve::Circle { radius } => Some(radius),
                    _ => None,
                },
                a: c.a,
                k: c.k.unwrap_or(0.0),
                curve_radius: c.h,
                crest: c.crest,
                extreme: c.extreme,
            })
            .collect(),
        algorithm: core::ALGORITHM.into(),
    }
}

/// The profile's surface made now, or why none is: the warning `profile_surface_gone`. None for a profile without one.
pub(crate) fn surface(doc: &Document, def: &ProfileDef) -> Option<Result<Surface, CommandWarning>> {
    let id = def.surface.as_deref()?;
    let gone = |why: &str| CommandWarning {
        code: codes::PROFILE_SURFACE_GONE.into(),
        message: format!("“{}” boykesitinin yüzeyi {why}; siyah kot verilmedi.", def.name),
        path: Some("profile".into()),
    };
    let Some(sdef) = terrains::surfaces(doc).iter().find(|s| s.id == id) else {
        return Some(Err(gone(&format!("({id}) projede yok"))));
    };
    Some(terrains::make(doc, sdef).surface.map_err(|_| gone(&format!("“{}” kurulamıyor", sdef.name))))
}

/// The elevations at a station on the alignment; none off it.
pub(crate) fn at(built: &Built, surface: Option<&Surface>, station: f64) -> Option<ProfileAt> {
    let place = built.alignment.core.at(station)?;
    let km = built.alignment.km(station);
    let red = built.vertical.at(station);
    let black = surface.and_then(|s| s.height(place.x, place.y).z);
    Some(ProfileAt {
        station,
        km,
        km_text: km_text(km, 3),
        red: red.map(|r| r.0),
        grade: red.map(|r| r.1),
        black,
        difference: red.zip(black).map(|(r, b)| r.0 - b),
    })
}

pub(crate) fn listed(doc: &Document, list: &[ProfileDef]) -> Vec<RoadProfileListed> {
    list.iter()
        .map(|p| RoadProfileListed {
            id: p.id.clone(),
            name: p.name.clone(),
            alignment: p.alignment.clone(),
            revision: p.revision,
            stale: stale(doc, p).0,
        })
        .collect()
}

/// The road project's profiles as they build now, for the desktop's Boykesit window (plan/road.md Y4): each
/// definition and its profile, or why it does not build. None outside a road project.
pub fn made_profiles(doc: &Document) -> Vec<(ProfileDef, Result<Built, String>)> {
    if road_project(doc).is_err() {
        return Vec::new();
    }
    profiles(doc)
        .iter()
        .map(|d| {
            let built = build(doc, d).map_err(|stop| match stop {
                Stop::Failed(e) | Stop::Conflict(e) => e.message,
                Stop::Cancelled => String::new(),
            });
            (d.clone(), built)
        })
        .collect()
}

/// The report of a built profile, as `road.profiles.query` gives it.
pub fn report_of(doc: &Document, def: &ProfileDef, built: &Built) -> ProfileReport {
    report(doc, def, built)
}

/// Writes `next` as one undo step named `label`; the answer reports `target` (none: removed).
pub(crate) fn write(
    cx: &mut ExecutionContext<'_>,
    next: Vec<ProfileDef>,
    label: &str,
    target: Option<&str>,
) -> CommandResult<ProfilesWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_profiles(next.clone(), label) {
        return fail(codes::INVALID_ROAD_PROFILE, format!("Boykesit kurallarına uymuyor: {}.", r.0), "").into();
    }
    let profile = match target.and_then(|id| next.iter().find(|p| p.id == id)) {
        Some(def) => match build(doc, def) {
            Ok(b) => Some(report(doc, def, &b)),
            Err(stop) => return stop.into(),
        },
        None => None,
    };
    CommandResult::Completed {
        output: ProfilesWritten {
            profile,
            profiles: listed(doc, &next),
            revision: doc.revision().to_string(),
        },
        warnings: vec![],
    }
}

/// A profile's crossings for the core (docs/adr/0946).
pub(crate) fn crossings_of(def: &kentos_commands::ProfileDef) -> Vec<kentos_road::profile::crossings::Crossing> {
    use kentos_road::profile::crossings::{Crossing, CrossingKind};
    def.crossings
        .iter()
        .map(|c| Crossing {
            station: c.station,
            kind: match c.side {
                kentos_commands::CrossingSide::Above => CrossingKind::Above,
                kentos_commands::CrossingSide::Below => CrossingKind::Below,
            },
            elevation: c.elevation,
            clearance: c.clearance,
        })
        .collect()
}

/// A profile's crossings against a grade line (docs/adr/0946), with whether each holds it (a proposal's flags).
pub(crate) fn crossing_margins(
    def: &kentos_commands::ProfileDef,
    line: &kentos_road::profile::vertical::Vertical,
    holds: &[i8],
) -> Vec<kentos_commands::CrossingMargin> {
    let cores = crossings_of(def);
    kentos_road::profile::crossings::margins(line, &cores)
        .into_iter()
        .zip(&def.crossings)
        .enumerate()
        .map(|(k, (m, c))| kentos_commands::CrossingMargin {
            name: c.name.clone(),
            station: c.station,
            side: c.side,
            limit: m.limit,
            elevation: m.z,
            margin: m.margin,
            holds: holds.get(k).is_some_and(|h| *h != 0),
        })
        .collect()
}
