//! Adding and removing path vertices (`apps/web/src/model/ops/vertex.ts`). An arc
//! segment split by a new vertex keeps its circle; Köşeyi sil may join two arcs of
//! one circle into one (docs/adr/5000).

use crate::api::Op;
use crate::entity::{
    Entity, Shape, area_parts, is_multi_part, locate_part, outer_count, replace_part,
};
use crate::geom::bulge::{
    bulge_arc, bulge_at, bulge_of_sweep, bulge_path_edges, clean_bulge_path, is_arc_bulge,
};
use crate::geom::intersect::{Edge, closest_on_edge};
use crate::jsmath::{js_hypot, js_min};
use crate::op;
use crate::ops::curve_cuts::Geometry;
use crate::ops::edges::entity_edges;
use crate::ops::offset::path_like;
use crate::vec2::Vec2;

/// Index of the edge of an entity nearest to p (0 when it has none). Edges
/// within 1e-9 m tie and the first wins: at a shared vertex both edges are
/// nearest, and rounding in an arc's end must not pick between them.
pub fn nearest_segment(e: &Shape, p: Vec2) -> usize {
    let (mut best_d, mut best_i) = (f64::INFINITY, 0);
    for (i, ed) in entity_edges(e).iter().enumerate() {
        let d = closest_on_edge(ed, p).d;
        if d < best_d - 1e-9 {
            best_d = d;
            best_i = i;
        }
    }
    best_i
}

/// Whether `p` is nearer to one of a closed area's holes than to its outer
/// ring (the vertex tool's `nearHole`, docs/adr/0047): there the tool says
/// that a hole's corners move with grips. False for anything else.
pub fn near_hole(e: &Shape, p: Vec2) -> bool {
    let nearest = |edges: &[Edge]| {
        edges
            .iter()
            .fold(f64::INFINITY, |m, ed| js_min(m, closest_on_edge(ed, p).d))
    };
    // Every part's rings: its outer ring against its holes (docs/adr/0143).
    let (mut outer, mut inner) = (Vec::new(), Vec::new());
    for part in area_parts(e).iter() {
        let Shape::Polygon {
            pts, bulges, holes, ..
        } = part
        else {
            return false;
        };
        outer.extend(bulge_path_edges(pts, bulges.as_deref(), true));
        for h in holes.iter().flatten() {
            inner.extend(bulge_path_edges(&h.pts, h.bulges.as_deref(), true));
        }
    }
    !inner.is_empty() && nearest(&inner) < nearest(&outer)
}

/// `edit` of the part of a multi-part area that holds `index` (counted by
/// `count` per part), the part keeping its holes, the others as they are
/// (docs/adr/0143); what `edit` refuses is the area's refusal.
fn in_part(
    e: &Shape,
    index: usize,
    count: impl Fn(&Shape) -> usize,
    edit: impl FnOnce(&Shape, usize) -> Result<Geometry, String>,
) -> Result<Geometry, String> {
    let parts = area_parts(e);
    let Some((k, local)) = locate_part(&parts, index, count) else {
        return Err(format!("{}. öğe yok.", index + 1));
    };
    let holes = match &parts[k] {
        Shape::Polygon { holes, .. } => holes.clone(),
        _ => None,
    };
    Ok(match edit(&parts[k], local)? {
        Geometry::Ok(done) => {
            let part = match done.shape {
                Shape::Polygon {
                    pts, bulges, parts, ..
                } => Shape::Polygon {
                    pts,
                    bulges,
                    holes,
                    parts,
                },
                other => other,
            };
            match replace_part(e, k, part) {
                Some(shape) => Geometry::Ok(Entity::new(shape)),
                None => Geometry::Error(part_kept(e).into()),
            }
        }
        refused => refused,
    })
}

/// Why a part's edit cannot be written back: it must stay its object's kind.
pub(crate) fn part_kept(e: &Shape) -> &'static str {
    match e {
        Shape::Polyline { .. } => "Parça çoklu çizgi olarak kalmalı.",
        _ => "Parça kapalı alan olarak kalmalı.",
    }
}

/// `Array.prototype.splice(start, 1, ...items)` on a bulge list.
fn splice(v: &mut Vec<f64>, start: usize, items: &[f64]) {
    let start = start.min(v.len());
    let end = (start + 1).min(v.len());
    v.splice(start..end, items.iter().copied());
}

/// Adds a vertex on segment `seg` at the point nearest to p. A line becomes
/// a two-segment polyline; an arc segment is split into two arcs on its
/// circle. Err where the TypeScript reads a segment that is not there.
pub fn insert_vertex(e: &Shape, seg: usize, p: Vec2) -> Result<Geometry, String> {
    // `seg` counts every part's edges, part after part (`entity_edges`).
    if is_multi_part(e) {
        return in_part(
            e,
            seg,
            |part| entity_edges(part).len(),
            |part, local| insert_vertex(part, local, p),
        );
    }
    let (pts, bulges, closed) = match e {
        Shape::Line { a, b } => {
            let q = closest_on_edge(&Edge::Seg { a: *a, b: *b }, p);
            if q.t <= 1e-9 || q.t >= 1.0 - 1e-9 {
                return Ok(Geometry::Error(
                    "Köşe çizginin iç kısmına eklenmeli.".into(),
                ));
            }
            return Ok(Geometry::Ok(Entity::new(Shape::Polyline {
                pts: vec![*a, q.p, *b],
                bulges: None,
                holes: None,
                parts: None,
                closed: false,
                widths: None,
            })));
        }
        // A closed polyline's closing edge takes a vertex as an area's (docs/adr/0258 §1).
        Shape::Polyline {
            pts,
            bulges,
            closed,
            ..
        } => (pts, bulges.as_deref(), *closed),
        Shape::Polygon { pts, bulges, .. } => (pts, bulges.as_deref(), true),
        _ => {
            return Ok(Geometry::Error(
                "Köşe yalnızca çizgi, çoklu çizgi ve kapalı alana eklenebilir.".into(),
            ));
        }
    };
    let n = pts.len();
    let missing = || format!("{}. kenar yok.", seg + 1);
    let bulge = bulge_at(bulges, seg);
    let arc = if is_arc_bulge(bulge) {
        let (Some(&a), Some(&b)) = (pts.get(seg), pts.get((seg + 1) % n.max(1))) else {
            return Err(missing());
        };
        bulge_arc(a, b, bulge)
    } else {
        None
    };
    let edges = entity_edges(e);
    let edge = edges.get(seg).ok_or_else(missing)?;
    let q = closest_on_edge(edge, p);
    if q.t <= 1e-9 || q.t >= 1.0 - 1e-9 {
        return Ok(Geometry::Error(
            "Köşe bir kenarın iç kısmına eklenmeli; mevcut köşeye çok yakın.".into(),
        ));
    }
    let cut = (seg + 1).min(n);
    let mut out_p = pts[..cut].to_vec();
    out_p.push(q.p);
    out_p.extend_from_slice(&pts[cut..]);
    let mut out_b: Vec<f64> = (0..n).map(|i| bulge_at(bulges, i)).collect();
    // Splitting an arc at parameter t: both halves keep the circle.
    let halves = match arc {
        Some(arc) => [
            bulge_of_sweep(arc.sweep * q.t),
            bulge_of_sweep(arc.sweep * (1.0 - q.t)),
        ],
        None => [0.0, 0.0],
    };
    splice(&mut out_b, seg, &halves);
    let clean = clean_bulge_path(&out_p, Some(&out_b), closed, 1e-9);
    Ok(Geometry::Ok(Entity::new(path_like(
        e,
        clean.pts,
        clean.bulges,
    ))))
}

/// Removes vertex `index`; its two segments merge into one straight segment.
pub fn remove_vertex(e: &Shape, index: usize) -> Geometry {
    remove_vertex_joining(e, index, None).0
}

/// How Köşeyi sil joins the two arcs at the removed vertex (docs/adr/5000).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ArcJoin {
    /// No tolerance given, the removed vertex an open path's end, or a side straight: the joined edge is straight.
    Straight,
    /// One arc of the two arcs' circle, its bulge.
    Joined(f64),
    /// Two arcs, not of one circle within the tolerance: their centres' and radii's distance, metres.
    Apart { centres: f64, radii: f64 },
    /// Two arcs turning opposite ways.
    Opposite,
    /// Two arcs of one circle making a whole turn or more together.
    Whole,
}

/// The arcs p → v (bulge `b1`) and v → q (`b2`) as one arc p → q: when both are arcs turning the same way, their
/// centres and radii within `tol` of each other and their sweeps less than a whole turn together. The joined bulge is
/// tan((θ₁ + θ₂)/4) by the tangent's sum, (b₁ + b₂)/(1 − b₁b₂): the arc keeps both ends and the sweeps' sum.
pub fn arc_join(p: Vec2, v: Vec2, q: Vec2, b1: f64, b2: f64, tol: f64) -> ArcJoin {
    let (Some(one), Some(two)) = (bulge_arc(p, v, b1), bulge_arc(v, q, b2)) else {
        return ArcJoin::Straight;
    };
    if (b1 > 0.0) != (b2 > 0.0) {
        return ArcJoin::Opposite;
    }
    let centres = js_hypot(one.c.x - two.c.x, one.c.y - two.c.y);
    let radii = (one.r - two.r).abs();
    if !(centres <= tol && radii <= tol) {
        return ArcJoin::Apart { centres, radii };
    }
    let rest = 1.0 - b1 * b2;
    if rest <= 0.0 {
        return ArcJoin::Whole;
    }
    let joined = (b1 + b2) / rest;
    if joined.is_finite() {
        ArcJoin::Joined(joined)
    } else {
        ArcJoin::Whole
    }
}

/// Köşeyi sil with the arcs at the vertex joined into one arc when `join` (a tolerance, metres) is given and they
/// are of one circle within it (`arc_join`); otherwise the joined edge is straight. The join says which.
pub fn remove_vertex_joining(e: &Shape, index: usize, join: Option<f64>) -> (Geometry, ArcJoin) {
    // `index` counts every part's outer vertices, part after part.
    if is_multi_part(e) {
        let mut how = ArcJoin::Straight;
        let done = in_part(e, index, outer_count, |part, local| {
            let (g, h) = remove_vertex_joining(part, local, join);
            how = h;
            Ok(g)
        })
        .unwrap_or_else(Geometry::Error);
        return (done, how);
    }
    let (pts, bulges, closed) = match e {
        Shape::Polyline {
            pts,
            bulges,
            closed,
            ..
        } => (pts, bulges.as_deref(), *closed),
        Shape::Polygon { pts, bulges, .. } => (pts, bulges.as_deref(), true),
        _ => {
            return (
                Geometry::Error("Köşe yalnızca çoklu çizgi ve kapalı alandan silinebilir.".into()),
                ArcJoin::Straight,
            );
        }
    };
    let n = pts.len();
    if n < if closed { 4 } else { 3 } {
        let why = if matches!(e, Shape::Polyline { .. }) && closed {
            "Kapalı çoklu çizgide en az üç köşe kalmalı.".into()
        } else if closed {
            "Kapalı alanda en az üç köşe kalmalı.".into()
        } else {
            "Çoklu çizgide en az iki köşe kalmalı.".into()
        };
        return (Geometry::Error(why), ArcJoin::Straight);
    }
    let out_p: Vec<Vec2> = pts
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != index)
        .map(|(_, &p)| p)
        .collect();
    let mut out_b: Vec<f64> = (0..n).map(|i| bulge_at(bulges, i)).collect();
    let mut how = ArcJoin::Straight;
    if !closed && index == 0 {
        out_b.remove(0);
    } else if !closed && index == n - 1 {
        out_b.remove(n - 1);
        out_b[n - 2] = 0.0;
    } else {
        // Segments (index−1 → index) and (index → index+1) become one segment: straight, or one arc of their circle.
        let prev = (index % n + n - 1) % n;
        if let Some(tol) = join {
            how = arc_join(
                pts[prev],
                pts[index],
                pts[(index + 1) % n],
                out_b[prev],
                out_b[index],
                tol,
            );
        }
        out_b[prev] = match how {
            ArcJoin::Joined(b) => b,
            _ => 0.0,
        };
        if index < out_b.len() {
            out_b.remove(index);
        }
    }
    let clean = clean_bulge_path(&out_p, Some(&out_b), closed, 1e-9);
    (
        Geometry::Ok(Entity::new(path_like(e, clean.pts, clean.bulges))),
        how,
    )
}

pub(crate) static OPS: &[Op] = &[
    op!("nearestSegment", |e: Entity, p: Vec2| nearest_segment(
        &e.shape, p
    )),
    op!("insertVertex", |e: Entity, seg: usize, p: Vec2| {
        insert_vertex(&e.shape, seg, p)
    }),
    op!("removeVertex", |e: Entity, index: usize| remove_vertex(
        &e.shape, index
    )),
    op!("nearHole", |e: Entity, p: Vec2| near_hole(&e.shape, p)),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// docs/adr/5000: two quarter arcs of the unit circle join into a half (bulge tan 45° = 1); opposite turns and a
    /// straight side do not join.
    #[test]
    fn two_arcs_of_one_circle_join_into_one() {
        let (p, v, q) = (
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(-1.0, 0.0),
        );
        let quarter = bulge_of_sweep(std::f64::consts::FRAC_PI_2);
        match arc_join(p, v, q, quarter, quarter, 1e-6) {
            ArcJoin::Joined(b) => assert!((b - 1.0).abs() < 1e-12, "{b}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(arc_join(p, v, q, quarter, -quarter, 1.0), ArcJoin::Opposite);
        assert_eq!(arc_join(p, v, q, 0.0, quarter, 1.0), ArcJoin::Straight);
        let shape = Shape::Polyline {
            pts: vec![p, v, q],
            bulges: Some(vec![quarter, quarter]),
            holes: None,
            parts: None,
            closed: false,
            widths: None,
        };
        let (Geometry::Ok(done), ArcJoin::Joined(_)) = remove_vertex_joining(&shape, 1, Some(1e-6))
        else {
            panic!("not joined");
        };
        let Shape::Polyline { pts, bulges, .. } = done.shape else {
            panic!("not a polyline");
        };
        assert_eq!(pts, vec![p, q]);
        assert!((bulges.expect("an arc")[0] - 1.0).abs() < 1e-12);
    }
}
