//! A proposed grade line (docs/adr/0926; ROAD-PROFILE-14): new elevations for a profile's PVIs (their stations and
//! parabolic curves kept) that follow the terrain profile as closely as the user's rules allow. The design is linear in
//! the PVI elevations (a grade is the difference of two over their distance, a parabola's elevation is linear in its
//! grades), the terrain profile piecewise linear between its points; so ∫(design − ground)² over the stretches on the
//! surface is a quadratic in the elevations, integrated exactly (three-point Gauss–Legendre on each stretch between
//! breakpoints, the integrand of degree four at most). It is made least subject to: held PVIs keep their elevations,
//! every grade's magnitude is at most the user's, and (balanced) ∫(design − ground) = 0, the area cut equal to the area
//! filled. A convex quadratic programme, solved by an active set (each step the KKT system of the equalities and the
//! inequalities taken as equalities; a violated inequality joins, one whose multiplier is negative leaves) until the
//! KKT conditions hold, which makes the answer the one optimum.

use super::terrain::ProfilePoint;
use super::vertical::{Pvi, VCurve};

/// The user's rules.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rules {
    /// Per PVI: whether it keeps its elevation.
    pub held: Vec<bool>,
    /// The most a grade may rise or fall (a ratio); none: no limit.
    pub most_grade: Option<f64>,
    /// The area cut equal to the area filled.
    pub balanced: bool,
}

/// A proposal.
#[derive(Clone, Debug, PartialEq)]
pub struct Proposal {
    /// The PVIs' new elevations, in order.
    pub elevations: Vec<f64>,
    /// The grades between them.
    pub grades: Vec<f64>,
    /// Per grade: +1 at the most rise, −1 at the most fall, 0 free.
    pub limited: Vec<i8>,
    /// ∫(design − ground)² over the stretches on the surface (m²·m), and their length (m).
    pub squares: f64,
    pub covered: f64,
    /// The areas where the design is below the ground (cut) and above it (fill), m².
    pub cut: f64,
    pub fill: f64,
}

/// Why no proposal is made.
#[derive(Clone, Debug, PartialEq)]
pub enum ProposeError {
    /// A circular curve: its elevation is not linear in the PVIs' elevations.
    Circle { pvi: String },
    /// No stretch of the profile is on the surface.
    NoGround,
    /// The ground does not fix these PVIs' elevations (their reach is off the surface) and nothing holds them.
    Unfixed,
    /// The rules contradict each other (held PVIs too steep for the most grade, a balance they cannot reach).
    Infeasible,
}

/// Three-point Gauss–Legendre on [−1, 1]: nodes ±√(3/5) and 0, weights 5/9 and 8/9.
fn gauss() -> [(f64, f64); 3] {
    let x = (0.6_f64).sqrt();
    [(-x, 5.0 / 9.0), (0.0, 8.0 / 9.0), (x, 5.0 / 9.0)]
}

/// A PVI's curve as the lengths before and after it (a symmetric parabola half and half); none: (0, 0).
fn reach(p: &Pvi) -> Result<(f64, f64), ProposeError> {
    match p.curve {
        VCurve::None => Ok((0.0, 0.0)),
        VCurve::Parabola { length } => Ok((0.5 * length, 0.5 * length)),
        VCurve::Asymmetric { length_in, length_out } => Ok((length_in, length_out)),
        VCurve::Circle { .. } => Err(ProposeError::Circle { pvi: p.id.clone() }),
    }
}

/// The design's elevation at `s` for the elevations `z` (stations and curves of `pvis`, curves' reaches `r`).
fn design(stations: &[f64], r: &[(f64, f64)], z: &[f64], s: f64) -> f64 {
    let n = stations.len();
    let grade = |k: usize| (z[k + 1] - z[k]) / (stations[k + 1] - stations[k]);
    for k in 1..n - 1 {
        let (l1, l2) = r[k];
        if l1 + l2 > 0.0 && s >= stations[k] - l1 && s <= stations[k] + l2 {
            let (g1, g2) = (grade(k - 1), grade(k));
            let r1 = (g2 - g1) * l2 / (l1 * (l1 + l2));
            if s <= stations[k] {
                let t = s - (stations[k] - l1);
                return z[k] - g1 * l1 + g1 * t + 0.5 * r1 * t * t;
            }
            let r2 = r1 * l1 * l1 / (l2 * l2);
            let gm = (l1 * g1 + l2 * g2) / (l1 + l2);
            let t = s - stations[k];
            return z[k] + 0.5 * r1 * l1 * l1 + gm * t + 0.5 * r2 * t * t;
        }
    }
    let k = stations.partition_point(|x| *x <= s).clamp(1, n - 1) - 1;
    z[k] + grade(k) * (s - stations[k])
}

/// The stretches on the surface between the breakpoints: (a, b, ground at a, ground at b).
fn stretches(stations: &[f64], r: &[(f64, f64)], ground: &[ProfilePoint]) -> Vec<(f64, f64, f64, f64)> {
    let (start, end) = (stations[0], stations[stations.len() - 1]);
    let mut breaks: Vec<f64> = stations.to_vec();
    for (k, s) in stations.iter().enumerate() {
        if r[k].0 + r[k].1 > 0.0 {
            breaks.push(s - r[k].0);
            breaks.push(s + r[k].1);
        }
    }
    let mut out = Vec::new();
    for w in ground.windows(2) {
        let (p, q) = (w[0], w[1]);
        let (Some(za), Some(zb)) = (p.z, q.z) else { continue };
        if q.gap || q.station <= p.station {
            continue;
        }
        let (a, b) = (p.station.max(start), q.station.min(end));
        if b <= a {
            continue;
        }
        let line = |s: f64| za + (zb - za) * (s - p.station) / (q.station - p.station);
        let mut cuts: Vec<f64> = breaks.iter().copied().filter(|x| *x > a && *x < b).collect();
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        let mut from = a;
        for x in cuts.into_iter().chain(std::iter::once(b)) {
            if x > from {
                out.push((from, x, line(from), line(x)));
            }
            from = x;
        }
    }
    out
}

/// Solves `m · x = rhs` by Gaussian elimination with partial pivoting; none where `m` is singular.
fn solve(mut m: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Option<Vec<f64>> {
    let n = rhs.len();
    let scale = m.iter().flatten().fold(0.0_f64, |a, v| a.max(v.abs())).max(1e-300);
    for c in 0..n {
        let p = (c..n).max_by(|a, b| m[*a][c].abs().total_cmp(&m[*b][c].abs()))?;
        if m[p][c].abs() <= 1e-12 * scale {
            return None;
        }
        m.swap(c, p);
        rhs.swap(c, p);
        for i in c + 1..n {
            let f = m[i][c] / m[c][c];
            if f != 0.0 {
                let (top, rest) = m.split_at_mut(i);
                for (v, p) in rest[0][c..n].iter_mut().zip(&top[c][c..n]) {
                    *v -= f * p;
                }
                rhs[i] -= f * rhs[c];
            }
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut v = rhs[i];
        for j in i + 1..n {
            v -= m[i][j] * x[j];
        }
        x[i] = v / m[i][i];
    }
    Some(x)
}

/// The proposal for `pvis` against `ground` under `rules`.
pub fn propose(pvis: &[Pvi], ground: &[ProfilePoint], rules: &Rules) -> Result<Proposal, ProposeError> {
    let n = pvis.len();
    let r = pvis.iter().map(reach).collect::<Result<Vec<_>, _>>()?;
    let stations: Vec<f64> = pvis.iter().map(|p| p.station).collect();
    let pieces = stretches(&stations, &r, ground);
    if pieces.is_empty() {
        return Err(ProposeError::NoGround);
    }
    // The design's coefficients at a station: the elevation for each unit elevation of a nearby PVI.
    let unit = |j: usize, s: f64| {
        let mut z = vec![0.0; n];
        z[j] = 1.0;
        design(&stations, &r, &z, s)
    };
    let near = |s: f64| {
        let k = stations.partition_point(|x| *x <= s);
        k.saturating_sub(3)..(k + 3).min(n)
    };
    let mut h = vec![vec![0.0; n]; n];
    let mut b = vec![0.0; n];
    let mut a = vec![0.0; n];
    let mut ground_area = 0.0;
    let mut covered = 0.0;
    for &(s0, s1, g0, g1) in &pieces {
        covered += s1 - s0;
        let half = 0.5 * (s1 - s0);
        for (x, w) in gauss() {
            let s = s0 + half * (1.0 + x);
            let g = g0 + (g1 - g0) * (s - s0) / (s1 - s0);
            let c: Vec<(usize, f64)> = near(s).map(|j| (j, unit(j, s))).filter(|(_, v)| *v != 0.0).collect();
            for &(i, ci) in &c {
                b[i] += w * half * ci * g;
                a[i] += w * half * ci;
                for &(j, cj) in &c {
                    h[i][j] += w * half * ci * cj;
                }
            }
            ground_area += w * half * g;
        }
    }
    // Equalities: held elevations, the balance. Inequalities: ±(z[k+1] − z[k]) ≤ G·L.
    let mut eq: Vec<(Vec<f64>, f64)> = Vec::new();
    for (k, p) in pvis.iter().enumerate() {
        if rules.held.get(k).copied().unwrap_or(false) {
            let mut row = vec![0.0; n];
            row[k] = 1.0;
            eq.push((row, p.elevation));
        }
    }
    if rules.balanced {
        eq.push((a.clone(), ground_area));
    }
    let mut ineq: Vec<(Vec<f64>, f64)> = Vec::new();
    if let Some(g) = rules.most_grade {
        for k in 0..n - 1 {
            let length = stations[k + 1] - stations[k];
            for sign in [1.0, -1.0] {
                let mut row = vec![0.0; n];
                row[k + 1] = sign;
                row[k] = -sign;
                ineq.push((row, g * length));
            }
        }
    }
    let mut active: Vec<usize> = Vec::new();
    let tol = |c: f64| 1e-9 * c.abs().max(1.0);
    let mut z = Vec::new();
    let mut done = false;
    for _ in 0..(20 * (n + ineq.len()) + 20) {
        let rows: Vec<&(Vec<f64>, f64)> = eq.iter().chain(active.iter().map(|i| &ineq[*i])).collect();
        let m = n + rows.len();
        let mut kkt = vec![vec![0.0; m]; m];
        let mut rhs = vec![0.0; m];
        for i in 0..n {
            kkt[i][..n].copy_from_slice(&h[i]);
            rhs[i] = b[i];
        }
        for (k, (row, c)) in rows.iter().enumerate() {
            for i in 0..n {
                kkt[i][n + k] = row[i];
                kkt[n + k][i] = row[i];
            }
            rhs[n + k] = *c;
        }
        // Singular: with no inequality taken, the ground and the equalities leave an elevation free (or the equalities
        // repeat); with one, the rules cannot hold together.
        let Some(x) = solve(kkt, rhs) else {
            return Err(if active.is_empty() { ProposeError::Unfixed } else { ProposeError::Infeasible });
        };
        z = x[..n].to_vec();
        // An active inequality pulling the wrong way leaves (the most negative first).
        let worst = active
            .iter()
            .enumerate()
            .map(|(k, i)| (k, *i, x[n + eq.len() + k]))
            .filter(|(_, i, l)| *l < -tol(ineq[*i].1))
            .min_by(|p, q| p.2.total_cmp(&q.2));
        if let Some((k, _, _)) = worst {
            active.remove(k);
            continue;
        }
        let over = ineq
            .iter()
            .enumerate()
            .filter(|(i, _)| !active.contains(i))
            .map(|(i, (row, c))| (i, row.iter().zip(&z).map(|(p, q)| p * q).sum::<f64>() - c))
            .filter(|(i, v)| *v > tol(ineq[*i].1))
            .max_by(|p, q| p.1.total_cmp(&q.1));
        match over {
            Some((i, _)) => active.push(i),
            None => {
                done = true;
                break;
            }
        }
    }
    if !done {
        return Err(ProposeError::Infeasible);
    }
    let grades: Vec<f64> = (0..n - 1).map(|k| (z[k + 1] - z[k]) / (stations[k + 1] - stations[k])).collect();
    let limited: Vec<i8> = (0..n - 1)
        .map(|k| {
            let m = |i: usize| active.contains(&i);
            if rules.most_grade.is_some() && m(2 * k) {
                1
            } else if rules.most_grade.is_some() && m(2 * k + 1) {
                -1
            } else {
                0
            }
        })
        .collect();
    // The squares and the areas, exactly: on each stretch the difference is a polynomial of degree two at most.
    let (mut squares, mut cut, mut fill) = (0.0, 0.0, 0.0);
    for &(s0, s1, g0, g1) in &pieces {
        let d = |s: f64| design(&stations, &r, &z, s) - (g0 + (g1 - g0) * (s - s0) / (s1 - s0));
        let half = 0.5 * (s1 - s0);
        for (x, w) in gauss() {
            let v = d(s0 + half * (1.0 + x));
            squares += w * half * v * v;
        }
        // d(t) = p0 + p1·t + p2·t², t ∈ [0, L].
        let length = s1 - s0;
        let (v0, vm, v1) = (d(s0), d(s0 + half), d(s1));
        let p2 = 2.0 * (v0 - 2.0 * vm + v1) / (length * length);
        let p1 = (v1 - v0) / length - p2 * length;
        let p0 = v0;
        let poly = |t: f64| p0 + p1 * t + p2 * t * t;
        let integral = |t0: f64, t1: f64| {
            p0 * (t1 - t0) + p1 * (t1 * t1 - t0 * t0) / 2.0 + p2 * (t1 * t1 * t1 - t0 * t0 * t0) / 3.0
        };
        let mut roots: Vec<f64> = Vec::new();
        if p2.abs() > 1e-14 * (p0.abs() + p1.abs() * length).max(1e-300) / (length * length) {
            let disc = p1 * p1 - 4.0 * p2 * p0;
            if disc > 0.0 {
                let q = -0.5 * (p1 + p1.signum() * disc.sqrt());
                roots.push(q / p2);
                if q != 0.0 {
                    roots.push(p0 / q);
                }
            }
        } else if p1 != 0.0 {
            roots.push(-p0 / p1);
        }
        let mut cuts: Vec<f64> = roots.into_iter().filter(|t| *t > 0.0 && *t < length).collect();
        cuts.sort_by(f64::total_cmp);
        let mut from = 0.0;
        for t in cuts.into_iter().chain(std::iter::once(length)) {
            if t > from {
                let v = integral(from, t);
                if poly(0.5 * (from + t)) >= 0.0 {
                    fill += v.max(0.0);
                } else {
                    cut += (-v).max(0.0);
                }
            }
            from = t;
        }
    }
    Ok(Proposal { elevations: z, grades, limited, squares, covered, cut, fill })
}
