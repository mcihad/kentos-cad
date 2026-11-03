//! Large triangle meshes cut into tiles (plan/3d.md SCENE-PREC-04, 11, SCENE-STREAM-01, 17): a binary tree over the
//! triangles' centroids, each split at the median along the longest side of the centroids' box, until a tile holds at
//! most `max_triangles` and spans at most `max_extent` metres. Each leaf is a content: its vertices once, as float32
//! offsets from its own f64 origin (its bounds' middle), so the rounding is bounded by the tile's extent and not by
//! the source's coordinates; its triangles grouped by the part they came from (a renderer draws a part's range with
//! one style); each triangle's source (part, triangle) kept, the lineage a pick refines through. The inner tiles have
//! no content (they always refine) and bound their children; a leaf's geometric error is 0: it is the source.
//!
//! The order is total and the cut does not depend on the input's order within a key: a triangle's key is its centroid
//! along the axis, then its part, then its index; the median splits the sorted triangles at `n / 2`. The tiles are
//! numbered depth first, a tile before its children, the lower half before the upper; the contents in that order.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use crate::math::V3;
use crate::tile::{Aabb, ContentRef, Hierarchy, Refine, Tile, TileKey};

/// A source mesh: its part (the host's: a surface, a corridor, an asset), its vertices in the source's coordinates
/// and its triangles.
#[derive(Clone, Copy, Debug)]
pub struct SourceMesh<'a> {
    pub part: u32,
    pub vertices: &'a [V3],
    pub triangles: &'a [[u32; 3]],
}

/// How large a tile may be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    /// Triangles in a tile, at most.
    pub max_triangles: usize,
    /// The largest side of a tile's bounds, in metres, at most (a single triangle larger than it is a tile of its
    /// own).
    pub max_extent: f64,
}

impl Default for Limits {
    /// 32 768 triangles and 1 000 m: a draw call large enough to be cheap, small enough to cull; and a float32 offset
    /// within 500 m of its origin keeps a half unit in the last place of 0.03 mm (plan/3d.md §5).
    fn default() -> Self {
        Limits { max_triangles: 32_768, max_extent: 1_000.0 }
    }
}

/// A tile's content.
#[derive(Clone, Debug, PartialEq)]
pub struct Content {
    pub id: u64,
    pub origin: V3,
    pub bounds: Aabb,
    /// The vertices relative to the origin.
    pub positions: Vec<[f32; 3]>,
    /// Each vertex's part.
    pub parts: Vec<u32>,
    pub indices: Vec<u32>,
    /// Each part's indices, in part order.
    pub ranges: Vec<(u32, std::ops::Range<u32>)>,
    /// Each triangle's source: its part and its index in the part's mesh.
    pub sources: Vec<(u32, u32)>,
    /// The largest gap along an axis between a vertex's offset and its float32, in metres.
    pub encoding_error: f64,
}

impl Content {
    /// Its size on the GPU: 16 bytes a vertex (position and part), 4 an index.
    pub fn bytes(&self) -> u64 {
        self.positions.len() as u64 * 16 + self.indices.len() as u64 * 4
    }

    /// A vertex's position in the source's coordinates, as the GPU has it (its origin plus its float32).
    pub fn world(&self, k: u32) -> V3 {
        let p = self.positions[k as usize];
        [self.origin[0] + f64::from(p[0]), self.origin[1] + f64::from(p[1]), self.origin[2] + f64::from(p[2])]
    }
}

/// The tiles and their contents.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chunked {
    pub hierarchy: Hierarchy,
    /// The contents, the tiles' `content.id` minus the first id.
    pub contents: Vec<Content>,
}

#[derive(Clone, Copy)]
struct Tri {
    mesh: u32,
    index: u32,
    centroid: V3,
}

/// A subtree as it is built: its tiles (the first its root; children indices local), its contents (ids local, from 0).
#[derive(Default)]
struct Fragment {
    tiles: Vec<Tile>,
    contents: Vec<Content>,
}

impl Fragment {
    /// `self`'s root over two subtrees: their tiles after the root's, the upper's after the lower's.
    fn adopt(&mut self, lower: Fragment, upper: Fragment) {
        let (ta, ca) = (lower.tiles.len() as u32, lower.contents.len() as u64);
        let base = self.tiles.len() as u32;
        self.tiles[0].children = vec![base, base + ta];
        for (fragment, shift, content_shift) in [(lower, base, 0u64), (upper, base + ta, ca)] {
            for mut t in fragment.tiles {
                for c in &mut t.children {
                    *c += shift;
                }
                if let Some(c) = &mut t.content {
                    c.id += content_shift;
                }
                self.tiles.push(t);
            }
            for mut c in fragment.contents {
                c.id += content_shift;
                self.contents.push(c);
            }
        }
    }
}

/// Subtrees of at least this many triangles are built on threads of their own, down to this depth.
#[cfg(not(target_arch = "wasm32"))]
const PARALLEL_TRIANGLES: usize = 65_536;
#[cfg(not(target_arch = "wasm32"))]
const PARALLEL_DEPTH: u32 = 4;

struct Cut<'a> {
    meshes: &'a [SourceMesh<'a>],
    limits: Limits,
}

impl Cut<'_> {
    fn corners(&self, t: &Tri) -> [V3; 3] {
        let m = &self.meshes[t.mesh as usize];
        m.triangles[t.index as usize].map(|k| m.vertices[k as usize])
    }

    fn bounds(&self, tris: &[Tri]) -> Aabb {
        let mut b = Aabb::EMPTY;
        for t in tris {
            for p in self.corners(t) {
                b.grow(p);
            }
        }
        b
    }

    /// The subtree of `tris`: its root first, then the lower half's subtree, then the upper's.
    fn tile(&self, tris: &mut [Tri], depth: u32) -> Fragment {
        let bounds = self.bounds(tris);
        let mut out = Fragment::default();
        out.tiles.push(Tile {
            key: TileKey(0),
            bounds,
            geometric_error: 0.0,
            refine: Refine::Replace,
            content: None,
            children: Vec::new(),
        });
        let small = tris.len() <= self.limits.max_triangles && bounds.extent() <= self.limits.max_extent;
        let axis = if small || tris.len() < 2 { None } else { split_axis(tris) };
        let Some(axis) = axis else {
            let content = self.content(tris, bounds);
            out.tiles[0].content = Some(ContentRef { id: 0, bytes: content.bytes() });
            out.contents.push(content);
            return out;
        };
        // The median by the total key: the lower half holds the `n / 2` smallest, as a full sort would split them.
        let mid = tris.len() / 2;
        tris.select_nth_unstable_by(mid, |a, b| {
            a.centroid[axis].total_cmp(&b.centroid[axis]).then(a.mesh.cmp(&b.mesh)).then(a.index.cmp(&b.index))
        });
        let (lower, upper) = tris.split_at_mut(mid);
        let (a, b) = self.halves(lower, upper, depth);
        // No content of its own: it always refines; its error bounds its children's (3D Tiles: not below them).
        out.tiles[0].geometric_error = bounds.diagonal();
        out.adopt(a, b);
        out
    }

    /// The two halves' subtrees, on two threads near the root of a large mesh (the browser has none: one after the
    /// other there). The result does not depend on which.
    #[cfg(not(target_arch = "wasm32"))]
    fn halves(&self, lower: &mut [Tri], upper: &mut [Tri], depth: u32) -> (Fragment, Fragment) {
        if depth < PARALLEL_DEPTH && lower.len() + upper.len() >= PARALLEL_TRIANGLES {
            std::thread::scope(|s| {
                let a = s.spawn(|| self.tile(lower, depth + 1));
                let b = self.tile(upper, depth + 1);
                (a.join().unwrap_or_else(|e| std::panic::resume_unwind(e)), b)
            })
        } else {
            (self.tile(lower, depth + 1), self.tile(upper, depth + 1))
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn halves(&self, lower: &mut [Tri], upper: &mut [Tri], depth: u32) -> (Fragment, Fragment) {
        (self.tile(lower, depth + 1), self.tile(upper, depth + 1))
    }

    fn content(&self, tris: &mut [Tri], bounds: Aabb) -> Content {
        let origin = bounds.center();
        tris.sort_unstable_by_key(|t| (self.meshes[t.mesh as usize].part, t.mesh, t.index));
        let (mut positions, mut parts, mut indices, mut ranges, mut sources) =
            (Vec::new(), Vec::new(), Vec::with_capacity(tris.len() * 3), Vec::new(), Vec::with_capacity(tris.len()));
        let mut seen: HashMap<u64, u32, BuildHasherDefault<Mix>> = HashMap::with_capacity_and_hasher(tris.len() * 2, Default::default());
        let mut error: f64 = 0.0;
        for t in tris.iter() {
            let m = &self.meshes[t.mesh as usize];
            if ranges.last().is_none_or(|r: &(u32, std::ops::Range<u32>)| r.0 != m.part) {
                let at = indices.len() as u32;
                ranges.push((m.part, at..at));
            }
            for k in m.triangles[t.index as usize] {
                let v = *seen.entry((u64::from(t.mesh) << 32) | u64::from(k)).or_insert_with(|| {
                    let p = m.vertices[k as usize];
                    let f: [f32; 3] = std::array::from_fn(|a| {
                        let d = p[a] - origin[a];
                        let f = d as f32;
                        error = error.max((f64::from(f) - d).abs());
                        f
                    });
                    positions.push(f);
                    parts.push(m.part);
                    (positions.len() - 1) as u32
                });
                indices.push(v);
            }
            if let Some(r) = ranges.last_mut() {
                r.1.end = indices.len() as u32;
            }
            sources.push((m.part, t.index));
        }
        Content { id: 0, origin, bounds, positions, parts, indices, ranges, sources, encoding_error: error }
    }
}

/// A multiplicative hash of a vertex's (mesh, index): the leaves look up millions of corners.
#[derive(Default)]
struct Mix(u64);

impl Hasher for Mix {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0.rotate_left(5) ^ u64::from(*b)).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
        }
    }

    fn write_u64(&mut self, v: u64) {
        self.0 = (v ^ (v >> 29)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        self.0 ^= self.0 >> 32;
    }
}

/// The longest side of the centroids' box, none when they all lie on one point.
fn split_axis(tris: &[Tri]) -> Option<usize> {
    let mut b = Aabb::EMPTY;
    for t in tris {
        b.grow(t.centroid);
    }
    let sides: [f64; 3] = std::array::from_fn(|a| b.max[a] - b.min[a]);
    let axis = (0..3).fold(0, |best, a| if sides[a] > sides[best] { a } else { best });
    (sides[axis] > 0.0).then_some(axis)
}

/// The meshes cut into tiles within `limits`, one root over them all; the contents numbered from `first_id`. Empty
/// meshes give an empty hierarchy. A triangle with a vertex out of its mesh is left out.
pub fn chunk(meshes: &[SourceMesh<'_>], limits: Limits, first_id: u64) -> Chunked {
    let mut tris: Vec<Tri> = Vec::new();
    for (mi, m) in meshes.iter().enumerate() {
        for (ti, t) in m.triangles.iter().enumerate() {
            if t.iter().any(|&k| k as usize >= m.vertices.len()) {
                continue;
            }
            let c = t.iter().fold([0.0; 3], |acc, &k| {
                let p = m.vertices[k as usize];
                [acc[0] + p[0], acc[1] + p[1], acc[2] + p[2]]
            });
            tris.push(Tri { mesh: mi as u32, index: ti as u32, centroid: c.map(|v| v / 3.0) });
        }
    }
    let cut = Cut { meshes, limits };
    if tris.is_empty() {
        return Chunked::default();
    }
    let f = cut.tile(&mut tris, 0);
    let mut out = Chunked { hierarchy: Hierarchy { tiles: f.tiles, roots: vec![0] }, contents: f.contents };
    for (i, t) in out.hierarchy.tiles.iter_mut().enumerate() {
        t.key = TileKey(i as u64);
        if let Some(c) = &mut t.content {
            c.id += first_id;
        }
    }
    for c in &mut out.contents {
        c.id += first_id;
    }
    out
}
