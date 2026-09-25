//! Stroke outlines and offsets as fill regions (`apps/web/src/style/svg/pathStroke.ts`).
//! A stroke is the union of simple pieces, all counter-clockwise in one
//! overlay source (so nonzero winding is their union): a ribbon along every
//! smooth run of the flattened path (per chord rectangles where the inner
//! side folds), a join at every corner (round disc, mitre within the limit,
//! else bevel) and a cap at every open end (round disc, square box, nothing
//! for butt). Dashes cut the path first. Inset and outset take the stroke
//! of the region's outline away from it or add it. The union's outline is
//! fitted with cubics; round joins and caps are exact arcs turned into
//! cubics.

use kentos_geometry_core::geom::arrangement::{Rule, Source};
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::jsmath::{PI, TAU, atan2, cos, js_hypot, js_max, js_min};
use kentos_geometry_core::vec2::Vec2;

use crate::bezier::{
    bez_tangent, flatten_cubic, ring_signed_area, segment_count, segment_cubic, segment_is_line,
};
use crate::boolean::{
    RegionInput, Tracer, area_source_z, areas_to_sub_paths, extent_of, overlay_z, region_source,
    tol_for, zv,
};
use crate::shape::{PathNode, Pt, SubPath};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cap {
    Butt,
    Round,
    Square,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Join {
    Miter,
    Round,
    Bevel,
}

impl Cap {
    /// By its SVG name; an unknown name draws nothing at the ends, as butt.
    pub fn named(s: &str) -> Cap {
        match s {
            "round" => Cap::Round,
            "square" => Cap::Square,
            _ => Cap::Butt,
        }
    }
}

impl Join {
    /// By its SVG name; an unknown name bevels, as the TypeScript did.
    pub fn named(s: &str) -> Join {
        match s {
            "round" => Join::Round,
            "miter" => Join::Miter,
            _ => Join::Bevel,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    pub width: f64,
    pub cap: Cap,
    pub join: Join,
    /// SVG stroke-miterlimit (default 4).
    pub miter_limit: Option<f64>,
    pub dash: Option<Vec<f64>>,
}

/// A flattened line: its points, whether each one is a corner (a join goes
/// there) and whether it is a node of the source (a sample inside a curve
/// turning sharply gets a round join whatever the style: a smooth curve's
/// stroke has no corners).
#[derive(Clone)]
struct Poly {
    pts: Vec<Pt>,
    corner: Vec<bool>,
    node: Vec<bool>,
    closed: bool,
}

fn sub(a: Pt, b: Pt) -> Pt {
    [a[0] - b[0], a[1] - b[1]]
}
fn unit(a: Pt) -> Pt {
    let l = js_hypot(a[0], a[1]);
    if l > 1e-15 {
        [a[0] / l, a[1] / l]
    } else {
        [0.0, 0.0]
    }
}
/// Left normal of the direction a→b (x right, y as given).
fn normal(a: Pt, b: Pt) -> Pt {
    let d = unit(sub(b, a));
    [-d[1], d[0]]
}
fn cross(a: Pt, b: Pt) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

/// Turns sharper than this (radians) between chords are corners that get the join.
fn corner_turn() -> f64 {
    (12.0 * PI) / 180.0
}

fn poly_of(sp: &SubPath, tol: f64) -> Poly {
    let mut pts: Vec<Pt> = Vec::new();
    let mut node: Vec<bool> = Vec::new();
    let push = |pts: &mut Vec<Pt>, node: &mut Vec<bool>, p: Pt, is_node: bool| {
        if let Some(q) = pts.last()
            && js_hypot(p[0] - q[0], p[1] - q[1]) < 1e-12
        {
            if let Some(last) = node.last_mut() {
                *last = *last || is_node;
            }
            return;
        }
        pts.push(p);
        node.push(is_node);
    };
    if sp.nodes.is_empty() {
        return Poly {
            pts,
            corner: Vec::new(),
            node,
            closed: sp.closed,
        };
    }
    let n = segment_count(sp);
    // A node where the tangent runs on (a smooth node) is no corner: its stroke needs no join.
    let corner_node = |i: usize| -> bool {
        if !sp.closed && (i == 0 || i == sp.nodes.len() - 1) {
            return true;
        }
        if n < 2 {
            return true;
        }
        let a = bez_tangent(&segment_cubic(sp, (i + n - 1) % n), 1.0);
        let b = bez_tangent(&segment_cubic(sp, i % n), 0.0);
        a[0] * b[0] + a[1] * b[1] < cos((0.5 * PI) / 180.0)
    };
    push(&mut pts, &mut node, sp.nodes[0].pt(), corner_node(0));
    for i in 0..n {
        let c = segment_cubic(sp, i);
        let end = corner_node(i + 1);
        if segment_is_line(sp, i) {
            push(&mut pts, &mut node, c[3], end);
        } else {
            let (ps, _) = flatten_cubic(&c, tol);
            for k in 1..ps.len() {
                push(&mut pts, &mut node, ps[k], k == ps.len() - 1 && end);
            }
        }
    }
    if sp.closed && pts.len() > 1 {
        let a = pts[0];
        let b = pts[pts.len() - 1];
        if js_hypot(a[0] - b[0], a[1] - b[1]) < 1e-12 {
            pts.pop();
            node.pop();
        }
    }
    with_corners(pts, node, sp.closed, 1e-3)
}

/// Corners: nodes where the line turns more than `node_turn` (radians), and sharp turns anywhere.
fn with_corners(pts: Vec<Pt>, node: Vec<bool>, closed: bool, node_turn: f64) -> Poly {
    let m = pts.len();
    let mut corner = vec![false; node.len()];
    for i in 0..m {
        if !closed && (i == 0 || i == m - 1) {
            continue;
        }
        let a = pts[(i + m - 1) % m];
        let b = pts[i];
        let c = pts[(i + 1) % m];
        let turn = atan2(
            cross(sub(b, a), sub(c, b)),
            (b[0] - a[0]) * (c[0] - b[0]) + (b[1] - a[1]) * (c[1] - b[1]),
        )
        .abs();
        let is_corner = turn > corner_turn() || (node[i] && turn > node_turn);
        if i < corner.len() {
            corner[i] = is_corner;
        }
    }
    Poly {
        pts,
        corner,
        node,
        closed,
    }
}

/// A line cut into dashes (on, off … repeated; an odd list is doubled as SVG does).
fn dashed(p: &Poly, pattern: &[f64]) -> Vec<Poly> {
    let mut pat: Vec<f64> = pattern.to_vec();
    if pattern.len() % 2 == 1 {
        pat.extend_from_slice(pattern);
    }
    let total: f64 = pat.iter().fold(0.0, |s, &x| s + x);
    if !(total > 0.0) || pat.iter().any(|&x| x < 0.0) {
        return vec![p.clone()];
    }
    let (pts, corner, node) = if p.closed {
        let mut pts = p.pts.clone();
        pts.push(p.pts[0]);
        let mut corner = p.corner.clone();
        corner.push(p.corner.first().copied().unwrap_or(false));
        let mut node = p.node.clone();
        node.push(p.node.first().copied().unwrap_or(false));
        (pts, corner, node)
    } else {
        (p.pts.clone(), p.corner.clone(), p.node.clone())
    };
    let mut out: Vec<Poly> = Vec::new();
    let mut k = 0;
    let mut left = pat[0];
    let mut on = true;
    let start = |q: Pt| Poly {
        pts: vec![q],
        corner: vec![false],
        node: vec![true],
        closed: false,
    };
    let mut cur: Option<Poly> = Some(start(pts[0]));
    for i in 1..pts.len() {
        let mut a = pts[i - 1];
        let b = pts[i];
        let mut seg = js_hypot(b[0] - a[0], b[1] - a[1]);
        while seg > left {
            let t = left / seg;
            let q: Pt = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
            match (on, cur.take()) {
                (true, Some(mut c)) => {
                    c.pts.push(q);
                    c.corner.push(false);
                    c.node.push(true);
                    out.push(c);
                }
                _ => cur = Some(start(q)),
            }
            on = !on;
            seg -= left;
            a = q;
            k = (k + 1) % pat.len();
            left = pat[k];
        }
        left -= seg;
        if on && let Some(c) = cur.as_mut() {
            c.pts.push(b);
            c.corner.push(corner.get(i).copied().unwrap_or(false));
            c.node.push(node.get(i).copied().unwrap_or(false));
        }
    }
    if on
        && let Some(c) = cur.take()
        && c.pts.len() > 1
    {
        out.push(c);
    }
    let zero_on = pat[0] == 0.0;
    out.into_iter()
        .filter(|d| d.pts.len() >= 2 || zero_on)
        .collect()
}

/// Collects the stroke's pieces (all counter-clockwise) into one source.
#[derive(Default)]
struct Pieces {
    edges: Vec<Edge>,
    points: Vec<Vec2>,
}

impl Pieces {
    fn poly(&mut self, ps: &[Pt]) {
        if ps.len() < 3 {
            return;
        }
        let area = ring_signed_area(ps);
        if area.abs() < 1e-18 {
            return;
        }
        let ring: Vec<Pt> = if area > 0.0 {
            ps.to_vec()
        } else {
            ps.iter().rev().copied().collect()
        };
        let n = ring.len();
        for i in 0..n {
            self.edges.push(Edge::Seg {
                a: zv(ring[i]),
                b: zv(ring[(i + 1) % n]),
            });
        }
        self.points.extend(ring.iter().map(|&p| zv(p)));
    }

    fn disc(&mut self, c: Pt, r: f64) {
        if r > 0.0 {
            self.edges.push(Edge::Arc {
                c: zv(c),
                r,
                a0: 0.0,
                sweep: TAU,
            });
        }
    }

    fn source(self) -> Source {
        Source {
            edges: self.edges,
            points: Some(self.points),
            cut: None,
        }
    }
}

fn join_at(out: &mut Pieces, p: Pt, a: Pt, b: Pt, h: f64, join: Join, miter_limit: Option<f64>) {
    let n1 = normal(a, p);
    let n2 = normal(p, b);
    let turn = cross(sub(p, a), sub(b, p));
    if turn.abs() < 1e-18 && n1[0] * n2[0] + n1[1] * n2[1] > 0.0 {
        return;
    }
    if join == Join::Round {
        out.disc(p, h);
        return;
    }
    // The outer side is the right side when the path turns left.
    let s = if turn > 0.0 { -1.0 } else { 1.0 };
    let pa: Pt = [p[0] + s * n1[0] * h, p[1] + s * n1[1] * h];
    let pb: Pt = [p[0] + s * n2[0] * h, p[1] + s * n2[1] * h];
    if join == Join::Miter {
        let cos_t = n1[0] * n2[0] + n1[1] * n2[1];
        // SVG: miter length / stroke width = 1 / sin(θ/2), θ the angle between the segments.
        let ratio = 1.0 / js_max(1e-18, (1.0 + cos_t) / 2.0).sqrt();
        if ratio <= miter_limit.unwrap_or(4.0) {
            let k = h / (1.0 + cos_t);
            let m: Pt = [
                p[0] + s * (n1[0] + n2[0]) * k,
                p[1] + s * (n1[1] + n2[1]) * k,
            ];
            out.poly(&[p, pa, m, pb]);
            return;
        }
    }
    out.poly(&[p, pa, pb]);
}

fn cap_at(out: &mut Pieces, p: Pt, towards: Pt, h: f64, cap: Cap) {
    if cap == Cap::Round {
        out.disc(p, h);
        return;
    }
    if cap != Cap::Square {
        return;
    }
    let d = unit(sub(p, towards));
    let n: Pt = [-d[1], d[0]];
    let e: Pt = [p[0] + d[0] * h, p[1] + d[1] * h];
    out.poly(&[
        [p[0] + n[0] * h, p[1] + n[1] * h],
        [e[0] + n[0] * h, e[1] + n[1] * h],
        [e[0] - n[0] * h, e[1] - n[1] * h],
        [p[0] - n[0] * h, p[1] - n[1] * h],
    ]);
}

/// A smooth run as ribbons: offset points on both sides (mitred between
/// chords), grown chord by chord while neither side folds back (a turn
/// tighter than the half width). Where one would fold, the ribbon ends and
/// a new one starts, a disc filling the gap. So a gentle curve is one
/// polygon and the overlay stays small.
fn run_at(out: &mut Pieces, pts: &[Pt], h: f64) {
    let k = pts.len();
    if k < 2 {
        return;
    }
    let nrm: Vec<Pt> = (1..k).map(|i| normal(pts[i - 1], pts[i])).collect();
    // Offset direction at point i of a ribbon from s to e (plain normals at its ends).
    let dir = |i: usize, s: usize, e: usize| -> Pt {
        if i == s {
            return nrm[s];
        }
        if i == e {
            return nrm[e - 1];
        }
        let n1 = nrm[i - 1];
        let n2 = nrm[i];
        let d = 1.0 + n1[0] * n2[0] + n1[1] * n2[1];
        if d > 1e-6 {
            [(n1[0] + n2[0]) / d, (n1[1] + n2[1]) / d]
        } else {
            n1
        }
    };
    // Chord i−1 → i keeps going forward on both sides.
    let forward = |i: usize, s: usize, e: usize| -> bool {
        let a = dir(i - 1, s, e);
        let b = dir(i, s, e);
        let dx = pts[i][0] - pts[i - 1][0];
        let dy = pts[i][1] - pts[i - 1][1];
        let l = dx + (b[0] - a[0]) * h;
        let l_y = dy + (b[1] - a[1]) * h;
        let r = dx - (b[0] - a[0]) * h;
        let r_y = dy - (b[1] - a[1]) * h;
        l * dx + l_y * dy > 0.0 && r * dx + r_y * dy > 0.0
    };
    let mut s = 0;
    while s < k - 1 {
        let mut e = s + 1;
        while e < k - 1 && forward(e, s, e + 1) && forward(e + 1, s, e + 1) {
            e += 1;
        }
        let mut ring: Vec<Pt> = Vec::with_capacity(2 * (e - s + 1));
        let mut right: Vec<Pt> = Vec::with_capacity(e - s + 1);
        for i in s..=e {
            let m = dir(i, s, e);
            ring.push([pts[i][0] + m[0] * h, pts[i][1] + m[1] * h]);
            right.push([pts[i][0] - m[0] * h, pts[i][1] - m[1] * h]);
        }
        // Every quad of an unfolded ribbon turns the same way, so its winding counts the quads covering a point.
        ring.extend(right.into_iter().rev());
        out.poly(&ring);
        if e < k - 1 {
            out.disc(pts[e], h);
        }
        s = e;
    }
}

fn stroke_poly(out: &mut Pieces, p: &Poly, h: f64, style: &StrokeStyle) {
    let pts = &p.pts;
    let join_of = |i: usize| -> Join {
        if p.node.get(i).copied().unwrap_or(false) {
            style.join
        } else {
            Join::Round
        }
    };
    let n = pts.len();
    if n == 1 {
        // A zero-length sub-path: SVG draws a dot for round and square caps.
        if style.cap == Cap::Round {
            out.disc(pts[0], h);
        } else if style.cap == Cap::Square {
            out.poly(&[
                [pts[0][0] - h, pts[0][1] - h],
                [pts[0][0] + h, pts[0][1] - h],
                [pts[0][0] + h, pts[0][1] + h],
                [pts[0][0] - h, pts[0][1] + h],
            ]);
        }
        return;
    }
    let corners: Vec<usize> = (0..n)
        .filter(|&i| {
            p.corner.get(i).copied().unwrap_or(false) && (p.closed || (i > 0 && i < n - 1))
        })
        .collect();
    if p.closed {
        if corners.is_empty() {
            // A smooth loop: one run all round, closed by repeating the start.
            let mut run = pts.clone();
            run.push(pts[0]);
            run.push(pts[1 % n]);
            run_at(out, &run, h);
            return;
        }
        for c in 0..corners.len() {
            let i0 = corners[c];
            let i1 = if c == corners.len() - 1 {
                corners[0] + n
            } else {
                corners[c + 1]
            };
            let run: Vec<Pt> = (i0..=i1).map(|i| pts[i % n]).collect();
            run_at(out, &run, h);
            join_at(
                out,
                pts[i0],
                pts[(i0 + n - 1) % n],
                pts[(i0 + 1) % n],
                h,
                join_of(i0),
                style.miter_limit,
            );
        }
        return;
    }
    let mut cuts = vec![0];
    cuts.extend(corners.iter().copied());
    cuts.push(n - 1);
    for c in 0..cuts.len() - 1 {
        run_at(out, &pts[cuts[c]..=cuts[c + 1]], h);
    }
    for &i in &corners {
        join_at(
            out,
            pts[i],
            pts[i - 1],
            pts[i + 1],
            h,
            join_of(i),
            style.miter_limit,
        );
    }
    cap_at(out, pts[0], pts[1], h, style.cap);
    cap_at(out, pts[n - 1], pts[n - 2], h, style.cap);
}

/// Pieces of the stroke of these sub-paths (one overlay source).
fn stroke_source(subs: &[SubPath], st: &StrokeStyle, tol: f64) -> Source {
    let mut out = Pieces::default();
    let h = st.width / 2.0;
    if !(h > 0.0) {
        return out.source();
    }
    for sp in subs {
        let p = poly_of(sp, tol);
        if p.pts.is_empty() {
            continue;
        }
        let parts = match &st.dash {
            Some(d) if !d.is_empty() && d.iter().any(|&x| x > 0.0) => dashed(&p, d),
            _ => vec![p],
        };
        for part in &parts {
            stroke_poly(&mut out, part, h, st);
        }
    }
    out.source()
}

/// Fit tolerance for outlines: a few times the flattening's (so a quarter
/// circle fits one cubic), never more than a small part of the stroke width
/// or offset (`size`), where a wobble would show.
fn fit_tol_for(tol: f64, size: f64) -> f64 {
    js_min(tol * 8.0, js_max(size * 0.02, tol * 2.0))
}

/// The outline of a stroke as closed sub-paths (the area the stroke paints).
pub fn stroke_outline(subs: &[SubPath], st: &StrokeStyle) -> Vec<SubPath> {
    let extent = extent_of(&[subs]) + st.width;
    // Relative to the width (the outline's detail), never coarser than a thousandth of the drawing.
    let tol = js_min(extent * 1e-3, js_max(st.width * 0.004, tol_for(extent)));
    let areas = overlay_z(&[stroke_source(subs, st, tol)], Rule::First);
    areas_to_sub_paths(&areas, None, fit_tol_for(tol, st.width))
}

/// The fill region grown (d > 0) or shrunk (d < 0) by |d|: its outline's
/// stroke of width 2|d| added or taken away. Round joins round the corners
/// that open up (Inkscape's outset), mitre joins keep them sharp.
pub fn offset_region(input: &RegionInput, d: f64, join: Join) -> Vec<SubPath> {
    if d == 0.0 || d.is_nan() {
        return input
            .subs
            .iter()
            .map(|sp| SubPath {
                closed: sp.closed,
                nodes: sp.nodes.clone(),
            })
            .collect();
    }
    let extent = extent_of(&[&input.subs]) + 2.0 * d.abs();
    let tol = js_min(tol_for(extent), js_max(d.abs() * 0.01, 1e-7));
    let mut tr = Tracer::new(tol, extent);
    let src = region_source(input, &mut tr);
    let clean = overlay_z(&[src], Rule::First);
    if clean.is_empty() {
        return Vec::new();
    }
    // The clean outline, with samples inside curves told apart from real corners.
    let mut band = Pieces::default();
    let st = StrokeStyle {
        width: 2.0 * d.abs(),
        cap: Cap::Butt,
        join,
        miter_limit: Some(8.0),
        dash: None,
    };
    for r in clean
        .iter()
        .flat_map(|a| std::iter::once(&a.outer).chain(a.holes.iter()))
    {
        let pts: Vec<Pt> = r.pts.iter().map(|q| [q.x, q.y]).collect();
        let nodes: Vec<bool> = pts.iter().map(|&q| !tr.is_sample(q)).collect();
        // Input nodes on a curve turn by the sampling angle: only clear turns get the join there.
        let poly = with_corners(pts, nodes, true, (8.0 * PI) / 180.0);
        stroke_poly(&mut band, &poly, d.abs(), &st);
    }
    let grown = overlay_z(
        &[area_source_z(&clean), band.source()],
        if d > 0.0 {
            Rule::Any
        } else {
            Rule::FirstNotOthers
        },
    );
    areas_to_sub_paths(&grown, Some(&tr), fit_tol_for(tol, 2.0 * d.abs()))
}

/// A copy of a node (the offset of 0 gives the input back).
pub fn copy_node(n: &PathNode) -> PathNode {
    n.clone()
}
