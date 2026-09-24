//! Planar arrangement of straight and circular edges: steps 1–3 of the
//! overlay (`apps/web/src/model/geom/arrangement.ts`, see overlay.rs).
//!
//!   1. Every edge is cut where it meets another (crossings, touches,
//!      overlaps), giving pieces that only meet at their ends.
//!   2. Pieces lying on top of each other (shared parcel boundaries) are
//!      merged into one, remembering which source ran along it and in which
//!      direction.
//!   3. A rule decides, from the winding numbers of the area sources on
//!      either side, whether a piece bounds the result.

use std::collections::HashMap;

use crate::geom::intersect::{Edge, closest_on_edge, intersect_edges, on_edge_arc, point_at};
use crate::geometry::Bounds;
use crate::jsmath::{
    PI, TAU, atan2, cos, js_cmp, js_hypot, js_max, js_max_all, js_min, js_min_all, js_round,
    js_sign, sin, stable_sort,
};
use crate::vec2::Vec2;

/// Closed boundary as vertices with DXF bulges (same form as a polygon entity).
#[derive(Clone, Debug, PartialEq)]
pub struct Ring {
    pub pts: Vec<Vec2>,
    pub bulges: Option<Vec<f64>>,
}

crate::json_struct!(Ring { pts, bulges });

/// One area: a counter-clockwise outer ring and clockwise holes.
#[derive(Clone, Debug, PartialEq)]
pub struct Area {
    pub outer: Ring,
    pub holes: Vec<Ring>,
}

crate::json_struct!(Area { outer, holes });

/// Overlay input: an area's oriented boundary (interior on the left) or cut lines.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    pub edges: Vec<Edge>,
    /// Exact input vertices: edge ends near them take these coordinates, so
    /// the output repeats input corners bit for bit.
    pub points: Option<Vec<Vec2>>,
    /// Cut lines take no part in inside tests; a cut inside the result splits it.
    pub cut: Option<bool>,
}

crate::json_struct!(Source { edges, points, cut });

impl Source {
    pub fn is_cut(&self) -> bool {
        self.cut == Some(true)
    }
}

/// Which pieces bound the result, from "inside source k" on one side
/// (the TypeScript passed a function; these are all the ones it used).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Inside any source (union).
    Any,
    /// Inside every source (intersection).
    All,
    /// Inside an odd number of sources (exclusion).
    Odd,
    /// Inside the first and no other (difference).
    FirstNotOthers,
    /// Inside the first (splitting by cut lines).
    First,
    /// Everything (the faces of line work).
    Always,
}

impl Rule {
    pub fn apply(self, inside: &[bool]) -> bool {
        match self {
            Rule::Any => inside.iter().any(|&b| b),
            Rule::All => inside.iter().all(|&b| b),
            Rule::Odd => inside.iter().filter(|&&b| b).count() % 2 == 1,
            Rule::FirstNotOthers => {
                inside.first().copied().unwrap_or(false) && !inside.iter().skip(1).any(|&b| b)
            }
            Rule::First => inside.first().copied().unwrap_or(false),
            Rule::Always => true,
        }
    }

    pub fn named(name: &str) -> Result<Rule, String> {
        Ok(match name {
            "any" => Rule::Any,
            "all" => Rule::All,
            "odd" => Rule::Odd,
            "firstNotOthers" => Rule::FirstNotOthers,
            "first" => Rule::First,
            "always" => Rule::Always,
            other => return Err(format!("Bilinmeyen bindirme kuralı “{other}”.")),
        })
    }
}

/// Points closer than this (metres) are one vertex.
pub const TOL: f64 = 1e-6;

// ── Vertices ──────────────────────────────────────────────────────────

/// Vertex clusters on a hash grid; input vertices win as representatives.
#[derive(Default)]
pub struct Vertices {
    pub pos: Vec<Vec2>,
    /// Exact input coordinates (absolute) for vertices that came from input.
    pub input: Vec<Option<Vec2>>,
    grid: HashMap<(i64, i64), Vec<usize>>,
}

impl Vertices {
    pub fn add(&mut self, p: Vec2, input: Option<Vec2>) -> usize {
        let gx = (p.x / (2.0 * TOL)).floor() as i64;
        let gy = (p.y / (2.0 * TOL)).floor() as i64;
        for i in gx - 1..=gx + 1 {
            for j in gy - 1..=gy + 1 {
                let Some(cell) = self.grid.get(&(i, j)) else {
                    continue;
                };
                for &id in cell {
                    let q = self.pos[id];
                    if (q.x - p.x).abs() <= TOL && (q.y - p.y).abs() <= TOL {
                        if input.is_some() && self.input[id].is_none() {
                            self.pos[id] = p;
                            self.input[id] = input;
                        }
                        return id;
                    }
                }
            }
        }
        let id = self.pos.len();
        self.pos.push(p);
        self.input.push(input);
        self.grid.entry((gx, gy)).or_default().push(id);
        id
    }
}

// ── Edge helpers ──────────────────────────────────────────────────────

pub fn edge_len(e: &Edge) -> f64 {
    match *e {
        Edge::Seg { a, b } => js_hypot(b.x - a.x, b.y - a.y),
        Edge::Arc { r, sweep, .. } => r * sweep.abs(),
    }
}

pub fn sub_edge(e: &Edge, t0: f64, t1: f64) -> Edge {
    match *e {
        Edge::Seg { .. } => Edge::Seg {
            a: point_at(e, t0),
            b: point_at(e, t1),
        },
        Edge::Arc { c, r, a0, sweep } => Edge::Arc {
            c,
            r,
            a0: a0 + sweep * t0,
            sweep: sweep * (t1 - t0),
        },
    }
}

pub fn reverse_edge(e: &Edge) -> Edge {
    match *e {
        Edge::Seg { a, b } => Edge::Seg { a: b, b: a },
        Edge::Arc { c, r, a0, sweep } => Edge::Arc {
            c,
            r,
            a0: a0 + sweep,
            sweep: -sweep,
        },
    }
}

pub fn translate_edge(e: &Edge, dx: f64, dy: f64) -> Edge {
    match *e {
        Edge::Seg { a, b } => Edge::Seg {
            a: Vec2::new(a.x + dx, a.y + dy),
            b: Vec2::new(b.x + dx, b.y + dy),
        },
        Edge::Arc { c, r, a0, sweep } => Edge::Arc {
            c: Vec2::new(c.x + dx, c.y + dy),
            r,
            a0,
            sweep,
        },
    }
}

/// Full circles become two halves: a piece must run between two distinct points.
fn split_full_circles(edges: &[Edge]) -> Vec<Edge> {
    let mut out = Vec::with_capacity(edges.len());
    for e in edges {
        match *e {
            Edge::Arc { sweep, .. } if sweep.abs() >= TAU - 1e-9 => {
                out.push(sub_edge(e, 0.0, 0.5));
                out.push(sub_edge(e, 0.5, 1.0));
            }
            _ => out.push(*e),
        }
    }
    out
}

pub fn edge_box(e: &Edge) -> Bounds {
    match *e {
        Edge::Seg { a, b } => Bounds {
            min_x: js_min(a.x, b.x),
            min_y: js_min(a.y, b.y),
            max_x: js_max(a.x, b.x),
            max_y: js_max(a.y, b.y),
        },
        Edge::Arc { c, r, .. } => Bounds {
            min_x: c.x - r,
            min_y: c.y - r,
            max_x: c.x + r,
            max_y: c.y + r,
        },
    }
}

/// The box of the edge itself: an arc's ends and the extreme points it passes.
pub fn tight_box(e: &Edge) -> Bounds {
    let (c, r, a0, sweep) = match *e {
        Edge::Seg { .. } => return edge_box(e),
        Edge::Arc { c, r, a0, sweep } => (c, r, a0, sweep),
    };
    if sweep.abs() >= TAU - 1e-12 {
        return edge_box(e);
    }
    let a = point_at(e, 0.0);
    let b = point_at(e, 1.0);
    let mut bx = Bounds {
        min_x: js_min(a.x, b.x),
        min_y: js_min(a.y, b.y),
        max_x: js_max(a.x, b.x),
        max_y: js_max(a.y, b.y),
    };
    for k in 0..4 {
        let theta = (k as f64 * PI) / 2.0;
        if !on_edge_arc(a0, sweep, theta) {
            continue;
        }
        let x = c.x + r * cos(theta);
        let y = c.y + r * sin(theta);
        bx.min_x = js_min(bx.min_x, x);
        bx.max_x = js_max(bx.max_x, x);
        bx.min_y = js_min(bx.min_y, y);
        bx.max_y = js_max(bx.max_y, y);
    }
    bx
}

/// Direction leaving `e` at its start (or arriving back along it from its end), bent by curvature.
pub fn leave_angle(e: &Edge, at_end: bool) -> f64 {
    let from = point_at(e, if at_end { 1.0 } else { 0.0 });
    // A point a little along the edge: the chord carries the curvature.
    let q = point_at(e, if at_end { 1.0 - 1e-4 } else { 1e-4 });
    atan2(q.y - from.y, q.x - from.x)
}

fn subtended(a: Vec2, b: Vec2, p: Vec2) -> f64 {
    let ax = a.x - p.x;
    let ay = a.y - p.y;
    let bx = b.x - p.x;
    let by = b.y - p.y;
    atan2(ax * by - ay * bx, ax * bx + ay * by)
}

/// Winding number of the closed edge set around `p` (angle summation). An
/// arc subtends its chord's angle plus a full turn when `p` lies in the
/// circular segment between chord and arc.
pub fn winding(edges: &[Edge], p: Vec2) -> f64 {
    let mut total = 0.0;
    for e in edges {
        let (c, r, sweep) = match *e {
            Edge::Seg { a, b } => {
                total += subtended(a, b, p);
                continue;
            }
            Edge::Arc { c, r, sweep, .. } => (c, r, sweep),
        };
        let a = point_at(e, 0.0);
        let b = point_at(e, 1.0);
        let in_disk = js_hypot(p.x - c.x, p.y - c.y) < r;
        if sweep.abs() >= TAU - 1e-12 {
            if in_disk {
                total += js_sign(sweep) * TAU;
            }
            continue;
        }
        total += subtended(a, b, p);
        if !in_disk {
            continue;
        }
        // A counter-clockwise arc bulges to the right of its chord a→b.
        let side = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
        if if sweep > 0.0 { side < 0.0 } else { side > 0.0 } {
            total += js_sign(sweep) * TAU;
        }
    }
    js_round(total / TAU)
}

// The ray of WindingIndex: a direction no drawing lines up with; u runs across it.
// V8's Math.cos/Math.sin of 0.4835389, bit for bit (libm may differ in the last bit).
const RAY_X: f64 = f64::from_bits(0x3fec54d463c913c8);
const RAY_Y: f64 = f64::from_bits(0x3fddc12bf342002b);

fn u_of(x: f64, y: f64) -> f64 {
    -RAY_Y * x + RAY_X * y
}

/// Winding numbers of many points against one closed edge set, counted as
/// signed crossings of a ray in a fixed generic direction, with the edges
/// bucketed in bands across it. Same result as `winding` off the edges.
pub struct WindingIndex<'a> {
    edges: &'a [Edge],
    bands: HashMap<i64, Vec<usize>>,
    u0: f64,
    h: f64,
}

impl<'a> WindingIndex<'a> {
    pub fn new(edges: &'a [Edge]) -> WindingIndex<'a> {
        let ranges: Vec<(f64, f64)> = edges
            .iter()
            .map(|e| {
                let b = edge_box(e);
                let us = [
                    u_of(b.min_x, b.min_y),
                    u_of(b.max_x, b.min_y),
                    u_of(b.min_x, b.max_y),
                    u_of(b.max_x, b.max_y),
                ];
                (js_min_all(us), js_max_all(us))
            })
            .collect();
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for &(a, b) in &ranges {
            lo = js_min(lo, a);
            hi = js_max(hi, b);
        }
        let u0 = if lo.is_finite() { lo } else { 0.0 };
        let count = js_max(1.0, js_min(4096.0, (edges.len() as f64 / 4.0).ceil()));
        let h = js_max((hi - lo) / count, 1e-12);
        let mut bands: HashMap<i64, Vec<usize>> = HashMap::new();
        for (i, &(a, b)) in ranges.iter().enumerate() {
            let k1 = ((b - u0) / h).floor();
            let mut k = ((a - u0) / h).floor();
            while k <= k1 {
                bands.entry(k as i64).or_default().push(i);
                k += 1.0;
            }
        }
        WindingIndex {
            edges,
            bands,
            u0,
            h,
        }
    }

    pub fn winding(&self, p: Vec2) -> f64 {
        let uq = u_of(p.x, p.y);
        let Some(list) = self.bands.get(&(((uq - self.u0) / self.h).floor() as i64)) else {
            return 0.0;
        };
        let mut w = 0.0;
        // Segments share ends safely (half-open in u); an arc's end on the ray could be
        // counted twice with its neighbour, so that case falls back to the exact angle sum.
        let mut degenerate = false;
        for &i in list {
            let e = &self.edges[i];
            let (c, r, a0s, sweep) = match *e {
                Edge::Seg { a, b } => {
                    let ua = u_of(a.x, a.y);
                    let ub = u_of(b.x, b.y);
                    // Half-open in u, so a ray through a shared end counts once.
                    let up = ua <= uq && uq < ub;
                    if !(up || (ub <= uq && uq < ua)) {
                        continue;
                    }
                    let s = (uq - ua) / (ub - ua);
                    let x = a.x + (b.x - a.x) * s;
                    let y = a.y + (b.y - a.y) * s;
                    if (x - p.x) * RAY_X + (y - p.y) * RAY_Y > 0.0 {
                        w += if up { 1.0 } else { -1.0 };
                    }
                    continue;
                }
                Edge::Arc { c, r, a0, sweep } => (c, r, a0, sweep),
            };
            let dx = p.x - c.x;
            let dy = p.y - c.y;
            let b = dx * RAY_X + dy * RAY_Y;
            let disc = b * b - (dx * dx + dy * dy - r * r);
            if disc <= 0.0 {
                continue;
            }
            let sq = disc.sqrt();
            let full = sweep.abs() >= TAU - 1e-12;
            for t in [-b - sq, -b + sq] {
                if t <= 0.0 {
                    continue;
                }
                let theta = atan2(dy + t * RAY_Y, dx + t * RAY_X);
                if !full && !on_edge_arc(a0s, sweep, theta) {
                    continue;
                }
                if !full {
                    let hit = Vec2::new(p.x + t * RAY_X, p.y + t * RAY_Y);
                    let tol = 1e-9 * js_max(1.0, r);
                    let s0 = point_at(e, 0.0);
                    let s1 = point_at(e, 1.0);
                    if js_hypot(hit.x - s0.x, hit.y - s0.y) < tol
                        || js_hypot(hit.x - s1.x, hit.y - s1.y) < tol
                    {
                        degenerate = true;
                    }
                }
                // Travel direction at θ, across the ray (its u component).
                let du = (-sin(theta) * -RAY_Y + cos(theta) * RAY_X) * js_sign(sweep);
                w += if du > 0.0 { 1.0 } else { -1.0 };
            }
        }
        if degenerate {
            winding(self.edges, p)
        } else {
            w
        }
    }
}

// ── Pieces ────────────────────────────────────────────────────────────

/// A maximal piece of the overlay: edges of several sources may run along it.
pub struct Piece {
    pub edge: Edge,
    pub from: usize,
    pub to: usize,
    /// Net count of area-source edges running along the piece, per source
    /// (+1 same direction, −1 opposite).
    pub delta: Vec<f64>,
    pub cut: bool,
}

/// A piece used in one direction by the result.
#[derive(Clone, Copy, Debug)]
pub struct DirPiece {
    pub piece: usize,
    pub fwd: bool,
    pub from: usize,
    pub to: usize,
    pub edge: Edge,
}

pub struct Built {
    pub verts: Vertices,
    pub pieces: Vec<Piece>,
}

struct Placed {
    e: Edge,
    src: usize,
    bx: Bounds,
}

/// Steps 1 and 2: cut edges at every meeting point and merge coincident
/// pieces. `local` is the input moved near the origin, `abs` the same input
/// in true coordinates (for the exact vertices).
pub fn build(local: &[Source], abs: &[Source], o: Vec2) -> Built {
    let mut verts = Vertices::default();
    for s in abs {
        for &p in s.points.iter().flatten() {
            verts.add(Vec2::new(p.x - o.x, p.y - o.y), Some(p));
        }
    }
    let mut edges: Vec<Placed> = Vec::new();
    for (src, s) in local.iter().enumerate() {
        let abs_edges = split_full_circles(&abs[src].edges);
        for (k, e) in split_full_circles(&s.edges).into_iter().enumerate() {
            if edge_len(&e) <= TOL {
                continue;
            }
            edges.push(Placed {
                e,
                src,
                bx: edge_box(&e),
            });
            // Edge ends are input corners too (a line's end is a real point).
            let ae = abs_edges.get(k).copied().unwrap_or(e);
            verts.add(point_at(&e, 0.0), Some(point_at(&ae, 0.0)));
            verts.add(point_at(&e, 1.0), Some(point_at(&ae, 1.0)));
        }
    }
    let mut cuts: Vec<Vec<f64>> = vec![Vec::new(); edges.len()];
    let add_cut = |cuts: &mut Vec<Vec<f64>>, i: usize, t: f64| {
        let len = edge_len(&edges[i].e);
        // Meeting points at an end are just shared vertices, not cuts.
        if t * len > TOL && (1.0 - t) * len > TOL {
            cuts[i].push(t);
        }
    };
    // Sweep over x so only edges with overlapping boxes are tested.
    let mut order: Vec<usize> = (0..edges.len()).collect();
    stable_sort(&mut order, &mut |&i, &j| {
        js_cmp(edges[i].bx.min_x - edges[j].bx.min_x, 0.0)
    });
    for oi in 0..order.len() {
        let i = order[oi];
        let bi = edges[i].bx;
        for &j in &order[oi + 1..] {
            let bj = edges[j].bx;
            if bj.min_x > bi.max_x + TOL {
                break;
            }
            if bj.min_y > bi.max_y + TOL || bj.max_y < bi.min_y - TOL {
                continue;
            }
            let ei = edges[i].e;
            let ej = edges[j].e;
            for h in intersect_edges(&ei, &ej) {
                add_cut(&mut cuts, i, h.t);
                add_cut(&mut cuts, j, h.u);
            }
            // Touching ends and overlaps (parallel lines, same circle) meet at an end point.
            for (x, other) in [(i, ej), (j, ei)] {
                for q in [point_at(&other, 0.0), point_at(&other, 1.0)] {
                    let c = closest_on_edge(&edges[x].e, q);
                    if c.d <= TOL {
                        add_cut(&mut cuts, x, c.t);
                    }
                }
            }
        }
    }

    let nsrc = local.len();
    let mut pieces: Vec<Piece> = Vec::new();
    let mut index: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (i, placed) in edges.iter().enumerate() {
        let e = placed.e;
        let src = placed.src;
        let mut ts = Vec::with_capacity(cuts[i].len() + 2);
        ts.push(0.0);
        let mut sorted = std::mem::take(&mut cuts[i]);
        stable_sort(&mut sorted, &mut |a, b| js_cmp(*a - *b, 0.0));
        ts.extend(sorted);
        ts.push(1.0);
        let mut last = 0.0;
        for k in 1..ts.len() {
            let t = ts[k];
            if k < ts.len() - 1 && (t - last) * edge_len(&e) <= TOL {
                continue;
            }
            let sub = sub_edge(&e, last, t);
            last = t;
            let from = verts.add(point_at(&sub, 0.0), None);
            let to = verts.add(point_at(&sub, 1.0), None);
            if from == to {
                continue;
            }
            let mid = point_at(&sub, 0.5);
            let key = if from < to { (from, to) } else { (to, from) };
            let same = index.get(&key).and_then(|list| {
                list.iter().copied().find(|&pi| {
                    let m = point_at(&pieces[pi].edge, 0.5);
                    js_hypot(m.x - mid.x, m.y - mid.y) <= 10.0 * TOL
                })
            });
            let cut = local[src].is_cut();
            match same {
                None => {
                    let mut delta = vec![0.0; nsrc];
                    if !cut {
                        delta[src] = 1.0;
                    }
                    index.entry(key).or_default().push(pieces.len());
                    pieces.push(Piece {
                        edge: sub,
                        from,
                        to,
                        delta,
                        cut,
                    });
                }
                Some(pi) => {
                    let p = &mut pieces[pi];
                    p.cut = p.cut || cut;
                    if !cut {
                        p.delta[src] += if p.from == from { 1.0 } else { -1.0 };
                    }
                }
            }
        }
    }
    Built { verts, pieces }
}

/// Boxes in square cells, for "what is near this point" without looking at every piece.
struct PieceGrid {
    cells: HashMap<(i64, i64), Vec<usize>>,
    big: Vec<usize>,
    size: f64,
    count: usize,
}

impl PieceGrid {
    fn new(boxes: &[Bounds]) -> PieceGrid {
        let mut sum = 0.0;
        for b in boxes {
            sum += js_max(b.max_x - b.min_x, b.max_y - b.min_y);
        }
        let size = js_max((sum / js_max(1.0, boxes.len() as f64)) * 2.0, 1e-9);
        let mut g = PieceGrid {
            cells: HashMap::new(),
            big: Vec::new(),
            size,
            count: boxes.len(),
        };
        for (i, b) in boxes.iter().enumerate() {
            let x0 = (b.min_x / size).floor();
            let x1 = (b.max_x / size).floor();
            let y0 = (b.min_y / size).floor();
            let y1 = (b.max_y / size).floor();
            // A huge box (a big circle) goes into the overflow list instead of thousands of cells.
            if (x1 - x0 + 1.0) * (y1 - y0 + 1.0) > 256.0 {
                g.big.push(i);
                continue;
            }
            for x in x0 as i64..=x1 as i64 {
                for y in y0 as i64..=y1 as i64 {
                    g.cells.entry((x, y)).or_default().push(i);
                }
            }
        }
        g
    }

    /// Calls `f` with every piece whose cells meet the square of radius r around p (some more than once).
    fn near(&self, p: Vec2, r: f64, f: &mut dyn FnMut(usize)) {
        let x0 = ((p.x - r) / self.size).floor();
        let x1 = ((p.x + r) / self.size).floor();
        let y0 = ((p.y - r) / self.size).floor();
        let y1 = ((p.y + r) / self.size).floor();
        if (x1 - x0 + 1.0) * (y1 - y0 + 1.0) > self.count as f64 {
            for i in 0..self.count {
                f(i);
            }
            return;
        }
        for x in x0 as i64..=x1 as i64 {
            for y in y0 as i64..=y1 as i64 {
                for &i in self.cells.get(&(x, y)).into_iter().flatten() {
                    f(i);
                }
            }
        }
        for &i in &self.big {
            f(i);
        }
    }
}

fn dir(p: &Piece, index: usize, fwd: bool) -> DirPiece {
    if fwd {
        DirPiece {
            piece: index,
            fwd,
            from: p.from,
            to: p.to,
            edge: p.edge,
        }
    } else {
        DirPiece {
            piece: index,
            fwd,
            from: p.to,
            to: p.from,
            edge: reverse_edge(&p.edge),
        }
    }
}

/// Step 3: keeps the pieces where the rule differs across them, oriented
/// with the rule's inside on the left. A cut piece with the inside on both
/// sides is kept both ways, so it splits the result.
pub fn classify(sources: &[Source], built: &Built, rule: Rule) -> Vec<DirPiece> {
    let pieces = &built.pieces;
    let mut out = Vec::new();
    if sources.iter().all(Source::is_cut) {
        // Line work only: every piece bounds a face on both sides.
        if rule.apply(&vec![false; sources.len()]) {
            for (i, p) in pieces.iter().enumerate() {
                out.push(dir(p, i, true));
                out.push(dir(p, i, false));
            }
        }
        return out;
    }
    let boxes: Vec<Bounds> = pieces.iter().map(|p| tight_box(&p.edge)).collect();
    let grid = PieceGrid::new(&boxes);
    let windings: Vec<Option<WindingIndex>> = sources
        .iter()
        .map(|s| {
            if s.is_cut() {
                None
            } else {
                Some(WindingIndex::new(&s.edges))
            }
        })
        .collect();
    let mut left = Vec::with_capacity(sources.len());
    let mut right = Vec::with_capacity(sources.len());
    for (index, piece) in pieces.iter().enumerate() {
        let e = &piece.edge;
        let len = edge_len(e);
        let m = point_at(e, 0.5);
        // Left normal at the middle.
        let (tx, ty) = match *e {
            Edge::Seg { a, b } => ((b.x - a.x) / len, (b.y - a.y) / len),
            Edge::Arc { a0, sweep, .. } => {
                let a = a0 + sweep / 2.0;
                let s = js_sign(sweep);
                (-sin(a) * s, cos(a) * s)
            }
        };
        // Step off the piece by less than the distance to anything else.
        let mut near = len;
        grid.near(m, len, &mut |k| {
            if k == index {
                return;
            }
            let bx = &boxes[k];
            if bx.min_x > m.x + near
                || bx.max_x < m.x - near
                || bx.min_y > m.y + near
                || bx.max_y < m.y - near
            {
                return;
            }
            near = js_min(near, closest_on_edge(&pieces[k].edge, m).d);
        });
        let eps = js_max(near * 0.25, 1e-9);
        let q = Vec2::new(m.x - ty * eps, m.y + tx * eps);
        left.clear();
        right.clear();
        for (k, w) in windings.iter().enumerate() {
            match w {
                None => {
                    left.push(false);
                    right.push(false);
                }
                Some(w) => {
                    let wn = w.winding(q);
                    left.push(wn != 0.0);
                    right.push(wn - piece.delta[k] != 0.0);
                }
            }
        }
        let l = rule.apply(&left);
        let r = rule.apply(&right);
        if l && !r {
            out.push(dir(piece, index, true));
        } else if r && !l {
            out.push(dir(piece, index, false));
        } else if l && r && piece.cut {
            out.push(dir(piece, index, true));
            out.push(dir(piece, index, false));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    const E: f64 = 486512.34;
    const N: f64 = 4420187.52;

    /// A ring with straight and arc edges, clockwise and counter-clockwise parts, at TM coordinates.
    fn ring() -> Vec<Edge> {
        let v = Vec2::new;
        let arc = |c, r, a0, sweep| Edge::Arc { c, r, a0, sweep };
        vec![
            Edge::Seg {
                a: v(E, N),
                b: v(E + 20.0, N),
            },
            arc(v(E + 20.0, N + 10.0), 10.0, -PI / 2.0, PI),
            Edge::Seg {
                a: v(E + 20.0, N + 20.0),
                b: v(E + 10.0, N + 20.0),
            },
            arc(v(E + 5.0, N + 20.0), 5.0, 0.0, -PI),
            Edge::Seg {
                a: v(E, N + 20.0),
                b: v(E, N),
            },
        ]
    }

    #[test]
    fn the_index_gives_the_winding_numbers_of_the_angle_sum() {
        let edges = ring();
        let index = WindingIndex::new(&edges);
        let mut seed: u64 = 7;
        let mut rnd = || {
            seed = seed * 16807 % 2147483647;
            seed as f64 / 2147483647.0
        };
        for _ in 0..400 {
            let p = Vec2::new(E - 5.0 + rnd() * 40.0, N - 5.0 + rnd() * 30.0);
            // `+ 0.0` folds the -0 the angle sum can round to.
            assert_eq!(index.winding(p) + 0.0, winding(&edges, p) + 0.0, "{p:?}");
        }
    }

    #[test]
    fn a_ray_through_an_arc_end_falls_back_to_the_angle_sum() {
        let edges = ring();
        let index = WindingIndex::new(&edges);
        // A point placed so the fixed ray runs straight through the start of the first arc.
        let end = Vec2::new(E + 20.0, N);
        let p = Vec2::new(end.x - RAY_X * 3.0, end.y - RAY_Y * 3.0);
        assert_eq!(index.winding(p) + 0.0, winding(&edges, p) + 0.0);
    }
}
