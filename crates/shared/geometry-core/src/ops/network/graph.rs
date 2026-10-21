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

use std::collections::HashMap;

use super::access::{Access, Closing, Varied};
use super::clock::Clock;
use super::rules::{Connect, Dir, Rules};
use super::terminals::{NO_DEVICE, NO_TERMINAL, Terminals};
use super::turns::{Level, TurnIn, TurnTable, heading, level_value};
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
    /// Each arc's head: `piece * 2` the piece's end, `piece * 2 + 1` its start (a step's next node at a glance).
    pub arc_head: Vec<u32>,
    tree: PackedTree,
    /// The turns, when the definition has them: searches go by arcs (docs/adr/0521 §5).
    pub turns: Option<TurnTable>,
    /// Each cost's time profile (none for most), `clocks[c]`.
    pub clocks: Vec<Option<Clock>>,
    /// The devices' terminals at the nodes and arcs, when the definition has devices (docs/adr/0557 §1).
    pub terms: Option<Terminals>,
    /// Each edge's profiles' and closures' values, when the definition has some (docs/adr/0567).
    facts: Vec<Vec<Option<String>>>,
    /// The costs and clocks as defined, kept while the graph is varied for a question's access.
    base: Option<(Vec<f64>, Vec<f64>, Vec<Option<Clock>>)>,
    /// The access the costs are for, and what varying them for it closed.
    access: Access,
    varied: Varied,
}

/// A cut along an edge: where (metres from its start), the point and its order among all cuts.
#[derive(Clone, Copy, Debug)]
struct Cut {
    s: f64,
    p: Vec2,
    /// The edge's own place (an end; in Köşelerde a vertex), not another edge's end or a junction cutting it.
    own: bool,
    level: Level,
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
        // The levels (docs/adr/0521 §3): each edge's start, end and inside; each junction's.
        let first_point = |i: usize| {
            edges[i]
                .path
                .first()
                .map_or(Vec2::new(0.0, 0.0), |e| point_at(e, 0.0))
        };
        let levels: Vec<[Level; 3]> = edges
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let Some(inside_given) = rules.levels else {
                    return [Level::At(0); 3];
                };
                let mut bad = false;
                let mut read = |k: usize| {
                    level_value(e.levels[k].as_deref()).unwrap_or_else(|_| {
                        bad = true;
                        None
                    })
                };
                let (a, b) = (read(0).unwrap_or(0), read(1).unwrap_or(0));
                let given = if inside_given { read(2) } else { None };
                let inside = match given {
                    Some(v) => Level::At(v),
                    None if a == b => Level::At(a),
                    None => Level::Own(i as u32),
                };
                if bad {
                    report.unread_other.push((e.id, first_point(i)));
                }
                [Level::At(a), Level::At(b), inside]
            })
            .collect();
        let junction_levels: Vec<Level> = junctions
            .iter()
            .map(|j| {
                if rules.levels.is_none() {
                    return Level::Any;
                }
                match level_value(j.level.as_deref()) {
                    Ok(Some(v)) => Level::At(v),
                    Ok(None) => Level::Any,
                    Err(_) => {
                        report.unread_other.push((j.id, j.p));
                        Level::Any
                    }
                }
            })
            .collect();
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
                level: levels[i][0],
            });
            if rules.connect == Connect::Vertices {
                for (k, p) in e.path.iter().enumerate().skip(1) {
                    cuts[i].push(Cut {
                        s: cums[i][k],
                        p: point_at(p, 0.0),
                        own: true,
                        level: levels[i][2],
                    });
                }
            }
            cuts[i].push(Cut {
                s: lengths[i],
                p: b,
                own: true,
                level: levels[i][1],
            });
        }
        // Where a point at `level` within the tolerance of an edge's inside at its level cuts it; `skip` an edge's
        // own end.
        let touch = |p: Vec2,
                     level: Level,
                     skip: Option<(usize, f64)>,
                     cuts: &mut Vec<Vec<Cut>>,
                     found: &mut Vec<u32>| {
            found.clear();
            near.at(p, found);
            found.sort_unstable();
            for &q in found.iter() {
                let (f, k) = owner[q as usize];
                let (f, k) = (f as usize, k as usize);
                if !level.meets(levels[f][2]) {
                    continue;
                }
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
                    level: levels[f][2],
                });
            }
        };
        for i in 0..edges.len() {
            for (s, end, level) in [
                (0.0, start_of(i), levels[i][0]),
                (lengths[i], end_of(i), levels[i][1]),
            ] {
                if let Some(p) = end {
                    touch(p, level, Some((i, s)), &mut cuts, &mut found);
                }
            }
        }
        for (j, jn) in junctions.iter().enumerate() {
            touch(jn.p, junction_levels[j], None, &mut cuts, &mut found);
        }
        for c in &mut cuts {
            c.sort_by(|a, b| a.s.total_cmp(&b.s));
        }
        // 2. The nodes: every cut and junction point, in order, joined within the tolerance.
        let mut points: Vec<Vec2> = Vec::new();
        let mut point_levels: Vec<Level> = Vec::new();
        for c in &cuts {
            points.extend(c.iter().map(|x| x.p));
            point_levels.extend(c.iter().map(|x| x.level));
        }
        let first_junction = points.len();
        points.extend(junctions.iter().map(|j| j.p));
        point_levels.extend(junction_levels.iter().copied());
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
                        if js_hypot(q.x - p.x, q.y - p.y) <= tol
                            && point_levels[i].meets(point_levels[j as usize])
                        {
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
        // A junction is on the network when its node holds a cut (it touched an edge at its level).
        let mut holds_cut = vec![false; nodes.len()];
        for &n in &node_of[..first_junction] {
            holds_cut[n as usize] = true;
        }
        let attached: Vec<bool> = (0..junctions.len())
            .map(|j| holds_cut[node_of[first_junction + j] as usize])
            .collect();
        for (j, jn) in junctions.iter().enumerate() {
            if !attached[j] {
                report.off_network.push((jn.id, jn.p));
            }
        }
        let mut kinds = vec![NodeKind::default(); nodes.len()];
        let mut junction_meta = Vec::with_capacity(junctions.len());
        let mut node_device = vec![NO_DEVICE; nodes.len()];
        for (j, jn) in junctions.iter().enumerate() {
            let node = attached[j].then(|| node_of[first_junction + j]);
            if let Some(n) = node {
                if let Some(d) = jn.device
                    && rules
                        .terminals
                        .as_ref()
                        .is_some_and(|t| (d as usize) < t.devices.len())
                    && node_device[n as usize] == NO_DEVICE
                {
                    node_device[n as usize] = d;
                }
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
                inside: levels[i][2],
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
        // The time profiles (docs/adr/0521 §4): each edge's speed through the day.
        let mut clocks: Vec<Option<Clock>> = vec![None; costs];
        for (c, clock) in clocks.iter_mut().enumerate().skip(1) {
            let periods = rules.periods.get(c - 1).map_or(&[][..], Vec::as_slice);
            if periods.is_empty() {
                continue;
            }
            let base: Vec<f64> = edges
                .iter()
                .map(|e| {
                    rules
                        .speed_of(c, e.costs.get(c - 1).and_then(|v| v.as_deref()))
                        .map_or(0.0, |s| s.0)
                })
                .collect();
            let values: Vec<Vec<Option<String>>> = edges
                .iter()
                .map(|e| e.periods.get(c - 1).cloned().unwrap_or_default())
                .collect();
            let mut bad = Vec::new();
            *clock = Some(Clock::new(periods, &base, &values, &mut bad));
            for e in bad {
                if !report.unread.contains(&(c, edges[e].id)) {
                    report.unread.push((c, edges[e].id));
                }
            }
        }
        let arc_head = pieces.iter().flat_map(|p| [p.to, p.from]).collect();
        // The devices' terminals (docs/adr/0557 §1): the terminal at each arc's tail, from the edge's start and end
        // fields; an edge running through a device, or an end naming none of its terminals, is said.
        let terms = rules.terminals.as_ref().map(|tr| {
            let mut arc_terminal = vec![NO_TERMINAL; pieces.len() * 2];
            for (k, p) in pieces.iter().enumerate() {
                let e = &edges[p.edge as usize];
                let ends = [
                    (0, p.from, p.s0 == 0.0, &e.terminals[0]),
                    (1, p.to, p.s1 == lengths[p.edge as usize], &e.terminals[1]),
                ];
                for (side, node, at_end, value) in ends {
                    let d = node_device[node as usize];
                    if d == NO_DEVICE {
                        continue;
                    }
                    let place = nodes[node as usize];
                    let named = at_end
                        .then(|| {
                            value
                                .as_deref()
                                .and_then(|v| tr.devices[d as usize].terminal(v))
                        })
                        .flatten();
                    match named {
                        Some(t) => arc_terminal[k * 2 + side] = t,
                        None => report.unterminated.push((e.id, place)),
                    }
                }
            }
            report.unterminated.sort_by(|a, b| {
                a.0.total_cmp(&b.0)
                    .then(a.1.x.total_cmp(&b.1.x))
                    .then(a.1.y.total_cmp(&b.1.y))
            });
            report.unterminated.dedup();
            Terminals::new(&rules, node_device.clone(), arc_terminal)
        });
        let rules_access_empty = rules.access.is_empty();
        let mut g = Graph {
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
            arc_head,
            tree,
            turns: None,
            clocks,
            terms,
            facts: if rules_access_empty {
                Vec::new()
            } else {
                edges.iter().map(|e| e.facts.clone()).collect()
            },
            base: None,
            access: Access::default(),
            varied: Varied::default(),
        };
        if g.rules.turns.is_some() {
            g.turns = Some(g.turn_table(turns, &mut report));
        }
        (g, report)
    }

    /// The turn table (docs/adr/0521 §2): the pieces' headings, the nodes' pieces, which costs are times, and the turn
    /// lines found on two pieces meeting at a node; the others said.
    fn turn_table(&self, turns: &[TurnIn], report: &mut BuildReport) -> TurnTable {
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
    fn crosses(&self, node: u32, in_arc: u32, out_arc: u32) -> bool {
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

    fn vary_from(&mut self, access: Access, (fw0, bw0, clocks0): &(Vec<f64>, Vec<f64>, Vec<Option<Clock>>)) {
        self.access = access;
        self.varied = Varied::default();
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
        for (i, c) in closing.iter_mut().enumerate() {
            let mut unread = false;
            let facts = self.facts.get(i).map_or(&[][..], Vec::as_slice);
            *c = rules.closing(&access, facts, &mut unread);
            match *c {
                Closing::Profile => self.varied.by_profile += 1,
                Closing::Date => self.varied.by_date += 1,
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
            for c in 0..n {
                let at = k * n + c;
                let (mut f, mut b) = (fw0[at], bw0[at]);
                if closing[e] != Closing::Open {
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
