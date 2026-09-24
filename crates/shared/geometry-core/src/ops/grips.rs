//! Grip points and what moving one means (`apps/web/src/model/ops/grips.ts`). The
//! order is stable: `move_grip` reads the index the way `entity_grips` lists it.

use crate::api::Op;
use crate::entity::{Entity, Shape, dimension_geom, ellipse_geom};
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
    match e {
        Shape::Point { p, .. } | Shape::Text { p, .. } => vec![*p],
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
        Shape::Polyline { pts, .. } => Some(pts.len().saturating_sub(1)),
        _ => None,
    }
}

/// Segment index of a path's mid grip (grip indices after the vertices), else None.
pub fn mid_grip_segment(e: &Shape, index: usize) -> Option<usize> {
    let (Shape::Polyline { pts, .. } | Shape::Polygon { pts, .. }) = e else {
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

pub fn hole_grip(e: &Shape, index: usize) -> Option<HoleGrip> {
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

/// Entity with grip `index` moved to `p` (same id), or None when the result would be degenerate.
pub fn move_grip(e: &Entity, index: usize, p: Vec2) -> Option<Entity> {
    let shape = match &e.shape {
        Shape::Point { z, .. } => Shape::Point { p, z: *z },
        Shape::Text {
            text,
            height,
            rotation,
            ..
        } => Shape::Text {
            p,
            text: text.clone(),
            height: *height,
            rotation: *rotation,
        },
        Shape::Line { a, b } => {
            if index == 0 {
                Shape::Line { a: p, b: *b }
            } else {
                Shape::Line { a: *a, b: p }
            }
        }
        Shape::Polyline { pts, bulges, holes } | Shape::Polygon { pts, bulges, holes } => {
            let rebuild = |pts: Vec<Vec2>, bulges: Option<Vec<f64>>, holes: Option<Vec<Ring>>| {
                match e.shape {
                    Shape::Polygon { .. } => Shape::Polygon { pts, bulges, holes },
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
