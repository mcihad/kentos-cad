//! Picking (plan/3d.md SCENE-PICK-04: the GPU's picture is a candidate, the source decides): a ray through a pixel
//! meets the tiles drawn; their boxes, as shown, narrow it to the tiles it passes nearest first, and each tile's
//! triangles are met in f64 with the source's own coordinates (its lineage gives each triangle's mesh and index), not
//! the float32 the GPU drew. The point returned is the source's, its height real (the exaggeration taken off).
//!
//! The triangle test is Möller and Trumbore's, both faces, with the corners taken relative to the ray's origin first,
//! so a projected coordinate of millions of metres keeps its millimetres.

use crate::camera::Exaggeration;
use crate::chunk::Content;
use crate::math::{self, V3};
use crate::tile::{Aabb, Hierarchy};

/// Where a ray meets a triangle (both faces): its distance along `dir` (of length one), none when it misses or meets
/// it behind the origin.
pub fn ray_triangle(origin: V3, dir: V3, t: [V3; 3]) -> Option<f64> {
    let [a, b, c] = t.map(|p| math::sub(p, origin));
    let e1 = math::sub(b, a);
    let e2 = math::sub(c, a);
    let p = math::cross(dir, e2);
    let det = math::dot(e1, p);
    if det.abs() < 1e-300 {
        return None;
    }
    let inv = 1.0 / det;
    let s = math::scale(a, -1.0);
    let u = math::dot(s, p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = math::cross(s, e1);
    let v = math::dot(dir, q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let d = math::dot(e2, q) * inv;
    (d > 0.0).then_some(d)
}

/// Where a ray enters a box (0 when it starts inside), none when it misses it.
pub fn ray_box(origin: V3, dir: V3, b: &Aabb) -> Option<f64> {
    let (mut lo, mut hi) = (0.0f64, f64::INFINITY);
    for a in 0..3 {
        let (min, max) = (b.min[a] - origin[a], b.max[a] - origin[a]);
        if dir[a] == 0.0 {
            if min > 0.0 || max < 0.0 {
                return None;
            }
            continue;
        }
        let (t0, t1) = (min / dir[a], max / dir[a]);
        let (t0, t1) = if t0 <= t1 { (t0, t1) } else { (t1, t0) };
        lo = lo.max(t0);
        hi = hi.min(t1);
        if lo > hi {
            return None;
        }
    }
    Some(lo)
}

/// What a ray met first: the tile and its content, the source triangle (mesh, index), the distance along the ray in
/// the shown space, and the point, its height real.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub tile: u32,
    pub content: u64,
    pub mesh: u32,
    pub triangle: u32,
    pub distance: f64,
    pub point: V3,
}

/// The first thing a ray (`origin`, `dir` of length one, in the shown space) meets among the tiles `drawn`: their
/// contents by id, the source triangle of a (mesh, index) in the source's coordinates (none: left out).
pub fn pick<'a>(
    h: &Hierarchy,
    drawn: &[u32],
    content: impl Fn(u64) -> Option<&'a Content>,
    source: impl Fn(u32, u32) -> Option<[V3; 3]>,
    origin: V3,
    dir: V3,
    ex: &Exaggeration,
) -> Option<Hit> {
    let mut tiles: Vec<(f64, u32)> = drawn
        .iter()
        .filter_map(|&i| {
            let t = h.tiles.get(i as usize)?;
            t.content?;
            ray_box(origin, dir, &ex.shown_box(&t.bounds)).map(|d| (d, i))
        })
        .collect();
    tiles.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut best: Option<Hit> = None;
    for (enter, i) in tiles {
        if best.is_some_and(|b| b.distance < enter) {
            break;
        }
        let Some(c) = h.tiles[i as usize].content.and_then(|r| content(r.id)) else { continue };
        for &(mesh, triangle) in &c.sources {
            let Some(t) = source(mesh, triangle) else { continue };
            let Some(d) = ray_triangle(origin, dir, t.map(|p| ex.shown(p))) else { continue };
            if best.is_none_or(|b| d < b.distance || (d == b.distance && (mesh, triangle) < (b.mesh, b.triangle))) {
                let shown = math::add(origin, math::scale(dir, d));
                best = Some(Hit { tile: i, content: c.id, mesh, triangle, distance: d, point: ex.real(shown) });
            }
        }
    }
    best
}
