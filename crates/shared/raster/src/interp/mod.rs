//! Interpolation (docs/adr/0232 §5–§9, §12): a value at any place from the
//! points, by Ters uzaklık (IDW), Doğal komşu, TIN, Spline or Kriging; and
//! each point's value from the others (cross-validation). A method is
//! prepared once (its index, triangulation or variogram) and then asked
//! cell by cell on the job's threads, each with its own scratch.

pub mod kriging;
pub mod natural;
pub mod spline;

use kentos_geometry_core::geom::delaunay::{Delaunay, Located, NONE, fill_star, next, triangulate};
use kentos_geometry_core::predicates::orient2d;
use kentos_geometry_core::vec2::Vec2;

use crate::index::Index;
use crate::points::Points;
use crate::solve::Lu;
use kriging::Variogram;
use natural::Mesh;
use spline::{Basis, Fit};

/// A method and its settings.
#[derive(Clone, Debug, PartialEq)]
pub enum Method {
    Idw {
        power: f64,
        k: usize,
        radius: f64,
        min: usize,
    },
    Natural,
    Tin,
    Spline {
        basis: Basis,
        k: usize,
    },
    Kriging {
        variogram: Variogram,
        k: usize,
        radius: f64,
    },
}

/// The most neighbours a method takes.
pub const MOST_NEIGHBOURS: usize = 64;

/// A method ready to be asked: the points and what it built over them.
pub struct Prepared {
    pub points: Points,
    pub method: Method,
    index: Option<Index>,
    tri: Option<(Delaunay, Vec<u32>)>,
}

/// One thread's buffers and caches.
#[derive(Default)]
pub struct Scratch {
    near: Vec<(f64, u32)>,
    set: Vec<u32>,
    last: u32,
    cavity: Vec<u32>,
    out: Vec<u32>,
    shares: Vec<(u32, f64)>,
    tmp: Vec<f64>,
    rhs: Vec<f64>,
    rel: Vec<(f64, f64)>,
    pts: Vec<Vec2>,
    vals: Vec<f64>,
    /// The neighbour sets met and their solutions (a set's solution is the
    /// same wherever it is met: the cache only spares the work).
    fits: Sets<Option<Fit>>,
    systems: Sets<Option<Lu>>,
}

/// How many solutions a scratch keeps before it forgets them all (a row
/// and the one before it, in the densest data).
const CACHE: usize = 8_192;

/// A small multiply-and-rotate hash of a neighbour set (no dependency).
#[derive(Default, Clone, Copy)]
struct SetHash(u64);

impl std::hash::Hasher for SetHash {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ u64::from(b)).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
    }

    fn write_u32(&mut self, i: u32) {
        self.0 = (self.0.rotate_left(5) ^ u64::from(i)).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

type Sets<T> = std::collections::HashMap<Vec<u32>, T, std::hash::BuildHasherDefault<SetHash>>;

/// The solution for `set`, worked out by `solve` when the cache has none.
fn cached<'a, T>(cache: &'a mut Sets<T>, set: &[u32], solve: impl FnOnce() -> T) -> &'a T {
    if !cache.contains_key(set) {
        if cache.len() >= CACHE {
            cache.clear();
        }
        cache.insert(set.to_vec(), solve());
    }
    &cache[set]
}

impl Prepared {
    /// `method` over `points`; why not (a triangulation of too few points).
    pub fn new(points: Points, method: Method) -> Result<Prepared, String> {
        let (index, tri) = match method {
            Method::Natural | Method::Tin => {
                let d = triangulate(&points.xy)?;
                let out = d.out_edges(points.len());
                (None, Some((d, out)))
            }
            _ => (Some(Index::new(&points.xy)), None),
        };
        Ok(Prepared {
            points,
            method,
            index,
            tri,
        })
    }

    /// The value at `q` and, for Kriging, its standard error (NaN where none).
    pub fn at(&self, q: Vec2, s: &mut Scratch) -> (f64, f64) {
        self.value(q, None, s)
    }

    /// Point `i`'s value from the others (cross-validation), and Kriging's
    /// standard error; none where there is none (a hull point of a
    /// triangulation, too few neighbours).
    pub fn leave_out(&self, i: u32, s: &mut Scratch) -> Option<(f64, f64)> {
        let q = self.points.xy[i as usize];
        let (v, e) = match self.method {
            Method::Natural | Method::Tin => self.star_value(i, s),
            _ => self.value(q, Some(i), s),
        };
        (!v.is_nan()).then_some((v, e))
    }

    fn value(&self, q: Vec2, skip: Option<u32>, s: &mut Scratch) -> (f64, f64) {
        let p = &self.points;
        match &self.method {
            Method::Idw {
                power,
                k,
                radius,
                min,
            } => {
                let Some(index) = &self.index else {
                    return (f64::NAN, f64::NAN);
                };
                index.nearest(&p.xy, q, *k, *radius, skip, &mut s.near);
                if s.near.is_empty() || s.near.len() < (*min).max(1) {
                    return (f64::NAN, f64::NAN);
                }
                if s.near[0].0 == 0.0 {
                    return (p.v[s.near[0].1 as usize], f64::NAN);
                }
                let half = power / 2.0;
                let (mut num, mut den) = (0.0, 0.0);
                for &(d2, i) in &s.near {
                    let w = 1.0 / libm::pow(d2, half);
                    num += w * p.v[i as usize];
                    den += w;
                }
                (num / den, f64::NAN)
            }
            Method::Tin => (self.tin(q, s), f64::NAN),
            Method::Natural => (self.natural(q, s), f64::NAN),
            Method::Spline { basis, k } => {
                let Some(index) = &self.index else {
                    return (f64::NAN, f64::NAN);
                };
                index.nearest(&p.xy, q, *k, 0.0, skip, &mut s.near);
                if s.near.len() < basis.trend().max(1) {
                    return (f64::NAN, f64::NAN);
                }
                self.neighbour_set(s);
                let first = p.xy[s.set[0] as usize];
                s.rel.clear();
                s.rel.extend(s.set.iter().map(|&i| {
                    let a = p.xy[i as usize];
                    (a.x - first.x, a.y - first.y)
                }));
                let at = (q.x - first.x, q.y - first.y);
                let (rel, tmp) = (&s.rel, &mut s.tmp);
                let fit = cached(&mut s.fits, &s.set, || {
                    let z: Vec<f64> = s.set.iter().map(|&i| p.v[i as usize]).collect();
                    spline::fit(basis, rel, &z, tmp)
                });
                match fit {
                    Some(f) => (f.at(basis, rel, at.0, at.1), f64::NAN),
                    None => (f64::NAN, f64::NAN),
                }
            }
            Method::Kriging {
                variogram,
                k,
                radius,
            } => {
                let Some(index) = &self.index else {
                    return (f64::NAN, f64::NAN);
                };
                index.nearest(&p.xy, q, *k, *radius, skip, &mut s.near);
                if s.near.is_empty() {
                    return (f64::NAN, f64::NAN);
                }
                if s.near[0].0 == 0.0 {
                    return (p.v[s.near[0].1 as usize], 0.0);
                }
                self.neighbour_set(s);
                s.pts.clear();
                s.pts.extend(s.set.iter().map(|&i| p.xy[i as usize]));
                let pts = &s.pts;
                let lu = cached(&mut s.systems, &s.set, || kriging::system(variogram, pts));
                let Some(lu) = lu else {
                    return (f64::NAN, f64::NAN);
                };
                s.vals.clear();
                s.vals.extend(s.set.iter().map(|&i| p.v[i as usize]));
                let (pred, var) =
                    kriging::predict(variogram, lu, pts, &s.vals, q, &mut s.rhs, &mut s.tmp);
                (pred, var.max(0.0).sqrt())
            }
        }
    }

    /// The neighbours' indices in their order (the cache's key, the system's order).
    fn neighbour_set(&self, s: &mut Scratch) {
        s.set.clear();
        s.set.extend(s.near.iter().map(|n| n.1));
        s.set.sort_unstable();
    }

    fn mesh(&self) -> Option<(Mesh<'_>, &Delaunay)> {
        let (d, _) = self.tri.as_ref()?;
        Some((
            Mesh {
                triangles: &d.triangles,
                halfedges: &d.halfedges,
            },
            d,
        ))
    }

    /// Linear in `q`'s triangle; NaN outside the hull.
    fn tin(&self, q: Vec2, s: &mut Scratch) -> f64 {
        let Some((_, d)) = self.mesh() else {
            return f64::NAN;
        };
        match d.locate(&self.points.xy, q, s.last) {
            Located::Inside(t) => {
                s.last = t;
                linear(&self.points, d.corners(t), q)
            }
            Located::Outside(e) => {
                s.last = e / 3;
                f64::NAN
            }
        }
    }

    /// Sibson's value at `q`: a point's own, linear on a hull edge, else Watson's.
    fn natural(&self, q: Vec2, s: &mut Scratch) -> f64 {
        let Some((mesh, d)) = self.mesh() else {
            return f64::NAN;
        };
        let p = &self.points;
        let t = match d.locate(&p.xy, q, s.last) {
            Located::Inside(t) => t,
            Located::Outside(e) => {
                s.last = e / 3;
                return f64::NAN;
            }
        };
        s.last = t;
        for i in d.corners(t) {
            if p.xy[i as usize] == q {
                return p.v[i as usize];
            }
        }
        for k in 0..3 {
            let e = 3 * t + k;
            if d.halfedges[e as usize] == NONE {
                let (a, b) = (d.triangles[e as usize], d.triangles[next(e) as usize]);
                if orient2d(p.xy[a as usize], p.xy[b as usize], q) == 0.0 {
                    return along(p, a, b, q);
                }
            }
        }
        natural::cavity(&mesh, &p.xy, q, t, &mut s.cavity, &mut s.out);
        self.weighted(&mesh, q, s)
    }

    /// Σ λ z over the cavity in `s`.
    fn weighted(&self, mesh: &Mesh<'_>, q: Vec2, s: &mut Scratch) -> f64 {
        let cavity = std::mem::take(&mut s.cavity);
        let ok = natural::shares(mesh, &self.points.xy, q, &cavity, &mut s.shares);
        s.cavity = cavity;
        if !ok {
            return f64::NAN;
        }
        let total: f64 = s.shares.iter().map(|x| x.1).sum();
        if !(total > 0.0) || !total.is_finite() {
            return f64::NAN;
        }
        let mut v = 0.0;
        for &(i, w) in &s.shares {
            v += w / total * self.points.v[i as usize];
        }
        v
    }

    /// Point `i` from the triangulation without it: its neighbours' hole
    /// filled again, linear or Sibson's in it; NaN for a hull point.
    fn star_value(&self, i: u32, s: &mut Scratch) -> (f64, f64) {
        let Some((d, out)) = &self.tri else {
            return (f64::NAN, f64::NAN);
        };
        let p = &self.points;
        let (ring, on_hull) = d.star(out, i);
        if on_hull || ring.len() < 3 {
            return (f64::NAN, f64::NAN);
        }
        let q = p.xy[i as usize];
        let tris = fill_star(&p.xy, &ring);
        let triangles: Vec<u32> = tris.iter().flatten().copied().collect();
        let n = triangles.len();
        let mut halfedges = vec![NONE; n];
        for a in 0..n {
            for b in a + 1..n {
                let (a0, a1) = (triangles[a], triangles[next(a as u32) as usize]);
                let (b0, b1) = (triangles[b], triangles[next(b as u32) as usize]);
                if a0 == b1 && a1 == b0 {
                    halfedges[a] = b as u32;
                    halfedges[b] = a as u32;
                }
            }
        }
        let v = match self.method {
            Method::Tin => {
                let holder = tris.iter().find(|c| {
                    let [a, b, cc] = c.map(|j| p.xy[j as usize]);
                    orient2d(a, b, q) >= 0.0
                        && orient2d(b, cc, q) >= 0.0
                        && orient2d(cc, a, q) >= 0.0
                });
                holder.map_or(f64::NAN, |c| linear(p, *c, q))
            }
            _ => {
                let mesh = Mesh {
                    triangles: &triangles,
                    halfedges: &halfedges,
                };
                s.cavity.clear();
                s.cavity.extend(0..tris.len() as u32);
                self.weighted(&mesh, q, s)
            }
        };
        (v, f64::NAN)
    }
}

/// Linear in triangle `c` (counter-clockwise): λₐ = orient(q, b, c) / orient(a, b, c) and the
/// others alike, each in coordinates relative to a.
fn linear(p: &Points, c: [u32; 3], q: Vec2) -> f64 {
    let [ia, ib, ic] = c;
    let (a, b, cc) = (p.xy[ia as usize], p.xy[ib as usize], p.xy[ic as usize]);
    let (bx, by) = (b.x - a.x, b.y - a.y);
    let (cx, cy) = (cc.x - a.x, cc.y - a.y);
    let (qx, qy) = (q.x - a.x, q.y - a.y);
    let area = bx * cy - by * cx;
    let la = ((bx - qx) * (cy - qy) - (by - qy) * (cx - qx)) / area;
    let lb = (qx * cy - qy * cx) / area;
    let lc = (bx * qy - by * qx) / area;
    la * p.v[ia as usize] + lb * p.v[ib as usize] + lc * p.v[ic as usize]
}

/// Linear between hull points a and b, `q` on their edge.
fn along(p: &Points, a: u32, b: u32, q: Vec2) -> f64 {
    let (pa, pb) = (p.xy[a as usize], p.xy[b as usize]);
    let (dx, dy) = (pb.x - pa.x, pb.y - pa.y);
    let t = ((q.x - pa.x) * dx + (q.y - pa.y) * dy) / (dx * dx + dy * dy);
    p.v[a as usize] * (1.0 - t) + p.v[b as usize] * t
}
