//! Stretch, "Esnet" (`apps/web/src/model/ops/stretch.ts`): vertices inside the
//! crossing window move by (dx, dy), the rest stay; None when nothing of
//! the entity lies in the window.

use crate::api::Op;
use crate::entity::{Entity, Part, Shape, entity_geometry};
use crate::geom::arc::{ArcGeom, arc_end, arc_mid, arc_start, arc_through};
use crate::geom::arrangement::Ring;
use crate::geometry::Bounds;
use crate::op;
use crate::vec2::Vec2;

fn inside(p: Vec2, r: &Bounds) -> bool {
    p.x >= r.min_x && p.x <= r.max_x && p.y >= r.min_y && p.y <= r.max_y
}

pub fn stretch_entity(e: &Entity, r: &Bounds, dx: f64, dy: f64) -> Option<Entity> {
    let mv = |p: Vec2| {
        if inside(p, r) {
            Vec2::new(p.x + dx, p.y + dy)
        } else {
            p
        }
    };
    let any = |pts: &[Vec2]| pts.iter().any(|&p| inside(p, r));
    let geom = entity_geometry(e);
    let shape = match &geom.shape {
        // A multi-point object's points in the window move (docs/adr/0174).
        Shape::Point { p, z, parts } => {
            let in_parts = parts.iter().flatten().any(|q| inside(q.p, r));
            (inside(*p, r) || in_parts).then(|| Shape::Point {
                p: mv(*p),
                z: *z,
                parts: parts.as_ref().map(|ps| {
                    ps.iter()
                        .map(|q| crate::entity::PointPart { p: mv(q.p), z: q.z })
                        .collect()
                }),
            })?
        }
        Shape::Text { p, .. } => inside(*p, r).then(|| {
            let mut moved = geom.shape.clone();
            if let Shape::Text { p: at, .. } = &mut moved {
                *at = mv(*p);
            }
            moved
        })?,
        // A raster moves whole when its first pixel's corner is in the window (docs/adr/0204 §7).
        Shape::Raster { affine, .. } => {
            let o = Vec2::new(affine[0], affine[3]);
            inside(o, r).then(|| {
                let to = mv(o);
                crate::ops::transform::transform_shape(
                    &geom.shape,
                    &crate::geom::affine::translation(to.x - o.x, to.y - o.y),
                )
            })?
        }
        // A table moves whole when its top left corner is in the window (docs/adr/0184 §2).
        Shape::Table { p, .. } | Shape::Image { p, .. } => inside(*p, r).then(|| {
            let mut moved = geom.shape.clone();
            if let Shape::Table { p: at, .. } | Shape::Image { p: at, .. } = &mut moved {
                *at = mv(*p);
            }
            moved
        })?,
        Shape::Line { a, b } => any(&[*a, *b]).then(|| Shape::Line {
            a: mv(*a),
            b: mv(*b),
        })?,
        // A block moves whole when its insertion point is in the window (docs/adr/0144).
        Shape::Insert {
            block,
            p,
            scale,
            rotation,
            mirror,
            attrs,
        } => inside(*p, r).then(|| Shape::Insert {
            block: block.clone(),
            p: mv(*p),
            scale: *scale,
            rotation: *rotation,
            mirror: *mirror,
            attrs: attrs.clone(),
        })?,
        // Arc segments keep their bulge, so they bend with their moved ends;
        // every part of a multi-part polyline so (docs/adr/0174).
        Shape::Polyline {
            pts,
            bulges,
            holes,
            parts,
        } => {
            (any(pts) || parts.iter().flatten().any(|q| any(&q.pts))).then(|| Shape::Polyline {
                pts: pts.iter().map(|&p| mv(p)).collect(),
                bulges: bulges.clone(),
                holes: holes.clone(),
                parts: parts.as_ref().map(|ps| {
                    ps.iter()
                        .map(|q| Part {
                            pts: q.pts.iter().map(|&p| mv(p)).collect(),
                            bulges: q.bulges.clone(),
                            holes: None,
                        })
                        .collect()
                }),
            })?
        }
        Shape::Spline { pts, closed } => any(pts).then(|| Shape::Spline {
            pts: pts.iter().map(|&p| mv(p)).collect(),
            closed: *closed,
        })?,
        // Its vertices in the window move; the landing and the note follow the last (docs/adr/0146 §4).
        Shape::Leader {
            pts,
            text,
            height,
            rotation,
            arrow,
            arrow_size,
            mask,
        } => any(pts).then(|| Shape::Leader {
            pts: pts.iter().map(|&p| mv(p)).collect(),
            text: text.clone(),
            height: *height,
            rotation: *rotation,
            arrow: arrow.clone(),
            arrow_size: *arrow_size,
            mask: *mask,
        })?,
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => {
            let touched = |pts: &[Vec2], holes: &Option<Vec<Ring>>| {
                any(pts) || holes.iter().flatten().any(|h| any(&h.pts))
            };
            // Every part of a multi-part area moves the vertices it has in the window (docs/adr/0143).
            if !touched(pts, holes) && !parts.iter().flatten().any(|p| touched(&p.pts, &p.holes)) {
                return None;
            }
            let moved_holes = |holes: &Option<Vec<Ring>>| {
                holes.as_ref().map(|hs| {
                    hs.iter()
                        .map(|h| Ring {
                            pts: h.pts.iter().map(|&p| mv(p)).collect(),
                            bulges: h.bulges.clone(),
                        })
                        .collect()
                })
            };
            Shape::Polygon {
                pts: pts.iter().map(|&p| mv(p)).collect(),
                bulges: bulges.clone(),
                holes: moved_holes(holes),
                parts: parts.as_ref().map(|ps| {
                    ps.iter()
                        .map(|p| Part {
                            pts: p.pts.iter().map(|&q| mv(q)).collect(),
                            bulges: p.bulges.clone(),
                            holes: moved_holes(&p.holes),
                        })
                        .collect()
                }),
            }
        }
        Shape::Hatch {
            ring,
            holes,
            pattern,
            assoc,
        } => {
            if !any(ring) && !holes.iter().flatten().any(|h| any(h)) {
                return None;
            }
            Shape::Hatch {
                ring: ring.iter().map(|&p| mv(p)).collect(),
                holes: holes.as_ref().map(|hs| {
                    hs.iter()
                        .map(|h| h.iter().map(|&p| mv(p)).collect())
                        .collect()
                }),
                pattern: pattern.clone(),
                assoc: assoc.clone(),
            }
        }
        Shape::Dimension {
            a,
            b,
            offset,
            height,
            text,
            style,
            angle,
            c,
            mask,
            za,
            zb,
            look,
        } => {
            let touched = match c {
                Some(c) => any(&[*a, *b, *c]),
                None => any(&[*a, *b]),
            };
            touched.then(|| Shape::Dimension {
                a: mv(*a),
                b: mv(*b),
                offset: *offset,
                height: *height,
                text: text.clone(),
                style: style.clone(),
                angle: *angle,
                c: c.map(&mv),
                mask: *mask,
                za: *za,
                zb: *zb,
                look: look.clone(),
            })?
        }
        Shape::Circle { c, r: radius } => inside(*c, r).then(|| Shape::Circle {
            c: mv(*c),
            r: *radius,
        })?,
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => inside(*c, r).then(|| Shape::Ellipse {
            c: mv(*c),
            major: *major,
            ratio: *ratio,
            t0: *t0,
            t1: *t1,
        })?,
        Shape::Xline { p, dir } => inside(*p, r).then(|| Shape::Xline {
            p: mv(*p),
            dir: *dir,
        })?,
        Shape::Ray { p, dir } => inside(*p, r).then(|| Shape::Ray {
            p: mv(*p),
            dir: *dir,
        })?,
        Shape::Arc {
            c,
            r: radius,
            a0,
            a1,
        } => {
            let g = ArcGeom {
                c: *c,
                r: *radius,
                a0: *a0,
                a1: *a1,
            };
            let s = arc_start(&g);
            let m = arc_mid(&g);
            let f = arc_end(&g);
            let in_s = inside(s, r);
            let in_f = inside(f, r);
            if !in_s && !in_f && !inside(m, r) {
                return None;
            }
            // One end moving drags the middle half-way, keeping the arc's bow.
            let half = |p: Vec2| Vec2::new(p.x + dx / 2.0, p.y + dy / 2.0);
            let mid = if in_s && in_f {
                mv(m)
            } else if in_s != in_f {
                half(m)
            } else {
                mv(m)
            };
            let g2 = arc_through(mv(s), mid, mv(f))?;
            // A new arc object: the TypeScript builds `{ kind: 'arc', ...g2 }`.
            return Some(Entity::new(Shape::Arc {
                c: g2.c,
                r: g2.r,
                a0: g2.a0,
                a1: g2.a1,
            }));
        }
    };
    Some(geom.with(shape))
}

pub(crate) static OPS: &[Op] =
    &[op!("stretchEntity", |e: Entity,
                            r: Bounds,
                            dx: f64,
                            dy: f64| {
        stretch_entity(&e, &r, dx, dy)
    })];
