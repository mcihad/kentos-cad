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
//!
//! The independent reference is `scripts/fixtures/network_cases.py`.

use std::collections::HashMap;

use super::rules::{Connect, Dir, Rules};
use crate::geom::arrangement::{edge_box, edge_len, reverse_edge, sub_edge};
use crate::geom::intersect::{Edge, closest_on_edge, point_at};
use crate::geometry::Bounds;
use crate::jsmath::{js_hypot, js_max, js_min};
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

/// The graph.
pub struct Graph {
    pub rules: Rules,
    pub nodes: Vec<Vec2>,
    pub kinds: Vec<NodeKind>,
    pub pieces: Vec<Piece>,
    pub geom: Vec<Edge>,
    /// Where each primitive starts along its piece (metres).
    pub geom_at: Vec<f64>,
    geom_piece: Vec<u32>,
    pub edges: Vec<EdgeMeta>,
    pub junctions: Vec<JunctionMeta>,
    /// Costs: the length and the definition's, `1 + rules.costs.len()`.
    pub costs: usize,
    /// Each piece's cost the way it is drawn and against it, `[piece * costs + c]`; infinite: not travelled.
    pub fw: Vec<f64>,
    pub bw: Vec<f64>,
    /// Each node's pieces: `piece * 2` leaving from its start the way it is drawn, `piece * 2 + 1` from its end against it.
    adj_start: Vec<u32>,
    adj: Vec<u32>,
    tree: PackedTree,
}

/// A cut along an edge: where (metres from its start), the point and its order among all cuts.
#[derive(Clone, Copy, Debug)]
struct Cut {
    s: f64,
    p: Vec2,
    /// The edge's own place (an end; in Köşelerde a vertex), not another edge's end or a junction cutting it.
    own: bool,
}

fn bounds_around(p: Vec2, r: f64) -> Bounds {
    Bounds {
        min_x: p.x - r,
        min_y: p.y - r,
        max_x: p.x + r,
        max_y: p.y + r,
    }
}

/// Boxes grown by a distance in a dense grid of square cells, counted then filled into one array: for the many
/// point questions of building (what an end or a junction touches), a cell's few items instead of a walk down a tree.
/// A box over more than 64 cells goes to a list every question gets.
struct CellIndex {
    x0: f64,
    y0: f64,
    size: f64,
    nx: usize,
    ny: usize,
    start: Vec<u32>,
    items: Vec<u32>,
    big: Vec<u32>,
}

impl CellIndex {
    fn new(boxes: &[(u32, Bounds)], grow: f64) -> CellIndex {
        let n = boxes.len().max(1);
        let (mut x0, mut y0, mut x1, mut y1) = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        let mut sum = 0.0;
        for (_, b) in boxes {
            x0 = js_min(x0, b.min_x - grow);
            y0 = js_min(y0, b.min_y - grow);
            x1 = js_max(x1, b.max_x + grow);
            y1 = js_max(y1, b.max_y + grow);
            sum += js_max(b.max_x - b.min_x, b.max_y - b.min_y) + 2.0 * grow;
        }
        if !x0.is_finite() {
            (x0, y0, x1, y1) = (0.0, 0.0, 0.0, 0.0);
        }
        // Cells about the boxes' usual size, at most four for each box over the whole extent.
        let mut size = js_max(sum / n as f64, 1e-9);
        let most = (4 * n).max(1024) as f64;
        while ((x1 - x0) / size + 1.0) * ((y1 - y0) / size + 1.0) > most {
            size *= 2.0;
        }
        let nx = ((x1 - x0) / size) as usize + 1;
        let ny = ((y1 - y0) / size) as usize + 1;
        let span = |b: &Bounds| {
            let cx = |x: f64| (((x - x0) / size) as usize).min(nx - 1);
            let cy = |y: f64| (((y - y0) / size) as usize).min(ny - 1);
            (
                cx(b.min_x - grow),
                cx(b.max_x + grow),
                cy(b.min_y - grow),
                cy(b.max_y + grow),
            )
        };
        let mut start = vec![0u32; nx * ny + 1];
        let mut big = Vec::new();
        for (_, b) in boxes {
            let (a, c, d, e) = span(b);
            if (c - a + 1) * (e - d + 1) > 64 {
                continue;
            }
            for y in d..=e {
                for x in a..=c {
                    start[y * nx + x + 1] += 1;
                }
            }
        }
        for k in 0..nx * ny {
            start[k + 1] += start[k];
        }
        let mut fill = start.clone();
        let mut items = vec![0u32; start[nx * ny] as usize];
        for (id, b) in boxes {
            let (a, c, d, e) = span(b);
            if (c - a + 1) * (e - d + 1) > 64 {
                big.push(*id);
                continue;
            }
            for y in d..=e {
                for x in a..=c {
                    items[fill[y * nx + x] as usize] = *id;
                    fill[y * nx + x] += 1;
                }
            }
        }
        CellIndex {
            x0,
            y0,
            size,
            nx,
            ny,
            start,
            items,
            big,
        }
    }

    /// Into `out`: every box whose grown box holds `p`, and some more.
    fn at(&self, p: Vec2, out: &mut Vec<u32>) {
        out.clear();
        out.extend_from_slice(&self.big);
        let (fx, fy) = ((p.x - self.x0) / self.size, (p.y - self.y0) / self.size);
        if !(fx >= 0.0 && fy >= 0.0) || fx >= self.nx as f64 || fy >= self.ny as f64 {
            return;
        }
        let k = fy as usize * self.nx + fx as usize;
        out.extend_from_slice(&self.items[self.start[k] as usize..self.start[k + 1] as usize]);
    }
}

fn find(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        let up = parent[parent[x as usize] as usize];
        parent[x as usize] = up;
        x = up;
    }
    x
}

/// The primitives of an edge from `a` to `b` metres along it, `cum` their starts.
fn span_of(path: &[Edge], cum: &[f64], a: f64, b: f64, out: &mut Vec<Edge>, at: &mut Vec<f64>) {
    let mut along = 0.0;
    for (k, e) in path.iter().enumerate() {
        let (k0, len) = (cum[k], edge_len(e));
        let k1 = k0 + len;
        if len <= 0.0 || k1 <= a || k0 >= b {
            continue;
        }
        let t0 = if a > k0 { (a - k0) / len } else { 0.0 };
        let t1 = if b < k1 { (b - k0) / len } else { 1.0 };
        let piece = if t0 == 0.0 && t1 == 1.0 {
            *e
        } else {
            sub_edge(e, t0, t1)
        };
        at.push(along);
        along += edge_len(&piece);
        out.push(piece);
    }
}

impl Graph {
    /// Builds the graph of `edges` and `junctions` by `rules` (§3).
    pub fn build(rules: Rules, edges: &[EdgeIn], junctions: &[JunctionIn]) -> (Graph, BuildReport) {
        let tol = rules.tolerance;
        let mut report = BuildReport::default();
        // Each edge's primitives' starts along it.
        let cums: Vec<Vec<f64>> = edges
            .iter()
            .map(|e| {
                let mut c = Vec::with_capacity(e.path.len() + 1);
                let mut s = 0.0;
                for p in &e.path {
                    c.push(s);
                    s += edge_len(p);
                }
                c.push(s);
                c
            })
            .collect();
        let lengths: Vec<f64> = cums.iter().map(|c| *c.last().unwrap_or(&0.0)).collect();
        // Every edge's primitives in one tree, to find what is near an end or a junction.
        let mut owner: Vec<(u32, u32)> = Vec::new();
        let mut boxes: Vec<(u32, Bounds)> = Vec::new();
        for (i, e) in edges.iter().enumerate() {
            for (k, p) in e.path.iter().enumerate() {
                boxes.push((owner.len() as u32, edge_box(p)));
                owner.push((i as u32, k as u32));
            }
        }
        let near = CellIndex::new(&boxes, tol);
        let mut found: Vec<u32> = Vec::new();
        let start_of =
            |i: usize| -> Option<Vec2> { edges[i].path.first().map(|e| point_at(e, 0.0)) };
        let end_of = |i: usize| -> Option<Vec2> { edges[i].path.last().map(|e| point_at(e, 1.0)) };
        // 1. The cuts.
        let mut cuts: Vec<Vec<Cut>> = vec![Vec::new(); edges.len()];
        for (i, e) in edges.iter().enumerate() {
            let (Some(a), Some(b)) = (start_of(i), end_of(i)) else {
                continue;
            };
            cuts[i].push(Cut {
                s: 0.0,
                p: a,
                own: true,
            });
            if rules.connect == Connect::Vertices {
                for (k, p) in e.path.iter().enumerate().skip(1) {
                    cuts[i].push(Cut {
                        s: cums[i][k],
                        p: point_at(p, 0.0),
                        own: true,
                    });
                }
            }
            cuts[i].push(Cut {
                s: lengths[i],
                p: b,
                own: true,
            });
        }
        // Where a point within the tolerance of an edge's inside cuts it; `skip` an edge's own end.
        let touch = |p: Vec2,
                     skip: Option<(usize, f64)>,
                     cuts: &mut Vec<Vec<Cut>>,
                     found: &mut Vec<u32>|
         -> bool {
            found.clear();
            near.at(p, found);
            found.sort_unstable();
            let mut any = false;
            for &q in found.iter() {
                let (f, k) = owner[q as usize];
                let (f, k) = (f as usize, k as usize);
                let prim = &edges[f].path[k];
                let c = closest_on_edge(prim, p);
                if c.d > tol {
                    continue;
                }
                let s = cums[f][k] + c.t * edge_len(prim);
                if let Some((own, at)) = skip
                    && own == f
                    && (s - at).abs() <= tol
                {
                    continue;
                }
                any = true;
                // Exactly at the other edge's own end: the cut there is that end's own, the same point bit for bit
                // (`point_at` at 0 or 1 both ways); a second would only be a piece of no length.
                if (s == 0.0 && c.t == 0.0 && k == 0)
                    || (s == lengths[f] && c.t == 1.0 && k + 1 == edges[f].path.len())
                {
                    continue;
                }
                cuts[f].push(Cut {
                    s,
                    p: c.p,
                    own: false,
                });
            }
            any
        };
        for i in 0..edges.len() {
            for (s, end) in [(0.0, start_of(i)), (lengths[i], end_of(i))] {
                if let Some(p) = end {
                    touch(p, Some((i, s)), &mut cuts, &mut found);
                }
            }
        }
        let mut attached = vec![false; junctions.len()];
        for (j, jn) in junctions.iter().enumerate() {
            attached[j] = touch(jn.p, None, &mut cuts, &mut found);
            if !attached[j] {
                report.off_network.push((jn.id, jn.p));
            }
        }
        for c in &mut cuts {
            c.sort_by(|a, b| a.s.total_cmp(&b.s));
        }
        // 2. The nodes: every cut and junction point, in order, joined within the tolerance.
        let mut points: Vec<Vec2> = Vec::new();
        for c in &cuts {
            points.extend(c.iter().map(|x| x.p));
        }
        let first_junction = points.len();
        points.extend(junctions.iter().map(|j| j.p));
        let mut parent: Vec<u32> = (0..points.len() as u32).collect();
        let cell =
            |p: Vec2| -> (i64, i64) { ((p.x / tol).floor() as i64, (p.y / tol).floor() as i64) };
        // The points by their cells: sorted once, each cell a run (no list a cell).
        let mut by_cell: Vec<((i64, i64), u32)> = points
            .iter()
            .enumerate()
            .map(|(i, p)| (cell(*p), i as u32))
            .collect();
        by_cell.sort_unstable();
        let mut runs: HashMap<(i64, i64), (u32, u32)> = HashMap::with_capacity(by_cell.len());
        let mut start = 0;
        for k in 1..=by_cell.len() {
            if k == by_cell.len() || by_cell[k].0 != by_cell[start].0 {
                runs.insert(by_cell[start].0, (start as u32, k as u32));
                start = k;
            }
        }
        for (i, p) in points.iter().enumerate() {
            let (cx, cy) = cell(*p);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let Some(&(a, b)) = runs.get(&(cx + dx, cy + dy)) else {
                        continue;
                    };
                    for &(_, j) in &by_cell[a as usize..b as usize] {
                        if j as usize <= i {
                            continue;
                        }
                        let q = points[j as usize];
                        if js_hypot(q.x - p.x, q.y - p.y) <= tol {
                            let (a, b) = (find(&mut parent, i as u32), find(&mut parent, j));
                            if a != b {
                                // The lower root stays: a node is where its first member is.
                                let (lo, hi) = if a < b { (a, b) } else { (b, a) };
                                parent[hi as usize] = lo;
                            }
                        }
                    }
                }
            }
        }
        let mut node_of_root = vec![u32::MAX; points.len()];
        let mut node_of = vec![0u32; points.len()];
        let mut nodes: Vec<Vec2> = Vec::new();
        for i in 0..points.len() {
            let r = find(&mut parent, i as u32) as usize;
            if node_of_root[r] == u32::MAX {
                nodes.push(points[r]);
                node_of_root[r] = (nodes.len() - 1) as u32;
            }
            node_of[i] = node_of_root[r];
        }
        let mut kinds = vec![NodeKind::default(); nodes.len()];
        let mut junction_meta = Vec::with_capacity(junctions.len());
        for (j, jn) in junctions.iter().enumerate() {
            let node = attached[j].then(|| node_of[first_junction + j]);
            if let Some(n) = node {
                let k = &mut kinds[n as usize];
                match jn.role {
                    Role::Source => k.source = true,
                    Role::Valve => k.valve = true,
                    Role::Junction => {}
                }
                if jn.closed {
                    k.closed = true;
                }
            }
            junction_meta.push(JunctionMeta {
                id: jn.id,
                p: jn.p,
                role: jn.role,
                closed: jn.closed,
                node,
            });
        }
        // 3. The pieces and their costs.
        let costs = 1 + rules.costs.len();
        let mut pieces: Vec<Piece> = Vec::new();
        let mut geom: Vec<Edge> = Vec::new();
        let mut geom_at: Vec<f64> = Vec::new();
        let mut fw: Vec<f64> = Vec::new();
        let mut bw: Vec<f64> = Vec::new();
        let mut metas = Vec::with_capacity(edges.len());
        let mut order = 0usize;
        for (i, e) in edges.iter().enumerate() {
            let dir = if e.closed {
                Dir::Closed
            } else {
                rules.direction_of(e.direction.as_deref())
            };
            let cs = &cuts[i];
            metas.push(EdgeMeta {
                id: e.id,
                part: e.part,
                length: lengths[i],
                dir,
                closed: e.closed,
                a: cs.first().map_or(Vec2::new(0.0, 0.0), |c| c.p),
                b: cs.last().map_or(Vec2::new(0.0, 0.0), |c| c.p),
                nodes: (!cs.is_empty()).then(|| (node_of[order], node_of[order + cs.len() - 1])),
            });
            let mut rates = vec![Some(1.0); costs];
            for c in 1..costs {
                let (r, unread) =
                    rules.rate(c, e.costs.get(c - 1).and_then(|v| v.as_deref()), lengths[i]);
                rates[c] = r;
                if unread {
                    report.unread.push((c, e.id));
                }
            }
            // The data's own short stretches (§9): the edge's own places (its ends; in Köşelerde its vertices) closer
            // than the tolerance along it. A sliver another end or a junction cuts beside a place (an arc's end computed
            // a hair short of its vertex) is dropped below unsaid.
            let mut own = cs.iter().filter(|c| c.own);
            if let Some(mut prev) = own.next() {
                for c in own {
                    let len = c.s - prev.s;
                    if len > 0.0 && len <= tol {
                        report.short.push((e.id, prev.p));
                    }
                    prev = c;
                }
            }
            for w in 0..cs.len().saturating_sub(1) {
                let (a, b) = (cs[w], cs[w + 1]);
                let (na, nb) = (node_of[order + w], node_of[order + w + 1]);
                let len = b.s - a.s;
                if na == nb && len <= tol {
                    continue;
                }
                let g0 = geom.len() as u32;
                span_of(&e.path, &cums[i], a.s, b.s, &mut geom, &mut geom_at);
                let g1 = geom.len() as u32;
                for c in 0..costs {
                    let cost = rates[c].map_or(f64::INFINITY, |r| r * len);
                    fw.push(if dir.allows(true) {
                        cost
                    } else {
                        f64::INFINITY
                    });
                    bw.push(if dir.allows(false) {
                        cost
                    } else {
                        f64::INFINITY
                    });
                }
                pieces.push(Piece {
                    from: na,
                    to: nb,
                    edge: i as u32,
                    s0: a.s,
                    s1: b.s,
                    len,
                    g0,
                    g1,
                });
            }
            if lengths[i] <= 0.0 && !e.path.is_empty() {
                report
                    .short
                    .push((e.id, cs.first().map_or(Vec2::new(0.0, 0.0), |c| c.p)));
            }
            order += cs.len();
        }
        // The nodes' pieces, in the pieces' order.
        let mut degree = vec![0u32; nodes.len() + 1];
        for p in &pieces {
            degree[p.from as usize + 1] += 1;
            degree[p.to as usize + 1] += 1;
        }
        for n in 0..nodes.len() {
            degree[n + 1] += degree[n];
        }
        let adj_start = degree.clone();
        let mut fill = degree;
        let mut adj = vec![0u32; pieces.len() * 2];
        for (k, p) in pieces.iter().enumerate() {
            adj[fill[p.from as usize] as usize] = (k * 2) as u32;
            fill[p.from as usize] += 1;
            adj[fill[p.to as usize] as usize] = (k * 2 + 1) as u32;
            fill[p.to as usize] += 1;
        }
        let mut geom_piece = vec![0u32; geom.len()];
        for (k, p) in pieces.iter().enumerate() {
            for g in p.g0..p.g1 {
                geom_piece[g as usize] = k as u32;
            }
        }
        let entries: Vec<(u32, Bounds)> = geom
            .iter()
            .enumerate()
            .map(|(g, e)| (g as u32, edge_box(e)))
            .collect();
        let tree = PackedTree::build(&entries);
        (
            Graph {
                rules,
                nodes,
                kinds,
                pieces,
                geom,
                geom_at,
                geom_piece,
                edges: metas,
                junctions: junction_meta,
                costs,
                fw,
                bw,
                adj_start,
                adj,
                tree,
            },
            report,
        )
    }

    /// The pieces at node `n`: `piece * 2` leaving its start the way it is drawn, `piece * 2 + 1` leaving its end against it.
    pub fn at_node(&self, n: u32) -> &[u32] {
        &self.adj[self.adj_start[n as usize] as usize..self.adj_start[n as usize + 1] as usize]
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
