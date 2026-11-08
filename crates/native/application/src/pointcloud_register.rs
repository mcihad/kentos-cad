//! `cad.pointclouds.register` v1 (plan/map-cad.md MAP-13, docs/adr/1800): Tarama kaydı denetimi, read only.
//!
//! The two clouds are read once at full resolution where their plan boxes meet (widened by the reach) and thinned to
//! one point per voxel, the mean of its points (voxels in key order). The fixed cloud's normal at a point is the
//! eigenvector of the smallest eigenvalue of its `neighbours` nearest points' covariance, turned upward (none unless
//! the middle eigenvalue is at least four times it). Each step pairs every moving point with the nearest fixed point within the
//! reach that has a normal (ties by the lower index), and solves Low's linearisation (2004): for r = (p − q)·n and
//! a = [(p − c) × n, n], (Σ a aᵀ) x = −Σ a r, x = [ω, φ, κ, tx, ty, tz], by the normal matrix's eigenvectors
//! (eigenvalues under 10⁻¹² of the largest left out); the step's rotation Rz(κ)·Ry(φ)·Rx(ω) about the centre c (the
//! fixed points' mean) is put before the transform so far. It ends when a step moves no point more than the
//! tolerance (its translation, and its angles times the moving points' RMS radius) or after `iterations` steps. The
//! nearest searches are exact, over a grid of twice the voxel. Asked, each step leaves out the pairs whose fixed point
//! is on its cloud's edge and those whose two normals lie further apart than an angle (docs/adr/2700). The shared cases
//! are `fixtures/commands/v1/cad.pointclouds.register.json`.

use std::collections::{BTreeMap, HashMap};

use kentos_commands::{
    CloudRules, CommandResult, CommandWarning, Entity, LayerNodeType, OverlapControlStats, PointCloudsRegister,
    PointCloudsRegistered, REGISTER_MOST_ITERATIONS, REGISTER_MOST_POINTS, RegisterCell, RegisterControl,
    RegisterRejectReport, RegisterRobustReport, cloud_rules_default,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::pointcloud_areas::Scan;
use crate::pointcloud_overlap::Sums;
use crate::pointclouds::clouds;

const FIELD: &str = "Ad";
const MOST_WORST: usize = 20;

type P = [f64; 3];

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: P, b: P) -> P {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// A symmetric matrix's eigenvalues and eigenvectors (columns), by cyclic Jacobi rotations; ascending.
fn eigen<const N: usize>(m: [[f64; N]; N]) -> ([f64; N], [[f64; N]; N]) {
    let mut a = m;
    let mut v = [[0.0; N]; N];
    for (i, row) in v.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    for _ in 0..100 {
        let off: f64 = (0..N).flat_map(|i| (0..N).filter(move |j| *j != i).map(move |j| (i, j))).map(|(i, j)| a[i][j] * a[i][j]).sum();
        if off <= 1e-300 {
            break;
        }
        for p in 0..N {
            for q in p + 1..N {
                if a[p][q] == 0.0 {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..N {
                    let (akp, akq) = (a[k][p], a[k][q]);
                    a[k][p] = c * akp - s * akq;
                    a[k][q] = s * akp + c * akq;
                }
                for k in 0..N {
                    let (apk, aqk) = (a[p][k], a[q][k]);
                    a[p][k] = c * apk - s * aqk;
                    a[q][k] = s * apk + c * aqk;
                }
                for row in v.iter_mut() {
                    let (vkp, vkq) = (row[p], row[q]);
                    row[p] = c * vkp - s * vkq;
                    row[q] = s * vkp + c * vkq;
                }
            }
        }
    }
    let mut order: Vec<usize> = (0..N).collect();
    order.sort_by(|i, j| a[*i][*i].total_cmp(&a[*j][*j]));
    let mut values = [0.0; N];
    let mut vectors = [[0.0; N]; N];
    for (k, &i) in order.iter().enumerate() {
        values[k] = a[i][i];
        for r in 0..N {
            vectors[r][k] = v[r][i];
        }
    }
    (values, vectors)
}

/// A uniform grid over points for exact nearest searches.
struct Grid<'a> {
    points: &'a [P],
    g: f64,
    cells: HashMap<(i64, i64, i64), Vec<u32>>,
    lo: (i64, i64, i64),
    hi: (i64, i64, i64),
}

impl<'a> Grid<'a> {
    fn new(points: &'a [P], g: f64) -> Self {
        let mut cells: HashMap<(i64, i64, i64), Vec<u32>> = HashMap::new();
        let (mut lo, mut hi) = ((i64::MAX, i64::MAX, i64::MAX), (i64::MIN, i64::MIN, i64::MIN));
        for (i, p) in points.iter().enumerate() {
            let k = key(*p, g);
            lo = (lo.0.min(k.0), lo.1.min(k.1), lo.2.min(k.2));
            hi = (hi.0.max(k.0), hi.1.max(k.1), hi.2.max(k.2));
            cells.entry(k).or_default().push(i as u32);
        }
        Grid { points, g, cells, lo, hi }
    }

    /// The cells at Chebyshev ring `r` around `c`, each point's squared distance to `q` given to `see`.
    fn ring(&self, c: (i64, i64, i64), r: i64, q: P, see: &mut dyn FnMut(f64, u32)) {
        for dx in -r..=r {
            for dy in -r..=r {
                for dz in -r..=r {
                    if dx.abs().max(dy.abs()).max(dz.abs()) != r {
                        continue;
                    }
                    if let Some(list) = self.cells.get(&(c.0 + dx, c.1 + dy, c.2 + dz)) {
                        for &i in list {
                            let d = sub(self.points[i as usize], q);
                            see(dot(d, d), i);
                        }
                    }
                }
            }
        }
    }

    /// The farthest ring that can hold a point.
    fn last_ring(&self, c: (i64, i64, i64)) -> i64 {
        [c.0 - self.lo.0, self.hi.0 - c.0, c.1 - self.lo.1, self.hi.1 - c.1, c.2 - self.lo.2, self.hi.2 - c.2]
            .into_iter()
            .fold(0, i64::max)
    }

    /// The `k` nearest points to `q`, by squared distance, then index.
    fn nearest_k(&self, q: P, k: usize) -> Vec<(f64, u32)> {
        let c = key(q, self.g);
        let last = self.last_ring(c);
        let mut best: Vec<(f64, u32)> = Vec::with_capacity(k + 1);
        for r in 0..=last {
            self.ring(c, r, q, &mut |d, i| {
                let at = best.partition_point(|b| (b.0, b.1) < (d, i));
                if at < k {
                    best.insert(at, (d, i));
                    best.truncate(k);
                }
            });
            let bound = r as f64 * self.g;
            if best.len() == k && best[k - 1].0 <= bound * bound {
                break;
            }
        }
        best
    }

    /// Every point within `radius` of `q` (the boundary included), with its squared distance, in no set order.
    fn within(&self, q: P, radius: f64) -> Vec<(f64, u32)> {
        let c = key(q, self.g);
        let rings = ((radius / self.g).ceil() as i64).min(self.last_ring(c));
        let mut out = Vec::new();
        for r in 0..=rings {
            self.ring(c, r, q, &mut |d, i| {
                if d <= radius * radius {
                    out.push((d, i));
                }
            });
        }
        out
    }

    /// The nearest point to `q` within `reach`, ties by index.
    fn nearest(&self, q: P, reach: f64) -> Option<(f64, u32)> {
        let c = key(q, self.g);
        let last = self.last_ring(c);
        let mut best: Option<(f64, u32)> = None;
        for r in 0..=last {
            self.ring(c, r, q, &mut |d, i| {
                if best.is_none_or(|b| (d, i) < b) {
                    best = Some((d, i));
                }
            });
            let bound = r as f64 * self.g;
            if best.is_some_and(|b| b.0 <= bound * bound) || bound > reach {
                break;
            }
        }
        best.filter(|b| b.0 <= reach * reach)
    }
}

fn key(p: P, g: f64) -> (i64, i64, i64) {
    ((p[0] / g).floor() as i64, (p[1] / g).floor() as i64, (p[2] / g).floor() as i64)
}

/// A rigid transform about a centre: p ↦ R(p − c) + c + t.
#[derive(Clone, Copy)]
struct Rigid {
    r: [[f64; 3]; 3],
    t: P,
}

impl Rigid {
    fn identity() -> Self {
        Rigid { r: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], t: [0.0; 3] }
    }

    fn apply(&self, p: P, c: P) -> P {
        let d = sub(p, c);
        let r = &self.r;
        [
            r[0][0] * d[0] + r[0][1] * d[1] + r[0][2] * d[2] + c[0] + self.t[0],
            r[1][0] * d[0] + r[1][1] * d[1] + r[1][2] * d[2] + c[1] + self.t[1],
            r[2][0] * d[0] + r[2][1] * d[1] + r[2][2] * d[2] + c[2] + self.t[2],
        ]
    }

    /// The step (rotation `w` about the centre, translation `t`) after this transform.
    fn then(&self, w: P, t: P) -> Rigid {
        let s = rotation(w);
        let mut r = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                r[i][j] = s[i][0] * self.r[0][j] + s[i][1] * self.r[1][j] + s[i][2] * self.r[2][j];
            }
        }
        let st = [dot(s[0], self.t), dot(s[1], self.t), dot(s[2], self.t)];
        Rigid { r, t: [st[0] + t[0], st[1] + t[1], st[2] + t[2]] }
    }
}

/// Rz(κ)·Ry(φ)·Rx(ω).
fn rotation(w: P) -> [[f64; 3]; 3] {
    let (sa, ca) = w[0].sin_cos();
    let (sb, cb) = w[1].sin_cos();
    let (sg, cg) = w[2].sin_cos();
    [
        [cg * cb, -sg * ca + cg * sb * sa, sg * sa + cg * sb * ca],
        [sg * cb, cg * ca + sg * sb * sa, -cg * sa + sg * sb * ca],
        [-sb, cb * sa, cb * ca],
    ]
}

/// The voxel points: each voxel's points' mean, voxels in key order.
fn voxels(sums: &BTreeMap<(i64, i64, i64), (f64, f64, f64, f64)>) -> Vec<P> {
    sums.values().map(|(x, y, z, n)| [x / n, y / n, z / n]).collect()
}

/// A pair: the moving point's index, its place now, the fixed point's index, the residual along its normal.
struct Pair {
    at: P,
    fixed: usize,
    r: f64,
}

/// The rejections asked (docs/adr/2700): the fixed points on their cloud's edge, and the moving points' own normals
/// with the cosine their angle to the fixed normal must reach.
#[derive(Clone, Copy, Default)]
struct Rule<'a> {
    edge: Option<&'a [bool]>,
    moving: Option<(&'a [Option<P>], f64)>,
}

/// The pairs now, and how many the rejections left out: at an edge, by the normals' angle.
fn pairs(
    moving: &[P],
    now: &Rigid,
    c: P,
    grid: &Grid<'_>,
    normals: &[Option<P>],
    fixed: &[P],
    reach: f64,
    rule: Rule<'_>,
) -> (Vec<Pair>, u32, u32) {
    let mut out = Vec::new();
    let (mut edges, mut angle) = (0u32, 0u32);
    for (j, m) in moving.iter().enumerate() {
        let at = now.apply(*m, c);
        let Some((_, i)) = grid.nearest(at, reach) else { continue };
        let Some(n) = normals[i as usize] else { continue };
        if rule.edge.is_some_and(|e| e[i as usize]) {
            edges += 1;
            continue;
        }
        if let Some((own, cos)) = rule.moving {
            // The moving normal turns with the cloud; the lines' angle, whichever way each normal points.
            let keep = own[j].is_some_and(|o| {
                let r = &now.r;
                let turned = [dot(r[0], o), dot(r[1], o), dot(r[2], o)];
                dot(n, turned).abs() >= cos
            });
            if !keep {
                angle += 1;
                continue;
            }
        }
        out.push(Pair { at, fixed: i as usize, r: dot(sub(at, fixed[i as usize]), n) });
    }
    (out, edges, angle)
}

/// A point's normal from its `k` nearest points of its cloud: the eigenvector of the smallest eigenvalue of their
/// covariance (each term over k), turned upward, else north, else east (the signed residual is the moving cloud above
/// the fixed one); none unless the middle eigenvalue is at least four times the smallest.
fn normal_of(points: &[P], grid: &Grid<'_>, p: P, k: usize) -> Option<P> {
    if k < 3 {
        return None;
    }
    let near = grid.nearest_k(p, k);
    let kk = near.len() as f64;
    let mut mean = [0.0; 3];
    for (_, i) in &near {
        let q = points[*i as usize];
        mean = [mean[0] + q[0], mean[1] + q[1], mean[2] + q[2]];
    }
    mean = [mean[0] / kk, mean[1] / kk, mean[2] / kk];
    let mut cov = [[0.0; 3]; 3];
    for (_, i) in &near {
        let d = sub(points[*i as usize], mean);
        for a in 0..3 {
            for b in 0..3 {
                cov[a][b] += d[a] * d[b] / kk;
            }
        }
    }
    let (values, vectors) = eigen(cov);
    let n = [vectors[0][0], vectors[1][0], vectors[2][0]];
    let flip = n[2] < 0.0 || (n[2] == 0.0 && (n[1] < 0.0 || (n[1] == 0.0 && n[0] < 0.0)));
    let n = if flip { [-n[0], -n[1], -n[2]] } else { n };
    (values[1] > 0.0 && values[1] >= 4.0 * values[0].max(0.0)).then_some(n)
}

/// Whether a point is on its cloud's edge (docs/adr/2700; PCL BoundaryEstimation's angle criterion, its radius search):
/// the directions of its neighbours (the points within the reach) in its tangent plane (u ⟂ n, v = n × u; the point
/// itself and a neighbour straight above it left out) leave a gap over 90°; fewer than two directions is an edge too.
fn on_edge(points: &[P], p: P, n: P, near: &[(f64, u32)]) -> bool {
    let e = if n[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0] };
    let u = cross(n, e);
    let len = dot(u, u).sqrt();
    let u = [u[0] / len, u[1] / len, u[2] / len];
    let v = cross(n, u);
    let mut angles: Vec<f64> = near
        .iter()
        .filter(|(d, _)| *d > 0.0)
        .filter_map(|(_, i)| {
            let q = sub(points[*i as usize], p);
            let (a, b) = (dot(q, u), dot(q, v));
            (a != 0.0 || b != 0.0).then(|| b.atan2(a))
        })
        .collect();
    if angles.len() < 2 {
        return true;
    }
    angles.sort_by(f64::total_cmp);
    let mut gap = 2.0 * std::f64::consts::PI - (angles[angles.len() - 1] - angles[0]);
    for w in angles.windows(2) {
        gap = gap.max(w[1] - w[0]);
    }
    gap > std::f64::consts::FRAC_PI_2
}

/// The normal matrix Σ w a aᵀ and Σ w a r of the pairs, each pair's weight `weights[k]` (none: 1).
fn normal_matrix(ps: &[Pair], normals: &[Option<P>], c: P, weights: Option<&[f64]>) -> ([[f64; 6]; 6], [f64; 6]) {
    let mut m = [[0.0; 6]; 6];
    let mut v = [0.0; 6];
    for (k, p) in ps.iter().enumerate() {
        let wk = weights.map_or(1.0, |w| w[k]);
        if wk == 0.0 {
            continue;
        }
        let n = normals[p.fixed].unwrap_or([0.0; 3]);
        let w = cross(sub(p.at, c), n);
        let a = [w[0], w[1], w[2], n[0], n[1], n[2]];
        for i in 0..6 {
            v[i] += wk * a[i] * p.r;
            for j in 0..6 {
                m[i][j] += wk * a[i] * a[j];
            }
        }
    }
    (m, v)
}

/// Huber's tuning constant when none is given: 95 % efficiency at the normal distribution (Holland and Welsch 1977).
pub const HUBER_K: f64 = 1.345;
/// The median absolute residual over this is the normal distribution's σ (1 / Φ⁻¹(3/4)).
const MAD_SIGMA: f64 = 1.4826;

/// The robust weights asked (docs/adr/2300).
enum Robust {
    Huber(f64),
    Trimmed(f64),
}

/// The pairs' weights by `robust`, and the residuals' scale (Huber's): `huber` 1 within k·s and k·s / |r| beyond it,
/// s = 1.4826 × the median |r| (the middle one, or the mean of the two middle ones); `trimmed` 1 for the ⌈ratio·n⌉
/// pairs of the smallest |r| (the earlier pair first among equals) and 0 for the rest.
fn weights(ps: &[Pair], robust: &Robust) -> (Vec<f64>, Option<f64>) {
    let n = ps.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| ps[a].r.abs().total_cmp(&ps[b].r.abs()).then(a.cmp(&b)));
    match robust {
        Robust::Huber(k) => {
            if n == 0 {
                return (Vec::new(), None);
            }
            let abs = |i: usize| ps[order[i]].r.abs();
            let median = if n % 2 == 1 { abs(n / 2) } else { (abs(n / 2 - 1) + abs(n / 2)) / 2.0 };
            let s = MAD_SIGMA * median;
            let cut = k * s;
            let w = ps.iter().map(|p| if cut <= 0.0 || p.r.abs() <= cut { 1.0 } else { cut / p.r.abs() }).collect();
            (w, Some(s))
        }
        Robust::Trimmed(ratio) => {
            let keep = ((ratio * n as f64).ceil() as usize).min(n);
            let mut w = vec![0.0; n];
            for &i in order.iter().take(keep) {
                w[i] = 1.0;
            }
            (w, None)
        }
    }
}

/// The least-squares step: −(Σ a aᵀ)⁺ Σ a r over its eigenvalues above 10⁻¹² of the largest.
fn step(m: [[f64; 6]; 6], v: [f64; 6]) -> [f64; 6] {
    let (values, vectors) = eigen(m);
    let top = values[5];
    let mut x = [0.0; 6];
    if top <= 0.0 {
        return x;
    }
    for k in 0..6 {
        if values[k] <= 1e-12 * top {
            continue;
        }
        let along: f64 = (0..6).map(|i| vectors[i][k] * v[i]).sum::<f64>() / values[k];
        for i in 0..6 {
            x[i] -= vectors[i][k] * along;
        }
    }
    x
}

fn run(doc: &Document, input: &PointCloudsRegister) -> Result<(PointCloudsRegistered, Vec<CommandWarning>), Stop> {
    if input.uids.len() != 2 || input.uids[0] == input.uids[1] {
        return Err(fail(codes::INVALID_INPUT, "İki ayrı nokta bulutu verin: sabit tarama ve ona oturtulacak tarama.".into(), "uids"));
    }
    checks::uids(&input.uids, "")?;
    let chosen = clouds(doc, &input.uids)?;
    let positive = |v: Option<f64>, default: f64, path: &str, what: &str| -> Result<f64, Stop> {
        let v = v.unwrap_or(default);
        if v.is_finite() && v > 0.0 {
            Ok(v)
        } else {
            Err(fail(codes::INVALID_INPUT, format!("{what} sonlu ve sıfırdan büyük olmalı (yazılan {v})."), path))
        }
    };
    let spacing = positive(input.spacing, 0.5, "spacing", "Voksel kenarı")?;
    let reach = positive(input.reach, 1.0, "reach", "Eşleşme erişimi")?;
    let iterations = input.iterations.unwrap_or(30);
    if !(1..=REGISTER_MOST_ITERATIONS).contains(&iterations) {
        return Err(fail(codes::INVALID_INPUT, format!("Yineleme sayısı 1 ile {REGISTER_MOST_ITERATIONS} arasında olmalı."), "iterations"));
    }
    let tolerance = positive(input.tolerance, 0.0001, "tolerance", "Durma toleransı")?;
    let neighbours = input.neighbours.unwrap_or(8);
    if !(3..=32).contains(&neighbours) {
        return Err(fail(codes::INVALID_INPUT, "Normal için komşu sayısı 3 ile 32 arasında olmalı.".into(), "neighbours"));
    }
    let cell = positive(input.cell, 2.0, "cell", "Kapsam hücresi")?;
    let least = input.least.unwrap_or(4);
    if least == 0 {
        return Err(fail(codes::INVALID_INPUT, "Bir kapsam hücresi için en az 1 eşleşme gerekir.".into(), "least"));
    }
    let radius = positive(input.radius, cell, "radius", "Kontrol noktasının çevresi")?;
    let scale = match input.scale {
        Some(0) => return Err(fail(codes::INVALID_INPUT, "Ölçek paydası 0 olamaz.".into(), "scale")),
        Some(s) => s,
        None => doc.settings().plot_scale.round().max(1.0) as u32,
    };
    // Robust weights (docs/adr/2300): the user's kind and value; Huber's k the literature's when not given.
    let robust = match &input.robust {
        None => None,
        Some(r) => match r.kind.trim() {
            "huber" => {
                let k = r.k.unwrap_or(HUBER_K);
                if !(k.is_finite() && k > 0.0) {
                    return Err(fail(codes::INVALID_INPUT, format!("Huber'in k'si sonlu ve sıfırdan büyük olmalı (yazılan {k})."), "robust.k"));
                }
                Some(Robust::Huber(k))
            }
            "trimmed" => {
                let Some(ratio) = r.ratio else {
                    return Err(fail(
                        codes::INVALID_INPUT,
                        "Kırpılmış ICP için tutulacak eşleşme payını (ratio, 0'dan büyük, en çok 1) verin; KentOS örtüşmeyi tahmin etmez.".into(),
                        "robust.ratio",
                    ));
                };
                if !(ratio.is_finite() && ratio > 0.0 && ratio <= 1.0) {
                    return Err(fail(codes::INVALID_INPUT, format!("Tutulacak eşleşme payı 0'dan büyük, en çok 1 olmalı (yazılan {ratio})."), "robust.ratio"));
                }
                Some(Robust::Trimmed(ratio))
            }
            other => {
                return Err(fail(
                    codes::INVALID_INPUT,
                    format!("“{other}” bir ağırlık türü değil; huber ya da trimmed yazın."),
                    "robust.kind",
                ));
            }
        },
    };
    // The pairs left out (docs/adr/2700): at the fixed cloud's edge, or by the normals' angle (the user's degrees).
    let reject = match &input.reject {
        None => None,
        Some(r) => {
            if !r.edges && r.angle.is_none() {
                return Err(fail(
                    codes::INVALID_INPUT,
                    "Ayıklama için kenardaki eşleşmeleri (edges) ya da normal açısını (angle) verin.".into(),
                    "reject",
                ));
            }
            if let Some(a) = r.angle
                && !(a.is_finite() && a > 0.0 && a < 90.0)
            {
                return Err(fail(codes::INVALID_INPUT, format!("Normal açısı 0° ile 90° arasında olmalı (yazılan {a})."), "reject.angle"));
            }
            Some(r.clone())
        }
    };
    let rules: CloudRules = match &input.rules {
        Some(r) => r.clone(),
        None => cloud_rules_default().ok_or_else(|| fail(codes::INVALID_INPUT, "Yerleşik kural paketi okunamadı.".into(), "rules"))?,
    };
    if rules.id.trim().is_empty() || rules.source.trim().is_empty() {
        return Err(fail(codes::INVALID_INPUT, "Kural paketinin kimliği ya da kaynağı boş; sınırın dayanağı yazılmadan denetim yapılmaz.".into(), "rules"));
    }
    if rules.xy.iter().chain([&rules.z_factor, &rules.interswath]).any(|v| !v.is_finite()) {
        return Err(fail(codes::INVALID_INPUT, "Kural paketinin bir sınırı sonlu bir sayı değil.".into(), "rules"));
    }
    // The control points: named, with a height.
    let field = input.field.as_deref().map(str::trim).filter(|f| !f.is_empty()).unwrap_or(FIELD);
    let mut controls: Vec<(String, f64, f64, f64)> = Vec::new();
    if let Some(layer) = input.control.as_deref().map(str::trim) {
        if !doc.layers().get(layer).is_some_and(|n| n.kind == LayerNodeType::Layer) {
            return Err(fail(codes::LAYER_NOT_FOUND, format!("“{layer}” katmanı çizimde yok."), "control"));
        }
        for e in doc.entities().filter(|e| e.base().layer_id == layer) {
            let Entity::Point(p) = e else { continue };
            let (Some(name), Some(z)) = (e.base().attrs.get(field).map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()), p.z) else {
                continue;
            };
            if !controls.iter().any(|c| c.0 == name) {
                controls.push((name, p.p.x, p.p.y, z));
            }
        }
    }
    // Where the two boxes meet, widened by the reach.
    let b: Vec<[f64; 4]> = chosen.iter().map(|(_, c)| [c.cloud.bounds[0], c.cloud.bounds[1], c.cloud.bounds[3], c.cloud.bounds[4]]).collect();
    let meet = [b[0][0].max(b[1][0]) - reach, b[0][1].max(b[1][1]) - reach, b[0][2].min(b[1][2]) + reach, b[0][3].min(b[1][3]) + reach];
    let classes = input.classes.clone();
    let mut sums: [BTreeMap<(i64, i64, i64), (f64, f64, f64, f64)>; 2] = [BTreeMap::new(), BTreeMap::new()];
    let mut warnings = Vec::new();
    let mut read = vec![0u64; 2];
    Scan { doc, drawing: input.drawing.as_deref(), chosen: &chosen, boxes: &[meet], what: "Bu kayıt", path: "uids" }.run(
        &mut warnings,
        &mut read,
        &mut |s| {
            if let Some(cs) = &classes
                && !cs.contains(&s.layout.class(s.record))
            {
                return;
            }
            let [x, y, z] = s.xyz;
            if x < meet[0] || x > meet[2] || y < meet[1] || y > meet[3] {
                return;
            }
            let e = sums[s.cloud].entry(key(s.xyz, spacing)).or_insert((0.0, 0.0, 0.0, 0.0));
            e.0 += x;
            e.1 += y;
            e.2 += z;
            e.3 += 1.0;
        },
    )?;
    for (k, s) in sums.iter().enumerate() {
        if s.len() > REGISTER_MOST_POINTS as usize {
            return Err(fail(
                codes::TOO_MANY_CLOUD_POINTS,
                format!("{}. bulut {} voksel noktası veriyor; bir bulut en çok {REGISTER_MOST_POINTS} voksel noktası verir. Vokseli büyütün (spacing) ya da sınıf seçin.", k + 1, s.len()),
                "spacing",
            ));
        }
    }
    let fixed = voxels(&sums[0]);
    let moving = voxels(&sums[1]);
    let n_fixed = fixed.len().max(1) as f64;
    let c = [
        fixed.iter().map(|p| p[0]).sum::<f64>() / n_fixed,
        fixed.iter().map(|p| p[1]).sum::<f64>() / n_fixed,
        fixed.iter().map(|p| p[2]).sum::<f64>() / n_fixed,
    ];
    let grid = Grid::new(&fixed, 2.0 * spacing);
    let k = (neighbours as usize).min(fixed.len());
    let normals: Vec<Option<P>> = fixed.iter().map(|p| normal_of(&fixed, &grid, *p, k)).collect();
    // docs/adr/2700: the fixed points on their cloud's edge, judged from the fixed points within the reach (as far as a
    // pair can be): a moving point beyond the edge pairs with it though the fixed scan saw nothing there.
    let edges: Vec<bool> = if reject.as_ref().is_some_and(|r| r.edges) {
        fixed
            .iter()
            .zip(&normals)
            .map(|(p, n)| n.is_some_and(|n| on_edge(&fixed, *p, n, &grid.within(*p, reach))))
            .collect()
    } else {
        Vec::new()
    };
    // The moving points' own normals, from their own neighbours, for the angle.
    let own: Vec<Option<P>> = match reject.as_ref().and_then(|r| r.angle) {
        Some(_) => {
            let moving_grid = Grid::new(&moving, 2.0 * spacing);
            let km = (neighbours as usize).min(moving.len());
            moving.iter().map(|p| normal_of(&moving, &moving_grid, *p, km)).collect()
        }
        None => Vec::new(),
    };
    let rule = Rule {
        edge: (!edges.is_empty()).then_some(edges.as_slice()),
        moving: reject.as_ref().and_then(|r| r.angle).map(|a| (own.as_slice(), a.to_radians().cos())),
    };
    let radius_rms = if moving.is_empty() {
        1.0
    } else {
        let r = (moving.iter().map(|m| dot(sub(*m, c), sub(*m, c))).sum::<f64>() / moving.len() as f64).sqrt();
        if r > 0.0 { r } else { 1.0 }
    };
    let mut now = Rigid::identity();
    let mut done = 0u32;
    let mut converged = false;
    let mut rms_before = 0.0;
    for it in 0..iterations {
        let (ps, _, _) = pairs(&moving, &now, c, &grid, &normals, &fixed, reach, rule);
        if ps.len() < 6 {
            break;
        }
        if it == 0 {
            rms_before = (ps.iter().map(|p| p.r * p.r).sum::<f64>() / ps.len() as f64).sqrt();
        }
        let weighed = robust.as_ref().map(|r| weights(&ps, r).0);
        let (m, v) = normal_matrix(&ps, &normals, c, weighed.as_deref());
        let x = step(m, v);
        now = now.then([x[0], x[1], x[2]], [x[3], x[4], x[5]]);
        done += 1;
        let moved = x[3].abs().max(x[4].abs()).max(x[5].abs()).max(x[0].abs().max(x[1].abs()).max(x[2].abs()) * radius_rms);
        if moved < tolerance {
            converged = true;
            break;
        }
    }
    let (last, at_edges, by_angle) = pairs(&moving, &now, c, &grid, &normals, &fixed, reach, rule);
    let np = last.len() as f64;
    let (rms_after, mean_after) = if last.is_empty() {
        (0.0, 0.0)
    } else {
        ((last.iter().map(|p| p.r * p.r).sum::<f64>() / np).sqrt(), last.iter().map(|p| p.r).sum::<f64>() / np)
    };
    // What the robust weights did at the last pairs.
    let robust_report = robust.as_ref().map(|r| {
        let (w, s) = weights(&last, r);
        let (kind, k, ratio) = match r {
            Robust::Huber(k) => ("huber", Some(*k), None),
            Robust::Trimmed(ratio) => ("trimmed", None, Some(*ratio)),
        };
        RegisterRobustReport { kind: kind.to_owned(), k, ratio, scale: s, lighter: w.iter().filter(|x| **x < 1.0).count() as u32 }
    });
    // How firmly the pairs hold the six parameters: the angles as metres at the RMS radius (every pair weighing one).
    let (m, _) = normal_matrix(&last, &normals, c, None);
    let mut scaled = m;
    for i in 0..6 {
        for j in 0..6 {
            let f = (if i < 3 { 1.0 / radius_rms } else { 1.0 }) * (if j < 3 { 1.0 / radius_rms } else { 1.0 });
            scaled[i][j] = m[i][j] * f;
        }
    }
    let (values, _) = eigen(scaled);
    let strength = if values[5] > 0.0 { (values[0].max(0.0) / values[5]).sqrt() } else { 0.0 };
    let (a, bb, d) = (m[3][3], m[3][4], m[4][4]);
    let plan_min = (a + d) / 2.0 - (((a - d) / 2.0).powi(2) + bb * bb).sqrt();
    let horizontal = if m[5][5] > 0.0 { (plan_min.max(0.0) / m[5][5]).sqrt() } else { 0.0 };
    // The coverage: plan cells of the moving points where they are now.
    let mut cells: BTreeMap<(i64, i64), (f64, u32)> = BTreeMap::new();
    for p in &last {
        let e = cells.entry(((p.at[0] / cell).floor() as i64, (p.at[1] / cell).floor() as i64)).or_insert((0.0, 0));
        e.0 += p.r * p.r;
        e.1 += 1;
    }
    let held: Vec<((i64, i64), f64, u32)> =
        cells.iter().filter(|(_, v)| v.1 >= least).map(|(k, v)| (*k, (v.0 / f64::from(v.1)).sqrt(), v.1)).collect();
    let covered = held.len() as f64 * cell * cell;
    let reliable = held.iter().filter(|h| h.1 <= rules.interswath).count() as f64 * cell * cell;
    let mut worst = held.clone();
    worst.sort_by(|x, y| y.1.total_cmp(&x.1).then(x.0.cmp(&y.0)));
    let worst: Vec<RegisterCell> = worst
        .into_iter()
        .take(MOST_WORST)
        .map(|((i, j), rms, n)| RegisterCell { x: (i as f64 + 0.5) * cell, y: (j as f64 + 0.5) * cell, n, rms })
        .collect();
    // The control points: each cloud's plane around one.
    let plane = |points: &mut dyn Iterator<Item = P>, cx: f64, cy: f64| -> Option<f64> {
        let mut s = Sums::default();
        for p in points {
            let (u, v) = (p[0] - cx, p[1] - cy);
            if u.abs() <= radius && v.abs() <= radius && u.hypot(v) <= radius {
                s.add(u, v, p[2]);
            }
        }
        s.height(least.max(3), radius)
    };
    let control: Vec<RegisterControl> = controls
        .iter()
        .map(|(name, x, y, z)| RegisterControl {
            name: name.clone(),
            z: *z,
            dz_fixed: plane(&mut fixed.iter().copied(), *x, *y).map(|h| h - z),
            dz_before: plane(&mut moving.iter().copied(), *x, *y).map(|h| h - z),
            dz_after: plane(&mut moving.iter().map(|m| now.apply(*m, c)), *x, *y).map(|h| h - z),
        })
        .collect();
    let s = f64::from(scale);
    let limit_z = rules.z_factor * (rules.xy[0] * s * s + rules.xy[1] * s + rules.xy[2]) / 100.0;
    let stats = |pick: fn(&RegisterControl) -> Option<f64>| -> Option<OverlapControlStats> {
        let v: Vec<f64> = control.iter().filter_map(pick).collect();
        (!controls.is_empty()).then(|| {
            let k = v.len() as f64;
            let koh = if v.is_empty() { 0.0 } else { (v.iter().map(|d| d * d).sum::<f64>() / k).sqrt() };
            OverlapControlStats {
                n: v.len() as u32,
                koh_z: koh,
                mean: if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / k },
                passed: (!v.is_empty()).then_some(koh <= limit_z),
            }
        })
    };
    let control_fixed = stats(|c| c.dz_fixed);
    let control_before = stats(|c| c.dz_before);
    let control_after = stats(|c| c.dz_after);
    let r = now.r;
    let rotation = [r[2][1].atan2(r[2][2]), (-r[2][0]).clamp(-1.0, 1.0).asin(), r[1][0].atan2(r[0][0])];
    // The matrix in the project's coordinates: t' = c + t − R c.
    let rc = [dot(r[0], c), dot(r[1], c), dot(r[2], c)];
    let t0 = [c[0] + now.t[0] - rc[0], c[1] + now.t[1] - rc[1], c[2] + now.t[2] - rc[2]];
    let matrix = [
        r[0][0], r[0][1], r[0][2], t0[0], r[1][0], r[1][1], r[1][2], t0[1], r[2][0], r[2][1], r[2][2], t0[2], 0.0, 0.0, 0.0, 1.0,
    ];
    let mut notes = vec![
        "Dönüşüm hareketli buluta uygulanmadı; bulut yerinde durur. Tarif: p ↦ R(p − merkez) + merkez + öteleme, R = Rz(κ)·Ry(φ)·Rx(ω).".to_owned(),
        "RMS nokta-düzlem uzaklığıdır ve yalnız erişim içinde eşleşen noktalardan hesaplanır; eşleşmeyen kısım örtüşme payında görünür.".to_owned(),
        format!("Güvenilir kapsamın sınırı {} paketinin şeritler arası örnek değeridir; kurum kendi değerini verir.", rules.id),
    ];
    if last.is_empty() {
        notes.push("Eşleşen nokta yok: bulutlar erişim içinde örtüşmüyor; erişimi büyütün ya da bulutları yaklaştırın.".to_owned());
    } else if horizontal < 0.1 {
        notes.push("Geometri yatay ötelemeyi zayıf tutuyor (düz zemin): kayıt düşeyde anlamlı, yatayda değil; yatay için eğimli yüzeyler, yapılar ya da kontrol noktaları gerekir.".to_owned());
    }
    if !converged && done == iterations {
        notes.push("Yineleme sınırına varıldı; son adım toleransın üstünde kaldı.".to_owned());
    }
    if controls.is_empty() {
        notes.push("Kontrol noktası yok: kaydın mutlak doğruluğu bilinmiyor; bulut ölçüye kesin hazır sayılmaz.".to_owned());
    }
    if reject.is_some() {
        notes.push("Ayıklanan eşleşmeler (kenarda ya da normal açısıyla) hiçbir adıma girmedi; RMS ve örtüşme payı kalan eşleşmelerden.".to_owned());
    }
    Ok((
        PointCloudsRegistered {
            rules: rules.id.clone(),
            source: rules.source.clone(),
            read: [read[0], read[1]],
            used: [fixed.len() as u32, moving.len() as u32],
            iterations: done,
            converged,
            centre: c,
            rotation,
            translation: now.t,
            matrix,
            pairs: last.len() as u32,
            overlap: if moving.is_empty() { 0.0 } else { np / moving.len() as f64 },
            rms_before,
            rms_after,
            mean_after,
            strength,
            horizontal,
            cell,
            covered,
            reliable,
            limit: rules.interswath,
            worst,
            control,
            control_fixed,
            control_before,
            control_after,
            limit_z,
            robust: robust_report,
            rejected: reject.as_ref().map(|_| RegisterRejectReport { edges: at_edges, angle: by_angle }),
            notes,
        },
        warnings,
    ))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &PointCloudsRegister) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &PointCloudsRegister) -> CommandResult<PointCloudsRegistered> {
    match run(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: PointCloudsRegister) -> CommandResult<PointCloudsRegistered> {
    plan(cx, &input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_jacobi_eigenvectors_diagonalise_a_symmetric_matrix() {
        let m = [[4.0, 1.0, 0.5], [1.0, 3.0, 0.25], [0.5, 0.25, 1.0]];
        let (values, vectors) = eigen(m);
        assert!(values[0] <= values[1] && values[1] <= values[2]);
        for k in 0..3 {
            let v = [vectors[0][k], vectors[1][k], vectors[2][k]];
            let mv = [dot(m[0], v), dot(m[1], v), dot(m[2], v)];
            for i in 0..3 {
                assert!((mv[i] - values[k] * v[i]).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn the_grid_finds_what_a_full_search_finds() {
        let points: Vec<P> = (0..400).map(|i| {
            let f = i as f64;
            [(f * 0.37).sin() * 10.0, (f * 0.11).cos() * 10.0, (f * 0.07).sin()]
        }).collect();
        let grid = Grid::new(&points, 1.3);
        for q in [[0.2, 0.3, 0.0], [5.0, -3.0, 0.4], [-9.0, 9.5, -0.8]] {
            let mut all: Vec<(f64, u32)> = points.iter().enumerate().map(|(i, p)| (dot(sub(*p, q), sub(*p, q)), i as u32)).collect();
            all.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            assert_eq!(grid.nearest_k(q, 8), all[..8].to_vec());
            assert_eq!(grid.nearest(q, 100.0), Some(all[0]));
            assert_eq!(grid.nearest(q, all[0].0.sqrt() * 0.5), None);
        }
    }
}
