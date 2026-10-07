//! Kes, Kesişim, Fark, Simetrik fark, Birleşim and Birleştir (docs/adr/0201
//! §3–§5): an object's parts inside or outside a set of areas. Areas meet
//! areas in the core's overlay; a path is cut where it crosses the set's
//! boundary and its pieces are kept by where their middles stand (on the
//! boundary is inside); a point by where it stands.

use super::{Class, Place, Region, chain, intersect_all, subtract_all, union_all};
use crate::geom::arrangement::edge_box;
use crate::geom::arrangement::{Area, TOL, WindingIndex};
use crate::geom::intersect::{Edge, intersect_edges, point_at};
use crate::geom::region::net_area;
use crate::geometry::{Bounds, empty_bounds, extend_bounds};
use crate::jsmath::js_hypot;
use crate::vec2::Vec2;

/// The piece of an edge between parameters `t0` and `t1`, from `p0` to `p1` (exact ends kept).
fn sub_edge(e: &Edge, t0: f64, t1: f64, p0: Vec2, p1: Vec2) -> Edge {
    match *e {
        Edge::Seg { .. } => Edge::Seg { a: p0, b: p1 },
        Edge::Arc { c, r, a0, sweep } => Edge::Arc {
            c,
            r,
            a0: a0 + sweep * t0,
            sweep: sweep * (t1 - t0),
        },
    }
}

/// An edge cut where it crosses `boundary`: its pieces in order.
fn split_edge(e: &Edge, boundary: &[Edge], boxes: &[Bounds]) -> Vec<Edge> {
    let bx = edge_box(e);
    let mut cuts: Vec<(f64, Vec2)> = Vec::new();
    for (b, bb) in boundary.iter().zip(boxes) {
        if bb.min_x > bx.max_x + TOL
            || bb.max_x < bx.min_x - TOL
            || bb.min_y > bx.max_y + TOL
            || bb.max_y < bx.min_y - TOL
        {
            continue;
        }
        for h in intersect_edges(e, b) {
            if h.t > 1e-12 && h.t < 1.0 - 1e-12 {
                cuts.push((h.t, h.p));
            }
        }
    }
    cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
    cuts.dedup_by(|b, a| js_hypot(b.1.x - a.1.x, b.1.y - a.1.y) <= TOL);
    let (start, end) = (point_at(e, 0.0), point_at(e, 1.0));
    let mut out = Vec::with_capacity(cuts.len() + 1);
    let (mut t0, mut p0) = (0.0, start);
    for (t, p) in cuts {
        if js_hypot(p.x - p0.x, p.y - p0.y) > TOL {
            out.push(sub_edge(e, t0, t, p0, p));
            t0 = t;
            p0 = p;
        }
    }
    if js_hypot(end.x - p0.x, end.y - p0.y) > TOL || out.is_empty() {
        out.push(sub_edge(e, t0, 1.0, p0, end));
    }
    out
}

/// The paths' pieces inside (and on) the areas, or outside them.
fn paths_in(paths: &[Vec<Edge>], region: &Region, inside: bool) -> Vec<Vec<Edge>> {
    let boundary = region.edges();
    let boxes: Vec<Bounds> = boundary.iter().map(edge_box).collect();
    let index = WindingIndex::new(boundary);
    let mut out = Vec::new();
    for path in paths {
        let mut kept = Vec::new();
        for e in path {
            for piece in split_edge(e, boundary, &boxes) {
                let place = region.place(&index, point_at(&piece, 0.5));
                if (place != Place::Outside) == inside {
                    kept.push(piece);
                }
            }
        }
        out.extend(chain(kept));
    }
    out
}

/// An object's parts inside (and on) the areas `cut`, or outside them.
pub fn within(c: &Class, cut: &[Area], inside: bool) -> Class {
    match c {
        Class::Areas(areas) => Class::Areas(if inside {
            intersect_all(areas, cut)
        } else {
            subtract_all(areas, cut)
        }),
        Class::Paths(paths) => {
            if cut.is_empty() {
                return if inside {
                    Class::Paths(Vec::new())
                } else {
                    c.clone()
                };
            }
            Class::Paths(paths_in(paths, &Region::of(cut), inside))
        }
        Class::Points(points) => {
            if cut.is_empty() {
                return if inside {
                    Class::Points(Vec::new())
                } else {
                    c.clone()
                };
            }
            let region = Region::of(cut);
            let index = WindingIndex::new(region.edges());
            Class::Points(
                points
                    .iter()
                    .copied()
                    .filter(|&p| (region.place(&index, p) != Place::Outside) == inside)
                    .collect(),
            )
        }
        Class::None => Class::None,
    }
}

/// Whether a class holds nothing.
pub fn is_empty(c: &Class) -> bool {
    match c {
        Class::Areas(a) => a.is_empty(),
        Class::Paths(p) => p.iter().all(Vec::is_empty),
        Class::Points(p) => p.is_empty(),
        Class::None => true,
    }
}

/// A class's box (none when empty).
pub fn class_box(c: &Class) -> Option<Bounds> {
    let mut b = empty_bounds();
    let mut any = false;
    let add_edge = |e: &Edge, b: &mut Bounds| {
        let eb = edge_box(e);
        extend_bounds(b, Vec2::new(eb.min_x, eb.min_y), 0.0);
        extend_bounds(b, Vec2::new(eb.max_x, eb.max_y), 0.0);
    };
    match c {
        Class::Areas(areas) => {
            for e in super::boundary_edges(areas) {
                add_edge(&e, &mut b);
                any = true;
            }
        }
        Class::Paths(paths) => {
            for e in paths.iter().flatten() {
                add_edge(e, &mut b);
                any = true;
            }
        }
        Class::Points(points) => {
            for &p in points {
                extend_bounds(&mut b, p, 0.0);
                any = true;
            }
        }
        Class::None => {}
    }
    any.then_some(b)
}

fn boxes_meet(a: &Bounds, b: &Bounds) -> bool {
    a.min_x <= b.max_x + TOL
        && b.min_x <= a.max_x + TOL
        && a.min_y <= b.max_y + TOL
        && b.min_y <= a.max_y + TOL
}

/// A piece an overlay gives: the A object it came from, the B object, its
/// shape's class, and its share of the A object (area, length or count;
/// none when it is not of an A object).
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub a: Option<usize>,
    pub b: Option<usize>,
    pub class: Class,
    pub share: Option<f64>,
}

fn share(piece: &Class, whole: &Class) -> Option<f64> {
    let w = super::measure(whole);
    (w > 0.0).then(|| super::measure(piece) / w)
}

/// Kesişim (§5): every A object with every B area it meets, a piece each.
pub fn intersection(a: &[Class], b: &[Class]) -> Vec<Piece> {
    let b_boxes: Vec<Option<Bounds>> = b.iter().map(class_box).collect();
    let mut out = Vec::new();
    for (i, ca) in a.iter().enumerate() {
        let Some(ab) = class_box(ca) else {
            continue;
        };
        for (j, cb) in b.iter().enumerate() {
            let Class::Areas(areas) = cb else {
                continue;
            };
            if !b_boxes[j].as_ref().is_some_and(|bb| boxes_meet(&ab, bb)) {
                continue;
            }
            let piece = within(ca, areas, true);
            if !is_empty(&piece) {
                out.push(Piece {
                    a: Some(i),
                    b: Some(j),
                    share: share(&piece, ca),
                    class: piece,
                });
            }
        }
    }
    out
}

/// Every area of the B objects.
fn all_areas(list: &[Class]) -> Vec<Area> {
    list.iter()
        .flat_map(|c| match c {
            Class::Areas(a) => a.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// Fark (§5): each A object less everything B covers.
pub fn difference(a: &[Class], b: &[Class]) -> Vec<Piece> {
    let cut = union_all(&all_areas(b));
    a.iter()
        .enumerate()
        .filter_map(|(i, ca)| {
            let piece = within(ca, &cut, false);
            (!is_empty(&piece)).then(|| Piece {
                a: Some(i),
                b: None,
                share: share(&piece, ca),
                class: piece,
            })
        })
        .collect()
}

/// B's pieces outside A (`b` the A side's index none).
fn outside_of(b: &[Class], a: &[Class]) -> Vec<Piece> {
    difference(b, a)
        .into_iter()
        .map(|p| Piece {
            a: None,
            b: p.a,
            share: None,
            class: p.class,
        })
        .collect()
}

/// Simetrik fark (§5): A less B, then B less A.
pub fn sym_difference(a: &[Class], b: &[Class]) -> Vec<Piece> {
    let mut out = difference(a, b);
    out.extend(outside_of(b, a));
    out
}

/// Birleşim (§5): the meetings, then A less B, then B less A.
pub fn union(a: &[Class], b: &[Class]) -> Vec<Piece> {
    let mut out = intersection(a, b);
    out.extend(difference(a, b));
    out.extend(outside_of(b, a));
    out
}

/// Birleştir (§4): each group's objects as one (areas joined, paths and
/// points gathered), or, not `multi`, each joined part on its own. Groups
/// by their first object's place.
pub fn dissolve(classes: &[Class], groups: &[usize], multi: bool) -> Vec<(usize, Class)> {
    let mut order: Vec<usize> = Vec::new();
    for &g in groups {
        if !order.contains(&g) {
            order.push(g);
        }
    }
    let mut out = Vec::new();
    for g in order {
        let members: Vec<&Class> = classes
            .iter()
            .zip(groups)
            .filter(|(_, k)| **k == g)
            .map(|(c, _)| c)
            .collect();
        let areas: Vec<Area> = members
            .iter()
            .flat_map(|c| match c {
                Class::Areas(a) => a.clone(),
                _ => Vec::new(),
            })
            .collect();
        let paths: Vec<Vec<Edge>> = members
            .iter()
            .flat_map(|c| match c {
                Class::Paths(p) => p.clone(),
                _ => Vec::new(),
            })
            .collect();
        let points: Vec<Vec2> = members
            .iter()
            .flat_map(|c| match c {
                Class::Points(p) => p.clone(),
                _ => Vec::new(),
            })
            .collect();
        let joined = union_all(&areas);
        let mut parts = Vec::new();
        if multi {
            if !joined.is_empty() {
                parts.push(Class::Areas(joined));
            }
            if !paths.is_empty() {
                parts.push(Class::Paths(paths));
            }
            if !points.is_empty() {
                parts.push(Class::Points(points));
            }
        } else {
            // The joined parts from the largest (stable: the overlay's order between equals).
            let mut joined = joined;
            joined.sort_by(|a, b| net_area(b).total_cmp(&net_area(a)));
            parts.extend(joined.into_iter().map(|a| Class::Areas(vec![a])));
            parts.extend(paths.into_iter().map(|p| Class::Paths(vec![p])));
            parts.extend(points.into_iter().map(|p| Class::Points(vec![p])));
        }
        out.extend(parts.into_iter().map(|c| (g, c)));
    }
    out
}
