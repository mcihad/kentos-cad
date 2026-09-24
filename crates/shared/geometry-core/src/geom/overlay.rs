//! Planar overlay of straight and circular edges (`apps/web/src/model/geom/overlay.ts`),
//! the engine behind area booleans, splitting and "click inside to make an
//! area". The arrangement cuts and classifies the pieces; this file chains
//! the kept pieces into rings, always turning so the result stays on the
//! left, splits rings touching at a point and sorts holes into their outer
//! rings. Arcs stay arcs, and an input vertex keeps its exact coordinates.

use std::collections::HashMap;

use crate::geom::arc::norm_angle;
use crate::geom::arrangement::{
    Area, Built, DirPiece, Ring, Rule, Source, TOL, Vertices, build, classify, edge_box, edge_len,
    leave_angle, translate_edge, winding,
};
use crate::geom::bulge::{bulge_of_sweep, bulge_ring_area};
use crate::geom::intersect::{Edge, point_at};
use crate::geometry::Bounds;
use crate::jsmath::{TAU, js_cmp, js_hypot, js_max, js_min, js_sign, or, stable_sort};
use crate::vec2::Vec2;

// ── Rings ─────────────────────────────────────────────────────────────

/// Step 4: chains directed pieces into closed walks. Arriving at a vertex
/// the walk leaves by the first piece clockwise from the way it came, which
/// keeps the region on its left and never crosses itself.
fn trace(dps: &[DirPiece], vertex_count: usize) -> Vec<Vec<DirPiece>> {
    let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); vertex_count];
    for (i, d) in dps.iter().enumerate() {
        if let Some(list) = outgoing.get_mut(d.from) {
            list.push(i);
        }
    }
    let leave: Vec<f64> = dps.iter().map(|d| leave_angle(&d.edge, false)).collect();
    let back: Vec<f64> = dps.iter().map(|d| leave_angle(&d.edge, true)).collect();
    let next = |cur: usize| -> Option<usize> {
        let mut best = None;
        let mut best_angle = f64::INFINITY;
        for &o in outgoing.get(dps[cur].to).into_iter().flatten() {
            let mut cw = norm_angle(back[cur] - leave[o]);
            // Straight back along the same piece is the last resort (a dead end). The twin
            // is recognised as such, not by its angle: on a short arc the two angles come
            // from tiny chords and may differ by 1e-12 or so (docs/adr/0008).
            if cw < 1e-12 || twin(&dps[o], &dps[cur]) {
                cw = TAU;
            }
            if cw < best_angle {
                best_angle = cw;
                best = Some(o);
            }
        }
        best
    };
    let mut used = vec![false; dps.len()];
    let mut cycles = Vec::new();
    for s in 0..dps.len() {
        if used[s] {
            continue;
        }
        let mut cycle = Vec::new();
        let mut cur = Some(s);
        while let Some(c) = cur {
            if used[c] {
                break;
            }
            used[c] = true;
            cycle.push(dps[c]);
            cur = next(c);
        }
        cycles.push(cycle);
    }
    cycles
}

fn twin(a: &DirPiece, b: &DirPiece) -> bool {
    a.piece == b.piece && a.fwd != b.fwd
}

/// Drops back-and-forth runs (dangling cut lines, isolated line work).
fn remove_spikes(cycle: Vec<DirPiece>) -> Vec<DirPiece> {
    let mut st: Vec<DirPiece> = Vec::with_capacity(cycle.len());
    for d in cycle {
        if st.last().is_some_and(|l| twin(l, &d)) {
            st.pop();
        } else {
            st.push(d);
        }
    }
    let (mut lo, mut hi) = (0, st.len());
    while hi - lo >= 2 && twin(&st[lo], &st[hi - 1]) {
        lo += 1;
        hi -= 1;
    }
    st[lo..hi].to_vec()
}

/// Splits a walk that passes a vertex twice into simple rings.
fn split_at_repeats(cycle: Vec<DirPiece>, out: &mut Vec<Vec<DirPiece>>) {
    let mut seen: HashMap<usize, usize> = HashMap::new();
    for i in 0..cycle.len() {
        let v = cycle[i].from;
        if let Some(&j) = seen.get(&v) {
            let lp = cycle[j..i].to_vec();
            let mut rest = cycle[..j].to_vec();
            rest.extend_from_slice(&cycle[i..]);
            split_at_repeats(lp, out);
            split_at_repeats(rest, out);
            return;
        }
        seen.insert(v, i);
    }
    out.push(cycle);
}

/// Joins pieces split at a point that was not an input vertex and does not turn.
fn merge_straight(cycle: &[DirPiece], verts: &Vertices) -> (Vec<Edge>, Vec<usize>) {
    let mut edges: Vec<Edge> = cycle.iter().map(|d| d.edge).collect();
    let mut from: Vec<usize> = cycle.iter().map(|d| d.from).collect();
    let mut i = 0;
    while i < edges.len() && edges.len() > 2 {
        let j = (i + 1) % edges.len();
        let m = if verts.input[from[j]].is_some() {
            None
        } else {
            joinable(&edges[i], &edges[j])
        };
        let Some(m) = m else {
            i += 1;
            continue;
        };
        edges[i] = m;
        edges.remove(j);
        from.remove(j);
        if j < i {
            i -= 1;
        }
    }
    (edges, from)
}

fn joinable(a: &Edge, b: &Edge) -> Option<Edge> {
    match (*a, *b) {
        (Edge::Seg { a: aa, b: ab }, Edge::Seg { a: ba, b: bb }) => {
            let ux = ab.x - aa.x;
            let uy = ab.y - aa.y;
            let vx = bb.x - ba.x;
            let vy = bb.y - ba.y;
            let l = js_hypot(ux, uy) * js_hypot(vx, vy);
            ((ux * vy - uy * vx).abs() <= 1e-12 * l && ux * vx + uy * vy > 0.0)
                .then_some(Edge::Seg { a: aa, b: bb })
        }
        (
            Edge::Arc {
                c: ca,
                r: ra,
                a0,
                sweep: sa,
            },
            Edge::Arc {
                c: cb,
                r: rb,
                sweep: sb,
                ..
            },
        ) => {
            let same = js_hypot(ca.x - cb.x, ca.y - cb.y) <= TOL
                && (ra - rb).abs() <= TOL
                && js_sign(sa) == js_sign(sb);
            (same && (sa + sb).abs() < TAU - 1e-6).then_some(Edge::Arc {
                c: ca,
                r: ra,
                a0,
                sweep: sa + sb,
            })
        }
        _ => None,
    }
}

/// A finished ring in local coordinates plus a point just inside its left side.
#[derive(Clone)]
pub(crate) struct LocalRing {
    pub ring: Ring,
    pub edges: Vec<Edge>,
    pub area: f64,
    pub probe: Vec2,
}

fn to_ring(edges: Vec<Edge>, cycle_from: &[usize], verts: &Vertices, origin: Vec2) -> LocalRing {
    let mut pts = Vec::with_capacity(edges.len());
    let mut bulges = Vec::with_capacity(edges.len());
    let mut local = Vec::with_capacity(edges.len());
    for (i, e) in edges.iter().enumerate() {
        let v = cycle_from[i];
        let p = verts.pos[v];
        local.push(p);
        pts.push(match verts.input[v] {
            Some(input) => input,
            None => Vec2::new(p.x + origin.x, p.y + origin.y),
        });
        bulges.push(match *e {
            Edge::Arc { sweep, .. } => bulge_of_sweep(sweep),
            Edge::Seg { .. } => 0.0,
        });
    }
    let area = bulge_ring_area(&local, Some(&bulges));
    let ring = if bulges.iter().any(|&b| b != 0.0) {
        Ring {
            pts,
            bulges: Some(bulges),
        }
    } else {
        Ring { pts, bulges: None }
    };
    // Probe: a hair to the left of the longest edge's middle.
    let mut best = 0;
    for (i, e) in edges.iter().enumerate() {
        if edge_len(e) > edge_len(&edges[best]) {
            best = i;
        }
    }
    let e = edges[best];
    let m = point_at(&e, 0.5);
    let q = point_at(&e, 0.5 + 1e-6);
    let l = or(js_hypot(q.x - m.x, q.y - m.y), 1.0);
    let eps = js_min(edge_len(&e) * 1e-4, 1e-4);
    let probe = Vec2::new(m.x - ((q.y - m.y) / l) * eps, m.y + ((q.x - m.x) / l) * eps);
    LocalRing {
        ring,
        edges,
        area,
        probe,
    }
}

/// Walks → simple local rings.
pub(crate) fn rings(dps: &[DirPiece], verts: &Vertices, origin: Vec2) -> Vec<LocalRing> {
    let mut out = Vec::new();
    for walk in trace(dps, verts.pos.len()) {
        let mut simple = Vec::new();
        split_at_repeats(remove_spikes(walk), &mut simple);
        for cycle in simple {
            if cycle.len() < 2 {
                continue;
            }
            let (edges, from) = merge_straight(&cycle, verts);
            if edges.len() < 2 {
                continue;
            }
            let perimeter = edges.iter().fold(0.0, |s, e| s + edge_len(e));
            let r = to_ring(edges, &from, verts, origin);
            // Slivers thinner than the tolerance are numerical dust.
            if r.area.abs() <= perimeter * TOL {
                continue;
            }
            out.push(r);
        }
    }
    out
}

/// Holes go to the smallest outer ring around them.
fn assemble(local: Vec<LocalRing>) -> Vec<Area> {
    let (mut outers, holes): (Vec<LocalRing>, Vec<LocalRing>) = local
        .into_iter()
        .filter(|r| r.area != 0.0)
        .partition(|r| r.area > 0.0);
    stable_sort(&mut outers, &mut |a, b| js_cmp(a.area - b.area, 0.0));
    let mut parts: Vec<Area> = outers
        .iter()
        .map(|o| Area {
            outer: o.ring.clone(),
            holes: Vec::new(),
        })
        .collect();
    for h in holes.into_iter().filter(|h| h.area < 0.0) {
        if let Some(host) = outers
            .iter()
            .position(|o| winding(&o.edges, h.probe) != 0.0)
        {
            parts[host].holes.push(h.ring);
        }
    }
    parts
}

// ── Entry points ──────────────────────────────────────────────────────

fn origin_of(sources: &[Source]) -> Vec2 {
    for s in sources {
        if let Some(e) = s.edges.first() {
            return point_at(e, 0.0);
        }
    }
    Vec2::new(0.0, 0.0)
}

/// Moves everything near the origin: intersections are computed on small numbers.
fn localize(sources: &[Source], o: Vec2) -> Vec<Source> {
    sources
        .iter()
        .map(|s| Source {
            edges: s
                .edges
                .iter()
                .map(|e| translate_edge(e, -o.x, -o.y))
                .collect(),
            points: s.points.clone(),
            cut: s.cut,
        })
        .collect()
}

fn run(sources: &[Source], rule: Rule) -> (Built, Vec<DirPiece>, Vec2) {
    let o = origin_of(sources);
    let local = localize(sources, o);
    let built = build(&local, sources, o);
    let dps = classify(&local, &built, rule);
    (built, dps, o)
}

/// Runs the overlay and returns the areas where `rule` holds.
pub fn overlay(sources: &[Source], rule: Rule) -> Vec<Area> {
    let (built, dps, o) = run(sources, rule);
    assemble(rings(&dps, &built.verts, o))
}

/// A closed walk of the line work: a bounded face (area > 0) or the outline of a connected group (< 0).
pub struct FaceRing {
    pub ring: Ring,
    pub area: f64,
    /// The ring's edges in local coordinates (winding tests: `contains`).
    pub edges: Vec<Edge>,
    pub origin: Vec2,
    /// A point just off the ring on its left (inside a face, outside a group).
    pub probe: Vec2,
    /// Bounding box in true coordinates (quick reject before `contains`).
    pub bx: Bounds,
}

impl FaceRing {
    /// Winding test in true coordinates.
    pub fn contains(&self, p: Vec2) -> bool {
        winding(
            &self.edges,
            Vec2::new(p.x - self.origin.x, p.y - self.origin.y),
        ) != 0.0
    }
}

crate::json_struct!(out FaceRing { ring, area, probe, bx => "box" });

/// Every closed walk formed by the line work.
pub fn face_rings(sources: &[Source]) -> Vec<FaceRing> {
    let cuts: Vec<Source> = sources
        .iter()
        .map(|s| Source {
            edges: s.edges.clone(),
            points: s.points.clone(),
            cut: Some(true),
        })
        .collect();
    let (built, dps, o) = run(&cuts, Rule::Always);
    rings(&dps, &built.verts, o)
        .into_iter()
        .map(|r| {
            let mut bx = Bounds {
                min_x: f64::INFINITY,
                min_y: f64::INFINITY,
                max_x: f64::NEG_INFINITY,
                max_y: f64::NEG_INFINITY,
            };
            for e in &r.edges {
                let b = edge_box(e);
                bx = Bounds {
                    min_x: js_min(bx.min_x, b.min_x + o.x),
                    min_y: js_min(bx.min_y, b.min_y + o.y),
                    max_x: js_max(bx.max_x, b.max_x + o.x),
                    max_y: js_max(bx.max_y, b.max_y + o.y),
                };
            }
            FaceRing {
                ring: r.ring,
                area: r.area,
                probe: Vec2::new(r.probe.x + o.x, r.probe.y + o.y),
                edges: r.edges,
                origin: o,
                bx,
            }
        })
        .collect()
}
