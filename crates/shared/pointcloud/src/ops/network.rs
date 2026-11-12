//! A network of scans adjusted from their pairwise registrations (plan/map-cad.md MAP-13, docs/adr/4720), the
//! arithmetic of `cad.pointclouds.network`. As a GNSS network adjusts its baselines: each scan not held has an unknown
//! small correction x = [ω, φ, κ, tx, ty, tz] about the network's centre (a held scan's is zero); each link (a, b) is an
//! observation m of b's correction relative to a's with its weight N, the registration's last normal matrix moved to
//! the centre. The residual r = x_b − x_a − m; Σ rᵀ·N·r is least where Σ over the links of the blocks (a, b) of
//! [N, −N; −N, N] times x equals Σ [−N·m; N·m] — solved by Cholesky in the scans' order. A pivot at or under 10⁻¹² of
//! the matrix's largest diagonal leaves a direction free (flat ground: a horizontal shift): no answer.
//!
//! Loops: the held scans are one node; a breadth-first tree from it over the links in their order gives each scan the
//! sum of the observations along its tree path; a link off the tree closes a loop, whose misclosure is a's sum + m − b's
//! sum (zero for consistent registrations).
//!
//! First order: rotations add as vectors, which holds for the small corrections between registered scans.

/// Six numbers: ω, φ, κ (radians), then tx, ty, tz (metres).
pub type V6 = [f64; 6];
/// A 6 × 6 matrix, row by row.
pub type M6 = [[f64; 6]; 6];

/// A link's observation: b's correction relative to a's (about the network's centre) and its weight.
#[derive(Clone, Debug, PartialEq)]
pub struct Observation {
    pub a: usize,
    pub b: usize,
    pub m: V6,
    pub n: M6,
}

/// Why a network has no answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkFault {
    /// No link path joins scan k to a held scan.
    Unjoined(usize),
    /// The links leave a direction of some scan free.
    Undetermined,
}

/// A loop of links: the scans along it (from a link's first scan through the tree to its second) and its misclosure.
#[derive(Clone, Debug, PartialEq)]
pub struct Loop {
    pub path: Vec<usize>,
    pub link: usize,
    pub w: V6,
}

/// The network adjusted: every scan's correction (held: zero), every link's residual, the loops, and every free scan's
/// horizontal strength (held: none): √(σz² / σh²) of its translation's covariance block in the normal matrix's inverse,
/// σh² the larger eigenvalue of its plan block — Tarama kaydı's `horizontal`, for the network (small: flat ground).
#[derive(Clone, Debug, PartialEq)]
pub struct Adjusted {
    pub x: Vec<V6>,
    pub residuals: Vec<V6>,
    pub loops: Vec<Loop>,
    pub horizontal: Vec<Option<f64>>,
}

fn add(a: V6, b: V6) -> V6 {
    std::array::from_fn(|i| a[i] + b[i])
}

fn minus(a: V6, b: V6) -> V6 {
    std::array::from_fn(|i| a[i] - b[i])
}

fn times(n: &M6, v: V6) -> V6 {
    std::array::from_fn(|i| (0..6).fold(0.0, |s, j| s + n[i][j] * v[j]))
}

/// rᵀ·N·r.
pub fn quadratic(n: &M6, r: V6) -> f64 {
    let nr = times(n, r);
    (0..6).fold(0.0, |s, i| s + r[i] * nr[i])
}

/// A registration's observation and weight moved from its own centre `c` to the network's `to` (docs/adr/4720): the
/// rotation `w` (ω, φ, κ) and matrix `r` (its 3 × 3) and translation `t` about `c` give the translation about `to`,
/// t + (R − I)(to − c); the weight N about `c` becomes Jᵀ·N·J, J = [I, 0; [d]×, I], d = to − c (a small motion's
/// translation about `c` is its translation about `to` plus d × ω).
pub fn moved(w: [f64; 3], r: [[f64; 3]; 3], t: [f64; 3], n: &M6, c: [f64; 3], to: [f64; 3]) -> (V6, M6) {
    let d = [to[0] - c[0], to[1] - c[1], to[2] - c[2]];
    let rd: [f64; 3] = std::array::from_fn(|i| r[i][0] * d[0] + r[i][1] * d[1] + r[i][2] * d[2]);
    let m = [w[0], w[1], w[2], t[0] + rd[0] - d[0], t[1] + rd[1] - d[1], t[2] + rd[2] - d[2]];
    // J: the identity, with [d]× in its lower left block ([d]× ω = d × ω).
    let mut j = [[0.0; 6]; 6];
    for (i, row) in j.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    let dx = [[0.0, -d[2], d[1]], [d[2], 0.0, -d[0]], [-d[1], d[0], 0.0]];
    for i in 0..3 {
        for k in 0..3 {
            j[3 + i][k] = dx[i][k];
        }
    }
    // Jᵀ·N·J, sums in index order.
    let mut nj = [[0.0; 6]; 6];
    for i in 0..6 {
        for k in 0..6 {
            nj[i][k] = (0..6).fold(0.0, |s, l| s + n[i][l] * j[l][k]);
        }
    }
    let mut out = [[0.0; 6]; 6];
    for i in 0..6 {
        for k in 0..6 {
            out[i][k] = (0..6).fold(0.0, |s, l| s + j[l][i] * nj[l][k]);
        }
    }
    (m, out)
}

/// The network adjusted: `scans` scans, `fixed` the held ones, the links' observations in their order.
pub fn adjust(scans: usize, fixed: &[bool], obs: &[Observation]) -> Result<Adjusted, NetworkFault> {
    // The breadth-first tree from the held scans (one node), links in their order.
    let held = |k: usize| fixed.get(k).copied().unwrap_or(false);
    let mut reached: Vec<bool> = (0..scans).map(held).collect();
    let mut sum: Vec<V6> = vec![[0.0; 6]; scans];
    let mut parent: Vec<Option<(usize, usize)>> = vec![None; scans];
    let mut on_tree = vec![false; obs.len()];
    let mut queue: std::collections::VecDeque<Option<usize>> = std::collections::VecDeque::new();
    queue.push_back(None);
    while let Some(node) = queue.pop_front() {
        let at = |k: usize| match node {
            None => held(k),
            Some(n) => k == n,
        };
        for (l, o) in obs.iter().enumerate() {
            let (from, to, step) = if at(o.a) && !reached[o.b] {
                (o.a, o.b, o.m)
            } else if at(o.b) && !reached[o.a] {
                (o.b, o.a, o.m.map(|v| -v))
            } else {
                continue;
            };
            reached[to] = true;
            sum[to] = add(sum[from], step);
            parent[to] = Some((from, l));
            on_tree[l] = true;
            queue.push_back(Some(to));
        }
    }
    if let Some(k) = (0..scans).find(|&k| !reached[k]) {
        return Err(NetworkFault::Unjoined(k));
    }
    // The loops: each link off the tree.
    let path_up = |mut k: usize| {
        let mut p = vec![k];
        while let Some((from, _)) = parent[k] {
            p.push(from);
            k = from;
        }
        p
    };
    let mut loops = Vec::new();
    for (l, o) in obs.iter().enumerate() {
        if on_tree[l] {
            continue;
        }
        let w = minus(add(sum[o.a], o.m), sum[o.b]);
        let (up_a, up_b) = (path_up(o.a), path_up(o.b));
        // Through their deepest common scan, or through the held scans (their roots).
        let mut path: Vec<usize> = Vec::new();
        match up_a.iter().position(|k| up_b.contains(k)) {
            Some(i) => {
                let j = up_b.iter().position(|k| *k == up_a[i]).unwrap_or(0);
                path.extend(&up_a[..=i]);
                path.extend(up_b[..j].iter().rev());
            }
            None => {
                path.extend(&up_a);
                path.extend(up_b.iter().rev());
            }
        }
        loops.push(Loop { path, link: l, w });
    }
    // The normal equations of the free scans, in their order.
    let free: Vec<usize> = (0..scans).filter(|&k| !held(k)).collect();
    let index = |k: usize| free.iter().position(|&f| f == k);
    let n = 6 * free.len();
    let mut a = vec![vec![0.0; n]; n];
    let mut rhs = vec![0.0; n];
    for o in obs {
        let nm = times(&o.n, o.m);
        let (ia, ib) = (index(o.a), index(o.b));
        for i in 0..6 {
            if let Some(b) = ib {
                rhs[6 * b + i] += nm[i];
            }
            if let Some(aa) = ia {
                rhs[6 * aa + i] -= nm[i];
            }
            for j in 0..6 {
                let v = o.n[i][j];
                if let Some(b) = ib {
                    a[6 * b + i][6 * b + j] += v;
                }
                if let Some(aa) = ia {
                    a[6 * aa + i][6 * aa + j] += v;
                }
                if let (Some(aa), Some(b)) = (ia, ib) {
                    a[6 * aa + i][6 * b + j] -= v;
                    a[6 * b + i][6 * aa + j] -= v;
                }
            }
        }
    }
    let l = factor(&a).ok_or(NetworkFault::Undetermined)?;
    let solved = solve(&l, &rhs);
    let mut x = vec![[0.0; 6]; scans];
    let mut horizontal = vec![None; scans];
    for (k, f) in free.iter().enumerate() {
        x[*f] = std::array::from_fn(|i| solved[6 * k + i]);
        // The translation's covariance block: three columns of the inverse.
        let cols: Vec<Vec<f64>> = (0..3)
            .map(|c| {
                let mut e = vec![0.0; n];
                e[6 * k + 3 + c] = 1.0;
                solve(&l, &e)
            })
            .collect();
        let t = |i: usize, j: usize| cols[j][6 * k + 3 + i];
        let (p, q, d) = (t(0, 0), t(0, 1), t(1, 1));
        let plan_max = (p + d) / 2.0 + (((p - d) / 2.0) * ((p - d) / 2.0) + q * q).sqrt();
        horizontal[*f] = Some(if plan_max > 0.0 { (t(2, 2).max(0.0) / plan_max).sqrt() } else { 0.0 });
    }
    let residuals = obs.iter().map(|o| minus(minus(x[o.b], x[o.a]), o.m)).collect();
    Ok(Adjusted { x, residuals, loops, horizontal })
}

/// A symmetric positive definite matrix's Cholesky factor L (rows in order); none when a pivot is at or under 10⁻¹² of
/// the largest diagonal.
fn factor(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let big = (0..n).fold(0.0_f64, |m, i| m.max(a[i][i].abs()));
    if n == 0 {
        return Some(Vec::new());
    }
    if !(big > 0.0) {
        return None;
    }
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let s = (0..j).fold(a[i][j], |s, k| s - l[i][k] * l[j][k]);
            if i == j {
                if !(s > 1e-12 * big) {
                    return None;
                }
                l[i][i] = s.sqrt();
            } else {
                l[i][j] = s / l[j][j];
            }
        }
    }
    Some(l)
}

/// L·Lᵀ·x = b, forward then back.
fn solve(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = b.len();
    let mut y = vec![0.0; n];
    for i in 0..n {
        y[i] = (0..i).fold(b[i], |s, k| s - l[i][k] * y[k]) / l[i][i];
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        x[i] = (i + 1..n).fold(y[i], |s, k| s - l[k][i] * x[k]) / l[i][i];
    }
    x
}

/// Rz(κ)·Ry(φ)·Rx(ω).
pub fn rotation(w: [f64; 3]) -> [[f64; 3]; 3] {
    let (sa, ca) = w[0].sin_cos();
    let (sb, cb) = w[1].sin_cos();
    let (sg, cg) = w[2].sin_cos();
    [
        [cg * cb, -sg * ca + cg * sb * sa, sg * sa + cg * sb * ca],
        [sg * cb, cg * ca + sg * sb * sa, -cg * sa + sg * sb * ca],
        [-sb, cb * sa, cb * ca],
    ]
}

/// The matrix of a correction about `centre`, row by row: p ↦ R(p − centre) + centre + t.
pub fn matrix(x: V6, centre: [f64; 3]) -> [f64; 16] {
    let r = rotation([x[0], x[1], x[2]]);
    let rc: [f64; 3] = std::array::from_fn(|i| r[i][0] * centre[0] + r[i][1] * centre[1] + r[i][2] * centre[2]);
    let t: [f64; 3] = std::array::from_fn(|i| centre[i] + x[3 + i] - rc[i]);
    [r[0][0], r[0][1], r[0][2], t[0], r[1][0], r[1][1], r[1][2], t[1], r[2][0], r[2][1], r[2][2], t[2], 0.0, 0.0, 0.0, 1.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eye(k: f64) -> M6 {
        let mut n = [[0.0; 6]; 6];
        for (i, row) in n.iter_mut().enumerate() {
            row[i] = k;
        }
        n
    }

    #[test]
    fn a_consistent_triangle_closes_and_keeps_its_observations() {
        // Held 0; 1 and 2 observed from it and from each other consistently.
        let m1 = [0.001, -0.002, 0.003, 0.30, -0.20, 0.06];
        let m2 = [-0.002, 0.001, 0.0, -0.10, 0.25, 0.02];
        let obs = vec![
            Observation { a: 0, b: 1, m: m1, n: eye(100.0) },
            Observation { a: 0, b: 2, m: m2, n: eye(100.0) },
            Observation { a: 1, b: 2, m: minus(m2, m1), n: eye(100.0) },
        ];
        let got = adjust(3, &[true, false, false], &obs).expect("an answer");
        for i in 0..6 {
            assert!((got.x[1][i] - m1[i]).abs() < 1e-12 && (got.x[2][i] - m2[i]).abs() < 1e-12);
        }
        assert_eq!(got.loops.len(), 1);
        assert!(got.loops[0].w.iter().all(|v| v.abs() < 1e-12), "{:?}", got.loops[0]);
        assert_eq!(got.loops[0].path, vec![1, 0, 2]);
    }

    #[test]
    fn a_misclosure_is_shared_by_weight() {
        // The loop fails to close by 0.03 m in x: equal weights share it in thirds.
        let obs = vec![
            Observation { a: 0, b: 1, m: [0.0, 0.0, 0.0, 1.0, 0.0, 0.0], n: eye(1.0) },
            Observation { a: 1, b: 2, m: [0.0, 0.0, 0.0, 1.0, 0.0, 0.0], n: eye(1.0) },
            Observation { a: 0, b: 2, m: [0.0, 0.0, 0.0, 2.03, 0.0, 0.0], n: eye(1.0) },
        ];
        let got = adjust(3, &[true, false, false], &obs).expect("an answer");
        assert!((got.x[1][3] - 1.01).abs() < 1e-12 && (got.x[2][3] - 2.02).abs() < 1e-12, "{:?}", got.x);
        assert!((got.loops[0].w[3] + 0.03).abs() < 1e-12, "{:?}", got.loops[0].w);
        assert!(got.residuals.iter().all(|r| (r[3].abs() - 0.01).abs() < 1e-12), "{:?}", got.residuals);
    }

    #[test]
    fn faults() {
        let o = |a, b| Observation { a, b, m: [0.0; 6], n: eye(1.0) };
        assert_eq!(adjust(3, &[true, false, false], &[o(0, 1)]).err(), Some(NetworkFault::Unjoined(2)));
        let mut flat = eye(1.0);
        flat[3][3] = 0.0;
        let weak = Observation { a: 0, b: 1, m: [0.0; 6], n: flat };
        assert_eq!(adjust(2, &[true, false], &[weak]).err(), Some(NetworkFault::Undetermined));
    }

    #[test]
    fn moving_the_centre_keeps_a_motion() {
        // A rotation about c with a translation is, about C, the same motion: a point moves alike.
        let w = [0.01, -0.02, 0.03];
        let r = rotation(w);
        let (c, to, t) = ([10.0, 20.0, 5.0], [0.0, 0.0, 0.0], [0.3, -0.2, 0.1]);
        let (m, _) = moved(w, r, t, &eye(1.0), c, to);
        let p = [13.0, 17.0, 6.0];
        let about = |cc: [f64; 3], tt: [f64; 3]| -> [f64; 3] {
            let d = [p[0] - cc[0], p[1] - cc[1], p[2] - cc[2]];
            std::array::from_fn(|i| r[i][0] * d[0] + r[i][1] * d[1] + r[i][2] * d[2] + cc[i] + tt[i])
        };
        let (one, two) = (about(c, t), about(to, [m[3], m[4], m[5]]));
        assert!((0..3).all(|i| (one[i] - two[i]).abs() < 1e-12), "{one:?} {two:?}");
    }
}
