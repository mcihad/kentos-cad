//! Path booleans on the app's plane overlay engine
//! (`apps/web/src/style/svg/pathBool.ts`, `kentos_geometry_core::geom::overlay`).
//!
//! Curves are flattened to chords within a tolerance (a ten-thousandth of
//! the drawing's size) and every chord remembers the input segment and the
//! parameter range it came from. The overlay keeps input vertices bit for
//! bit, so each edge of a result ring lies on one input chord; runs of
//! edges on the same input segment are turned back into the exact part of
//! that Bézier (de Casteljau). **Curves are kept**: only the new corners
//! where outlines cross move to the crossing, within the tolerance. Edges
//! that no input owns (offsets, stroke outlines) are fitted with cubics,
//! and arcs of round joins become cubics.
//!
//! Fill rules: a nonzero shape is one overlay source as drawn (the overlay's
//! inside test is nonzero winding). An even-odd shape whose rings do not
//! cross is re-oriented by nesting depth; one whose rings cross is split
//! into the faces of its line work and the faces with odd winding kept.
//!
//! The TypeScript called the geometry core through JSON, which writes −0 as
//! 0: the arguments of those calls are passed here with −0 made 0 (`z`),
//! so every decision sees the numbers it saw.

use std::collections::{HashMap, HashSet};

use kentos_geometry_core::geom::arrangement::{Area, Ring, Rule, Source};
use kentos_geometry_core::geom::bulge::bulge_arc;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::overlay::{face_rings, overlay};
use kentos_geometry_core::geom::region::{area_source, inside_area};
use kentos_geometry_core::jsmath::{
    PI, cos, js_cmp, js_floor, js_max, js_min, sin, stable_sort, tan, truthy,
};
use kentos_geometry_core::vec2::Vec2;

use crate::bezier::{
    Cubic, bez, flatten_cubic, ring_signed_area, segment_count, segment_cubic, segment_is_line,
    sub_cubic, winding_of,
};
use crate::fit::{FitOptions, dist_to_segment, fit_polyline};
use crate::shape::{PathNode, Pt, SubPath};

/// A number as it crosses to the geometry core in JSON (−0 is written 0).
pub fn z(x: f64) -> f64 {
    if x == 0.0 { 0.0 } else { x }
}

/// A point as the geometry core reads it.
pub fn zv(p: Pt) -> Vec2 {
    Vec2::new(z(p[0]), z(p[1]))
}

fn zvec(p: Vec2) -> Vec2 {
    Vec2::new(z(p.x), z(p.y))
}

/// An edge as it crossed to the geometry core.
pub fn z_edge(e: &Edge) -> Edge {
    match *e {
        Edge::Seg { a, b } => Edge::Seg {
            a: zvec(a),
            b: zvec(b),
        },
        Edge::Arc { c, r, a0, sweep } => Edge::Arc {
            c: zvec(c),
            r: z(r),
            a0: z(a0),
            sweep: z(sweep),
        },
    }
}

pub fn z_ring(r: &Ring) -> Ring {
    Ring {
        pts: r.pts.iter().map(|&p| zvec(p)).collect(),
        bulges: r.bulges.as_ref().map(|b| b.iter().map(|&x| z(x)).collect()),
    }
}

pub fn z_area(a: &Area) -> Area {
    Area {
        outer: z_ring(&a.outer),
        holes: a.holes.iter().map(z_ring).collect(),
    }
}

pub fn z_source(s: &Source) -> Source {
    Source {
        edges: s.edges.iter().map(z_edge).collect(),
        points: s
            .points
            .as_ref()
            .map(|ps| ps.iter().map(|&p| zvec(p)).collect()),
        cut: s.cut,
    }
}

/// `overlay(sources, rule)` as the TypeScript called it.
pub fn overlay_z(sources: &[Source], rule: Rule) -> Vec<Area> {
    let zs: Vec<Source> = sources.iter().map(z_source).collect();
    overlay(&zs, rule)
}

/// `areaSource(list)` as the TypeScript called it.
pub fn area_source_z(list: &[Area]) -> Source {
    let za: Vec<Area> = list.iter().map(z_area).collect();
    area_source(&za)
}

fn seg_edge(a: Pt, b: Pt) -> Edge {
    Edge::Seg { a: zv(a), b: zv(b) }
}

/// A shape's fill for the booleans: its sub-paths and whether it fills nonzero (else even-odd).
pub struct RegionInput {
    pub subs: Vec<SubPath>,
    pub nonzero: bool,
}

struct Owner {
    c: Cubic,
    line: bool,
}

struct Chord {
    a: Pt,
    b: Pt,
    t0: f64,
    t1: f64,
    owner: usize,
}

/// Hit of a result edge on an input chord: the input segment and the parameters of the edge's ends.
#[derive(Clone, Copy, Debug)]
struct Own {
    o: usize,
    ta: f64,
    tb: f64,
}

/// A number as a key the way `${x}` tells numbers apart (−0 is 0, NaN is one value).
fn num_key(x: f64) -> u64 {
    if x == 0.0 {
        0
    } else if x.is_nan() {
        f64::NAN.to_bits()
    } else {
        x.to_bits()
    }
}

fn cell_key(x: f64, y: f64) -> (u64, u64) {
    (num_key(x), num_key(y))
}

/// Flattens input sub-paths, remembering where each chord came from, and
/// finds a result edge's chord back (a grid of cells over the chords).
pub struct Tracer {
    pub tol: f64,
    cell: f64,
    owners: Vec<Owner>,
    chords: Vec<Chord>,
    grid: HashMap<(u64, u64), Vec<usize>>,
    /// Points flattened inside curves (not nodes), by exact coordinates.
    samples: HashSet<(u64, u64)>,
}

impl Tracer {
    pub fn new(tol: f64, extent: f64) -> Tracer {
        Tracer {
            tol,
            cell: js_max(js_max(extent / 48.0, tol * 4.0), 1e-9),
            owners: Vec::new(),
            chords: Vec::new(),
            grid: HashMap::new(),
            samples: HashSet::new(),
        }
    }

    fn add_chord(&mut self, a: Pt, b: Pt, t0: f64, t1: f64, owner: usize) {
        let id = self.chords.len();
        self.chords.push(Chord {
            a,
            b,
            t0,
            t1,
            owner,
        });
        let s = self.cell;
        let x0 = js_floor(js_min(a[0], b[0]) / s);
        let x1 = js_floor(js_max(a[0], b[0]) / s);
        let y0 = js_floor(js_min(a[1], b[1]) / s);
        let y1 = js_floor(js_max(a[1], b[1]) / s);
        let mut x = x0;
        while x <= x1 {
            let mut y = y0;
            while y <= y1 {
                self.grid.entry(cell_key(x, y)).or_default().push(id);
                y += 1.0;
            }
            x += 1.0;
        }
    }

    fn add_segment(&mut self, c: Cubic, line: bool, out: &mut Vec<Pt>) {
        let owner = self.owners.len();
        self.owners.push(Owner { c, line });
        if line {
            self.add_chord(c[0], c[3], 0.0, 1.0, owner);
            out.push(c[3]);
            return;
        }
        let (pts, ts) = flatten_cubic(&c, self.tol);
        for k in 1..pts.len() {
            self.add_chord(pts[k - 1], pts[k], ts[k - 1], ts[k], owner);
            out.push(pts[k]);
            if k < pts.len() - 1 {
                self.samples.insert(cell_key(pts[k][0], pts[k][1]));
            }
        }
    }

    /// Whether p is a point flattened inside a curve (the overlay keeps such points exactly).
    pub fn is_sample(&self, p: Pt) -> bool {
        self.samples.contains(&cell_key(p[0], p[1]))
    }

    /// A sub-path as a closed ring (an open one is closed by a straight chord,
    /// as SVG fills it). The ring does not repeat its first point.
    pub fn ring(&mut self, sp: &SubPath) -> Vec<Pt> {
        let mut out = Vec::new();
        if sp.nodes.is_empty() {
            return out;
        }
        let first = sp.nodes[0].pt();
        out.push(first);
        let n = segment_count(sp);
        for i in 0..n {
            self.add_segment(segment_cubic(sp, i), segment_is_line(sp, i), &mut out);
        }
        if !sp.closed && out.len() > 1 {
            let last = out[out.len() - 1];
            if last[0] != first[0] || last[1] != first[1] {
                self.add_segment([last, last, first, first], true, &mut out);
            }
        }
        let last = out[out.len() - 1];
        if out.len() > 1 && last[0] == first[0] && last[1] == first[1] {
            out.pop();
        }
        out
    }

    /// A sub-path as a polyline of cut lines (a closed one ends where it starts).
    pub fn line(&mut self, sp: &SubPath) -> Vec<Pt> {
        let mut out = Vec::new();
        if sp.nodes.is_empty() {
            return out;
        }
        out.push(sp.nodes[0].pt());
        let n = segment_count(sp);
        for i in 0..n {
            self.add_segment(segment_cubic(sp, i), segment_is_line(sp, i), &mut out);
        }
        out
    }

    /// The input chord a result edge p→q lies on, with the parameters of p and q.
    fn find(&self, p: Pt, q: Pt) -> Option<Own> {
        let m: Pt = [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0];
        // Overlay vertices merge within 1e-6; a little more covers the rounding.
        let eps = 4e-6 + self.tol * 1e-6;
        let s = self.cell;
        // The cells around the midpoint (a chord on a cell border may be filed on either side).
        let mut ids: Vec<usize> = Vec::new();
        let mut seen: HashSet<usize> = HashSet::new();
        let mut x = js_floor((m[0] - eps) / s);
        while x <= js_floor((m[0] + eps) / s) {
            let mut y = js_floor((m[1] - eps) / s);
            while y <= js_floor((m[1] + eps) / s) {
                if let Some(list) = self.grid.get(&cell_key(x, y)) {
                    for &id in list {
                        if seen.insert(id) {
                            ids.push(id);
                        }
                    }
                }
                y += 1.0;
            }
            x += 1.0;
        }
        let mut best: Option<Own> = None;
        let mut best_d = eps;
        for id in ids {
            let c = &self.chords[id];
            let d = js_max(
                js_max(dist_to_segment(p, c.a, c.b), dist_to_segment(q, c.a, c.b)),
                dist_to_segment(m, c.a, c.b),
            );
            if d > best_d {
                continue;
            }
            best_d = d;
            let at = |r: Pt| {
                let dx = c.b[0] - c.a[0];
                let dy = c.b[1] - c.a[1];
                let l2 = dx * dx + dy * dy;
                let u = if l2 > 0.0 {
                    js_max(
                        0.0,
                        js_min(1.0, ((r[0] - c.a[0]) * dx + (r[1] - c.a[1]) * dy) / l2),
                    )
                } else {
                    0.0
                };
                c.t0 + (c.t1 - c.t0) * u
            };
            best = Some(Own {
                o: c.owner,
                ta: at(p),
                tb: at(q),
            });
        }
        best
    }
}

// ── Regions ────────────────────────────────────────────────────────────

fn ring_edges(ring: &[Pt]) -> Vec<Edge> {
    let n = ring.len();
    (0..n)
        .map(|i| seg_edge(ring[i], ring[(i + 1) % n]))
        .collect()
}

/// Whether any two chords of the rings cross or touch (neighbours on one ring excepted).
pub fn rings_cross(rings: &[Vec<Pt>]) -> bool {
    #[derive(Clone)]
    struct Seg {
        a: Pt,
        b: Pt,
        r: usize,
        i: usize,
        n: usize,
        min_x: f64,
        max_x: f64,
        min_y: f64,
        max_y: f64,
    }
    let mut segs: Vec<Seg> = Vec::new();
    for (r, ring) in rings.iter().enumerate() {
        for (i, &a) in ring.iter().enumerate() {
            let b = ring[(i + 1) % ring.len()];
            segs.push(Seg {
                a,
                b,
                r,
                i,
                n: ring.len(),
                min_x: js_min(a[0], b[0]),
                max_x: js_max(a[0], b[0]),
                min_y: js_min(a[1], b[1]),
                max_y: js_max(a[1], b[1]),
            });
        }
    }
    stable_sort(&mut segs, &mut |s, t| js_cmp(s.min_x - t.min_x, 0.0));
    let eps = 1e-9;
    for x in 0..segs.len() {
        let s = &segs[x];
        for t in &segs[x + 1..] {
            if t.min_x > s.max_x + eps {
                break;
            }
            if t.min_y > s.max_y + eps || t.max_y < s.min_y - eps {
                continue;
            }
            let di = s.i.abs_diff(t.i);
            if s.r == t.r && (di == 1 || di + 1 == s.n) {
                continue;
            }
            if segments_meet(s.a, s.b, t.a, t.b, eps) {
                return true;
            }
        }
    }
    false
}

fn segments_meet(a: Pt, b: Pt, c: Pt, d: Pt, eps: f64) -> bool {
    let o = |p: Pt, q: Pt, r: Pt| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
    let d1 = o(c, d, a);
    let d2 = o(c, d, b);
    let d3 = o(a, b, c);
    let d4 = o(a, b, d);
    if ((d1 > eps && d2 < -eps) || (d1 < -eps && d2 > eps))
        && ((d3 > eps && d4 < -eps) || (d3 < -eps && d4 > eps))
    {
        return true;
    }
    // Touching or collinear overlaps.
    dist_to_segment(a, c, d) <= eps
        || dist_to_segment(b, c, d) <= eps
        || dist_to_segment(c, a, b) <= eps
        || dist_to_segment(d, a, b) <= eps
}

/// The fill of a shape as one overlay source (nonzero-clean), its rings flattened through the tracer.
pub fn region_source(input: &RegionInput, tr: &mut Tracer) -> Source {
    let rings: Vec<Vec<Pt>> = input
        .subs
        .iter()
        .map(|sp| tr.ring(sp))
        .filter(|r| r.len() >= 2)
        .collect();
    let points: Vec<Vec2> = rings.iter().flatten().map(|&p| zv(p)).collect();
    if input.nonzero {
        return Source {
            edges: rings.iter().flat_map(|r| ring_edges(r)).collect(),
            points: Some(points),
            cut: None,
        };
    }
    if !rings_cross(&rings) {
        // Nested rings alternate: even depth one way round, odd depth the other.
        let edges = rings
            .iter()
            .enumerate()
            .flat_map(|(i, ring)| {
                let depth = rings
                    .iter()
                    .enumerate()
                    .filter(|&(j, other)| j != i && winding_of(other, ring[0]) != 0)
                    .count();
                let pos = ring_signed_area(ring) > 0.0;
                if pos == (depth % 2 == 0) {
                    ring_edges(ring)
                } else {
                    let rev: Vec<Pt> = ring.iter().rev().copied().collect();
                    ring_edges(&rev)
                }
            })
            .collect();
        return Source {
            edges,
            points: Some(points),
            cut: None,
        };
    }
    // Crossing rings: faces of the line work, kept where the winding is odd.
    let rs = face_rings(&[Source {
        edges: rings.iter().flat_map(|r| ring_edges(r)).collect(),
        points: Some(points),
        cut: Some(true),
    }]);
    let in_box = |r: &kentos_geometry_core::geom::overlay::FaceRing, p: Vec2| {
        p.x >= r.bx.min_x && p.x <= r.bx.max_x && p.y >= r.bx.min_y && p.y <= r.bx.max_y
    };
    let mut faces: Vec<usize> = (0..rs.len()).filter(|&i| rs[i].area > 0.0).collect();
    stable_sort(&mut faces, &mut |&a, &b| {
        js_cmp(rs[a].area - rs[b].area, 0.0)
    });
    let groups: Vec<usize> = (0..rs.len()).filter(|&i| rs[i].area < 0.0).collect();
    let host: Vec<Option<usize>> = groups
        .iter()
        .map(|&g| {
            let probe = rs[g].probe;
            faces.iter().copied().find(|&f| {
                in_box(&rs[f], probe)
                    && inside_area(
                        &Area {
                            outer: z_ring(&rs[f].ring),
                            holes: Vec::new(),
                        },
                        zvec(probe),
                    )
            })
        })
        .collect();
    let mut kept: Vec<Area> = Vec::new();
    for &f in &faces {
        let p: Pt = [rs[f].probe.x, rs[f].probe.y];
        let w: i32 = rings.iter().map(|r| winding_of(r, p)).sum();
        if w.abs() % 2 == 1 {
            kept.push(Area {
                outer: rs[f].ring.clone(),
                holes: groups
                    .iter()
                    .zip(&host)
                    .filter(|&(_, h)| *h == Some(f))
                    .map(|(&g, _)| rs[g].ring.clone())
                    .collect(),
            });
        }
    }
    area_source_z(&kept)
}

// ── Back to sub-paths ──────────────────────────────────────────────────

struct REdge {
    p: Pt,
    q: Pt,
    bulge: f64,
    own: Option<Own>,
}

fn follows(a: &REdge, b: &REdge) -> bool {
    if truthy(a.bulge) || truthy(b.bulge) {
        return false;
    }
    let (ao, bo) = match (a.own, b.own) {
        (Some(ao), Some(bo)) => (ao, bo),
        (ao, bo) => return ao.is_none() && bo.is_none(),
    };
    if ao.o != bo.o || (ao.tb - bo.ta).abs() > 1e-6 {
        return false;
    }
    let da = ao.tb - ao.ta;
    let db = bo.tb - bo.ta;
    da == 0.0 || db == 0.0 || (da > 0.0) == (db > 0.0)
}

/// Cubic pieces of a circular arc edge (bulge form) from p to q.
fn arc_cubics(p: Pt, q: Pt, bulge: f64) -> Vec<Cubic> {
    let Some(arc) = bulge_arc(zv(p), zv(q), z(bulge)) else {
        return vec![[p, p, q, q]];
    };
    let n = js_max(1.0, (arc.sweep.abs() / (PI / 2.0) - 1e-9).ceil());
    let step = arc.sweep / n;
    let k = (4.0 / 3.0) * tan(step / 4.0);
    let at = |a: f64| -> Pt { [arc.c.x + arc.r * cos(a), arc.c.y + arc.r * sin(a)] };
    let tng = |a: f64| -> Pt { [-arc.r * sin(a), arc.r * cos(a)] };
    let mut out = Vec::new();
    let mut s = 0.0;
    while s < n {
        let a0 = arc.a0 + s * step;
        let a1 = a0 + step;
        let p0 = if s == 0.0 { p } else { at(a0) };
        let p3 = if s == n - 1.0 { q } else { at(a1) };
        let t0 = tng(a0);
        let t1 = tng(a1);
        out.push([
            p0,
            [p0[0] + k * t0[0], p0[1] + k * t0[1]],
            [p3[0] - k * t1[0], p3[1] - k * t1[1]],
            p3,
        ]);
        s += 1.0;
    }
    out
}

/// Builds a sub-path node by node, skipping zero-length steps.
pub struct Builder {
    pub nodes: Vec<PathNode>,
}

impl Builder {
    pub fn new() -> Builder {
        Builder { nodes: Vec::new() }
    }

    pub fn start(&mut self, p: Pt) {
        self.nodes.push(PathNode::at(p[0], p[1]));
    }

    pub fn line_to(&mut self, p: Pt) {
        let Some(l) = self.nodes.last() else { return };
        if kentos_geometry_core::jsmath::js_hypot(l.x - p[0], l.y - p[1]) < 1e-12 {
            return;
        }
        self.nodes.push(PathNode::at(p[0], p[1]));
    }

    pub fn curve_to(&mut self, c1: Pt, c2: Pt, p: Pt) {
        use kentos_geometry_core::jsmath::js_hypot;
        let Some(l) = self.nodes.last_mut() else {
            return;
        };
        if js_hypot(l.x - p[0], l.y - p[1]) < 1e-12 && js_hypot(c1[0] - p[0], c1[1] - p[1]) < 1e-12
        {
            return;
        }
        l.out = Some([c1[0], c1[1]]);
        self.nodes.push(PathNode {
            in_: Some([c2[0], c2[1]]),
            ..PathNode::at(p[0], p[1])
        });
    }

    /// Appends a fitted sub-path whose first node is the current end.
    pub fn append(&mut self, sp: &SubPath) {
        if let Some(first) = sp.nodes.first()
            && let Some(o) = first.out
            && let Some(l) = self.nodes.last_mut()
        {
            l.out = Some(o);
        }
        self.nodes.extend(sp.nodes.iter().skip(1).cloned());
    }

    pub fn close(mut self) -> SubPath {
        let n = self.nodes.len();
        if n > 1 {
            let (a, b) = (&self.nodes[0], &self.nodes[n - 1]);
            if kentos_geometry_core::jsmath::js_hypot(a.x - b.x, a.y - b.y) < 1e-9 {
                let b_in = b.in_;
                self.nodes.pop();
                self.nodes[0].in_ = b_in;
            }
        }
        SubPath {
            closed: true,
            nodes: self.nodes,
        }
    }
}

impl Default for Builder {
    fn default() -> Builder {
        Builder::new()
    }
}

fn plain(pts: &[Pt]) -> SubPath {
    SubPath {
        closed: true,
        nodes: pts.iter().map(|p| PathNode::at(p[0], p[1])).collect(),
    }
}

/// A result ring back as a sub-path: runs on one input segment become that
/// segment's exact part, arcs become cubics, everything else is fitted
/// within `fit_tol` (0 keeps it as lines).
pub fn ring_to_sub_path(ring: &Ring, tr: Option<&Tracer>, fit_tol: f64) -> SubPath {
    let pts: Vec<Pt> = ring.pts.iter().map(|p| [p.x, p.y]).collect();
    let n = pts.len();
    let edges: Vec<REdge> = (0..n)
        .map(|i| {
            let p = pts[i];
            let q = pts[(i + 1) % n];
            let bulge = ring
                .bulges
                .as_ref()
                .and_then(|b| b.get(i).copied())
                .unwrap_or(0.0);
            let own = match tr {
                Some(tr) if !truthy(bulge) => tr.find(p, q),
                _ => None,
            };
            REdge { p, q, bulge, own }
        })
        .collect();
    if n < 2 {
        return plain(&pts);
    }
    // A ring that is one free run all round is fitted as a smooth loop.
    if edges.iter().all(|e| e.own.is_none() && !truthy(e.bulge)) {
        return if fit_tol > 0.0 {
            fit_polyline(
                &pts,
                fit_tol,
                &FitOptions {
                    closed: true,
                    corners: &[],
                    corner_deg: 35.0,
                },
            )
        } else {
            plain(&pts)
        };
    }
    let found = (0..n).find(|&i| !follows(&edges[(i + n - 1) % n], &edges[i]));
    // One input segment all round (a single-node loop): break it in two.
    let forced = found.is_none();
    let start = found.unwrap_or(0);
    let mut runs: Vec<Vec<usize>> = Vec::new();
    for k in 0..n {
        let e = (start + k) % n;
        let joins = match runs.last() {
            Some(cur) => follows(&edges[cur[cur.len() - 1]], &edges[e]) && !(forced && k == n / 2),
            None => false,
        };
        match runs.last_mut() {
            Some(cur) if joins => cur.push(e),
            _ => runs.push(vec![e]),
        }
    }
    let mut b = Builder::new();
    b.start(edges[runs[0][0]].p);
    for run in &runs {
        let first = &edges[run[0]];
        let last = &edges[run[run.len() - 1]];
        let end = last.q;
        if truthy(first.bulge) {
            for c in arc_cubics(first.p, first.q, first.bulge) {
                b.curve_to(c[1], c[2], c[3]);
            }
        } else if let (Some(own), Some(tr)) = (first.own, tr) {
            let o = &tr.owners[own.o];
            if o.line {
                b.line_to(end);
            } else {
                let tb = last.own.map_or(f64::NAN, |l| l.tb);
                let s = sub_cubic(&o.c, own.ta, tb);
                // The crossing may sit a hair off the curve: carry the handles with the ends.
                let p0 = first.p;
                b.curve_to(
                    [s[1][0] + p0[0] - s[0][0], s[1][1] + p0[1] - s[0][1]],
                    [s[2][0] + end[0] - s[3][0], s[2][1] + end[1] - s[3][1]],
                    end,
                );
            }
        } else if fit_tol > 0.0 && run.len() > 1 {
            let mut line = vec![first.p];
            line.extend(run.iter().map(|&e| edges[e].q));
            b.append(&fit_polyline(
                &line,
                fit_tol,
                &FitOptions {
                    closed: false,
                    corners: &[],
                    corner_deg: 35.0,
                },
            ));
        } else {
            for &e in run {
                b.line_to(edges[e].q);
            }
        }
    }
    b.close()
}

pub fn areas_to_sub_paths(areas: &[Area], tr: Option<&Tracer>, fit_tol: f64) -> Vec<SubPath> {
    areas
        .iter()
        .flat_map(|a| std::iter::once(&a.outer).chain(a.holes.iter()))
        .map(|r| ring_to_sub_path(r, tr, fit_tol))
        .filter(|sp| sp.nodes.len() >= 2)
        .collect()
}

// ── Booleans ───────────────────────────────────────────────────────────

/// Size of everything involved (for tolerances).
pub fn extent_of(list: &[&[SubPath]]) -> f64 {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for subs in list {
        for sp in subs.iter() {
            for n in &sp.nodes {
                for p in [Some([n.x, n.y]), n.in_, n.out].into_iter().flatten() {
                    min_x = js_min(min_x, p[0]);
                    min_y = js_min(min_y, p[1]);
                    max_x = js_max(max_x, p[0]);
                    max_y = js_max(max_y, p[1]);
                }
            }
        }
    }
    if min_x.is_finite() {
        js_max(js_max(max_x - min_x, max_y - min_y), 1e-6)
    } else {
        1.0
    }
}

/// Flattening tolerance for a drawing of this size.
pub fn tol_for(extent: f64) -> f64 {
    js_max(extent * 1e-4, 1e-7)
}

/// A boolean of fill regions (Inkscape's Path menu).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoolOp {
    Union,
    Difference,
    Intersection,
    Exclusion,
    Division,
}

impl BoolOp {
    /// The operation by its TypeScript name (any other name takes the others away, as `difference`).
    pub fn named(s: &str) -> BoolOp {
        match s {
            "union" => BoolOp::Union,
            "intersection" => BoolOp::Intersection,
            "exclusion" => BoolOp::Exclusion,
            "division" => BoolOp::Division,
            _ => BoolOp::Difference,
        }
    }
}

/// A boolean of fill regions, bottom first. Union, intersection and
/// exclusion (odd count) use all inputs; difference takes the others away
/// from the first; division cuts the first along the others' outlines.
/// Every result is a list of sub-paths (division gives one per piece).
pub fn boolean_op(op: BoolOp, inputs: &[RegionInput]) -> Vec<Vec<SubPath>> {
    if inputs.is_empty() {
        return Vec::new();
    }
    let lists: Vec<&[SubPath]> = inputs.iter().map(|i| i.subs.as_slice()).collect();
    let extent = extent_of(&lists);
    let mut tr = Tracer::new(tol_for(extent), extent);
    if op == BoolOp::Division {
        let src = region_source(&inputs[0], &mut tr);
        let mut cuts: Vec<Edge> = Vec::new();
        let mut points: Vec<Vec2> = Vec::new();
        for r in &inputs[1..] {
            for sp in &r.subs {
                let line = tr.line(sp);
                for i in 1..line.len() {
                    cuts.push(seg_edge(line[i - 1], line[i]));
                }
                points.extend(line.iter().map(|&p| zv(p)));
            }
        }
        let pieces = overlay_z(
            &[
                src,
                Source {
                    edges: cuts,
                    points: Some(points),
                    cut: Some(true),
                },
            ],
            Rule::First,
        );
        return pieces
            .iter()
            .map(|a| areas_to_sub_paths(std::slice::from_ref(a), Some(&tr), 0.0))
            .collect();
    }
    let sources: Vec<Source> = inputs.iter().map(|i| region_source(i, &mut tr)).collect();
    let rule = match op {
        BoolOp::Union => Rule::Any,
        BoolOp::Intersection => Rule::All,
        BoolOp::Exclusion => Rule::Odd,
        _ => Rule::FirstNotOthers,
    };
    if op == BoolOp::Intersection && sources.len() < 2 {
        return Vec::new();
    }
    vec![areas_to_sub_paths(
        &overlay_z(&sources, rule),
        Some(&tr),
        0.0,
    )]
}

// ── Cut path ───────────────────────────────────────────────────────────

/// A cut point on a sub-path: segment and parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cut {
    pub seg: isize,
    pub t: f64,
}

/// The bottom path's outline cut where the others' outlines cross it: open
/// pieces, each exactly the part of the curves it covers (Béziers split at
/// the crossings). A sub-path nothing crosses stays as it is.
pub fn cut_path(target: &[SubPath], cutters: &[Vec<SubPath>]) -> Vec<SubPath> {
    let mut lists: Vec<&[SubPath]> = vec![target];
    lists.extend(cutters.iter().map(|c| c.as_slice()));
    let extent = extent_of(&lists);
    let tol = tol_for(extent);
    let mut tr = Tracer::new(tol, extent);
    struct Knife {
        a: Pt,
        b: Pt,
        min_x: f64,
        max_x: f64,
        min_y: f64,
        max_y: f64,
    }
    let mut knives: Vec<Knife> = Vec::new();
    for subs in cutters {
        for sp in subs {
            let line = if sp.closed {
                let mut r = tr.ring(sp);
                if let Some(n0) = sp.nodes.first() {
                    r.push(n0.pt());
                }
                r
            } else {
                tr.line(sp)
            };
            for i in 1..line.len() {
                let a = line[i - 1];
                let b = line[i];
                knives.push(Knife {
                    a,
                    b,
                    min_x: js_min(a[0], b[0]),
                    max_x: js_max(a[0], b[0]),
                    min_y: js_min(a[1], b[1]),
                    max_y: js_max(a[1], b[1]),
                });
            }
        }
    }
    let mut out = Vec::new();
    for sp in target {
        let n = segment_count(sp);
        let mut cuts: Vec<Cut> = Vec::new();
        for i in 0..n {
            let c = segment_cubic(sp, i);
            let line = segment_is_line(sp, i);
            let (pts, ts) = if line {
                (vec![c[0], c[3]], vec![0.0, 1.0])
            } else {
                flatten_cubic(&c, tol)
            };
            for k in 1..pts.len() {
                let a = pts[k - 1];
                let b = pts[k];
                for kn in &knives {
                    if kn.min_x > js_max(a[0], b[0])
                        || kn.max_x < js_min(a[0], b[0])
                        || kn.min_y > js_max(a[1], b[1])
                        || kn.max_y < js_min(a[1], b[1])
                    {
                        continue;
                    }
                    let Some(u) = cross_param(a, b, kn.a, kn.b) else {
                        continue;
                    };
                    let mut t = ts[k - 1] + (ts[k] - ts[k - 1]) * u;
                    if !line {
                        t = refine_on_line(&c, t, kn.a, kn.b);
                    }
                    cuts.push(Cut { seg: i as isize, t });
                }
            }
        }
        out.extend(split_at(sp, &cuts));
    }
    out
}

/// Parameter along a→b where it crosses c→d (ends included), or None.
fn cross_param(a: Pt, b: Pt, c: Pt, d: Pt) -> Option<f64> {
    let rx = b[0] - a[0];
    let ry = b[1] - a[1];
    let sx = d[0] - c[0];
    let sy = d[1] - c[1];
    let den = rx * sy - ry * sx;
    if den.abs() < 1e-18 {
        return None;
    }
    let t = ((c[0] - a[0]) * sy - (c[1] - a[1]) * sx) / den;
    let u = ((c[0] - a[0]) * ry - (c[1] - a[1]) * rx) / den;
    let e = 1e-12;
    if t >= -e && t <= 1.0 + e && u >= -e && u <= 1.0 + e {
        Some(js_min(1.0, js_max(0.0, t)))
    } else {
        None
    }
}

/// Newton steps moving t onto the line c–d (the flattened crossing is within the tolerance already).
fn refine_on_line(cu: &Cubic, t0: f64, c: Pt, d: Pt) -> f64 {
    let nx = -(d[1] - c[1]);
    let ny = d[0] - c[0];
    let mut t = t0;
    for _ in 0..6 {
        let p = bez(cu, t);
        let f = (p[0] - c[0]) * nx + (p[1] - c[1]) * ny;
        let h = 1e-6;
        let q = bez(cu, js_min(1.0, t + h));
        let df = ((q[0] - c[0]) * nx + (q[1] - c[1]) * ny - f) / h;
        if df.abs() < 1e-18 {
            break;
        }
        let next = t - f / df;
        if !(next >= 0.0 && next <= 1.0) || (next - t0).abs() > 0.05 {
            break;
        }
        t = next;
    }
    t
}

/// A sub-path split at (segment, t) points into open pieces.
pub fn split_at(sp: &SubPath, cuts_in: &[Cut]) -> Vec<SubPath> {
    let n = segment_count(sp) as isize;
    // Cuts at a segment's end are cuts at the next one's start; near-equal ones are one.
    let mut norm: Vec<Cut> = cuts_in
        .iter()
        .map(|&c| {
            if c.t >= 1.0 - 1e-9 {
                Cut {
                    seg: (c.seg + 1) % n.max(1),
                    t: 0.0,
                }
            } else if c.t <= 1e-9 {
                Cut { seg: c.seg, t: 0.0 }
            } else {
                c
            }
        })
        .filter(|c| sp.closed || c.seg < n)
        .collect();
    stable_sort(&mut norm, &mut |a, b| {
        a.seg.cmp(&b.seg).then(js_cmp(a.t - b.t, 0.0))
    });
    let mut cuts: Vec<Cut> = Vec::new();
    for c in norm {
        match cuts.last() {
            Some(l) if l.seg == c.seg && !(c.t - l.t > 1e-9) => {}
            _ => cuts.push(c),
        }
    }
    if !sp.closed {
        let inner: Vec<Cut> = cuts
            .into_iter()
            .filter(|c| !(c.seg == 0 && c.t == 0.0) && c.seg < n)
            .collect();
        if inner.is_empty() {
            return vec![sp.clone()];
        }
        let mut marks = vec![Cut { seg: 0, t: 0.0 }];
        marks.extend(inner);
        marks.push(Cut { seg: n - 1, t: 1.0 });
        return pieces_between(sp, &marks);
    }
    if cuts.is_empty() {
        return vec![sp.clone()];
    }
    let first = cuts[0];
    cuts.push(Cut {
        seg: first.seg + n,
        t: first.t,
    });
    pieces_between(sp, &cuts)
}

/// Open pieces between consecutive cut points (segment indices may run past the end of a closed ring).
fn pieces_between(sp: &SubPath, marks: &[Cut]) -> Vec<SubPath> {
    let n = segment_count(sp) as isize;
    let mut out = Vec::new();
    for k in 0..marks.len().saturating_sub(1) {
        let a = marks[k];
        let zm = marks[k + 1];
        let mut b = Builder::new();
        let mut started = false;
        let mut s = a.seg;
        while s <= zm.seg {
            let t0 = if s == a.seg { a.t } else { 0.0 };
            let t1 = if s == zm.seg { zm.t } else { 1.0 };
            if t1 - t0 <= 1e-12 && !(s == a.seg && s == zm.seg) {
                s += 1;
                continue;
            }
            let i = (((s % n) + n) % n) as usize;
            let c = segment_cubic(sp, i);
            let part = sub_cubic(&c, t0, t1);
            if !started {
                b.start(part[0]);
                started = true;
            }
            if segment_is_line(sp, i) {
                b.line_to(part[3]);
            } else {
                b.curve_to(part[1], part[2], part[3]);
            }
            s += 1;
        }
        if b.nodes.len() >= 2 {
            out.push(SubPath {
                closed: false,
                nodes: b.nodes,
            });
        }
    }
    out
}
