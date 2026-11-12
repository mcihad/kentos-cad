//! `road.swept.draw` v1 (plan/road.md Y17, docs/adr/0915 §2): a swept path query drawn on a layer: the front axle's
//! path and the bodies' corners' tracks as polylines, and every `every`th sample's bodies (the first and the last
//! always) as closed polylines; with `labels` the off-tracking and the clearance at the path's end. One undo step
//! “Araç izi çiz”. The shared cases are `fixtures/commands/v1/road.swept.draw.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, EntityGeometry, NewObject, SweptDraw, SweptDrawn, TextAlign,
    TextFace, Vec2,
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

fn comma(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

struct Checked {
    objects: Vec<NewObject>,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &SweptDraw) -> Result<Checked, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    if input.every == Some(0) {
        return Err(fail(codes::INVALID_SWEPT, "Gövdeler her kaç örnekte bir çizilecek (every) sıfırdan büyük olmalı.".into(), "every"));
    }
    let (q, mut warnings) = crate::road_swept::answer(doc, &input.query)?;
    warnings.extend(checks::layer(doc, &input.layer_id)?);
    let mut objects = vec![polyline(&q.front, false)];
    objects.extend(q.corners.iter().map(|c| polyline(c, false)));
    if let Some(every) = input.every {
        let n = q.front.len();
        let units = q.corners.len() / 4;
        for k in (0..n).filter(|k| k % every as usize == 0 || *k + 1 == n) {
            for j in 0..units {
                let body: Vec<[f64; 2]> = (0..4).map(|i| q.corners[4 * j + i][k]).collect();
                objects.push(polyline(&body, true));
            }
        }
    }
    if input.labels
        && let Some(end) = q.front.last()
    {
        let mut words = format!("İç kayma {} m", comma(q.offtracking));
        if let Some(t) = q.trailer_offtracking {
            words.push_str(&format!(" · römork {} m", comma(t)));
        }
        if let Some(c) = &q.clearance {
            words.push_str(&format!(" · açıklık {} m", comma(c.distance)));
        }
        objects.push(object(EntityGeometry::Text {
            layout: None,
            annotative: None,
            p: Vec2 { x: end[0], y: end[1] },
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
        }));
    }
    Ok(Checked { objects, warnings })
}

fn write(cx: &mut ExecutionContext<'_>, input: &SweptDraw, c: Checked) -> CommandResult<SweptDrawn> {
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: c.objects,
        operation: Some(CreateOperation::Swept),
        expected_revision: None,
    };
    let created = match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, .. } => output,
        other => return crate::trim::relay(other),
    };
    CommandResult::Completed {
        output: SweptDrawn { uids: created.created, revision: cx.doc.revision().to_string() },
        warnings: c.warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &SweptDraw) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &SweptDraw) -> CommandResult<SweptDrawn> {
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

/// Draws the tracks and the bodies: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: SweptDraw) -> CommandResult<SweptDrawn> {
    crate::call::recorded(cx, kentos_commands::ROAD_SWEPT_DRAW, kentos_commands::ROAD_SWEPT_DRAW_VERSION, input, |cx, input| {
        match check(cx.doc, &input) {
            Ok(c) => write(cx, &input, c),
            Err(stop) => stop.into(),
        }
    })
}
