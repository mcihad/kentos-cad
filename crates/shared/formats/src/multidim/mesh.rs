//! A mesh in memory and its values at a point (docs/adr/0243 §4, §5): the
//! faces as triangles (a convex face fanned from its first node, as QGIS
//! does; a concave one ear-clipped), a bucket index of the triangles and of
//! the faces' edges, the triangle a point lies in (exactly: the robust
//! orientation predicate, the lowest triangle of those whose closed area
//! holds it), node values interpolated linearly in it, a face's value as it
//! is, a vector's components interpolated before its magnitude.

use std::ops::Range;

use kentos_geometry_core::predicates::orient2d;
use kentos_geometry_core::triangulate::triangulate_into;
use kentos_geometry_core::vec2::Vec2;

use super::ugrid::Location;
use crate::math;
use crate::raster::{RasterError, Samples};

/// The most cells a bucket index takes.
const MAX_CELLS: usize = 4_000_000;

/// A grid of buckets over a box, each listing the items whose box meets it.
#[derive(Clone, Debug, Default)]
struct Buckets {
    x0: f64,
    y0: f64,
    size: f64,
    nx: usize,
    ny: usize,
    start: Vec<u32>,
    items: Vec<u32>,
}

impl Buckets {
    fn build(bbox: [f64; 4], n: usize, item_box: impl Fn(usize) -> [f64; 4]) -> Buckets {
        let (w, h) = (bbox[2] - bbox[0], bbox[3] - bbox[1]);
        let want = (n / 2).clamp(1, MAX_CELLS) as f64;
        let mut size = (w.max(f64::MIN_POSITIVE) * h.max(f64::MIN_POSITIVE) / want).sqrt();
        if !(size.is_finite() && size > 0.0) {
            size = w.max(h).max(1.0);
        }
        let nx = ((w / size).ceil() as usize).clamp(1, MAX_CELLS);
        let ny = ((h / size).ceil() as usize).clamp(1, (MAX_CELLS / nx).max(1));
        let size = (w / nx as f64).max(h / ny as f64).max(f64::MIN_POSITIVE);
        let mut b = Buckets {
            x0: bbox[0],
            y0: bbox[1],
            size,
            nx,
            ny,
            start: vec![0; nx * ny + 1],
            items: Vec::new(),
        };
        // Two passes: count, then place.
        for k in 0..n {
            let (ix0, iy0, ix1, iy1) = b.span(item_box(k));
            for iy in iy0..=iy1 {
                for ix in ix0..=ix1 {
                    b.start[iy * nx + ix + 1] += 1;
                }
            }
        }
        for c in 0..nx * ny {
            b.start[c + 1] += b.start[c];
        }
        let mut fill = b.start.clone();
        b.items = vec![0; b.start[nx * ny] as usize];
        for k in 0..n {
            let (ix0, iy0, ix1, iy1) = b.span(item_box(k));
            for iy in iy0..=iy1 {
                for ix in ix0..=ix1 {
                    let c = iy * nx + ix;
                    b.items[fill[c] as usize] = k as u32;
                    fill[c] += 1;
                }
            }
        }
        b
    }

    fn col(&self, x: f64) -> usize {
        (((x - self.x0) / self.size).floor().max(0.0) as usize).min(self.nx - 1)
    }

    fn row(&self, y: f64) -> usize {
        (((y - self.y0) / self.size).floor().max(0.0) as usize).min(self.ny - 1)
    }

    /// The cells a box meets.
    fn span(&self, b: [f64; 4]) -> (usize, usize, usize, usize) {
        (
            self.col(b[0]),
            self.row(b[1]),
            self.col(b[2]),
            self.row(b[3]),
        )
    }

    fn items(&self, ix: usize, iy: usize) -> &[u32] {
        let c = iy * self.nx + ix;
        &self.items[self.start[c] as usize..self.start[c + 1] as usize]
    }
}

/// A mesh's values for one slice: one per node or face; a vector's second
/// component; a face mask (0: the face is inactive).
#[derive(Clone, Copy, Debug)]
pub struct Data<'a> {
    pub location: Location,
    pub x: &'a Samples,
    pub y: Option<&'a Samples>,
    pub mask: Option<&'a Samples>,
}

/// A mesh.
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub faces: u32,
    /// Triangles, counter-clockwise, in their faces' order.
    pub tri: Vec<[u32; 3]>,
    pub tri_face: Vec<u32>,
    /// The faces' edges, each once.
    pub edges: Vec<[u32; 2]>,
    /// x₁, y₁, x₂, y₂.
    pub bbox: [f64; 4],
    /// The faces' area, together.
    pub area: f64,
    /// The edges' mean length.
    pub mean_edge: f64,
    tris: Buckets,
    edge_cells: Buckets,
}

impl Mesh {
    /// A mesh from its nodes and its faces' nodes (indices from 0), each face checked.
    pub fn new(x: Vec<f64>, y: Vec<f64>, faces: &[Vec<u32>]) -> Result<Mesh, RasterError> {
        let n = x.len();
        if n != y.len() || n < 3 {
            return Err(RasterError::new("Ağın en az üç düğümü olmalı."));
        }
        if let Some(k) = (0..n).find(|&k| !(x[k].is_finite() && y[k].is_finite())) {
            return Err(RasterError::new(format!(
                "Ağın {}. düğümünün koordinatı sayı değil.",
                k + 1
            )));
        }
        let p = |k: u32| Vec2 {
            x: x[k as usize],
            y: y[k as usize],
        };
        let mut tri = Vec::with_capacity(faces.len() * 2);
        let mut tri_face = Vec::with_capacity(faces.len() * 2);
        let mut edges: Vec<[u32; 2]> = Vec::with_capacity(faces.len() * 3);
        let mut area = 0.0;
        let mut ring: Vec<u32> = Vec::new();
        let mut pts: Vec<Vec2> = Vec::new();
        let mut idx: Vec<u32> = Vec::new();
        for (f, nodes) in faces.iter().enumerate() {
            let label = f + 1;
            if nodes.len() < 3 {
                return Err(RasterError::new(format!(
                    "Ağın {label}. yüzünün üçten az düğümü var."
                )));
            }
            if let Some(&k) = nodes.iter().find(|&&k| k as usize >= n) {
                return Err(RasterError::new(format!(
                    "Ağın {label}. yüzü olmayan bir düğümü ({}) anıyor.",
                    k as u64 + 1
                )));
            }
            for (i, a) in nodes.iter().enumerate() {
                if nodes[i + 1..].contains(a) {
                    return Err(RasterError::new(format!(
                        "Ağın {label}. yüzünde aynı düğüm iki kez var."
                    )));
                }
            }
            // Signed area by the shoelace, relative to the first node.
            let o = p(nodes[0]);
            let mut twice = 0.0;
            for i in 1..nodes.len() - 1 {
                let (a, b) = (p(nodes[i]), p(nodes[i + 1]));
                twice += (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
            }
            if !(twice != 0.0 && twice.is_finite()) {
                return Err(RasterError::new(format!(
                    "Ağın {label}. yüzünün alanı sıfır."
                )));
            }
            ring.clear();
            ring.extend_from_slice(nodes);
            if twice < 0.0 {
                ring.reverse();
            }
            area += twice.abs() / 2.0;
            let m = ring.len();
            for i in 0..m {
                let (a, b) = (ring[i], ring[(i + 1) % m]);
                edges.push([a.min(b), a.max(b)]);
            }
            // Convex: every turn left or straight.
            let convex = (0..m)
                .all(|i| orient2d(p(ring[i]), p(ring[(i + 1) % m]), p(ring[(i + 2) % m])) >= 0.0);
            if convex {
                for i in 1..m - 1 {
                    let t = [ring[0], ring[i], ring[i + 1]];
                    if orient2d(p(t[0]), p(t[1]), p(t[2])) > 0.0 {
                        tri.push(t);
                        tri_face.push(f as u32);
                    }
                }
            } else {
                pts.clear();
                pts.extend(ring.iter().map(|&k| p(k)));
                idx.clear();
                triangulate_into(&pts, &[Range { start: 0, end: m }], &mut idx);
                for c in idx.chunks_exact(3) {
                    let mut t = [
                        ring[c[0] as usize],
                        ring[c[1] as usize],
                        ring[c[2] as usize],
                    ];
                    let o = orient2d(p(t[0]), p(t[1]), p(t[2]));
                    if o == 0.0 {
                        continue;
                    }
                    if o < 0.0 {
                        t.swap(1, 2);
                    }
                    tri.push(t);
                    tri_face.push(f as u32);
                }
            }
        }
        edges.sort_unstable();
        edges.dedup();
        let mut bbox = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for k in 0..n {
            bbox[0] = bbox[0].min(x[k]);
            bbox[1] = bbox[1].min(y[k]);
            bbox[2] = bbox[2].max(x[k]);
            bbox[3] = bbox[3].max(y[k]);
        }
        let mean_edge = if edges.is_empty() {
            0.0
        } else {
            edges
                .iter()
                .map(|&[a, b]| {
                    math::hypot(x[b as usize] - x[a as usize], y[b as usize] - y[a as usize])
                })
                .sum::<f64>()
                / edges.len() as f64
        };
        let tbox = |k: usize| {
            let [a, b, c] = tri[k];
            let (xa, xb, xc) = (x[a as usize], x[b as usize], x[c as usize]);
            let (ya, yb, yc) = (y[a as usize], y[b as usize], y[c as usize]);
            [
                xa.min(xb).min(xc),
                ya.min(yb).min(yc),
                xa.max(xb).max(xc),
                ya.max(yb).max(yc),
            ]
        };
        let tris = Buckets::build(bbox, tri.len(), tbox);
        let ebox = |k: usize| {
            let [a, b] = edges[k];
            let (xa, xb, ya, yb) = (x[a as usize], x[b as usize], y[a as usize], y[b as usize]);
            [xa.min(xb), ya.min(yb), xa.max(xb), ya.max(yb)]
        };
        let edge_cells = Buckets::build(bbox, edges.len(), ebox);
        Ok(Mesh {
            x,
            y,
            faces: faces.len() as u32,
            tri,
            tri_face,
            edges,
            bbox,
            area,
            mean_edge,
            tris,
            edge_cells,
        })
    }

    #[inline]
    fn pt(&self, k: u32) -> Vec2 {
        Vec2 {
            x: self.x[k as usize],
            y: self.y[k as usize],
        }
    }

    /// The triangle point (`px`, `py`) lies in: the lowest of those whose
    /// closed area holds it; none outside the mesh.
    pub fn locate(&self, px: f64, py: f64) -> Option<u32> {
        if !(px >= self.bbox[0] && px <= self.bbox[2] && py >= self.bbox[1] && py <= self.bbox[3]) {
            return None;
        }
        let q = Vec2 { x: px, y: py };
        let mut best: Option<u32> = None;
        for &t in self.tris.items(self.tris.col(px), self.tris.row(py)) {
            if best.is_some_and(|b| t >= b) {
                continue;
            }
            let [a, b, c] = self.tri[t as usize];
            let (pa, pb, pc) = (self.pt(a), self.pt(b), self.pt(c));
            if px < pa.x.min(pb.x).min(pc.x)
                || px > pa.x.max(pb.x).max(pc.x)
                || py < pa.y.min(pb.y).min(pc.y)
                || py > pa.y.max(pb.y).max(pc.y)
            {
                continue;
            }
            if orient2d(pa, pb, q) >= 0.0
                && orient2d(pb, pc, q) >= 0.0
                && orient2d(pc, pa, q) >= 0.0
            {
                best = Some(t);
            }
        }
        best
    }

    /// The value at (`px`, `py`) of `data` (NaN outside, on an inactive face, or from a value that is nothing).
    pub fn value_at(&self, data: &Data<'_>, px: f64, py: f64) -> f64 {
        match self.locate(px, py) {
            Some(t) => self.value_in(data, t, px, py),
            None => f64::NAN,
        }
    }

    /// The value at a point of triangle `t`.
    pub fn value_in(&self, data: &Data<'_>, t: u32, px: f64, py: f64) -> f64 {
        let y = data.y.map(|yv| move |k: usize| yv.get(k));
        let mask = data.mask.map(|m| move |k: usize| m.get(k));
        self.value_by(t, px, py, data.location, |k| data.x.get(k), y, mask)
    }

    /// [`Mesh::value_in`] with the values (a node's or a face's, by its
    /// place) asked for one by one: the same arithmetic wherever they are kept
    /// (Zaman serisi reads only the places its points need; docs/adr/0243 §9).
    #[allow(clippy::too_many_arguments)]
    pub fn value_by(
        &self,
        t: u32,
        px: f64,
        py: f64,
        location: Location,
        x: impl Fn(usize) -> f64,
        y: Option<impl Fn(usize) -> f64>,
        mask: Option<impl Fn(usize) -> f64>,
    ) -> f64 {
        let face = self.tri_face[t as usize] as usize;
        if mask.is_some_and(|m| !(m(face) != 0.0)) {
            return f64::NAN;
        }
        match location {
            Location::Face => {
                let u = x(face);
                match y {
                    Some(yv) => math::hypot(u, yv(face)),
                    None => u,
                }
            }
            Location::Node => {
                let [a, b, c] = self.tri[t as usize];
                let (pa, pb, pc) = (self.pt(a), self.pt(b), self.pt(c));
                let wa = (pb.x - px) * (pc.y - py) - (pb.y - py) * (pc.x - px);
                let wb = (pc.x - px) * (pa.y - py) - (pc.y - py) * (pa.x - px);
                let wc = (pa.x - px) * (pb.y - py) - (pa.y - py) * (pb.x - px);
                let sum = wa + wb + wc;
                let lerp = |s: &dyn Fn(usize) -> f64| {
                    (wa * s(a as usize) + wb * s(b as usize) + wc * s(c as usize)) / sum
                };
                let u = lerp(&x);
                match y {
                    Some(yv) => math::hypot(u, lerp(&yv)),
                    None => u,
                }
            }
        }
    }

    /// The places a value at a point of triangle `t` reads: its three nodes,
    /// or its face; and the face (its mask's).
    pub fn places_of(&self, t: u32, location: Location) -> (Vec<u32>, u32) {
        let face = self.tri_face[t as usize];
        match location {
            Location::Face => (vec![face], face),
            Location::Node => (self.tri[t as usize].to_vec(), face),
        }
    }

    /// The edges whose box meets `bbox`, each once.
    pub fn edges_in(&self, bbox: [f64; 4], out: &mut Vec<u32>) {
        out.clear();
        if bbox[2] < self.bbox[0]
            || bbox[0] > self.bbox[2]
            || bbox[3] < self.bbox[1]
            || bbox[1] > self.bbox[3]
        {
            return;
        }
        let b = &self.edge_cells;
        let (ix0, iy0, ix1, iy1) = b.span(bbox);
        for iy in iy0..=iy1 {
            for ix in ix0..=ix1 {
                for &e in b.items(ix, iy) {
                    let [p, q] = self.edges[e as usize];
                    let (xa, xb, ya, yb) = (
                        self.x[p as usize],
                        self.x[q as usize],
                        self.y[p as usize],
                        self.y[q as usize],
                    );
                    // The edge is taken in the first cell of the query its box meets.
                    let home = (b.col(xa.min(xb)).max(ix0), b.row(ya.min(yb)).max(iy0));
                    if home != (ix, iy) {
                        continue;
                    }
                    if xa.max(xb) < bbox[0]
                        || xa.min(xb) > bbox[2]
                        || ya.max(yb) < bbox[1]
                        || ya.min(yb) > bbox[3]
                    {
                        continue;
                    }
                    out.push(e);
                }
            }
        }
    }
}

/// A sanal grid's cell for a mesh (§5): an eighth of the faces' mean size
/// (fine enough that the fill is smooth and the mesh's lines thin where its
/// faces fill the view), rounded down to 1, 2 or 5 times a power of ten;
/// larger while a side would pass 65 536 cells.
pub fn default_cell(mesh: &Mesh) -> f64 {
    let mean = (mesh.area / f64::from(mesh.faces.max(1))).sqrt() / 8.0;
    let mut c = nice_below(mean.max(1e-9));
    let (w, h) = (mesh.bbox[2] - mesh.bbox[0], mesh.bbox[3] - mesh.bbox[1]);
    for _ in 0..64 {
        if w / c <= 65_536.0 && h / c <= 65_536.0 {
            break;
        }
        c = nice_above(c * 1.0001);
    }
    c
}

/// The largest of 1, 2, 5 × 10ⁿ not above `v`.
pub fn nice_below(v: f64) -> f64 {
    let e = libm::floor(libm::log10(v));
    let base = libm::pow(10.0, e);
    let m = v / base;
    let k = if m >= 5.0 {
        5.0
    } else if m >= 2.0 {
        2.0
    } else {
        1.0
    };
    k * base
}

/// The smallest of 1, 2, 5 × 10ⁿ not below `v`.
fn nice_above(v: f64) -> f64 {
    let e = libm::floor(libm::log10(v));
    let base = libm::pow(10.0, e);
    let m = v / base;
    let k = if m <= 1.0 {
        1.0
    } else if m <= 2.0 {
        2.0
    } else if m <= 5.0 {
        5.0
    } else {
        10.0
    };
    k * base
}

/// The sanal grid of a mesh with cells of `cell`: its affine (axis-parallel,
/// north up, corners on multiples of the cell) and size.
pub fn grid_of(mesh: &Mesh, cell: f64) -> Option<([f64; 6], u32, u32)> {
    grid_of_box(mesh.bbox, cell)
}

/// [`grid_of`] of a mesh's box (x₁, y₁, x₂, y₂); none past 65 536 cells a side.
pub fn grid_of_box(bbox: [f64; 4], cell: f64) -> Option<([f64; 6], u32, u32)> {
    if !(cell.is_finite() && cell > 0.0) {
        return None;
    }
    let x0 = (bbox[0] / cell).floor() * cell;
    let y0 = (bbox[3] / cell).ceil() * cell;
    let w = ((bbox[2] - x0) / cell).ceil().max(1.0);
    let h = ((y0 - bbox[1]) / cell).ceil().max(1.0);
    if w > 65_536.0 || h > 65_536.0 {
        return None;
    }
    Some(([x0, cell, 0.0, y0, 0.0, -cell], w as u32, h as u32))
}
