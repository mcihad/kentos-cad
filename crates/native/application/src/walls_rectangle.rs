//! `cad.walls.rectangle` v1 (plan/architecture.md ARCH-DRAW-01): a sharp rectangle on an explicit
//! inner/outer/centre reference, written as one ordinary closed wall. The shape and wall command
//! paths are reused; the free wall, rectangle, renderer and project-unit rules are unchanged.

use kentos_commands::{
    CommandError, CommandResult, Entity, NewWall, RectangleCreate, RectangleFrame,
    WallRectangleReference, WallsCreate, WallsCreated, WallsPlan, WallsRectangle,
};
use kentos_domain::Document;
use kentos_geometry_core::geometry::signed_area;
use kentos_geometry_core::jsmath::{cos, js_hypot, sin};

use crate::checks::error;
use crate::{ExecutionContext, codes, shapes, walls_create};

/// Stable failure used by every mode when the chosen reference would leave no inner rectangle.
const NO_INTERIOR: &str = "Dikdörtgenin içi duvar kalınlığından sonra kapanıyor. Boyları büyütün, kalınlığı azaltın ya da İç net seçin.";

/// The user's rectangle as the wall request, and with `adopt` (docs/adr/0770 §1) the walls it would run on left out:
/// what is left as open walls, the adopted walls' ids. Writes nothing.
fn request_in(
    doc: &Document,
    input: &WallsRectangle,
) -> Result<(WallsCreate, Vec<String>), Box<CommandError>> {
    let mut wall = request(input)?;
    if !input.adopt {
        return Ok((wall, Vec::new()));
    }
    use kentos_geometry_core::Vec2 as P;
    use kentos_geometry_core::geom::wall_join::WallIn;
    use kentos_geometry_core::ops::wall_adopt::{Reference, adopt};
    let level = input.arch.as_ref().and_then(|a| a.level.clone());
    let mut uids = Vec::new();
    let mut shapes = Vec::new();
    for e in doc.entities() {
        if let Entity::Wall(w) = e
            && w.wall.arch.level == level
            && let Some(u) = doc.uid(kentos_domain::Slot(e.base().id))
        {
            uids.push(u.to_string());
            shapes.push(crate::walls::core_shape(w));
        }
    }
    let ins: Vec<WallIn<'_>> = shapes
        .iter()
        .filter_map(|s| match s {
            kentos_geometry_core::entity::Shape::Wall { pts, bulges, wall } => Some(WallIn {
                pts,
                bulges: bulges.as_deref(),
                spec: wall,
            }),
            _ => None,
        })
        .collect();
    let template = wall.walls.remove(0);
    let ring: Vec<P> = template.pts.iter().map(|p| P::new(p.x, p.y)).collect();
    let reference = match input.reference {
        WallRectangleReference::Inner => Reference::Inner,
        WallRectangleReference::Outer => Reference::Outer,
        WallRectangleReference::Center => Reference::Center,
    };
    let left = adopt(&ring, reference, &ins);
    if left.paths.is_empty() {
        return Err(Box::new(error(
            codes::NOTHING_TO_SET,
            "Dikdörtgenin bütün kenarları var olan duvarlarda; çizilecek yeni duvar kalmadı."
                .into(),
            Some("frame".into()),
        )));
    }
    for path in left.paths {
        wall.walls.push(NewWall {
            pts: path
                .into_iter()
                .map(|p| kentos_commands::Vec2 { x: p.x, y: p.y })
                .collect(),
            closed: left.closed,
            ..template.clone()
        });
    }
    Ok((
        wall,
        left.walls.into_iter().map(|j| uids[j].clone()).collect(),
    ))
}

/// The user's rectangle converted to the existing wall request. This writes neither geometry nor styles.
fn request(input: &WallsRectangle) -> Result<WallsCreate, Box<CommandError>> {
    let rectangle = RectangleCreate {
        layer_id: input.layer_id.clone(),
        frame: input.frame.clone(),
        rotation: input.rotation,
        corners: None,
        color: None,
        line_weight: None,
        line_type: None,
        attrs: None,
        label: None,
        symbol: None,
        rules: None,
        expected_revision: None,
    };
    let (mut ring, _) = shapes::rectangle_ring(&rectangle)?;
    // Rectangle helpers retain the quadrant's winding. This command alone makes the meaning of
    // inner and outer independent of that quadrant while keeping the user's first corner.
    if signed_area(&ring) < 0.0 {
        ring[1..].reverse();
    }
    Ok(WallsCreate {
        layer_id: input.layer_id.clone(),
        walls: vec![NewWall {
            pts: ring
                .into_iter()
                .map(|p| kentos_commands::Vec2 { x: p.x, y: p.y })
                .collect(),
            bulges: None,
            closed: true,
            width: input.width,
            justify: Some(input.reference.justify()),
            offset: None,
            style: input.style.clone(),
            caps: None,
            height: input.height,
            arch: input.arch.clone(),
            attrs: input.attrs.clone(),
            label: input.label.clone(),
        }],
        color: input.color.clone(),
        line_weight: input.line_weight,
        line_type: input.line_type,
        expected_revision: input.expected_revision.clone(),
    })
}

/// A delegated wall's public error paths describe this request rather than its internal one-item list.
fn public_path(path: &mut Option<String>) {
    let Some(rest) = path.as_deref().and_then(|p| p.strip_prefix("walls[0]")) else {
        return;
    };
    *path = Some(
        if rest.is_empty() || rest.starts_with(".pts") || rest.starts_with(".bulges") {
            "frame".to_owned()
        } else {
            rest.strip_prefix('.').unwrap_or(rest).to_owned()
        },
    );
}

fn public_paths<T>(mut result: CommandResult<T>) -> CommandResult<T> {
    match &mut result {
        CommandResult::Completed { warnings, .. } => {
            for warning in warnings {
                public_path(&mut warning.path);
            }
        }
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => public_path(&mut error.path),
        _ => {}
    }
    result
}

/// Check the user's dimensions before adding the distant global origin to the rotated vertices.
/// In particular Size(4,3) stays exactly 4 by 3 at an inset boundary, rather than becoming
/// 3.00000000002 after a rotation near large map coordinates. No tolerance is enlarged here.
fn dimensions(input: &WallsRectangle) -> (f64, f64) {
    match &input.frame {
        RectangleFrame::Size { length, width, .. } => (*length, *width),
        RectangleFrame::Area { area, length, .. } => (*length, area / length),
        RectangleFrame::Edge { a, b, width } => (js_hypot(b.x - a.x, b.y - a.y), width.abs()),
        RectangleFrame::Corners { a, b } => {
            let rotation = input.rotation.unwrap_or(0.0);
            let (c, s) = (cos(rotation), sin(rotation));
            let (x, y) = (b.x - a.x, b.y - a.y);
            ((x * c + y * s).abs(), (-x * s + y * c).abs())
        }
    }
}

fn interior_is_empty(input: &WallsRectangle, thickness: f64) -> bool {
    let inset = match input.reference {
        WallRectangleReference::Inner => 0.0,
        WallRectangleReference::Outer => 2.0,
        WallRectangleReference::Center => 1.0,
    };
    let (length, width) = dimensions(input);
    length - inset * thickness <= 0.0 || width - inset * thickness <= 0.0
}

fn checked_plan(
    doc: &Document,
    input: &WallsRectangle,
    wall: &WallsCreate,
) -> CommandResult<WallsPlan> {
    let result = public_paths(walls_create::plan_in(doc, wall));
    let thickness = match &result {
        CommandResult::Completed { output, .. } => output.walls.first().and_then(|e| match e {
            Entity::Wall(w) => Some(w.wall.width),
            _ => None,
        }),
        // A straight inset may collapse before the normal plan can return its resolved wall.
        // Its existing style/width validations have already passed; give this command's more
        // useful inner-rectangle refusal for that same case, rather than a generic offset error.
        CommandResult::Failed { error } if error.code == codes::WALL_COLLAPSES => {
            let style = input
                .style
                .as_deref()
                .and_then(|s| crate::walls::find_style(doc, s).map(|v| v.0));
            Some(crate::walls::styled(style.as_ref(), input.width).0)
        }
        _ => None,
    };
    if let Some(thickness) = thickness
        && interior_is_empty(input, thickness)
    {
        return CommandResult::Failed {
            error: error(
                codes::INVALID_WALL,
                NO_INTERIOR.into(),
                Some("frame".into()),
            ),
        };
    }
    result
}

pub fn validate(cx: &ExecutionContext<'_>, input: &WallsRectangle) -> CommandResult<()> {
    match plan_in(cx.doc, input) {
        CommandResult::Completed { warnings, .. } => CommandResult::Completed {
            output: (),
            warnings,
        },
        CommandResult::Failed { error } => CommandResult::Failed { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
        CommandResult::Cancelled => CommandResult::Cancelled,
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &WallsRectangle) -> CommandResult<WallsPlan> {
    plan_in(cx.doc, input)
}

/// The tool's preview; the exact wall and style values execute will write, without changing the document.
pub fn plan_in(doc: &Document, input: &WallsRectangle) -> CommandResult<WallsPlan> {
    match request_in(doc, input) {
        Ok((wall, adopted)) => match checked_plan(doc, input, &wall) {
            CommandResult::Completed {
                mut output,
                warnings,
            } => {
                output.adopted = adopted;
                CommandResult::Completed { output, warnings }
            }
            other => other,
        },
        Err(error) => CommandResult::Failed { error: *error },
    }
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: WallsRectangle,
) -> CommandResult<WallsCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_WALLS_RECTANGLE,
        kentos_commands::CAD_WALLS_RECTANGLE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: WallsRectangle,
) -> CommandResult<WallsCreated> {
    let (wall, adopted) = match request_in(cx.doc, &input) {
        Ok(w) => w,
        Err(error) => return CommandResult::Failed { error: *error },
    };
    match checked_plan(cx.doc, &input, &wall) {
        CommandResult::Completed { .. } => match public_paths(walls_create::execute(cx, wall)) {
            CommandResult::Completed {
                mut output,
                warnings,
            } => {
                output.adopted = adopted;
                CommandResult::Completed { output, warnings }
            }
            other => other,
        },
        CommandResult::Failed { error } => CommandResult::Failed { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
        CommandResult::Cancelled => CommandResult::Cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exact_size_boundary_and_one_ulp_of_clearance_are_distinct_even_after_rotation_at_large_coordinates()
     {
        let mut input: WallsRectangle = serde_json::from_value(json!({
            "layerId":"a", "frame":{"kind":"size","corner":{"x":500000.0,"y":4400000.0},
                "length":4.0,"width":3.0,"toward":{"x":500010.0,"y":4400010.0}},
            "rotation":std::f64::consts::FRAC_PI_4,"reference":"outer","width":1.5
        }))
        .unwrap();
        assert_eq!(dimensions(&input), (4.0, 3.0));
        assert!(interior_is_empty(&input, 1.5));
        if let RectangleFrame::Size { width, .. } = &mut input.frame {
            *width = f64::from_bits(3.0_f64.to_bits() + 1);
        }
        assert!(
            !interior_is_empty(&input, 1.5),
            "positive source clearance must not be rounded to zero"
        );
        // This tests the source-dimension guard, not a promise that arbitrary sub-ULP global
        // geometry is drawable. The existing wall's geometric checks remain authoritative too.
    }
}
