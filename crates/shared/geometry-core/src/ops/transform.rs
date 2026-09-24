//! A similarity transform (move, rotate, uniform scale, mirror) applied to
//! any entity (`apps/web/src/model/ops/transform.ts`). The result keeps the id and
//! every other field; mirrored text stays readable (MIRRTEXT = 0).

use crate::api::Op;
use crate::entity::{Entity, Shape, ellipse_geom};
use crate::geom::affine::{Affine, apply, apply_linear, is_reflection, length_scale, translation};
use crate::geom::arc::{ArcGeom, arc_end, arc_start, norm_angle};
use crate::geom::arrangement::Ring;
use crate::geom::ellipse::is_full_ellipse;
use crate::jsmath::{PI, atan2, cos, js_hypot, or, sin};
use crate::op;
use crate::vec2::Vec2;

fn angle_of(c: Vec2, p: Vec2) -> f64 {
    norm_angle(atan2(p.y - c.y, p.x - c.x))
}

/// A ring's vertices transformed; a reflection turns every arc segment the other way.
fn transform_ring(
    pts: &[Vec2],
    bulges: &Option<Vec<f64>>,
    m: &Affine,
) -> (Vec<Vec2>, Option<Vec<f64>>) {
    let pts = pts.iter().map(|&p| apply(m, p)).collect();
    let bulges = bulges.as_ref().map(|b| {
        if is_reflection(m) {
            b.iter().map(|&x| -x).collect()
        } else {
            b.clone()
        }
    });
    (pts, bulges)
}

/// The geometry `s` under `m`. The store transforms its own copies of the
/// objects with it (move, copy, arrays and paste answer packed, without the
/// objects crossing as JSON; `Store::transform_packed`).
pub fn transform_shape(shape: &Shape, m: &Affine) -> Shape {
    let s = length_scale(m);
    match shape {
        Shape::Point { p, z } => Shape::Point {
            p: apply(m, *p),
            z: *z,
        },
        Shape::Line { a, b } => Shape::Line {
            a: apply(m, *a),
            b: apply(m, *b),
        },
        Shape::Polyline { pts, bulges, holes } => {
            let (pts, bulges) = transform_ring(pts, bulges, m);
            // Only a polygon's holes are transformed (a polyline has none to speak of).
            Shape::Polyline {
                pts,
                bulges,
                holes: holes.clone(),
            }
        }
        Shape::Polygon { pts, bulges, holes } => {
            let (pts, bulges) = transform_ring(pts, bulges, m);
            let holes = holes.as_ref().map(|hs| {
                hs.iter()
                    .map(|h| {
                        let (pts, bulges) = transform_ring(&h.pts, &h.bulges, m);
                        Ring { pts, bulges }
                    })
                    .collect()
            });
            Shape::Polygon { pts, bulges, holes }
        }
        Shape::Circle { c, r } => Shape::Circle {
            c: apply(m, *c),
            r: r * s,
        },
        Shape::Arc { c, r, a0, a1 } => {
            let g = ArcGeom {
                c: *c,
                r: *r,
                a0: *a0,
                a1: *a1,
            };
            let c = apply(m, *c);
            let start = apply(m, arc_start(&g));
            let end = apply(m, arc_end(&g));
            // A reflection reverses orientation; swap so the arc stays CCW.
            if is_reflection(m) {
                Shape::Arc {
                    c,
                    r: r * s,
                    a0: angle_of(c, end),
                    a1: angle_of(c, start),
                }
            } else {
                Shape::Arc {
                    c,
                    r: r * s,
                    a0: angle_of(c, start),
                    a1: angle_of(c, end),
                }
            }
        }
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let new_major = apply_linear(m, *major);
            // A reflection runs the parameter the other way: t → −t keeps the arc counter-clockwise.
            if !is_reflection(m) || is_full_ellipse(&ellipse_geom(*c, *major, *ratio, *t0, *t1)) {
                Shape::Ellipse {
                    c: apply(m, *c),
                    major: new_major,
                    ratio: *ratio,
                    t0: *t0,
                    t1: *t1,
                }
            } else {
                Shape::Ellipse {
                    c: apply(m, *c),
                    major: new_major,
                    ratio: *ratio,
                    t0: norm_angle(-t1),
                    t1: norm_angle(-t0),
                }
            }
        }
        Shape::Xline { p, dir } | Shape::Ray { p, dir } => {
            let d = apply_linear(m, *dir);
            let l = or(js_hypot(d.x, d.y), 1.0);
            let (p, dir) = (apply(m, *p), Vec2::new(d.x / l, d.y / l));
            if matches!(shape, Shape::Ray { .. }) {
                Shape::Ray { p, dir }
            } else {
                Shape::Xline { p, dir }
            }
        }
        Shape::Spline { pts, closed } => Shape::Spline {
            pts: pts.iter().map(|&p| apply(m, p)).collect(),
            closed: *closed,
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
            let st = style.as_deref().unwrap_or("aligned");
            let flip = is_reflection(m);
            let na = apply(m, *a);
            let nb = apply(m, *b);
            let height = height * s;
            let c = c.map(|c| apply(m, c));
            let (text, style) = (text.clone(), style.clone());
            if st == "angular" {
                // An angle stays counter-clockwise from a to b: a reflection swaps the arms.
                let (a, b) = if flip { (nb, na) } else { (na, nb) };
                Shape::Dimension {
                    a,
                    b,
                    offset: offset * s,
                    height,
                    text,
                    style,
                    angle: *angle,
                    c,
                }
            } else if st == "radius" || st == "diameter" {
                Shape::Dimension {
                    a: na,
                    b: nb,
                    offset: offset * s,
                    height,
                    text,
                    style,
                    angle: *angle,
                    c,
                }
            } else {
                // Aligned and linear: a reflection swaps left and right, so the offset changes sign.
                let offset = offset * s * if flip { -1.0 } else { 1.0 };
                let angle = if st == "linear" {
                    let rad = (angle.unwrap_or(0.0) * PI) / 180.0;
                    let dir = apply_linear(m, Vec2::new(cos(rad), sin(rad)));
                    Some((atan2(dir.y, dir.x) * 180.0) / PI)
                } else {
                    *angle
                };
                Shape::Dimension {
                    a: na,
                    b: nb,
                    offset,
                    height,
                    text,
                    style,
                    angle,
                    c,
                }
            }
        }
        Shape::Hatch {
            ring,
            holes,
            pattern,
        } => {
            let rad = (pattern.angle * PI) / 180.0;
            let dir = apply_linear(m, Vec2::new(cos(rad), sin(rad)));
            let angle = ((((atan2(dir.y, dir.x) * 180.0) / PI) % 180.0) + 180.0) % 180.0;
            let mut pattern = pattern.clone();
            pattern.angle = angle;
            pattern.spacing *= s;
            Shape::Hatch {
                ring: ring.iter().map(|&p| apply(m, p)).collect(),
                holes: holes.as_ref().map(|hs| {
                    hs.iter()
                        .map(|h| h.iter().map(|&p| apply(m, p)).collect())
                        .collect()
                }),
                pattern,
            }
        }
        Shape::Text {
            p,
            text,
            height,
            rotation,
        } => {
            let rad = (rotation * PI) / 180.0;
            let dir = apply_linear(m, Vec2::new(cos(rad), sin(rad)));
            let mut rot = (atan2(dir.y, dir.x) * 180.0) / PI;
            // Mirrored text stays readable (like AutoCAD MIRRTEXT = 0).
            if is_reflection(m) {
                rot += 180.0;
            }
            rot = ((rot % 360.0) + 360.0) % 360.0;
            Shape::Text {
                p: apply(m, *p),
                text: text.clone(),
                height: height * s,
                rotation: rot,
            }
        }
    }
}

pub fn transform_entity(e: &Entity, m: &Affine) -> Entity {
    e.with(transform_shape(&e.shape, m))
}

pub fn translate_entity(e: &Entity, dx: f64, dy: f64) -> Entity {
    transform_entity(e, &translation(dx, dy))
}

/// Every entity by every affine, affine after affine (S3): move, copy and
/// array a whole selection in one call instead of one call per object.
pub fn transform_entities(list: &[Entity], ms: &[Affine]) -> Vec<Entity> {
    let mut out = Vec::with_capacity(list.len() * ms.len());
    for m in ms {
        out.extend(list.iter().map(|e| transform_entity(e, m)));
    }
    out
}

pub(crate) static OPS: &[Op] = &[
    op!("transformEntity", |e: Entity, m: Affine| transform_entity(
        &e, &m
    )),
    op!("translateEntity", |e: Entity, dx: f64, dy: f64| {
        translate_entity(&e, dx, dy)
    }),
    op!("transformEntities", |list: Vec<Entity>, ms: Vec<Affine>| {
        transform_entities(&list, &ms)
    }),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::{FromJson, Json};
    use crate::geom::affine::rotation;

    fn entity(text: &str) -> Entity {
        Entity::from_json(&Json::parse(text).unwrap()).unwrap()
    }

    #[test]
    fn many_entities_by_many_affines_come_affine_after_affine() {
        let list = [
            entity(
                r#"{"id":1,"layerId":"a","attrs":{"Ada":"1"},"kind":"line","a":{"x":0,"y":0},"b":{"x":10,"y":0}}"#,
            ),
            entity(
                r#"{"id":2,"layerId":"b","attrs":{},"kind":"arc","c":{"x":5,"y":5},"r":2,"a0":0,"a1":1}"#,
            ),
        ];
        let ms = [translation(3.0, 4.0), rotation(0.5, Vec2::new(1.0, 2.0))];
        let out = transform_entities(&list, &ms);
        assert_eq!(out.len(), 4);
        for (k, m) in ms.iter().enumerate() {
            for (i, e) in list.iter().enumerate() {
                assert_eq!(out[k * list.len() + i], transform_entity(e, m));
            }
        }
        assert!(transform_entities(&list, &[]).is_empty());
    }
}
