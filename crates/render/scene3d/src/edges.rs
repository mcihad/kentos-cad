//! A tile's edges (docs/adr/8120): the lines drawn over a body's faces where two of them meet at an angle, the way a
//! drawing outlines what it shows. Found once, when the tile's content is uploaded, from its own triangles: a body's
//! (a source mesh's) corners joined by their float32 positions (a flat body's faces each have their own vertices),
//! each edge with the normals of the faces on it. An edge is drawn when one face has it (an open body's rim), when more
//! than two do (where a body's parts meet), or when its two faces turn by at least the crease angle; a face split in
//! triangles, or a corridor's strips on a gentle curve, stay one surface. Two bodies are never joined: a wall's end
//! against another wall keeps both outlines. A smooth body (the terrain: its vertices carry normals) has none.

use std::collections::HashMap;

use kentos_scene3d::Content;

/// The least angle between two faces, degrees, for the edge between them to be drawn.
pub const CREASE_DEGREES: f32 = 25.0;

/// `c`'s edges as pairs of indices into its vertices, in the order its triangles first meet them.
pub fn feature_edges(c: &Content, crease_degrees: f32) -> Vec<u32> {
    let cos = crease_degrees.to_radians().cos();
    // A corner by its body and its position's bits.
    let mut corners: HashMap<(u32, [u32; 3]), u32> = HashMap::new();
    // An edge by its body and its two corners: one pair of its vertex indices and its faces' normals.
    let mut index: HashMap<(u32, u32, u32), usize> = HashMap::new();
    let mut edges: Vec<(u32, u32, Vec<[f32; 3]>)> = Vec::new();
    let smooth = |i: u32| c.normals.get(i as usize).is_some_and(|n| n[0] != 0 || n[1] != 0 || n[2] != 0);
    for (t, tri) in c.indices.as_chunks::<3>().0.iter().enumerate() {
        if tri.iter().any(|&i| i as usize >= c.positions.len() || smooth(i)) {
            continue;
        }
        let body = c.sources.get(t).map_or(0, |s| s.0);
        let Some(n) = normal(c.positions[tri[0] as usize], c.positions[tri[1] as usize], c.positions[tri[2] as usize]) else {
            continue;
        };
        let mut corner = |i: u32| {
            let next = corners.len() as u32;
            *corners.entry((body, c.positions[i as usize].map(f32::to_bits))).or_insert(next)
        };
        let ids = [corner(tri[0]), corner(tri[1]), corner(tri[2])];
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            if ids[a] == ids[b] {
                continue;
            }
            let key = (body, ids[a].min(ids[b]), ids[a].max(ids[b]));
            let at = *index.entry(key).or_insert_with(|| {
                edges.push((tri[a], tri[b], Vec::new()));
                edges.len() - 1
            });
            edges[at].2.push(n);
        }
    }
    let mut out = Vec::new();
    for (a, b, normals) in edges {
        let drawn = match normals.as_slice() {
            [_] => true,
            // Either winding: a face turned over by its writer is still the same surface.
            [m, n] => (m[0] * n[0] + m[1] * n[1] + m[2] * n[2]).abs() < cos,
            _ => true,
        };
        if drawn {
            out.extend([a, b]);
        }
    }
    out
}

/// A triangle's unit normal; none for one without area.
fn normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Option<[f32; 3]> {
    let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
    let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    (l > 1e-12).then(|| n.map(|x| x / l))
}

#[cfg(test)]
mod tests {
    use kentos_scene3d::{Limits, SourceMesh, chunk};

    use super::*;

    /// A box's six faces, each two triangles with vertices of their own (as a flat body's are).
    fn cube(x: f64, size: f64) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
        let c = |i: usize| [x + if i & 1 == 0 { 0.0 } else { size }, if i & 2 == 0 { 0.0 } else { size }, if i & 4 == 0 { 0.0 } else { size }];
        let faces = [[0, 1, 3, 2], [4, 6, 7, 5], [0, 4, 5, 1], [2, 3, 7, 6], [0, 2, 6, 4], [1, 5, 7, 3]];
        let (mut v, mut t) = (Vec::new(), Vec::new());
        for f in faces {
            let at = v.len() as u32;
            v.extend(f.map(c));
            t.extend([[at, at + 1, at + 2], [at, at + 2, at + 3]]);
        }
        (v, t)
    }

    /// A mesh's vertices and triangles.
    type Mesh = (Vec<[f64; 3]>, Vec<[u32; 3]>);

    fn edges_of(meshes: &[Mesh], smooth: bool) -> usize {
        let src: Vec<SourceMesh> =
            meshes.iter().map(|(v, t)| SourceMesh { part: 0, vertices: v, triangles: t, smooth }).collect();
        let chunked = chunk(&src, Limits::default(), 1);
        chunked.contents.iter().map(|c| feature_edges(c, CREASE_DEGREES).len() / 2).sum()
    }

    #[test]
    fn a_box_has_its_twelve_edges_and_no_diagonal() {
        assert_eq!(edges_of(&[cube(0.0, 1.0)], false), 12);
    }

    #[test]
    fn two_boxes_side_by_side_keep_both_outlines() {
        // Touching along a face: one body's corners are never joined with the other's.
        assert_eq!(edges_of(&[cube(0.0, 1.0), cube(1.0, 1.0)], false), 24);
    }

    #[test]
    fn a_gentle_fold_is_one_surface_and_a_sharp_one_an_edge() {
        // Two triangles sharing the edge along x, the second turned up by `deg` about it.
        let fold = |deg: f64| {
            let (s, c) = deg.to_radians().sin_cos();
            (vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.5, -c, s]], vec![[0, 1, 2], [0, 3, 1]])
        };
        // The rim: three edges of each triangle but the shared one, open; the fold drawn only past the crease.
        assert_eq!(edges_of(&[fold(10.0)], false), 4);
        assert_eq!(edges_of(&[fold(40.0)], false), 5);
    }

    #[test]
    fn a_smooth_body_has_none() {
        assert_eq!(edges_of(&[cube(0.0, 1.0)], true), 0);
    }
}
