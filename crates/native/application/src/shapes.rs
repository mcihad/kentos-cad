//! `cad.rectangle.create` and `cad.polygon.regular` v1 (docs/adr/0290): a rectangle by two corners, a corner with a
//! size or an area, or an edge with a width, its corners sharp, rounded or cut; a regular polygon by its centre and
//! first corner or first edge's middle, or by an edge. The ring is the shared core's (`geom::shapes`,
//! `ops::fillet::corners_of_ring`), here only; the tools draw their previews from [`rectangle_ring`] and
//! [`regular_ring`] and write through the commands. The polygon is written as `cad.polygon.create` writes it, its
//! checks after the shape's own. The shared cases are `fixtures/commands/v1/cad.rectangle.create.json` and
//! `cad.polygon.regular.json` (`scripts/fixtures/shape_command_cases.py`).

use kentos_contracts::{
    CommandResult, PolygonCreate, PolygonCreated, PolygonPlan, RectangleCorners, RectangleCreate,
    RectangleFrame, RegularFrame, RegularPolygonCreate, Vec2 as Wire,
};
use kentos_geometry_core::geom::shapes::{
    rect_from_corners, rect_from_edge, rect_from_size, rect_side_for_area, regular_polygon,
    regular_polygon_on_edge,
};
use kentos_geometry_core::ops::fillet::{CornerOp, CornerResult, corners_of_ring};
use kentos_geometry_core::vec2::Vec2;

use crate::checks::{self, Stop, error};
use crate::{ExecutionContext, codes, polygon};

/// A ring's corners and its bulges (none: every edge straight).
pub type Ring = (Vec<Vec2>, Option<Vec<f64>>);

fn v(p: Wire) -> Vec2 {
    Vec2::new(p.x, p.y)
}

fn w(p: Vec2) -> Wire {
    Wire { x: p.x, y: p.y }
}

fn refuse(code: &str, message: &str, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn length(value: f64, what: &str, path: &str) -> Result<(), Stop> {
    checks::finite(value, what, "Sonlu bir sayı verin.", path)
}

fn positive(value: f64, message: &str, path: &str) -> Result<(), Stop> {
    if value > 0.0 {
        Ok(())
    } else {
        Err(refuse(codes::INVALID_DISTANCE, message, path))
    }
}

const DEGENERATE: &str = "Dikdörtgenin bir kenarı sıfır uzunlukta: köşeler çakışıyor ya da dönüşün doğrultusunda bir doğru üzerinde. Başka bir köşe verin.";

/// The rectangle's ring as the command writes it, or why not (the checks of the shape, in the contract's order).
fn rectangle_ring_in(input: &RectangleCreate) -> Result<Ring, Stop> {
    let rotation = input.rotation.unwrap_or(0.0);
    // Finite numbers first, the frame's then the rotation and the corners'.
    match &input.frame {
        RectangleFrame::Corners { a, b } => {
            checks::point(*a, "İlk köşenin", "frame.a")?;
            checks::point(*b, "Karşı köşenin", "frame.b")?;
        }
        RectangleFrame::Size {
            corner,
            length: l,
            width,
            toward,
        } => {
            checks::point(*corner, "Köşenin", "frame.corner")?;
            length(*l, "Uzunluk", "frame.length")?;
            length(*width, "Genişlik", "frame.width")?;
            checks::point(*toward, "Yönün", "frame.toward")?;
        }
        RectangleFrame::Area {
            corner,
            area,
            length: l,
            toward,
        } => {
            checks::point(*corner, "Köşenin", "frame.corner")?;
            length(*area, "Alan", "frame.area")?;
            length(*l, "Uzunluk", "frame.length")?;
            checks::point(*toward, "Yönün", "frame.toward")?;
        }
        RectangleFrame::Edge { a, b, width } => {
            checks::point(*a, "Kenarın başının", "frame.a")?;
            checks::point(*b, "Kenarın sonunun", "frame.b")?;
            length(*width, "Genişlik", "frame.width")?;
        }
    }
    length(rotation, "Dönüş", "rotation")?;
    match &input.corners {
        Some(RectangleCorners::Fillet { radius }) => {
            length(*radius, "Köşe yarıçapı", "corners.radius")?
        }
        Some(RectangleCorners::Chamfer { distance }) => {
            length(*distance, "Pah uzaklığı", "corners.distance")?
        }
        _ => {}
    }
    // Then what must be above zero.
    match &input.frame {
        RectangleFrame::Size {
            length: l, width, ..
        } => {
            positive(*l, "Uzunluk sıfırdan büyük olmalı.", "frame.length")?;
            positive(*width, "Genişlik sıfırdan büyük olmalı.", "frame.width")?;
        }
        RectangleFrame::Area {
            area, length: l, ..
        } => {
            positive(*area, "Alan sıfırdan büyük olmalı.", "frame.area")?;
            positive(*l, "Uzunluk sıfırdan büyük olmalı.", "frame.length")?;
        }
        RectangleFrame::Edge { width, .. } if *width == 0.0 => {
            return Err(refuse(
                codes::INVALID_DISTANCE,
                "Genişlik sıfır olamaz: kenarın solu için artı, sağı için eksi bir genişlik verin.",
                "frame.width",
            ));
        }
        _ => {}
    }
    match &input.corners {
        Some(RectangleCorners::Fillet { radius }) => {
            positive(
                *radius,
                "Köşe yarıçapı sıfırdan büyük olmalı; keskin köşe için corners'ı vermeyin.",
                "corners.radius",
            )?;
        }
        Some(RectangleCorners::Chamfer { distance }) => {
            positive(
                *distance,
                "Pah uzaklığı sıfırdan büyük olmalı; keskin köşe için corners'ı vermeyin.",
                "corners.distance",
            )?;
        }
        _ => {}
    }
    let ring = match &input.frame {
        RectangleFrame::Corners { a, b } => rect_from_corners(v(*a), v(*b), rotation),
        RectangleFrame::Size {
            corner,
            length,
            width,
            toward,
        } => rect_from_size(v(*corner), *length, *width, rotation, v(*toward)),
        RectangleFrame::Area {
            corner,
            area,
            length,
            toward,
        } => rect_side_for_area(*area, *length)
            .and_then(|width| rect_from_size(v(*corner), *length, width, rotation, v(*toward))),
        RectangleFrame::Edge { a, b, width } => rect_from_edge(v(*a), v(*b), *width),
    }
    .ok_or_else(|| refuse(codes::DEGENERATE_SHAPE, DEGENERATE, "frame"))?;
    let op = match &input.corners {
        None | Some(RectangleCorners::Sharp) => return Ok((ring, None)),
        Some(RectangleCorners::Fillet { radius }) => CornerOp::Radius(*radius),
        Some(RectangleCorners::Chamfer { distance }) => CornerOp::Chamfer(*distance, *distance),
    };
    match corners_of_ring(&ring, &op) {
        Ok(CornerResult::Path(path)) => Ok((path.pts, path.bulges)),
        Ok(CornerResult::Error(why)) | Err(why) => Err(Stop::failed(error(
            codes::INVALID_CORNERS,
            format!("{why} Daha küçük bir yarıçap ya da pah verin."),
            Some("corners".into()),
        ))),
    }
}

/// The regular polygon's ring as the command writes it, or why not.
fn regular_ring_in(input: &RegularPolygonCreate) -> Result<Vec<Vec2>, Stop> {
    let (a, b, which) = match &input.frame {
        RegularFrame::Inscribed { center, corner } => (
            center,
            corner,
            ("Merkezin", "İlk köşenin", "center", "corner"),
        ),
        RegularFrame::Circumscribed { center, middle } => (
            center,
            middle,
            ("Merkezin", "İlk kenarın ortasının", "center", "middle"),
        ),
        RegularFrame::Edge { a, b } => (a, b, ("Kenarın başının", "Kenarın sonunun", "a", "b")),
    };
    checks::point(*a, which.0, &format!("frame.{}", which.2))?;
    checks::point(*b, which.1, &format!("frame.{}", which.3))?;
    if !(3..=kentos_contracts::MOST_SIDES).contains(&input.sides) {
        return Err(refuse(
            codes::INVALID_COUNT,
            "Kenar sayısı 3 ile 1024 arasında olmalı.",
            "sides",
        ));
    }
    let n = f64::from(input.sides);
    let ring = match &input.frame {
        RegularFrame::Inscribed { center, corner } => {
            regular_polygon(v(*center), n, v(*corner), "inscribed")
        }
        RegularFrame::Circumscribed { center, middle } => {
            regular_polygon(v(*center), n, v(*middle), "circumscribed")
        }
        RegularFrame::Edge { a, b } => regular_polygon_on_edge(v(*a), v(*b), n),
    };
    ring.ok_or_else(|| {
        refuse(
            codes::DEGENERATE_SHAPE,
            "Çokgenin iki noktası çakışıyor: merkezden uzakta bir köşe ya da kenar ortası, ya da uzunluğu olan bir kenar verin.",
            "frame",
        )
    })
}

/// The rectangle's ring as the command writes it, or the command's refusal: the tools' preview.
pub fn rectangle_ring(
    input: &RectangleCreate,
) -> Result<Ring, Box<kentos_contracts::CommandError>> {
    rectangle_ring_in(input).map_err(|stop| Box::new(stop.into_error()))
}

/// The regular polygon's ring as the command writes it, or the command's refusal: the tool's preview.
pub fn regular_ring(
    input: &RegularPolygonCreate,
) -> Result<Vec<Vec2>, Box<kentos_contracts::CommandError>> {
    regular_ring_in(input).map_err(|stop| Box::new(stop.into_error()))
}

fn rectangle_polygon(input: &RectangleCreate, (pts, bulges): Ring) -> PolygonCreate {
    PolygonCreate {
        layer_id: input.layer_id.clone(),
        pts: pts.into_iter().map(w).collect(),
        bulges,
        holes: None,
        color: input.color.clone(),
        line_weight: input.line_weight,
        line_type: input.line_type,
        wipeout: None,
        attrs: input.attrs.clone(),
        label: input.label.clone(),
        symbol: input.symbol.clone(),
        expected_revision: input.expected_revision.clone(),
    }
}

fn regular_polygon_input(input: &RegularPolygonCreate, pts: Vec<Vec2>) -> PolygonCreate {
    PolygonCreate {
        layer_id: input.layer_id.clone(),
        pts: pts.into_iter().map(w).collect(),
        bulges: None,
        holes: None,
        color: input.color.clone(),
        line_weight: input.line_weight,
        line_type: input.line_type,
        wipeout: None,
        attrs: input.attrs.clone(),
        label: input.label.clone(),
        symbol: input.symbol.clone(),
        expected_revision: input.expected_revision.clone(),
    }
}

/// The rectangle's checks, then the polygon's.
pub fn validate_rectangle(cx: &ExecutionContext<'_>, input: &RectangleCreate) -> CommandResult<()> {
    match rectangle_ring_in(input) {
        Ok(ring) => polygon::validate(cx, &rectangle_polygon(input, ring)),
        Err(stop) => stop.into(),
    }
}

/// The rectangle as it would be written.
pub fn plan_rectangle(
    cx: &ExecutionContext<'_>,
    input: &RectangleCreate,
) -> CommandResult<PolygonPlan> {
    match rectangle_ring_in(input) {
        Ok(ring) => polygon::plan(cx, &rectangle_polygon(input, ring)),
        Err(stop) => stop.into(),
    }
}

/// Writes the rectangle, one undo step.
pub fn execute_rectangle(
    cx: &mut ExecutionContext<'_>,
    input: RectangleCreate,
) -> CommandResult<PolygonCreated> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_RECTANGLE_CREATE,
        kentos_contracts::CAD_RECTANGLE_CREATE_VERSION,
        input,
        |cx, input| match rectangle_ring_in(&input) {
            Ok(ring) => polygon::write(cx, rectangle_polygon(&input, ring)),
            Err(stop) => stop.into(),
        },
    )
}

/// The regular polygon's checks, then the polygon's.
pub fn validate_regular(
    cx: &ExecutionContext<'_>,
    input: &RegularPolygonCreate,
) -> CommandResult<()> {
    match regular_ring_in(input) {
        Ok(pts) => polygon::validate(cx, &regular_polygon_input(input, pts)),
        Err(stop) => stop.into(),
    }
}

/// The regular polygon as it would be written.
pub fn plan_regular(
    cx: &ExecutionContext<'_>,
    input: &RegularPolygonCreate,
) -> CommandResult<PolygonPlan> {
    match regular_ring_in(input) {
        Ok(pts) => polygon::plan(cx, &regular_polygon_input(input, pts)),
        Err(stop) => stop.into(),
    }
}

/// Writes the regular polygon, one undo step.
pub fn execute_regular(
    cx: &mut ExecutionContext<'_>,
    input: RegularPolygonCreate,
) -> CommandResult<PolygonCreated> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_POLYGON_REGULAR,
        kentos_contracts::CAD_POLYGON_REGULAR_VERSION,
        input,
        |cx, input| match regular_ring_in(&input) {
            Ok(pts) => polygon::write(cx, regular_polygon_input(&input, pts)),
            Err(stop) => stop.into(),
        },
    )
}
