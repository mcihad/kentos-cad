//! Explode (`apps/web/src/model/ops/explode.ts`): a compound entity breaks into simple
//! ones. Paths become lines and arcs (holes included), a spline a polyline
//! through its curve, a dimension lines, an arc and its text, a patterned
//! hatch its lines. The dimension's value text comes in already formatted
//! (project units belong to the app), and is used only when the dimension
//! has no text of its own.

use crate::api::Op;
use crate::entity::{Entity, Shape, dimension_geom};
use crate::geom::arc::norm_angle;
use crate::geom::bulge::{bulge_arc, bulge_at};
use crate::geom::dimension::layout_dimension;
use crate::geom::hatch::hatch_lines;
use crate::geom::intersect::Edge;
use crate::geom::spline::catmull_rom;
use crate::jsmath::{PI, cos, js_hypot, js_max, sin};
use crate::op;
use crate::ops::curve_cuts::Cut;
use crate::text::{Font, width_em};
use crate::vec2::Vec2;

fn line(a: Vec2, b: Vec2) -> Entity {
    Entity::new(Shape::Line { a, b })
}

pub fn explode_entity(e: &Shape, value_text: &str, font: Font) -> Cut {
    match e {
        Shape::Polyline { pts, bulges, .. } => {
            let pieces = segment_pieces(pts, bulges.as_deref(), false);
            if pieces.is_empty() {
                Cut::Error("Patlatılacak bir kenar yok.".into())
            } else {
                Cut::Pieces(pieces)
            }
        }
        Shape::Polygon { pts, bulges, holes } => {
            // A polygon's holes come apart too.
            let mut pieces = segment_pieces(pts, bulges.as_deref(), true);
            for h in holes.iter().flatten() {
                pieces.extend(segment_pieces(&h.pts, h.bulges.as_deref(), true));
            }
            if pieces.is_empty() {
                Cut::Error("Patlatılacak bir kenar yok.".into())
            } else {
                Cut::Pieces(pieces)
            }
        }
        Shape::Spline { pts, closed } => {
            let mut out = catmull_rom(pts, *closed, 16.0);
            if *closed {
                out.pop(); // the tessellation repeats the first point
            }
            Cut::Pieces(vec![Entity::new(if *closed {
                Shape::Polygon {
                    pts: out,
                    bulges: None,
                    holes: None,
                }
            } else {
                Shape::Polyline {
                    pts: out,
                    bulges: None,
                    holes: None,
                }
            })])
        }
        Shape::Dimension { text, height, .. } => {
            let Some(l) = dimension_geom(e).and_then(|d| layout_dimension(&d)) else {
                return Cut::Error("Ölçü geometrisi geçersiz.".into());
            };
            let arc = match l.pick.first() {
                Some(&Edge::Arc { c, r, a0, sweep }) => Some((c, r, a0, sweep)),
                _ => None,
            };
            // An angular dimension's arc comes out as one arc, not as the chords it is drawn with.
            let on_arc = |p: Vec2| match arc {
                Some((c, r, ..)) => {
                    (js_hypot(p.x - c.x, p.y - c.y) - r).abs() < 1e-9 * js_max(1.0, r)
                }
                None => false,
            };
            let mut pieces: Vec<Entity> = l
                .lines
                .iter()
                .filter(|[a, b]| !(on_arc(*a) && on_arc(*b)))
                .map(|&[a, b]| line(a, b))
                .collect();
            if let Some((c, r, a0, sweep)) = arc {
                pieces.push(Entity::new(Shape::Arc {
                    c,
                    r,
                    a0: norm_angle(a0),
                    a1: norm_angle(a0 + sweep),
                }));
            }
            let text = match text {
                Some(t) if !t.is_empty() => t.clone(),
                _ => value_text.to_string(),
            };
            // textAt is the text's centre; single-line text is anchored at its start (measured in the drawing's face).
            let r = (l.rotation * PI) / 180.0;
            let half = width_em(&text, font) * height * 0.5;
            pieces.push(Entity::new(Shape::Text {
                p: Vec2::new(l.text_at.x - cos(r) * half, l.text_at.y - sin(r) * half),
                text,
                height: *height,
                rotation: l.rotation,
            }));
            Cut::Pieces(pieces)
        }
        Shape::Hatch {
            ring,
            holes,
            pattern,
        } => {
            if pattern.kind == "solid" {
                return Cut::Error(
                    "Dolu tarama patlatılamaz; sınır olarak taranan şekli kullanın.".into(),
                );
            }
            let holes = holes.as_deref().unwrap_or(&[]);
            let mut segs = hatch_lines(ring, pattern.angle, pattern.spacing, holes).segments;
            if pattern.kind == "cross" {
                segs.extend(
                    hatch_lines(ring, pattern.angle + 90.0, pattern.spacing, holes).segments,
                );
            }
            Cut::Pieces(segs.into_iter().map(|[a, b]| line(a, b)).collect())
        }
        Shape::Circle { .. } => {
            Cut::Error("Daire patlatılamaz; parçalamak için Kır (B) kullanın.".into())
        }
        Shape::Ellipse { .. } => {
            Cut::Error("Elips patlatılamaz; parçalamak için Kır (B) kullanın.".into())
        }
        _ => Cut::Error("Bu nesne zaten temel bir nesne; patlatılacak bir şey yok.".into()),
    }
}

/// Lines and counter-clockwise arcs, one per segment of a bulged path.
fn segment_pieces(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool) -> Vec<Entity> {
    let n = pts.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    let mut pieces = Vec::new();
    for i in 0..count {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        if js_hypot(b.x - a.x, b.y - a.y) < 1e-12 {
            continue;
        }
        match bulge_arc(a, b, bulge_at(bulges, i)) {
            None => pieces.push(line(a, b)),
            Some(arc) => {
                // Arc entities are counter-clockwise; a clockwise segment swaps its ends.
                let end = arc.a0 + arc.sweep;
                let (a0, a1) = if arc.sweep > 0.0 {
                    (arc.a0, end)
                } else {
                    (end, arc.a0)
                };
                pieces.push(Entity::new(Shape::Arc {
                    c: arc.c,
                    r: arc.r,
                    a0: norm_angle(a0),
                    a1: norm_angle(a1),
                }));
            }
        }
    }
    pieces
}

pub(crate) static OPS: &[Op] = &[op!(
    "explodeEntity",
    |e: Entity, value_text: String, font: Option<String>| {
        explode_entity(
            &e.shape,
            &value_text,
            font.as_deref().map_or(Font::DEFAULT, Font::from_id),
        )
    }
)];
