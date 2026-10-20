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
//!
//! With turns (docs/adr/0521 §5) a search goes by states, not nodes: an arc
//! (a piece one way) as it reaches its node (`piece * 2`, `piece * 2 + 1`),
//! the same at a virtual node, and each node as a start (no way in); a turn
//! is tried on every step, forbidden ones not taken, delays added; at a
//! virtual node a way goes straight on. A reverse search's state is the arc
//! a way leaves its node by. With a time (`at`) a cost's time profile is read
//! at the time each step is set off (a reverse search: arrived), the
//! departure (the arrival) plus (minus) the cost so far.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use super::graph::{Graph, Location};
use crate::geom::bulge::bulge_of_sweep;
use crate::geom::intersect::{Edge, point_at};
use crate::vec2::Vec2;

const NONE: u32 = u32::MAX;
pub static PUSHES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
pub static POPS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// A search's question: from where (`origins`, at cost 0), with which cost, the way pieces are drawn or the reverse,
/// which places to read (`targets`) and which not to pass (`barriers`), how far at most, how many targets are enough
/// and at what time of day (minutes; a reverse search's arrival, else its departure) when the cost has a time profile.
#[derive(Clone, Copy, Debug)]
pub struct Query<'a> {
    pub cost: usize,
    pub reverse: bool,
    pub origins: &'a [Location],
    pub targets: &'a [Location],
    pub barriers: &'a [Location],
    pub cutoff: Option<f64>,
    pub enough: Option<usize>,
    pub at: Option<f64>,
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
            at: None,
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

    /// The graph's node count: virtual nodes are numbered from it.
    pub fn base(&self) -> u32 {
        self.base
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
                match self.split_of(piece) {
                    Some(list) => {
                        let p = &g.pieces[piece as usize];
                        let (next, share) = if forward {
                            let v = list[0];
                            (v, self.vnodes[(v - self.base) as usize].1 / p.len)
                        } else {
                            let v = list[list.len() - 1];
                            (v, (p.len - self.vnodes[(v - self.base) as usize].1) / p.len)
                        };
                        f(next, piece, forward, share);
                    }
                    None => f(g.arc_head[entry as usize], piece, forward, 1.0),
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

/// The states of a search with turns (docs/adr/0521 §5): `arcs` arc states (two a piece), two at each of `virt`
/// virtual nodes (forward, backward), then a start state at each node (graph and virtual: `base + virt`).
#[derive(Clone, Copy, Debug)]
struct States {
    arcs: u32,
    base: u32,
    virt: u32,
}

impl States {
    fn count(&self) -> usize {
        (self.arcs + 2 * self.virt + self.base + self.virt) as usize
    }

    /// Node `n`'s start state.
    fn start(&self, n: u32) -> u32 {
        self.arcs + 2 * self.virt + n
    }

    /// Virtual node `n`'s state of the way that goes `forward` (the way its piece is drawn) or against it.
    fn virtual_state(&self, n: u32, forward: bool) -> u32 {
        self.arcs + 2 * (n - self.base) + u32::from(!forward)
    }

    /// The node state `s` is at: an arc's head (a reverse search's: its tail), a virtual node, a start's node.
    fn node(&self, g: &Graph, s: u32, reverse: bool) -> u32 {
        if s < self.arcs {
            g.arc_head[(if reverse { s ^ 1 } else { s }) as usize]
        } else if s < self.arcs + 2 * self.virt {
            self.base + (s - self.arcs) / 2
        } else {
            s - self.arcs - 2 * self.virt
        }
    }

    /// The arc state `s` came by (a reverse search's: leaves by); none for a start.
    fn arc(&self, places: &Places, s: u32) -> Option<u32> {
        if s < self.arcs {
            Some(s)
        } else if s < self.arcs + 2 * self.virt {
            let k = s - self.arcs;
            Some(places.vnodes[(k / 2) as usize].0 * 2 + k % 2)
        } else {
            None
        }
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

/// A search's answer: every node's cost and the way to it (with turns: every state's).
pub struct Tree {
    pub cost: usize,
    pub reverse: bool,
    pub places: Places,
    /// The time of day the search set off (a reverse search: arrived) when its cost has a time profile.
    pub time: Option<f64>,
    states: Option<States>,
    dist: Vec<f64>,
    pred: Vec<Pred>,
    stamp: Vec<u32>,
    round: u32,
    /// Each query place's cost (origins, targets, barriers in order).
    place_costs: Vec<f64>,
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
        let states = g.turns.as_ref().map(|_| States {
            arcs: 2 * g.pieces.len() as u32,
            base: g.nodes.len() as u32,
            virt: places.vnodes.len() as u32,
        });
        let n = states.map_or(places.count(), |s| s.count());
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
            let s = states.map_or(node, |st| st.start(node));
            if get(&dist, &stamp, s) > 0.0 {
                dist[s as usize] = 0.0;
                stamp[s as usize] = round;
                pred[s as usize] = Pred {
                    node: NONE,
                    piece: NONE,
                    forward: true,
                };
                self.heap.push(Waiting { cost: 0.0, node: s });
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
        // A time is read only by a cost with a time profile.
        let time = q.at.filter(|_| g.clock(c).is_some());
        let base = g.nodes.len() as u32;
        // Once enough targets are settled, the search goes on only at the same cost: of equal costs every target is
        // settled, so the list's order decides among them.
        let mut last: Option<f64> = None;
        while let Some(Waiting { cost, node: s }) = self.heap.pop() {
            if cost > get(&dist, &stamp, s) {
                continue;
            }
            POPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if q.cutoff.is_some_and(|m| cost > m) || last.is_some_and(|l| cost > l) {
                break;
            }
            let node = states.map_or(s, |st| st.node(g, s, q.reverse));
            if !wanted.is_empty()
                && let Some(ts) = wanted.remove(&node)
            {
                settled.extend(ts);
                if last.is_none() && q.enough.is_some_and(|k| settled.len() >= k) {
                    last = Some(cost);
                }
            }
            let came = states.and_then(|st| st.arc(&places, s));
            // At a graph node the way in is one of its pieces (the way out of a reverse search's state): the turns'
            // table is read by the pieces' places, the steps coming in the node's order.
            let came_at = match came {
                Some(a) if node < base => g
                    .at_node(node)
                    .iter()
                    .position(|&e| e == if q.reverse { a } else { a ^ 1 }),
                _ => None,
            };
            let mut m = 0usize;
            places.steps(g, node, |next, piece, forward, share| {
                let step_at = m;
                m += 1;
                let real = forward != q.reverse;
                let along = g.cost(piece, c, real);
                if !along.is_finite() || places.blocked(g, next) {
                    return;
                }
                let arc = piece * 2 + u32::from(!real);
                // The turn from the way in (a reverse search: to the way out); straight on at a virtual node.
                let delay = match (states, came) {
                    (Some(_), Some(a)) if node >= base => {
                        if a != arc {
                            return;
                        }
                        0.0
                    }
                    (Some(_), Some(a)) => {
                        let t = match came_at {
                            Some(k) if q.reverse => g.turn_at(node, step_at, k, c),
                            Some(k) => g.turn_at(node, k, step_at, c),
                            None if q.reverse => g.turn(arc, a, node, c),
                            None => g.turn(a, arc, node, c),
                        };
                        match t {
                            Some(d) => d,
                            None => return,
                        }
                    }
                    _ => 0.0,
                };
                let set_off = cost + delay;
                let step = match time {
                    None => along * share,
                    Some(t) => g.travel(
                        piece,
                        c,
                        real,
                        share * g.pieces[piece as usize].len,
                        Some(if q.reverse {
                            (t - set_off, true)
                        } else {
                            (t + set_off, false)
                        }),
                    ),
                };
                let nd = set_off + step;
                let ns = match states {
                    None => next,
                    Some(st) if next >= base => st.virtual_state(next, real),
                    Some(_) => arc,
                };
                if nd < get(&dist, &stamp, ns) {
                    dist[ns as usize] = nd;
                    stamp[ns as usize] = round;
                    pred[ns as usize] = Pred {
                        node: s,
                        piece,
                        forward,
                    };
                    PUSHES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    self.heap.push(Waiting { cost: nd, node: ns });
                }
            });
        }
        settled.sort_unstable();
        let mut tree = Tree {
            cost: c,
            reverse: q.reverse,
            places,
            time,
            states,
            dist,
            pred,
            stamp,
            round,
            place_costs: Vec::new(),
            settled,
        };
        tree.place_costs = (0..tree.places.node_of.len())
            .map(|k| tree.node_cost(g, tree.places.node_of[k]).0)
            .collect();
        tree
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
    /// State (without turns: node) `k`'s cost; infinite when it was not reached.
    fn got(&self, k: u32) -> f64 {
        if (k as usize) < self.stamp.len() && self.stamp[k as usize] == self.round {
            self.dist[k as usize]
        } else {
            f64::INFINITY
        }
    }

    /// The node of state (without turns: node) `k`.
    fn node_of(&self, g: &Graph, k: u32) -> u32 {
        self.states.map_or(k, |st| st.node(g, k, self.reverse))
    }

    /// Node `n`'s cost, and the state it is reached by; infinite when it was not reached.
    fn node_cost(&self, g: &Graph, n: u32) -> (f64, u32) {
        let Some(st) = self.states else {
            return (self.got(n), n);
        };
        let mut best = (self.got(st.start(n)), st.start(n));
        let mut take = |s: u32| {
            let d = self.got(s);
            if d < best.0 {
                best = (d, s);
            }
        };
        if n >= st.base {
            take(st.virtual_state(n, true));
            take(st.virtual_state(n, false));
        } else {
            for &e in g.at_node(n) {
                // An arc arriving at the node is the reverse of one leaving it; a reverse search's leaves it.
                take(if self.reverse { e } else { e ^ 1 });
            }
        }
        best
    }

    /// Node `n`'s cost; infinite when it was not reached.
    pub fn at(&self, g: &Graph, n: u32) -> f64 {
        self.node_cost(g, n).0
    }

    /// What it costs to be at node `n` ready to go along `piece` `forward` (the way it is drawn) or against it, and the
    /// state that is from: the node's cost without turns; with turns its states' and the turn onto the piece (straight
    /// on at a virtual node).
    pub fn enter(&self, g: &Graph, n: u32, piece: u32, forward: bool) -> f64 {
        self.enter_by(g, n, piece, forward).0
    }

    fn enter_by(&self, g: &Graph, n: u32, piece: u32, forward: bool) -> (f64, u32) {
        let Some(st) = self.states else {
            return (self.got(n), n);
        };
        let real = forward != self.reverse;
        let arc = piece * 2 + u32::from(!real);
        let mut best = (self.got(st.start(n)), st.start(n));
        if n >= st.base {
            let s = st.virtual_state(n, real);
            if self.got(s) < best.0 {
                best = (self.got(s), s);
            }
            return best;
        }
        for &e in g.at_node(n) {
            let s = if self.reverse { e } else { e ^ 1 };
            let d = self.got(s);
            if !d.is_finite() {
                continue;
            }
            let turn = if self.reverse {
                g.turn(arc, s, n, self.cost)
            } else {
                g.turn(s, arc, n, self.cost)
            };
            if let Some(t) = turn
                && d + t < best.0
            {
                best = (d + t, s);
            }
        }
        best
    }

    /// The cost of query place `k` (origins, targets, barriers in order).
    pub fn place_cost(&self, k: usize) -> f64 {
        self.place_costs[k]
    }

    /// What going `d` metres along `piece` `forward` (or against it) costs having come at cost `so_far`: its share of
    /// the piece's cost, or with a time its time set off at (arrived at) the search's time plus (minus) `so_far`.
    pub fn go(&self, g: &Graph, piece: u32, forward: bool, d: f64, so_far: f64) -> f64 {
        let real = forward != self.reverse;
        if !so_far.is_finite() {
            return f64::INFINITY;
        }
        match self.time {
            None => {
                g.cost(piece, self.cost, real) * (d / g.pieces[piece as usize].len)
            }
            Some(t) => g.travel(
                piece,
                self.cost,
                real,
                d,
                Some(if self.reverse {
                    (t - so_far, true)
                } else {
                    (t + so_far, false)
                }),
            ),
        }
    }

    /// How far along `piece` `forward` (or against it) a way gets in `dt` having come at cost `so_far`, with the search's
    /// time (none: the cost's own rate).
    pub fn reach(&self, g: &Graph, piece: u32, forward: bool, dt: f64, so_far: f64) -> f64 {
        let real = forward != self.reverse;
        let whole = g.cost(piece, self.cost, real);
        if !whole.is_finite() {
            return 0.0;
        }
        let p = &g.pieces[piece as usize];
        match (self.time, g.clock(self.cost)) {
            (Some(t), Some(k)) if self.reverse => k.reach_backward(p.edge, t - so_far, dt),
            (Some(t), Some(k)) => k.reach_forward(p.edge, t + so_far, dt),
            _ if whole > 0.0 => dt / (whole / p.len),
            _ => f64::INFINITY,
        }
    }

    /// What reaching `loc` costs (the search's way), through which state and from which offset, and where on its piece
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
        // At a node itself the way only reaches it; inside the piece it turns onto it.
        let from = |n: u32, o: f64, forward: bool| {
            if o == x {
                self.node_cost(g, n)
            } else {
                let (c, s) = self.enter_by(g, n, loc.piece, forward);
                if !c.is_finite() {
                    return (f64::INFINITY, s);
                }
                (c + self.go(g, loc.piece, forward, (x - o).abs(), c), s)
            }
        };
        let (via_lo, s_lo) = from(lo, olo, true);
        let (via_hi, s_hi) = from(hi, ohi, false);
        if via_lo <= via_hi && via_lo.is_finite() {
            Some((via_lo, s_lo, olo, x))
        } else if via_hi.is_finite() {
            Some((via_hi, s_hi, ohi, x))
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
        let (cost, state, o, x) = self.best(g, loc)?;
        let mut spans = Vec::new();
        let mut k = state;
        loop {
            let pr = self.pred[k as usize];
            if self.stamp[k as usize] != self.round || pr.node == NONE {
                break;
            }
            let start = self
                .places
                .offset(g, self.node_of(g, pr.node), pr.piece, pr.forward);
            let end = self
                .places
                .offset(g, self.node_of(g, k), pr.piece, !pr.forward);
            spans.push(Span {
                piece: pr.piece,
                a: start,
                b: end,
            });
            k = pr.node;
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

/// The node two spans in a row meet at and the arcs they go by, when the first ends at a node (not inside a piece).
fn joint(g: &Graph, a: &Span, b: &Span) -> Option<(u32, u32, u32)> {
    let p = &g.pieces[a.piece as usize];
    let node = if a.b == p.len && a.b != a.a {
        p.to
    } else if a.b == 0.0 && a.b != a.a {
        p.from
    } else {
        return None;
    };
    let q = &g.pieces[b.piece as usize];
    if a.piece == b.piece && b.a != 0.0 && b.a != q.len {
        return None;
    }
    let arc = |s: &Span| s.piece * 2 + u32::from(s.b < s.a);
    Some((node, arc(a), arc(b)))
}

/// What a way through `spans` costs with cost `c`, its turns (docs/adr/0521) and, with a time profile, set off at
/// `at` (`arrive`: arriving at `at`): infinite when a span cannot be travelled with it or a turn is forbidden.
pub fn way_cost(g: &Graph, spans: &[Span], c: usize, at: Option<f64>, arrive: bool) -> f64 {
    let at = at.filter(|_| g.clock(c).is_some());
    let len = |s: &Span| (s.b - s.a).abs();
    let turn = |k: usize| match joint(g, &spans[k], &spans[k + 1]) {
        Some((n, a, b)) => g.turn(a, b, n, c).unwrap_or(f64::INFINITY),
        None => 0.0,
    };
    let mut total = 0.0;
    if arrive && at.is_some() {
        for k in (0..spans.len()).rev() {
            if k + 1 < spans.len() {
                total += turn(k);
            }
            let s = &spans[k];
            total += g.travel(s.piece, c, s.b >= s.a, len(s), at.map(|t| (t - total, true)));
        }
        return total;
    }
    for (k, s) in spans.iter().enumerate() {
        if k > 0 {
            total += turn(k - 1);
        }
        total += match at {
            Some(t) => g.travel(s.piece, c, s.b >= s.a, len(s), Some((t + total, false))),
            None => g.cost(s.piece, c, s.b >= s.a) * (len(s) / g.pieces[s.piece as usize].len),
        };
    }
    total
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
                levels: [None, None, None],
                periods: Vec::new(),
                terminals: [None, None],
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
