//! `road.walls.draw` v1 (plan/road.md Y11.3, docs/adr/0909 §3): a retaining wall drawn in plan: its face through its
//! rows' places and its footing's outline (the toe's edge along the rows, back along the heel's); with `labels` a text
//! beside its middle row, outward of the road, saying its km range and highest stem. One undo step “İstinat duvarı
//! çiz”. The shared cases are `fixtures/commands/v1/road.walls.draw.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, EntityGeometry, NewObject, RetainingWallsDraw,
    RetainingWallsDrawn, TextAlign, TextFace, Vec2, WallSide,
};
use kentos_domain::Document;
use kentos_road::corridor::place;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::road_walls as common;

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

fn check(doc: &Document, input: &RetainingWallsDraw) -> Result<Checked, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_walls(), &input.wall, "wall")?;
    let read = common::read(doc, def)?;
    let mut warnings = checks::layer(doc, &input.layer_id)?;
    warnings.extend(read.warnings);
    let r = &read.report;
    let face: Vec<[f64; 2]> = r.rows.iter().map(|w| [w.x, w.y]).collect();
    let edges: Vec<[f64; 2]> = r.rows.iter().map(|w| common::footing_edges(def, w.offset, w.fill)).collect();
    let mut outline: Vec<[f64; 2]> = read.lines.iter().zip(&edges).map(|(l, e)| place(l, e[0])).collect();
    outline.extend(read.lines.iter().zip(&edges).rev().map(|(l, e)| place(l, e[1])));
    let mut objects = vec![polyline(&face, false), polyline(&outline, true)];
    if input.labels {
        let h = TEXT_MM * doc.settings().plot_scale / 1000.0;
        let k = r.rows.len() / 2;
        let (row, line) = (&r.rows[k], &read.lines[k]);
        let out = if def.side == WallSide::Right { 1.0 } else { -1.0 };
        let reach = edges[k][0].max(edges[k][1]).max(row.offset) * f64::from(out > 0.0)
            + edges[k][0].min(edges[k][1]).min(row.offset) * f64::from(out < 0.0);
        let p = place(line, reach + out * 2.0 * h);
        let mut angle = line.direction.to_degrees();
        if !(-90.0..=90.0).contains(&angle) {
            angle += if angle > 0.0 { -180.0 } else { 180.0 };
        }
        objects.push(object(EntityGeometry::Text {
            layout: None,
            annotative: None,
            p: Vec2 { x: p[0], y: p[1] },
            text: format!(
                "İstinat duvarı km {} – {} · H en çok {:.2} m",
                r.rows[0].km_text,
                r.rows[r.rows.len() - 1].km_text,
                r.highest
            ),
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

fn write(cx: &mut ExecutionContext<'_>, input: &RetainingWallsDraw, c: Checked) -> CommandResult<RetainingWallsDrawn> {
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: c.objects,
        operation: Some(CreateOperation::RetainingWalls),
        expected_revision: None,
    };
    let created = match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, .. } => output,
        other => return crate::trim::relay(other),
    };
    CommandResult::Completed {
        output: RetainingWallsDrawn {
            wall: c.id,
            length: c.length,
            uids: created.created,
            revision: cx.doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &RetainingWallsDraw) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would draw now, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &RetainingWallsDraw) -> CommandResult<RetainingWallsDrawn> {
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

/// Draws the wall: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RetainingWallsDraw) -> CommandResult<RetainingWallsDrawn> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_WALLS_DRAW,
        kentos_commands::ROAD_WALLS_DRAW_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, &input, c),
            Err(stop) => stop.into(),
        },
    )
}
