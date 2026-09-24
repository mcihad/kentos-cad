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
    reverse_edge, translate_edge, winding,
};
use crate::geom::bulge::{bulge_of_sweep, bulge_ring_area};
use crate::geom::intersect::{Edge, point_at};
use crate::geometry::Bounds;
use crate::jsmath::{
    PI, TAU, atan2, js_cmp, js_hypot, js_max, js_min, js_sign, or, sin, stable_sort,
};
use crate::predicates::orientation;
use crate::vec2::Vec2;

// ── Rings ─────────────────────────────────────────────────────────────

/// How far from a vertex the order of the pieces leaving it is read: past
/// the meeting points the arrangement leaves unresolved (within `TOL` of the
/// vertex) and short of the ones it keeps (pieces run at least that far
/// apart before they meet again).
const RHO: f64 = 10.0 * TOL;
/// Angles computed from difference vectors are good to a few ulps; two
/// directions closer than this are one tangent.
const SAME_TANGENT: f64 = 1e-14;

/// How a directed piece leaves vertex `at` towards vertex `far`, in the
/// ring's own geometry (vertex positions, and an arc's sweep: the bulge the
/// ring is written with): the direction of the chord to the point `RHO`
/// along it, and its signed curvature (+ turning left). Angles come from
/// difference vectors, never from points placed along the piece, so they
/// hold however short the piece or far the vertex from the origin (the
/// chord to 1e-4 of the piece they replace lost its bits there).
#[derive(Clone, Copy)]
struct Leave {
    angle: f64,
    kappa: f64,
    /// A straight piece's far vertex: straight directions are decided exactly.
    far: Option<Vec2>,
}

fn leave(edge: &Edge, at: Vec2, far: Vec2) -> Leave {
    let chord = atan2(far.y - at.y, far.x - at.x);
    match *edge {
        Edge::Seg { .. } => Leave {
            angle: chord,
            kappa: 0.0,
            far: Some(far),
        },
        Edge::Arc { sweep, .. } => {
            // The tangent turns from the chord by half the sweep; the radius follows from the chord.
            let kappa = js_sign(sweep) * 2.0 * sin(sweep.abs() / 2.0)
                / js_hypot(far.x - at.x, far.y - at.y);
            Leave {
                angle: chord - sweep / 2.0 + kappa * RHO / 2.0,
                kappa,
                far: None,
            }
        }
    }
}

/// Where a way out lies turning clockwise from the way back: `group` 0 just
/// clockwise of it (the same tangent, bending right of it), 1 anywhere
/// else, 2 just counter-clockwise of it, 3 back along it (the same piece:
/// a dead end, the last resort). `half` orders straight pieces exactly:
/// 0 clockwise angle in (0, π], 1 in (π, 2π).
struct Rank {
    group: u8,
    half: u8,
    angle: f64,
    leave: Leave,
}

/// A clockwise angle known (exactly) to lie in (0, π], held there if rounding put it outside.
fn in_first_half(cw: f64) -> f64 {
    if cw > 1.5 * PI || cw <= 0.0 {
        f64::MIN_POSITIVE
    } else if cw > PI {
        PI
    } else {
        cw
    }
}

/// A clockwise angle known to lie in (π, 2π), held there likewise.
fn in_second_half(cw: f64) -> f64 {
    if cw > PI {
        cw
    } else if cw < 0.5 * PI {
        TAU - 1e-15
    } else {
        PI + 1e-15
    }
}

fn sign(x: f64) -> i8 {
    i8::from(x > 0.0) - i8::from(x < 0.0)
}

fn rank(v: Vec2, back: &Leave, o: &Leave, dead_end: bool) -> Rank {
    let at = |group, half, angle| Rank {
        group,
        half,
        angle,
        leave: *o,
    };
    if dead_end {
        return at(3, 0, TAU);
    }
    let cw = norm_angle(back.angle - o.angle);
    if let (Some(r), Some(p)) = (back.far, o.far) {
        // Both straight: the side of the way back is exact (§23.3); the angle, kept for
        // comparing with arcs, is held on that side where rounding put it across.
        return match orientation(v, r, p) {
            s if s < 0 => at(1, 0, in_first_half(cw)),
            s if s > 0 => at(1, 1, in_second_half(cw)),
            // On the line of the way back: along it (a piece on top of it) or straight on (π).
            _ if sign(r.x - v.x) == sign(p.x - v.x) && sign(r.y - v.y) == sign(p.y - v.y) => {
                at(3, 0, TAU)
            }
            _ => at(1, 0, PI),
        };
    }
    if cw <= SAME_TANGENT || cw >= TAU - SAME_TANGENT {
        // The way back's own tangent: bending right of it is just clockwise, left just counter-clockwise.
        return if o.kappa < back.kappa {
            at(0, 0, 0.0)
        } else if o.kappa > back.kappa {
            at(2, 0, TAU)
        } else {
            at(3, 0, TAU)
        };
    }
    at(1, 0, cw)
}

/// Whether `a` comes before `b` turning clockwise; on one tangent a piece
/// bending left comes first (a curve bending right lies clockwise of it).
fn before(v: Vec2, a: &Rank, b: &Rank, exact: bool) -> bool {
    if a.group != b.group {
        return a.group < b.group;
    }
    if a.group == 3 {
        return false;
    }
    let (fa, fb) = (a.leave.far, b.leave.far);
    if exact {
        if a.half != b.half {
            return a.half < b.half;
        }
        if let (Some(pa), Some(pb)) = (fa, fb) {
            return orientation(v, pa, pb) < 0;
        }
    }
    if a.group == 1 && (a.angle - b.angle).abs() > SAME_TANGENT {
        return a.angle < b.angle;
    }
    if let (Some(pa), Some(pb)) = (fa, fb) {
        return orientation(v, pa, pb) < 0;
    }
    a.leave.kappa > b.leave.kappa
}

/// Step 4: chains directed pieces into closed walks. Arriving at a vertex
/// the walk leaves by the first piece clockwise from the way it came, which
/// keeps the region on its left and never crosses itself. The order is the
/// ring's own (vertex positions and sweeps); between straight pieces it is
/// exact (CLAUDE.md §23.3).
fn trace(dps: &[DirPiece], verts: &Vertices) -> Vec<Vec<DirPiece>> {
    let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); verts.pos.len()];
    for (i, d) in dps.iter().enumerate() {
        if let Some(list) = outgoing.get_mut(d.from) {
            list.push(i);
        }
    }
    let pos = |i: usize| verts.pos[i];
    let out: Vec<Leave> = dps
        .iter()
        .map(|d| leave(&d.edge, pos(d.from), pos(d.to)))
        .collect();
    let back: Vec<Leave> = dps
        .iter()
        .map(|d| leave(&reverse_edge(&d.edge), pos(d.to), pos(d.from)))
        .collect();
    let next = |cur: usize| -> Option<usize> {
        let v = pos(dps[cur].to);
        let b = &back[cur];
        let mut best: Option<(usize, Rank)> = None;
        for &o in outgoing.get(dps[cur].to).into_iter().flatten() {
            // Straight back along the same piece is the last resort (a dead end).
            let r = rank(v, b, &out[o], twin(&dps[o], &dps[cur]));
            let exact = b.far.is_some() && r.leave.far.is_some();
            let better = match &best {
                None => true,
                Some((_, cur_best)) => {
                    before(v, &r, cur_best, exact && cur_best.leave.far.is_some())
                }
            };
            if better {
                best = Some((o, r));
            }
        }
        best.map(|(o, _)| o)
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
    for walk in trace(dps, verts) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jsmath::cos;

    /// The order rule before §23.3: the chord to 1e-4 of each piece, its
    /// angle taken from a point placed along the piece in local coordinates.
    fn chord_angle(e: &Edge, at_end: bool) -> f64 {
        let from = point_at(e, if at_end { 1.0 } else { 0.0 });
        let q = point_at(e, if at_end { 1.0 - 1e-4 } else { 1e-4 });
        atan2(q.y - from.y, q.x - from.x)
    }

    fn chord_cw(back: f64, leave: f64) -> f64 {
        let cw = norm_angle(back - leave);
        if cw < 1e-12 { TAU } else { cw }
    }

    struct Rnd(u64);
    impl Rnd {
        fn next(&mut self) -> f64 {
            self.0 = self.0 * 16807 % 2147483647;
            self.0 as f64 / 2147483647.0
        }
    }

    /// Straight pieces leaving a vertex 1 500 km from the overlay's origin
    /// within a few µrad of each other: the order turning clockwise from the
    /// way back agrees pair by pair with exact integer arithmetic (2⁻³⁵ grid).
    /// The chord rule, its bits lost to the large coordinates, got some wrong.
    #[test]
    fn straight_pieces_leave_a_far_vertex_in_the_exact_order() {
        let v = Vec2::new(1_500_000.37, 250_000.11);
        let r = Vec2::new(v.x - 100.25, v.y - 7.5);
        let back = leave(&Edge::Seg { a: v, b: r }, v, r);
        let k = |x: f64| (x * (1u64 << 35) as f64) as i128;
        let cross = |a: Vec2, b: Vec2| {
            (k(a.x) - k(v.x)) * (k(b.y) - k(v.y)) - (k(a.y) - k(v.y)) * (k(b.x) - k(v.x))
        };
        // Exact: clockwise half of the way back first, then clockwise order within the half.
        let half = |p: Vec2| u8::from(cross(r, p) > 0);
        let exact_before = |a: Vec2, b: Vec2| {
            if half(a) != half(b) {
                half(a) < half(b)
            } else {
                cross(a, b) < 0
            }
        };
        let old_back = chord_angle(&Edge::Seg { a: r, b: v }, true);
        let mut rnd = Rnd(11);
        let (mut pairs, mut chord_wrong) = (0, 0);
        for _ in 0..300 {
            let theta0 = rnd.next() * TAU;
            let pts: Vec<Vec2> = (0..6)
                .map(|_| {
                    let t = theta0 + (rnd.next() - 0.5) * 4e-6;
                    let l = 0.5 + rnd.next() * 1.5;
                    Vec2::new(v.x + l * cos(t), v.y + l * sin(t))
                })
                .collect();
            for (i, &a) in pts.iter().enumerate() {
                for &b in &pts[i + 1..] {
                    if cross(a, b) == 0 || cross(r, a) == 0 || cross(r, b) == 0 {
                        continue;
                    }
                    pairs += 1;
                    let (la, lb) = (
                        leave(&Edge::Seg { a: v, b: a }, v, a),
                        leave(&Edge::Seg { a: v, b }, v, b),
                    );
                    let (ra, rb) = (rank(v, &back, &la, false), rank(v, &back, &lb, false));
                    assert_eq!(before(v, &ra, &rb, true), exact_before(a, b), "{a:?} {b:?}");
                    assert_eq!(before(v, &rb, &ra, true), exact_before(b, a), "{b:?} {a:?}");
                    let (ca, cb) = (
                        chord_cw(old_back, chord_angle(&Edge::Seg { a: v, b: a }, false)),
                        chord_cw(old_back, chord_angle(&Edge::Seg { a: v, b }, false)),
                    );
                    if (ca < cb) != exact_before(a, b) {
                        chord_wrong += 1;
                    }
                }
            }
        }
        assert!(pairs > 4000, "{pairs}");
        assert!(chord_wrong > 0, "the chord rule no longer loses bits here");
        // Straight on (π) and along the way back.
        let on = Vec2::new(v.x + (v.x - r.x), v.y + (v.y - r.y));
        let ro = rank(v, &back, &leave(&Edge::Seg { a: v, b: on }, v, on), false);
        assert_eq!((ro.group, ro.half, ro.angle), (1, 0, PI));
        let along = Vec2::new(v.x - (v.x - r.x) / 2.0, v.y - (v.y - r.y) / 2.0);
        assert_eq!(
            rank(
                v,
                &back,
                &leave(&Edge::Seg { a: v, b: along }, v, along),
                false
            )
            .group,
            3
        );
    }

    /// A line through a vertex 100 km from the origin and two short arcs of
    /// radius 1 leaving it on the line's tangent, one bending left, one
    /// right. Coming back along the line, the left one comes first, then the
    /// line, then the right one: the bend decides. The chord rule, on arcs
    /// only a few centimetres long, often could not tell.
    #[test]
    fn a_short_arc_on_a_line_tangent_is_ordered_by_its_bend() {
        let v = Vec2::new(100_000.37, 40_000.11);
        let mut rnd = Rnd(5);
        let mut chord_wrong = 0;
        for _ in 0..400 {
            let phi = rnd.next() * TAU;
            let t = Vec2::new(cos(phi), sin(phi));
            let (sweep, radius) = (0.005 + rnd.next() * 0.04, 0.5 + rnd.next());
            let line = Vec2::new(v.x + 5.0 * t.x, v.y + 5.0 * t.y);
            let r = Vec2::new(v.x - 5.0 * t.x, v.y - 5.0 * t.y);
            let arc = |side: f64| {
                let c = Vec2::new(v.x - side * radius * t.y, v.y + side * radius * t.x);
                Edge::Arc {
                    c,
                    r: radius,
                    a0: atan2(v.y - c.y, v.x - c.x),
                    sweep: side * sweep,
                }
            };
            let (left, right) = (arc(1.0), arc(-1.0));
            let back = leave(&Edge::Seg { a: v, b: r }, v, r);
            let ranks = [
                rank(v, &back, &leave(&left, v, point_at(&left, 1.0)), false),
                rank(
                    v,
                    &back,
                    &leave(&Edge::Seg { a: v, b: line }, v, line),
                    false,
                ),
                rank(v, &back, &leave(&right, v, point_at(&right, 1.0)), false),
            ];
            for i in 0..3 {
                for j in 0..3 {
                    if i != j {
                        assert_eq!(
                            before(v, &ranks[i], &ranks[j], false),
                            i < j,
                            "φ {phi} {i} {j}"
                        );
                    }
                }
            }
            let old_back = chord_angle(&Edge::Seg { a: r, b: v }, true);
            let old = [
                chord_cw(old_back, chord_angle(&left, false)),
                chord_cw(old_back, chord_angle(&Edge::Seg { a: v, b: line }, false)),
                chord_cw(old_back, chord_angle(&right, false)),
            ];
            if !(old[0] < old[1] && old[1] < old[2]) {
                chord_wrong += 1;
            }
        }
        assert!(chord_wrong > 0, "the chord rule no longer loses bits here");
    }

    /// A fillet left untrimmed (the line goes on) 100 km from the overlay's
    /// origin, crossed 3 cm from the tangent point: every point in the
    /// frame lies in exactly one face, the thin ones between line and arc
    /// included.
    #[test]
    fn faces_around_an_untrimmed_fillet_far_from_the_origin() {
        let o = Vec2::new(486_512.34, 4_420_187.52);
        let seg = |a: Vec2, b: Vec2| Edge::Seg { a, b };
        let mut rnd = Rnd(3);
        for _ in 0..40 {
            let v = Vec2::new(o.x + 100_000.37 + rnd.next(), o.y + 40_000.11 + rnd.next());
            let phi = rnd.next() * TAU;
            let (t, n) = (
                Vec2::new(cos(phi), sin(phi)),
                Vec2::new(-sin(phi), cos(phi)),
            );
            let at = |a: f64, b: f64| Vec2::new(v.x + a * t.x + b * n.x, v.y + a * t.y + b * n.y);
            let arc = |side: f64| {
                let c = at(0.0, side);
                Edge::Arc {
                    c,
                    r: 1.0,
                    a0: atan2(v.y - c.y, v.x - c.x),
                    sweep: side * 0.8,
                }
            };
            let frame = [at(-2.0, -2.0), at(2.0, -2.0), at(2.0, 2.0), at(-2.0, 2.0)];
            let sources = [
                // Far away first: the overlay's origin.
                Source {
                    edges: vec![seg(o, Vec2::new(o.x + 1.0, o.y))],
                    points: None,
                    cut: None,
                },
                Source {
                    edges: (0..4).map(|i| seg(frame[i], frame[(i + 1) % 4])).collect(),
                    points: None,
                    cut: None,
                },
                Source {
                    edges: vec![
                        seg(at(-2.0, 0.0), at(2.0, 0.0)),
                        seg(at(0.03, -2.0), at(0.03, 2.0)),
                    ],
                    points: None,
                    cut: None,
                },
                Source {
                    edges: vec![arc(1.0), arc(-1.0)],
                    points: None,
                    cut: None,
                },
            ];
            let faces: Vec<FaceRing> = face_rings(&sources)
                .into_iter()
                .filter(|f| f.area > 0.0)
                .collect();
            for _ in 0..300 {
                // Mostly near the tangent point, where the thin faces are.
                let (a, b) = if rnd.next() < 0.7 {
                    (rnd.next() * 0.06 - 0.015, (rnd.next() - 0.5) * 0.004)
                } else {
                    ((rnd.next() - 0.5) * 3.9, (rnd.next() - 0.5) * 3.9)
                };
                let p = at(a, b);
                let n = faces.iter().filter(|f| f.contains(p)).count();
                assert_eq!(n, 1, "φ {phi}: ({a}, {b}) in {n} faces");
            }
        }
    }
}
