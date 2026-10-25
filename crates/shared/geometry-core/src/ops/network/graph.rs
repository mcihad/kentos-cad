//! A network's graph (docs/adr/0209 §3): the edges cut where they connect,
//! the nodes the cuts make, and what travelling each piece costs each way.
//!
//! 1. Every edge's two ends (`vertices`: every vertex) are cuts; an end that
//!    lies within the tolerance of another edge's inside cuts it at its
//!    nearest point (a T junction); a junction point cuts every edge within
//!    the tolerance at its nearest point, and one near none is off the network.
//! 2. Cuts and junction points closer than the tolerance are one node, the
//!    chain along: the node is where its first member is (edges in their
//!    order, each from its start; then the junction points).
//! 3. Each edge is pieces between its cuts, its own vertices and arcs (an end
//!    is not moved to its node); a piece no longer than the tolerance whose
//!    ends are one node is dropped and counted.
//! 4. Edges that cross without an end or (`vertices`) a vertex there do not
//!    connect: bridges, underpasses, pipes at other depths.
//! 5. With level fields (docs/adr/0521 §3) a place connects only to places at
//!    its level: an edge's start and end at theirs, its inside (its vertices,
//!    another's end on it) at the inside field's, else its ends' when they are
//!    one and nothing when they differ; a junction point at its own, or at every
//!    level when it gives none (a connector). An end touches only an inside at
//!    its level.
//!
//! The independent reference is `scripts/fixtures/network_cases.py`.

use super::access::{Access, Closing, Varied};
use super::clock::Clock;
use super::rules::{Dir, Rules};
use super::terminals::{NO_TERMINAL, Terminals};
use super::turns::{Level, TurnIn, TurnTable, heading};
use crate::geom::arrangement::{edge_len, reverse_edge, sub_edge};
use crate::geom::intersect::{Edge, closest_on_edge, point_at};
use crate::geometry::Bounds;
use crate::jsmath::{js_hypot, js_max};
use crate::store::rtree::PackedTree;
use crate::vec2::Vec2;

/// An edge as the host gives it: its object's id and part, its path, its
/// direction field's value, its costs' fields' values (one each, in the
/// definition's order) and whether the closed edges' expression holds for it.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeIn {
    pub id: f64,
    pub part: u32,
    pub path: Vec<Edge>,
    pub direction: Option<String>,
    pub costs: Vec<Option<String>>,
    pub closed: bool,
    /// Its level fields' values: at its start, at its end, inside it (docs/adr/0521 §3).
    pub levels: [Option<String>; 3],
    /// Its time profiles' field values: `periods[c - 1][k]` for cost `c`'s period `k` (docs/adr/0521 §4).
    pub periods: Vec<Vec<Option<String>>>,
    /// Its terminal fields' values: the terminal its start and its end are attached to (docs/adr/0557 §1).
    pub terminals: [Option<String>; 2],
    /// Its profiles' and closures' field values, `facts[i]` of the rules' `access.facts[i]` (docs/adr/0567).
    pub facts: Vec<Option<String>>,
}

impl EdgeIn {
    /// An edge with only its id, part, path and values (no levels, no period values).
    pub fn new(
        id: f64,
        part: u32,
        path: Vec<Edge>,
        direction: Option<String>,
        costs: Vec<Option<String>>,
        closed: bool,
    ) -> EdgeIn {
        EdgeIn {
            id,
            part,
            path,
            direction,
            costs,
            closed,
            levels: [None, None, None],
            periods: Vec::new(),
            terminals: [None, None],
            facts: Vec::new(),
        }
    }
}

/// What a junction is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Junction,
    Source,
    Valve,
}

/// A junction point as the host gives it: its object's id, the point, its
/// role and whether its closed expression holds (a closed valve).
#[derive(Clone, Debug, PartialEq)]
pub struct JunctionIn {
    pub id: f64,
    pub p: Vec2,
    pub role: Role,
    pub closed: bool,
    /// Its level field's value; none: every level (docs/adr/0521 §3).
    pub level: Option<String>,
    /// Its device: an index into the rules' `terminals.devices`; none: a point (docs/adr/0557 §1).
    pub device: Option<u32>,
}

/// A piece: between two nodes along one edge, from `s0` to `s1` (metres
/// from the edge's start), its primitives `geom[g0..g1]` in the edge's way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Piece {
    pub from: u32,
    pub to: u32,
    pub edge: u32,
    pub s0: f64,
    pub s1: f64,
    pub len: f64,
    pub g0: u32,
    pub g1: u32,
}

/// An edge as the graph keeps it: its ends and their nodes (none for an edge without a path).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeMeta {
    pub id: f64,
    pub part: u32,
    pub length: f64,
    pub dir: Dir,
    pub closed: bool,
    pub a: Vec2,
    pub b: Vec2,
    pub nodes: Option<(u32, u32)>,
    /// Its inside's level (docs/adr/0521 §3).
    pub inside: Level,
}

/// A junction as the graph keeps it: its node, none when it is off the network.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JunctionMeta {
    pub id: f64,
    pub p: Vec2,
    pub role: Role,
    pub closed: bool,
    pub node: Option<u32>,
}

/// What a node is besides a place: a source, a valve, closed (passed by no analysis).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NodeKind {
    pub source: bool,
    pub valve: bool,
    pub closed: bool,
}

/// What building said besides the graph (Denetle's and the tools' notes).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuildReport {
    /// Pieces dropped as no longer than the tolerance with one node at both ends: the edge's id and where.
    pub short: Vec<(f64, Vec2)>,
    /// Junctions near no edge: their ids and places.
    pub off_network: Vec<(f64, Vec2)>,
    /// Edges whose cost field was not read: (cost, the edge's id); a speed's default was taken, a field cost's edge is not travelled with it.
    pub unread: Vec<(usize, f64)>,
    /// Edges and junctions whose level field was not read (ground, every level), and turn lines whose delay was not
    /// read (forbidden): their ids and places.
    pub unread_other: Vec<(f64, Vec2)>,
    /// Turn lines not on two pieces meeting at a node: their ids and their first points.
    pub unplaced: Vec<(f64, Vec2)>,
    /// Edge ends at a device that name none of its terminals, and edges running through a device instead of ending at
    /// it (docs/adr/0557 §1): the edge's id and the device's place.
    pub unterminated: Vec<(f64, Vec2)>,
}

/// A place on the network: a piece and the metres along it from its start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Location {
    pub piece: u32,
    pub offset: f64,
    pub p: Vec2,
    /// How far the point it was found for is (metres).
    pub d: f64,
}

/// The costs both ways and the clocks as defined, kept while a question varies them (docs/adr/0567 §4).
type Defined = (Vec<f64>, Vec<f64>, Vec<Option<Clock>>);

/// The graph.
pub struct Graph {
    pub rules: Rules,
    pub nodes: Vec<Vec2>,
    pub kinds: Vec<NodeKind>,
    pub pieces: Vec<Piece>,
    pub geom: Vec<Edge>,
    /// Where each primitive starts along its piece (metres).
    pub geom_at: Vec<f64>,
    pub(super) geom_piece: Vec<u32>,
    pub edges: Vec<EdgeMeta>,
    pub junctions: Vec<JunctionMeta>,
    /// Costs: the length and the definition's, `1 + rules.costs.len()`.
    pub costs: usize,
    /// Each piece's cost the way it is drawn and against it, `[piece * costs + c]`; infinite: not travelled.
    pub fw: Vec<f64>,
    pub bw: Vec<f64>,
    /// Each node's pieces: `piece * 2` leaving from its start the way it is drawn, `piece * 2 + 1` from its end against it.
    pub(super) adj_start: Vec<u32>,
    pub(super) adj: Vec<u32>,
    /// Each arc's head: `piece * 2` the piece's end, `piece * 2 + 1` its start (a step's next node at a glance).
    pub arc_head: Vec<u32>,
    pub(super) tree: PackedTree,
    /// The turns, when the definition has them: searches go by arcs (docs/adr/0521 §5).
    pub turns: Option<TurnTable>,
    /// Each cost's time profile (none for most), `clocks[c]`.
    pub clocks: Vec<Option<Clock>>,
    /// The devices' terminals at the nodes and arcs, when the definition has devices (docs/adr/0557 §1).
    pub terms: Option<Terminals>,
    /// Each edge's profiles' and closures' values, when the definition has some (docs/adr/0567).
    pub(super) facts: Vec<Vec<Option<String>>>,
    /// The costs and clocks as defined, kept while the graph is varied for a question's access.
    pub(super) base: Option<Defined>,
    /// The access the costs are for, and what varying them for it closed.
    pub(super) access: Access,
    pub(super) varied: Varied,
    /// Why each edge is closed to the access (docs/adr/0570); empty for none.
    pub(super) closing: Vec<Closing>,
    /// The level gaps (docs/adr/0640 §3): each node's nodes within the tolerance of it that its level keeps apart (a
    /// bridge over a road where both have a place), `gap_to[gap_start[n]..gap_start[n + 1]]`; empty without levels.
    pub(super) gap_start: Vec<u32>,
    pub(super) gap_to: Vec<u32>,
    /// Each node's part of the network (every piece both ways and the levels' gaps stepped over) and each part's
    /// edges: a service area's edges no facility meets are counted from them (docs/adr/0640 §2).
    pub(super) part: Vec<u32>,
    pub(super) part_edges: Vec<u32>,
    /// What a rebuild reuses, when built to be rebuilt (`build_kept`, docs/adr/0641 §1).
    pub(super) kept: Option<Box<super::rebuild::Kept>>,
}

fn bounds_around(p: Vec2, r: f64) -> Bounds {
    Bounds {
        min_x: p.x - r,
        min_y: p.y - r,
        max_x: p.x + r,
        max_y: p.y + r,
    }
}

/// How far a turn line's first and last points may be from the pieces they name (metres; the tolerance when larger).
pub const TURN_REACH: f64 = 1.0;

impl Graph {
    /// Builds the graph of `edges` and `junctions` by `rules` (§3).
    pub fn build(rules: Rules, edges: &[EdgeIn], junctions: &[JunctionIn]) -> (Graph, BuildReport) {
        Self::build_with(rules, edges, junctions, &[])
    }

    /// Builds the graph of `edges`, `junctions` and the turn layers' lines `turns` by `rules` (§3, docs/adr/0521).
    pub fn build_with(
        rules: Rules,
        edges: &[EdgeIn],
        junctions: &[JunctionIn],
        turns: &[TurnIn],
    ) -> (Graph, BuildReport) {
        Self::build_all(rules, edges, junctions, turns, false)
    }

    /// The pieces' costs both ways as the definition gives them, not as a question's access varied them.
    pub(super) fn defined_costs(&self) -> (&[f64], &[f64]) {
        match &self.base {
            Some((fw, bw, _)) => (fw, bw),
            None => (&self.fw, &self.bw),
        }
    }

    /// The bytes a graph built to be rebuilt keeps for it, about (none otherwise).
    pub fn kept_bytes(&self) -> usize {
        self.kept.as_ref().map_or(0, |k| k.bytes())
    }

    /// Builds as `build_with` and keeps what `rebuild` reuses when the edges change (docs/adr/0641 §1): the interactive
    /// tools' network, built again after every edit.
    pub fn build_kept(
        rules: Rules,
        edges: &[EdgeIn],
        junctions: &[JunctionIn],
        turns: &[TurnIn],
    ) -> (Graph, BuildReport) {
        Self::build_all(rules, edges, junctions, turns, true)
    }

    /// The turn table (docs/adr/0521 §2): the pieces' headings, the nodes' pieces, which costs are times, and the turn
    /// lines found on two pieces meeting at a node; the others said.
    pub(super) fn turn_table(&self, turns: &[TurnIn], report: &mut BuildReport) -> TurnTable {
        let heads = self
            .pieces
            .iter()
            .map(|p| {
                let (g0, g1) = (p.g0 as usize, p.g1 as usize);
                if g1 <= g0 {
                    return [Vec2::new(0.0, 0.0); 2];
                }
                [
                    heading(&self.geom[g0], false),
                    heading(&self.geom[g1 - 1], true),
                ]
            })
            .collect();
        let degree = (0..self.nodes.len())
            .map(|n| self.adj_start[n + 1] - self.adj_start[n])
            .collect();
        let times = (0..self.costs).map(|c| self.rules.is_time(c)).collect();
        let reach = js_max(TURN_REACH, self.rules.tolerance);
        let mut named: Vec<(u64, f64)> = Vec::new();
        for t in turns {
            let (Some(&first), Some(&last)) = (t.pts.first(), t.pts.last()) else {
                continue;
            };
            let via = if t.pts.len() >= 3 {
                t.pts[1]
            } else {
                Vec2::new((first.x + last.x) / 2.0, (first.y + last.y) / 2.0)
            };
            let Some(key) = self.turn_key(first, last, via, reach) else {
                report.unplaced.push((t.id, first));
                continue;
            };
            let delay = if !t.field {
                f64::INFINITY
            } else {
                match t.value.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
                    None => f64::INFINITY,
                    Some(v) => match super::rules::number(v).filter(|d| *d >= 0.0) {
                        Some(d) => d / 60.0,
                        None => {
                            report.unread_other.push((t.id, first));
                            f64::INFINITY
                        }
                    },
                }
            };
            named.push((key, delay));
        }
        // A turn named twice: forbidden if either forbids it, else the longer delay.
        named.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.total_cmp(&a.1)));
        named.dedup_by_key(|t| t.0);
        let mut table = TurnTable {
            rule: self.rules.turns,
            named,
            heads,
            degree,
            times,
            ..TurnTable::default()
        };
        table.code_all(&self.adj_start, &self.adj);
        table
    }

    /// The turn a line from `first` to `last` through `via` names (`in_arc << 32 | out_arc`): from the piece at its
    /// first point to the piece at its last through the node they share nearest `via` (of equal ones the lower);
    /// back along the one piece when both points are on it. None when a point is on no piece or they share no node.
    fn turn_key(&self, first: Vec2, last: Vec2, via: Vec2, reach: f64) -> Option<u64> {
        let (a, b) = (self.locate(first, reach)?, self.locate(last, reach)?);
        let (pa, pb) = (&self.pieces[a.piece as usize], &self.pieces[b.piece as usize]);
        let d = |n: u32| {
            let q = self.nodes[n as usize];
            js_hypot(q.x - via.x, q.y - via.y)
        };
        let mut node: Option<u32> = None;
        for n in [pa.from, pa.to] {
            if n != pb.from && n != pb.to {
                continue;
            }
            if node.is_none_or(|m| d(n) < d(m) || (d(n) == d(m) && n < m)) {
                node = Some(n);
            }
        }
        let n = node?;
        let in_arc = if pa.to == n {
            a.piece * 2
        } else {
            a.piece * 2 + 1
        };
        let out_arc = if a.piece == b.piece {
            in_arc ^ 1
        } else if pb.from == n {
            b.piece * 2
        } else {
            b.piece * 2 + 1
        };
        Some((u64::from(in_arc) << 32) | u64::from(out_arc))
    }

    /// Whether a way that arrived at `node` by terminal `from` (`NO_TERMINAL`: by none) may leave along arc `arc`
    /// (docs/adr/0557 §1): always in a network without devices.
    pub fn passes(&self, node: u32, from: u8, arc: u32) -> bool {
        match &self.terms {
            None => true,
            Some(t) => t.passes(&self.rules, node, from, t.arc_terminal[arc as usize]),
        }
    }

    /// The terminal by which a way along arc `arc` arrives at its head.
    pub fn arrives(&self, arc: u32) -> u8 {
        match &self.terms {
            None => NO_TERMINAL,
            Some(t) => t.arc_terminal[(arc ^ 1) as usize],
        }
    }

    /// What turning at `node` from arc `in_arc` to arc `out_arc` adds to cost `c`; none when it is forbidden. Every
    /// turn is free in a network without turns.
    pub fn turn(&self, in_arc: u32, out_arc: u32, node: u32, c: usize) -> Option<f64> {
        if !self.crosses(node, in_arc, out_arc) {
            return None;
        }
        match &self.turns {
            Some(t) => t.turn(in_arc, out_arc, node, c),
            None => Some(0.0),
        }
    }

    /// Whether a way arriving at `node` along `in_arc` may leave along `out_arc` through the node's device
    /// (docs/adr/0562 §1): only along a terminal path; always at a point and in a network without devices.
    pub(super) fn crosses(&self, node: u32, in_arc: u32, out_arc: u32) -> bool {
        match &self.terms {
            None => true,
            // A virtual node (a stop inside an edge, a barrier) is no device.
            Some(t) if node as usize >= t.node_device.len() => true,
            Some(t) => t.passes(
                &self.rules,
                node,
                t.arc_terminal[(in_arc ^ 1) as usize],
                t.arc_terminal[out_arc as usize],
            ),
        }
    }

    /// The turn at graph node `node` from the arc arriving along its `k`th piece to the arc leaving along its `m`th
    /// (`at_node`'s order); every turn is free in a network without turns.
    pub fn turn_at(&self, node: u32, k: usize, m: usize, c: usize) -> Option<f64> {
        if self.terms.is_some() {
            let at = self.at_node(node);
            if !self.crosses(node, at[k] ^ 1, at[m]) {
                return None;
            }
        }
        match &self.turns {
            Some(t) => t.turn_at(node, self.at_node(node), k, m, c),
            None => Some(0.0),
        }
    }

    /// Cost `c`'s time profile, when it has one.
    pub fn clock(&self, c: usize) -> Option<&Clock> {
        self.clocks.get(c).and_then(Option::as_ref)
    }

    /// What going `d` metres along piece `p` costs with cost `c`, the way it is drawn (`forward`) or against it:
    /// with `at`, the time of day it is set off at (`back`: arrived at) when the cost has a time profile; else its
    /// share of the piece's cost. Infinite when it cannot be travelled.
    pub fn travel(&self, p: u32, c: usize, forward: bool, d: f64, at: Option<(f64, bool)>) -> f64 {
        let whole = self.cost(p, c, forward);
        if !whole.is_finite() {
            return f64::INFINITY;
        }
        match (at, self.clock(c)) {
            (Some((t, back)), Some(clock)) => {
                let edge = self.pieces[p as usize].edge;
                if back {
                    clock.backward(edge, t, d)
                } else {
                    clock.forward(edge, t, d)
                }
            }
            _ => whole * (d / self.pieces[p as usize].len),
        }
    }


    /// The pieces at node `n`: `piece * 2` leaving its start the way it is drawn, `piece * 2 + 1` leaving its end against it.
    pub fn at_node(&self, n: u32) -> &[u32] {
        &self.adj[self.adj_start[n as usize] as usize..self.adj_start[n as usize + 1] as usize]
    }

    /// Numbers the network's parts: the nodes the pieces and the levels' gaps join, and each part's edges (an edge in
    /// the part of its first piece).
    pub(super) fn find_parts(&mut self) {
        fn find(parent: &mut [u32], mut x: u32) -> u32 {
            while parent[x as usize] != x {
                parent[x as usize] = parent[parent[x as usize] as usize];
                x = parent[x as usize];
            }
            x
        }
        let mut parent: Vec<u32> = (0..self.nodes.len() as u32).collect();
        let mut join = |a: u32, b: u32| {
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            if ra != rb {
                parent[ra.max(rb) as usize] = ra.min(rb);
            }
        };
        for p in &self.pieces {
            join(p.from, p.to);
        }
        for (n, w) in self.gap_start.windows(2).enumerate() {
            for &m in &self.gap_to[w[0] as usize..w[1] as usize] {
                join(n as u32, m);
            }
        }
        let mut number = vec![u32::MAX; self.nodes.len()];
        let mut count = 0u32;
        self.part = (0..self.nodes.len() as u32)
            .map(|n| {
                let r = find(&mut parent, n) as usize;
                if number[r] == u32::MAX {
                    number[r] = count;
                    count += 1;
                }
                number[r]
            })
            .collect();
        self.part_edges = vec![0; count as usize];
        let mut seen = vec![false; self.edges.len()];
        for p in &self.pieces {
            if !std::mem::replace(&mut seen[p.edge as usize], true) {
                self.part_edges[self.part[p.from as usize] as usize] += 1;
            }
        }
    }

    /// The edges on parts of the network that none of `nodes` is on.
    pub fn edges_apart_from(&self, nodes: &[u32]) -> usize {
        let mut parts: Vec<u32> = nodes.iter().filter_map(|&n| self.part.get(n as usize).copied()).collect();
        parts.sort_unstable();
        parts.dedup();
        let total: u32 = self.part_edges.iter().sum();
        let near: u32 = parts.iter().map(|&p| self.part_edges[p as usize]).sum();
        (total - near) as usize
    }

    /// Whether any node's level keeps a node beside it apart.
    pub fn has_level_gaps(&self) -> bool {
        !self.gap_to.is_empty()
    }

    /// The nodes node `n`'s level keeps apart from it within the tolerance (docs/adr/0640 §3).
    pub fn level_gaps(&self, n: u32) -> &[u32] {
        if self.gap_start.is_empty() {
            return &[];
        }
        &self.gap_to[self.gap_start[n as usize] as usize..self.gap_start[n as usize + 1] as usize]
    }

    /// The access the costs are for (docs/adr/0567).
    pub fn access(&self) -> Access {
        self.access
    }

    /// Varies the costs for `access` (docs/adr/0567): its profile's closed edges and the day's closed edges not
    /// travelled, its top speed capping every speed cost, a both-ways profile against one-way edges; what it closed.
    /// The pieces, the nodes and the turns stay; the same access again is free, none the network as defined.
    pub fn vary(&mut self, access: Access) -> &Varied {
        if access == self.access {
            return &self.varied;
        }
        let base = self
            .base
            .take()
            .unwrap_or_else(|| (self.fw.clone(), self.bw.clone(), self.clocks.clone()));
        self.vary_from(access, &base);
        self.base = Some(base);
        &self.varied
    }

    fn vary_from(&mut self, access: Access, (fw0, bw0, clocks0): &Defined) {
        self.access = access;
        self.varied = Varied::default();
        self.closing.clear();
        if access.is_none() {
            self.fw.clone_from(fw0);
            self.bw.clone_from(bw0);
            self.clocks.clone_from(clocks0);
            return;
        }
        let rules = &self.rules.access;
        let profile = access.profile.and_then(|i| rules.profiles.get(i));
        let (top, both) = profile.map_or((None, false), |p| (p.top_speed, p.both_ways));
        let mut closing = vec![Closing::Open; self.edges.len()];
        // The hours each edge is closed in (docs/adr/0568, 0640), minutes from the access's day's midnight in its frame.
        let mut windows: Vec<Vec<(f64, f64)>> = Vec::new();
        let mut frames: Vec<Option<(f64, f64)>> = Vec::new();
        let mut shut = Vec::new();
        let mut frame = (0.0, 0.0);
        for (i, c) in closing.iter_mut().enumerate() {
            let mut unread = false;
            let facts = self.facts.get(i).map_or(&[][..], Vec::as_slice);
            shut.clear();
            *c = rules.closing(&access, facts, &mut unread, &mut shut, &mut frame);
            match *c {
                Closing::Profile => self.varied.by_profile += 1,
                Closing::Date => self.varied.by_date += 1,
                Closing::Hours => {
                    self.varied.by_hours += 1;
                    if windows.is_empty() {
                        windows = vec![Vec::new(); self.edges.len()];
                        frames = vec![None; self.edges.len()];
                    }
                    windows[i].clone_from(&shut);
                    frames[i] = Some(frame);
                }
                Closing::Open => {}
            }
            let id = self.edges[i].id;
            if unread && self.varied.unread.last() != Some(&id) {
                self.varied.unread.push(id);
            }
        }
        let n = self.costs;
        for (k, p) in self.pieces.iter().enumerate() {
            let e = p.edge as usize;
            let meta = &self.edges[e];
            // With a time a cost without a clock passes none closed at the question's moment (docs/adr/0568 §2).
            let shut_now = closing[e] == Closing::Hours
                && access
                    .time
                    .is_some_and(|t| windows[e].iter().any(|w| w.0 <= t && t < w.1));
            for c in 0..n {
                let at = k * n + c;
                let (mut f, mut b) = (fw0[at], bw0[at]);
                let shut = match closing[e] {
                    Closing::Open => false,
                    Closing::Hours => shut_now && !self.rules.is_time(c),
                    _ => true,
                };
                if shut {
                    f = f64::INFINITY;
                    b = f64::INFINITY;
                } else {
                    if both && !meta.closed && meta.dir != Dir::Closed {
                        let any = if f.is_finite() { f } else { b };
                        (f, b) = (any, any);
                    }
                    if let Some(kmh) = top
                        && self.rules.is_time(c)
                    {
                        // Minutes at the top speed: 60 over the metres an hour.
                        let least = p.len * 60.0 / (kmh * 1000.0);
                        f = if f < least { least } else { f };
                        b = if b < least { least } else { b };
                    }
                }
                self.fw[at] = f;
                self.bw[at] = b;
            }
        }
        for (c, clock) in self.clocks.iter_mut().enumerate() {
            *clock = match (&clocks0[c], top) {
                (Some(k), Some(kmh)) => Some(k.capped(kmh)),
                (k, _) => k.clone(),
            };
        }
        self.varied.hours_unasked = self.varied.by_hours > 0 && access.time.is_none();
        // An edge closed in hours is closed to a question only when its cost passes none at the question's moment.
        for (e, c) in closing.iter_mut().enumerate() {
            let shut_now = access
                .time
                .is_some_and(|t| windows.get(e).is_some_and(|w| w.iter().any(|w| w.0 <= t && t < w.1)));
            if *c == Closing::Hours && !shut_now {
                *c = Closing::Open;
            }
        }
        self.closing = closing;
        if self.varied.by_hours == 0 || access.time.is_none() {
            return;
        }
        // A speed cost's clock through each edge's frame, at no speed in its hours (docs/adr/0568 §3, 0640 §1).
        for c in 1..n {
            if !self.rules.is_time(c) {
                continue;
            }
            let speeds = self.edge_speeds(c);
            let days: Vec<Vec<(f64, f64)>> = (0..self.edges.len())
                .map(|e| match &self.clocks[c] {
                    Some(k) => k.day_steps(e as u32),
                    None => vec![(0.0, speeds[e])],
                })
                .collect();
            self.clocks[c] = Some(Clock::framed(&days, &windows, &frames));
        }
    }

    /// Each edge's speed with time cost `c` as varied (metres a minute): a piece's length over its minutes, either
    /// way; none (0) for an edge closed both ways.
    fn edge_speeds(&self, c: usize) -> Vec<f64> {
        let n = self.costs;
        let mut speeds = vec![0.0; self.edges.len()];
        for (k, p) in self.pieces.iter().enumerate() {
            let e = p.edge as usize;
            if speeds[e] > 0.0 {
                continue;
            }
            if let Some(m) = [self.fw[k * n + c], self.bw[k * n + c]]
                .into_iter()
                .find(|m| m.is_finite() && *m > 0.0)
            {
                speeds[e] = p.len / m;
            }
        }
        speeds
    }

    /// Why edge `e` is closed to the access the costs are for (docs/adr/0570).
    pub fn closing_of(&self, e: usize) -> Closing {
        self.closing.get(e).copied().unwrap_or(Closing::Open)
    }

    /// Piece `p`'s cost `c` travelled the way it is drawn (`forward`) or against it.
    pub fn cost(&self, p: u32, c: usize, forward: bool) -> f64 {
        let k = p as usize * self.costs + c;
        if forward { self.fw[k] } else { self.bw[k] }
    }

    /// The network's total length (metres): its pieces'.
    pub fn length(&self) -> f64 {
        self.pieces.iter().map(|p| p.len).sum()
    }

    /// The place of the network nearest to `p` within `reach` metres; on a tie the lower piece, then the nearer its start.
    pub fn locate(&self, p: Vec2, reach: f64) -> Option<Location> {
        let mut found = Vec::new();
        self.tree.search(&bounds_around(p, reach), &mut found);
        let mut best: Option<Location> = None;
        for g in found {
            let e = &self.geom[g as usize];
            let c = closest_on_edge(e, p);
            if c.d > reach {
                continue;
            }
            let piece = self.geom_piece[g as usize];
            let offset = self.geom_at[g as usize] + c.t * edge_len(e);
            let better = match &best {
                None => true,
                Some(b) => {
                    c.d < b.d
                        || (c.d == b.d
                            && (piece < b.piece || (piece == b.piece && offset < b.offset)))
                }
            };
            if better {
                best = Some(Location {
                    piece,
                    offset,
                    p: c.p,
                    d: c.d,
                });
            }
        }
        best
    }

    /// The primitives (indices into `geom`) whose boxes come within `r` of `p`.
    pub fn near_geom(&self, p: Vec2, r: f64, out: &mut Vec<u32>) {
        out.clear();
        self.tree.search(&bounds_around(p, r), out);
        out.sort_unstable();
    }

    /// The primitives whose boxes meet `b`.
    pub fn near_box(&self, b: &Bounds, out: &mut Vec<u32>) {
        self.tree.search(b, out);
        out.sort_unstable();
    }

    /// The piece primitive `g` is on.
    pub fn piece_of_geom(&self, g: u32) -> u32 {
        self.geom_piece[g as usize]
    }

    /// The point `offset` metres along piece `p`.
    pub fn point_on(&self, p: u32, offset: f64) -> Vec2 {
        let piece = &self.pieces[p as usize];
        let (g0, g1) = (piece.g0 as usize, piece.g1 as usize);
        for g in g0..g1 {
            let len = edge_len(&self.geom[g]);
            let at = self.geom_at[g];
            if offset <= at + len || g + 1 == g1 {
                let t = if len > 0.0 {
                    ((offset - at) / len).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                return point_at(&self.geom[g], t);
            }
        }
        self.nodes[piece.from as usize]
    }

    /// Piece `p`'s primitives from `a` to `b` metres along it, in that way (backward when `b < a`).
    pub fn span_edges(&self, p: u32, a: f64, b: f64, out: &mut Vec<Edge>) {
        let piece = &self.pieces[p as usize];
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let start = out.len();
        for g in piece.g0..piece.g1 {
            let e = &self.geom[g as usize];
            let len = edge_len(e);
            let (k0, k1) = (self.geom_at[g as usize], self.geom_at[g as usize] + len);
            if len <= 0.0 || k1 <= lo || k0 >= hi {
                continue;
            }
            let t0 = if lo > k0 { (lo - k0) / len } else { 0.0 };
            let t1 = if hi < k1 { (hi - k0) / len } else { 1.0 };
            out.push(if t0 == 0.0 && t1 == 1.0 {
                *e
            } else {
                sub_edge(e, t0, t1)
            });
        }
        if b < a {
            out[start..].reverse();
            for e in &mut out[start..] {
                *e = reverse_edge(e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::Json;

    fn rules(connect: &str, tolerance: f64) -> Rules {
        Rules::from_json(
            &Json::parse(&format!(
                r#"{{"connect":"{connect}","tolerance":{tolerance},"direction":{{"kind":"both"}}}}"#
            ))
            .expect("JSON"),
        )
        .expect("rules")
    }

    fn line(id: f64, pts: &[(f64, f64)]) -> EdgeIn {
        EdgeIn {
            id,
            part: 0,
            path: pts
                .windows(2)
                .map(|w| Edge::Seg {
                    a: Vec2::new(w[0].0, w[0].1),
                    b: Vec2::new(w[1].0, w[1].1),
                })
                .collect(),
            direction: None,
            costs: Vec::new(),
            closed: false,
            levels: [None, None, None],
            periods: Vec::new(),
            terminals: [None, None],
            facts: Vec::new(),
        }
    }

    #[test]
    fn a_t_junction_cuts_the_through_street_and_a_crossing_does_not_connect() {
        // A street from (0,0) to (100,0); a side street ending on it at (40,0.004); one crossing it at (70,-10)–(70,10).
        let edges = [
            line(1.0, &[(0.0, 0.0), (100.0, 0.0)]),
            line(2.0, &[(40.0, 30.0), (40.0, 0.004)]),
            line(3.0, &[(70.0, -10.0), (70.0, 10.0)]),
        ];
        let (g, report) = Graph::build(rules("ends", 0.01), &edges, &[]);
        assert!(report.short.is_empty() && report.off_network.is_empty());
        // The through street is two pieces at the side street's end; the crossing is not cut.
        assert_eq!(g.pieces.iter().filter(|p| p.edge == 0).count(), 2);
        assert_eq!(g.pieces.iter().filter(|p| p.edge == 2).count(), 1);
        assert_eq!(g.nodes.len(), 6);
        let cut = g.pieces.iter().find(|p| p.edge == 0).expect("a piece");
        assert!((cut.s1 - 40.0).abs() < 1e-9, "{cut:?}");
        // The side street's end and the cut are one node, where its first member is: the through street's cut.
        let node = g.nodes[cut.to as usize];
        assert_eq!(node, Vec2::new(40.0, 0.0));
    }

    #[test]
    fn locate_finds_the_nearest_place_and_point_on_walks_the_piece() {
        let edges = [line(1.0, &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)])];
        let (g, _) = Graph::build(rules("ends", 0.01), &edges, &[]);
        let at = g.locate(Vec2::new(10.5, 4.0), 1.0).expect("found");
        assert_eq!(at.piece, 0);
        assert!((at.offset - 14.0).abs() < 1e-12);
        assert_eq!(g.point_on(0, 14.0), Vec2::new(10.0, 4.0));
        assert!(g.locate(Vec2::new(20.0, 4.0), 1.0).is_none());
        let mut out = Vec::new();
        g.span_edges(0, 14.0, 5.0, &mut out);
        assert_eq!(out.len(), 2, "backward over the corner");
        assert_eq!(point_at(&out[0], 0.0), Vec2::new(10.0, 4.0));
        assert_eq!(point_at(&out[1], 1.0), Vec2::new(5.0, 0.0));
    }
}
