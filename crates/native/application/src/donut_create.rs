//! Halka's single computation/write path, shared by its tool, Python and MCP.
use kentos_commands::{
    CommandResult, DonutCreate, EntitiesCreate, EntitiesCreatePlan, EntitiesCreated,
    EntityGeometry, HatchLoop, HatchPattern, HatchPatternType, NewObject, Vec2,
};
use kentos_geometry_core::tools::drawing::donut_rings;
use crate::{ExecutionContext, creating, codes};
use crate::checks::{self, Stop, error};

fn made(input: &DonutCreate) -> Result<EntitiesCreate, Stop> {
    checks::point(input.center, "Merkezin", "center")?;
    for (n, name, path) in [
        (input.inner, "İç çap", "inner"),
        (input.outer, "Dış çap", "outer"),
    ] {
        checks::finite(n, name, "Sonlu bir çap verin.", path)?;
    }
    let invalid = if input.inner < 0.0 {
        Some(("İç çap sıfır ya da pozitif olmalı.", "inner"))
    } else if input.outer <= input.inner {
        Some(("Dış çap iç çaptan büyük olmalı.", "outer"))
    } else {
        None
    };
    if let Some((message, path)) = invalid {
        return Err(Stop::failed(error(
            codes::INVALID_DISTANCE,
            message.into(),
            Some(path.into()),
        )));
    }
    checks::line_weight(input.line_weight, "lineWeight")?;
    let c = input.center;
    let rings = donut_rings(
        kentos_geometry_core::Vec2::new(c.x, c.y),
        input.inner,
        input.outer,
    );
    let wire = |p: kentos_geometry_core::Vec2| Vec2 { x: p.x, y: p.y };
    let circle = |r: f64| HatchLoop {
        pts: vec![Vec2 { x: c.x + r, y: c.y }, Vec2 { x: c.x - r, y: c.y }],
        bulges: Some(vec![1.0, 1.0]),
        curves: None,
    };
    let mut boundary = vec![circle(input.outer / 2.0)];
    if input.inner > 0.0 {
        boundary.push(circle(input.inner / 2.0));
    }
    Ok(EntitiesCreate {
        layer_id: input.layer_id.clone(),
        operation: None,
        expected_revision: input.expected_revision.clone(),
        objects: vec![NewObject {
            geometry: EntityGeometry::Hatch {
                boundary: Some(boundary),
                ring: rings.ring.into_iter().map(wire).collect(),
                holes: rings.holes.map(|holes| {
                    holes
                        .into_iter()
                        .map(|h| h.into_iter().map(wire).collect())
                        .collect()
                }),
                pattern: HatchPattern::user(HatchPatternType::Solid, 0.0, 1.0),
                assoc: None,
                annotative: None,
            },
            color: input.color.clone(),
            line_weight: input.line_weight,
            line_type: input.line_type,
            attrs: input.attrs.clone(),
            label: input.label.clone(),
            symbol: input.symbol.clone(),
            label_of: None,
            label_scale: None,
            transparency: None,
            order: None,
        }],
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &DonutCreate) -> CommandResult<()> {
    match made(input) {
        Ok(made) => creating::validate(cx, &made),
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &DonutCreate) -> CommandResult<EntitiesCreatePlan> {
    match made(input) {
        Ok(made) => creating::plan(cx.doc, &made),
        Err(stop) => stop.into(),
    }
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: DonutCreate,
) -> CommandResult<EntitiesCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_DONUT_CREATE,
        kentos_commands::CAD_DONUT_CREATE_VERSION,
        input,
        |cx, input| match made(&input) {
            Ok(made) => creating::execute(cx, made),
            Err(stop) => stop.into(),
        },
    )
}
