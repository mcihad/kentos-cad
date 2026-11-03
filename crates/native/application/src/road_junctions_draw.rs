//! `road.junctions.draw` v1 (plan/road.md Y12.2, docs/adr/0910 §2): an at-grade junction's curb returns drawn in plan,
//! each an arc from its tangent point on the first leg's edge to the second's; with `labels` a text at the crossing
//! saying the junction's name and the angle. One undo step “Kavşak çiz”. The shared cases are
//! `fixtures/commands/v1/road.junctions.draw.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, EntityGeometry, JunctionKind, JunctionsDraw,
    JunctionsDrawn, NewObject, TextAlign, TextFace, Vec2,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_junctions as common;
use crate::road_terrains::fail;

/// A text's paper height (mm) at the drawing's plot scale.
const TEXT_MM: f64 = 2.5;

struct Checked {
    id: String,
    objects: Vec<NewObject>,
    warnings: Vec<CommandWarning>,
}

fn object(geometry: EntityGeometry) -> NewObject {
    NewObject {
        geometry,
        color: None,
        line_weight: None,
        line_type: None,
        attrs: None,
        label: None,
        label_of: None,
        label_scale: None,
        symbol: None,
        order: None,
        transparency: None,
    }
}

fn check(doc: &Document, input: &JunctionsDraw) -> Result<Checked, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_junctions(), &input.junction, "junction")?;
    if def.kind == JunctionKind::Separated {
        return Err(fail(
            codes::INVALID_JUNCTION,
            format!("“{}” katlı bir kavşak; kolları birleşmez, dönüş yayı yok.", def.name),
            "junction",
        ));
    }
    let (r, read_warnings) = common::read(doc, def)?;
    let mut warnings = checks::layer(doc, &input.layer_id)?;
    warnings.extend(read_warnings);
    let mut objects: Vec<NewObject> = r
        .returns
        .iter()
        .map(|q| {
            object(EntityGeometry::Polyline {
                pts: vec![Vec2 { x: q.on_a.x, y: q.on_a.y }, Vec2 { x: q.on_b.x, y: q.on_b.y }],
                bulges: Some(vec![(q.sweep.to_radians() / 4.0).tan(), 0.0]),
                zs: None,
                ms: None,
                parts: None,
                closed: false,
                widths: None,
            })
        })
        .collect();
    if input.labels {
        let h = TEXT_MM * doc.settings().plot_scale / 1000.0;
        objects.push(object(EntityGeometry::Text {
            layout: None,
            annotative: None,
            p: Vec2 { x: r.x, y: r.y },
            text: format!("{} · {:.1}°", r.name, r.angle),
            height: h,
            rotation: 0.0,
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
    Ok(Checked { id: def.id.clone(), objects, warnings })
}

fn write(cx: &mut ExecutionContext<'_>, input: &JunctionsDraw, c: Checked) -> CommandResult<JunctionsDrawn> {
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: c.objects,
        operation: Some(CreateOperation::Junctions),
        expected_revision: None,
    };
    let created = match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, .. } => output,
        other => return crate::trim::relay(other),
    };
    CommandResult::Completed {
        output: JunctionsDrawn { junction: c.id, uids: created.created, revision: cx.doc.revision().to_string() },
        warnings: c.warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &JunctionsDraw) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would draw now, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &JunctionsDraw) -> CommandResult<JunctionsDrawn> {
    let c = match check(cx.doc, input) {
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

/// Draws the returns: one undo step.
/// Draws `input` now, checked and written in the step it runs in, its drawing not kept (`road.project.rebuild` keeps it,
/// docs/adr/0911 §3).
pub(crate) fn draw(cx: &mut ExecutionContext<'_>, input: &kentos_commands::JunctionsDraw) -> CommandResult<kentos_commands::JunctionsDrawn> {
    match check(cx.doc, input) {
        Ok(c) => write(cx, input, c),
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: JunctionsDraw) -> CommandResult<JunctionsDrawn> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_JUNCTIONS_DRAW,
        kentos_commands::ROAD_JUNCTIONS_DRAW_VERSION,
        input,
        |cx, input| {
            let r = match check(cx.doc, &input) {
                Ok(c) => write(cx, &input, c),
                Err(stop) => stop.into(),
            };
            crate::road_drawings::kept(cx, kentos_commands::ROAD_JUNCTIONS_DRAW, &input, (kentos_commands::RoadSourceKind::Junction, "junction"), r, |o| &o.uids)
        },
    )
}
