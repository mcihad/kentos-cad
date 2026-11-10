//! `road.guardrails.draw` v1 (plan/road.md Y11.1, docs/adr/0909 §1): a guardrail's runs drawn in plan, each a polyline
//! at the rail's place: every cross's hinge plus the guardrail's offset outward, its ends where the run starts and ends
//! (the hinge's offset read linearly between the crosses beside), placed on the corridor's alignment; with `labels` a
//! text at each run's middle along the alignment saying its km and length. One undo step “Otokorkuluk çiz”. The
//! shared cases are `fixtures/commands/v1/road.guardrails.draw.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, EntityGeometry, GuardrailsDraw, GuardrailsDrawn,
    NewObject, TextAlign, TextFace, Vec2,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::road_guardrails::{self as common, Side};

/// A text's paper height (mm) at the drawing's plot scale.
const TEXT_MM: f64 = 2.5;

struct Checked {
    id: String,
    runs: u32,
    length: f64,
    objects: Vec<NewObject>,
    warnings: Vec<CommandWarning>,
}

/// The hinge's offset at station `s`, read linearly between the crosses beside it.
fn hinge_at(sides: &[Side], s: f64) -> f64 {
    let i = sides.partition_point(|x| x.station <= s);
    match (i.checked_sub(1).and_then(|k| sides.get(k)), sides.get(i)) {
        (Some(a), Some(b)) if b.station > a.station => a.hinge + (b.hinge - a.hinge) * (s - a.station) / (b.station - a.station),
        (Some(a), _) => a.hinge,
        (None, Some(b)) => b.hinge,
        (None, None) => 0.0,
    }
}

fn object(geometry: EntityGeometry) -> NewObject {
    NewObject {
        geometry,
        color: None,
        line_weight: None,
        line_type: None,
        line_pattern: None,
        attrs: None,
        label: None,
        label_of: None,
        label_scale: None,
        symbol: None,
        order: None,
        transparency: None,
    }
}

fn check(doc: &Document, input: &GuardrailsDraw, told: &dyn Fn(u64, u64) -> bool) -> Result<Checked, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_guardrails(), &input.guardrail, "guardrail")?;
    let read = common::read_told(doc, def, told)?;
    let mut warnings = checks::layer(doc, &input.layer_id)?;
    warnings.extend(read.warnings);
    let core = &read.alignment.core;
    let h = TEXT_MM * doc.settings().plot_scale / 1000.0;
    let mut objects = Vec::new();
    let mut length = 0.0;
    for run in &read.runs {
        let sides = &read.sides[usize::from(run.right)];
        let outward = if run.right { def.offset } else { -def.offset };
        let at = |s: f64| {
            let o = hinge_at(sides, s) + outward;
            core.at(s).map(|p| {
                let (sin, cos) = p.direction.sin_cos();
                ([p.x + sin * o, p.y - cos * o], p.direction)
            })
        };
        let mut stations = vec![run.from];
        stations.extend(sides.iter().map(|x| x.station).filter(|&s| s > run.from + 1e-9 && s < run.to - 1e-9));
        stations.push(run.to);
        let pts: Vec<[f64; 2]> = stations.iter().filter_map(|&s| at(s).map(|p| p.0)).collect();
        if pts.len() < 2 {
            continue;
        }
        length += run.to - run.from;
        objects.push(object(EntityGeometry::Polyline {
            pts: pts.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
            bulges: None,
            zs: None,
            ms: None,
            parts: None,
            closed: false,
            widths: None,
        }));
        if input.labels
            && let Some((p, direction)) = at(0.5 * (run.from + run.to))
        {
            let text = |s: f64| kentos_road::alignment::km_text(read.alignment.km(s), 3);
            let mut angle = direction.to_degrees();
            if !(-90.0..=90.0).contains(&angle) {
                angle += if angle > 0.0 { -180.0 } else { 180.0 };
            }
            // Outward of the rail by a text's height.
            let side = if run.right { 1.0 } else { -1.0 };
            let (sin, cos) = direction.sin_cos();
            objects.push(object(EntityGeometry::Text {
                layout: None,
                annotative: None,
                p: Vec2 { x: p[0] + sin * h * side, y: p[1] - cos * h * side },
                text: format!("Otokorkuluk km {} – {}, {:.1} m", text(run.from), text(run.to), run.to - run.from),
                height: h,
                rotation: angle,
                align: Some(TextAlign::MiddleCenter),
                width_factor: None,
                mask: false,
                box_width: None,
                line_spacing: None,
                runs: Vec::new(),
                face: TextFace::default(),
                path: None,
            }));
        }
    }
    Ok(Checked { id: def.id.clone(), runs: read.runs.len() as u32, length, objects, warnings })
}

fn write(cx: &mut ExecutionContext<'_>, input: &GuardrailsDraw, c: Checked) -> CommandResult<GuardrailsDrawn> {
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: c.objects,
        operation: Some(CreateOperation::Guardrails),
        expected_revision: None,
    };
    let created = match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, .. } => output,
        other => return crate::trim::relay(other),
    };
    CommandResult::Completed {
        output: GuardrailsDrawn {
            guardrail: c.id,
            runs: c.runs,
            length: c.length,
            uids: created.created,
            revision: cx.doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &GuardrailsDraw) -> CommandResult<()> {
    match check(cx.doc, input, &cx.stop_only()) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would draw now, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &GuardrailsDraw) -> CommandResult<GuardrailsDrawn> {
    let c = match check(cx.doc, input, &cx.stop_only()) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), input, c);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.uids.clear();
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Draws the runs: one undo step.
/// Draws `input` now, checked and written in the step it runs in, its drawing not kept (`road.project.rebuild` keeps it,
/// docs/adr/0911 §3).
pub(crate) fn draw(cx: &mut ExecutionContext<'_>, input: &kentos_commands::GuardrailsDraw) -> CommandResult<kentos_commands::GuardrailsDrawn> {
    let checked = check(cx.doc, input, &cx.told());
    match checked {
        Ok(c) => write(cx, input, c),
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: GuardrailsDraw) -> CommandResult<GuardrailsDrawn> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_GUARDRAILS_DRAW,
        kentos_commands::ROAD_GUARDRAILS_DRAW_VERSION,
        input,
        |cx, input| {
            let checked = check(cx.doc, &input, &cx.told());
            let r = match checked {
                Ok(c) => write(cx, &input, c),
                Err(stop) => stop.into(),
            };
            crate::road_drawings::kept(cx, kentos_commands::ROAD_GUARDRAILS_DRAW, &input, (kentos_commands::RoadSourceKind::Guardrail, "guardrail"), r, |o| &o.uids)
        },
    )
}

#[cfg(test)]
mod stop_tests {
    //! Docs/adr/4860: the corridor's sections, then the crosses told and a stop heard within (`crate::told_tests`), on the shared cases' project.
    use super::*;

    const CASES: &str = include_str!("../../../../fixtures/commands/v1/road.guardrails.draw.json");

    #[test]
    fn its_reckoning_is_told_and_hears_a_stop() {
        let (doc, input): (kentos_domain::Document, kentos_commands::GuardrailsDraw) = crate::told_tests::project(CASES);
        crate::told_tests::told_and_stopped(|told| check(&doc, &input, told));
    }

    #[test]
    fn a_stopped_job_draws_nothing() {
        let (mut doc, input): (kentos_domain::Document, kentos_commands::GuardrailsDraw) = crate::told_tests::project(CASES);
        crate::told_tests::stopped_job_draws_nothing(&mut doc, |cx| draw(cx, &input));
    }
}
