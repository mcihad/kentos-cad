//! Parallel copy of an entity (`apps/web/src/model/ops/offset.ts`): circles and arcs
//! grow or shrink about their centre, paths are offset with mitred corners,
//! ellipses and construction lines by their own rules. An area's holes go with
//! it, to the other side; a fit-point or NURBS curve is offset by its chords
//! (docs/adr/0293). [`offset_noted`] also says whether the copy is approximate
//! and where it crosses itself; [`collapses`] tells a radius collapsing.

use crate::api::Op;
use crate::entity::{
    Entity, Shape, area_parts, inside_polygon, is_closed_path, is_multi_part, join_parts,
    leader_refused,
};
use crate::geom::arrangement::{Area, Ring};
use crate::geom::bulge::{bulge_ring_area, has_bulges};
use crate::geom::intersect::{Edge, closest_on_edge};
use crate::geom::offset::{OffsetResult, offset_bulge_path, offset_path, side_of};
use crate::geom::curve_outline::{CURVE_TOL, spline_outline};
use crate::geom::ellipse::major_length;
use crate::geom::intersect::intersect_edges;
use crate::geom::region::{subtract_areas, union_areas};
use crate::geometry::point_in_polygon;
use crate::jsmath::{js_hypot, js_min};
use crate::op;
use crate::ops::curve_cuts::{
    Geometry, construction_of, ellipse_of, offset_construction, offset_ellipse,
};
use crate::ops::edges::entity_edges;
use crate::vec2::Vec2;

/// A path of `like`'s kind: an area's ring, or a polyline open or closed as `like` is
/// (docs/adr/0258 §1).
pub fn path_like(like: &Shape, pts: Vec<Vec2>, bulges: Option<Vec<f64>>) -> Shape {
    match like {
        Shape::Polygon { .. } => path_shape(true, pts, bulges),
        _ => Shape::Polyline {
            pts,
            bulges,
            holes: None,
            parts: None,
            closed: matches!(like, Shape::Polyline { closed: true, .. }),
            widths: None,
        },
    }
}

pub(crate) fn path_shape(closed: bool, pts: Vec<Vec2>, bulges: Option<Vec<f64>>) -> Shape {
    if closed {
        Shape::Polygon {
            pts,
            bulges,
            holes: None,
            parts: None,
        }
    } else {
        Shape::Polyline {
            pts,
            bulges,
            holes: None,
            parts: None,
            closed: false,
            widths: None,
        }
    }
}

/// A multi-part area's parallel (docs/adr/0143): every part's ring to the
/// side `through` is on of the whole area (inside it: inward, outside:
/// outward), as one part's ring is offset, its holes to the other side
/// (docs/adr/0293); rings that meet are joined, a part that closes up inward
/// is left out.
fn offset_parts(e: &Shape, distance: f64, through: Vec2) -> Geometry {
    let inward = inside_polygon(e, through);
    let mut areas = Vec::new();
    for part in area_parts(e).iter() {
        let Shape::Polygon {
            pts, bulges, holes, ..
        } = part
        else {
            continue;
        };
        // Left of travel is inside a counter-clockwise ring.
        let ccw = bulge_ring_area(pts, bulges.as_deref()) > 0.0;
        let d = if inward == ccw { distance } else { -distance };
        let ring = match bulges.as_deref() {
            Some(bs) if has_bulges(Some(bs)) => match offset_bulge_path(pts, bs, d, true) {
                OffsetResult::Path { pts, bulges } => Ring {
                    pts,
                    bulges: Some(bulges),
                },
                OffsetResult::Error { .. } => continue,
            },
            _ => {
                let out = offset_path(pts, d, true);
                if out.len() < 3 {
                    continue;
                }
                Ring {
                    pts: out,
                    bulges: None,
                }
            }
        };
        // Its holes to the other side (docs/adr/0293); one closing up is gone.
        let holes = holes
            .iter()
            .flatten()
            .filter_map(|h| {
                let hccw = bulge_ring_area(&h.pts, h.bulges.as_deref()) > 0.0;
                offset_ring(h, if inward == hccw { -distance } else { distance })
            })
            .collect();
        areas.push(Area { outer: ring, holes });
    }
    let shapes: Vec<Shape> = union_areas(&areas)
        .into_iter()
        .map(|a| Shape::Polygon {
            pts: a.outer.pts,
            bulges: a.outer.bulges,
            holes: (!a.holes.is_empty()).then_some(a.holes),
            parts: None,
        })
        .collect();
    match join_parts(&shapes) {
        Some(shape) => Geometry::Ok(Entity::new(shape)),
        None => Geometry::Error("Öteleme sonucu geçerli bir şekil oluşmadı.".into()),
    }
}

/// A multi-part polyline's parallel (docs/adr/0174): every part offset by
/// the same distance to the same side of its direction (left or right of
/// travel, as QGIS and ArcGIS offset a multi-part line), the side `through`
/// is on of the nearest edge.
fn offset_path_parts(e: &Shape, distance: f64, through: Vec2) -> Geometry {
    let d = f64::from(bulged_side(e, through)) * distance;
    let mut shapes = Vec::new();
    for part in area_parts(e).iter() {
        let Shape::Polyline { pts, bulges, .. } = part else {
            continue;
        };
        let shape = match bulges.as_deref() {
            Some(bs) if has_bulges(Some(bs)) => match offset_bulge_path(pts, bs, d, false) {
                OffsetResult::Path { pts, bulges } => path_shape(false, pts, Some(bulges)),
                OffsetResult::Error { error } => return Geometry::Error(error),
            },
            _ => {
                let out = offset_path(pts, d, false);
                if out.len() < 2 {
                    return Geometry::Error("Öteleme sonucu geçerli bir şekil oluşmadı.".into());
                }
                path_shape(false, out, None)
            }
        };
        shapes.push(shape);
    }
    match join_parts(&shapes) {
        Some(shape) => Geometry::Ok(Entity::new(shape)),
        None => Geometry::Error("Öteleme sonucu geçerli bir şekil oluşmadı.".into()),
    }
}

/// Parallel copy at `distance`, on the side of `through`.
pub fn offset_entity(e: &Shape, distance: f64, through: Vec2) -> Geometry {
    if !(distance > 0.0) {
        return Geometry::Error("Öteleme mesafesi sıfırdan büyük olmalı.".into());
    }
    if let Shape::Polyline { .. } = e
        && is_multi_part(e)
    {
        return offset_path_parts(e, distance, through);
    }
    if is_multi_part(e) {
        return offset_parts(e, distance, through);
    }
    match e {
        Shape::Line { a, b } => {
            let side = side_of(&[*a, *b], false, through);
            let pts = offset_path(&[*a, *b], f64::from(side) * distance, false);
            match pts[..] {
                [a, b] => Geometry::Ok(Entity::new(Shape::Line { a, b })),
                _ => Geometry::Error("Öteleme sonucu geçerli bir şekil oluşmadı.".into()),
            }
        }
        // An area's holes go with it, to the other side (docs/adr/0293).
        Shape::Polygon {
            holes: Some(holes), ..
        } if !holes.is_empty() => offset_region(e, distance, through),
        // A closed polyline's parallel is a closed ring as an area's, and a line still
        // (docs/adr/0258 §1).
        Shape::Polyline { pts, bulges, .. } | Shape::Polygon { pts, bulges, .. } => {
            let closed = is_closed_path(e);
            if let Some(bs) = bulges.as_deref()
                && has_bulges(Some(bs))
            {
                let d = f64::from(bulged_side(e, through)) * distance;
                return match offset_bulge_path(pts, bs, d, closed) {
                    OffsetResult::Path { pts, bulges } => {
                        Geometry::Ok(Entity::new(path_like(e, pts, Some(bulges))))
                    }
                    OffsetResult::Error { error } => Geometry::Error(error),
                };
            }
            let side = side_of(pts, closed, through);
            let out = offset_path(pts, f64::from(side) * distance, closed);
            // A ring offset inward past its width turns over: it closes up (docs/adr/0293).
            if closed && out.len() >= 3 && turned_over(pts, None, &out, None) {
                return Geometry::Error(REGION_CLOSES.into());
            }
            if out.len() >= if closed { 3 } else { 2 } {
                Geometry::Ok(Entity::new(path_like(e, out, None)))
            } else {
                Geometry::Error("Öteleme sonucu geçerli bir şekil oluşmadı.".into())
            }
        }
        Shape::Circle { c, r } | Shape::Arc { c, r, .. } => {
            let outside = js_hypot(through.x - c.x, through.y - c.y) > *r;
            let r = r + if outside { distance } else { -distance };
            if r <= 1e-9 {
                return Geometry::Error(
                    "Yarıçap sıfırın altına düşüyor; daha küçük bir mesafe girin.".into(),
                );
            }
            Geometry::Ok(Entity::new(match e {
                Shape::Arc { a0, a1, .. } => Shape::Arc {
                    c: *c,
                    r,
                    a0: *a0,
                    a1: *a1,
                },
                _ => Shape::Circle { c: *c, r },
            }))
        }
        Shape::Leader { .. } => Geometry::Error(leader_refused("ötelenemez")),
        _ => {
            if let Some(g) = ellipse_of(e) {
                return offset_ellipse(&g, distance, through);
            }
            if let Some(c) = construction_of(e) {
                return offset_construction(&c, distance, through);
            }
            Geometry::Error(
                "Yalnızca çizgi, çoklu çizgi, kapalı alan, daire, yay, elips ve yardımcı çizgiler ötelenebilir."
                    .into(),
            )
        }
    }
}

/// The messages of an offset whose radius collapses: an arc of a path, a circle or arc, an ellipse's
/// curvature, an area closing up inward (docs/adr/0293).
pub const COLLAPSES: [&str; 4] = [
    "Öteleme mesafesi bir yay parçasının yarıçapından büyük.",
    "Yarıçap sıfırın altına düşüyor; daha küçük bir mesafe girin.",
    "Mesafe elipsin en küçük eğrilik yarıçapından büyük; içe doğru öteleme bozulur.",
    REGION_CLOSES,
];

const REGION_CLOSES: &str = "Mesafe alanın genişliğinden büyük; içe öteleme alanı kapatıyor.";

/// Whether an offset's refusal is a radius collapsing.
pub fn collapses(message: &str) -> bool {
    COLLAPSES.contains(&message)
}

/// What an offset says besides its copy.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OffsetNote {
    /// The copy follows a curve's parallel only this closely, metres: an ellipse, a fit-point or NURBS
    /// curve's; none for an exact copy.
    pub approximate: Option<f64>,
    /// How many times the copy's edges cross each other (a tight bend offset outward, a narrow neck inward).
    pub crossings: usize,
}

/// [`offset_entity`] and what it says besides: approximate or exact, and its crossings. A fit-point or NURBS curve
/// is offset here by its chords, each within 0.1 mm of it (docs/adr/0293; `offset_entity` keeps the web's answer).
pub fn offset_noted(e: &Shape, distance: f64, through: Vec2) -> (Geometry, OffsetNote) {
    let g = match e {
        Shape::Spline { pts, closed } if distance > 0.0 => {
            offset_chords(&spline_outline(pts, *closed), *closed, distance, through)
        }
        Shape::Nurbs { closed, .. } if distance > 0.0 => {
            offset_chords(&crate::geom::nurbs::outline(e), *closed, distance, through)
        }
        _ => offset_entity(e, distance, through),
    };
    let approximate = match e {
        Shape::Spline { .. } | Shape::Nurbs { .. } => Some(CURVE_TOL),
        // The ellipse's parallel is taken at 512 steps a turn: a chord keeps within a·h²/8.
        _ => ellipse_of(e).map(|g| {
            let h = core::f64::consts::TAU / 512.0;
            major_length(&g) * h * h / 8.0
        }),
    };
    let crossings = match &g {
        Geometry::Ok(copy) => crossings(&copy.shape),
        Geometry::Error(_) => 0,
    };
    (
        g,
        OffsetNote {
            approximate,
            crossings,
        },
    )
}

/// How many times a single path's edges cross each other, but for neighbours; 0 for anything else, or a path of
/// more than 4 000 edges (not looked through).
pub fn crossings(s: &Shape) -> usize {
    let (Shape::Polyline { holes: None, parts: None, .. } | Shape::Polygon { holes: None, parts: None, .. }) = s else {
        return 0;
    };
    let edges = entity_edges(s);
    let n = edges.len();
    if n > 4000 {
        return 0;
    }
    let closed = is_closed_path(s);
    let mut count = 0;
    for i in 0..n {
        for j in i + 2..n {
            if closed && i == 0 && j == n - 1 {
                continue;
            }
            count += intersect_edges(&edges[i], &edges[j]).len();
        }
    }
    count
}

/// A curve's chords offset as a polyline, closed as the curve is.
fn offset_chords(chords: &[Vec2], closed: bool, distance: f64, through: Vec2) -> Geometry {
    let mut pts = chords.to_vec();
    if closed && pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    let side = side_of(&pts, closed, through);
    let out = offset_path(&pts, f64::from(side) * distance, closed);
    if out.len() < 2 {
        return Geometry::Error("Öteleme sonucu geçerli bir şekil oluşmadı.".into());
    }
    Geometry::Ok(Entity::new(Shape::Polyline {
        pts: out,
        bulges: None,
        holes: None,
        parts: None,
        closed,
        widths: None,
    }))
}

/// A ring offset `d` to the left of its travel; none when it closes up (its turn reversed or its area gone).
fn offset_ring(r: &Ring, d: f64) -> Option<Ring> {
    let ring = offset_ring_raw(r, d)?;
    (!turned_over(&r.pts, r.bulges.as_deref(), &ring.pts, ring.bulges.as_deref())).then_some(ring)
}

/// Whether a ring's offset closed up: an edge of it runs against its source's (as many corners), or its area turned
/// over or went (docs/adr/0293). A square offset inward past its middle keeps its turn but every edge reverses.
fn turned_over(before: &[Vec2], bb: Option<&[f64]>, after: &[Vec2], ab: Option<&[f64]>) -> bool {
    let n = before.len();
    if after.len() == n {
        let reversed = (0..n).any(|i| {
            let (a, b) = (before[i], before[(i + 1) % n]);
            let (c, d) = (after[i], after[(i + 1) % n]);
            (b.x - a.x) * (d.x - c.x) + (b.y - a.y) * (d.y - c.y) < 0.0
        });
        if reversed {
            return true;
        }
    }
    let (x, y) = (bulge_ring_area(before, bb), bulge_ring_area(after, ab));
    y.abs() <= 1e-12 || (x > 0.0) != (y > 0.0)
}

fn offset_ring_raw(r: &Ring, d: f64) -> Option<Ring> {
    match r.bulges.as_deref() {
        Some(bs) if has_bulges(Some(bs)) => match offset_bulge_path(&r.pts, bs, d, true) {
            OffsetResult::Path { pts, bulges } => Some(Ring {
                pts,
                bulges: Some(bulges),
            }),
            OffsetResult::Error { .. } => None,
        },
        _ => {
            let pts = offset_path(&r.pts, d, true);
            (pts.len() >= 3).then_some(Ring { pts, bulges: None })
        }
    }
}

/// An area with holes offset (docs/adr/0293): `through` inside the area shrinks it (its ring inward, its holes
/// outward), outside it or in a hole grows it (the ring outward, the holes inward, a hole closing up gone). Rings
/// that stay apart make the copy as they are; rings that meet are put together by the overlay.
fn offset_region(e: &Shape, distance: f64, through: Vec2) -> Geometry {
    let Shape::Polygon {
        pts,
        bulges,
        holes: Some(holes),
        ..
    } = e
    else {
        return Geometry::Error("Öteleme sonucu geçerli bir şekil oluşmadı.".into());
    };
    let inward = inside_polygon(e, through);
    // Left of travel is inside a counter-clockwise ring; a hole's inside is the area's outside.
    let ccw = bulge_ring_area(pts, bulges.as_deref()) > 0.0;
    let outer = Ring {
        pts: pts.clone(),
        bulges: bulges.clone(),
    };
    let Some(outer) = offset_ring(&outer, if inward == ccw { distance } else { -distance }) else {
        return Geometry::Error(if inward {
            REGION_CLOSES.into()
        } else {
            "Öteleme sonucu geçerli bir şekil oluşmadı.".into()
        });
    };
    let grown: Vec<Ring> = holes
        .iter()
        .filter_map(|h| {
            let hccw = bulge_ring_area(&h.pts, h.bulges.as_deref()) > 0.0;
            offset_ring(h, if inward == hccw { -distance } else { distance })
        })
        .collect();
    let rings: Vec<&Ring> = std::iter::once(&outer).chain(&grown).collect();
    let apart = rings.iter().enumerate().all(|(i, a)| {
        rings[i + 1..].iter().all(|b| {
            let (ea, eb) = (ring_edges(a), ring_edges(b));
            ea.iter().all(|x| eb.iter().all(|y| intersect_edges(x, y).is_empty()))
        })
    }) && grown
        .iter()
        .all(|h| point_in_polygon(h.pts[0], &ring_outline(&outer)));
    if apart {
        return Geometry::Ok(Entity::new(Shape::Polygon {
            pts: outer.pts,
            bulges: outer.bulges,
            holes: (!grown.is_empty()).then_some(grown),
            parts: None,
        }));
    }
    let cutters: Vec<Area> = grown
        .into_iter()
        .map(|outer| Area {
            outer,
            holes: Vec::new(),
        })
        .collect();
    let shapes: Vec<Shape> = subtract_areas(
        &[Area {
            outer,
            holes: Vec::new(),
        }],
        &cutters,
    )
    .into_iter()
    .map(|a| Shape::Polygon {
        pts: a.outer.pts,
        bulges: a.outer.bulges,
        holes: (!a.holes.is_empty()).then_some(a.holes),
        parts: None,
    })
    .collect();
    match join_parts(&shapes) {
        Some(shape) => Geometry::Ok(Entity::new(shape)),
        None => Geometry::Error(REGION_CLOSES.into()),
    }
}

fn ring_edges(r: &Ring) -> Vec<Edge> {
    entity_edges(&Shape::Polygon {
        pts: r.pts.clone(),
        bulges: r.bulges.clone(),
        holes: None,
        parts: None,
    })
}

fn ring_outline(r: &Ring) -> Vec<Vec2> {
    crate::entity::polygon_ring(&r.pts, r.bulges.as_deref())
}

/// The distance “Noktadan geç” (through) offsets by: from `p` to the
/// object's nearest edge, so the copy passes through `p` (the offset tool's
/// `distanceFor`, docs/adr/0047); +∞ for an object without edges.
pub fn through_distance(e: &Shape, p: Vec2) -> f64 {
    let mut d = f64::INFINITY;
    for ed in entity_edges(e) {
        d = js_min(d, closest_on_edge(&ed, p).d);
    }
    d
}

/// Side of a bulged path a point lies on (+1 left of travel), judged at the nearest edge.
fn bulged_side(e: &Shape, p: Vec2) -> i32 {
    let (mut best_d, mut best_side) = (f64::INFINITY, 1);
    for ed in entity_edges(e) {
        let c = closest_on_edge(&ed, p);
        if c.d >= best_d {
            continue;
        }
        let side = match ed {
            Edge::Seg { a, b } => {
                if (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x) >= 0.0 {
                    1
                } else {
                    -1
                }
            }
            Edge::Arc { c, r, sweep, .. } => {
                // Inside the circle is left of a counter-clockwise arc.
                let inside = js_hypot(p.x - c.x, p.y - c.y) < r;
                if inside == (sweep > 0.0) { 1 } else { -1 }
            }
        };
        best_d = c.d;
        best_side = side;
    }
    best_side
}

pub(crate) static OPS: &[Op] = &[
    op!("offsetEntity", |e: Entity, distance: f64, through: Vec2| {
        offset_entity(&e.shape, distance, through)
    }),
    op!("offsetThroughDistance", |e: Entity, p: Vec2| {
        through_distance(&e.shape, p)
    }),
];
