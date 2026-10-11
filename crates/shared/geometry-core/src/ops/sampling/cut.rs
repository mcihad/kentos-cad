//! An area cut in two by a line along an axis (docs/adr/0220 §6): what the
//! overlay gives for the area and either half of its box, without the
//! overlay's general arrangement (a 10 000-vertex ring in a few
//! milliseconds instead of hundreds). The rings are cut where they cross
//! the line (a straight edge's cut exactly on it; a vertex within the
//! overlay's 1 µm of the line counts as on it, its node the vertex itself,
//! as the overlay merges them), their pieces go to the side their middles
//! are on, an edge along the line to the side the area is on;
//! the line's stretches inside the area join the pieces of both halves; the
//! pieces are linked into rings keeping the area on their left (at a point
//! where several meet, the first turn clockwise from the way back).

use std::collections::HashMap;

use crate::geom::arrangement::{Area, Ring, TOL, WindingIndex, winding};
use crate::geom::bulge::{bulge_arc, bulge_at, bulge_of_sweep};
use crate::geom::intersect::{Edge, intersect_edges, point_at};
use crate::geom::region::{area_source, orient_ring, ring_area, ring_edges};
use crate::jsmath::{atan2, js_sign};
use crate::vec2::Vec2;

/// The cutting line: `x = at` when `across_x`, else `y = at`.
#[derive(Clone, Copy, Debug)]
pub struct Line {
    pub across_x: bool,
    pub at: f64,
}

impl Line {
    /// The coordinate that the line fixes.
    fn of(&self, p: Vec2) -> f64 {
        if self.across_x { p.x } else { p.y }
    }

    /// The coordinate along the line.
    fn along(&self, p: Vec2) -> f64 {
        if self.across_x { p.y } else { p.x }
    }

    /// Which side `p` is on: −1 below, 1 above, 0 on the line (within 1 µm).
    fn side(&self, p: Vec2) -> i8 {
        let d = self.of(p) - self.at;
        if d < -TOL {
            -1
        } else if d > TOL {
            1
        } else {
            0
        }
    }

    /// The point of the line at `s` along it.
    fn point(&self, s: f64) -> Vec2 {
        if self.across_x {
            Vec2::new(self.at, s)
        } else {
            Vec2::new(s, self.at)
        }
    }
}

/// A piece of a ring, with its own ends (a cut point exactly on the line).
#[derive(Clone, Copy, Debug)]
struct Piece {
    edge: Edge,
    a: Vec2,
    b: Vec2,
}

impl Piece {
    fn middle(&self) -> Vec2 {
        match self.edge {
            Edge::Seg { .. } => Vec2::new((self.a.x + self.b.x) / 2.0, (self.a.y + self.b.y) / 2.0),
            Edge::Arc { .. } => point_at(&self.edge, 0.5),
        }
    }

    /// The way it leaves its start and the way it reaches its end (radians).
    fn ways(&self) -> (f64, f64) {
        match self.edge {
            Edge::Seg { .. } => {
                let w = atan2(self.b.y - self.a.y, self.b.x - self.a.x);
                (w, w)
            }
            Edge::Arc { a0, sweep, .. } => {
                let turn = js_sign(sweep) * std::f64::consts::FRAC_PI_2;
                (a0 + turn, a0 + sweep + turn)
            }
        }
    }
}

/// The piece of an edge between parameters `t0` and `t1`, from `p0` to `p1`.
fn sub(edge: &Edge, t0: f64, t1: f64, p0: Vec2, p1: Vec2) -> Piece {
    let edge = match *edge {
        Edge::Seg { .. } => Edge::Seg { a: p0, b: p1 },
        Edge::Arc { c, r, a0, sweep } => Edge::Arc {
            c,
            r,
            a0: a0 + sweep * t0,
            sweep: sweep * (t1 - t0),
        },
    };
    Piece { edge, a: p0, b: p1 }
}

/// A ring's edges with their ends exactly its vertices (an arc's ends not worked out again from its angles).
fn ring_pieces(r: &Ring) -> Vec<(Edge, Vec2, Vec2)> {
    let n = r.pts.len();
    (0..n)
        .map(|i| {
            let (a, b) = (r.pts[i], r.pts[(i + 1) % n]);
            let edge = match bulge_arc(a, b, bulge_at(r.bulges.as_deref(), i)) {
                Some(arc) => Edge::Arc {
                    c: arc.c,
                    r: arc.r,
                    a0: arc.a0,
                    sweep: arc.sweep,
                },
                None => Edge::Seg { a, b },
            };
            (edge, a, b)
        })
        .filter(|(_, a, b)| a != b)
        .collect()
}

/// An edge from `start` to `end` cut where it crosses the line; its ends kept exactly.
fn cut_edge(
    e: &Edge,
    start: Vec2,
    end: Vec2,
    line: &Line,
    reach: (f64, f64),
    out: &mut Vec<Piece>,
) {
    let mut cuts: Vec<(f64, Vec2)> = Vec::new();
    match *e {
        Edge::Seg { a, b } => {
            if line.side(a) * line.side(b) == -1 {
                let (ca, cb) = (line.of(a), line.of(b));
                let t = (line.at - ca) / (cb - ca);
                let s = line.along(a) + (line.along(b) - line.along(a)) * t;
                cuts.push((t, line.point(s)));
            }
        }
        Edge::Arc { .. } => {
            let across = Edge::Seg {
                a: line.point(reach.0),
                b: line.point(reach.1),
            };
            let near = |p: Vec2, q: Vec2| crate::jsmath::js_hypot(p.x - q.x, p.y - q.y) <= TOL;
            for h in intersect_edges(e, &across) {
                // An end on the line is its own node.
                if h.t > 1e-12 && h.t < 1.0 - 1e-12 && !near(h.p, start) && !near(h.p, end) {
                    cuts.push((h.t, line.point(line.along(h.p))));
                }
            }
            cuts.sort_by(|x, y| x.0.total_cmp(&y.0));
        }
    }
    let (mut t0, mut p0) = (0.0, start);
    for (t, p) in cuts {
        if p != p0 {
            out.push(sub(e, t0, t, p0, p));
            t0 = t;
            p0 = p;
        }
    }
    if end != p0 {
        out.push(sub(e, t0, 1.0, p0, end));
    }
}

fn key(p: Vec2) -> (u64, u64) {
    (p.x.to_bits(), p.y.to_bits())
}

/// The angle turned clockwise from `from` to `to`, in (0, 2π].
fn clockwise(from: f64, to: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    let d = (from - to) % tau;
    let d = if d < 0.0 { d + tau } else { d };
    if d == 0.0 { tau } else { d }
}

/// Pieces linked into rings, the area on their left.
fn rings(pieces: &[Piece]) -> Vec<Ring> {
    let mut starts: HashMap<(u64, u64), Vec<usize>> = HashMap::new();
    for (i, p) in pieces.iter().enumerate() {
        starts.entry(key(p.a)).or_default().push(i);
    }
    let mut used = vec![false; pieces.len()];
    let mut out = Vec::new();
    for first in 0..pieces.len() {
        if used[first] {
            continue;
        }
        used[first] = true;
        let mut run = vec![first];
        let mut cur = first;
        let closed = loop {
            let (_, reach) = pieces[cur].ways();
            let back = reach + std::f64::consts::PI;
            let next = starts.get(&key(pieces[cur].b)).and_then(|list| {
                list.iter()
                    .copied()
                    .filter(|&j| !used[j] || j == first)
                    .min_by(|&i, &j| {
                        clockwise(back, pieces[i].ways().0)
                            .total_cmp(&clockwise(back, pieces[j].ways().0))
                    })
            });
            match next {
                Some(j) if j == first => break true,
                Some(j) => {
                    used[j] = true;
                    run.push(j);
                    cur = j;
                }
                None => break false,
            }
        };
        if !closed {
            continue;
        }
        let pts: Vec<Vec2> = run.iter().map(|&i| pieces[i].a).collect();
        let bulges: Vec<f64> = run
            .iter()
            .map(|&i| match pieces[i].edge {
                Edge::Seg { .. } => 0.0,
                Edge::Arc { sweep, .. } => bulge_of_sweep(sweep),
            })
            .collect();
        let curved = bulges.iter().any(|&b| b != 0.0);
        out.push(Ring {
            pts,
            bulges: curved.then_some(bulges),
        });
    }
    out
}

/// Rings as areas: counter-clockwise ones outer, clockwise ones holes in the
/// smallest outer one holding them.
fn areas(rings: Vec<Ring>) -> Vec<Area> {
    let mut outers: Vec<(f64, Area)> = Vec::new();
    let mut holes = Vec::new();
    for r in rings {
        let a = ring_area(&r);
        if a > 0.0 {
            outers.push((
                a,
                Area {
                    outer: r,
                    holes: Vec::new(),
                },
            ));
        } else if a < 0.0 {
            holes.push(r);
        }
    }
    for h in holes {
        let p = h.pts[0];
        let holder = outers
            .iter()
            .enumerate()
            .filter(|(_, (_, o))| winding(&ring_edges(&o.outer), p) != 0.0)
            .min_by(|(_, (x, _)), (_, (y, _))| x.total_cmp(y))
            .map(|(i, _)| i);
        if let Some(i) = holder {
            outers[i].1.holes.push(h);
        }
    }
    outers.into_iter().map(|(_, a)| a).collect()
}

/// The area's parts below the line (west of it, or south) and above it.
pub fn cut_area(area: &Area, line: Line) -> [Vec<Area>; 2] {
    let src = area_source(std::slice::from_ref(area));
    // Far enough along the line to cross every arc.
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for e in &src.edges {
        let b = crate::geom::arrangement::edge_box(e);
        let (a, z) = if line.across_x {
            (b.min_y, b.max_y)
        } else {
            (b.min_x, b.max_x)
        };
        lo = crate::jsmath::js_min(lo, a);
        hi = crate::jsmath::js_max(hi, z);
    }
    let reach = (lo - 1.0, hi + 1.0);
    let rings_in: Vec<Ring> = std::iter::once(orient_ring(&area.outer, true))
        .chain(area.holes.iter().map(|h| orient_ring(h, false)))
        .collect();
    let mut halves: [Vec<Piece>; 2] = [Vec::new(), Vec::new()];
    let mut events: Vec<Vec2> = Vec::new();
    let mut along: Vec<(f64, f64)> = Vec::new();
    let mut pieces = Vec::new();
    for r in &rings_in {
        for (e, a, b) in ring_pieces(r) {
            pieces.clear();
            cut_edge(&e, a, b, &line, reach, &mut pieces);
            for p in &pieces {
                let (sa, sb) = (line.side(p.a), line.side(p.b));
                if sa == 0 {
                    events.push(p.a);
                }
                if sb == 0 {
                    events.push(p.b);
                }
                if matches!(p.edge, Edge::Seg { .. }) && sa == 0 && sb == 0 {
                    // Along the line: the side the area is on (on its left).
                    let d = line.along(p.b) - line.along(p.a);
                    let low = if line.across_x { d > 0.0 } else { d < 0.0 };
                    halves[usize::from(!low)].push(*p);
                    let (s0, s1) = (line.along(p.a), line.along(p.b));
                    along.push(if s0 < s1 { (s0, s1) } else { (s1, s0) });
                } else {
                    let low = line.of(p.middle()) < line.at;
                    halves[usize::from(!low)].push(*p);
                }
            }
        }
    }
    // The nodes on the line in order along it; a node is a point (a vertex near the line keeps its own place).
    events.sort_by(|p, q| {
        line.along(*p)
            .total_cmp(&line.along(*q))
            .then(line.of(*p).total_cmp(&line.of(*q)))
    });
    events.dedup();
    let open: Vec<(Vec2, Vec2)> = events
        .windows(2)
        .map(|w| (w[0], w[1]))
        .filter(|&(p0, p1)| {
            let (s0, s1) = (line.along(p0), line.along(p1));
            !along.iter().any(|&(a, b)| a <= s0 && s1 <= b)
        })
        .collect();
    // A few stretches are counted round the edges; many through an index (its building is the dearer part).
    let index = (open.len() > 16).then(|| WindingIndex::new(&src.edges));
    let inside = |p: Vec2| match &index {
        Some(i) => i.winding(p) != 0.0,
        None => winding(&src.edges, p) != 0.0,
    };
    for (p0, p1) in open {
        let (s0, s1) = (line.along(p0), line.along(p1));
        if !inside(line.point((s0 + s1) / 2.0)) {
            continue;
        }
        let seg = |a: Vec2, b: Vec2| Piece {
            edge: Edge::Seg { a, b },
            a,
            b,
        };
        // Below a vertical line the area is west of it: its edge runs north; above it south.
        // Below a horizontal line the area is south of it: its edge runs west; above it east.
        if line.across_x {
            halves[0].push(seg(p0, p1));
            halves[1].push(seg(p1, p0));
        } else {
            halves[0].push(seg(p1, p0));
            halves[1].push(seg(p0, p1));
        }
    }
    let [low, high] = halves;
    [areas(rings(&low)), areas(rings(&high))]
}
