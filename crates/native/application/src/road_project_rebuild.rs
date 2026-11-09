//! `road.project.rebuild` v1 and drawing again by itself (plan/road.md Y13.2, docs/adr/0911 §3; ROAD-MOD-06): a road
//! drawing drawn again with the input it was drawn with, in one step: its objects removed, its command's check and
//! write run (they refuse what no longer draws: a layer gone or locked, a source that does not build), the drawing kept
//! anew with its sources' stamps and objects. The shared cases are `fixtures/commands/v1/road.project.rebuild.json`.

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, CommandWarning, RoadDrawingDef, RoadDrawingLeft, RoadDrawingRebuilt, RoadNodeState as S,
    RoadProjectRebuild, RoadProjectRebuilt,
};
use kentos_domain::Uuid;
use serde_json::Value;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::road_drawings;
use crate::trim::relay;

pub(crate) type Refused = Box<CommandResult<()>>;

/// Runs drawing command `command` on `input` (a drawing's or, docs/adr/3240, a recipe's; `id` names it in a refusal):
/// its check, the objects `remove` removed (a drawing drawn again: its own; a new one: none), then its write in the step
/// it runs in. Answers the objects written and the command's warnings, or its refusal (nothing written: the check runs
/// before the objects go). A refusal says whether it came after the check, when the step may hold a part of the write.
pub(crate) fn run_drawing(
    cx: &mut ExecutionContext<'_>,
    id: &str,
    command: &str,
    input: &BTreeMap<String, Value>,
    remove: &[kentos_domain::Slot],
) -> Result<(Vec<String>, Vec<CommandWarning>), (Refused, bool)> {
    let value = Value::Object(input.clone().into_iter().collect());
    let bad = |e: serde_json::Error| -> (Refused, bool) {
        (Box::new(Stop::failed(error(codes::UNKNOWN_DRAWING, format!("“{id}” çiziminin girdisi okunamadı: {e}."), None)).into()), false)
    };
    macro_rules! again {
        ($m:ident, $t:ty) => {{
            let input: $t = serde_json::from_value(value).map_err(bad)?;
            match crate::$m::validate(cx, &input) {
                CommandResult::Completed { .. } => {}
                other => return Err((Box::new(relay(other)), false)),
            }
            if !remove.is_empty() {
                cx.doc.remove(remove);
            }
            match crate::$m::draw(cx, &input) {
                CommandResult::Completed { output, warnings } => (output.uids, warnings),
                other => return Err((Box::new(relay(other)), true)),
            }
        }};
    }
    use kentos_commands as k;
    Ok(match command {
        "road.alignments.label" => again!(road_alignments_label, k::AlignmentsLabel),
        "road.profiles.draw" => again!(road_profiles_draw, k::ProfilesDraw),
        "road.sections.draw" => again!(road_sections_draw, k::SectionsDraw),
        "road.corridors.draw" => again!(road_corridors_draw, k::CorridorsDraw),
        "road.guardrails.draw" => again!(road_guardrails_draw, k::GuardrailsDraw),
        "road.culverts.draw" => again!(road_culverts_draw, k::CulvertsDraw),
        "road.walls.draw" => again!(road_walls_draw, k::RetainingWallsDraw),
        "road.junctions.draw" => again!(road_junctions_draw, k::JunctionsDraw),
        "road.platforms.draw" => again!(road_platforms_draw, k::PlatformsDraw),
        "road.terrains.zeroline" => again!(road_terrains_zeroline, k::TerrainsZeroLine),
        "road.terrains.cutfill" => again!(road_terrains_cutfill, k::TerrainsCutFill),
        _ => again!(road_terrains_contours, k::TerrainsContours),
    })
}

/// Draws `d` again in the step it runs in. Answers what it did and the command's warnings, or the command's refusal
/// (nothing written: the check runs before the objects go).
pub(crate) fn redraw(cx: &mut ExecutionContext<'_>, d: &RoadDrawingDef) -> Result<(RoadDrawingRebuilt, Vec<CommandWarning>), Refused> {
    let Some(main) = road_drawings::main_of(&d.command) else {
        return Err(Box::new(Stop::failed(error(codes::UNKNOWN_DRAWING, format!("“{}” çiziminin komutu yeniden üretilemez.", d.id), None)).into()));
    };
    let present: Vec<kentos_domain::Slot> = d
        .uids
        .iter()
        .filter_map(|u| Uuid::parse_str(u).ok().and_then(|u| cx.doc.slot_of(u)))
        .collect();
    let (uids, warnings) = run_drawing(cx, &d.id, &d.command, &d.input, &present).map_err(|(refused, _)| refused)?;
    road_drawings::keep(cx.doc, &d.command, &d.input, main, &uids, Some(&d.id)).map_err(|stop| Box::new(stop.into()))?;
    Ok((
        RoadDrawingRebuilt {
            drawing: d.id.clone(),
            command: d.command.clone(),
            removed: u32::try_from(present.len()).unwrap_or(u32::MAX),
            uids,
        },
        warnings,
    ))
}

fn left(d: &RoadDrawingDef, why: &str, message: String) -> RoadDrawingLeft {
    RoadDrawingLeft { drawing: d.id.clone(), why: why.into(), message }
}

/// What to draw again: the named drawings, refused when one is not kept, gone or (without `edited`) changed by hand;
/// or every stale or missing drawing, those changed by hand and those whose source is gone left with why.
fn chosen(cx: &ExecutionContext<'_>, input: &RoadProjectRebuild) -> Result<(Vec<RoadDrawingDef>, Vec<RoadDrawingLeft>), Stop> {
    checks::revision(cx.doc, input.expected_revision.as_deref())?;
    crate::road_project::road_project(cx.doc)?;
    let all = cx.doc.road_drawings();
    let mut out = Vec::new();
    let mut skipped = Vec::new();
    match &input.drawings {
        Some(ids) => {
            for id in ids {
                let Some(d) = all.iter().find(|d| &d.id == id) else {
                    return Err(Stop::failed(error(codes::UNKNOWN_DRAWING, format!("“{id}” yol çizimi projede yok."), Some("drawings".into()))));
                };
                match crate::road_project::drawing_state(cx.doc, d).0 {
                    S::Gone => {
                        return Err(Stop::failed(error(
                            codes::DRAWING_SOURCE_GONE,
                            format!("“{id}” çiziminin kaynağı projede yok; yeniden üretilemez, isterseniz nesnelerini silin."),
                            Some("drawings".into()),
                        )));
                    }
                    S::Edited if !input.edited => {
                        return Err(Stop::failed(error(
                            codes::DRAWING_EDITED,
                            format!("“{id}” çiziminin nesneleri elle değişti; yeniden üretmek bu değişiklikleri siler. edited ile isteyin."),
                            Some("edited".into()),
                        )));
                    }
                    _ => {
                        if !out.iter().any(|x: &RoadDrawingDef| &x.id == id) {
                            out.push(d.clone());
                        }
                    }
                }
            }
        }
        None => {
            for d in all {
                match crate::road_project::drawing_state(cx.doc, d).0 {
                    S::Stale | S::Missing => out.push(d.clone()),
                    S::Edited => skipped.push(left(d, "edited", "Nesneleri elle değişti; adıyla ve edited ile yeniden üretilir.".into())),
                    S::Gone => skipped.push(left(d, "gone", "Kaynağı projede yok.".into())),
                    _ => {}
                }
            }
        }
    }
    Ok((out, skipped))
}

fn write(cx: &mut ExecutionContext<'_>, input: RoadProjectRebuild) -> CommandResult<RoadProjectRebuilt> {
    let (drawings, left) = match chosen(cx, &input) {
        Ok(x) => x,
        Err(stop) => return stop.into(),
    };
    let mut rebuilt = Vec::new();
    let mut warnings = Vec::new();
    if !drawings.is_empty() {
        cx.doc.name_step("Yeniden üret");
    }
    // Run as a job (docs/adr/2440): its progress a drawing at a time, a stop heard between the drawings; the job's copy
    // is dropped, so a stopped rebuild writes nothing.
    let total = drawings.len() as u64;
    for (k, d) in drawings.iter().enumerate() {
        cx.progress(k as u64, total);
        if cx.stopped() {
            return CommandResult::Cancelled;
        }
        match redraw(cx, d) {
            Ok((r, w)) => {
                rebuilt.push(r);
                warnings.extend(w);
            }
            Err(refused) => return relay(*refused),
        }
    }
    cx.progress(total, total);
    CommandResult::Completed { output: RoadProjectRebuilt { rebuilt, left, revision: cx.doc.revision().to_string() }, warnings }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &RoadProjectRebuild) -> CommandResult<()> {
    match plan(cx, input) {
        CommandResult::Completed { warnings, .. } => CommandResult::Completed { output: (), warnings },
        other => relay(other),
    }
}

/// What execute would draw again, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &RoadProjectRebuild) -> CommandResult<RoadProjectRebuilt> {
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), input.clone());
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.rebuilt.iter_mut().for_each(|r| r.uids.clear());
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: RoadProjectRebuild) -> CommandResult<RoadProjectRebuilt> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_PROJECT_REBUILD,
        kentos_commands::ROAD_PROJECT_REBUILD_VERSION,
        input,
        write,
    )
}

/// Drawing again by itself (docs/adr/0911 §3): after a write, when the project says so, the drawings it left stale are
/// drawn again in its step; those changed by hand, those whose source is gone and those that do not draw now are left
/// and said. The rebuild command and the drawing commands do not follow.
pub(crate) fn follow(doc: &mut kentos_domain::Document, command: &str) -> Vec<CommandWarning> {
    if !doc.road_auto_rebuild()
        || command == kentos_commands::ROAD_PROJECT_REBUILD
        || command == kentos_commands::ROAD_PROJECT_EDIT
        || kentos_commands::ROAD_DRAWING_COMMANDS.contains(&command)
        || doc.road_drawings().is_empty()
    {
        return Vec::new();
    }
    let stale: Vec<RoadDrawingDef> =
        doc.road_drawings().iter().filter(|d| crate::road_project::drawing_state(doc, d).0 == S::Stale).cloned().collect();
    let mut warnings = Vec::new();
    let mut names = Vec::new();
    for d in doc.road_drawings().iter().filter(|d| crate::road_project::drawing_state(doc, d).0 == S::Edited) {
        let stale_too = d.sources.iter().any(|s| road_drawings::stamp_now(doc, s.kind, &s.id).as_deref() != Some(s.digest.as_str()));
        if stale_too {
            warnings.push(CommandWarning {
                code: codes::DRAWING_EDITED.into(),
                message: format!("“{}” çizimi eskidi ama nesneleri elle değiştiği için kendiliğinden yeniden üretilmedi.", d.id),
                path: None,
            });
        }
    }
    let mut cx = ExecutionContext::new(doc);
    for d in &stale {
        match redraw(&mut cx, d) {
            Ok(_) => names.push(d.id.clone()),
            Err(refused) => {
                let why = match *refused {
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => error.message,
                    _ => String::new(),
                };
                warnings.push(CommandWarning {
                    code: codes::DRAWING_NOT_REBUILT.into(),
                    message: format!("“{}” çizimi kendiliğinden yeniden üretilemedi: {why}", d.id),
                    path: None,
                });
            }
        }
    }
    if !names.is_empty() {
        warnings.insert(
            0,
            CommandWarning {
                code: codes::DRAWINGS_REBUILT.into(),
                message: format!("Eskiyen yol çizimleri kendiliğinden yeniden üretildi: {}.", names.join(", ")),
                path: None,
            },
        );
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::Progress;
    use kentos_commands::{CallOp, DocumentSnapshotV2};
    use serde_json::{Value, json};

    /// The shared cases' road project with its corridor and guardrail drawn, then the template they read changed: two
    /// drawings stale (`road.project.rebuild`'s case “İki eskiyen çizim birlikte, tek adımda”).
    fn two_stale() -> kentos_domain::Document {
        let cases: Value =
            serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.project.rebuild.json")).expect("cases");
        let snapshot: DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("a snapshot");
        let mut doc = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
        for (command, input) in [
            ("road.corridors.draw", json!({"corridor": "c-1", "layerId": "0"})),
            ("road.guardrails.draw", json!({"guardrail": "o-1", "layerId": "korkuluk"})),
            ("road.templates.edit", json!({"template": "tk-2", "operation": "set", "name": "Tip 2b"})),
        ] {
            let r = crate::registry::run(&mut ExecutionContext::new(&mut doc), command, CallOp::Execute, input).expect("runs");
            assert_eq!(r["status"], "completed", "{command}: {r}");
        }
        doc
    }

    fn all() -> RoadProjectRebuild {
        RoadProjectRebuild { drawings: None, edited: false, expected_revision: None }
    }

    /// docs/adr/2440: a stop asked before the first drawing is heard: cancelled, nothing written.
    #[test]
    fn a_stopped_rebuild_writes_nothing() {
        let mut doc = two_stale();
        let before = doc.to_snapshot_v2();
        let job = Progress::default();
        job.stop();
        let mut cx = ExecutionContext::new(&mut doc);
        cx.job = Some(&job);
        assert!(matches!(write(&mut cx, all()), CommandResult::Cancelled));
        assert_eq!(doc.to_snapshot_v2(), before);
    }

    /// As a job (the desktop's and Python's way): stopped, nothing is written; let run, it is done with its progress
    /// whole and the tree current.
    #[test]
    fn as_a_job_it_stops_or_finishes() {
        use kentos_commands::{CallEnvelope, JobPhase};
        let mut doc = two_stale();
        let before = doc.to_snapshot_v2();
        let envelope = || CallEnvelope {
            command: kentos_commands::ROAD_PROJECT_REBUILD.into(),
            version: None,
            op: CallOp::Execute,
            input: json!({}),
            request_id: None,
            expected: None,
            relax_constraints: false,
        };
        let mut job = crate::jobs::Job::start("yeniden-uret-durdur".into(), &doc, envelope());
        job.cancel();
        assert_eq!(job.wait(&mut doc).phase, JobPhase::Cancelled);
        assert_eq!(doc.to_snapshot_v2(), before);
        let mut job = crate::jobs::Job::start("yeniden-uret".into(), &doc, envelope());
        let status = job.wait(&mut doc);
        assert_eq!(status.phase, JobPhase::Done, "{status:?}");
        assert_eq!(status.progress, Some(1.0));
        let tree = crate::road_project_query::preview(&doc, false).expect("the tree");
        assert_eq!(tree.rebuildable, 0, "every drawing current");
        assert_eq!(doc.undo().as_deref(), Some("Yeniden üret"));
    }

    /// The progress goes a drawing at a time and ends whole; the answer is the shared case's two drawings.
    #[test]
    fn the_progress_is_the_drawings() {
        let mut doc = two_stale();
        let job = Progress::default();
        let mut cx = ExecutionContext::new(&mut doc);
        cx.job = Some(&job);
        let CommandResult::Completed { output, .. } = write(&mut cx, all()) else { panic!("rebuilt") };
        let names: Vec<&str> = output.rebuilt.iter().map(|r| r.drawing.as_str()).collect();
        assert_eq!(names, ["cz-1", "cz-2"]);
        assert_eq!(job.fraction(), Some(1.0));
    }
}
