//! What KentOS Yol's superelevation commands share (plan/road.md Y6, docs/adr/0904): the project's schedules, one
//! built by `kentos-road` from its curves or rows on its alignment, its report, and the refusals its geometry gives.
//! The commands are [`crate::road_superelevations_edit`] and [`crate::road_superelevations_query`]; a sample line set
//! reads its schedule in [`crate::road_sections`].

use kentos_commands::{
    CommandWarning, SuperCritical, SuperelevationDef, SuperelevationMethod, SuperelevationReport,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::superelevation::{self as sup, Crossfall, CurveRule, Mark, Row, Schedule, SuperError};

use crate::checks::Stop;
use crate::codes;
use crate::road_alignments::{self as alignments, Built};
use crate::road_terrains::fail;

/// Only a road project has superelevation.
/// A grade written as percent for a message: 0.08 is “8”, rounded to nine places so the conversion's last bit is not
/// shown (docs/adr/0977: the sources hold grades, the words show percent).
pub(crate) fn percent(grade: f64) -> String {
    let p = (grade * 100.0 * 1e9).round() / 1e9;
    format!("{}", if p == 0.0 { 0.0 } else { p })
}

pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work == Some(kentos_commands::CadWork::Road) {
        Ok(())
    } else {
        Err(fail(
            codes::NOT_A_ROAD_PROJECT,
            "Dever yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(),
            "",
        ))
    }
}

pub(crate) fn find<'a>(list: &'a [SuperelevationDef], name: &str, path: &str) -> Result<(usize, &'a SuperelevationDef), Stop> {
    list.iter()
        .position(|t| t.id == name)
        .or_else(|| list.iter().position(|t| t.name.to_lowercase() == name.to_lowercase()))
        .map(|i| (i, &list[i]))
        .ok_or_else(|| fail(codes::UNKNOWN_SUPERELEVATION, format!("“{name}” kimlikli ya da adlı dever çizelgesi projede yok."), path))
}

pub(crate) fn normal(def: &SuperelevationDef) -> Crossfall {
    Crossfall { left: def.normal_left, right: def.normal_right }
}

/// The schedule's alignment built.
pub(crate) fn alignment_of(doc: &Document, def: &SuperelevationDef) -> Result<Built, Stop> {
    let list = alignments::alignments(doc);
    let a = list.iter().find(|a| a.id == def.alignment).ok_or_else(|| {
        fail(codes::UNKNOWN_ALIGNMENT, format!("“{}” kimlikli aks projede yok.", def.alignment), "alignment")
    })?;
    alignments::build(list, a, "")
}

/// The schedule built on its alignment, and its critical stations; why not, at the curve's or row's path.
pub(crate) fn schedule(def: &SuperelevationDef, built: &Built) -> Result<(Schedule, Vec<sup::Critical>), Stop> {
    schedule_slopes(def, built).map(|(mut s, critical)| {
        s.pivot = pivot_of(def);
        (s, critical)
    })
}

/// The core's axis of rotation of a definition (docs/adr/0931 §1).
pub fn pivot_of(def: &SuperelevationDef) -> kentos_road::superelevation_physics::Pivot {
    use kentos_commands::SuperelevationPivot as P;
    use kentos_road::superelevation_physics::Pivot;
    match def.pivot {
        None | Some(P::Centerline) => Pivot::Centerline,
        Some(P::LeftEdge) => Pivot::LeftEdge,
        Some(P::RightEdge) => Pivot::RightEdge,
        Some(P::InsideEdge) => Pivot::InsideEdge,
        Some(P::OutsideEdge) => Pivot::OutsideEdge,
    }
}

fn schedule_slopes(def: &SuperelevationDef, built: &Built) -> Result<(Schedule, Vec<sup::Critical>), Stop> {
    let bad = |path: String, words: String| fail(codes::INVALID_SUPERELEVATION, words, &path);
    match &def.method {
        SuperelevationMethod::Stations { rows } => {
            let length = built.core.length;
            for (i, r) in rows.iter().enumerate() {
                if r.station < -1e-9 || r.station > length + 1e-9 {
                    return Err(bad(
                        format!("method.rows[{i}].station"),
                        format!("{}. satır aksın dışında: aksın km'si 0 ile {length:.3} m arası.", i + 1),
                    ));
                }
            }
            let rows: Vec<Row> = rows.iter().map(|r| Row { station: r.station, left: r.left, right: r.right }).collect();
            sup::from_rows(&rows).map(|s| (s, Vec::new())).map_err(|e| match e {
                SuperError::BadRow { row } => bad(
                    format!("method.rows[{row}].station"),
                    format!("{}. satırın km'si bir öncekinden büyük olmalı.", row + 1),
                ),
                other => bad("method".into(), format!("{other:?}")),
            })
        }
        SuperelevationMethod::Curves { curves } => {
            let rules: Vec<CurveRule> = curves
                .iter()
                .map(|c| CurveRule {
                    pi: c.pi.clone(),
                    rate: c.rate,
                    runoff: c.runoff,
                    on_tangent: c.on_tangent,
                    widening: c.widening,
                })
                .collect();
            let at = |pi: &str| curves.iter().position(|c| c.pi == pi).unwrap_or(0);
            sup::from_curves(&built.core, normal(def), &rules).map_err(|e| match e {
                SuperError::NoCurve { pi } => {
                    bad(format!("method.curves[{}].pi", at(&pi)), format!("“{pi}” PI'sinde kurp yok."))
                }
                SuperError::Bad { pi } => bad(
                    format!("method.curves[{}]", at(&pi)),
                    format!("“{pi}” kurbunda oran ve geçiş boyu 0'dan büyük, teğet payı 0 ile 1 arası, genişleme eksi olmamalı."),
                ),
                SuperError::RateBelowNormal { pi } => bad(
                    format!("method.curves[{}].rate", at(&pi)),
                    format!("“{pi}” kurbunun dever oranı normal enine eğimden küçük."),
                ),
                SuperError::TooShort { pi } => bad(
                    format!("method.curves[{}].runoff", at(&pi)),
                    format!("“{pi}” kurbunda tam dever kurba sığmıyor: geçiş boyunu kısaltın ya da teğet payını büyütün."),
                ),
                SuperError::Overlap { a, b } => bad(
                    format!("method.curves[{}]", at(&b)),
                    format!("“{a}” ile “{b}” kurplarının geçişleri biniyor: geçiş boylarını kısaltın ya da km satırlarıyla yazın."),
                ),
                SuperError::OffAlignment { pi } => bad(
                    format!("method.curves[{}]", at(&pi)),
                    format!("“{pi}” kurbunun geçişi aksın dışına taşıyor."),
                ),
                SuperError::Twice { pi } => {
                    bad(format!("method.curves[{}]", at(&pi)), format!("“{pi}” PI'si iki kez yazılmış."))
                }
                SuperError::BadRow { row } => bad(format!("method.rows[{row}]"), "Satırlar artan km'de olmalı.".into()),
            })
        }
    }
}

/// `def` on its alignment turned round (docs/adr/0964; ROAD-DEVER-03): the same physical road, read from the other end.
/// The alignment of length `length` now runs the other way, so a station s becomes `length` − s and the left side is
/// the old right. Every cross slope is written outward on its own side (ADR 0904), so a side's value moves to the other
/// column unchanged, never negated:
///
/// - the normal slopes swap;
/// - rows written by station go to `length` − s, in the reversed order, their left and right swapped;
/// - a rule written on a curve follows its PI: the curve turns the other way but its outside is the same edge, and a
///   rule is the same at the curve's two ends, so it stays as written;
/// - a pivot on the left edge is now on the right edge and back; the centreline, the inside and the outside stay.
///
/// It is bound to the alignment's new revision `revision`.
pub(crate) fn reversed(def: &SuperelevationDef, length: f64, revision: u32) -> SuperelevationDef {
    use kentos_commands::{SuperelevationMethod, SuperelevationPivot as P};
    let method = match &def.method {
        SuperelevationMethod::Stations { rows } => SuperelevationMethod::Stations {
            rows: rows
                .iter()
                .rev()
                .map(|r| kentos_commands::SuperRowDef { station: length - r.station, left: r.right, right: r.left })
                .collect(),
        },
        m @ SuperelevationMethod::Curves { .. } => m.clone(),
    };
    SuperelevationDef {
        normal_left: def.normal_right,
        normal_right: def.normal_left,
        method,
        pivot: def.pivot.map(|p| match p {
            P::LeftEdge => P::RightEdge,
            P::RightEdge => P::LeftEdge,
            other => other,
        }),
        alignment_revision: revision,
        revision: def.revision.saturating_add(1),
        ..def.clone()
    }
}

pub(crate) fn stale(doc: &Document, def: &SuperelevationDef) -> bool {
    doc.road_alignments().iter().find(|a| a.id == def.alignment).is_some_and(|a| a.revision != def.alignment_revision)
}

pub(crate) fn stale_warning(doc: &Document, def: &SuperelevationDef) -> Option<CommandWarning> {
    let now = doc.road_alignments().iter().find(|a| a.id == def.alignment).map_or(0, |a| a.revision);
    stale(doc, def).then(|| CommandWarning {
        code: codes::SUPERELEVATION_STALE.into(),
        message: format!(
            "“{}” dever çizelgesinin aksı değişti (çizelge {}. revizyona göre kurulu, aks şimdi {now}.); km'leri denetleyip aksa yeniden bağlayın.",
            def.name, def.alignment_revision
        ),
        path: Some("superelevation".into()),
    })
}

fn mark(m: Mark) -> &'static str {
    match m {
        Mark::NormalCrown => "normalCrown",
        Mark::LevelCrown => "levelCrown",
        Mark::ReverseCrown => "reverseCrown",
        Mark::Full => "full",
    }
}

/// The schedule's report: built, or why not.
pub(crate) fn report(doc: &Document, def: &SuperelevationDef) -> SuperelevationReport {
    let mut out = SuperelevationReport {
        id: def.id.clone(),
        name: def.name.clone(),
        revision: def.revision,
        alignment: def.alignment.clone(),
        stale: stale(doc, def),
        normal_left: def.normal_left,
        normal_right: def.normal_right,
        left: Vec::new(),
        right: Vec::new(),
        left_widening: Vec::new(),
        right_widening: Vec::new(),
        critical: Vec::new(),
        problem: None,
    };
    let built = alignment_of(doc, def).and_then(|b| schedule(def, &b).map(|s| (b, s)));
    match built {
        Ok((b, (s, critical))) => {
            out.left = s.left;
            out.right = s.right;
            out.left_widening = s.left_widening;
            out.right_widening = s.right_widening;
            out.critical = critical
                .into_iter()
                .map(|c| {
                    let km = b.km(c.station);
                    SuperCritical { pi: c.pi, mark: mark(c.mark).into(), station: c.station, km, km_text: km_text(km, 3), exit: c.exit }
                })
                .collect();
        }
        Err(Stop::Failed(e) | Stop::Conflict(e)) => out.problem = Some(e.message),
        Err(Stop::Cancelled) => {}
    }
    out
}

/// The schedule a sample line set reads, built; why not as a warning.
pub(crate) fn schedule_of(doc: &Document, id: &str, set: &str) -> Result<(Schedule, Crossfall), CommandWarning> {
    let gone = |why: String| CommandWarning {
        code: codes::SECTION_SUPERELEVATION_GONE.into(),
        message: format!("“{set}” kesit takımının dever çizelgesi {why}; şeritler tip kesitin eğimiyle verildi."),
        path: Some("set".into()),
    };
    let Some(def) = doc.road_superelevations().iter().find(|d| d.id == id) else {
        return Err(gone(format!("({id}) projede yok")));
    };
    let words = |stop: Stop| match stop {
        Stop::Failed(e) | Stop::Conflict(e) => e.message,
        Stop::Cancelled => String::new(),
    };
    let why = |s: Stop| gone(format!("kurulamıyor ({})", words(s).trim_end_matches('.')));
    let built = alignment_of(doc, def).map_err(why)?;
    let (s, _) = schedule(def, &built).map_err(why)?;
    Ok((s, normal(def)))
}

/// The road project's superelevation schedules and their reports now, for the desktop's Dever window (plan/road.md Y6).
pub fn made_superelevations(doc: &Document) -> Vec<(SuperelevationDef, SuperelevationReport)> {
    if road_project(doc).is_err() {
        return Vec::new();
    }
    doc.road_superelevations().iter().map(|d| (d.clone(), report(doc, d))).collect()
}

/// A schedule (perhaps not written) built now, for the desktop's Dever window: its report.
pub fn preview(doc: &Document, def: &SuperelevationDef) -> SuperelevationReport {
    report(doc, def)
}
