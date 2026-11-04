//! `road.volumes.compare` v1 (plan/road.md Y37, docs/adr/0943): two to six earthwork volumes side by side, the first the
//! base. Each alternative names its corridor, alignment, profile and surface with their revisions and the surface's
//! sources' digest, its alignment's length, its volumes and its approval's changes; against the base, the changes and
//! the stretches its alignment departs in plan and its profile in height (`kentos_road::alternatives`). Reads only. The
//! shared cases are `fixtures/commands/v1/road.volumes.compare.json`.

use kentos_commands::{
    AgainstBase, AlternativeApproval, AlternativeFigures, AlternativeReport, CommandResult, CommandWarning, DepartureRow,
    VolumeDef, VolumesCompare, VolumesCompared,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::alternatives::{self as alt, Departure};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_corridors as corridors;
use crate::road_terrains::{self as terrains, fail};
use crate::road_volumes as common;

/// The tolerances when none is given (m): two lines within a centimetre are one line.
pub const TOLERANCE: f64 = 0.01;
pub const HEIGHT: f64 = 0.01;

fn bad(path: &str, words: &str) -> Stop {
    fail(codes::INVALID_VOLUME_COMPARE, words.into(), path)
}

/// What an alternative is built of: its report row, and its alignment and profile when its corridor builds.
struct Built {
    report: AlternativeReport,
    made: Option<corridors::Made>,
}

fn of(doc: &Document, def: &VolumeDef, warnings: &mut Vec<CommandWarning>) -> Built {
    let cdef = doc.road_corridors().iter().find(|c| c.id == def.corridor);
    let alignment = cdef.and_then(|c| doc.road_alignments().iter().find(|a| a.id == c.alignment));
    let profile = cdef.and_then(|c| doc.road_profiles().iter().find(|p| p.id == c.profile));
    let surface = cdef.and_then(|c| terrains::surfaces(doc).iter().find(|s| s.id == c.surface));
    let made = cdef.and_then(|c| corridors::make(doc, c, false).ok());
    let computed = common::compute(doc, def);
    let (figures, approved, problem) = match &computed {
        Ok((r, _)) => (
            Some(AlternativeFigures { cut: r.cut, fill: r.fill, factored_cut: r.factored_cut, factored_fill: r.factored_fill }),
            r.approval.as_ref().map(|a| AlternativeApproval {
                at: a.approved.at.clone(),
                changed: a.changed,
                cut_change: a.cut_change,
                fill_change: a.fill_change,
            }),
            r.problem.clone(),
        ),
        Err(Stop::Failed(e) | Stop::Conflict(e)) => (None, None, Some(e.message.clone())),
        Err(Stop::Cancelled) => (None, None, Some(String::new())),
    };
    if let Some(why) = &problem {
        warnings.push(CommandWarning {
            code: codes::VOLUME_COMPARE_BROKEN.into(),
            message: format!("“{}” kübajı hesaplanamıyor; karşılaştırmada hacmi yok: {why}", def.name),
            path: Some("volumes".into()),
        });
    }
    Built {
        report: AlternativeReport {
            volume: def.id.clone(),
            name: def.name.clone(),
            corridor: def.corridor.clone(),
            alignment: cdef.map(|c| c.alignment.clone()).unwrap_or_default(),
            alignment_revision: alignment.map_or(0, |a| a.revision),
            profile: cdef.map(|c| c.profile.clone()).unwrap_or_default(),
            profile_revision: profile.map_or(0, |p| p.revision),
            surface: cdef.map(|c| c.surface.clone()).unwrap_or_default(),
            surface_digest: surface.map(|s| terrains::sources_digest(doc, s)).unwrap_or_default(),
            length: made.as_ref().map(|m| m.sampler.alignment.core.length),
            figures: figures.filter(|_| problem.is_none()),
            problem,
            approved,
            against: None,
        },
        made,
    }
}

fn rows(list: Vec<Departure>, other: &corridors::Made, base: &corridors::Made) -> Vec<DepartureRow> {
    let text = |m: &corridors::Made, s: f64| km_text(m.sampler.alignment.km(s), 3);
    list.into_iter()
        .map(|d| DepartureRow {
            from: d.from,
            to: d.to,
            from_text: text(other, d.from),
            to_text: text(other, d.to),
            base_from_text: d.base_from.map(|s| text(base, s)),
            base_to_text: d.base_to.map(|s| text(base, s)),
            largest: d.largest,
        })
        .collect()
}

/// The comparison now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &VolumesCompare) -> Result<(VolumesCompared, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    if !(2..=6).contains(&input.volumes.len()) {
        return Err(bad("volumes", "İkiden altıya kadar kübaj karşılaştırılır; ilki taban."));
    }
    let mut defs: Vec<&VolumeDef> = Vec::new();
    for (i, name) in input.volumes.iter().enumerate() {
        let (_, def) = common::find(doc.road_volumes(), name, &format!("volumes[{i}]"))?;
        if defs.iter().any(|d| d.id == def.id) {
            return Err(bad(&format!("volumes[{i}]"), &format!("“{}” kübajı iki kez seçilmiş.", def.name)));
        }
        defs.push(def);
    }
    let fine = |v: Option<f64>| v.is_none_or(|t| t.is_finite() && t > 0.0 && t <= 10.0);
    if !fine(input.tolerance) {
        return Err(bad("tolerance", "Plan toleransı 0'dan büyük, en çok 10 m olmalı."));
    }
    if !fine(input.height) {
        return Err(bad("height", "Kot toleransı 0'dan büyük, en çok 10 m olmalı."));
    }
    let (tolerance, h) = (input.tolerance.unwrap_or(TOLERANCE), input.height.unwrap_or(HEIGHT));
    let mut warnings = Vec::new();
    let built: Vec<Built> = defs.iter().map(|d| of(doc, d, &mut warnings)).collect();
    let base = &built[0];
    let mut alternatives = vec![base.report.clone()];
    for b in &built[1..] {
        let mut report = b.report.clone();
        let (plan_rows, height_rows) = match (&b.made, &base.made) {
            (Some(o), Some(m)) => (
                rows(alt::plan(&m.sampler.alignment.core, &o.sampler.alignment.core, tolerance), o, m),
                rows(alt::height(&m.sampler.alignment.core, &m.sampler.vertical, &o.sampler.alignment.core, &o.sampler.vertical, tolerance, h), o, m),
            ),
            _ => (Vec::new(), Vec::new()),
        };
        let figures_change = match (&b.report.figures, &base.report.figures) {
            (Some(o), Some(m)) => Some(AlternativeFigures {
                cut: o.cut - m.cut,
                fill: o.fill - m.fill,
                factored_cut: o.factored_cut - m.factored_cut,
                factored_fill: o.factored_fill - m.factored_fill,
            }),
            _ => None,
        };
        report.against = Some(AgainstBase {
            length_change: b.report.length.zip(base.report.length).map(|(o, m)| o - m),
            figures_change,
            plan: plan_rows,
            height: height_rows,
        });
        alternatives.push(report);
    }
    Ok((VolumesCompared { tolerance, height: h, alternatives }, warnings))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &VolumesCompare) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &VolumesCompare) -> CommandResult<VolumesCompared> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: VolumesCompare) -> CommandResult<VolumesCompared> {
    plan(cx, &input)
}

/// The comparison for the desktop's Kübaj window: the reports, or why there is none.
pub fn preview(doc: &Document, input: &VolumesCompare) -> Result<(VolumesCompared, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(|stop| match stop {
        Stop::Failed(e) | Stop::Conflict(e) => e.message,
        Stop::Cancelled => String::new(),
    })
}

/// An alternative's departures along its stations for the desktop's diagram: (station, distance from the base in
/// plan, m) and (station, elevation difference where the two lie within the tolerance, m; none elsewhere).
#[derive(Clone, Debug, PartialEq)]
pub struct Trace {
    pub name: String,
    pub plan: Vec<(f64, f64)>,
    pub height: Vec<(f64, Option<f64>)>,
}

/// The alternatives' traces against the base (the first), sampled every 2 m at least 300 times; empty where an
/// alignment does not build.
pub fn traces(doc: &Document, input: &VolumesCompare) -> Vec<Trace> {
    let tolerance = input.tolerance.unwrap_or(TOLERANCE);
    let mut warnings = Vec::new();
    let built: Vec<Built> = input
        .volumes
        .iter()
        .filter_map(|n| common::find(doc.road_volumes(), n, "").ok().map(|(_, d)| d))
        .map(|d| of(doc, d, &mut warnings))
        .collect();
    let Some(Some(base)) = built.first().map(|b| b.made.as_ref()) else { return Vec::new() };
    built[1..]
        .iter()
        .filter_map(|b| {
            let o = b.made.as_ref()?;
            let length = o.sampler.alignment.core.length;
            let n = ((length / 2.0).ceil() as usize).max(300);
            let mut plan_pts = Vec::with_capacity(n + 1);
            let mut height_pts = Vec::with_capacity(n + 1);
            for i in 0..=n {
                let s = length * i as f64 / n as f64;
                let Some(l) = o.sampler.alignment.core.at(s) else { continue };
                let (d, station) = alt::distance(&base.sampler.alignment.core, [l.x, l.y]);
                plan_pts.push((s, d));
                let dz = (d <= tolerance)
                    .then_some(station)
                    .flatten()
                    .and_then(|sb| Some(o.sampler.vertical.at(s)?.0 - base.sampler.vertical.at(sb)?.0));
                height_pts.push((s, dz));
            }
            Some(Trace { name: b.report.name.clone(), plan: plan_pts, height: height_pts })
        })
        .collect()
}
