//! `road.volumes.progress` v1 (plan/road.md Y36, docs/adr/0942): an earthwork's progress. The design (the volume's
//! corridor's top surface), the original ground (the corridor's surface) and a survey of the site now are overlaid
//! exactly (`kentos_road::volume::progress`): the cut's and fill's work done, left and over; with a previous survey, its
//! work too and the period's (now less before). Reads only. The shared cases are
//! `fixtures/commands/v1/road.volumes.progress.json`.

use kentos_commands::{
    CommandResult, CommandWarning, ProgressAmounts, StretchProgress, TerrainSurfaceDef, VolumeProgressReport, VolumesProgress,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::surface::Surface;
use kentos_road::volume::{Progress, Triangle, between_surfaces, progress};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_corridors as corridors;
use crate::road_profiles::surface_id;
use crate::road_terrains::{self as terrains, fail};
use crate::road_volumes as common;

/// Less than this done in a period (m³) is going back, not the arithmetic's noise.
const BACKWARD: f64 = -1e-6;

fn bad(path: &str, words: String) -> Stop {
    fail(codes::INVALID_VOLUME_PROGRESS, words, path)
}

fn amounts(p: &Progress) -> ProgressAmounts {
    ProgressAmounts {
        cut_done: p.cut_done,
        cut_left: p.cut_left,
        cut_over: p.cut_over,
        fill_done: p.fill_done,
        fill_left: p.fill_left,
        fill_over: p.fill_over,
    }
}

fn less(a: &ProgressAmounts, b: &ProgressAmounts) -> ProgressAmounts {
    ProgressAmounts {
        cut_done: a.cut_done - b.cut_done,
        cut_left: a.cut_left - b.cut_left,
        cut_over: a.cut_over - b.cut_over,
        fill_done: a.fill_done - b.fill_done,
        fill_left: a.fill_left - b.fill_left,
        fill_over: a.fill_over - b.fill_over,
    }
}

fn share(done: f64, left: f64) -> Option<f64> {
    (done + left > 1e-9).then(|| 100.0 * done / (done + left))
}

fn triangles(s: &Surface) -> Vec<Triangle> {
    s.triangles.iter().map(|t| t.map(|i| s.vertices[i as usize])).collect()
}

/// A quantity as the warnings write it: one decimal, a comma.
fn decimal(v: f64) -> String {
    format!("{v:.1}").replace('.', ",")
}

/// The survey of `name` (field `path`): its definition.
fn survey<'a>(doc: &'a Document, name: &str, path: &str) -> Result<&'a TerrainSurfaceDef, Stop> {
    let id = surface_id(doc, name, path)?;
    terrains::surfaces(doc)
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| fail(codes::UNKNOWN_TERRAIN, format!("“{name}” kimlikli ya da adlı yüzey projede yok."), path))
}

/// The progress now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &VolumesProgress) -> Result<(VolumeProgressReport, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_volumes(), &input.volume, "volume")?;
    report(doc, def, input)
}

/// The progress of a volume as typed (the desktop's Kübaj window, before it is saved).
pub fn preview(
    doc: &Document,
    def: &kentos_commands::VolumeDef,
    input: &VolumesProgress,
) -> Result<(VolumeProgressReport, Vec<CommandWarning>), String> {
    let words = |stop: Stop| match stop {
        Stop::Failed(e) | Stop::Conflict(e) => e.message,
        Stop::Cancelled => String::new(),
    };
    common::road_project(doc).map_err(words)?;
    report(doc, def, input).map_err(words)
}

fn report(
    doc: &Document,
    def: &kentos_commands::VolumeDef,
    input: &VolumesProgress,
) -> Result<(VolumeProgressReport, Vec<CommandWarning>), Stop> {
    let cdef = doc.road_corridors().iter().find(|c| c.id == def.corridor).ok_or_else(|| {
        fail(codes::UNKNOWN_CORRIDOR, format!("“{}” kimlikli koridor projede yok.", def.corridor), "corridor")
    })?;
    let now_def = survey(doc, &input.survey, "survey")?;
    let before_def = input.previous.as_deref().map(|p| survey(doc, p, "previous")).transpose()?;
    if now_def.id == cdef.surface {
        return Err(bad(
            "survey",
            format!("“{}” yüzeyi “{}” koridorunun ilk arazisi; ara ölçü başka bir yüzey olmalı.", now_def.name, cdef.name),
        ));
    }
    if before_def.is_some_and(|b| b.id == now_def.id) {
        return Err(bad("previous", format!("Önceki ölçü ara ölçünün kendisi (“{}”); daha önceki bir yüzey seçin.", now_def.name)));
    }
    let made = corridors::make(doc, cdef, false).map_err(|stop| {
        fail(codes::INVALID_VOLUME, format!("“{}” koridoru kurulamıyor: {}", cdef.name, corridors::words(stop)), "corridor")
    })?;
    let Some(original) = &made.sampler.surface else {
        return Err(fail(
            codes::INVALID_VOLUME,
            format!("“{}” koridorunun yüzeyi kurulamıyor; ilk arazi okunamaz.", cdef.name),
            "corridor",
        ));
    };
    let built = |sd: &TerrainSurfaceDef, path: &str| match terrains::make(doc, sd).surface {
        Ok(s) => Ok(triangles(&s)),
        Err(_) => Err(bad(path, format!("“{}” yüzeyi kaynaklarından kurulamıyor.", sd.name))),
    };
    let now_tris = built(now_def, "survey")?;
    let before_tris = before_def.map(|b| built(b, "previous")).transpose()?;
    let original_tris = triangles(original);
    let design = &made.built.triangles;
    let base = between_surfaces(design, &original_tris).uncovered;
    let now = progress(design, &original_tris, &now_tris);
    let before = before_tris.as_ref().map(|b| progress(design, &original_tris, b));
    let mut warnings = Vec::new();
    let tolerance = 1e-6 * (now.covered + now.uncovered).max(1.0);
    for (sd, p, path) in [(Some(now_def), Some(&now), "survey"), (before_def, before.as_ref(), "previous")] {
        if let (Some(sd), Some(p)) = (sd, p)
            && p.uncovered > base + tolerance
        {
            warnings.push(CommandWarning {
                code: codes::VOLUME_PROGRESS_UNCOVERED.into(),
                message: format!(
                    "“{}” yüzeyi tasarımın {} m²'sini örtmüyor; orada yapılan iş sayılmadı.",
                    sd.name,
                    decimal(p.uncovered - base)
                ),
                path: Some(path.into()),
            });
        }
    }
    let (now_amounts, before_amounts) = (amounts(&now), before.as_ref().map(amounts));
    let period = before_amounts.as_ref().map(|b| less(&now_amounts, b));
    if let Some(p) = &period {
        let back: Vec<String> = [("kazı", p.cut_done), ("dolgu", p.fill_done)]
            .into_iter()
            .filter(|(_, v)| *v < BACKWARD)
            .map(|(w, v)| format!("{w} {} m³", decimal(-v)))
            .collect();
        if !back.is_empty() {
            warnings.push(CommandWarning {
                code: codes::VOLUME_PROGRESS_BACKWARD.into(),
                message: format!(
                    "“{}” kübajında önceki ölçüden bu yana yapılan iş geri gitmiş ({}): ölçü hatası ya da geri doldurma.",
                    def.name,
                    back.join(", ")
                ),
                path: Some("previous".into()),
            });
        }
    }
    let text = |s: f64| km_text(made.sampler.alignment.km(s), 3);
    let stretches = made
        .built
        .strips
        .iter()
        .map(|(_, from, to, range)| {
            let p = progress(&design[range.clone()], &original_tris, &now_tris);
            StretchProgress { from: *from, to: *to, from_text: text(*from), to_text: text(*to), amounts: amounts(&p), uncovered: p.uncovered }
        })
        .collect();
    let digest = |sd: &TerrainSurfaceDef| terrains::sources_digest(doc, sd);
    // The corridor's surface built above, so it is in the project.
    let original_digest = terrains::surfaces(doc).iter().find(|s| s.id == cdef.surface).map(digest).unwrap_or_default();
    Ok((
        VolumeProgressReport {
            volume: def.id.clone(),
            original: cdef.surface.clone(),
            original_digest,
            survey: now_def.id.clone(),
            survey_digest: digest(now_def),
            previous: before_def.map(|b| b.id.clone()),
            previous_digest: before_def.map(digest),
            cut_share: share(now.cut_done, now.cut_left),
            fill_share: share(now.fill_done, now.fill_left),
            now: now_amounts,
            before: before_amounts,
            period,
            covered: now.covered,
            uncovered: now.uncovered,
            stretches,
        },
        warnings,
    ))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &VolumesProgress) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &VolumesProgress) -> CommandResult<VolumeProgressReport> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: VolumesProgress) -> CommandResult<VolumeProgressReport> {
    plan(cx, &input)
}
