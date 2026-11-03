//! Surface drainage (plan/road.md Y16, docs/adr/0914; ROAD-DRAIN-03, 05): where water runs and stands on a terrain TIN,
//! the area that drains to a place, and what a culvert or a ditch carries. Exact on the triangles: a triangle's plane
//! has one steepest way down.
//!
//! - **Water drop** ([`water_drop`], Civil 3D's Water Drop, OpenRoads' Analyze Trace Slope by maximum slope): from a
//!   place, down each triangle's steepest line to the edge it leaves by; where the next triangle runs back into that
//!   edge (a valley) along the edge to its lower end; at a corner the steepest way out, into a triangle whose steepest
//!   line leaves the corner inside it or along a falling edge. It ends at a low point (no way down), on a flat
//!   triangle, or at the surface's edge.
//! - **Ponds** ([`ponds`], OpenRoads' Analyze Pond): the corners' spill levels by a priority flood from the surface's
//!   edge (each corner's level is the lowest, over the ways out to the edge, of the highest corner passed); a pond is a
//!   connected set of corners below their level: its bottom, its level, its depth, and the area and volume of water in
//!   its triangles (each triangle clipped at the level; the depth is linear, so its integral is the area times the
//!   depth at the clipped polygon's centroid). Shallower than a given depth are left out (OpenRoads' minimum depth).
//! - **Catchment** ([`catchment`], Civil 3D's Catchment from Surface): the cells of a grid whose centre's water drop
//!   passes within a radius of the discharge point; its area at the grid's resolution, its outline, and the flow path
//!   from the hydraulically most distant cell (the longest drop to the discharge point).
//! - **Capacity** ([`manning`]): Manning's Q = A R^(2/3) S^(1/2) / n of a full pipe, a full box or a ditch's trapezoid,
//!   SI units; and the Rational method's peak flow Q = C i A / 3.6 (A km², i mm/h, Q m³/s). The roughness, the
//!   runoff coefficient and the rainfall intensity are the user's.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::surface::Surface;

/// The kept triangles' neighbours: for each triangle and edge (corner i to i + 1) the triangle across, if any; each
/// corner's triangles and neighbouring corners.
pub struct Tin<'s> {
    pub surface: &'s Surface,
    across: Vec<[Option<u32>; 3]>,
    around: Vec<Vec<u32>>,
    corners: Vec<Vec<u32>>,
    /// The corners on the surface's edge.
    edge: Vec<bool>,
}

impl<'s> Tin<'s> {
    pub fn new(surface: &'s Surface) -> Tin<'s> {
        let n = surface.vertices.len();
        let mut by_edge: HashMap<(u32, u32), (u32, usize)> = HashMap::new();
        let mut across = vec![[None; 3]; surface.triangles.len()];
        let mut around = vec![Vec::new(); n];
        let mut corners: Vec<HashSet<u32>> = vec![HashSet::new(); n];
        for (t, tri) in surface.triangles.iter().enumerate() {
            for i in 0..3 {
                let (a, b) = (tri[i], tri[(i + 1) % 3]);
                around[a as usize].push(t as u32);
                corners[a as usize].insert(b);
                corners[b as usize].insert(a);
                let key = (a.min(b), a.max(b));
                match by_edge.remove(&key) {
                    Some((u, j)) => {
                        across[t][i] = Some(u);
                        across[u as usize][j] = Some(t as u32);
                    }
                    None => {
                        by_edge.insert(key, (t as u32, i));
                    }
                }
            }
        }
        let mut edge = vec![false; n];
        for (a, b) in by_edge.keys() {
            edge[*a as usize] = true;
            edge[*b as usize] = true;
        }
        let corners = corners
            .into_iter()
            .map(|s| {
                let mut v: Vec<u32> = s.into_iter().collect();
                v.sort_unstable();
                v
            })
            .collect();
        Tin { surface, across, around, corners, edge }
    }

    fn v(&self, i: u32) -> [f64; 3] {
        self.surface.vertices[i as usize]
    }

    /// A triangle's steepest way down in plan (unit) and its slope; none on a flat triangle.
    fn down(&self, t: u32) -> Option<([f64; 2], f64)> {
        let [a, b, c] = self.surface.triangles[t as usize].map(|i| self.v(i));
        let (ux, uy, uz) = (b[0] - a[0], b[1] - a[1], b[2] - a[2]);
        let (vx, vy, vz) = (c[0] - a[0], c[1] - a[1], c[2] - a[2]);
        let det = ux * vy - uy * vx;
        if det == 0.0 {
            return None;
        }
        let gx = (uz * vy - vz * uy) / det;
        let gy = (vz * ux - uz * vx) / det;
        let g = gx.hypot(gy);
        if g.is_nan() || g <= 1e-12 {
            return None;
        }
        Some(([-gx / g, -gy / g], g))
    }

    /// The height on triangle `t`'s plane at (x, y).
    fn z_on(&self, t: u32, x: f64, y: f64) -> f64 {
        let [a, b, c] = self.surface.triangles[t as usize].map(|i| self.v(i));
        let d = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
        let l1 = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / d;
        let l2 = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / d;
        l1 * a[2] + l2 * b[2] + (1.0 - l1 - l2) * c[2]
    }

    /// The kept triangle at (x, y).
    fn triangle_at(&self, x: f64, y: f64) -> Option<u32> {
        self.surface.height(x, y).triangle
    }
}

fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

/// Why a water drop ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropEnd {
    /// At a corner with no way down.
    LowPoint,
    /// On a flat triangle or a level valley.
    Flat,
    /// At the surface's edge.
    Edge,
    /// Off the surface at its start.
    Outside,
    /// Past the step limit (a guard).
    Steps,
}

/// A water drop's path: its places with their heights, its plan length, and why it ended.
#[derive(Clone, Debug, PartialEq)]
pub struct Drop {
    pub path: Vec<[f64; 3]>,
    pub length: f64,
    pub end: DropEnd,
}

/// Where a drop is: inside a triangle, or at a corner.
enum At {
    In(u32, [f64; 2]),
    Corner(u32),
}

const STEPS: usize = 200_000;

/// The steepest way down from a place on the surface.
pub fn water_drop(tin: &Tin<'_>, start: [f64; 2]) -> Drop {
    let Some(t0) = tin.triangle_at(start[0], start[1]) else {
        return Drop { path: Vec::new(), length: 0.0, end: DropEnd::Outside };
    };
    let mut path = vec![[start[0], start[1], tin.z_on(t0, start[0], start[1])]];
    let mut at = At::In(t0, start);
    let mut seen: HashSet<u32> = HashSet::new();
    for _ in 0..STEPS {
        match at {
            At::In(t, p) => {
                let Some((d, _)) = tin.down(t) else {
                    return finish(path, DropEnd::Flat);
                };
                let tri = tin.surface.triangles[t as usize];
                // The edge the ray leaves by: the smallest positive step to an edge's line, within the edge.
                let mut best: Option<(f64, usize)> = None;
                for i in 0..3 {
                    let a = tin.v(tri[i]);
                    let b = tin.v(tri[(i + 1) % 3]);
                    let e = [b[0] - a[0], b[1] - a[1]];
                    let den = cross(d, e);
                    if den.abs() < 1e-15 {
                        continue;
                    }
                    let w = [a[0] - p[0], a[1] - p[1]];
                    let s = cross(w, e) / den;
                    let u = cross(w, d) / den;
                    if s > 1e-12 && (-1e-9..=1.0 + 1e-9).contains(&u) && best.is_none_or(|(bs, _)| s < bs) {
                        best = Some((s, i));
                    }
                }
                let Some((s, i)) = best else {
                    // On a corner of the triangle, or on an edge it leaves across: the drop goes on from there.
                    if let Some(&v) = tri.iter().find(|&&v| {
                        let c = tin.v(v);
                        (c[0] - p[0]).hypot(c[1] - p[1]) < 1e-9
                    }) {
                        at = At::Corner(v);
                        continue;
                    }
                    let on = (0..3).find(|&i| {
                        let (a, b) = (tin.v(tri[i]), tin.v(tri[(i + 1) % 3]));
                        let e = [b[0] - a[0], b[1] - a[1]];
                        let len = e[0].hypot(e[1]);
                        len > 0.0 && (cross(e, [p[0] - a[0], p[1] - a[1]]) / len).abs() < 1e-9
                    });
                    let Some(i) = on else {
                        return finish(path, DropEnd::Flat);
                    };
                    match cross_edge(tin, t, i, p) {
                        Ok(next) => {
                            if let At::Corner(v) = next {
                                path.push(tin.v(v));
                            }
                            at = next;
                            continue;
                        }
                        Err(end) => return finish(path, end),
                    }
                };
                let q = [p[0] + s * d[0], p[1] + s * d[1]];
                path.push([q[0], q[1], tin.z_on(t, q[0], q[1])]);
                let (a, b) = (tri[i], tri[(i + 1) % 3]);
                // Leaving by a corner: the drop is at that corner.
                let near = |v: u32| {
                    let c = tin.v(v);
                    (c[0] - q[0]).hypot(c[1] - q[1]) < 1e-9
                };
                if near(a) || near(b) {
                    at = At::Corner(if near(a) { a } else { b });
                    continue;
                }
                let Some(u) = tin.across[t as usize][i] else {
                    return finish(path, DropEnd::Edge);
                };
                // The next triangle runs on away from the edge, or back into it (a valley: along the edge, down).
                let (ea, eb) = (tin.v(a), tin.v(b));
                let edge = [eb[0] - ea[0], eb[1] - ea[1]];
                let into_u = match tin.down(u) {
                    // `u` is on the right of a→b (t is on its left, counter-clockwise).
                    Some((du, _)) => cross(edge, du) < -1e-12,
                    None => false,
                };
                if into_u {
                    at = At::In(u, q);
                } else if (ea[2] - eb[2]).abs() < 1e-12 {
                    return finish(path, DropEnd::Flat);
                } else {
                    let low = if ea[2] < eb[2] { a } else { b };
                    let c = tin.v(low);
                    path.push(c);
                    at = At::Corner(low);
                }
            }
            At::Corner(v) => {
                if !seen.insert(v) {
                    return finish(path, DropEnd::LowPoint);
                }
                let c = tin.v(v);
                // The steepest way out: a triangle whose steepest line leaves the corner inside it, or a falling edge.
                let mut best: Option<(f64, Option<u32>, Option<u32>)> = None;
                for &t in &tin.around[v as usize] {
                    let Some((d, g)) = tin.down(t) else { continue };
                    let tri = tin.surface.triangles[t as usize];
                    let k = tri.iter().position(|x| *x == v).unwrap_or(0);
                    let (p1, p2) = (tin.v(tri[(k + 1) % 3]), tin.v(tri[(k + 2) % 3]));
                    let (e1, e2) = ([p1[0] - c[0], p1[1] - c[1]], [p2[0] - c[0], p2[1] - c[1]]);
                    if cross(e1, d) > 1e-12 && cross(d, e2) > 1e-12 && best.is_none_or(|(bg, _, _)| g > bg) {
                        best = Some((g, Some(t), None));
                    }
                }
                for &w in &tin.corners[v as usize] {
                    let p = tin.v(w);
                    let len = (p[0] - c[0]).hypot(p[1] - c[1]);
                    if len > 0.0 && p[2] < c[2] {
                        let g = (c[2] - p[2]) / len;
                        if best.is_none_or(|(bg, _, _)| g > bg) {
                            best = Some((g, None, Some(w)));
                        }
                    }
                }
                match best {
                    None => {
                        let end = if tin.edge[v as usize] { DropEnd::Edge } else { DropEnd::LowPoint };
                        return finish(path, end);
                    }
                    Some((_, Some(t), _)) => {
                        // A step into the triangle along its steepest line, from just past the corner.
                        let (d, _) = tin.down(t).unwrap_or(([0.0, 0.0], 0.0));
                        let tri = tin.surface.triangles[t as usize];
                        let k = tri.iter().position(|x| *x == v).unwrap_or(0);
                        let (p1, p2) = (tin.v(tri[(k + 1) % 3]), tin.v(tri[(k + 2) % 3]));
                        let e = [p2[0] - p1[0], p2[1] - p1[1]];
                        let den = cross(d, e);
                        let w = [p1[0] - c[0], p1[1] - c[1]];
                        let s = if den.abs() > 1e-15 { cross(w, e) / den } else { 0.0 };
                        let q = [c[0] + s * d[0], c[1] + s * d[1]];
                        path.push([q[0], q[1], tin.z_on(t, q[0], q[1])]);
                        // Past the opposite edge: across it, or the edge's end, or off the surface.
                        let i = (k + 1) % 3;
                        let near = |x: u32| {
                            let p = tin.v(x);
                            (p[0] - q[0]).hypot(p[1] - q[1]) < 1e-9
                        };
                        let (a, b) = (tri[i], tri[(i + 1) % 3]);
                        if near(a) || near(b) {
                            at = At::Corner(if near(a) { a } else { b });
                            continue;
                        }
                        match tin.across[t as usize][i] {
                            None => return finish(path, DropEnd::Edge),
                            Some(u) => {
                                let (ea, eb) = (tin.v(a), tin.v(b));
                                let edge = [eb[0] - ea[0], eb[1] - ea[1]];
                                let into_u = tin.down(u).is_some_and(|(du, _)| cross(edge, du) < -1e-12);
                                if into_u {
                                    at = At::In(u, q);
                                } else if (ea[2] - eb[2]).abs() < 1e-12 {
                                    return finish(path, DropEnd::Flat);
                                } else {
                                    let low = if ea[2] < eb[2] { a } else { b };
                                    path.push(tin.v(low));
                                    at = At::Corner(low);
                                }
                            }
                        }
                    }
                    Some((_, None, Some(w))) => {
                        path.push(tin.v(w));
                        at = At::Corner(w);
                    }
                    Some(_) => return finish(path, DropEnd::Flat),
                }
            }
        }
    }
    finish(path, DropEnd::Steps)
}

/// A drop at `q` on triangle `t`'s edge `i`: on into the triangle across when its steepest line leaves the edge, else
/// (a valley) to the edge's lower end; the surface's edge or a level valley ends it.
fn cross_edge(tin: &Tin<'_>, t: u32, i: usize, q: [f64; 2]) -> Result<At, DropEnd> {
    let tri = tin.surface.triangles[t as usize];
    let (a, b) = (tri[i], tri[(i + 1) % 3]);
    let Some(u) = tin.across[t as usize][i] else {
        return Err(DropEnd::Edge);
    };
    let (ea, eb) = (tin.v(a), tin.v(b));
    let edge = [eb[0] - ea[0], eb[1] - ea[1]];
    if tin.down(u).is_some_and(|(du, _)| cross(edge, du) < -1e-12) {
        return Ok(At::In(u, q));
    }
    if (ea[2] - eb[2]).abs() < 1e-12 {
        return Err(DropEnd::Flat);
    }
    Ok(At::Corner(if ea[2] < eb[2] { a } else { b }))
}

fn finish(mut path: Vec<[f64; 3]>, end: DropEnd) -> Drop {
    path.dedup_by(|a, b| (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-12);
    let length = path.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum();
    Drop { path, length, end }
}

/// A pond: the water standing in a low place up to the level it spills at.
#[derive(Clone, Debug, PartialEq)]
pub struct Pond {
    /// Its lowest corner.
    pub bottom: [f64; 3],
    /// The level it spills at, and the corner it spills over.
    pub level: f64,
    pub outlet: [f64; 3],
    pub depth: f64,
    /// The water's plan area (m²) and volume (m³).
    pub area: f64,
    pub volume: f64,
    /// The corners under the water.
    pub corners: usize,
}

#[derive(PartialEq)]
struct Level(f64, u32);
impl Eq for Level {}
impl PartialOrd for Level {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Level {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.total_cmp(&self.0).then(o.1.cmp(&self.1))
    }
}

/// The corners' spill levels: a priority flood from the surface's edge.
pub fn spill_levels(tin: &Tin<'_>) -> Vec<f64> {
    let n = tin.surface.vertices.len();
    let mut level = vec![f64::INFINITY; n];
    let mut done = vec![false; n];
    let mut heap = BinaryHeap::new();
    for (v, l) in level.iter_mut().enumerate() {
        if tin.edge[v] && !tin.around[v].is_empty() {
            *l = tin.surface.vertices[v][2];
            heap.push(Level(*l, v as u32));
        }
    }
    while let Some(Level(l, v)) = heap.pop() {
        if done[v as usize] {
            continue;
        }
        done[v as usize] = true;
        for &w in &tin.corners[v as usize] {
            let w = w as usize;
            if done[w] {
                continue;
            }
            let lw = l.max(tin.surface.vertices[w][2]);
            if lw < level[w] {
                level[w] = lw;
                heap.push(Level(lw, w as u32));
            }
        }
    }
    level
}

/// The area and the water's volume of a triangle under `level`.
fn under(tin: &Tin<'_>, t: u32, level: f64) -> (f64, f64) {
    let pts = tin.surface.triangles[t as usize].map(|i| tin.v(i));
    let mut poly: Vec<[f64; 3]> = Vec::new();
    for i in 0..3 {
        let (a, b) = (pts[i], pts[(i + 1) % 3]);
        let (ina, inb) = (a[2] <= level, b[2] <= level);
        if ina {
            poly.push(a);
        }
        if ina != inb {
            let s = (level - a[2]) / (b[2] - a[2]);
            poly.push([a[0] + s * (b[0] - a[0]), a[1] + s * (b[1] - a[1]), level]);
        }
    }
    if poly.len() < 3 {
        return (0.0, 0.0);
    }
    let o = poly[0];
    let (mut area2, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let c = (a[0] - o[0]) * (b[1] - o[1]) - (b[0] - o[0]) * (a[1] - o[1]);
        area2 += c;
        cx += (a[0] - o[0] + b[0] - o[0]) * c;
        cy += (a[1] - o[1] + b[1] - o[1]) * c;
    }
    let area = area2.abs() / 2.0;
    if area2 == 0.0 {
        return (0.0, 0.0);
    }
    let (gx, gy) = (o[0] + cx / (3.0 * area2), o[1] + cy / (3.0 * area2));
    (area, area * (level - tin.z_on(t, gx, gy)).max(0.0))
}

/// The ponds at least `min_depth` deep, deepest first.
pub fn ponds(tin: &Tin<'_>, min_depth: f64) -> Vec<Pond> {
    let level = spill_levels(tin);
    let n = level.len();
    let z = |v: usize| tin.surface.vertices[v][2];
    let wet: Vec<bool> = (0..n).map(|v| level[v].is_finite() && level[v] > z(v) + 1e-12).collect();
    let mut group = vec![usize::MAX; n];
    let mut out = Vec::new();
    for start in 0..n {
        if !wet[start] || group[start] != usize::MAX {
            continue;
        }
        let k = out.len();
        let mut stack = vec![start];
        let mut members = Vec::new();
        group[start] = k;
        while let Some(v) = stack.pop() {
            members.push(v);
            for &w in &tin.corners[v] {
                let w = w as usize;
                if wet[w] && group[w] == usize::MAX {
                    group[w] = k;
                    stack.push(w);
                }
            }
        }
        let l = members.iter().map(|&v| level[v]).fold(f64::NEG_INFINITY, f64::max);
        let bottom = *members.iter().min_by(|&&a, &&b| z(a).total_cmp(&z(b)).then(a.cmp(&b))).unwrap_or(&start);
        // The outlet: a neighbouring dry corner at the level, the lowest index.
        let outlet = members
            .iter()
            .flat_map(|&v| tin.corners[v].iter().map(|w| *w as usize))
            .filter(|&w| !wet[w] && (z(w) - l).abs() < 1e-9)
            .min()
            .map_or([f64::NAN; 3], |w| tin.surface.vertices[w]);
        let mut tris: HashSet<u32> = HashSet::new();
        // The triangles with a wet corner: every triangle under the level has one.
        for &v in &members {
            tris.extend(tin.around[v].iter().copied());
        }
        let (mut area, mut volume) = (0.0, 0.0);
        let mut sorted: Vec<u32> = tris.into_iter().collect();
        sorted.sort_unstable();
        for t in sorted {
            let (a, v) = under(tin, t, l);
            area += a;
            volume += v;
        }
        out.push(Pond {
            bottom: tin.surface.vertices[bottom],
            level: l,
            outlet,
            depth: l - z(bottom),
            area,
            volume,
            corners: members.len(),
        });
    }
    out.retain(|p| p.depth >= min_depth);
    out.sort_by(|a, b| b.depth.total_cmp(&a.depth).then(a.bottom[0].total_cmp(&b.bottom[0])).then(a.bottom[1].total_cmp(&b.bottom[1])));
    out
}

/// A catchment: its cells' outline rings, its area at the grid's resolution, and the flow path from its most distant
/// cell to the discharge point with its length, fall and mean slope.
#[derive(Clone, Debug, PartialEq)]
pub struct Catchment {
    pub cells: usize,
    pub area: f64,
    pub outline: Vec<Vec<[f64; 2]>>,
    pub path: Vec<[f64; 3]>,
    pub length: f64,
    pub fall: f64,
}

/// The nearest place of a path to `p`: its distance and how far along the path it is (the path cut there).
fn nearest_on(path: &[[f64; 3]], p: [f64; 2]) -> Option<(f64, usize, [f64; 3])> {
    let mut best: Option<(f64, usize, [f64; 3])> = None;
    for (i, w) in path.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len2 = dx * dx + dy * dy;
        let t = if len2 > 0.0 { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
        let q = [a[0] + t * dx, a[1] + t * dy, a[2] + t * (b[2] - a[2])];
        let d = (q[0] - p[0]).hypot(q[1] - p[1]);
        if best.is_none_or(|(bd, _, _)| d < bd) {
            best = Some((d, i, q));
        }
    }
    if best.is_none()
        && let Some(a) = path.first()
    {
        best = Some(((a[0] - p[0]).hypot(a[1] - p[1]), 0, *a));
    }
    best
}

/// The catchment of `discharge` on a grid of `cell` metres over the surface (cells centred on its multiples): the cells
/// whose centre's drop passes within `radius` of it. None when no cell's does.
pub fn catchment(tin: &Tin<'_>, discharge: [f64; 2], cell: f64, radius: f64) -> Option<Catchment> {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for t in &tin.surface.triangles {
        for &i in t {
            let v = tin.v(i);
            x0 = x0.min(v[0]);
            y0 = y0.min(v[1]);
            x1 = x1.max(v[0]);
            y1 = y1.max(v[1]);
        }
    }
    if !x0.is_finite() || cell.is_nan() || cell <= 0.0 {
        return None;
    }
    let mut taken: HashSet<(i64, i64)> = HashSet::new();
    let mut longest: Option<(f64, Vec<[f64; 3]>)> = None;
    for i in (x0 / cell).floor() as i64..=(x1 / cell).floor() as i64 {
        for j in (y0 / cell).floor() as i64..=(y1 / cell).floor() as i64 {
            let c = [(i as f64 + 0.5) * cell, (j as f64 + 0.5) * cell];
            let drop = water_drop(tin, c);
            if drop.end == DropEnd::Outside {
                continue;
            }
            let Some((d, at, q)) = nearest_on(&drop.path, discharge) else { continue };
            if d > radius {
                continue;
            }
            taken.insert((i, j));
            let mut part: Vec<[f64; 3]> = drop.path[..=at].to_vec();
            part.push(q);
            let length: f64 = part.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum();
            if longest.as_ref().is_none_or(|(l, _)| length > *l) {
                longest = Some((length, part));
            }
        }
    }
    let (length, path) = longest?;
    let fall = path.first().map_or(0.0, |a| a[2]) - path.last().map_or(0.0, |b| b[2]);
    Some(Catchment { cells: taken.len(), area: taken.len() as f64 * cell * cell, outline: outline(&taken, cell), path, length, fall })
}

/// The outline of a set of grid cells: rings of their outer edges, counter-clockwise outside and clockwise around holes.
fn outline(cells: &HashSet<(i64, i64)>, cell: f64) -> Vec<Vec<[f64; 2]>> {
    // Each cell's edges counter-clockwise; an edge shared by two cells is inside.
    let mut next: HashMap<(i64, i64), Vec<(i64, i64)>> = HashMap::new();
    let mut sorted: Vec<&(i64, i64)> = cells.iter().collect();
    sorted.sort_unstable();
    for &&(i, j) in &sorted {
        let sides = [((i, j), (i + 1, j), (i, j - 1)), ((i + 1, j), (i + 1, j + 1), (i + 1, j)), ((i + 1, j + 1), (i, j + 1), (i, j + 1)), ((i, j + 1), (i, j), (i - 1, j))];
        for (a, b, other) in sides {
            if !cells.contains(&other) {
                next.entry(a).or_default().push(b);
            }
        }
    }
    let mut rings = Vec::new();
    let mut starts: Vec<(i64, i64)> = next.keys().copied().collect();
    starts.sort_unstable();
    for s in starts {
        while let Some(list) = next.get_mut(&s) {
            let Some(mut b) = list.pop() else { break };
            let mut ring = vec![s];
            let mut a = s;
            while b != s {
                ring.push(b);
                let Some(list) = next.get_mut(&b) else { break };
                // At a corner two cells share, keep turning left (the smallest turn).
                let pick = if list.len() > 1 {
                    let d = (b.0 - a.0, b.1 - a.1);
                    let left = (-d.1, d.0);
                    list.iter().position(|n| (n.0 - b.0, n.1 - b.1) == left).unwrap_or(0)
                } else {
                    0
                };
                if list.is_empty() {
                    break;
                }
                let n = list.swap_remove(pick);
                a = b;
                b = n;
            }
            // Corners in the middle of a straight run are left out.
            let mut pts: Vec<(i64, i64)> = Vec::new();
            let m = ring.len();
            for k in 0..m {
                let (p, c, q) = (ring[(k + m - 1) % m], ring[k], ring[(k + 1) % m]);
                if (c.0 - p.0) * (q.1 - c.1) - (c.1 - p.1) * (q.0 - c.0) != 0 {
                    pts.push(c);
                }
            }
            rings.push(pts.iter().map(|&(i, j)| [i as f64 * cell, j as f64 * cell]).collect());
        }
    }
    rings
}

/// A channel's full-flow shape: a pipe of a diameter, a box of a width and a height, or a trapezoid of a bottom width,
/// a depth and its two sides' slopes (horizontal per vertical).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Channel {
    Pipe { diameter: f64 },
    Box { width: f64, height: f64 },
    Trapezoid { bottom: f64, depth: f64, left: f64, right: f64 },
}

/// A channel's flow by Manning's equation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flow {
    /// The flow area (m²), the wetted perimeter (m), the hydraulic radius (m).
    pub area: f64,
    pub perimeter: f64,
    pub radius: f64,
    /// The velocity (m/s) and the discharge (m³/s).
    pub velocity: f64,
    pub discharge: f64,
}

/// Manning's full flow of `channel` at slope `slope` (m/m) and roughness `n`; none for a value not positive.
pub fn manning(channel: Channel, slope: f64, n: f64) -> Option<Flow> {
    if !(slope > 0.0 && n > 0.0) {
        return None;
    }
    let (area, perimeter) = match channel {
        Channel::Pipe { diameter } if diameter > 0.0 => {
            (std::f64::consts::PI * diameter * diameter / 4.0, std::f64::consts::PI * diameter)
        }
        Channel::Box { width, height } if width > 0.0 && height > 0.0 => (width * height, 2.0 * (width + height)),
        Channel::Trapezoid { bottom, depth, left, right } if depth > 0.0 && bottom >= 0.0 && left >= 0.0 && right >= 0.0 => (
            bottom * depth + 0.5 * (left + right) * depth * depth,
            bottom + depth * ((1.0 + left * left).sqrt() + (1.0 + right * right).sqrt()),
        ),
        _ => return None,
    };
    let radius = area / perimeter;
    let velocity = radius.powf(2.0 / 3.0) * slope.sqrt() / n;
    Some(Flow { area, perimeter, radius, velocity, discharge: velocity * area })
}

/// The Rational method's peak flow (m³/s) of a catchment of `area` m² with runoff coefficient `c` and rainfall
/// intensity `i` mm/h: Q = C i A / 3.6 with A in km².
pub fn rational(c: f64, i: f64, area: f64) -> f64 {
    c * i * (area / 1e6) / 3.6
}
