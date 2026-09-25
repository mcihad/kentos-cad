//! Trace bitmap (`apps/web/src/style/svg/trace.ts`; Inkscape's,
//! simplified): a raster picture into filled paths with holes, for
//! redrawing the regulation's raster pictograms. Brightness threshold → ink
//! mask; marching squares give the outlines on the pixel lattice with ink
//! always on the same side, so outer rings and holes differ by the sign of
//! their area; rings are nested into outers with their holes and specks
//! below an area go; Douglas–Peucker picks the corners (chamfered pixel
//! corners are recovered first) and the runs between them are fitted with
//! cubics (`fit.rs`). Coordinates are pixels from the image's top-left corner.

use std::collections::HashMap;

use kentos_geometry_core::api::json::{FromJson, Json, read_field};
use kentos_geometry_core::jsmath::{
    PI, atan2, cos, js_cmp, js_hypot, js_max, js_min, js_round, or, sin, stable_sort,
};

use crate::fit::fit_run;
use crate::shape::{PathNode, SubPath};

pub type Point = [f64; 2];
pub type Ring = Vec<Point>;

#[derive(Clone, Debug, PartialEq)]
pub struct TraceOptions {
    /// 0–255: pixels darker than this are ink.
    pub threshold: f64,
    /// Light pixels are ink instead.
    pub invert: bool,
    /// Specks and pinholes smaller than this (px²) are dropped.
    pub speckle: f64,
    /// Douglas–Peucker tolerance (px).
    pub tolerance: f64,
    /// Turns sharper than this (degrees) stay corners.
    pub corner: f64,
    /// 0: straight edges only; above 0: fitted curves, looser as it grows to 1.
    pub smooth: f64,
}

impl FromJson for TraceOptions {
    /// Every option given: the page fills in its defaults (`TRACE_DEFAULTS`, which its panel shows).
    fn from_json(v: &Json) -> Result<TraceOptions, String> {
        Ok(TraceOptions {
            threshold: read_field(v, "threshold")?,
            invert: crate::shape::truthy(v.get("invert")),
            speckle: read_field(v, "speckle")?,
            tolerance: read_field(v, "tolerance")?,
            corner: read_field(v, "corner")?,
            smooth: read_field(v, "smooth")?,
        })
    }
}

/// A picture's pixels, RGBA row by row, as the page has them (bytes, or any numbers).
pub trait Pixels {
    /// How many values there are (`data.length`).
    fn count(&self) -> usize;
    fn at(&self, i: usize) -> f64;
}

impl Pixels for [u8] {
    fn count(&self) -> usize {
        self.len()
    }
    fn at(&self, i: usize) -> f64 {
        self[i] as f64
    }
}

impl Pixels for [f64] {
    fn count(&self) -> usize {
        self.len()
    }
    fn at(&self, i: usize) -> f64 {
        self[i]
    }
}

/// Ink (1) where the pixel, laid on white paper, is darker than the threshold.
pub fn ink_mask<P: Pixels + ?Sized>(
    width: usize,
    height: usize,
    d: &P,
    threshold: f64,
    invert: bool,
) -> Vec<u8> {
    let n = width * height;
    // `d[k]` past the data reads undefined: NaN in arithmetic, 255 for a missing alpha.
    let get = |k: usize| if k < d.count() { d.at(k) } else { f64::NAN };
    (0..n)
        .map(|i| {
            let a = (if i * 4 + 3 < d.count() {
                d.at(i * 4 + 3)
            } else {
                255.0
            }) / 255.0;
            let y = 0.299 * get(i * 4) + 0.587 * get(i * 4 + 1) + 0.114 * get(i * 4 + 2);
            let lum = 255.0 - a * (255.0 - y);
            u8::from((lum < threshold) != invert)
        })
        .collect()
}

/// Outlines of the ink by marching squares on pixel centres: every ring
/// closes, ink lies on the same side of every segment (outer rings have a
/// positive shoelace area in image coordinates, holes a negative one), and
/// diagonal pixels of ink connect.
pub fn trace_contours(mask: &[u8], w: usize, h: usize) -> Vec<Ring> {
    let at = |i: usize, j: usize| -> u8 {
        if i >= 1 && j >= 1 && i <= w && j <= h {
            mask.get((j - 1) * w + (i - 1)).copied().unwrap_or(0)
        } else {
            0
        }
    };
    // Crossing points in doubled coordinates (2x + 1, 2y + 1) as one number.
    let s = 2 * h + 4;
    let key = |x2: usize, y2: usize| x2 * s + y2;
    // A Map: successors in the order their crossings were first seen.
    let mut order: Vec<(usize, Option<usize>)> = Vec::new();
    let mut slot: HashMap<usize, usize> = HashMap::new();
    let mut seg = |p: usize, q: usize| match slot.get(&p) {
        Some(&k) => order[k].1 = Some(q),
        None => {
            slot.insert(p, order.len());
            order.push((p, Some(q)));
        }
    };
    for j in 0..=h {
        for i in 0..=w {
            let tl = at(i, j);
            let tr = at(i + 1, j);
            let br = at(i + 1, j + 1);
            let bl = at(i, j + 1);
            let code =
                (u32::from(tl) << 3) | (u32::from(tr) << 2) | (u32::from(br) << 1) | u32::from(bl);
            if code == 0 || code == 15 {
                continue;
            }
            let top = key(2 * i + 1, 2 * j);
            let right = key(2 * i + 2, 2 * j + 1);
            let bottom = key(2 * i + 1, 2 * j + 2);
            let left = key(2 * i, 2 * j + 1);
            // Each case as (from, to) with the ink on the positive side; saddles connect the ink.
            match code {
                1 => seg(left, bottom),
                2 => seg(bottom, right),
                3 => seg(left, right),
                4 => seg(right, top),
                5 => {
                    seg(left, top);
                    seg(right, bottom);
                }
                6 => seg(bottom, top),
                7 => seg(left, top),
                8 => seg(top, left),
                9 => seg(top, bottom),
                10 => {
                    seg(top, right);
                    seg(bottom, left);
                }
                11 => seg(top, right),
                12 => seg(right, left),
                13 => seg(right, bottom),
                14 => seg(bottom, left),
                // A mask value other than 0 or 1 makes no case (the TypeScript's switch had no default).
                _ => {}
            }
        }
    }
    // Walk the chains; a visited crossing is deleted (the Map shrinks as it is walked).
    let mut rings = Vec::new();
    for start_slot in 0..order.len() {
        let (start, alive) = order[start_slot];
        if alive.is_none() {
            continue;
        }
        let mut ring: Ring = Vec::new();
        let mut k = Some(start);
        while let Some(cur) = k {
            ring.push([
                ((cur / s) as f64 - 1.0) / 2.0,
                ((cur % s) as f64 - 1.0) / 2.0,
            ]);
            let next = match slot.get(&cur) {
                Some(&sl) => order[sl].1.take(),
                None => None,
            };
            if next == Some(start) {
                break;
            }
            k = next;
        }
        if ring.len() >= 3 {
            rings.push(ring);
        }
    }
    rings
}

/// Signed shoelace area (image coordinates, y down): outer rings positive.
pub fn ring_area(r: &[Point]) -> f64 {
    let mut a = 0.0;
    let n = r.len();
    for i in 0..n {
        let p = r[i];
        let q = r[(i + 1) % n];
        a += p[0] * q[1] - q[0] * p[1];
    }
    a / 2.0
}

fn inside(p: Point, r: &Ring) -> bool {
    let mut hit = false;
    let n = r.len();
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let [xi, yi] = r[i];
        let [xj, yj] = r[j];
        if (yi > p[1]) != (yj > p[1]) && p[0] < ((xj - xi) * (p[1] - yi)) / (yj - yi) + xi {
            hit = !hit;
        }
        j = i;
    }
    hit
}

/// Outer rings with the holes they hold.
pub struct Nested {
    pub shapes: Vec<(Ring, Vec<Ring>)>,
    pub removed: usize,
}

/// Outer rings with the holes they hold (the smallest outer around each hole); rings under `min_area` go.
pub fn nest_rings(rings: Vec<Ring>, min_area: f64) -> Nested {
    let mut removed = 0;
    #[derive(Clone)]
    struct Outer {
        r: Ring,
        a: f64,
        bx: [f64; 4],
        holes: Vec<Ring>,
    }
    let mut outers: Vec<Outer> = Vec::new();
    let mut holes: Vec<(Ring, f64)> = Vec::new();
    let box_of = |r: &Ring| {
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for &[x, y] in r {
            b[0] = js_min(b[0], x);
            b[1] = js_min(b[1], y);
            b[2] = js_max(b[2], x);
            b[3] = js_max(b[3], y);
        }
        b
    };
    for r in rings {
        let a = ring_area(&r);
        if a.abs() < js_max(min_area, 1e-9) {
            removed += 1;
            continue;
        }
        if a > 0.0 {
            let bx = box_of(&r);
            outers.push(Outer {
                r,
                a,
                bx,
                holes: Vec::new(),
            });
        } else {
            holes.push((r, -a));
        }
    }
    stable_sort(&mut outers, &mut |p, q| js_cmp(p.a - q.a, 0.0));
    for (hr, ha) in holes {
        let p = hr[0];
        let owner = outers.iter_mut().find(|o| {
            o.a > ha
                && p[0] >= o.bx[0]
                && p[0] <= o.bx[2]
                && p[1] >= o.bx[1]
                && p[1] <= o.bx[3]
                && inside(p, &o.r)
        });
        match owner {
            Some(o) => o.holes.push(hr),
            None => removed += 1,
        }
    }
    stable_sort(&mut outers, &mut |p, q| js_cmp(q.a - p.a, 0.0));
    Nested {
        shapes: outers.into_iter().map(|o| (o.r, o.holes)).collect(),
        removed,
    }
}

fn seg_dist(p: Point, a: Point, b: Point) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let l2 = dx * dx + dy * dy;
    let t = if l2 != 0.0 && !l2.is_nan() {
        js_max(
            0.0,
            js_min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2),
        )
    } else {
        0.0
    };
    js_hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy)
}

/// Douglas–Peucker on an open chain `a..b` of a ring (both ends kept): the kept indices.
fn dp_chain(r: &Ring, idx: &[usize], tol: f64) -> Vec<usize> {
    if idx.len() < 3 {
        return idx.to_vec();
    }
    let mut keep = vec![false; idx.len()];
    keep[0] = true;
    keep[idx.len() - 1] = true;
    let mut stack: Vec<(usize, usize)> = vec![(0, idx.len() - 1)];
    while let Some((s, e)) = stack.pop() {
        let mut best: Option<usize> = None;
        let mut best_d = tol;
        for i in s + 1..e {
            let d = seg_dist(r[idx[i]], r[idx[s]], r[idx[e]]);
            if d > best_d {
                best_d = d;
                best = Some(i);
            }
        }
        if let Some(b) = best {
            keep[b] = true;
            stack.push((s, b));
            stack.push((b, e));
        }
    }
    idx.iter()
        .zip(&keep)
        .filter(|(_, k)| **k)
        .map(|(&i, _)| i)
        .collect()
}

/// Douglas–Peucker on a closed ring, anchored at two extreme points (real corners, not the arbitrary start): kept indices in ring order.
pub fn simplify_indices(r: &Ring, tol: f64) -> Vec<usize> {
    let n = r.len();
    if n < 4 {
        return (0..n).collect();
    }
    let mut i0 = 0;
    for i in 1..n {
        if r[i][0] + r[i][1] < r[i0][0] + r[i0][1] {
            i0 = i;
        }
    }
    let mut i1 = i0;
    let mut far = -1.0;
    for i in 0..n {
        let d = js_hypot(r[i][0] - r[i0][0], r[i][1] - r[i0][1]);
        if d > far {
            far = d;
            i1 = i;
        }
    }
    let chain = |a: usize, b: usize| {
        let mut out = Vec::new();
        let mut i = a;
        loop {
            out.push(i);
            if i == b {
                break;
            }
            i = (i + 1) % n;
        }
        out
    };
    let t = js_max(tol, 1e-6);
    let mut kept = dp_chain(r, &chain(i0, i1), t);
    let back = dp_chain(r, &chain(i1, i0), t);
    if back.len() > 2 {
        kept.extend_from_slice(&back[1..back.len() - 1]);
    }
    kept.sort_unstable();
    kept
}

pub fn simplify_ring(r: &Ring, tol: f64) -> Ring {
    simplify_indices(r, tol).into_iter().map(|i| r[i]).collect()
}

fn turn(a: Point, b: Point, c: Point) -> f64 {
    let u = atan2(b[1] - a[1], b[0] - a[0]);
    let v = atan2(c[1] - b[1], c[0] - b[0]);
    atan2(sin(v - u), cos(v - u)).abs()
}

/// Points where the ring goes straight on are dropped (a run along pixel edges or diagonals becomes one edge).
pub fn compact_ring(r: &Ring) -> Ring {
    let n = r.len();
    let mut out = Vec::new();
    for i in 0..n {
        let p = r[(i + n - 1) % n];
        let a = r[i];
        let q = r[(i + 1) % n];
        if ((a[0] - p[0]) * (q[1] - a[1]) - (a[1] - p[1]) * (q[0] - a[0])).abs() > 1e-12 {
            out.push(a);
        }
    }
    if out.len() >= 3 { out } else { r.clone() }
}

/// Sharp corners back from the pixel lattice: marching squares cut a
/// square corner with one short diagonal. Where such an edge joins two
/// straight runs of at least `min_run` that turn by `corner_rad` or more
/// together, its ends become the runs' crossing. Staircases of curves
/// (short runs) are left alone.
pub fn recover_corners(r: &Ring, min_run: f64, corner_rad: f64) -> Ring {
    let n = r.len();
    if n < 4 {
        return r.clone();
    }
    let mut out: Ring = Vec::new();
    let mut skip = vec![false; n];
    for i in 0..n {
        if skip[i] {
            continue;
        }
        let a = r[i];
        let b = r[(i + 1) % n];
        let p = r[(i + n - 1) % n];
        let q = r[(i + 2) % n];
        let len = js_hypot(b[0] - a[0], b[1] - a[1]);
        let d1 = [a[0] - p[0], a[1] - p[1]];
        let d2 = [q[0] - b[0], q[1] - b[1]];
        let wraps = (i + 1) % n == 0 && out.is_empty();
        if len < 0.75
            && js_hypot(d1[0], d1[1]) >= min_run
            && js_hypot(d2[0], d2[1]) >= min_run
            && !wraps
            && !skip[(i + 1) % n]
        {
            let den = d1[0] * d2[1] - d1[1] * d2[0];
            let total = atan2(den, d1[0] * d2[0] + d1[1] * d2[1]).abs();
            if den.abs() > 1e-12 && total >= corner_rad {
                let t = ((b[0] - a[0]) * d2[1] - (b[1] - a[1]) * d2[0]) / den;
                out.push([a[0] + d1[0] * t, a[1] + d1[1] * t]);
                skip[(i + 1) % n] = true;
                // The ring's first point may have been this corner's second end.
                if (i + 1) % n == 0 && !out.is_empty() {
                    out.remove(0);
                }
                continue;
            }
        }
        out.push(a);
    }
    if out.len() >= 3 { out } else { r.clone() }
}

/// Neighbour averaging of the points that are not corners (staircase noise out of curves).
fn smooth_ring(r: &Ring, fixed: &[bool], passes: usize) -> Ring {
    let mut cur = r.clone();
    let n = r.len();
    for _ in 0..passes {
        cur = cur
            .iter()
            .enumerate()
            .map(|(i, v)| {
                if fixed[i] {
                    *v
                } else {
                    let (p, q) = (cur[(i + n - 1) % n], cur[(i + 1) % n]);
                    [
                        (p[0] + 2.0 * v[0] + q[0]) / 4.0,
                        (p[1] + 2.0 * v[1] + q[1]) / 4.0,
                    ]
                }
            })
            .collect();
    }
    cur
}

fn round(v: f64) -> f64 {
    js_round(v * 1000.0) / 1000.0
}

fn round_node(n: &PathNode) -> PathNode {
    PathNode {
        x: round(n.x),
        y: round(n.y),
        in_: n.in_.map(|h| [round(h[0]), round(h[1])]),
        out: n.out.map(|h| [round(h[0]), round(h[1])]),
        ty: None,
    }
}

fn unit(x: f64, y: f64) -> Point {
    let l = or(js_hypot(x, y), 1.0);
    [x / l, y / l]
}

/// One traced ring as a closed path. Douglas–Peucker picks the vertices;
/// those that turn by `corner` degrees or more are corners. Without curves
/// the vertices are the path. With curves the ring's own points (lightly
/// smoothed) are fitted with cubics within the tolerance (loosened by
/// `smooth`), run by run: runs end at corners and, on long bends, about
/// every quarter turn, where both runs share one tangent so the join stays
/// smooth.
pub fn fit_traced_ring(dense: &Ring, o: &TraceOptions) -> Option<SubPath> {
    let corner_rad = (o.corner * PI) / 180.0;
    let r = recover_corners(&compact_ring(dense), 1.5, js_min(corner_rad, PI / 3.0));
    let idx = simplify_indices(&r, o.tolerance);
    // Too small for the tolerance (a speck kept on purpose): its own outline.
    let kept: Vec<usize> = if idx.len() >= 3 {
        idx
    } else {
        (0..r.len()).collect()
    };
    if kept.len() < 3 {
        return None;
    }
    if o.smooth <= 0.0 {
        return Some(SubPath {
            closed: true,
            nodes: kept
                .iter()
                .map(|&i| PathNode::at(round(r[i][0]), round(r[i][1])))
                .collect(),
        });
    }
    // Corners are judged on a coarser outline: small tolerances keep staircase steps that are no corners.
    let coarse = if o.tolerance >= 1.2 {
        kept.clone()
    } else {
        simplify_indices(&r, 1.2)
    };
    let cs = if coarse.len() >= 3 { coarse } else { kept };
    let m = cs.len();
    let n = r.len();
    let turns: Vec<f64> = (0..m)
        .map(|k| turn(r[cs[(k + m - 1) % m]], r[cs[k]], r[cs[(k + 1) % m]]))
        .collect();
    // The point where the ring about 2.5 px away lies, backwards or forwards.
    let away = |i: usize, forward: bool| -> Point {
        let mut j = i;
        for _ in 0..n {
            j = if forward {
                (j + 1) % n
            } else {
                (j + n - 1) % n
            };
            if js_hypot(r[j][0] - r[i][0], r[j][1] - r[i][1]) >= 2.5 {
                break;
            }
        }
        r[j]
    };
    // A corner turns sharply both on the outline and right at the point (a coarse outline of a bend does not).
    let mut corner = vec![false; n];
    for (k, &i) in cs.iter().enumerate() {
        if turns[k] >= corner_rad && turn(away(i, false), r[i], away(i, true)) >= corner_rad * 0.8 {
            corner[i] = true;
        }
    }
    // Soft splits: a quarter turn gathered since the last split.
    let mut splits: Vec<usize> = Vec::new();
    let start_k = cs.iter().position(|&i| corner[i]).unwrap_or(0);
    let mut gathered = 0.0;
    for j in 0..m {
        let k = (start_k + j) % m;
        let i = cs[k];
        if j == 0 || corner[i] || gathered + turns[k] > PI / 2.0 {
            splits.push(i);
            gathered = 0.0;
        } else {
            gathered += turns[k];
        }
    }
    // One light pass takes the staircase out; more would shrink the shape.
    let pts = smooth_ring(&r, &corner, 1);
    // The tangent at a smooth split, over about 2 px either side.
    let tangent = |i: usize| -> Point {
        let mut a = i;
        let mut b = i;
        let mut d = 0.0;
        let mut k = 0;
        while k < n && d < 2.0 {
            a = (a + n - 1) % n;
            d = js_hypot(pts[a][0] - pts[i][0], pts[a][1] - pts[i][1]);
            k += 1;
        }
        d = 0.0;
        k = 0;
        while k < n && d < 2.0 {
            b = (b + 1) % n;
            d = js_hypot(pts[b][0] - pts[i][0], pts[b][1] - pts[i][1]);
            k += 1;
        }
        unit(pts[b][0] - pts[a][0], pts[b][1] - pts[a][1])
    };
    // Smoothing loosens the fit: fewer nodes, rounder curves.
    let tol = js_max(o.tolerance, 0.05) * (0.6 + 0.8 * js_min(1.0, o.smooth));
    let mut nodes: Vec<PathNode> = Vec::new();
    for s in 0..splits.len() {
        let i0 = splits[s];
        let i1 = splits[(s + 1) % splits.len()];
        let mut run: Vec<Point> = Vec::new();
        let mut i = i0;
        loop {
            run.push(pts[i]);
            if i == i1 && run.len() > 1 {
                break;
            }
            i = (i + 1) % n;
        }
        let t0 = (!corner[i0]).then(|| tangent(i0));
        let t1 = (!corner[i1]).then(|| tangent(i1));
        if nodes.is_empty() {
            nodes.push(PathNode::at(pts[i0][0], pts[i0][1]));
        }
        let mut curves = fit_run(&run, tol, t0, t1.map(|t| [-t[0], -t[1]]));
        if curves.len() == 1
            && curves[0].is_none()
            && (t0.is_some() || t1.is_some())
            && run.len() > 2
        {
            // Straight within the tolerance, but a smooth join must not kink: one cubic along the tangents.
            let a = run[0];
            let b = run[run.len() - 1];
            let k = js_hypot(b[0] - a[0], b[1] - a[1]) / 3.0;
            let u = t0.unwrap_or_else(|| unit(b[0] - a[0], b[1] - a[1]));
            let v = t1.unwrap_or_else(|| unit(b[0] - a[0], b[1] - a[1]));
            curves = vec![Some([
                a,
                [a[0] + u[0] * k, a[1] + u[1] * k],
                [b[0] - v[0] * k, b[1] - v[1] * k],
                b,
            ])];
        }
        if curves.is_empty() {
            curves.push(None);
        }
        for c in curves {
            match c {
                Some(c) => {
                    if let Some(cur) = nodes.last_mut() {
                        cur.out = Some([c[1][0], c[1][1]]);
                    }
                    nodes.push(PathNode {
                        in_: Some([c[2][0], c[2][1]]),
                        ..PathNode::at(c[3][0], c[3][1])
                    });
                }
                None => nodes.push(PathNode::at(pts[i1][0], pts[i1][1])),
            }
        }
    }
    // Back at the first split: its incoming handle moves there.
    if let Some(last) = nodes.pop()
        && let Some(h) = last.in_
        && let Some(first) = nodes.first_mut()
    {
        first.in_ = Some(h);
    }
    (nodes.len() >= 2).then(|| SubPath {
        closed: true,
        nodes: nodes.iter().map(round_node).collect(),
    })
}

/// A traced shape: its outline and holes.
pub struct TracedShape {
    pub outer: SubPath,
    pub holes: Vec<SubPath>,
}

kentos_geometry_core::json_struct!(out TracedShape { outer, holes });

pub struct TraceResult {
    pub shapes: Vec<TracedShape>,
    pub nodes: usize,
    pub holes: usize,
    /// Specks and pinholes dropped.
    pub removed: usize,
}

kentos_geometry_core::json_struct!(out TraceResult { shapes, nodes, holes, removed });

/// The whole trace: mask, outlines, nesting, simplification and curves.
pub fn trace_bitmap<P: Pixels + ?Sized>(
    width: usize,
    height: usize,
    data: &P,
    o: &TraceOptions,
) -> TraceResult {
    let mask = ink_mask(width, height, data, o.threshold, o.invert);
    let nested = nest_rings(trace_contours(&mask, width, height), o.speckle);
    let mut out = Vec::new();
    let mut nodes = 0;
    let mut holes = 0;
    for (outer, hs) in &nested.shapes {
        let Some(outer) = fit_traced_ring(outer, o) else {
            continue;
        };
        let hs: Vec<SubPath> = hs.iter().filter_map(|h| fit_traced_ring(h, o)).collect();
        nodes += outer.nodes.len() + hs.iter().map(|h| h.nodes.len()).sum::<usize>();
        holes += hs.len();
        out.push(TracedShape { outer, holes: hs });
    }
    TraceResult {
        shapes: out,
        nodes,
        holes,
        removed: nested.removed,
    }
}
