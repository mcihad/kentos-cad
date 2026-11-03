//! `road.routes.draw` v1 (plan/road.md Y12.1, docs/adr/0910 §1): the route found drawn as one 3D polyline on a layer,
//! to be fitted as an alignment (`road.alignments.fit`); one undo step “Ön güzergâh”. The shared cases are
//! `fixtures/commands/v1/road.routes.draw.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, EntityGeometry, NewObject, RouteFound, RoutesDraw,
    RoutesDrawn, Vec2,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};

struct Checked {
    route: RouteFound,
    object: NewObject,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &RoutesDraw) -> Result<Checked, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let route = crate::road_routes::find(doc, &input.ask)?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    let object = NewObject {
        geometry: EntityGeometry::Polyline {
            pts: route.points.iter().map(|p| Vec2 { x: p.x, y: p.y }).collect(),
            bulges: None,
            zs: Some(route.points.iter().map(|p| Some(p.z)).collect()),
            ms: None,
            parts: None,
            closed: false,
            widths: None,
        },
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
    };
    Ok(Checked { route, object, warnings })
}

fn write(cx: &mut ExecutionContext<'_>, input: &RoutesDraw, c: Checked) -> CommandResult<RoutesDrawn> {
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: vec![c.object],
        operation: Some(CreateOperation::Routes),
        expected_revision: None,
    };
    let created = match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, .. } => output,
        other => return crate::trim::relay(other),
    };
    CommandResult::Completed {
        output: RoutesDrawn { route: c.route, uids: created.created, revision: cx.doc.revision().to_string() },
        warnings: c.warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &RoutesDraw) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would draw now, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &RoutesDraw) -> CommandResult<RoutesDrawn> {
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

/// Draws the route: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RoutesDraw) -> CommandResult<RoutesDrawn> {
    crate::call::recorded(cx, kentos_commands::ROAD_ROUTES_DRAW, kentos_commands::ROAD_ROUTES_DRAW_VERSION, input, |cx, input| {
        match check(cx.doc, &input) {
            Ok(c) => write(cx, &input, c),
            Err(stop) => stop.into(),
        }
    })
}
