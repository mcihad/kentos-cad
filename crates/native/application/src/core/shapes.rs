//! `cad.rectangle.create`, `cad.polygon.regular` (docs/adr/0290) and `cad.sector.create` v1 (docs/adr/0622 §9): a rectangle by two corners, a corner with a
//! size or an area, or an edge with a width, its corners sharp, rounded or cut; a regular polygon by its centre and
//! first corner or first edge's middle, or by an edge. The ring is the shared core's (`geom::shapes`,
//! `ops::fillet::corners_of_ring`), here only; the tools draw their previews from [`rectangle_ring`] and
//! [`regular_ring`] and write through the commands. The polygon is written as `cad.polygon.create` writes it, its
//! checks after the shape's own. The shared cases are `fixtures/commands/v1/cad.rectangle.create.json` and
//! `cad.polygon.regular.json` (`scripts/fixtures/shape_command_cases.py`). `cad.revcloud.create` (docs/adr/0622 §9)
//! makes a revision cloud round a rectangle, a polygon's corners or a closed object (`cloud_of`).

use kentos_commands::{
    CommandResult, PolygonCreate, PolygonCreated, PolygonPlan, RectangleCorners, RectangleCreate,
    RectangleFrame, RegularFrame, RegularPolygonCreate, RevcloudCreate, RevcloudFrame, SectorCreate, Vec2 as Wire,
};
use kentos_geometry_core::geom::shapes::{
    closed_outline, cloud_arcs, cloud_of, rect_from_corners, rect_from_edge, rect_from_size,
    rect_side_for_area, regular_polygon, regular_polygon_on_edge,
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
    if !(3..=kentos_commands::MOST_SIDES).contains(&input.sides) {
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
) -> Result<Ring, Box<kentos_commands::CommandError>> {
    rectangle_ring_in(input).map_err(|stop| Box::new(stop.into_error()))
}

/// The regular polygon's ring as the command writes it, or the command's refusal: the tool's preview.
pub fn regular_ring(
    input: &RegularPolygonCreate,
) -> Result<Vec<Vec2>, Box<kentos_commands::CommandError>> {
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
        rules: input.rules.clone(),
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
        rules: input.rules.clone(),
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
        kentos_commands::CAD_RECTANGLE_CREATE,
        kentos_commands::CAD_RECTANGLE_CREATE_VERSION,
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
        kentos_commands::CAD_POLYGON_REGULAR,
        kentos_commands::CAD_POLYGON_REGULAR_VERSION,
        input,
        |cx, input| match regular_ring_in(&input) {
            Ok(pts) => polygon::write(cx, regular_polygon_input(&input, pts)),
            Err(stop) => stop.into(),
        },
    )
}

// ── cad.sector.create ──────────────────────────────────────────────────────────────────────────────────────────

const SECTOR_DEGENERATE: &str = "Dilimin süpürmesi yok: bitiş açısı başlangıçla aynı ya da tam tur. Başka bir bitiş açısı verin.";

/// The slice's ring as the command writes it (`construct::sector`), or why not.
fn sector_ring_in(input: &SectorCreate) -> Result<Ring, Stop> {
    checks::point(input.c, "Merkezin", "c")?;
    length(input.r, "Yarıçap", "r")?;
    length(input.a0, "Başlangıç açısı", "a0")?;
    length(input.a1, "Bitiş açısı", "a1")?;
    // The core's own bound for a radius (a nanometre), so a radius it would not draw is said as a radius.
    if input.r <= 1e-9 {
        return Err(refuse(codes::INVALID_DISTANCE, "Yarıçap sıfırdan büyük olmalı.", "r"));
    }
    let path = kentos_geometry_core::tools::construct::sector(v(input.c), input.r, input.a0, input.a1)
        .ok_or_else(|| refuse(codes::DEGENERATE_SHAPE, SECTOR_DEGENERATE, "a1"))?;
    Ok((path.pts, path.bulges))
}

/// The slice's ring as the command writes it, or the command's refusal: the tool's preview.
pub fn sector_ring(input: &SectorCreate) -> Result<Ring, Box<kentos_commands::CommandError>> {
    sector_ring_in(input).map_err(|stop| Box::new(stop.into_error()))
}

fn sector_polygon(input: &SectorCreate, (pts, bulges): Ring) -> PolygonCreate {
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
        rules: input.rules.clone(),
    }
}

/// The slice's checks, then the polygon's.
pub fn validate_sector(cx: &ExecutionContext<'_>, input: &SectorCreate) -> CommandResult<()> {
    match sector_ring_in(input) {
        Ok(ring) => polygon::validate(cx, &sector_polygon(input, ring)),
        Err(stop) => stop.into(),
    }
}

/// The slice as it would be written.
pub fn plan_sector(cx: &ExecutionContext<'_>, input: &SectorCreate) -> CommandResult<PolygonPlan> {
    match sector_ring_in(input) {
        Ok(ring) => polygon::plan(cx, &sector_polygon(input, ring)),
        Err(stop) => stop.into(),
    }
}

/// Writes the slice, one undo step.
pub fn execute_sector(cx: &mut ExecutionContext<'_>, input: SectorCreate) -> CommandResult<PolygonCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SECTOR_CREATE,
        kentos_commands::CAD_SECTOR_CREATE_VERSION,
        input,
        |cx, input| match sector_ring_in(&input) {
            Ok(ring) => polygon::write(cx, sector_polygon(&input, ring)),
            Err(stop) => stop.into(),
        },
    )
}

const CLOUD_RECT_DEGENERATE: &str = "Dikdörtgenin bir kenarı sıfır uzunlukta: iki köşe aynı doğu ya da kuzey çizgisinde. Başka bir karşı köşe verin.";
const CLOUD_FLAT: &str = "Bulutun köşeleri bir doğru üzerinde: alanı yok. Başka köşeler verin.";
const CLOUD_FEW: &str = "Bulutun en az üç köşesi olmalı.";
const CLOUD_OPEN: &str = "Nesne kapalı değil: bulut alanın, kapalı çoklu çizginin, dairenin, kapalı eğrinin ya da tam elipsin çevresine çizilir.";

/// The ring a cloud goes round, counter-clockwise or not: the rectangle's, the corners (repeats dropped), the object's
/// outer outline. `doc` is none for a preview, which has no object.
fn cloud_frame(doc: Option<&kentos_domain::Document>, input: &RevcloudCreate) -> Result<Vec<Vec2>, Stop> {
    match &input.frame {
        RevcloudFrame::Rectangle { a, b } => {
            checks::point(*a, "İlk köşenin", "frame.a")?;
            checks::point(*b, "Karşı köşenin", "frame.b")?;
            length(input.arc, "Yay boyu", "arc")?;
            positive(input.arc, "Yay boyu sıfırdan büyük olmalı.", "arc")?;
            if (b.x - a.x).abs() <= 1e-9 || (b.y - a.y).abs() <= 1e-9 {
                return Err(refuse(codes::DEGENERATE_SHAPE, CLOUD_RECT_DEGENERATE, "frame"));
            }
            Ok(vec![v(*a), Vec2::new(b.x, a.y), v(*b), Vec2::new(a.x, b.y)])
        }
        RevcloudFrame::Polygon { pts } => {
            for (i, p) in pts.iter().enumerate() {
                checks::point(*p, "Köşenin", &format!("frame.pts[{i}]"))?;
            }
            length(input.arc, "Yay boyu", "arc")?;
            positive(input.arc, "Yay boyu sıfırdan büyük olmalı.", "arc")?;
            let mut ring: Vec<Vec2> = Vec::with_capacity(pts.len());
            for p in pts.iter().copied().map(v) {
                if ring.last().is_none_or(|q| kentos_geometry_core::geometry::dist(*q, p) > 1e-9) {
                    ring.push(p);
                }
            }
            while ring.len() > 1 && kentos_geometry_core::geometry::dist(ring[0], ring[ring.len() - 1]) <= 1e-9 {
                ring.pop();
            }
            if ring.len() < 3 {
                return Err(refuse(codes::TOO_FEW_CORNERS, CLOUD_FEW, "frame.pts"));
            }
            if kentos_geometry_core::geometry::signed_area(&ring).abs() <= 1e-9 {
                return Err(refuse(codes::DEGENERATE_SHAPE, CLOUD_FLAT, "frame.pts"));
            }
            Ok(ring)
        }
        RevcloudFrame::Object { uid } => {
            length(input.arc, "Yay boyu", "arc")?;
            positive(input.arc, "Yay boyu sıfırdan büyük olmalı.", "arc")?;
            let Some(doc) = doc else {
                return Err(refuse(codes::ENTITY_NOT_FOUND, "Önizlemede nesne yok.", "frame.uid"));
            };
            let entity = kentos_domain::Uuid::parse_str(uid)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .and_then(|slot| doc.get(slot))
                .ok_or_else(|| {
                    Stop::failed(error(
                        codes::ENTITY_NOT_FOUND,
                        format!(
                            "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                        ),
                        Some("frame.uid".into()),
                    ))
                })?;
            let shape = crate::geometry_store::record(entity).3;
            closed_outline(&shape, 64.0)
                .filter(|ring| ring.len() >= 3)
                .ok_or_else(|| refuse(codes::NOT_CLOSED, CLOUD_OPEN, "frame.uid"))
        }
    }
}

/// The cloud as the command writes it, or why not.
pub(crate) fn cloud_ring_in(doc: Option<&kentos_domain::Document>, input: &RevcloudCreate) -> Result<Ring, Stop> {
    let ring = cloud_frame(doc, input)?;
    if cloud_arcs(&ring, input.arc) > kentos_commands::MOST_CLOUD_ARCS as f64 {
        return Err(refuse(
            codes::TOO_MANY_POINTS,
            "Bulut 10 000'den çok yay olurdu; daha büyük bir yay boyu verin.",
            "arc",
        ));
    }
    let cloud = cloud_of(&ring, input.arc)
        .ok_or_else(|| refuse(codes::DEGENERATE_SHAPE, CLOUD_FLAT, "frame"))?;
    Ok((cloud.pts, Some(cloud.bulges)))
}

/// The cloud as the command makes it, or the command's refusal: the tool's preview and what it writes. An object's
/// needs the drawing; with none (a preview) it is refused.
pub fn revcloud_ring(
    doc: Option<&kentos_domain::Document>,
    input: &RevcloudCreate,
) -> Result<Ring, Box<kentos_commands::CommandError>> {
    cloud_ring_in(doc, input).map_err(|stop| Box::new(stop.into_error()))
}

pub(crate) fn revcloud_polygon(input: &RevcloudCreate, (pts, bulges): Ring) -> PolygonCreate {
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
        rules: input.rules.clone(),
    }
}

/// The cloud's checks, then the polygon's.
pub fn validate_revcloud(cx: &ExecutionContext<'_>, input: &RevcloudCreate) -> CommandResult<()> {
    match cloud_ring_in(Some(&*cx.doc), input) {
        Ok(ring) => polygon::validate(cx, &revcloud_polygon(input, ring)),
        Err(stop) => stop.into(),
    }
}

/// The cloud as it would be written.
pub fn plan_revcloud(cx: &ExecutionContext<'_>, input: &RevcloudCreate) -> CommandResult<PolygonPlan> {
    match cloud_ring_in(Some(&*cx.doc), input) {
        Ok(ring) => polygon::plan(cx, &revcloud_polygon(input, ring)),
        Err(stop) => stop.into(),
    }
}

/// Writes the cloud, one undo step.
pub fn execute_revcloud(cx: &mut ExecutionContext<'_>, input: RevcloudCreate) -> CommandResult<PolygonCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_REVCLOUD_CREATE,
        kentos_commands::CAD_REVCLOUD_CREATE_VERSION,
        input,
        |cx, input| match cloud_ring_in(Some(&*cx.doc), &input) {
            Ok(ring) => polygon::write(cx, revcloud_polygon(&input, ring)),
            Err(stop) => stop.into(),
        },
    )
}
