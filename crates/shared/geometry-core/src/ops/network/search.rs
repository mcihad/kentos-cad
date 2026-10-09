//! Searching a network (docs/adr/0209 §4): Dijkstra from one or more
//! places, the way pieces are drawn or (`reverse`) the costs of reaching the
//! places. A query's places that lie inside a piece cut it there for that
//! search only, so two places on one piece reach each other along it; a
//! place within the tolerance of a piece's end is that end's node. Barriers
//! are passed by nothing, nor are closed junctions (a closed valve).
//!
//! Equal costs: the lower node is taken first, and a node reached again at
//! the same cost keeps its first way; a node's pieces are tried in the
//! pieces' order (a virtual node: forward first). A search that has settled
//! enough targets stops after the nodes of that same cost. The buffers live in a
//! [`Searcher`] and are used again search after search: a tree given back
//! (`recycle`) costs nothing to the next.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use super::graph::{Graph, Location};
use crate::geom::bulge::bulge_of_sweep;
use crate::geom::intersect::{Edge, point_at};
use crate::vec2::Vec2;

const NONE: u32 = u32::MAX;

/// A search's question: from where (`origins`, at cost 0), with which cost, the way pieces are drawn or the reverse,
/// which places to read (`targets`) and which not to pass (`barriers`), how far at most and how many targets are enough.
#[derive(Clone, Copy, Debug)]
pub struct Query<'a> {
    pub cost: usize,
    pub reverse: bool,
    pub origins: &'a [Location],
    pub targets: &'a [Location],
    pub barriers: &'a [Location],
    pub cutoff: Option<f64>,
    pub enough: Option<usize>,
}

impl<'a> Query<'a> {
    /// From `origins` with cost `c`, nothing else asked.
    pub fn from(origins: &'a [Location], c: usize) -> Query<'a> {
        Query {
            cost: c,
            reverse: false,
            origins,
            targets: &[],
            barriers: &[],
            cutoff: None,
            enough: None,
        }
    }
}

/// A way along a piece: from `a` to `b` metres along it (backward when `b < a`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub piece: u32,
    pub a: f64,
    pub b: f64,
}

/// A way through the network: its spans in the way it is travelled and what it costs with the search's cost.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    pub spans: Vec<Span>,
    pub cost: f64,
}

/// The nodes a query adds: places inside pieces, and which nodes and places are barriers.
#[derive(Clone, Debug, Default)]
pub struct Places {
    base: u32,
    /// Each query place's node (origins, then targets, then barriers).
    pub node_of: Vec<u32>,
    /// Virtual nodes: their piece and offset (node `base + k`).
    vnodes: Vec<(u32, f64)>,
    blocked_v: Vec<bool>,
    /// The pieces cut by virtual nodes, by piece: their virtual nodes, by offset. A few, looked up by halving on every
    /// step of a search (a hash a step cost more).
    split: Vec<(u32, Vec<u32>)>,
    /// Graph nodes that are barriers in this query.
    blocked_n: Vec<u32>,
}

impl Places {
    /// The places of `origins`, `targets` and `barriers` on graph `g`.
    pub fn new(
        g: &Graph,
        origins: &[Location],
        targets: &[Location],
        barriers: &[Location],
    ) -> Places {
        let base = g.nodes.len() as u32;
        let tol = g.rules.tolerance;
        let mut places = Places {
            base,
            ..Places::default()
        };
        let mut at: HashMap<(u32, u64), u32> = HashMap::new();
        let all = origins
            .iter()
            .map(|l| (l, false))
            .chain(targets.iter().map(|l| (l, false)))
            .chain(barriers.iter().map(|l| (l, true)));
        for (loc, barrier) in all {
            let piece = &g.pieces[loc.piece as usize];
            let node = if loc.offset <= tol && loc.offset <= piece.len - loc.offset {
                piece.from
            } else if piece.len - loc.offset <= tol {
                piece.to
            } else {
                let key = (loc.piece, loc.offset.to_bits());
                *at.entry(key).or_insert_with(|| {
                    places.vnodes.push((loc.piece, loc.offset));
                    places.blocked_v.push(false);
                    base + (places.vnodes.len() - 1) as u32
                })
            };
            if barrier {
                if node >= base {
                    places.blocked_v[(node - base) as usize] = true;
                } else {
                    places.blocked_n.push(node);
                }
            }
            places.node_of.push(node);
        }
        let mut cut: Vec<(u32, u32)> = places
            .vnodes
            .iter()
            .enumerate()
            .map(|(k, (piece, _))| (*piece, base + k as u32))
            .collect();
        cut.sort_by_key(|&(piece, _)| piece);
        for (piece, v) in cut {
            match places.split.last_mut() {
                Some((last, list)) if *last == piece => list.push(v),
                _ => places.split.push((piece, vec![v])),
            }
        }
        let vnodes = &places.vnodes;
        for (_, list) in &mut places.split {
            list.sort_by(|a, b| {
                vnodes[(a - base) as usize]
                    .1
                    .total_cmp(&vnodes[(b - base) as usize].1)
            });
        }
        places.blocked_n.sort_unstable();
        places.blocked_n.dedup();
        places
    }

    /// A cut piece's virtual nodes, by offset.
    fn split_of(&self, piece: u32) -> Option<&Vec<u32>> {
        if self.split.is_empty() {
            return None;
        }
        self.split
            .binary_search_by_key(&piece, |(p, _)| *p)
            .ok()
            .map(|k| &self.split[k].1)
    }

    /// Nodes in all: the graph's and the virtual ones.
    pub fn count(&self) -> usize {
        self.base as usize + self.vnodes.len()
    }

    /// Whether node `n` is passed by nothing: a closed junction, a barrier.
    pub fn blocked(&self, g: &Graph, n: u32) -> bool {
        if n >= self.base {
            self.blocked_v[(n - self.base) as usize]
        } else {
            g.kinds[n as usize].closed || self.blocked_n.binary_search(&n).is_ok()
        }
    }

    /// Where node `n` is on `piece`: a virtual node's offset; a graph node's end of the piece (its start when
    /// `start`, for a piece whose two ends are the one node).
    pub fn offset(&self, g: &Graph, n: u32, piece: u32, start: bool) -> f64 {
        if n >= self.base {
            return self.vnodes[(n - self.base) as usize].1;
        }
        let p = &g.pieces[piece as usize];
        if start { 0.0 } else { p.len }
    }

    /// Node `n`'s steps: `(next, piece, forward, share)` along each piece it is on, `share` the piece's part travelled.
    pub fn steps(&self, g: &Graph, n: u32, mut f: impl FnMut(u32, u32, bool, f64)) {
        if n < self.base {
            for &entry in g.at_node(n) {
                let piece = entry / 2;
                let forward = entry % 2 == 0;
                let p = &g.pieces[piece as usize];
                match self.split_of(piece) {
                    Some(list) => {
                        let (next, share) = if forward {
                            let v = list[0];
                            (v, self.vnodes[(v - self.base) as usize].1 / p.len)
                        } else {
                            let v = list[list.len() - 1];
                            (v, (p.len - self.vnodes[(v - self.base) as usize].1) / p.len)
                        };
                        f(next, piece, forward, share);
                    }
                    None => f(if forward { p.to } else { p.from }, piece, forward, 1.0),
                }
            }
            return;
        }
        let (piece, o) = self.vnodes[(n - self.base) as usize];
        let p = &g.pieces[piece as usize];
        let Some(list) = self.split_of(piece) else {
            return;
        };
        let at = list.iter().position(|&v| v == n).unwrap_or(0);
        let (next, o_next) = match list.get(at + 1) {
            Some(&v) => (v, self.vnodes[(v - self.base) as usize].1),
            None => (p.to, p.len),
        };
        f(next, piece, true, (o_next - o) / p.len);
        let (prev, o_prev) = match at.checked_sub(1).map(|k| list[k]) {
            Some(v) => (v, self.vnodes[(v - self.base) as usize].1),
            None => (p.from, 0.0),
        };
        f(prev, piece, false, (o - o_prev) / p.len);
    }

    /// A piece's nodes along it, with their offsets: its start, the virtual nodes cutting it, its end (into `out`,
    /// cleared first).
    pub fn cuts_into(&self, g: &Graph, piece: u32, out: &mut Vec<(u32, f64)>) {
        let p = &g.pieces[piece as usize];
        out.clear();
        out.push((p.from, 0.0));
        if let Some(list) = self.split_of(piece) {
            out.extend(
                list.iter()
                    .map(|&v| (v, self.vnodes[(v - self.base) as usize].1)),
            );
        }
        out.push((p.to, p.len));
    }

    /// The nodes around offset `x` on `piece`: the last at or before it and the first at or after it, with their offsets.
    fn around(&self, g: &Graph, piece: u32, x: f64) -> ((u32, f64), (u32, f64)) {
        let p = &g.pieces[piece as usize];
        let mut lo = (p.from, 0.0);
        let mut hi = (p.to, p.len);
        if let Some(list) = self.split_of(piece) {
            for &v in list {
                let o = self.vnodes[(v - self.base) as usize].1;
                if o <= x {
                    lo = (v, o);
                } else {
                    hi = (v, o);
                    break;
                }
            }
        }
        (lo, hi)
    }
}

#[derive(Clone, Copy, Debug)]
struct Pred {
    node: u32,
    piece: u32,
    forward: bool,
}

/// A node waiting in the search, the cheapest first, then the lower node.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Waiting {
    cost: f64,
    node: u32,
}

impl Eq for Waiting {}

impl Ord for Waiting {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.node.cmp(&self.node))
    }
}

impl PartialOrd for Waiting {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A search's answer: every node's cost and the way to it.
pub struct Tree {
    pub cost: usize,
    pub reverse: bool,
    pub places: Places,
    dist: Vec<f64>,
    pred: Vec<Pred>,
    stamp: Vec<u32>,
    round: u32,
    /// The targets settled when the search stopped, in their order.
    pub settled: Vec<usize>,
}

/// The buffers searches use again.
#[derive(Default)]
pub struct Searcher {
    dist: Vec<f64>,
    pred: Vec<Pred>,
    stamp: Vec<u32>,
    round: u32,
    heap: BinaryHeap<Waiting>,
}

impl Searcher {
    pub fn new() -> Searcher {
        Searcher::default()
    }

    /// Searches `g` for `q`.
    pub fn search(&mut self, g: &Graph, q: &Query) -> Tree {
        let places = Places::new(g, q.origins, q.targets, q.barriers);
        let n = places.count();
        if self.stamp.len() < n {
            self.dist.resize(n, f64::INFINITY);
            self.pred.resize(
                n,
                Pred {
                    node: NONE,
                    piece: NONE,
                    forward: true,
                },
            );
            self.stamp.resize(n, 0);
        }
        self.round = self.round.wrapping_add(1);
        if self.round == 0 {
            self.stamp.iter_mut().for_each(|s| *s = 0);
            self.round = 1;
        }
        let round = self.round;
        let mut dist = std::mem::take(&mut self.dist);
        let mut pred = std::mem::take(&mut self.pred);
        let mut stamp = std::mem::take(&mut self.stamp);
        self.heap.clear();
        let get = |dist: &[f64], stamp: &[u32], k: u32| {
            if stamp[k as usize] == round {
                dist[k as usize]
            } else {
                f64::INFINITY
            }
        };
        for k in 0..q.origins.len() {
            let node = places.node_of[k];
            if get(&dist, &stamp, node) > 0.0 {
                dist[node as usize] = 0.0;
                stamp[node as usize] = round;
                pred[node as usize] = Pred {
                    node: NONE,
                    piece: NONE,
                    forward: true,
                };
                self.heap.push(Waiting { cost: 0.0, node });
            }
        }
        // The targets each node stands for.
        let mut wanted: HashMap<u32, Vec<usize>> = HashMap::new();
        for t in 0..q.targets.len() {
            wanted
                .entry(places.node_of[q.origins.len() + t])
                .or_default()
                .push(t);
        }
        let mut settled = Vec::new();
        let c = q.cost;
        // Once enough targets are settled, the search goes on only at the same cost: of equal costs every target is
        // settled, so the list's order decides among them.
        let mut last: Option<f64> = None;
        while let Some(Waiting { cost, node }) = self.heap.pop() {
            if cost > get(&dist, &stamp, node) {
                continue;
            }
            if q.cutoff.is_some_and(|m| cost > m) || last.is_some_and(|l| cost > l) {
                break;
            }
            if let Some(ts) = wanted.remove(&node) {
                settled.extend(ts);
                if last.is_none() && q.enough.is_some_and(|k| settled.len() >= k) {
                    last = Some(cost);
                }
            }
            places.steps(g, node, |next, piece, forward, share| {
                let along = g.cost(piece, c, forward != q.reverse);
                if !along.is_finite() || places.blocked(g, next) {
                    return;
                }
                let nd = cost + along * share;
                if nd < get(&dist, &stamp, next) {
                    dist[next as usize] = nd;
                    stamp[next as usize] = round;
                    pred[next as usize] = Pred {
                        node,
                        piece,
                        forward,
                    };
                    self.heap.push(Waiting {
                        cost: nd,
                        node: next,
                    });
                }
            });
        }
        settled.sort_unstable();
        Tree {
            cost: c,
            reverse: q.reverse,
            places,
            dist,
            pred,
            stamp,
            round,
            settled,
        }
    }

    /// Gives a tree's buffers back for the next search.
    pub fn recycle(&mut self, t: Tree) {
        if t.dist.len() >= self.dist.len() {
            self.dist = t.dist;
            self.pred = t.pred;
            self.stamp = t.stamp;
        }
    }
}

impl Tree {
    /// Node `n`'s cost; infinite when it was not reached.
    pub fn at(&self, n: u32) -> f64 {
        if (n as usize) < self.stamp.len() && self.stamp[n as usize] == self.round {
            self.dist[n as usize]
        } else {
            f64::INFINITY
        }
    }

    /// The cost of query place `k` (origins, targets, barriers in order).
    pub fn place_cost(&self, k: usize) -> f64 {
        self.at(self.places.node_of[k])
    }

    /// The cost of a piece's cost travelled the way the search reads it.
    fn along(&self, g: &Graph, piece: u32, forward: bool) -> f64 {
        g.cost(piece, self.cost, forward != self.reverse)
    }

    /// What reaching `loc` costs (the search's way), through which node and from which offset, and where on its piece
    /// `loc` is taken: a place within the tolerance of a piece's end is that end, as a query's places are (`Places::new`);
    /// none when it is not reached.
    fn best(&self, g: &Graph, loc: &Location) -> Option<(f64, u32, f64, f64)> {
        let p = &g.pieces[loc.piece as usize];
        let tol = g.rules.tolerance;
        let x = if loc.offset <= tol && loc.offset <= p.len - loc.offset {
            0.0
        } else if p.len - loc.offset <= tol {
            p.len
        } else {
            loc.offset
        };
        let ((lo, olo), (hi, ohi)) = self.places.around(g, loc.piece, x);
        let via_lo = self.at(lo) + self.along(g, loc.piece, true) * ((x - olo) / p.len);
        let via_hi = self.at(hi) + self.along(g, loc.piece, false) * ((ohi - x) / p.len);
        if via_lo <= via_hi && via_lo.is_finite() {
            Some((via_lo, lo, olo, x))
        } else if via_hi.is_finite() {
            Some((via_hi, hi, ohi, x))
        } else {
            None
        }
    }

    /// What reaching `loc` costs; infinite when it is not reached.
    pub fn cost_to(&self, g: &Graph, loc: &Location) -> f64 {
        self.best(g, loc).map_or(f64::INFINITY, |b| b.0)
    }

    /// The way between the search's origins and `loc`, in the way it is travelled: from an origin to `loc`, or in a
    /// reverse search from `loc` to an origin; none when `loc` is not reached.
    pub fn path_to(&self, g: &Graph, loc: &Location) -> Option<Path> {
        let (cost, node, o, x) = self.best(g, loc)?;
        let mut spans = Vec::new();
        let mut n = node;
        loop {
            let pr = self.pred[n as usize];
            if self.stamp[n as usize] != self.round || pr.node == NONE {
                break;
            }
            let start = self.places.offset(g, pr.node, pr.piece, pr.forward);
            let end = self.places.offset(g, n, pr.piece, !pr.forward);
            spans.push(Span {
                piece: pr.piece,
                a: start,
                b: end,
            });
            n = pr.node;
        }
        let last = Span {
            piece: loc.piece,
            a: o,
            b: x,
        };
        if self.reverse {
            // From `loc` back to the origin, each span the way it is travelled.
            let mut out = vec![Span {
                piece: last.piece,
                a: last.b,
                b: last.a,
            }];
            out.extend(spans.into_iter().map(|s| Span {
                piece: s.piece,
                a: s.b,
                b: s.a,
            }));
            out.retain(|s| s.a != s.b);
            return Some(Path { spans: out, cost });
        }
        spans.reverse();
        spans.push(last);
        spans.retain(|s| s.a != s.b);
        Some(Path { spans, cost })
    }
}

/// What travelling `spans` costs with cost `c`; infinite when a span cannot be travelled with it.
pub fn spans_cost(g: &Graph, spans: &[Span], c: usize) -> f64 {
    spans
        .iter()
        .map(|s| {
            let p = &g.pieces[s.piece as usize];
            g.cost(s.piece, c, s.b >= s.a) * ((s.b - s.a).abs() / p.len)
        })
        .sum()
}

/// The primitives of `spans`, in the way they are travelled.
pub fn spans_edges(g: &Graph, spans: &[Span]) -> Vec<Edge> {
    let mut out = Vec::new();
    for s in spans {
        g.span_edges(s.piece, s.a, s.b, &mut out);
    }
    out
}

/// A polyline of primitives in a row: its points and one bulge a segment; where one does not begin where the last
/// ended (a node's members within the tolerance) a straight segment joins them.
pub fn polyline_of(edges: &[Edge]) -> (Vec<Vec2>, Vec<f64>) {
    let mut pts: Vec<Vec2> = Vec::new();
    let mut bulges: Vec<f64> = Vec::new();
    for e in edges {
        let a = point_at(e, 0.0);
        match pts.last() {
            None => pts.push(a),
            Some(&last) if last != a => {
                bulges.push(0.0);
                pts.push(a);
            }
            _ => {}
        }
        bulges.push(match *e {
            Edge::Seg { .. } => 0.0,
            Edge::Arc { sweep, .. } => bulge_of_sweep(sweep),
        });
        pts.push(point_at(e, 1.0));
    }
    (pts, bulges)
}

#[cfg(test)]
mod tests {
    use super::super::graph::{EdgeIn, Graph};
    use super::super::rules::Rules;
    use super::*;
    use crate::api::json::Json;

    /// An edge: its id, its points and its direction field's value.
    type Street<'a> = (f64, &'a [(f64, f64)], Option<&'a str>);

    fn graph(direction: &str, edges: &[Street]) -> Graph {
        let rules = Rules::from_json(
            &Json::parse(&format!(
                r#"{{"connect":"ends","tolerance":0.01,"direction":{direction}}}"#
            ))
            .expect("JSON"),
        )
        .expect("rules");
        let edges: Vec<EdgeIn> = edges
            .iter()
            .map(|(id, pts, dir)| EdgeIn {
                id: *id,
                part: 0,
                path: pts
                    .windows(2)
                    .map(|w| Edge::Seg {
                        a: Vec2::new(w[0].0, w[0].1),
                        b: Vec2::new(w[1].0, w[1].1),
                    })
                    .collect(),
                direction: dir.map(str::to_owned),
                costs: Vec::new(),
                closed: false,
            })
            .collect();
        Graph::build(rules, &edges, &[]).0
    }

    #[test]
    fn a_one_way_street_makes_the_way_back_longer_and_places_on_one_piece_reach_each_other() {
        // A square block 100 m a side; the south side is one way to the east.
        let g = graph(
            r#"{"kind":"field","field":"yon","forward":["FT"]}"#,
            &[
                (1.0, &[(0.0, 0.0), (100.0, 0.0)], Some("FT")),
                (2.0, &[(100.0, 0.0), (100.0, 100.0)], None),
                (3.0, &[(100.0, 100.0), (0.0, 100.0)], None),
                (4.0, &[(0.0, 100.0), (0.0, 0.0)], None),
            ],
        );
        let at = |x: f64, y: f64| g.locate(Vec2::new(x, y), 1.0).expect("on the network");
        let (a, b) = (at(20.0, 0.0), at(80.0, 0.0));
        let mut s = Searcher::new();
        let east = s.search(&g, &Query::from(&[a], 0));
        let p = east.path_to(&g, &b).expect("reached");
        assert!((p.cost - 60.0).abs() < 1e-9, "{p:?}");
        assert_eq!(p.spans.len(), 1, "straight along the piece");
        s.recycle(east);
        let west = s.search(&g, &Query::from(&[b], 0));
        let back = west.path_to(&g, &a).expect("reached");
        // 20 east, then around the block: 100 + 100 + 100, then 20 east again.
        assert!((back.cost - 340.0).abs() < 1e-9, "{back:?}");
        assert!((spans_cost(&g, &back.spans, 0) - 340.0).abs() < 1e-9);
        let (pts, bulges) = polyline_of(&spans_edges(&g, &back.spans));
        assert_eq!(pts.first(), Some(&Vec2::new(80.0, 0.0)));
        assert_eq!(pts.last(), Some(&Vec2::new(20.0, 0.0)));
        assert_eq!(pts.len(), bulges.len() + 1);
    }

    #[test]
    fn a_reverse_search_gives_the_way_to_the_origin_and_a_barrier_cuts_the_piece() {
        let g = graph(
            r#"{"kind":"both"}"#,
            &[
                (1.0, &[(0.0, 0.0), (100.0, 0.0)], None),
                (2.0, &[(100.0, 0.0), (100.0, 50.0)], None),
            ],
        );
        let at = |x: f64, y: f64| g.locate(Vec2::new(x, y), 1.0).expect("on the network");
        let mut s = Searcher::new();
        let to_corner = s.search(
            &g,
            &Query {
                reverse: true,
                ..Query::from(&[at(100.0, 30.0)], 0)
            },
        );
        let p = to_corner.path_to(&g, &at(10.0, 0.0)).expect("reached");
        assert!((p.cost - 120.0).abs() < 1e-9);
        assert_eq!(
            g.point_on(p.spans[0].piece, p.spans[0].a),
            Vec2::new(10.0, 0.0),
            "from the place to the origin"
        );
        s.recycle(to_corner);
        let barred = s.search(
            &g,
            &Query {
                barriers: &[at(50.0, 0.0)],
                ..Query::from(&[at(100.0, 30.0)], 0)
            },
        );
        assert!(barred.path_to(&g, &at(10.0, 0.0)).is_none());
        assert!(
            (barred.cost_to(&g, &at(60.0, 0.0)) - 70.0).abs() < 1e-9,
            "up to the barrier"
        );
    }
}
