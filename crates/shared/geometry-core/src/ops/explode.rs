//! Explode (`apps/web/src/model/ops/explode.ts`): a compound entity breaks into simple
//! ones. Paths become lines and arcs (holes included), a spline a polyline
//! through its curve, a dimension lines, an arc, its filled arrowheads and
//! dots (solid hatches) and its text, a patterned hatch its lines, a leader
//! its line, its arrowhead and its note, a table its lines and each cell's
//! words (docs/adr/0184 §2). The
//! dimension's value text comes in already formatted (project units belong
//! to the app), and is used only when the dimension has no text of its own.

use crate::api::Op;
use crate::entity::{Entity, HatchPattern, Shape, dimension_geom};
use crate::geom::arc::norm_angle;
use crate::geom::bulge::{bulge_arc, bulge_at};
use crate::geom::curve_outline::spline_outline;
use crate::geom::dimension::layout_dimension;
use crate::geom::hatch::hatch_lines;
use crate::geom::hatch_pattern::pattern_pieces;
use crate::geom::leader::{self, Head};
use crate::geom::intersect::Edge;
use crate::jsmath::{PI, cos, js_hypot, js_max, sin};
use crate::op;
use crate::ops::curve_cuts::Cut;
use crate::text::{Font, width_em};
use crate::vec2::Vec2;

/// The most lines and points a pattern's hatch explodes into (docs/adr/0186 §8).
pub const EXPLODE_PIECES: usize = 20_000;

fn line(a: Vec2, b: Vec2) -> Entity {
    Entity::new(Shape::Line { a, b })
}

pub fn explode_entity(e: &Shape, value_text: &str, font: Font) -> Cut {
    match e {
        // Its lines, and each cell's words a text on its baseline in the table's face; a
        // heading row's bold in its typeface (bold needs one, docs/adr/0183 §2).
        Shape::Table {
            height,
            rotation,
            cells,
            face,
            ..
        } => {
            let Some(t) = crate::geom::table::table_geom(e) else {
                return Cut::Error("Tablo geçersiz.".into());
            };
            let laid = t.layout(font);
            let mut pieces: Vec<Entity> = laid.lines.iter().map(|[a, b]| line(*a, *b)).collect();
            // Its frame's band, solid hatches as a dimension's arrowheads (docs/adr/0184 §2).
            for strip in &laid.frame {
                pieces.push(Entity::new(Shape::Hatch {
                    ring: strip.to_vec(),
                    holes: None,
                    pattern: HatchPattern::user("solid", 0.0, *height),
                    assoc: None,
                }));
            }
            for c in &laid.cells {
                let words = cells[c.row][c.col].clone();
                let mut f = face.clone();
                if c.bold && !f.is_bold() {
                    f.font = Some(f.font_or(font));
                    f.bold = true;
                }
                pieces.push(Entity::new(Shape::Text {
                    p: c.at,
                    text: words,
                    height: *height,
                    rotation: *rotation,
                    align: None,
                    width_factor: None,
                    mask: None,
                    box_width: None,
                    line_spacing: None,
                    runs: None,
                    face: f,
                }));
            }
            if pieces.is_empty() {
                Cut::Error("Tablonun çizgisi ve yazısı yok.".into())
            } else {
                Cut::Pieces(pieces)
            }
        }
        // A multi-point object comes apart into its points (docs/adr/0174).
        Shape::Point { .. } if crate::entity::is_multi_part(e) => Cut::Pieces(
            crate::entity::area_parts(e)
                .iter()
                .map(|s| Entity::new(s.clone()))
                .collect(),
        ),
        Shape::Polyline {
            pts, bulges, parts, ..
        } => {
            // Every part's edges (docs/adr/0174).
            let mut pieces = segment_pieces(pts, bulges.as_deref(), false);
            for part in parts.iter().flatten() {
                pieces.extend(segment_pieces(&part.pts, part.bulges.as_deref(), false));
            }
            if pieces.is_empty() {
                Cut::Error("Patlatılacak bir kenar yok.".into())
            } else {
                Cut::Pieces(pieces)
            }
        }
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => {
            // A polygon's holes come apart too, and every part's (docs/adr/0143).
            let mut pieces = segment_pieces(pts, bulges.as_deref(), true);
            for h in holes.iter().flatten() {
                pieces.extend(segment_pieces(&h.pts, h.bulges.as_deref(), true));
            }
            for part in parts.iter().flatten() {
                pieces.extend(segment_pieces(&part.pts, part.bulges.as_deref(), true));
                for h in part.holes.iter().flatten() {
                    pieces.extend(segment_pieces(&h.pts, h.bulges.as_deref(), true));
                }
            }
            if pieces.is_empty() {
                Cut::Error("Patlatılacak bir kenar yok.".into())
            } else {
                Cut::Pieces(pieces)
            }
        }
        Shape::Spline { pts, closed } => {
            // Chords within 0.1 mm of the curve (docs/adr/0149 §5.3); a closed ring does not repeat its first point.
            let out = spline_outline(pts, *closed);
            Cut::Pieces(vec![Entity::new(if *closed {
                Shape::Polygon {
                    pts: out,
                    bulges: None,
                    holes: None,
                    parts: None,
                }
            } else {
                Shape::Polyline {
                    pts: out,
                    bulges: None,
                    holes: None,
                    parts: None,
                }
            })])
        }
        Shape::Dimension {
            text, height, look, ..
        } => {
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
            // Its filled arrowheads and dots, solid hatches as a leader's (docs/adr/0183 §3).
            for ring in l.fills.iter().flatten() {
                pieces.push(Entity::new(Shape::Hatch {
                    ring: ring.clone(),
                    holes: None,
                    pattern: HatchPattern::user("solid", 0.0, *height),
                    assoc: None,
                }));
            }
            let text = match text {
                Some(t) if !t.is_empty() => t.clone(),
                _ => value_text.to_string(),
            };
            // textAt is the text's centre; single-line text is anchored at its start (measured in the drawing's face).
            let r = (l.rotation * PI) / 180.0;
            let half = width_em(&text, look.font.unwrap_or(font)) * height * 0.5;
            pieces.push(Entity::new(Shape::Text {
                p: Vec2::new(l.text_at.x - cos(r) * half, l.text_at.y - sin(r) * half),
                text,
                height: *height,
                rotation: l.rotation,
                align: None,
                width_factor: None,
                mask: None,
                box_width: None,
                line_spacing: None,
                runs: None,
                // The value's typeface, when the dimension has its own (docs/adr/0183 §3).
                face: crate::text::face::Face {
                    font: look.font,
                    ..Default::default()
                },
            }));
            Cut::Pieces(pieces)
        }
        Shape::Hatch {
            ring,
            holes,
            pattern,
            ..
        } => {
            if pattern.kind == "solid" {
                return Cut::Error(
                    "Dolu tarama patlatılamaz; sınır olarak taranan şekli kullanın.".into(),
                );
            }
            // A gradient explodes as a solid fill does (docs/adr/0186 §8).
            if pattern.kind == "gradient" {
                return Cut::Error(
                    "Degrade tarama patlatılamaz; sınır olarak taranan şekli kullanın.".into(),
                );
            }
            let holes = holes.as_deref().unwrap_or(&[]);
            if pattern.kind == "pattern" {
                // Its lines and dashes as lines, its dots as points (docs/adr/0186 §8).
                let cut = pattern_pieces(ring, holes, pattern, EXPLODE_PIECES);
                if cut.capped {
                    return Cut::Error(format!(
                        "Desen bu alan için çok sık: {EXPLODE_PIECES} parçadan çok olur; deseni büyütüp yeniden deneyin."
                    ));
                }
                let mut pieces: Vec<Entity> =
                    cut.segments.into_iter().map(|[a, b]| line(a, b)).collect();
                pieces.extend(cut.dots.into_iter().map(|p| {
                    Entity::new(Shape::Point {
                        p,
                        z: None,
                        parts: None,
                    })
                }));
                return Cut::Pieces(pieces);
            }
            let mut segs = hatch_lines(ring, pattern.angle, pattern.spacing, holes).segments;
            if pattern.kind == "cross" {
                segs.extend(
                    hatch_lines(ring, pattern.angle + 90.0, pattern.spacing, holes).segments,
                );
            }
            Cut::Pieces(segs.into_iter().map(|[a, b]| line(a, b)).collect())
        }
        // Its line on to its landing's end a polyline, its note a text, its arrowhead a solid
        // hatch (a filled one, a dot) or a polyline (an open one) (docs/adr/0146 §4).
        Shape::Leader {
            pts,
            text,
            height,
            rotation,
            mask,
            ..
        } => {
            let Some(l) = leader::layout_of(e) else {
                return Cut::Error("Kılavuzun köşesi yok.".into());
            };
            let path = |pts: Vec<Vec2>| {
                Entity::new(Shape::Polyline {
                    pts,
                    bulges: None,
                    holes: None,
                    parts: None,
                })
            };
            let mut pieces = vec![path(leader::drawn_path(pts, &l))];
            if let Head::Open { lines } = &l.head {
                pieces.push(path(lines.to_vec()));
            } else if let Some(ring) = leader::head_ring(&l.head) {
                pieces.push(Entity::new(Shape::Hatch {
                    ring,
                    holes: None,
                    pattern: HatchPattern::user("solid", 0.0, *height),
                    assoc: None,
                }));
            }
            if let (Some(text), Some(p)) = (text, l.note_point) {
                pieces.push(Entity::new(Shape::Text {
                    p,
                    text: text.clone(),
                    height: *height,
                    rotation: *rotation,
                    align: l.note_align,
                    width_factor: None,
                    mask: *mask,
                    box_width: None,
                    line_spacing: None,
                    runs: None,
                    face: Default::default(),
                }));
            }
            Cut::Pieces(pieces)
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
