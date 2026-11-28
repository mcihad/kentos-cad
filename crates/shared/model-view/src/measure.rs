//! Measuring on the bodies shown (docs/adr/8121): a point the pointer finds snapped to what is near it on the screen,
//! and what two, three or one of them measure. Everything is the source's float64 coordinates in metres, never the
//! float32 the GPU drew; a body's corners are its own vertices, so a corner's coordinates are the file's.
//!
//! - **Snapping** ([`snap`]): of the body the pointer is over, a corner within the tolerance (on the screen) first, then
//!   an edge's middle, then the nearest point of an edge (where the pointer's ray passes nearest it), else the point on
//!   the face. Only a body's edges count (where two of its faces meet at an angle, [`edges`]): a face's triangles'
//!   diagonals are no edge.
//! - **Distance** ([`Distance`]): the length, the horizontal length and the three components along the axes.
//! - **Angle** ([`angle`]): at the middle of three points, in degrees.
//! - **Face** ([`face`]): the triangles of a body in one plane joined by their edges to the one met: its area (the sum
//!   of its triangles', in f64) and its outline.
//!
//! What the mesh measures is the shown body's: a curved face is its triangles (a chord of a circle, not its arc). The
//! viewer says so; a file's own quantities are in its sets.

use std::collections::HashMap;

use kentos_scene3d::math::{self, V3};

/// What a point snapped to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Snap {
    Corner,
    Middle,
    Edge,
    Face,
}

impl Snap {
    /// Its name on the screen.
    pub fn name(self) -> &'static str {
        match self {
            Snap::Corner => "Köşe",
            Snap::Middle => "Kenar ortası",
            Snap::Edge => "Kenar",
            Snap::Face => "Yüz",
        }
    }
}

/// The least angle between two faces, degrees, for the edge between them to be one a point snaps to (the renderer's).
pub const CREASE_DEGREES: f64 = 25.0;

/// A body's edges as pairs of indices into its vertices: where one of its faces ends (an open body's rim), where more
/// than two meet, or where two turn by at least `crease_degrees`; its corners joined by their exact coordinates.
pub fn edges(vertices: &[V3], triangles: &[[u32; 3]], crease_degrees: f64) -> Vec<[u32; 2]> {
    let cos = kentos_num::cos(crease_degrees.to_radians());
    let mut corners: HashMap<[u64; 3], u32> = HashMap::new();
    let mut index: HashMap<(u32, u32), usize> = HashMap::new();
    let mut found: Vec<([u32; 2], Vec<V3>)> = Vec::new();
    for t in triangles {
        if t.iter().any(|&k| k as usize >= vertices.len()) {
            continue;
        }
        let [a, b, c] = t.map(|k| vertices[k as usize]);
        let Some(n) = normal(a, b, c) else { continue };
        let mut corner = |k: u32| {
            let next = corners.len() as u32;
            *corners
                .entry(vertices[k as usize].map(f64::to_bits))
                .or_insert(next)
        };
        let ids = [corner(t[0]), corner(t[1]), corner(t[2])];
        for (i, j) in [(0, 1), (1, 2), (2, 0)] {
            if ids[i] == ids[j] {
                continue;
            }
            let key = (ids[i].min(ids[j]), ids[i].max(ids[j]));
            let at = *index.entry(key).or_insert_with(|| {
                found.push(([t[i], t[j]], Vec::new()));
                found.len() - 1
            });
            found[at].1.push(n);
        }
    }
    found
        .into_iter()
        .filter(|(_, normals)| match normals.as_slice() {
            [m, n] => math::dot(*m, *n).abs() < cos,
            _ => true,
        })
        .map(|(e, _)| e)
        .collect()
}

/// A triangle's unit normal; none for one without area.
fn normal(a: V3, b: V3, c: V3) -> Option<V3> {
    let n = math::cross(math::sub(b, a), math::sub(c, a));
    let l = math::length(n);
    (l > 1e-12).then(|| math::scale(n, 1.0 / l))
}

/// The point of the segment `a`–`b` nearest the ray from `origin` along `dir` (of length one), and the ray's distance
/// to it there.
pub fn nearest_on_segment(origin: V3, dir: V3, a: V3, b: V3) -> (V3, f64) {
    let ab = math::sub(b, a);
    let len2 = math::dot(ab, ab);
    if len2 <= 0.0 {
        return (a, distance_to_ray(origin, dir, a));
    }
    // The closest points of the two lines, the segment's parameter kept within it.
    let w = math::sub(a, origin);
    let (b_, d, e) = (math::dot(ab, dir), math::dot(ab, w), math::dot(dir, w));
    let denom = len2 - b_ * b_;
    let s = if denom.abs() < 1e-18 {
        0.0
    } else {
        (b_ * e - d) / denom
    };
    let s = s.clamp(0.0, 1.0);
    let p = math::add(a, math::scale(ab, s));
    (p, distance_to_ray(origin, dir, p))
}

/// The distance of `p` from the ray's line.
fn distance_to_ray(origin: V3, dir: V3, p: V3) -> f64 {
    let w = math::sub(p, origin);
    let along = math::dot(w, dir);
    math::length(math::sub(w, math::scale(dir, along)))
}

/// What the pointer finds: the point on the face its ray met (`hit`), snapped among the body's `vertices` and `edges` to
/// the first within `tolerance` pixels of the pointer at `cursor` on the screen (`project`: a point to its pixel; none
/// behind the camera): a corner, then an edge's middle, then an edge, else the face.
pub fn snap(
    hit: V3,
    vertices: &[V3],
    edges: &[[u32; 2]],
    ray: (V3, V3),
    cursor: (f64, f64),
    tolerance: f64,
    project: impl Fn(V3) -> Option<(f64, f64)>,
) -> (V3, Snap) {
    let on_screen = |p: V3| project(p).map(|(x, y)| kentos_num::hypot(x - cursor.0, y - cursor.1));
    let mut best: Option<(f64, V3)> = None;
    let consider = |p: V3, best: &mut Option<(f64, V3)>| {
        if let Some(d) = on_screen(p).filter(|d| *d <= tolerance)
            && best.is_none_or(|(b, _)| d < b)
        {
            *best = Some((d, p));
        }
    };
    for e in edges {
        for &k in e {
            if let Some(p) = vertices.get(k as usize) {
                consider(*p, &mut best);
            }
        }
    }
    if let Some((_, p)) = best {
        return (p, Snap::Corner);
    }
    let ends = |e: &[u32; 2]| Some((*vertices.get(e[0] as usize)?, *vertices.get(e[1] as usize)?));
    for e in edges {
        if let Some((a, b)) = ends(e) {
            consider(math::scale(math::add(a, b), 0.5), &mut best);
        }
    }
    if let Some((_, p)) = best {
        return (p, Snap::Middle);
    }
    for e in edges {
        if let Some((a, b)) = ends(e) {
            consider(nearest_on_segment(ray.0, ray.1, a, b).0, &mut best);
        }
    }
    match best {
        Some((_, p)) => (p, Snap::Edge),
        None => (hit, Snap::Face),
    }
}

/// A distance: from, to, its length, its length on the ground and its components along the axes (east, north, up).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Distance {
    pub length: f64,
    pub horizontal: f64,
    pub delta: V3,
}

impl Distance {
    pub fn between(a: V3, b: V3) -> Distance {
        let delta = math::sub(b, a);
        Distance {
            length: math::length(delta),
            horizontal: kentos_num::hypot(delta[0], delta[1]),
            delta,
        }
    }
}

/// The angle at `vertex` between the points `a` and `b`, degrees (0–180); none when either is at the vertex.
pub fn angle(a: V3, vertex: V3, b: V3) -> Option<f64> {
    let (u, v) = (math::sub(a, vertex), math::sub(b, vertex));
    let (lu, lv) = (math::length(u), math::length(v));
    if lu <= 1e-12 || lv <= 1e-12 {
        return None;
    }
    // atan2 of the cross's length and the dot: exact near 0° and 180°, where acos is not.
    Some(kentos_num::atan2(math::length(math::cross(u, v)), math::dot(u, v)).to_degrees())
}

/// A body's face: its area (m²), its outline's segments and its unit normal.
#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    pub area: f64,
    pub outline: Vec<[V3; 2]>,
    pub normal: V3,
}

/// The face of a body a triangle is on: its triangles in the triangle's plane (their normals within a tenth of a degree,
/// their corners within a millimetre of it) joined to it through shared edges; none for a triangle without area.
pub fn face(vertices: &[V3], triangles: &[[u32; 3]], triangle: usize) -> Option<Face> {
    let corner = |k: u32| vertices.get(k as usize).copied();
    let corners = |t: &[u32; 3]| Some([corner(t[0])?, corner(t[1])?, corner(t[2])?]);
    let [a, b, c] = corners(triangles.get(triangle)?)?;
    let n = normal(a, b, c)?;
    let offset = math::dot(n, a);
    let flat = kentos_num::cos(0.1f64.to_radians());
    let in_plane = |t: &[u32; 3]| {
        corners(t).is_some_and(|[p, q, r]| {
            normal(p, q, r).is_some_and(|m| math::dot(m, n).abs() >= flat)
                && [p, q, r]
                    .iter()
                    .all(|v| (math::dot(n, *v) - offset).abs() <= 1e-3)
        })
    };
    // The triangles by their edges (corners joined by exact coordinates).
    let key = |k: u32| vertices[k as usize].map(f64::to_bits);
    let mut by_edge: HashMap<([u64; 3], [u64; 3]), Vec<usize>> = HashMap::new();
    for (i, t) in triangles.iter().enumerate() {
        if t.iter().any(|&k| k as usize >= vertices.len()) || !in_plane(t) {
            continue;
        }
        for (p, q) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (kp, kq) = (key(p), key(q));
            by_edge
                .entry(if kp <= kq { (kp, kq) } else { (kq, kp) })
                .or_default()
                .push(i);
        }
    }
    let mut seen = vec![false; triangles.len()];
    let mut todo = vec![triangle];
    seen[triangle] = true;
    let (mut area, mut members) = (0.0f64, Vec::new());
    while let Some(i) = todo.pop() {
        let t = triangles[i];
        let [p, q, r] = corners(&t)?;
        area += 0.5 * math::length(math::cross(math::sub(q, p), math::sub(r, p)));
        members.push(i);
        for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (ku, kv) = (key(u), key(v));
            for &j in by_edge
                .get(&if ku <= kv { (ku, kv) } else { (kv, ku) })
                .into_iter()
                .flatten()
            {
                if !seen[j] {
                    seen[j] = true;
                    todo.push(j);
                }
            }
        }
    }
    // The outline: the members' edges no other member has.
    /// How many members have the edge, and its two corners.
    type Seen = (u32, V3, V3);
    let mut count: HashMap<([u64; 3], [u64; 3]), Seen> = HashMap::new();
    for &i in &members {
        let t = triangles[i];
        for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (ku, kv) = (key(u), key(v));
            let e = count
                .entry(if ku <= kv { (ku, kv) } else { (kv, ku) })
                .or_insert((0, vertices[u as usize], vertices[v as usize]));
            e.0 += 1;
        }
    }
    let mut outline: Vec<[V3; 2]> = count
        .into_values()
        .filter(|e| e.0 == 1)
        .map(|e| [e.1, e.2])
        .collect();
    outline.sort_by(|x, y| x[0].partial_cmp(&y[0]).unwrap_or(std::cmp::Ordering::Equal));
    Some(Face {
        area,
        outline,
        normal: n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3 × 2 m wall face on x = 0 (two triangles), its box closed by the other faces 0.25 m thick.
    fn slab() -> (Vec<V3>, Vec<[u32; 3]>) {
        let c = |i: usize| {
            [
                if i & 1 == 0 { 0.0 } else { 0.25 },
                if i & 2 == 0 { 0.0 } else { 3.0 },
                if i & 4 == 0 { 0.0 } else { 2.0 },
            ]
        };
        let faces = [
            [0, 4, 6, 2],
            [1, 3, 7, 5],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 2, 3, 1],
            [4, 5, 7, 6],
        ];
        let (mut v, mut t) = (Vec::new(), Vec::new());
        for f in faces {
            let k = v.len() as u32;
            v.extend(f.map(c));
            t.extend([[k, k + 1, k + 2], [k, k + 2, k + 3]]);
        }
        (v, t)
    }

    #[test]
    fn a_box_s_face_is_its_area_and_outline_not_its_triangles() {
        let (v, t) = slab();
        // The first face (x = 0): 3 × 2 m, four outline segments, its diagonal no outline.
        let f = face(&v, &t, 0).expect("a face");
        assert!((f.area - 6.0).abs() < 1e-12, "{}", f.area);
        assert_eq!(f.outline.len(), 4);
        assert!((f.normal[0].abs() - 1.0).abs() < 1e-12);
        // A narrow side (0.25 × 2 m).
        let f = face(&v, &t, 4).expect("a face");
        assert!((f.area - 0.5).abs() < 1e-12, "{}", f.area);
    }

    #[test]
    fn a_box_has_twelve_edges_to_snap_to() {
        let (v, t) = slab();
        assert_eq!(edges(&v, &t, CREASE_DEGREES).len(), 12);
    }

    #[test]
    fn distance_angle_and_the_ray_s_nearest_point() {
        let d = Distance::between([1.0, 2.0, 3.0], [4.0, 6.0, 15.0]);
        assert_eq!(d.delta, [3.0, 4.0, 12.0]);
        assert_eq!((d.horizontal, d.length), (5.0, 13.0));
        assert!(
            (angle([1.0, 0.0, 0.0], [0.0; 3], [0.0, 1.0, 0.0]).expect("an angle") - 90.0).abs()
                < 1e-12
        );
        assert!(
            (angle([1.0, 0.0, 0.0], [0.0; 3], [-1.0, 1e-9, 0.0]).expect("an angle") - 180.0).abs()
                < 1e-6
        );
        assert_eq!(angle([0.0; 3], [0.0; 3], [1.0, 0.0, 0.0]), None);
        // A ray down the z axis at x = 1, y = 0.5 passes nearest the segment along y at x = 1, z = 0 at y = 0.5.
        let (p, d) = nearest_on_segment(
            [1.0, 0.5, 10.0],
            [0.0, 0.0, -1.0],
            [1.0, 0.0, 0.0],
            [1.0, 2.0, 0.0],
        );
        assert!((p[1] - 0.5).abs() < 1e-12 && d < 1e-12, "{p:?} {d}");
    }

    #[test]
    fn a_point_snaps_to_a_corner_before_a_middle_before_an_edge() {
        let (v, t) = slab();
        let e = edges(&v, &t, CREASE_DEGREES);
        // Seen from straight in front of x = 0: a pixel a metre, the screen's (x, y) the source's (y, z).
        let project = |p: V3| Some((p[1] * 100.0, 200.0 - p[2] * 100.0));
        let ray = |x: f64, y: f64| ([-10.0, x / 100.0, (200.0 - y) / 100.0], [1.0, 0.0, 0.0]);
        let at = |x: f64, y: f64| {
            let r = ray(x, y);
            snap([0.0, r.0[1], r.0[2]], &v, &e, r, (x, y), 8.0, project)
        };
        // Near the corner (0, 0, 2): the corner's exact coordinates.
        assert_eq!(at(3.0, 4.0), ([0.0, 0.0, 2.0], Snap::Corner));
        // Near the top edge's middle (0, 1.5, 2).
        assert_eq!(at(152.0, 5.0), ([0.0, 1.5, 2.0], Snap::Middle));
        // On the top edge away from both: the edge.
        let (p, s) = at(80.0, 3.0);
        assert_eq!(s, Snap::Edge);
        assert!((p[1] - 0.8).abs() < 1e-9 && p[2] == 2.0, "{p:?}");
        // In the middle of the face: the face.
        assert_eq!(at(120.0, 100.0).1, Snap::Face);
    }
}
