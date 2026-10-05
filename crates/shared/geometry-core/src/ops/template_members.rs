//! What a group template's members make from the drawn shape (docs/adr/0176
//! §5): its parallels at a distance, on one side or both (Ötele's rule: the
//! side by the drawing's direction on an open shape, inside or outside on a
//! closed one), and the point at its centroid (the label's place,
//! docs/adr/0175). The same shape needs no work; the points at its vertices
//! are `ops::vertex_points`'.

use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::entity::{Entity, Shape, entity_anchor, is_multi_part};
use crate::geom::bulge::{bulge_ring_area, has_bulges};
use crate::geom::offset::{OffsetResult, offset_bulge_path, offset_path};
use crate::op;
use crate::ops::offset::path_shape;

/// An offset member's side (`MEMBER_SIDES`): left or right of the drawing's
/// direction on an open shape, inside or outside on a closed one, or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Inside,
    Outside,
    Both,
}

impl Side {
    /// The side by its name in a template (`left`, `right`, `inside`, `outside`, `both`).
    pub fn from_name(name: &str) -> Option<Side> {
        Some(match name {
            "left" => Side::Left,
            "right" => Side::Right,
            "inside" => Side::Inside,
            "outside" => Side::Outside,
            "both" => Side::Both,
            _ => return None,
        })
    }
}

/// The parallels an offset member makes from the drawn `shape` (a line, a
/// polyline or an area of one part) at `distance`: one on its side, two for
/// Both (the left or inside one first). A side that does not fit the shape,
/// a distance not above zero or an offset that gives no shape is said.
pub fn member_offsets(shape: &Shape, distance: f64, side: Side) -> Result<Vec<Shape>, String> {
    if !(distance > 0.0) {
        return Err("Öteleme mesafesi sıfırdan büyük olmalı.".into());
    }
    if is_multi_part(shape) {
        return Err("Çok parçalı nesne grup şablonunun üyesiyle ötelenmez.".into());
    }
    let closed = match shape {
        Shape::Polygon { .. } => true,
        Shape::Line { .. } | Shape::Polyline { .. } => false,
        _ => {
            return Err(
                "Grup şablonunun ötelemesi yalnız çizgi, çoklu çizgi ve kapalı alanda olur.".into(),
            );
        }
    };
    // Left of the drawing's direction is inside a counter-clockwise ring.
    let inside = match shape {
        Shape::Polygon { pts, bulges, .. } if bulge_ring_area(pts, bulges.as_deref()) < 0.0 => -1.0,
        _ => 1.0,
    };
    let signs: &[f64] = match (side, closed) {
        (Side::Left, false) => &[1.0],
        (Side::Right, false) => &[-1.0],
        (Side::Both, false) => &[1.0, -1.0],
        (Side::Inside, true) => &[1.0],
        (Side::Outside, true) => &[-1.0],
        (Side::Both, true) => &[1.0, -1.0],
        (_, true) => return Err("Kapalı şekil içe, dışa ya da iki yana ötelenir.".into()),
        (_, false) => return Err("Açık şekil sola, sağa ya da iki yana ötelenir.".into()),
    };
    let factor = if closed { inside } else { 1.0 };
    signs
        .iter()
        .map(|s| offset_signed(shape, s * factor * distance))
        .collect()
}

/// The parallel at `d`: left of the drawing's direction when above zero,
/// right when below (`geom::offset`'s sign).
fn offset_signed(shape: &Shape, d: f64) -> Result<Shape, String> {
    let none = || "Öteleme sonucu geçerli bir şekil oluşmadı.".to_owned();
    match shape {
        Shape::Line { a, b } => {
            let out = offset_path(&[*a, *b], d, false);
            match out.as_slice() {
                [a, b] => Ok(Shape::Line { a: *a, b: *b }),
                _ => Err(none()),
            }
        }
        Shape::Polyline { pts, bulges, .. } | Shape::Polygon { pts, bulges, .. } => {
            let closed = matches!(shape, Shape::Polygon { .. });
            if let Some(bs) = bulges.as_deref()
                && has_bulges(Some(bs))
            {
                return match offset_bulge_path(pts, bs, d, closed) {
                    OffsetResult::Path { pts, bulges } => Ok(path_shape(closed, pts, Some(bulges))),
                    OffsetResult::Error { error } => Err(error),
                };
            }
            let out = offset_path(pts, d, closed);
            if out.len() < if closed { 3 } else { 2 } {
                return Err(none());
            }
            Ok(path_shape(closed, out, None))
        }
        _ => Err(none()),
    }
}

/// An offset member's parallels, or why there are none.
struct Offsets(Result<Vec<Shape>, String>);

impl ToJson for Offsets {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok(shapes) => {
                let geometries: Vec<Entity> = shapes.iter().cloned().map(Entity::new).collect();
                field(out, &mut first, "geometries", &geometries);
            }
            Err(e) => field(out, &mut first, "error", e),
        }
        out.push('}');
    }
}

pub(crate) static OPS: &[Op] = &[
    op!(
        "templateMemberOffsets",
        |e: Entity, distance: f64, side: String| {
            Offsets(match Side::from_name(&side) {
                Some(side) => member_offsets(&e.shape, distance, side),
                None => Err(format!("Bilinmeyen öteleme yanı “{side}”.")),
            })
        }
    ),
    // An Ağırlık merkezine member's point: the label's place.
    op!("templateMemberCentroid", |e: Entity| entity_anchor(
        &e.shape
    )),
];
