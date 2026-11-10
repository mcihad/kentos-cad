//! An alignment's terrain profile on a surface (docs/adr/0902 §1, ROAD-PROFILE-05): every place the alignment crosses
//! a kept triangle's edge, its height the edge's own (linear along it); a tangent's crossings by line against line, a
//! curve's by circle against line in closed form, a clothoid's where the side of the edge's line changes, refined by
//! bisection to 1e-12 of the element. Inside a triangle the height is its plane, but not linear in station along a
//! curve: between two crossings the alignment is cut into pieces whose chord's sagitta times the plane's slope is at
//! most [`TOLERANCE`], so reading the profile linearly is never more than that off; the elements' ends are points
//! too (a kink bends the path inside a triangle). Off the surface and in a void
//! the profile has a gap: no height is made up.

use std::collections::BTreeSet;

use crate::alignment::{Alignment, Element, Kind};
use crate::surface::{Place, Surface};

/// The most a linear reading of the profile is off the surface between its points, m.
pub const TOLERANCE: f64 = 1e-3;

/// A point of a terrain profile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProfilePoint {
    pub station: f64,
    /// None off the surface.
    pub z: Option<f64>,
    /// The profile from the previous point to this one is off the surface (outside or in a void).
    pub gap: bool,
    /// It is where the alignment crosses a triangle's edge.
    pub crossing: bool,
}

/// The kept triangles' edges, each once, as their corners.
fn edges(s: &Surface) -> Vec<([f64; 3], [f64; 3])> {
    let set: BTreeSet<[u32; 2]> = s
        .triangles
        .iter()
        .flat_map(|t| [[t[0], t[1]], [t[1], t[2]], [t[2], t[0]]])
        .map(|[a, b]| if a < b { [a, b] } else { [b, a] })
        .collect();
    set.into_iter().map(|[a, b]| (s.vertices[a as usize], s.vertices[b as usize])).collect()
}

/// The element's bounding box, a little wider.
fn bounds(e: &Element) -> ([f64; 2], [f64; 2]) {
    let turn = e.k0.abs().max(e.k1.abs()) * e.length;
    let n = ((turn / 0.05).ceil() as usize).clamp(1, 4096);
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for i in 0..=n {
        let p = e.point(e.length * i as f64 / n as f64);
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    // The chords' sagitta at most turn²·L/8 per piece; one metre more covers it.
    let pad = 1.0 + e.length * (turn / n as f64).powi(2);
    ([lo[0] - pad, lo[1] - pad], [hi[0] + pad, hi[1] + pad])
}

/// Where an element crosses the edge a–b: (s along the element, u along the edge).
fn crossings(e: &Element, a: [f64; 3], b: [f64; 3]) -> Vec<(f64, f64)> {
    let r = [b[0] - a[0], b[1] - a[1]];
    let len2 = r[0] * r[0] + r[1] * r[1];
    if len2 == 0.0 {
        return vec![];
    }
    let u_of = |p: [f64; 2]| ((p[0] - a[0]) * r[0] + (p[1] - a[1]) * r[1]) / len2;
    let keep = |s: f64, u: f64| (-1e-12..=e.length + 1e-12).contains(&s) && (-1e-12..=1.0 + 1e-12).contains(&u);
    match e.kind {
        Kind::Tangent => {
            let d = [e.theta0.cos(), e.theta0.sin()];
            let denom = d[0] * r[1] - d[1] * r[0];
            if denom == 0.0 {
                return vec![];
            }
            let ap = [a[0] - e.start[0], a[1] - e.start[1]];
            let s = (ap[0] * r[1] - ap[1] * r[0]) / denom;
            let u = (ap[0] * d[1] - ap[1] * d[0]) / denom;
            if keep(s, u) { vec![(s.clamp(0.0, e.length), u.clamp(0.0, 1.0))] } else { vec![] }
        }
        Kind::Arc => {
            let k = e.k0;
            let n = [-e.theta0.sin(), e.theta0.cos()];
            let c = [e.start[0] + n[0] / k, e.start[1] + n[1] / k];
            let rad = 1.0 / k.abs();
            // |a + u·r − c|² = R²
            let f = [a[0] - c[0], a[1] - c[1]];
            let (qa, qb, qc) = (len2, 2.0 * (f[0] * r[0] + f[1] * r[1]), f[0] * f[0] + f[1] * f[1] - rad * rad);
            let disc = qb * qb - 4.0 * qa * qc;
            if disc < 0.0 {
                return vec![];
            }
            let sq = disc.sqrt();
            // The stable pair of roots.
            let q = -0.5 * (qb + if qb >= 0.0 { sq } else { -sq });
            let roots = if q == 0.0 { vec![0.0] } else { vec![q / qa, qc / q] };
            let r0 = [e.start[0] - c[0], e.start[1] - c[1]];
            let tau = std::f64::consts::TAU;
            let mut out = Vec::new();
            for u in roots {
                let p = [a[0] + u * r[0] - c[0], a[1] + u * r[1] - c[1]];
                let alpha = (r0[0] * p[1] - r0[1] * p[0]).atan2(r0[0] * p[0] + r0[1] * p[1]);
                for al in [alpha, alpha - tau, alpha + tau] {
                    let s = al / k;
                    if keep(s, u) {
                        out.push((s.clamp(0.0, e.length), u.clamp(0.0, 1.0)));
                    }
                }
            }
            out
        }
        Kind::Spiral => {
            let side = |s: f64| {
                let p = e.point(s);
                (p[0] - a[0]) * r[1] - (p[1] - a[1]) * r[0]
            };
            let turn = e.k0.abs().max(e.k1.abs()) * e.length;
            let steps = ((turn / 0.01).ceil() as usize).max((e.length / 1.0).ceil() as usize).clamp(8, 1 << 16);
            let mut out = Vec::new();
            let mut lo = 0.0;
            let mut f_lo = side(lo);
            for i in 1..=steps {
                let hi = e.length * i as f64 / steps as f64;
                let f_hi = side(hi);
                if f_lo == 0.0 && i == 1 {
                    let u = u_of(e.point(0.0));
                    if keep(0.0, u) {
                        out.push((0.0, u.clamp(0.0, 1.0)));
                    }
                }
                if (f_lo < 0.0 && f_hi >= 0.0) || (f_lo > 0.0 && f_hi <= 0.0) {
                    let (mut x0, mut x1, mut g0) = (lo, hi, f_lo);
                    for _ in 0..200 {
                        let m = 0.5 * (x0 + x1);
                        let gm = side(m);
                        if (gm < 0.0) == (g0 < 0.0) && gm != 0.0 {
                            x0 = m;
                            g0 = gm;
                        } else {
                            x1 = m;
                        }
                        if x1 - x0 <= 1e-12 * e.length.max(1.0) {
                            break;
                        }
                    }
                    let s = 0.5 * (x0 + x1);
                    let u = u_of(e.point(s));
                    if keep(s, u) {
                        out.push((s, u.clamp(0.0, 1.0)));
                    }
                }
                lo = hi;
                f_lo = f_hi;
            }
            out
        }
    }
}

/// The slope of a kept triangle's plane, |∇z|.
fn slope(s: &Surface, t: u32) -> f64 {
    let [a, b, c] = s.triangles[t as usize].map(|i| s.vertices[i as usize]);
    let (ux, uy, uz) = (b[0] - a[0], b[1] - a[1], b[2] - a[2]);
    let (vx, vy, vz) = (c[0] - a[0], c[1] - a[1], c[2] - a[2]);
    let det = ux * vy - uy * vx;
    if det == 0.0 {
        return 0.0;
    }
    let gx = (uz * vy - vz * uy) / det;
    let gy = (ux * vz - vx * uz) / det;
    gx.hypot(gy)
}

/// The terrain profile of `al` on `surface`, in station order.
pub fn terrain(surface: &Surface, al: &Alignment) -> Vec<ProfilePoint> {
    // Nothing asks it to stop.
    terrain_told(surface, al, &|_, _| false).unwrap_or_default()
}

/// The parts of the whole [`terrain_told`] tells its progress in, per half.
const HALF: u64 = 1000;
/// The surface's edges looked at between two ticks of [`terrain_told`].
const BLOCK: usize = 4096;

/// [`terrain`] telling `tick` how far it has come (`done` of 2 × 1000, docs/adr/4860): the elements' crossings with the
/// surface's edges the first half, a block of edges at a time; the points' heights the second, a point at a time.
/// `tick` answering true stops it there: none. It does the same reckonings in the same order: the profile is
/// [`terrain`]'s.
pub fn terrain_told(surface: &Surface, al: &Alignment, tick: &dyn Fn(u64, u64) -> bool) -> Option<Vec<ProfilePoint>> {
    let whole = 2 * HALF;
    let all = edges(surface);
    let blocks = all.len().div_ceil(BLOCK) as u64;
    let units = (al.elements.len() as u64 * blocks).max(1);
    // (station, height from an edge)
    // The elements' ends too: at a kink the path bends inside a triangle.
    let mut at: Vec<(f64, Option<f64>)> = vec![(0.0, None), (al.length, None)];
    at.extend(al.elements.iter().map(|e| (e.station, None)));
    for (k, e) in al.elements.iter().enumerate() {
        let (lo, hi) = bounds(e);
        for (j, block) in all.chunks(BLOCK).enumerate() {
            if tick((k as u64 * blocks + j as u64) * HALF / units, whole) {
                return None;
            }
            for &(a, b) in block {
                if a[0].max(b[0]) < lo[0] || a[0].min(b[0]) > hi[0] || a[1].max(b[1]) < lo[1] || a[1].min(b[1]) > hi[1] {
                    continue;
                }
                for (s, u) in crossings(e, a, b) {
                    at.push((e.station + s, Some(a[2] + u * (b[2] - a[2]))));
                }
            }
        }
    }
    at.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut kept: Vec<(f64, Option<f64>)> = Vec::new();
    for (s, z) in at {
        match kept.last_mut() {
            Some(last) if s - last.0 < 1e-9 => {
                if last.1.is_none() {
                    last.1 = z;
                }
            }
            _ => kept.push((s, z)),
        }
    }
    let place = |s: f64| {
        let p = al.at(s).map_or([f64::NAN; 2], |l| [l.x, l.y]);
        surface.height(p[0], p[1])
    };
    let mut out: Vec<ProfilePoint> = Vec::new();
    let count = kept.len().max(1) as u64;
    for (i, &(s, z)) in kept.iter().enumerate() {
        if tick(HALF + i as u64 * HALF / count, whole) {
            return None;
        }
        if i > 0 {
            let s0 = kept[i - 1].0;
            let mid = place(0.5 * (s0 + s));
            let gap = mid.place != Place::Inside;
            // Pieces short enough that their sagitta times the plane's slope is within the tolerance.
            if !gap
                && let Some(t) = mid.triangle
            {
                let kmax = al
                    .elements
                    .iter()
                    .filter(|e| e.station < s && e.end_station() > s0)
                    .map(|e| e.k0.abs().max(e.k1.abs()))
                    .fold(0.0, f64::max);
                let g = slope(surface, t);
                let len = s - s0;
                let n = if kmax == 0.0 || g == 0.0 {
                    1
                } else {
                    ((len * len * kmax * g / (8.0 * TOLERANCE)).sqrt().ceil() as usize).clamp(1, 1 << 16)
                };
                for j in 1..n {
                    let sj = s0 + len * j as f64 / n as f64;
                    out.push(ProfilePoint { station: sj, z: place(sj).z, gap: false, crossing: false });
                }
            }
            let z = z.or_else(|| place(s).z);
            out.push(ProfilePoint { station: s, z, gap, crossing: kept[i].1.is_some() });
        } else {
            out.push(ProfilePoint { station: s, z: z.or_else(|| place(s).z), gap: false, crossing: z.is_some() });
        }
    }
    Some(out)
}
