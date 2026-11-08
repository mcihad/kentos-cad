//! `road.drainage.draw` v1 (plan/road.md Y16, docs/adr/0929 §2): a drainage query drawn on a layer: the catchment's
//! outline as closed polylines and its flow path as a 3D polyline, each pond's bottom as a point with its height; with
//! `labels` the catchment's area and peak flow at the discharge point and each pond's depth. One undo step
//! “Drenaj çiz”. The shared cases are `fixtures/commands/v1/road.drainage.draw.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, DrainageDraw, DrainageDrawn, EntitiesCreate, EntityGeometry, NewObject,
    TextAlign, TextFace, Vec2,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_terrains::fail;

/// A text's paper height (mm) at the drawing's plot scale.
const TEXT_MM: f64 = 2.5;

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

fn comma(v: f64, digits: usize) -> String {
    format!("{v:.digits$}").replace('.', ",")
}

fn text(doc: &Document, at: [f64; 2], words: String) -> NewObject {
    object(EntityGeometry::Text {
        layout: None,
        annotative: None,
        p: Vec2 { x: at[0], y: at[1] },
        text: words,
        height: TEXT_MM * doc.settings().plot_scale / 1000.0,
        rotation: 0.0,
        align: Some(TextAlign::BottomLeft),
        width_factor: None,
        mask: false,
        box_width: None,
        line_spacing: None,
        runs: Vec::new(),
        face: TextFace::default(),
        path: None,
    })
}

struct Checked {
    objects: Vec<NewObject>,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &DrainageDraw) -> Result<Checked, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let (q, mut warnings) = crate::road_drainage::answer(doc, &input.query)?;
    warnings.extend(checks::layer(doc, &input.layer_id)?);
    let mut objects = Vec::new();
    if let Some(c) = &q.catchment {
        for ring in &c.outline {
            objects.push(object(EntityGeometry::Polyline {
                pts: ring.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
                bulges: None,
                zs: None,
                ms: None,
                parts: None,
                closed: true,
                widths: None,
            }));
        }
        if c.path.len() >= 2 {
            objects.push(object(EntityGeometry::Polyline {
                pts: c.path.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
                bulges: None,
                zs: Some(c.path.iter().map(|p| Some(p[2])).collect()),
                ms: None,
                parts: None,
                closed: false,
                widths: None,
            }));
        }
        if input.labels {
            let peak = c.peak.map_or(String::new(), |p| format!(" · Q {} m³/s", comma(p, 3)));
            objects.push(text(doc, c.at, format!("Havza {} ha{peak}", comma(c.area / 10_000.0, 3))));
        }
    }
    for p in &q.ponds {
        objects.push(object(EntityGeometry::Point { p: Vec2 { x: p.bottom[0], y: p.bottom[1] }, z: Some(p.bottom[2]), parts: None }));
        if input.labels {
            objects.push(text(doc, [p.bottom[0], p.bottom[1]], format!("Gölet · derinlik {} m", comma(p.depth, 2))));
        }
    }
    if objects.is_empty() {
        return Err(fail(
            codes::DRAINAGE_NOTHING,
            "Çizilecek havza ya da gölet yok: soruya bir havza (catchment) ya da göletler (ponds) katın.".into(),
            "query",
        ));
    }
    Ok(Checked { objects, warnings })
}

fn write(cx: &mut ExecutionContext<'_>, input: &DrainageDraw, c: Checked) -> CommandResult<DrainageDrawn> {
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: c.objects,
        operation: Some(CreateOperation::Drainage),
        expected_revision: None,
    };
    let created = match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, .. } => output,
        other => return crate::trim::relay(other),
    };
    CommandResult::Completed {
        output: DrainageDrawn { uids: created.created, revision: cx.doc.revision().to_string() },
        warnings: c.warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &DrainageDraw) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &DrainageDraw) -> CommandResult<DrainageDrawn> {
    let c = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), input, c);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Draws the catchment and the ponds: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: DrainageDraw) -> CommandResult<DrainageDrawn> {
    crate::call::recorded(cx, kentos_commands::ROAD_DRAINAGE_DRAW, kentos_commands::ROAD_DRAINAGE_DRAW_VERSION, input, |cx, input| {
        match check(cx.doc, &input) {
            Ok(c) => write(cx, &input, c),
            Err(stop) => stop.into(),
        }
    })
}
