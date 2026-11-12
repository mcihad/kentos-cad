//! `cad.topology.repair` (docs/adr/1260 §4): a repair recipe applied to the topology findings. Its steps go in order;
//! before each the project's rules run again over the drawing as the steps before left it (GRASS's v.clean rebuilds
//! its topology before each tool in the order given), the step's finding is found by its lasting key and the shared
//! core's fix (`ops::topology_rules::fix`, the one Düzelt ▾ writes) is applied to it. A finding left on purpose, a fix
//! the finding does not offer, an object on a locked layer or outside a bound part, or a source vertex moved past the
//! quality profile's acceptance threshold refuses the whole recipe with the step that did it; a job asked to stop
//! answers cancelled. Nothing is written but by `execute`, as one undo step “Topoloji düzelt” through
//! `cad.entities.edit`'s write. The answer says the recipe as applied, every object changed with its measures before
//! and after and how far its source vertices moved (`ops::topology_rules::shift`), the change of the areas' sum and the
//! open findings before and after. The same recipe over the same drawing (a reopened file too) gives the same answer:
//! the steps are named by lasting keys and nothing depends on the session.

use std::collections::HashMap;

use kentos_commands::{
    CommandResult, EditOperation, EntitiesEdit, EntityEdit, TOPOLOGY_REPAIR_STEPS_MAX,
    TopologyRepair, TopologyRepairApplied, TopologyRepairChange, TopologyRepaired,
};
use kentos_domain::Document;
use kentos_geometry_core::entity::{Shape, entity_area, entity_length};
use kentos_geometry_core::ops::topology_rules::{self as core, Shift, fix, shift};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::edit_geometry;
use crate::topology_check::{JobWatch, gather, run_watched};

fn count(n: usize) -> u32 {
    n.min(u32::MAX as usize) as u32
}

fn fail(code: &str, message: String, path: String) -> Stop {
    Stop::failed(error(code, message, Some(path)))
}

/// What the recipe would write, and the answer.
struct Planned {
    edits: Vec<EntityEdit>,
    output: TopologyRepaired,
}

fn plan_in(
    doc: &Document,
    input: &TopologyRepair,
    watch: &dyn core::Watch,
) -> Result<Planned, Stop> {
    if input.steps.is_empty() || input.steps.len() > TOPOLOGY_REPAIR_STEPS_MAX {
        return Err(fail(
            codes::INVALID_INPUT,
            format!(
                "Onarım tarifi 1 ile {TOPOLOGY_REPAIR_STEPS_MAX} arasında adım ister; her adım bir bulgunun anahtarını ve düzeltmesini verir."
            ),
            "steps".into(),
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let settings = doc.settings().topology.clone().unwrap_or_default();
    let mut g = gather(doc, None);
    // The objects as the recipe found them, by persistent id, in the check's order.
    let source: Vec<(String, String, Shape)> = g
        .uids
        .iter()
        .zip(&g.layers)
        .zip(&g.shapes)
        .map(|((u, l), s)| (u.clone(), l.clone(), s.clone()))
        .collect();
    let mut found = run_watched(&g, watch).ok_or(Stop::Cancelled)?;
    let open_before = found.findings.iter().filter(|f| !f.exception).count();
    let mut recipe = Vec::with_capacity(input.steps.len());
    let mut by_step: HashMap<String, Vec<u32>> = HashMap::new();
    let layers = doc.layers();
    for (i, step) in input.steps.iter().enumerate() {
        if i > 0 {
            found = run_watched(&g, watch).ok_or(Stop::Cancelled)?;
        }
        let Some(f) = found.findings.iter().find(|f| g.key(f) == step.key) else {
            let message = if i == 0 {
                format!(
                    "Çizimin bugünkü denetiminde “{}” anahtarlı bulgu yok; önce cad.topology.check ile denetleyin.",
                    step.key
                )
            } else {
                format!(
                    "Tarifin ilk {i} adımından sonra çizimde “{}” anahtarlı bulgu yok: önceki adımlar onu giderdi ya da değiştirdi. Adımların sırasını ve anahtarlarını önizlemeyle denetleyin.",
                    step.key
                )
            };
            return Err(fail(codes::FINDING_NOT_FOUND, message, format!("steps[{i}].key")));
        };
        if f.exception {
            return Err(fail(
                codes::REPAIR_REFUSED_EXCEPTION,
                format!(
                    "“{}” anahtarlı bulgu istisna: bilerek bırakıldı. Onarmak için önce istisnayı kaldırın (cad.topology.exceptions, unmark).",
                    step.key
                ),
                format!("steps[{i}].key"),
            ));
        }
        let Some(fix_key) = f.fixes.iter().copied().find(|k| *k == step.fix) else {
            return Err(fail(
                codes::FIX_UNAVAILABLE,
                format!(
                    "Bu bulgu “{}” düzeltmesini sunmuyor; sunduğu düzeltmeler: {}.",
                    step.fix,
                    if f.fixes.is_empty() {
                        "yok".to_owned()
                    } else {
                        f.fixes.join(", ")
                    }
                ),
                format!("steps[{i}].fix"),
            ));
        };
        let mut changes = fix(&g.objects(), &g.core_rules, f, fix_key)
            .map_err(|why| fail(codes::FIX_UNAVAILABLE, why, format!("steps[{i}].fix")))?;
        if changes.iter().any(|c| g.outside(c.object)) {
            return Err(fail(
                codes::FIX_UNAVAILABLE,
                "Bu düzeltme alanın dışındaki bir satırı değiştirir; alanı genişletip satırı çalışma kopyasına alın."
                    .into(),
                format!("steps[{i}].fix"),
            ));
        }
        // Nothing on a locked layer is changed (docs/adr/0047): the whole recipe is refused.
        if let Some(c) = changes
            .iter()
            .find(|c| g.layers.get(c.object).is_some_and(|l| layers.is_locked(l)))
        {
            let layer = &g.layers[c.object];
            let name = layers.get(layer).map_or(layer.as_str(), |n| n.name.as_str());
            return Err(fail(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; tarifin {}. adımı üzerindeki nesneyi değiştirir. Hiçbir şey yazılmadı: kilidi Katmanlar panelinden açın ya da adımı tariften çıkarın.",
                    i + 1
                ),
                format!("steps[{i}]"),
            ));
        }
        recipe.push(TopologyRepairApplied {
            key: step.key.clone(),
            fix: fix_key.to_owned(),
            rule: g
                .core_rules
                .get(f.rule)
                .map_or_else(String::new, |r| r.id.clone()),
            problem: f.problem.to_owned(),
            uids: f
                .objects
                .iter()
                .filter_map(|&k| g.uids.get(k).cloned())
                .collect(),
        });
        // Applied to what the next step checks: deletions from the last place first, so the places before stay.
        changes.sort_by(|a, b| b.object.cmp(&a.object));
        for c in changes {
            let Some(uid) = g.uids.get(c.object).cloned() else {
                continue;
            };
            let steps = by_step.entry(uid).or_default();
            if steps.last() != Some(&(i as u32)) {
                steps.push(i as u32);
            }
            match c.shape {
                Some(s) => g.shapes[c.object] = s,
                None => {
                    g.shapes.remove(c.object);
                    g.layers.remove(c.object);
                    g.uids.remove(c.object);
                    if g.slots.remove(c.object).is_some() {
                        g.drawn -= 1;
                    }
                }
            }
        }
    }
    let last = run_watched(&g, watch).ok_or(Stop::Cancelled)?;
    let open_after = last.findings.iter().filter(|f| !f.exception).count();
    let now: HashMap<&str, &Shape> = g
        .uids
        .iter()
        .map(String::as_str)
        .zip(&g.shapes)
        .collect();
    let precision = settings.profile.as_ref().map(|p| p.precision);
    let mut changes = Vec::new();
    let mut edits = Vec::new();
    let mut whole = Shift::default();
    let mut below = 0usize;
    let mut area_change = 0.0;
    let mut farthest: Option<(f64, String, u32)> = None;
    for (uid, layer, before) in &source {
        let Some(steps) = by_step.get(uid) else {
            continue;
        };
        let after = now.get(uid.as_str()).copied();
        let area_before = entity_area(before);
        let area_after = match after {
            Some(s) => area_before.and(entity_area(s)),
            None => area_before.map(|_| 0.0),
        };
        if let (Some(a), Some(b)) = (area_before, area_after) {
            area_change += b - a;
        }
        let (moved, under) = match after {
            Some(s) => shift(before, s, precision.unwrap_or(0.0)),
            None => (Shift::default(), 0),
        };
        whole = whole.and(moved);
        below += under;
        if let Some(&step) = steps.last()
            && farthest.as_ref().is_none_or(|(d, _, _)| moved.max > *d)
        {
            farthest = Some((moved.max, uid.clone(), step));
        }
        changes.push(TopologyRepairChange {
            uid: uid.clone(),
            layer: layer.clone(),
            removed: after.is_none(),
            area_before,
            area_after: area_before.and(area_after).filter(|_| after.is_some()),
            length_before: if area_before.is_none() {
                entity_length(before)
            } else {
                None
            },
            length_after: if area_before.is_none() {
                after.and_then(entity_length)
            } else {
                None
            },
            kept: count(moved.kept),
            moved: count(moved.moved),
            max_moved: moved.max,
            total_moved: moved.total,
            steps: steps.clone(),
        });
        edits.push(match after {
            Some(s) => EntityEdit::Update {
                uid: uid.clone(),
                geometry: edit_geometry(s.clone()).ok_or_else(|| {
                    fail(
                        codes::FIX_UNAVAILABLE,
                        format!("“{uid}” nesnesinin düzeltilmiş biçimi yazılamıyor."),
                        "steps".into(),
                    )
                })?,
            },
            None => EntityEdit::Remove { uid: uid.clone() },
        });
    }
    // The institution's threshold (docs/adr/1260 §2): no source vertex farther than it.
    if let (Some(p), Some((d, uid, step))) = (&settings.profile, &farthest)
        && *d > p.acceptance
    {
        return Err(fail(
            codes::REPAIR_BEYOND_ACCEPTANCE,
            format!(
                "Tarifin {}. adımı “{uid}” nesnesinin bir kaynak köşesini {d:.4} m taşıyor; kalite profilinin (“{}”) kabul eşiği {} m. Hiçbir şey yazılmadı: adımı tariften çıkarın ya da bulguyu ölçüyle düzeltin.",
                step + 1,
                p.name,
                p.acceptance
            ),
            format!("steps[{step}]"),
        ));
    }
    Ok(Planned {
        edits,
        output: TopologyRepaired {
            recipe,
            changes,
            moved: count(whole.moved),
            max_moved: whole.max,
            total_moved: whole.total,
            below_precision: precision.map(|_| count(below)),
            area_change,
            open_before: count(open_before),
            open_after: count(open_after),
            revision: doc.revision().to_string(),
        },
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &TopologyRepair) -> CommandResult<()> {
    match plan_in(cx.doc, input, &JobWatch(cx)) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &TopologyRepair) -> CommandResult<TopologyRepaired> {
    match plan_in(cx.doc, input, &JobWatch(cx)) {
        Ok(p) => CommandResult::Completed {
            output: p.output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: TopologyRepair,
) -> CommandResult<TopologyRepaired> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_TOPOLOGY_REPAIR,
        kentos_commands::CAD_TOPOLOGY_REPAIR_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the document (docs/adr/0253 §1, §3): the objects
/// through `cad.entities.edit`'s write, its checks once more.
fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: TopologyRepair,
) -> CommandResult<TopologyRepaired> {
    let planned = match plan_in(cx.doc, &input, &JobWatch(cx)) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let Planned { edits, mut output } = planned;
    let edit = EntitiesEdit {
        operation: EditOperation::TopologyFix,
        changes: edits,
        expected_revision: None,
    };
    match crate::edit::write_in(cx, edit) {
        CommandResult::Completed { warnings, .. } => {
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output, warnings }
        }
        CommandResult::Failed { error } => CommandResult::Failed { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Cancelled => CommandResult::Cancelled,
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::Progress;

    /// The shared cases' drawing and the two-step recipe of their fifth case.
    fn drawing() -> (Document, TopologyRepair) {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/commands/v1/cad.topology.repair.json"
        );
        let fixture: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("case file"))
                .expect("case json");
        let snapshot: kentos_commands::DocumentSnapshotV2 =
            serde_json::from_value(fixture["setup"].clone()).expect("setup");
        let recipe = serde_json::from_value(fixture["cases"][4]["steps"][0]["input"].clone())
            .expect("the two-step recipe");
        (Document::from_snapshot_v2(snapshot).expect("setup"), recipe)
    }

    /// MAP-07's acceptance (docs/adr/1260 §5): a repair whose job is asked to stop answers cancelled and leaves the
    /// source geometry, the revision, the dirty state and the undo chain as they were; unasked it writes one step.
    #[test]
    fn a_stopped_repair_leaves_the_drawing_as_it_was() {
        let (mut doc, recipe) = drawing();
        let shapes: Vec<Shape> = doc.entities().map(crate::geometry::shape).collect();
        let (revision, dirty, undo) = (doc.revision(), doc.is_dirty(), doc.can_undo());
        for stop_plan in [true, false] {
            let progress = Progress::default();
            progress.stop();
            let mut cx = ExecutionContext {
                doc: &mut doc,
                store: None,
                call: None,
                job: Some(&progress),
            };
            let answer = if stop_plan {
                match plan(&cx, &recipe) {
                    CommandResult::Cancelled => None,
                    other => Some(format!("{other:?}")),
                }
            } else {
                match execute(&mut cx, recipe.clone()) {
                    CommandResult::Cancelled => None,
                    other => Some(format!("{other:?}")),
                }
            };
            assert_eq!(answer, None, "a stopped repair answers cancelled");
            let now: Vec<Shape> = doc.entities().map(crate::geometry::shape).collect();
            assert_eq!(now, shapes, "the source geometry is unchanged");
            assert_eq!(
                (doc.revision(), doc.is_dirty(), doc.can_undo()),
                (revision, dirty, undo)
            );
        }
        let progress = Progress::default();
        let mut cx = ExecutionContext {
            doc: &mut doc,
            store: None,
            call: None,
            job: Some(&progress),
        };
        match execute(&mut cx, recipe) {
            CommandResult::Completed { output, .. } => {
                assert_eq!(output.changes.len(), 2);
                assert_eq!((output.open_before, output.open_after), (3, 1));
            }
            other => panic!("the repair answered {other:?}"),
        }
        assert!(doc.can_undo() && doc.revision() != revision);
    }
}
