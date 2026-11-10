//! `road.culverts.draw` v1 (plan/road.md Y11.2, docs/adr/0909 §2): a culvert's barrel drawn in plan: its outline (the
//! two outer edges, half its outer width each side of its line, closed by its ends) and its line between its ends; with
//! `labels` a text along its line, outward of the outline, saying its km, cross and length. One undo step “Menfez çiz”.
//! The shared cases are `fixtures/commands/v1/road.culverts.draw.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, CulvertShapeDef, CulvertsDraw, CulvertsDrawn, EntitiesCreate,
    EntityGeometry, NewObject, TextAlign, TextFace, Vec2,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::road_culverts as common;

/// A text's paper height (mm) at the drawing's plot scale.
const TEXT_MM: f64 = 2.5;

struct Checked {
    id: String,
    length: f64,
    objects: Vec<NewObject>,
    warnings: Vec<CommandWarning>,
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

fn polyline(pts: &[[f64; 2]], closed: bool) -> NewObject {
    object(EntityGeometry::Polyline {
        pts: pts.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
        bulges: None,
        zs: None,
        ms: None,
        parts: None,
        closed,
        widths: None,
    })
}

/// The cross in words: “Ø1.00 m × 2” or “2.00 × 1.50 m kutu × 2”.
fn cross_words(shape: &CulvertShapeDef) -> String {
    match *shape {
        CulvertShapeDef::Pipe { diameter, count, .. } => {
            if count > 1 { format!("Ø{diameter:.2} m × {count}") } else { format!("Ø{diameter:.2} m") }
        }
        CulvertShapeDef::Box { width, height, cells, .. } => {
            let one = format!("{width:.2} × {height:.2} m kutu");
            if cells > 1 { format!("{one} × {cells}") } else { one }
        }
    }
}

fn check(doc: &Document, input: &CulvertsDraw, told: &dyn Fn(u64, u64) -> bool) -> Result<Checked, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_culverts(), &input.culvert, "culvert")?;
    let read = common::read_told(doc, def, told)?;
    let mut warnings = checks::layer(doc, &input.layer_id)?;
    warnings.extend(read.warnings);
    let r = &read.report;
    let (cos, sin) = (read.angle.cos(), read.angle.sin());
    let half = 0.5 * r.width;
    let along = |d: f64, side: f64| [read.at[0] + cos * d - sin * side, read.at[1] + sin * d + cos * side];
    let (d0, d1) = (r.ends[0].distance, r.ends[1].distance);
    let mut objects = vec![
        polyline(&[along(d0, half), along(d1, half), along(d1, -half), along(d0, -half)], true),
        polyline(&[along(d0, 0.0), along(d1, 0.0)], false),
    ];
    if input.labels {
        let h = TEXT_MM * doc.settings().plot_scale / 1000.0;
        let mut angle = read.angle.to_degrees();
        if !(-90.0..=90.0).contains(&angle) {
            angle += if angle > 0.0 { -180.0 } else { 180.0 };
        }
        let p = along(0.5 * (d0 + d1), half + h);
        objects.push(object(EntityGeometry::Text {
            layout: None,
            annotative: None,
            p: Vec2 { x: p[0], y: p[1] },
            text: format!("Menfez km {} · {} · L={:.2} m", r.km_text, cross_words(&def.shape), r.length),
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
    Ok(Checked { id: def.id.clone(), length: r.length, objects, warnings })
}

fn write(cx: &mut ExecutionContext<'_>, input: &CulvertsDraw, c: Checked) -> CommandResult<CulvertsDrawn> {
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: c.objects,
        operation: Some(CreateOperation::Culverts),
        expected_revision: None,
    };
    let created = match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, .. } => output,
        other => return crate::trim::relay(other),
    };
    CommandResult::Completed {
        output: CulvertsDrawn {
            culvert: c.id,
            length: c.length,
            uids: created.created,
            revision: cx.doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &CulvertsDraw) -> CommandResult<()> {
    match check(cx.doc, input, &cx.stop_only()) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would draw now, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &CulvertsDraw) -> CommandResult<CulvertsDrawn> {
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

/// Draws the barrel: one undo step.
/// Draws `input` now, checked and written in the step it runs in, its drawing not kept (`road.project.rebuild` keeps it,
/// docs/adr/0911 §3).
pub(crate) fn draw(cx: &mut ExecutionContext<'_>, input: &kentos_commands::CulvertsDraw) -> CommandResult<kentos_commands::CulvertsDrawn> {
    let checked = check(cx.doc, input, &cx.told());
    match checked {
        Ok(c) => write(cx, input, c),
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: CulvertsDraw) -> CommandResult<CulvertsDrawn> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_CULVERTS_DRAW,
        kentos_commands::ROAD_CULVERTS_DRAW_VERSION,
        input,
        |cx, input| {
            let checked = check(cx.doc, &input, &cx.told());
            let r = match checked {
                Ok(c) => write(cx, &input, c),
                Err(stop) => stop.into(),
            };
            crate::road_drawings::kept(cx, kentos_commands::ROAD_CULVERTS_DRAW, &input, (kentos_commands::RoadSourceKind::Culvert, "culvert"), r, |o| &o.uids)
        },
    )
}

#[cfg(test)]
mod stop_tests {
    //! Docs/adr/4860: the corridor's sections, then the cross told and a stop heard within (`crate::told_tests`), on the shared cases' project.
    use super::*;

    const CASES: &str = include_str!("../../../../fixtures/commands/v1/road.culverts.draw.json");

    #[test]
    fn its_reckoning_is_told_and_hears_a_stop() {
        let (doc, input): (kentos_domain::Document, kentos_commands::CulvertsDraw) = crate::told_tests::project(CASES);
        crate::told_tests::told_and_stopped(|told| check(&doc, &input, told));
    }

    #[test]
    fn a_stopped_job_draws_nothing() {
        let (mut doc, input): (kentos_domain::Document, kentos_commands::CulvertsDraw) = crate::told_tests::project(CASES);
        crate::told_tests::stopped_job_draws_nothing(&mut doc, |cx| draw(cx, &input));
    }
}
