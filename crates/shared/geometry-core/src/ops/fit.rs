//! Vektör oturtma's solution (docs/adr/0156 §2–§3), one for both platforms
//! (the web through WASM): the Helmert, affine or projective transform that
//! best carries the used pairs' sources onto their targets, worked out in the
//! pairs' centred frames; every pair's residual and m0. The independent
//! reference is `scripts/fixtures/fit_cases.py` (exact fractions).
//!
//! National coordinates (4 420 000 m) are never multiplied together: the
//! centres are found from small differences to the first used pair, and
//! every sum is of centred coordinates (hundreds of metres).

use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::jsmath::{atan2, js_hypot, js_max};
use crate::op;
use crate::vec2::Vec2;

/// A control point (docs/adr/0156 §1): where it is in the drawing, where it
/// is to go, and whether it takes part in the solution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitPair {
    pub source: Vec2,
    pub target: Vec2,
    pub used: bool,
}

crate::json_struct!(FitPair {
    source,
    target,
    used
});

/// The transform's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitKind {
    /// A similarity: X̄ = a·x̄ − b·ȳ, Ȳ = b·x̄ + a·ȳ.
    Helmert,
    /// X̄ = a·x̄ + c·ȳ, Ȳ = b·x̄ + d·ȳ (the core's `Affine` order).
    Affine,
    /// X̄ = (a1·x̄ + a2·ȳ + a3) / (c1·x̄ + c2·ȳ + 1), Ȳ likewise with b1, b2, b3.
    Projective,
}

impl FitKind {
    pub fn from_name(name: &str) -> Option<FitKind> {
        match name {
            "helmert" => Some(FitKind::Helmert),
            "affine" => Some(FitKind::Affine),
            "projective" => Some(FitKind::Projective),
            _ => None,
        }
    }

    /// The fewest used pairs the kind needs.
    pub fn need(self) -> usize {
        match self {
            FitKind::Helmert => 2,
            FitKind::Affine => 3,
            FitKind::Projective => 4,
        }
    }

    /// How many numbers the solution has (u in m0's 2n − u).
    pub fn unknowns(self) -> usize {
        2 * self.need()
    }
}

/// Why there is no solution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitError {
    /// Fewer used pairs than the kind needs.
    TooFew(usize),
    /// Helmert: the sources all in one place.
    Coincident,
    /// Affine: the sources on one line.
    Collinear,
    /// Projective: the equations have no single solution.
    Singular,
}

impl FitError {
    pub fn code(self) -> &'static str {
        match self {
            FitError::TooFew(_) => "too_few",
            FitError::Coincident => "coincident",
            FitError::Collinear => "collinear",
            FitError::Singular => "singular",
        }
    }
}

/// A solution: the centres (`from` of the used sources, `to` of their
/// targets), the numbers between the centred frames (helmert a, b; affine
/// a, b, c, d; projective a1, a2, a3, b1, b2, b3, c1, c2), every pair's
/// residual (transformed source less target: x, y, length), m0 (none
/// without redundancy) and the derived values.
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    pub kind: FitKind,
    pub from: Vec2,
    pub to: Vec2,
    pub params: Vec<f64>,
    pub residuals: Vec<[f64; 3]>,
    pub m0: Option<f64>,
}

impl Fit {
    /// Where a centred source lands, centred; none beyond the projective
    /// transform's horizon (its denominator zero or less).
    fn map_centred(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        map_centred(self.kind, &self.params, x, y)
    }

    /// Where a source lands (none beyond the horizon).
    pub fn map(&self, p: Vec2) -> Option<Vec2> {
        let (x, y) = self.map_centred(p.x - self.from.x, p.y - self.from.y)?;
        Some(Vec2::new(self.to.x + x, self.to.y + y))
    }
}

/// The centred mapping of `kind` with `params` (none beyond a projective
/// transform's horizon, docs/adr/0156 §4).
pub fn map_centred(kind: FitKind, params: &[f64], x: f64, y: f64) -> Option<(f64, f64)> {
    match (kind, params) {
        (FitKind::Helmert, [a, b]) => Some((a * x - b * y, b * x + a * y)),
        (FitKind::Affine, [a, b, c, d]) => Some((a * x + c * y, b * x + d * y)),
        (FitKind::Projective, [a1, a2, a3, b1, b2, b3, c1, c2]) => {
            let w = c1 * x + c2 * y + 1.0;
            (w > 0.0).then(|| ((a1 * x + a2 * y + a3) / w, (b1 * x + b2 * y + b3) / w))
        }
        _ => None,
    }
}

impl ToJson for Fit {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "from", &self.from);
        field(out, &mut first, "to", &self.to);
        field(out, &mut first, "params", &self.params);
        field(out, &mut first, "residuals", &self.residuals);
        field(out, &mut first, "m0", &self.m0);
        let p = &self.params;
        match self.kind {
            FitKind::Helmert => {
                field(out, &mut first, "scale", &js_hypot(p[0], p[1]));
                field(out, &mut first, "rotation", &atan2(p[1], p[0]));
            }
            FitKind::Affine => {
                let (a, b, c, d) = (p[0], p[1], p[2], p[3]);
                field(out, &mut first, "scaleX", &js_hypot(a, b));
                field(out, &mut first, "scaleY", &js_hypot(c, d));
                field(out, &mut first, "rotation", &atan2(b, a));
                field(
                    out,
                    &mut first,
                    "shear",
                    &atan2(a * c + b * d, a * d - b * c),
                );
            }
            FitKind::Projective => {}
        }
        out.push('}');
    }
}

/// The answer as both platforms read it: the solution, or `{error, need}`.
pub struct FitAnswer(pub Result<Fit, FitError>);

impl ToJson for FitAnswer {
    fn write_json(&self, out: &mut String) {
        match &self.0 {
            Ok(fit) => fit.write_json(out),
            Err(e) => {
                out.push('{');
                let mut first = true;
                field(out, &mut first, "error", e.code());
                if let FitError::TooFew(n) = e {
                    field(out, &mut first, "need", &(*n as f64));
                }
                out.push('}');
            }
        }
    }
}

/// The used pairs' centres and their centred coordinates, from differences
/// to the first used pair (small numbers) so that nothing of the national
/// coordinates is lost.
struct Centred {
    from: Vec2,
    to: Vec2,
    src: Vec<(f64, f64)>,
    dst: Vec<(f64, f64)>,
    /// The first used pair (the differences' base) and the means of the differences.
    base: (Vec2, Vec2),
    mean: ((f64, f64), (f64, f64)),
}

fn centre(used: &[FitPair]) -> Centred {
    let (s0, t0) = (used[0].source, used[0].target);
    let n = used.len() as f64;
    let mut diffs = Vec::with_capacity(used.len());
    let (mut sx, mut sy, mut tx, mut ty) = (0.0, 0.0, 0.0, 0.0);
    for p in used {
        let d = (
            (p.source.x - s0.x, p.source.y - s0.y),
            (p.target.x - t0.x, p.target.y - t0.y),
        );
        sx += d.0.0;
        sy += d.0.1;
        tx += d.1.0;
        ty += d.1.1;
        diffs.push(d);
    }
    let ms = (sx / n, sy / n);
    let mt = (tx / n, ty / n);
    Centred {
        from: Vec2::new(s0.x + ms.0, s0.y + ms.1),
        to: Vec2::new(t0.x + mt.0, t0.y + mt.1),
        src: diffs.iter().map(|d| (d.0.0 - ms.0, d.0.1 - ms.1)).collect(),
        dst: diffs.iter().map(|d| (d.1.0 - mt.0, d.1.1 - mt.1)).collect(),
        base: (s0, t0),
        mean: (ms, mt),
    }
}

/// The transform of `kind` that best carries the used pairs' sources onto
/// their targets (docs/adr/0156 §2), with every pair's residual and m0 (§3).
pub fn fit(pairs: &[FitPair], kind: FitKind) -> Result<Fit, FitError> {
    let used: Vec<FitPair> = pairs.iter().copied().filter(|p| p.used).collect();
    if used.len() < kind.need() {
        return Err(FitError::TooFew(kind.need()));
    }
    let c = centre(&used);
    let params = match kind {
        FitKind::Helmert => helmert(&c.src, &c.dst)?,
        FitKind::Affine => affine(&c.src, &c.dst)?,
        FitKind::Projective => projective(&c.src, &c.dst)?,
    };
    // Every pair's residual in the centred frames: its source and target as
    // differences to the base, less the means (as the used ones were).
    let (s0, t0) = c.base;
    let ((msx, msy), (mtx, mty)) = c.mean;
    let mut vv = 0.0;
    let mut residuals = Vec::with_capacity(pairs.len());
    for p in pairs {
        let x = (p.source.x - s0.x) - msx;
        let y = (p.source.y - s0.y) - msy;
        let tx = (p.target.x - t0.x) - mtx;
        let ty = (p.target.y - t0.y) - mty;
        let (vx, vy) = match map_centred(kind, &params, x, y) {
            Some((mx, my)) => (mx - tx, my - ty),
            None => (f64::NAN, f64::NAN),
        };
        if p.used {
            vv += vx * vx + vy * vy;
        }
        residuals.push([vx, vy, js_hypot(vx, vy)]);
    }
    let dof = 2 * used.len() - kind.unknowns();
    let m0 = (dof > 0).then(|| (vv / dof as f64).sqrt());
    Ok(Fit {
        kind,
        from: c.from,
        to: c.to,
        params,
        residuals,
        m0,
    })
}

fn helmert(src: &[(f64, f64)], dst: &[(f64, f64)]) -> Result<Vec<f64>, FitError> {
    let mut s = 0.0;
    let mut pa = 0.0;
    let mut pb = 0.0;
    for (&(x, y), &(tx, ty)) in src.iter().zip(dst) {
        s += x * x + y * y;
        pa += x * tx + y * ty;
        pb += x * ty - y * tx;
    }
    if s <= 0.0 {
        return Err(FitError::Coincident);
    }
    Ok(vec![pa / s, pb / s])
}

/// The affine's normal determinant below this share of Σx̄²·Σȳ²: the
/// sources lie on one line (docs/adr/0156 §2).
const COLLINEAR: f64 = 1e-12;

fn affine(src: &[(f64, f64)], dst: &[(f64, f64)]) -> Result<Vec<f64>, FitError> {
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    let (mut sx_x, mut sy_x, mut sx_y, mut sy_y) = (0.0, 0.0, 0.0, 0.0);
    for (&(x, y), &(tx, ty)) in src.iter().zip(dst) {
        sxx += x * x;
        syy += y * y;
        sxy += x * y;
        sx_x += x * tx;
        sy_x += y * tx;
        sx_y += x * ty;
        sy_y += y * ty;
    }
    let det = sxx * syy - sxy * sxy;
    if det <= COLLINEAR * sxx * syy || det <= 0.0 {
        return Err(FitError::Collinear);
    }
    Ok(vec![
        (sx_x * syy - sy_x * sxy) / det,
        (sx_y * syy - sy_y * sxy) / det,
        (sy_x * sxx - sx_x * sxy) / det,
        (sy_y * sxx - sx_y * sxy) / det,
    ])
}

/// A pivot below this share of the normal matrix's largest diagonal: the
/// projective equations have no single solution (docs/adr/0156 §2).
const SINGULAR: f64 = 1e-12;

/// The projective transform's linearised equations in the centred frames,
/// scaled by each frame's largest coordinate (the solution does not depend
/// on the scale) so that the normal equations stay well conditioned; their
/// normal equations solved by Gaussian elimination with partial pivoting.
fn projective(src: &[(f64, f64)], dst: &[(f64, f64)]) -> Result<Vec<f64>, FitError> {
    let largest = |v: &[(f64, f64)]| {
        v.iter()
            .fold(0.0_f64, |m, &(x, y)| js_max(js_max(m, x.abs()), y.abs()))
    };
    let (s, t) = (largest(src), largest(dst));
    if s <= 0.0 || t <= 0.0 {
        return Err(FitError::Singular);
    }
    let mut ata = [[0.0_f64; 8]; 8];
    let mut atb = [0.0_f64; 8];
    for (&(x, y), &(tx, ty)) in src.iter().zip(dst) {
        let (x, y, tx, ty) = (x / s, y / s, tx / t, ty / t);
        for (row, rhs) in [
            ([x, y, 1.0, 0.0, 0.0, 0.0, -x * tx, -y * tx], tx),
            ([0.0, 0.0, 0.0, x, y, 1.0, -x * ty, -y * ty], ty),
        ] {
            for i in 0..8 {
                atb[i] += row[i] * rhs;
                for j in 0..8 {
                    ata[i][j] += row[i] * row[j];
                }
            }
        }
    }
    let largest_diagonal = (0..8).fold(0.0_f64, |m, i| js_max(m, ata[i][i]));
    let h = solve(ata, atb, SINGULAR * largest_diagonal).ok_or(FitError::Singular)?;
    // Back from the scaled frames: x̂ = x̄/s, X̂ = X̄/t.
    Ok(vec![
        h[0] * t / s,
        h[1] * t / s,
        h[2] * t,
        h[3] * t / s,
        h[4] * t / s,
        h[5] * t,
        h[6] / s,
        h[7] / s,
    ])
}

/// Gaussian elimination with partial pivoting; none when a pivot is at or
/// below `least`.
fn solve(mut a: [[f64; 8]; 8], mut b: [f64; 8], least: f64) -> Option<[f64; 8]> {
    for col in 0..8 {
        let pivot = (col..8).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() <= least {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for r in col + 1..8 {
            let f = a[r][col] / a[col][col];
            if f != 0.0 {
                for k in col..8 {
                    a[r][k] -= f * a[col][k];
                }
                b[r] -= f * b[col];
            }
        }
    }
    let mut x = [0.0_f64; 8];
    for i in (0..8).rev() {
        let s: f64 = (i + 1..8).map(|k| a[i][k] * x[k]).sum();
        x[i] = (b[i] - s) / a[i][i];
    }
    Some(x)
}

pub(crate) static OPS: &[Op] = &[op!("fitTransform", |pairs: Vec<FitPair>, kind: String| {
    FitAnswer(match FitKind::from_name(&kind) {
        Some(kind) => fit(&pairs, kind),
        None => Err(FitError::Singular),
    })
})];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::{FromJson, Json};

    /// The reference's cases (scripts/fixtures/fit_cases.py, exact
    /// fractions): the centres, parameters, residuals, m0 and derived values
    /// within the file's tolerances, or the same error.
    #[test]
    fn every_case_is_solved_as_the_reference_solves_it() {
        let text = include_str!("../../../../../fixtures/fit/v1/solve.json");
        let file = Json::parse(text).expect("solve.json reads");
        let tolerance = file.get("tolerance");
        let metres = f64::from_json(tolerance.get("metres")).expect("metres");
        let relative = f64::from_json(tolerance.get("relative")).expect("relative");
        let Json::Arr(cases) = file.get("cases") else {
            panic!("cases")
        };
        assert!(cases.len() >= 15, "{} cases", cases.len());
        let mut off = Vec::new();
        for case in cases {
            let name = String::from_json(case.get("name")).unwrap_or_default();
            let kind = String::from_json(case.get("kind")).unwrap_or_default();
            let kind = FitKind::from_name(&kind).expect("a kind");
            let pairs = Vec::<FitPair>::from_json(case.get("pairs")).expect("pairs read");
            let answer = crate::api::json::to_string(&FitAnswer(fit(&pairs, kind)));
            let got = Json::parse(&answer).expect("the answer reads");
            if let Err(e) = close(&got, case.get("expected"), metres, relative, "") {
                off.push(format!("{name}: {e}"));
            }
        }
        assert!(
            off.is_empty(),
            "{} durum farklı:\n{}",
            off.len(),
            off.join("\n")
        );
    }

    /// Whether `a` matches `e`: numbers under `from`, `to`, `residuals` and
    /// `m0` within `metres`, the others within `relative` of their size (at
    /// least 1); everything else exactly, no field more or less.
    fn close(a: &Json, e: &Json, metres: f64, relative: f64, path: &str) -> Result<(), String> {
        let in_metres = ["from", "to", "residuals", "m0"]
            .iter()
            .any(|k| path.starts_with(&format!(".{k}")));
        match (a, e) {
            (Json::Num(x), Json::Num(y)) => {
                let limit = if in_metres {
                    metres
                } else {
                    relative * js_max(y.abs(), 1.0)
                };
                if (x - y).abs() <= limit {
                    Ok(())
                } else {
                    Err(format!("{path}: {x} ≠ {y} (fark {:e})", (x - y).abs()))
                }
            }
            (Json::Arr(xs), Json::Arr(ys)) if xs.len() == ys.len() => {
                for (i, (x, y)) in xs.iter().zip(ys).enumerate() {
                    close(x, y, metres, relative, &format!("{path}[{i}]"))?;
                }
                Ok(())
            }
            (Json::Obj(xs), Json::Obj(ys)) => {
                for (k, y) in ys {
                    close(a.get(k), y, metres, relative, &format!("{path}.{k}"))?;
                }
                match xs.iter().find(|(k, _)| !ys.iter().any(|(k2, _)| k2 == k)) {
                    Some((k, _)) => Err(format!("{path}.{k}: fazladan alan")),
                    None => Ok(()),
                }
            }
            _ if a == e => Ok(()),
            _ => Err(format!("{path}: {a:?} ≠ {e:?}")),
        }
    }

    #[test]
    fn a_source_lands_where_its_target_is_on_an_exact_fit() {
        let pairs = [
            FitPair {
                source: Vec2::new(0.0, 0.0),
                target: Vec2::new(487_000.0, 4_420_000.0),
                used: true,
            },
            FitPair {
                source: Vec2::new(100.0, 0.0),
                target: Vec2::new(487_000.0, 4_420_100.0),
                used: true,
            },
        ];
        let f = fit(&pairs, FitKind::Helmert).expect("a solution");
        let p = f.map(Vec2::new(100.0, 0.0)).expect("a place");
        assert!(
            (p.x - 487_000.0).abs() < 1e-9 && (p.y - 4_420_100.0).abs() < 1e-9,
            "{p:?}"
        );
        assert_eq!(f.m0, None);
    }
}
