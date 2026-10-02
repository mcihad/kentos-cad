//! İzle (docs/adr/0161 §1, §5): the shortest way along visible line work
//! between two points on it, for the path tools. The line work is cut
//! wherever it meets itself, as Tarama's and İçine tıklayarak alan's faces
//! are (`arrangement::build`), so a path turns at crossings; the way is the
//! shortest by length (chords on straight edges, arc lengths on arcs), ties
//! going to the vertex found first. The path's corners are the given ends,
//! the line work's own vertices bit for bit and the crossings where it
//! turns; a crossing it goes straight through is no corner, and an arc
//! keeps its circle. The independent reference is
//! `scripts/fixtures/trace_cases.py`.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::api::Op;
use crate::entity::{Entity, Shape};
use crate::geom::arrangement::{
    Built, Source, TOL, build, edge_box, edge_len, reverse_edge, sub_edge,
};
use crate::geom::bulge::bulge_of_sweep;
use crate::geom::intersect::{Edge, closest_on_edge};
use crate::geom::overlay::{joinable, localize, origin_of};
use crate::geometry::Bounds;
use crate::jsmath::js_hypot;
use crate::op;
use crate::ops::areas::line_source;
use crate::store::rtree::PackedTree;
use crate::vec2::Vec2;

/// A traced way: its corners (the given ends first and last), one bulge
/// per edge, and its length.
#[derive(Clone, Debug, PartialEq)]
pub struct Traced {
    pub pts: Vec<Vec2>,
    pub bulges: Vec<f64>,
    pub length: f64,
}

crate::json_struct!(out Traced {
    pts,
    bulges,
    length
});

/// The kinds İzle follows: lines, paths, areas' rings, arcs and circles
/// (ellipses and curves only have an approximate outline, docs/adr/0149).
pub fn traced_kind(shape: &Shape) -> bool {
    matches!(
        shape,
        Shape::Line { .. }
            | Shape::Polyline { .. }
            | Shape::Polygon { .. }
            | Shape::Arc { .. }
            | Shape::Circle { .. }
    )
}

/// The line work cut at every meeting point: the pieces between vertices,
/// the pieces at each vertex and their boxes, built once and asked many times.
pub struct TraceGraph {
    built: Built,
    origin: Vec2,
    /// The pieces at each vertex: (piece, true when it leaves from the vertex).
    at: Vec<Vec<(usize, bool)>>,
    tree: PackedTree,
}

/// Where a point lies on the graph: on a vertex, or inside a piece at a parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Spot {
    Vertex(usize),
    Inside(usize, f64),
}

/// An edge of the search: to a node, along a local edge (with its length),
/// through a vertex when it ends at one.
#[derive(Clone, Copy, Debug)]
struct Step {
    to: usize,
    edge: Edge,
    length: f64,
}

/// A node waiting in the search, nearest first; equal distances go to the
/// lower node, so the search is the same wherever it runs.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Waiting {
    dist: f64,
    node: usize,
}

impl Eq for Waiting {}

impl Ord for Waiting {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .dist
            .total_cmp(&self.dist)
            .then_with(|| other.node.cmp(&self.node))
    }
}

impl PartialOrd for Waiting {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl TraceGraph {
    /// The graph of overlay sources' line work.
    pub fn new(lines: &[Source]) -> TraceGraph {
        let cuts: Vec<Source> = lines
            .iter()
            .map(|s| Source {
                edges: s.edges.clone(),
                points: s.points.clone(),
                cut: Some(true),
            })
            .collect();
        let origin = origin_of(&cuts);
        let local = localize(&cuts, origin);
        let built = build(&local, &cuts, origin);
        let mut at = vec![Vec::new(); built.verts.pos.len()];
        for (i, p) in built.pieces.iter().enumerate() {
            at[p.from].push((i, true));
            at[p.to].push((i, false));
        }
        let boxes: Vec<(u32, Bounds)> = built
            .pieces
            .iter()
            .enumerate()
            .map(|(i, p)| (i as u32, edge_box(&p.edge)))
            .collect();
        let tree = PackedTree::build(&boxes);
        TraceGraph {
            built,
            origin,
            at,
            tree,
        }
    }

    /// The graph of the objects İzle follows among `entities`.
    pub fn of_entities(entities: &[Entity]) -> TraceGraph {
        let traced: Vec<Entity> = entities
            .iter()
            .filter(|e| traced_kind(&e.shape))
            .cloned()
            .collect();
        TraceGraph::new(&[line_source(&traced)])
    }

    fn local(&self, p: Vec2) -> Vec2 {
        Vec2::new(p.x - self.origin.x, p.y - self.origin.y)
    }

    /// Where `p` lies on the line work (within 1 µm): on a vertex when
    /// one is that near, else inside the nearest piece; none when off it.
    fn spot(&self, p: Vec2) -> Option<Spot> {
        let q = self.local(p);
        let reach = Bounds {
            min_x: q.x - TOL,
            min_y: q.y - TOL,
            max_x: q.x + TOL,
            max_y: q.y + TOL,
        };
        let mut hits = Vec::new();
        self.tree.search(&reach, &mut hits);
        hits.sort_unstable();
        let mut best: Option<(f64, usize, f64)> = None;
        for &h in &hits {
            let piece = &self.built.pieces[h as usize];
            for v in [piece.from, piece.to] {
                let w = self.built.verts.pos[v];
                if js_hypot(w.x - q.x, w.y - q.y) <= TOL {
                    return Some(Spot::Vertex(v));
                }
            }
            let c = closest_on_edge(&piece.edge, q);
            if c.d <= TOL && best.is_none_or(|(d, ..)| c.d < d) {
                best = Some((c.d, h as usize, c.t));
            }
        }
        best.map(|(_, piece, t)| Spot::Inside(piece, t))
    }

    /// The point of the line work nearest to `p` within `reach` metres (the
    /// first piece of equally near ones): a pointer near a line is put on it
    /// (docs/adr/0161 §1). A piece's end is its vertex's place, bit for bit.
    pub fn nearest(&self, p: Vec2, reach: f64) -> Option<Vec2> {
        let q = self.local(p);
        let around = Bounds {
            min_x: q.x - reach,
            min_y: q.y - reach,
            max_x: q.x + reach,
            max_y: q.y + reach,
        };
        let mut hits = Vec::new();
        self.tree.search(&around, &mut hits);
        hits.sort_unstable();
        let mut best: Option<(f64, usize, f64)> = None;
        for &h in &hits {
            let c = closest_on_edge(&self.built.pieces[h as usize].edge, q);
            if c.d <= reach && best.is_none_or(|(d, ..)| c.d < d) {
                best = Some((c.d, h as usize, c.t));
            }
        }
        let (_, piece, t) = best?;
        let piece = &self.built.pieces[piece];
        Some(if t <= 0.0 {
            self.place(piece.from)
        } else if t >= 1.0 {
            self.place(piece.to)
        } else {
            let at = closest_on_edge(&piece.edge, q).p;
            Vec2::new(at.x + self.origin.x, at.y + self.origin.y)
        })
    }

    /// The vertex's place: its exact input coordinates when it is an input
    /// corner, else where the cutting put it.
    fn place(&self, v: usize) -> Vec2 {
        match self.built.verts.input[v] {
            Some(p) => p,
            None => {
                let p = self.built.verts.pos[v];
                Vec2::new(p.x + self.origin.x, p.y + self.origin.y)
            }
        }
    }

    /// The steps out of `node`: a vertex's pieces, either way; the two
    /// virtual ends' halves of the piece they lie inside.
    fn steps(&self, node: usize, ends: &[(usize, Spot); 2], out: &mut Vec<Step>) {
        out.clear();
        let vertices = self.built.verts.pos.len();
        if node < vertices {
            for &(i, leaves) in &self.at[node] {
                let p = &self.built.pieces[i];
                let (to, edge) = if leaves {
                    (p.to, p.edge)
                } else {
                    (p.from, reverse_edge(&p.edge))
                };
                out.push(Step {
                    to,
                    edge,
                    length: edge_len(&edge),
                });
            }
        }
        // The end inside a piece: reached from either end of its piece.
        let (b, b_spot) = ends[1];
        if let Spot::Inside(piece, t) = b_spot {
            let p = &self.built.pieces[piece];
            if node == p.from || node == p.to {
                let edge = if node == p.from {
                    sub_edge(&p.edge, 0.0, t)
                } else {
                    sub_edge(&p.edge, 1.0, t)
                };
                out.push(Step {
                    to: b,
                    edge,
                    length: edge_len(&edge),
                });
            }
        }
        // The start inside a piece: out to either end of its piece, or
        // straight to the other end when that lies inside the same piece.
        let (a, a_spot) = ends[0];
        if node == a
            && let Spot::Inside(piece, t) = a_spot
        {
            let p = &self.built.pieces[piece];
            for (to, edge) in [
                (p.from, sub_edge(&p.edge, t, 0.0)),
                (p.to, sub_edge(&p.edge, t, 1.0)),
            ] {
                out.push(Step {
                    to,
                    edge,
                    length: edge_len(&edge),
                });
            }
            if let Spot::Inside(other, u) = b_spot
                && other == piece
            {
                let edge = sub_edge(&p.edge, t, u);
                out.push(Step {
                    to: b,
                    edge,
                    length: edge_len(&edge),
                });
            }
        }
    }

    /// The shortest way along the line work from `a` to `b`; none when
    /// either is off the line work, they are the same point, or nothing
    /// joins them.
    pub fn path(&self, a: Vec2, b: Vec2) -> Option<Traced> {
        if js_hypot(a.x - b.x, a.y - b.y) <= TOL {
            return None;
        }
        let (sa, sb) = (self.spot(a)?, self.spot(b)?);
        let vertices = self.built.verts.pos.len();
        let node = |s: Spot, virtual_node: usize| match s {
            Spot::Vertex(v) => v,
            Spot::Inside(..) => virtual_node,
        };
        let (start, goal) = (node(sa, vertices), node(sb, vertices + 1));
        if start == goal {
            return None;
        }
        let ends = [(start, sa), (goal, sb)];
        let count = vertices + 2;
        let mut dist = vec![f64::INFINITY; count];
        let mut came: Vec<Option<(usize, Edge, f64)>> = vec![None; count];
        let mut heap = BinaryHeap::new();
        dist[start] = 0.0;
        heap.push(Waiting {
            dist: 0.0,
            node: start,
        });
        let mut out = Vec::new();
        while let Some(Waiting { dist: d, node: n }) = heap.pop() {
            if d > dist[n] {
                continue;
            }
            if n == goal {
                break;
            }
            self.steps(n, &ends, &mut out);
            for step in &out {
                let next = d + step.length;
                if next < dist[step.to] {
                    dist[step.to] = next;
                    came[step.to] = Some((n, step.edge, step.length));
                    heap.push(Waiting {
                        dist: next,
                        node: step.to,
                    });
                }
            }
        }
        if !dist[goal].is_finite() {
            return None;
        }
        // The way back from the goal, then forwards: each edge with the node it reaches.
        let mut walk: Vec<(Edge, usize)> = Vec::new();
        let mut at = goal;
        let mut length = 0.0;
        while at != start {
            let (from, edge, l) = came[at]?;
            walk.push((edge, at));
            length += l;
            at = from;
        }
        walk.reverse();
        Some(self.corners(a, b, &walk, vertices, length))
    }

    /// The way's corners: the given ends, the input vertices bit for bit,
    /// and the cut points where the way turns; edges it goes straight on
    /// through a cut point are one (the faces' `merge_straight`).
    fn corners(
        &self,
        a: Vec2,
        b: Vec2,
        walk: &[(Edge, usize)],
        vertices: usize,
        length: f64,
    ) -> Traced {
        let mut edges: Vec<Edge> = Vec::with_capacity(walk.len());
        let mut pts = vec![a];
        for (k, &(edge, node)) in walk.iter().enumerate() {
            // The vertex between the last edge and this one: a cut point gone
            // straight on through is no corner, and its two edges are one.
            let through = k.checked_sub(1).map(|j| walk[j].1);
            let merged = match (edges.last(), through) {
                (Some(last), Some(v)) if v < vertices && self.built.verts.input[v].is_none() => {
                    joinable(last, &edge)
                }
                _ => None,
            };
            match (merged, edges.last_mut()) {
                (Some(joined), Some(last)) => {
                    *last = joined;
                    pts.pop();
                }
                _ => edges.push(edge),
            }
            pts.push(if node < vertices { self.place(node) } else { b });
        }
        if let Some(end) = pts.last_mut() {
            *end = b;
        }
        let bulges = edges
            .iter()
            .map(|e| match *e {
                Edge::Arc { sweep, .. } => bulge_of_sweep(sweep),
                Edge::Seg { .. } => 0.0,
            })
            .collect();
        Traced {
            pts,
            bulges,
            length,
        }
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("tracePath", |lines: Vec<Entity>, a: Vec2, b: Vec2| {
        TraceGraph::of_entities(&lines).path(a, b)
    }),
    op!("traceNearest", |lines: Vec<Entity>, p: Vec2, reach: f64| {
        TraceGraph::of_entities(&lines).nearest(p, reach)
    }),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::{FromJson, Json};

    fn line(a: (f64, f64), b: (f64, f64)) -> Entity {
        Entity::new(Shape::Line {
            a: Vec2::new(a.0, a.1),
            b: Vec2::new(b.0, b.1),
        })
    }

    #[test]
    fn a_way_turns_at_a_crossing_and_keeps_the_input_corners() {
        // An L of two lines and a third crossing the first at (5, 0).
        let g = TraceGraph::of_entities(&[
            line((0.0, 0.0), (10.0, 0.0)),
            line((10.0, 0.0), (10.0, 10.0)),
            line((5.0, -5.0), (5.0, 5.0)),
        ]);
        let t = g
            .path(Vec2::new(5.0, 5.0), Vec2::new(10.0, 4.0))
            .expect("joined");
        assert_eq!(
            t.pts,
            [
                Vec2::new(5.0, 5.0),
                Vec2::new(5.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(10.0, 4.0)
            ]
        );
        assert_eq!(t.bulges, [0.0, 0.0, 0.0]);
        assert!((t.length - 14.0).abs() < 1e-12);
    }

    #[test]
    fn a_crossing_gone_straight_through_is_no_corner() {
        let g = TraceGraph::of_entities(&[
            line((0.0, 0.0), (10.0, 0.0)),
            line((5.0, -5.0), (5.0, 5.0)),
        ]);
        let t = g
            .path(Vec2::new(1.0, 0.0), Vec2::new(9.0, 0.0))
            .expect("joined");
        assert_eq!(t.pts, [Vec2::new(1.0, 0.0), Vec2::new(9.0, 0.0)]);
        assert!(
            g.path(Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0)).is_none(),
            "off the line work"
        );
    }

    /// The reference's cases (scripts/fixtures/trace_cases.py): the corners
    /// and bulges within its tolerances, its exact corners bit for bit.
    #[test]
    fn every_case_is_traced_as_the_reference_traces_it() {
        let file = Json::parse(include_str!("../../../../../fixtures/trace/v1/trace.json"))
            .expect("trace.json reads");
        let Json::Arr(cases) = file.get("paths") else {
            panic!("paths")
        };
        assert!(cases.len() >= 10, "{} cases", cases.len());
        let mut off = Vec::new();
        for case in cases {
            let name = String::from_json(case.get("name")).unwrap_or_default();
            let lines = Vec::<Entity>::from_json(case.get("lines")).expect("lines");
            let a = Vec2::from_json(case.get("a")).expect("a");
            let b = Vec2::from_json(case.get("b")).expect("b");
            let got = TraceGraph::of_entities(&lines).path(a, b);
            let want = case.get("expected");
            if matches!(want, Json::Null) {
                if got.is_some() {
                    off.push(format!("{name}: {got:?}, beklenen yok"));
                }
                continue;
            }
            let Some(got) = got else {
                off.push(format!("{name}: yol yok"));
                continue;
            };
            off.extend(compare(&name, &got, want));
        }
        assert!(off.is_empty(), "{}", off.join("\n"));
    }

    /// The nearest points of the same reference: within 1e-9 m, a vertex bit for bit.
    #[test]
    fn every_nearest_point_is_the_reference_s() {
        let file = Json::parse(include_str!("../../../../../fixtures/trace/v1/trace.json"))
            .expect("trace.json reads");
        let Json::Arr(cases) = file.get("nearest") else {
            panic!("nearest")
        };
        assert!(cases.len() >= 5, "{} cases", cases.len());
        let mut off = Vec::new();
        for case in cases {
            let name = String::from_json(case.get("name")).unwrap_or_default();
            let lines = Vec::<Entity>::from_json(case.get("lines")).expect("lines");
            let p = Vec2::from_json(case.get("p")).expect("p");
            let reach = f64::from_json(case.get("reach")).expect("reach");
            let got = TraceGraph::of_entities(&lines).nearest(p, reach);
            let want = Option::<Vec2>::from_json(case.get("expected")).expect("expected");
            let exact = bool::from_json(case.get("exact")).expect("exact");
            let ok = match (got, want) {
                (None, None) => true,
                (Some(g), Some(w)) if exact => g == w,
                (Some(g), Some(w)) => js_hypot(g.x - w.x, g.y - w.y) <= 1e-9,
                _ => false,
            };
            if !ok {
                off.push(format!("{name}: {got:?}, beklenen {want:?}"));
            }
        }
        assert!(off.is_empty(), "{}", off.join("\n"));
    }

    /// Zincir's cases from the same reference: the members in order, exactly.
    #[test]
    fn every_chain_is_walked_as_the_reference_walks_it() {
        use crate::ops::join::{ChainObject, chain};
        let file = Json::parse(include_str!("../../../../../fixtures/trace/v1/trace.json"))
            .expect("trace.json reads");
        let Json::Arr(cases) = file.get("chains") else {
            panic!("chains")
        };
        assert!(cases.len() >= 7, "{} cases", cases.len());
        let mut off = Vec::new();
        for case in cases {
            let name = String::from_json(case.get("name")).unwrap_or_default();
            let objects = Vec::<ChainObject>::from_json(case.get("objects")).expect("objects");
            let seed = usize::from_json(case.get("seed")).expect("seed");
            let tol = f64::from_json(case.get("tol")).expect("tol");
            let got = chain(&objects, seed, tol);
            let want = case.get("expected");
            let members = Vec::<usize>::from_json(want.get("members")).expect("members");
            let locked = bool::from_json(want.get("locked")).expect("locked");
            let closed = bool::from_json(want.get("closed")).expect("closed");
            if got.members != members || got.locked != locked || got.closed != closed {
                off.push(format!("{name}: {got:?}"));
            }
        }
        assert!(off.is_empty(), "{}", off.join("\n"));
    }

    fn compare(name: &str, got: &Traced, want: &Json) -> Vec<String> {
        let mut off = Vec::new();
        let options = Vec::<Json>::from_json(want.get("ways")).expect("ways");
        let fits = options.iter().any(|w| {
            let pts = Vec::<Vec2>::from_json(w.get("pts")).expect("pts");
            let bulges = Vec::<f64>::from_json(w.get("bulges")).expect("bulges");
            let exact = Vec::<usize>::from_json(w.get("exact")).expect("exact");
            pts.len() == got.pts.len()
                && pts.iter().zip(&got.pts).enumerate().all(|(i, (p, q))| {
                    if exact.contains(&i) {
                        p == q
                    } else {
                        js_hypot(p.x - q.x, p.y - q.y) <= 1e-9
                    }
                })
                && bulges.len() == got.bulges.len()
                && bulges
                    .iter()
                    .zip(&got.bulges)
                    .all(|(p, q)| (p - q).abs() <= 1e-12)
        });
        if !fits {
            off.push(format!("{name}: {:?} {:?}", got.pts, got.bulges));
        }
        let length = f64::from_json(want.get("length")).expect("length");
        if (got.length - length).abs() > 1e-9 {
            off.push(format!("{name}: uzunluk {} ≠ {length}", got.length));
        }
        off
    }
}
