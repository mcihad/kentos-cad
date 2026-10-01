//! Grip points and what moving one means (`apps/web/src/model/ops/grips.ts`). The
//! order is stable: `move_grip` reads the index the way `entity_grips` lists it.

use crate::api::Op;
use crate::entity::{
    Entity, Shape, area_parts, dimension_geom, ellipse_geom, is_multi_part, locate_part,
    replace_part,
};
use crate::geom::affine::translation;
use crate::geom::arc::{ArcGeom, arc_end, arc_mid, arc_start, arc_through};
use crate::geom::arrangement::Ring;
use crate::geom::bulge::{bulge_at, bulge_through, is_arc_bulge, segment_mid};
use crate::geom::dimension::{dimension_offset_at, layout_dimension};
use crate::geom::ellipse::{closest_param, ellipse_from_center, ellipse_point, is_full_ellipse};
use crate::jsmath::{PI, js_hypot};
use crate::op;
use crate::ops::transform::transform_entity;
use crate::vec2::Vec2;

/// Construction lines show their direction grip this far (m) from the base point.
const DIRECTION_GRIP: f64 = 10.0;

pub fn entity_grips(e: &Shape) -> Vec<Vec2> {
    // A multi-part area's grips are its parts', part after part (docs/adr/0143).
    if is_multi_part(e) {
        return area_parts(e).iter().flat_map(entity_grips).collect();
    }
    match e {
        // An insert's one grip is its insertion point (docs/adr/0144 §3).
        Shape::Point { p, .. } | Shape::Text { p, .. } | Shape::Insert { p, .. } => vec![*p],
        Shape::Line { a, b } => vec![*a, *b],
        Shape::Polyline { pts, bulges, .. } | Shape::Polygon { pts, bulges, .. } => {
            let polygon = matches!(e, Shape::Polygon { .. });
            let n = pts.len();
            let count = if polygon { n } else { n.saturating_sub(1) };
            let mut out = pts.clone();
            for i in 0..count {
                out.push(segment_mid(
                    pts[i],
                    pts[(i + 1) % n],
                    bulge_at(bulges.as_deref(), i),
                ));
            }
            if let Shape::Polygon {
                holes: Some(hs), ..
            } = e
            {
                out.extend(hs.iter().flat_map(|h| h.pts.iter().copied()));
            }
            out
        }
        Shape::Circle { c, r } => vec![
            *c,
            Vec2::new(c.x + r, c.y),
            Vec2::new(c.x, c.y + r),
            Vec2::new(c.x - r, c.y),
            Vec2::new(c.x, c.y - r),
        ],
        Shape::Arc { c, r, a0, a1 } => {
            let g = ArcGeom {
                c: *c,
                r: *r,
                a0: *a0,
                a1: *a1,
            };
            vec![arc_start(&g), arc_mid(&g), arc_end(&g), *c]
        }
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let g = ellipse_geom(*c, *major, *ratio, *t0, *t1);
            if !is_full_ellipse(&g) {
                return vec![*c, ellipse_point(&g, *t0), ellipse_point(&g, *t1)];
            }
            let mut out = vec![*c];
            out.extend((0..4).map(|k| ellipse_point(&g, (k as f64 * PI) / 2.0)));
            out
        }
        Shape::Xline { p, dir } | Shape::Ray { p, dir } => vec![
            *p,
            Vec2::new(p.x + dir.x * DIRECTION_GRIP, p.y + dir.y * DIRECTION_GRIP),
        ],
        Shape::Spline { pts, .. } => pts.clone(),
        // Its vertices, then the middle of each segment, as a polyline's (docs/adr/0146 §4).
        Shape::Leader { pts, .. } => {
            let mut out = pts.clone();
            out.extend(pts.windows(2).map(|s| segment_mid(s[0], s[1], 0.0)));
            out
        }
        Shape::Hatch { ring, .. } => ring.clone(),
        Shape::Dimension { a, b, c, .. } => {
            match dimension_geom(e).and_then(|d| layout_dimension(&d)) {
                Some(l) => {
                    let mut out = vec![*a, *b, l.handle];
                    out.extend(*c);
                    out
                }
                None => vec![*a, *b],
            }
        }
    }
}

fn segment_count(e: &Shape) -> Option<usize> {
    match e {
        Shape::Polygon { pts, .. } => Some(pts.len()),
        Shape::Polyline { pts, .. } | Shape::Leader { pts, .. } => {
            Some(pts.len().saturating_sub(1))
        }
        _ => None,
    }
}

/// Segment index of a path's mid grip (grip indices after the vertices), else None.
/// A multi-part area's grip counts in its own part (`grip_part` says which).
pub fn mid_grip_segment(e: &Shape, index: usize) -> Option<usize> {
    if is_multi_part(e) {
        let (k, local) = grip_part(e, index)?;
        return mid_grip_segment(&area_parts(e)[k], local);
    }
    let (Shape::Polyline { pts, .. } | Shape::Polygon { pts, .. } | Shape::Leader { pts, .. }) = e
    else {
        return None;
    };
    let n = pts.len();
    let count = segment_count(e)?;
    (index >= n && index < n + count).then(|| index - n)
}

/// Hole and vertex of a polygon's hole grip (after the outer vertices and mid grips).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoleGrip {
    pub hole: usize,
    pub vertex: usize,
}

crate::json_struct!(HoleGrip { hole, vertex });

/// A multi-part area's grip counts in its own part (`grip_part` says which).
pub fn hole_grip(e: &Shape, index: usize) -> Option<HoleGrip> {
    if is_multi_part(e) {
        let (k, local) = grip_part(e, index)?;
        return hole_grip(&area_parts(e)[k], local);
    }
    let Shape::Polygon {
        pts,
        holes: Some(holes),
        ..
    } = e
    else {
        return None;
    };
    let mut i = index.checked_sub(2 * pts.len())?;
    for (hole, h) in holes.iter().enumerate() {
        if i < h.pts.len() {
            return Some(HoleGrip { hole, vertex: i });
        }
        i -= h.pts.len();
    }
    None
}

fn replace_at(pts: &[Vec2], index: usize, p: Vec2) -> Vec<Vec2> {
    pts.iter()
        .enumerate()
        .map(|(i, &q)| if i == index { p } else { q })
        .collect()
}

/// The part of a multi-part area that grip `index` belongs to, and the
/// grip's index within that part as `entity_grips` lists it there; `(0,
/// index)` for any other shape (docs/adr/0143).
pub fn grip_part(e: &Shape, index: usize) -> Option<(usize, usize)> {
    if !is_multi_part(e) {
        return Some((0, index));
    }
    locate_part(&area_parts(e), index, |part| entity_grips(part).len())
}

/// Entity with grip `index` moved to `p` (same id), or None when the result would be degenerate.
pub fn move_grip(e: &Entity, index: usize, p: Vec2) -> Option<Entity> {
    // A multi-part area: the grip's part moves it, the others stay.
    if is_multi_part(&e.shape) {
        let (k, local) = grip_part(&e.shape, index)?;
        let part = area_parts(&e.shape)[k].clone();
        let moved = move_grip(&Entity::new(part), local, p)?;
        return Some(e.with(replace_part(&e.shape, k, moved.shape)?));
    }
    let shape = match &e.shape {
        Shape::Point { z, .. } => Shape::Point { p, z: *z },
        Shape::Insert {
            block,
            scale,
            rotation,
            mirror,
            attrs,
            ..
        } => Shape::Insert {
            block: block.clone(),
            p,
            scale: *scale,
            rotation: *rotation,
            mirror: *mirror,
            attrs: attrs.clone(),
        },
        Shape::Text {
            text,
            height,
            rotation,
            align,
            width_factor,
            mask,
            ..
        } => Shape::Text {
            p,
            text: text.clone(),
            height: *height,
            rotation: *rotation,
            align: *align,
            width_factor: *width_factor,
            mask: *mask,
        },
        // A vertex moves; a segment's middle becomes a new vertex there (docs/adr/0146 §4).
        Shape::Leader {
            pts,
            text,
            height,
            rotation,
            arrow,
            mask,
        } => {
            let pts = match mid_grip_segment(&e.shape, index) {
                Some(seg) => {
                    let mut out = pts[..seg + 1].to_vec();
                    out.push(p);
                    out.extend_from_slice(&pts[seg + 1..]);
                    out
                }
                None if index < pts.len() => replace_at(pts, index, p),
                None => return None,
            };
            Shape::Leader {
                pts,
                text: text.clone(),
                height: *height,
                rotation: *rotation,
                arrow: arrow.clone(),
                mask: *mask,
            }
        }
        Shape::Line { a, b } => {
            if index == 0 {
                Shape::Line { a: p, b: *b }
            } else {
                Shape::Line { a: *a, b: p }
            }
        }
        Shape::Polyline { pts, bulges, holes }
        | Shape::Polygon {
            pts, bulges, holes, ..
        } => {
            let rebuild = |pts: Vec<Vec2>, bulges: Option<Vec<f64>>, holes: Option<Vec<Ring>>| {
                match e.shape {
                    Shape::Polygon { .. } => Shape::Polygon {
                        pts,
                        bulges,
                        holes,
                        parts: None,
                    },
                    _ => Shape::Polyline { pts, bulges, holes },
                }
            };
            if let Some(hole) = hole_grip(&e.shape, index) {
                let hs = holes
                    .iter()
                    .flatten()
                    .enumerate()
                    .map(|(k, h)| {
                        if k == hole.hole {
                            Ring {
                                pts: replace_at(&h.pts, hole.vertex, p),
                                bulges: h.bulges.clone(),
                            }
                        } else {
                            h.clone()
                        }
                    })
                    .collect();
                rebuild(pts.clone(), bulges.clone(), Some(hs))
            } else if let Some(seg) = mid_grip_segment(&e.shape, index) {
                let n = pts.len();
                let a = pts[seg];
                let b = pts[(seg + 1) % n];
                // Arc segment: the arc now passes through p. Straight: p becomes a new vertex.
                if is_arc_bulge(bulge_at(bulges.as_deref(), seg)) {
                    let bulge = bulge_through(a, p, b);
                    let bs = (0..n)
                        .map(|i| {
                            if i == seg {
                                bulge
                            } else {
                                bulge_at(bulges.as_deref(), i)
                            }
                        })
                        .collect();
                    rebuild(pts.clone(), Some(bs), holes.clone())
                } else {
                    let mut new_pts = pts[..seg + 1].to_vec();
                    new_pts.push(p);
                    new_pts.extend_from_slice(&pts[seg + 1..]);
                    let new_bulges = bulges.as_ref().map(|b| {
                        let cut = (seg + 1).min(b.len());
                        let mut out = b[..cut].to_vec();
                        out.push(0.0);
                        out.extend_from_slice(&b[cut..]);
                        out
                    });
                    rebuild(new_pts, new_bulges, holes.clone())
                }
            } else {
                rebuild(replace_at(pts, index, p), bulges.clone(), holes.clone())
            }
        }
        Shape::Circle { c, r } => {
            if index == 0 {
                Shape::Circle { c: p, r: *r }
            } else {
                let r = js_hypot(p.x - c.x, p.y - c.y);
                if !(r > 1e-9) {
                    return None;
                }
                Shape::Circle { c: *c, r }
            }
        }
        Shape::Arc { c, r, a0, a1 } => {
            if index == 3 {
                return Some(transform_entity(e, &translation(p.x - c.x, p.y - c.y)));
            }
            // The arc keeps passing through its other two defining points.
            let g = ArcGeom {
                c: *c,
                r: *r,
                a0: *a0,
                a1: *a1,
            };
            let mut pts = [arc_start(&g), arc_mid(&g), arc_end(&g)];
            if index < 3 {
                pts[index] = p;
            }
            let g = arc_through(pts[0], pts[1], pts[2])?;
            Shape::Arc {
                c: g.c,
                r: g.r,
                a0: g.a0,
                a1: g.a1,
            }
        }
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            if index == 0 {
                return Some(transform_entity(e, &translation(p.x - c.x, p.y - c.y)));
            }
            let g = ellipse_geom(*c, *major, *ratio, *t0, *t1);
            if !is_full_ellipse(&g) {
                // Arc ends slide along the ellipse.
                let t = closest_param(&ellipse_geom(*c, *major, *ratio, 0.0, 0.0), p);
                return Some(e.with(if index == 1 {
                    Shape::Ellipse {
                        c: *c,
                        major: *major,
                        ratio: *ratio,
                        t0: t,
                        t1: *t1,
                    }
                } else {
                    Shape::Ellipse {
                        c: *c,
                        major: *major,
                        ratio: *ratio,
                        t0: *t0,
                        t1: t,
                    }
                }));
            }
            let a = js_hypot(major.x, major.y);
            let b = a * ratio;
            let d = js_hypot(p.x - c.x, p.y - c.y);
            // Axis ends: 1/3 re-aim and resize the major axis, 2/4 resize the minor one.
            let g = if index == 1 || index == 3 {
                ellipse_from_center(
                    *c,
                    if index == 1 {
                        p
                    } else {
                        Vec2::new(2.0 * c.x - p.x, 2.0 * c.y - p.y)
                    },
                    b,
                )
            } else {
                ellipse_from_center(*c, Vec2::new(c.x + major.x, c.y + major.y), d)
            }?;
            Shape::Ellipse {
                c: g.c,
                major: g.major,
                ratio: g.ratio,
                t0: g.t0,
                t1: g.t1,
            }
        }
        Shape::Xline { p: base, dir } | Shape::Ray { p: base, dir } => {
            let ray = matches!(e.shape, Shape::Ray { .. });
            let (np, nd) = if index == 0 {
                (p, *dir)
            } else {
                let l = js_hypot(p.x - base.x, p.y - base.y);
                if !(l > 1e-9) {
                    return None;
                }
                (*base, Vec2::new((p.x - base.x) / l, (p.y - base.y) / l))
            };
            if ray {
                Shape::Ray { p: np, dir: nd }
            } else {
                Shape::Xline { p: np, dir: nd }
            }
        }
        Shape::Spline { pts, closed } => Shape::Spline {
            pts: replace_at(pts, index, p),
            closed: *closed,
        },
        Shape::Hatch {
            ring,
            holes,
            pattern,
        } => Shape::Hatch {
            ring: replace_at(ring, index, p),
            holes: holes.clone(),
            pattern: pattern.clone(),
        },
        Shape::Dimension {
            a,
            b,
            offset,
            height,
            text,
            style,
            angle,
            c,
        } => {
            let d = |a: Vec2, b: Vec2, offset: f64, c: Option<Vec2>| Shape::Dimension {
                a,
                b,
                offset,
                height: *height,
                text: text.clone(),
                style: style.clone(),
                angle: *angle,
                c,
            };
            match index {
                0 => {
                    if !(js_hypot(b.x - p.x, b.y - p.y) > 1e-9) {
                        return None;
                    }
                    d(p, *b, *offset, *c)
                }
                1 => {
                    if !(js_hypot(p.x - a.x, p.y - a.y) > 1e-9) {
                        return None;
                    }
                    d(*a, p, *offset, *c)
                }
                3 => d(*a, *b, *offset, Some(p)),
                _ => d(
                    *a,
                    *b,
                    dimension_offset_at(&dimension_geom(&e.shape)?, p),
                    *c,
                ),
            }
        }
    };
    Some(e.with(shape))
}

pub(crate) static OPS: &[Op] = &[
    op!("entityGrips", |e: Entity| entity_grips(&e.shape)),
    op!("moveGrip", |e: Entity, index: usize, p: Vec2| move_grip(
        &e, index, p
    )),
    op!(
        "midGripSegment",
        |e: Entity, index: usize| mid_grip_segment(&e.shape, index)
    ),
    op!("holeGrip", |e: Entity, index: usize| hole_grip(
        &e.shape, index
    )),
];
