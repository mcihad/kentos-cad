//! A similarity transform (move, rotate, uniform scale, mirror) applied to
//! any entity (`apps/web/src/model/ops/transform.ts`). The result keeps the id and
//! every other field; mirrored text stays readable (MIRRTEXT = 0).

use crate::api::Op;
use crate::entity::{Entity, Part, PointPart, Shape, ellipse_geom};
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

/// A polygon's or a part's holes under `m`.
fn transform_holes(holes: &Option<Vec<Ring>>, m: &Affine) -> Option<Vec<Ring>> {
    holes.as_ref().map(|hs| {
        hs.iter()
            .map(|h| {
                let (pts, bulges) = transform_ring(&h.pts, &h.bulges, m);
                Ring { pts, bulges }
            })
            .collect()
    })
}

/// The geometry `s` under `m`. The store transforms its own copies of the
/// objects with it (move, copy, arrays and paste answer packed, without the
/// objects crossing as JSON; `Store::transform_packed`).
pub fn transform_shape(shape: &Shape, m: &Affine) -> Shape {
    let s = length_scale(m);
    match shape {
        Shape::Point { p, z, parts } => Shape::Point {
            p: apply(m, *p),
            z: *z,
            // Every point of a multi-point object (docs/adr/0174).
            parts: parts.as_ref().map(|ps| {
                ps.iter()
                    .map(|q| PointPart {
                        p: apply(m, q.p),
                        z: q.z,
                    })
                    .collect()
            }),
        },
        Shape::Line { a, b } => Shape::Line {
            a: apply(m, *a),
            b: apply(m, *b),
        },
        Shape::Polyline {
            pts,
            bulges,
            holes,
            parts,
        } => {
            let (pts, bulges) = transform_ring(pts, bulges, m);
            // Only a polygon's holes are transformed (a polyline has none to speak of).
            // Every part of a multi-part polyline, as the first (docs/adr/0174).
            let parts = parts.as_ref().map(|ps| {
                ps.iter()
                    .map(|p| {
                        let (pts, bulges) = transform_ring(&p.pts, &p.bulges, m);
                        Part {
                            pts,
                            bulges,
                            holes: None,
                        }
                    })
                    .collect()
            });
            Shape::Polyline {
                pts,
                bulges,
                holes: holes.clone(),
                parts,
            }
        }
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => {
            let (pts, bulges) = transform_ring(pts, bulges, m);
            let holes = transform_holes(holes, m);
            // Every part of a multi-part area, as the first (docs/adr/0143).
            let parts = parts.as_ref().map(|ps| {
                ps.iter()
                    .map(|p| {
                        let (pts, bulges) = transform_ring(&p.pts, &p.bulges, m);
                        Part {
                            pts,
                            bulges,
                            holes: transform_holes(&p.holes, m),
                        }
                    })
                    .collect()
            });
            Shape::Polygon {
                pts,
                bulges,
                holes,
                parts,
            }
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
            mask,
            za,
            zb,
        } => {
            let st = style.as_deref().unwrap_or("aligned");
            let (mask, za, zb) = (*mask, *za, *zb);
            let flip = is_reflection(m);
            let na = apply(m, *a);
            let nb = apply(m, *b);
            let height = height * s;
            let c = c.map(|c| apply(m, c));
            let (text, style) = (text.clone(), style.clone());
            if st == "angular" || st == "arcLength" {
                // An angle (an arc) stays counter-clockwise from a to b: a reflection swaps the arms (ends).
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
                    mask,
                    za,
                    zb,
                }
            } else if matches!(st, "radius" | "diameter" | "jogged" | "ordinate") {
                // Along the radius (the jog's from the centre shown): no side to swap. An
                // ordinate's axis is the world's and stays (docs/adr/0147 §4).
                Shape::Dimension {
                    a: na,
                    b: nb,
                    offset: offset * s,
                    height,
                    text,
                    style,
                    angle: *angle,
                    c,
                    mask,
                    za,
                    zb,
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
                    mask,
                    za,
                    zb,
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
        // A block's similarity composed with `m` (docs/adr/0144 §3): the
        // insertion point moves, the scale takes the length scale, the turn
        // adds `m`'s; a reflection flips `mirror` and runs the turn backwards
        // (R(θ)·M·R(ρ) = R(θ − ρ)·M).
        Shape::Insert {
            block,
            p,
            scale,
            rotation,
            mirror,
            attrs,
        } => {
            let theta = atan2(m[1], m[0]);
            let flip = is_reflection(m);
            let mirrored = mirror.unwrap_or(false) != flip;
            Shape::Insert {
                block: block.clone(),
                p: apply(m, *p),
                scale: scale * s,
                rotation: norm_angle(if flip {
                    theta - rotation
                } else {
                    theta + rotation
                }),
                mirror: mirrored.then_some(true),
                attrs: attrs.clone(),
            }
        }
        // Its point moves; its alignment, width factor and mask stay (docs/adr/0145); a multi-line
        // text's box scales with its height, its spacing and formats stay (docs/adr/0182).
        Shape::Text {
            p,
            text,
            height,
            rotation,
            align,
            width_factor,
            mask,
            box_width,
            line_spacing,
            runs,
        } => Shape::Text {
            p: apply(m, *p),
            text: text.clone(),
            height: height * s,
            rotation: text_turn(*rotation, m),
            align: *align,
            width_factor: *width_factor,
            mask: *mask,
            box_width: box_width.map(|w| w * s),
            line_spacing: *line_spacing,
            runs: runs.clone(),
        },
        // Its vertices move; its note turns as a text does (docs/adr/0146 §4).
        Shape::Leader {
            pts,
            text,
            height,
            rotation,
            arrow,
            mask,
        } => Shape::Leader {
            pts: pts.iter().map(|&p| apply(m, p)).collect(),
            text: text.clone(),
            height: height * s,
            rotation: text_turn(*rotation, m),
            arrow: arrow.clone(),
            mask: *mask,
        },
    }
}

/// A text's turn (degrees) under `m`, from 0 up to 360: its baseline's
/// direction mapped; mirrored, a half turn more, so that it stays readable
/// (like AutoCAD's MIRRTEXT = 0).
fn text_turn(rotation: f64, m: &Affine) -> f64 {
    let rad = (rotation * PI) / 180.0;
    let dir = apply_linear(m, Vec2::new(cos(rad), sin(rad)));
    let mut rot = (atan2(dir.y, dir.x) * 180.0) / PI;
    if is_reflection(m) {
        rot += 180.0;
    }
    ((rot % 360.0) + 360.0) % 360.0
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

    /// A block's similarity composed by hand (docs/adr/0144 §3): the
    /// insertion point moves with the transform, the scale takes its length
    /// scale, the turn adds its own; a reflection flips `mirror` and turns
    /// the other way (R(θ)·M·R(ρ) = R(θ − ρ)·M).
    #[test]
    fn an_insert_composes_its_similarity() {
        use crate::geom::affine::{mirror, scaling};
        use std::f64::consts::{FRAC_PI_2, PI, TAU};
        let block = entity(
            r#"{"id":1,"layerId":"a","attrs":{},"kind":"insert","block":"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001","p":{"x":3,"y":4},"scale":2,"rotation":0.25}"#,
        );
        let parts = |e: &Entity| match &e.shape {
            Shape::Insert {
                block,
                p,
                scale,
                rotation,
                mirror,
                ..
            } => (block.clone(), *p, *scale, *rotation, *mirror),
            other => panic!("{other:?}"),
        };
        let near = |a: f64, b: f64| (a - b).abs() < 1e-12;
        let moved = parts(&transform_entity(&block, &translation(10.0, -5.0)));
        assert_eq!(moved.1, Vec2::new(13.0, -1.0));
        assert_eq!((moved.2, moved.3, moved.4), (2.0, 0.25, None));
        assert_eq!(moved.0, "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001");
        let turned = parts(&transform_entity(
            &block,
            &rotation(FRAC_PI_2, Vec2::new(0.0, 0.0)),
        ));
        assert!(
            near(turned.1.x, -4.0) && near(turned.1.y, 3.0),
            "{:?}",
            turned.1
        );
        assert!(near(turned.3, 0.25 + FRAC_PI_2) && turned.2 == 2.0 && turned.4.is_none());
        let scaled = parts(&transform_entity(
            &block,
            &scaling(2.0, Vec2::new(0.0, 0.0)),
        ));
        assert_eq!(
            (scaled.1, scaled.2, scaled.3),
            (Vec2::new(6.0, 8.0), 4.0, 0.25)
        );
        // Mirrored in the x axis: the turn runs backwards, into [0, 2π).
        let x_axis = mirror(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        let once = transform_entity(&block, &x_axis);
        let flipped = parts(&once);
        assert_eq!((flipped.1, flipped.4), (Vec2::new(3.0, -4.0), Some(true)));
        assert!(near(flipped.3, TAU - 0.25), "{}", flipped.3);
        // Twice is the block as it was, not mirrored.
        let back = parts(&transform_entity(&once, &x_axis));
        assert_eq!((back.1, back.4), (Vec2::new(3.0, 4.0), None));
        assert!(near(back.3, 0.25), "{}", back.3);
        // In the y axis: a half turn after the x axis's mirror.
        let y_axis = mirror(Vec2::new(0.0, 0.0), Vec2::new(0.0, 1.0));
        let other = parts(&transform_entity(&block, &y_axis));
        assert_eq!((other.1, other.4), (Vec2::new(-3.0, 4.0), Some(true)));
        assert!(near(other.3, PI - 0.25), "{}", other.3);
        // The JSON keeps `mirror` only when true.
        let json = |e: &Entity| {
            let mut out = String::new();
            crate::api::json::ToJson::write_json(e, &mut out);
            out
        };
        assert!(json(&once).contains(r#""mirror":true"#), "{}", json(&once));
        assert!(!json(&block).contains("mirror"), "{}", json(&block));
    }
}
