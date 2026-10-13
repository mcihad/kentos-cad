//! A polyline's widths carried to an edit's new edges (docs/adr/0258 §2), as
//! elevations are carried to its vertices (`ops::elevation`). An edit's
//! geometry comes from the core without widths; each edge of the result takes
//! a start and an end width from the polylines the edit started from, its
//! start and its end each by the first of these rules that reaches it:
//!
//! 1. the edge runs along a source edge (its ends and its middle on it,
//!    within [`ON`]): that edge's widths at the edge's ends, linearly along it
//!    (by length on a straight edge, by angle on an arc): a cut, a new vertex,
//!    a piece of a trimmed or broken line, a turned one (its widths swap);
//! 2. `same`, given for an edit that moves an object's own vertices (a grip,
//!    Esnet, a typed coordinate) and for Ötele's copy, with as many vertices:
//!    the widths of the edge in the same place of its order;
//! 3. the end on a source vertex: the width the source has there, the start
//!    of the edge leaving it for the edge's start, the end of the edge
//!    arriving at it for the edge's end (Köşe sil's two edges made one); a
//!    line's end without that edge takes the other one's;
//! 4. the end on a source edge: that edge's width there (a fillet's or a
//!    chamfer's new edge between two cut edges);
//! 5. for Ötele, the width at the closest point of the sources.
//!
//! An end no rule reaches has width 0. A result whose edges are all 0 has no
//! widths (`None`).

use crate::geom::bulge::{bulge_arc, bulge_at};
use crate::geom::intersect::{Edge, closest_on_edge, point_at};
use crate::jsmath::js_hypot;
use crate::ops::elevation::{Carry, ON};
use crate::vec2::Vec2;

/// A polyline before the edit, with its widths: one pair for each vertex,
/// the start and the end of the edge leaving it (the last a closed line's
/// closing edge's).
#[derive(Clone, Debug, PartialEq)]
pub struct Widened {
    pub pts: Vec<Vec2>,
    pub bulges: Option<Vec<f64>>,
    pub closed: bool,
    pub widths: Vec<[f64; 2]>,
}

/// The edge from vertex `i` to the next of a path (`closed`: the last to the first).
fn edge_of(pts: &[Vec2], bulges: Option<&[f64]>, i: usize) -> Edge {
    let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
    match bulge_arc(a, b, bulge_at(bulges, i)) {
        Some(arc) => Edge::Arc {
            c: arc.c,
            r: arc.r,
            a0: arc.a0,
            sweep: arc.sweep,
        },
        None => Edge::Seg { a, b },
    }
}

fn edge_count(n: usize, closed: bool) -> usize {
    if closed { n } else { n.saturating_sub(1) }
}

/// A source's edges, each with its start and end width.
fn edges(s: &Widened) -> Vec<(Edge, [f64; 2])> {
    let n = s.pts.len();
    (0..edge_count(n, s.closed))
        .map(|i| {
            let w = s.widths.get(i).copied().unwrap_or([0.0, 0.0]);
            (edge_of(&s.pts, s.bulges.as_deref(), i), w)
        })
        .collect()
}

fn lerp([a, b]: [f64; 2], t: f64) -> f64 {
    a + (b - a) * t
}

fn near(p: Vec2, q: Vec2) -> bool {
    js_hypot(p.x - q.x, p.y - q.y) <= ON
}

/// Rule 3: the width a source has at its vertex `p`, leaving it (`start`) or arriving.
fn at_vertex(p: Vec2, start: bool, sources: &[Widened]) -> Option<f64> {
    for s in sources {
        let n = s.pts.len();
        let count = edge_count(n, s.closed);
        if count == 0 {
            continue;
        }
        for (k, &q) in s.pts.iter().enumerate() {
            if !near(p, q) {
                continue;
            }
            let leaving = (k < count).then(|| s.widths.get(k).map(|w| w[0])).flatten();
            let arriving = if k > 0 {
                s.widths.get(k - 1).map(|w| w[1])
            } else if s.closed {
                s.widths.get(n - 1).map(|w| w[1])
            } else {
                None
            };
            let found = if start {
                leaving.or(arriving)
            } else {
                arriving.or(leaving)
            };
            if found.is_some() {
                return found;
            }
        }
    }
    None
}

/// Widths for the edges of an edit's result (`pts`, `bulges`, `closed`),
/// carried from `sources` by the module's rules; `same` is the object itself
/// before the edit, when the result replaces it. `None` when every edge has 0.
pub fn carry_widths(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    closed: bool,
    same: Option<&Widened>,
    sources: &[Widened],
    carry: Carry,
) -> Option<Vec<[f64; 2]>> {
    let n = pts.len();
    let all: Vec<Vec<(Edge, [f64; 2])>> = sources.iter().map(edges).collect();
    let same = same.filter(|s| s.pts.len() == n);
    let mut out = vec![[0.0, 0.0]; n];
    for (i, slot) in out.iter_mut().enumerate().take(edge_count(n, closed)) {
        let edge = edge_of(pts, bulges, i);
        let (a, b, mid) = (
            point_at(&edge, 0.0),
            point_at(&edge, 1.0),
            point_at(&edge, 0.5),
        );
        // 1. Along a source edge.
        let along = all.iter().flatten().find_map(|(e, w)| {
            let (ha, hb, hm) = (
                closest_on_edge(e, a),
                closest_on_edge(e, b),
                closest_on_edge(e, mid),
            );
            (ha.d <= ON && hb.d <= ON && hm.d <= ON).then(|| [lerp(*w, ha.t), lerp(*w, hb.t)])
        });
        if let Some(w) = along {
            *slot = w;
            continue;
        }
        // 2. The same object's edge in the same place of its order.
        if let Some(w) = same.and_then(|s| s.widths.get(i)) {
            *slot = *w;
            continue;
        }
        let end = |p: Vec2, start: bool| -> f64 {
            // 3. On a source vertex.
            if let Some(w) = at_vertex(p, start, sources) {
                return w;
            }
            // 4. On a source edge.
            for (e, w) in all.iter().flatten() {
                let hit = closest_on_edge(e, p);
                if hit.d <= ON {
                    return lerp(*w, hit.t);
                }
            }
            // 5. Ötele: the closest point of the sources.
            if carry == Carry::Offset {
                let mut best: Option<(f64, f64)> = None;
                for (e, w) in all.iter().flatten() {
                    let hit = closest_on_edge(e, p);
                    if best.is_none_or(|(d, _)| hit.d < d) {
                        best = Some((hit.d, lerp(*w, hit.t)));
                    }
                }
                if let Some((_, w)) = best {
                    return w;
                }
            }
            0.0
        };
        *slot = [end(a, true), end(b, false)];
    }
    out.iter()
        .any(|[a, b]| *a != 0.0 || *b != 0.0)
        .then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    /// A 10 m line from 0 to 2 m wide, then a 10 m edge 2 m wide.
    fn tapered() -> Widened {
        Widened {
            pts: vec![v(0.0, 0.0), v(10.0, 0.0), v(10.0, 10.0)],
            bulges: None,
            closed: false,
            widths: vec![[0.0, 2.0], [2.0, 2.0], [0.0, 0.0]],
        }
    }

    #[test]
    fn a_cut_piece_takes_the_widths_where_it_lies() {
        let s = tapered();
        let got = carry_widths(
            &[v(2.5, 0.0), v(10.0, 0.0), v(10.0, 4.0)],
            None,
            false,
            None,
            &[s],
            Carry::Along,
        );
        assert_eq!(got, Some(vec![[0.5, 2.0], [2.0, 2.0], [0.0, 0.0]]));
    }

    #[test]
    fn a_turned_line_swaps_each_edges_widths() {
        let s = tapered();
        let got = carry_widths(
            &[v(10.0, 10.0), v(10.0, 0.0), v(0.0, 0.0)],
            None,
            false,
            None,
            &[s],
            Carry::Along,
        );
        assert_eq!(got, Some(vec![[2.0, 2.0], [2.0, 0.0], [0.0, 0.0]]));
    }

    #[test]
    fn two_edges_made_one_take_the_start_and_the_end_of_their_vertices() {
        let s = tapered();
        let got = carry_widths(
            &[v(0.0, 0.0), v(10.0, 10.0)],
            None,
            false,
            None,
            &[s],
            Carry::Along,
        );
        assert_eq!(got, Some(vec![[0.0, 2.0], [0.0, 0.0]]));
    }

    #[test]
    fn a_moved_vertex_keeps_the_widths_in_its_place() {
        let s = tapered();
        let got = carry_widths(
            &[v(0.0, 0.0), v(12.0, 1.0), v(10.0, 10.0)],
            None,
            false,
            Some(&s),
            std::slice::from_ref(&s),
            Carry::Along,
        );
        assert_eq!(got, Some(vec![[0.0, 2.0], [2.0, 2.0], [0.0, 0.0]]));
    }

    #[test]
    fn a_closed_lines_closing_edge_carries() {
        let s = Widened {
            pts: vec![v(0.0, 0.0), v(4.0, 0.0), v(4.0, 4.0), v(0.0, 4.0)],
            bulges: None,
            closed: true,
            widths: vec![[1.0, 1.0], [1.0, 1.0], [1.0, 1.0], [1.0, 3.0]],
        };
        // The ring opened at (0, 2): from there round to the same place.
        let got = carry_widths(
            &[
                v(0.0, 2.0),
                v(0.0, 0.0),
                v(4.0, 0.0),
                v(4.0, 4.0),
                v(0.0, 4.0),
                v(0.0, 2.0),
            ],
            None,
            false,
            None,
            &[s],
            Carry::Along,
        );
        assert_eq!(
            got,
            Some(vec![
                [2.0, 3.0],
                [1.0, 1.0],
                [1.0, 1.0],
                [1.0, 1.0],
                [1.0, 2.0],
                [0.0, 0.0],
            ])
        );
    }

    #[test]
    fn nothing_reached_gives_no_widths() {
        let got = carry_widths(
            &[v(50.0, 50.0), v(60.0, 50.0)],
            None,
            false,
            None,
            &[tapered()],
            Carry::Along,
        );
        assert_eq!(got, None);
    }
}
