//! Building a network's graph (docs/adr/0209 §3) in stages a rebuild shares (docs/adr/0641 §1): the edges' lengths and
//! levels, their primitives in a cell index, the cuts, the points joined into nodes, and the graph put together from
//! them. `Graph::build_with` runs every stage; `Graph::rebuild` (rebuild.rs) runs the cuts and the joining only where
//! the edges changed and the rest as here, so the two give the same graph bit for bit.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use super::access::{Access, Varied};
use super::clock::Clock;
use super::graph::{
    BuildReport, EdgeIn, EdgeMeta, Graph, JunctionIn, JunctionMeta, NodeKind, Piece,
};
use super::rebuild::Kept;
use super::rules::{Connect, Dir, Rules};
use super::terminals::{NO_DEVICE, NO_TERMINAL, Terminals};
use super::turns::{Level, TurnIn, level_value};
use crate::geom::arrangement::{edge_box, edge_len, sub_edge};
use crate::geom::intersect::{Edge, closest_on_edge, point_at};
use crate::geometry::Bounds;
use crate::jsmath::{js_hypot, js_max, js_min};
use crate::store::rtree::PackedTree;
use crate::vec2::Vec2;

/// A cut along an edge: where (metres from its start), the point and its order among all cuts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Cut {
    pub s: f64,
    pub p: Vec2,
    /// The edge's own place (an end; in Köşelerde a vertex), not another edge's end or a junction cutting it.
    pub own: bool,
    pub level: Level,
}

/// How far the cell indexes grow a box: the tolerance and a hair more, so that every point the nearest-point test
/// finds within the tolerance is among an index's candidates whatever the index's cells (a full build's and a
/// rebuild's then find the same; docs/adr/0641 §1).
pub(super) fn reach(tol: f64) -> f64 {
    tol + js_max(tol * 1e-6, 1e-6)
}

/// Boxes grown by a distance in a dense grid of square cells, counted then filled into one array: for the many
/// point questions of building (what an end or a junction touches), a cell's few items instead of a walk down a tree.
/// A box over more than 64 cells goes to a list every question gets.
pub(super) struct CellIndex {
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
    pub fn new(boxes: &[(u32, Bounds)], grow: f64) -> CellIndex {
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
    pub fn at(&self, p: Vec2, out: &mut Vec<u32>) {
        out.clear();
        out.extend_from_slice(&self.big);
        let (fx, fy) = ((p.x - self.x0) / self.size, (p.y - self.y0) / self.size);
        if !(fx >= 0.0 && fy >= 0.0) || fx >= self.nx as f64 || fy >= self.ny as f64 {
            return;
        }
        let k = fy as usize * self.nx + fx as usize;
        out.extend_from_slice(&self.items[self.start[k] as usize..self.start[k + 1] as usize]);
    }

    /// Into `out`: every box whose grown box meets `b`, and some more (each once per cell it is in).
    pub fn within(&self, b: &Bounds, out: &mut Vec<u32>) {
        out.extend_from_slice(&self.big);
        let cx = |x: f64| ((x - self.x0) / self.size).floor();
        let cy = |y: f64| ((y - self.y0) / self.size).floor();
        let (x0, x1, y0, y1) = (cx(b.min_x), cx(b.max_x), cy(b.min_y), cy(b.max_y));
        if !(x1 >= 0.0 && y1 >= 0.0) || x0 >= self.nx as f64 || y0 >= self.ny as f64 {
            return;
        }
        let clamp = |v: f64, n: usize| (js_max(v, 0.0) as usize).min(n - 1);
        for y in clamp(y0, self.ny)..=clamp(y1, self.ny) {
            for x in clamp(x0, self.nx)..=clamp(x1, self.nx) {
                let k = y * self.nx + x;
                out.extend_from_slice(
                    &self.items[self.start[k] as usize..self.start[k + 1] as usize],
                );
            }
        }
    }

    /// The bytes it holds, about (a kept build's share, docs/adr/0641 §1).
    pub fn bytes(&self) -> usize {
        (self.start.len() + self.items.len() + self.big.len()) * 4
    }
}

pub(super) fn find(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        let up = parent[parent[x as usize] as usize];
        parent[x as usize] = up;
        x = up;
    }
    x
}

/// Joins two points' sets: the lower root stays, so a set's root is always its first member.
pub(super) fn unite(parent: &mut [u32], a: u32, b: u32) {
    let (a, b) = (find(parent, a), find(parent, b));
    if a != b {
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        parent[hi as usize] = lo;
    }
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

/// A cheap hash for the cells (two whole numbers): the joining asks a map millions of times.
#[derive(Default)]
pub(super) struct CellHash(u64);

impl Hasher for CellHash {
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.write_u64(u64::from(*b));
        }
    }

    fn write_i64(&mut self, v: i64) {
        self.write_u64(v as u64);
    }

    fn write_u64(&mut self, v: u64) {
        self.0 = (self.0.rotate_left(5) ^ v).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

pub(super) type CellMap<V> = HashMap<(i64, i64), V, BuildHasherDefault<CellHash>>;

/// What every edge and junction gives before the cuts: each edge's primitives' starts along it (`cums`, edge `i`'s
/// at `cum_start[i]..cum_start[i + 1]`, one more than its primitives), its length and its levels (start, end,
/// inside); each junction's level.
pub(super) struct Prep {
    pub cum_start: Vec<u32>,
    pub cums: Vec<f64>,
    pub lengths: Vec<f64>,
    pub levels: Vec<[Level; 3]>,
    pub junction_levels: Vec<Level>,
}

impl Prep {
    pub fn new(
        rules: &Rules,
        edges: &[EdgeIn],
        junctions: &[JunctionIn],
        report: &mut BuildReport,
    ) -> Prep {
        let mut cum_start = Vec::with_capacity(edges.len() + 1);
        let mut cums = Vec::with_capacity(edges.iter().map(|e| e.path.len() + 1).sum());
        let mut lengths = Vec::with_capacity(edges.len());
        for e in edges {
            cum_start.push(cums.len() as u32);
            let mut s = 0.0;
            for p in &e.path {
                cums.push(s);
                s += edge_len(p);
            }
            cums.push(s);
            lengths.push(s);
        }
        cum_start.push(cums.len() as u32);
        // The levels (docs/adr/0521 §3): each edge's start, end and inside; each junction's.
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
                    let first = e
                        .path
                        .first()
                        .map_or(Vec2::new(0.0, 0.0), |p| point_at(p, 0.0));
                    report.unread_other.push((e.id, first));
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
        Prep {
            cum_start,
            cums,
            lengths,
            levels,
            junction_levels,
        }
    }

    pub fn cum(&self, i: usize) -> &[f64] {
        &self.cums[self.cum_start[i] as usize..self.cum_start[i + 1] as usize]
    }
}

pub(super) fn start_of(e: &EdgeIn) -> Option<Vec2> {
    e.path.first().map(|p| point_at(p, 0.0))
}

pub(super) fn end_of(e: &EdgeIn) -> Option<Vec2> {
    e.path.last().map(|p| point_at(p, 1.0))
}

/// Edge `i`'s own cuts, into `out`: its ends (in Köşelerde its vertices too).
pub(super) fn own_cuts(
    rules: &Rules,
    edges: &[EdgeIn],
    prep: &Prep,
    i: usize,
    out: &mut Vec<(u32, Cut)>,
) {
    let e = &edges[i];
    let (Some(a), Some(b)) = (start_of(e), end_of(e)) else {
        return;
    };
    let f = i as u32;
    out.push((
        f,
        Cut {
            s: 0.0,
            p: a,
            own: true,
            level: prep.levels[i][0],
        },
    ));
    if rules.connect == Connect::Vertices {
        let cum = prep.cum(i);
        for (k, p) in e.path.iter().enumerate().skip(1) {
            out.push((
                f,
                Cut {
                    s: cum[k],
                    p: point_at(p, 0.0),
                    own: true,
                    level: prep.levels[i][2],
                },
            ));
        }
    }
    out.push((
        f,
        Cut {
            s: prep.lengths[i],
            p: b,
            own: true,
            level: prep.levels[i][1],
        },
    ));
}

/// Where a point at `level` within the tolerance of an edge's inside at its level cuts it, into `out`: `found` the
/// candidate primitives (ascending), `owner` each one's edge and place in its path; `skip` an edge's own end.
#[allow(clippy::too_many_arguments)]
pub(super) fn touch(
    edges: &[EdgeIn],
    prep: &Prep,
    tol: f64,
    owner: &[(u32, u32)],
    found: &[u32],
    p: Vec2,
    level: Level,
    skip: Option<(usize, f64)>,
    out: &mut Vec<(u32, Cut)>,
) {
    for &q in found {
        let (f, k) = owner[q as usize];
        let (f, k) = (f as usize, k as usize);
        if !level.meets(prep.levels[f][2]) {
            continue;
        }
        let prim = &edges[f].path[k];
        let c = closest_on_edge(prim, p);
        if c.d > tol {
            continue;
        }
        let s = prep.cum(f)[k] + c.t * edge_len(prim);
        if let Some((own, at)) = skip
            && own == f
            && (s - at).abs() <= tol
        {
            continue;
        }
        // Exactly at the other edge's own end: the cut there is that end's own, the same point bit for bit
        // (`point_at` at 0 or 1 both ways); a second would only be a piece of no length.
        if (s == 0.0 && c.t == 0.0 && k == 0)
            || (s == prep.lengths[f] && c.t == 1.0 && k + 1 == edges[f].path.len())
        {
            continue;
        }
        out.push((
            f as u32,
            Cut {
                s,
                p: c.p,
                own: false,
                level: prep.levels[f][2],
            },
        ));
    }
}

/// The cuts pushed edge by edge (each edge's in the order pushed) laid out edge after edge, each edge's sorted along it
/// (equal places keep their order): `start[i]..start[i + 1]` edge `i`'s.
pub(super) fn lay_out(n: usize, pushed: Vec<(u32, Cut)>) -> (Vec<u32>, Vec<Cut>) {
    let mut start = vec![0u32; n + 1];
    for (f, _) in &pushed {
        start[*f as usize + 1] += 1;
    }
    for i in 0..n {
        start[i + 1] += start[i];
    }
    let mut fill = start.clone();
    let blank = Cut {
        s: 0.0,
        p: Vec2::new(0.0, 0.0),
        own: false,
        level: Level::Any,
    };
    let mut cuts = vec![blank; pushed.len()];
    for (f, c) in pushed {
        cuts[fill[f as usize] as usize] = c;
        fill[f as usize] += 1;
    }
    for i in 0..n {
        cuts[start[i] as usize..start[i + 1] as usize].sort_by(|a, b| a.s.total_cmp(&b.s));
    }
    (start, cuts)
}

/// The edges' primitives, each with its edge and place in its path, and the cell index of their boxes.
pub(super) fn primitives(edges: &[EdgeIn], tol: f64) -> (Vec<(u32, u32)>, CellIndex) {
    let mut owner: Vec<(u32, u32)> = Vec::new();
    let mut boxes: Vec<(u32, Bounds)> = Vec::new();
    for (i, e) in edges.iter().enumerate() {
        for (k, p) in e.path.iter().enumerate() {
            boxes.push((owner.len() as u32, edge_box(p)));
            owner.push((i as u32, k as u32));
        }
    }
    let near = CellIndex::new(&boxes, reach(tol));
    (owner, near)
}

/// The points joined: each point's node, the nodes (each where its first point is), the level gaps.
pub(super) struct Joined {
    pub node_of: Vec<u32>,
    pub nodes: Vec<Vec2>,
    pub gap_start: Vec<u32>,
    pub gap_to: Vec<u32>,
}

/// The points' sets (`parent`, roots their first members) as nodes, with the level gaps between the nodes those
/// point pairs (`level_pairs`) are on, each way, each pair once.
pub(super) fn nodes_of(points: &[Vec2], parent: &mut [u32], level_pairs: &[(u32, u32)]) -> Joined {
    let mut node_of_root = vec![u32::MAX; points.len()];
    let mut node_of = vec![0u32; points.len()];
    let mut nodes: Vec<Vec2> = Vec::new();
    for i in 0..points.len() {
        let r = find(parent, i as u32) as usize;
        if node_of_root[r] == u32::MAX {
            nodes.push(points[r]);
            node_of_root[r] = (nodes.len() - 1) as u32;
        }
        node_of[i] = node_of_root[r];
    }
    let gaps = level_pairs
        .iter()
        .map(|&(i, j)| (node_of[i as usize], node_of[j as usize]));
    let (gap_start, gap_to) = gaps_of(nodes.len(), gaps);
    Joined {
        node_of,
        nodes,
        gap_start,
        gap_to,
    }
}

/// The level gaps of node pairs: each way, each pair once, a node's own none.
pub(super) fn gaps_of(
    nodes: usize,
    pairs: impl Iterator<Item = (u32, u32)>,
) -> (Vec<u32>, Vec<u32>) {
    let mut gaps: Vec<(u32, u32)> = pairs
        .filter(|(a, b)| a != b)
        .flat_map(|(a, b)| [(a, b), (b, a)])
        .collect();
    gaps.sort_unstable();
    gaps.dedup();
    if gaps.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let mut start = vec![0u32; nodes + 1];
    for &(a, _) in &gaps {
        start[a as usize + 1] += 1;
    }
    for k in 1..start.len() {
        start[k] += start[k - 1];
    }
    (start, gaps.iter().map(|g| g.1).collect())
}

/// The cell of a point for the joining: squares as large as the tolerance.
pub(super) fn cell(p: Vec2, tol: f64) -> (i64, i64) {
    ((p.x / tol).floor() as i64, (p.y / tol).floor() as i64)
}

/// Every point joined with those within the tolerance whose levels meet (the chain along); the pairs whose levels
/// keep them apart noted. Each pair of points in the same or neighbouring cells is looked at once: a cell's own pairs,
/// then its pairs with the four cells after it.
pub(super) fn join_all(points: &[Vec2], levels: &[Level], tol: f64) -> Joined {
    let mut parent: Vec<u32> = (0..points.len() as u32).collect();
    let mut level_pairs: Vec<(u32, u32)> = Vec::new();
    // The points by their cells: sorted once, each cell a run (no list a cell).
    let mut by_cell: Vec<((i64, i64), u32)> = points
        .iter()
        .enumerate()
        .map(|(i, p)| (cell(*p, tol), i as u32))
        .collect();
    by_cell.sort_unstable();
    let mut runs: CellMap<(u32, u32)> = CellMap::default();
    runs.reserve(by_cell.len());
    let mut start = 0;
    for k in 1..=by_cell.len() {
        if k == by_cell.len() || by_cell[k].0 != by_cell[start].0 {
            runs.insert(by_cell[start].0, (start as u32, k as u32));
            start = k;
        }
    }
    let mut pair = |i: u32, j: u32, parent: &mut [u32]| {
        let (p, q) = (points[i as usize], points[j as usize]);
        if js_hypot(q.x - p.x, q.y - p.y) > tol {
            return;
        }
        if !levels[i as usize].meets(levels[j as usize]) {
            level_pairs.push((i.min(j), i.max(j)));
            return;
        }
        unite(parent, i, j);
    };
    let mut k = 0;
    while k < by_cell.len() {
        let (c, (a, b)) = (by_cell[k].0, runs[&by_cell[k].0]);
        let (a, b) = (a as usize, b as usize);
        for x in a..b {
            for y in x + 1..b {
                pair(by_cell[x].1, by_cell[y].1, &mut parent);
            }
        }
        for (dx, dy) in [(0, 1), (1, -1), (1, 0), (1, 1)] {
            let Some(&(a2, b2)) = runs.get(&(c.0 + dx, c.1 + dy)) else {
                continue;
            };
            for x in a..b {
                for y in a2 as usize..b2 as usize {
                    pair(by_cell[x].1, by_cell[y].1, &mut parent);
                }
            }
        }
        k = b;
    }
    nodes_of(points, &mut parent, &level_pairs)
}

/// The points to join: every edge's cuts in order, then the junction points; and their levels.
pub(super) fn points_of(
    cuts: &[Cut],
    junctions: &[JunctionIn],
    prep: &Prep,
) -> (Vec<Vec2>, Vec<Level>) {
    let mut points: Vec<Vec2> = Vec::with_capacity(cuts.len() + junctions.len());
    let mut levels: Vec<Level> = Vec::with_capacity(cuts.len() + junctions.len());
    points.extend(cuts.iter().map(|c| c.p));
    levels.extend(cuts.iter().map(|c| c.level));
    points.extend(junctions.iter().map(|j| j.p));
    levels.extend(prep.junction_levels.iter().copied());
    (points, levels)
}

/// What a rebuild takes from the last graph as it is (docs/adr/0641 §1): an edge carried over (its cuts and their
/// nodes' joins as before) keeps its pieces, their primitives and costs, and its cost fields' unread notes.
pub(super) struct Reuse<'a> {
    pub old: &'a Graph,
    /// Each old edge's first piece, `old.edges.len() + 1` of them.
    pub piece_start: Vec<u32>,
    /// Each new edge's old edge it is carried over from; none: put together again.
    pub carry: Vec<u32>,
    /// The old edges' unread cost fields, `(edge, cost)` in the build's order, and each old edge's first.
    pub unread: &'a [(u32, u32)],
    pub unread_start: Vec<u32>,
}

/// What a build has made before the graph is put together.
pub(super) struct Made {
    pub prep: Prep,
    pub cut_start: Vec<u32>,
    pub cuts: Vec<Cut>,
    pub joined: Joined,
}

impl Graph {
    /// Builds the graph of `edges`, `junctions` and the turn layers' lines `turns` by `rules` (§3, docs/adr/0521);
    /// `keep`: with what a rebuild reuses (docs/adr/0641 §1).
    pub(super) fn build_all(
        rules: Rules,
        edges: &[EdgeIn],
        junctions: &[JunctionIn],
        turns: &[TurnIn],
        keep: bool,
    ) -> (Graph, BuildReport) {
        let tol = rules.tolerance;
        let mut report = BuildReport::default();
        let prep = Prep::new(&rules, edges, junctions, &mut report);
        // Every edge's primitives in one index, to find what is near an end or a junction.
        let (owner, near) = primitives(edges, tol);
        let mut found: Vec<u32> = Vec::new();
        // 1. The cuts.
        let mut pushed: Vec<(u32, Cut)> = Vec::with_capacity(edges.len() * 2);
        for i in 0..edges.len() {
            own_cuts(&rules, edges, &prep, i, &mut pushed);
        }
        for (i, e) in edges.iter().enumerate() {
            for (s, end, level) in [
                (0.0, start_of(e), prep.levels[i][0]),
                (prep.lengths[i], end_of(e), prep.levels[i][1]),
            ] {
                if let Some(p) = end {
                    near.at(p, &mut found);
                    found.sort_unstable();
                    touch(
                        edges,
                        &prep,
                        tol,
                        &owner,
                        &found,
                        p,
                        level,
                        Some((i, s)),
                        &mut pushed,
                    );
                }
            }
        }
        for (j, jn) in junctions.iter().enumerate() {
            near.at(jn.p, &mut found);
            found.sort_unstable();
            touch(
                edges,
                &prep,
                tol,
                &owner,
                &found,
                jn.p,
                prep.junction_levels[j],
                None,
                &mut pushed,
            );
        }
        let (cut_start, cuts) = lay_out(edges.len(), pushed);
        // 2. The nodes: every cut and junction point, in order, joined within the tolerance.
        let (points, point_levels) = points_of(&cuts, junctions, &prep);
        let joined = join_all(&points, &point_levels, tol);
        let made = Made {
            prep,
            cut_start,
            cuts,
            joined,
        };
        let (mut g, report, unread) =
            Self::put_together(rules, edges, junctions, turns, &made, report, None);
        if keep {
            g.kept = Some(Box::new(Kept::new(
                &g.rules, edges, junctions, made, owner, near, unread,
            )));
        }
        (g, report)
    }

    /// The graph put together from what the build made (§3 from the junctions on); with `reuse`, the carried edges'
    /// pieces from the last graph. Also each edge's unread cost fields, `(edge, cost)`, for the next rebuild.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn put_together(
        rules: Rules,
        edges: &[EdgeIn],
        junctions: &[JunctionIn],
        turns: &[TurnIn],
        made: &Made,
        mut report: BuildReport,
        reuse: Option<&Reuse>,
    ) -> (Graph, BuildReport, Vec<(u32, u32)>) {
        let tol = rules.tolerance;
        let Made {
            prep,
            cut_start,
            cuts,
            joined,
        } = made;
        let Joined {
            node_of,
            nodes,
            gap_start,
            gap_to,
        } = joined;
        let nodes = nodes.clone();
        let first_junction = cuts.len();
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
                    super::graph::Role::Source => k.source = true,
                    super::graph::Role::Valve => k.valve = true,
                    super::graph::Role::Junction => {}
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
        let mut rates = vec![Some(1.0); costs];
        let mut unread_at: Vec<(u32, u32)> = Vec::new();
        for (i, e) in edges.iter().enumerate() {
            let dir = if e.closed {
                Dir::Closed
            } else {
                rules.direction_of(e.direction.as_deref())
            };
            let order = cut_start[i] as usize;
            let cs = &cuts[order..cut_start[i + 1] as usize];
            metas.push(EdgeMeta {
                id: e.id,
                part: e.part,
                length: prep.lengths[i],
                dir,
                closed: e.closed,
                a: cs.first().map_or(Vec2::new(0.0, 0.0), |c| c.p),
                b: cs.last().map_or(Vec2::new(0.0, 0.0), |c| c.p),
                nodes: (!cs.is_empty()).then(|| (node_of[order], node_of[order + cs.len() - 1])),
                inside: prep.levels[i][2],
            });
            let carried =
                reuse.and_then(|r| (r.carry[i] != u32::MAX).then(|| (r, r.carry[i] as usize)));
            match carried {
                Some((r, o)) => {
                    for &(_, c) in
                        &r.unread[r.unread_start[o] as usize..r.unread_start[o + 1] as usize]
                    {
                        report.unread.push((c as usize, e.id));
                        unread_at.push((i as u32, c));
                    }
                }
                None => {
                    for (c, rate) in rates.iter_mut().enumerate().skip(1) {
                        let (r, unread) = rules.rate(
                            c,
                            e.costs.get(c - 1).and_then(|v| v.as_deref()),
                            prep.lengths[i],
                        );
                        *rate = r;
                        if unread {
                            report.unread.push((c, e.id));
                            unread_at.push((i as u32, c as u32));
                        }
                    }
                }
            }
            let mut next_old = carried.map_or(0, |(r, o)| r.piece_start[o] as usize);
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
                if let Some((r, _)) = carried {
                    // The same piece as the last graph's: its primitives and costs as they were.
                    let op = &r.old.pieces[next_old];
                    geom.extend_from_slice(&r.old.geom[op.g0 as usize..op.g1 as usize]);
                    geom_at.extend_from_slice(&r.old.geom_at[op.g0 as usize..op.g1 as usize]);
                    let (old_fw, old_bw) = r.old.defined_costs();
                    fw.extend_from_slice(&old_fw[next_old * costs..(next_old + 1) * costs]);
                    bw.extend_from_slice(&old_bw[next_old * costs..(next_old + 1) * costs]);
                    next_old += 1;
                    pieces.push(Piece {
                        from: na,
                        to: nb,
                        edge: i as u32,
                        s0: a.s,
                        s1: b.s,
                        len,
                        g0,
                        g1: geom.len() as u32,
                    });
                    continue;
                }
                span_of(&e.path, prep.cum(i), a.s, b.s, &mut geom, &mut geom_at);
                let g1 = geom.len() as u32;
                for rate in &rates {
                    let cost = rate.map_or(f64::INFINITY, |r| r * len);
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
            if prep.lengths[i] <= 0.0 && !e.path.is_empty() {
                report
                    .short
                    .push((e.id, cs.first().map_or(Vec2::new(0.0, 0.0), |c| c.p)));
            }
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
                    (
                        1,
                        p.to,
                        p.s1 == prep.lengths[p.edge as usize],
                        &e.terminals[1],
                    ),
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
            closing: Vec::new(),
            gap_start: gap_start.clone(),
            gap_to: gap_to.clone(),
            part: Vec::new(),
            part_edges: Vec::new(),
            kept: None,
        };
        g.find_parts();
        if g.rules.turns.is_some() {
            g.turns = Some(g.turn_table(turns, &mut report));
        }
        (g, report, unread_at)
    }
}
