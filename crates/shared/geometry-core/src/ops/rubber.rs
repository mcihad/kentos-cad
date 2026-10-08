//! Kauçuk levha's solution (docs/adr/0158 §2), one for both platforms (the
//! web through WASM): the thin plate spline of the links' displacements.
//! It passes exactly through every link (a fixed point is a link whose
//! target is its source), bends as little as it can between them, and far
//! from them follows their affine trend; it needs no triangulation, so it
//! is one surface whatever the links' order. The independent reference is
//! `scripts/fixtures/rubber_cases.py` (mpmath, 50 digits).
//!
//! The displacement d(p) = target − source is, for each of east and north,
//! a₀ + a₁·x + a₂·y + Σ wᵢ·φ(|p − sᵢ|) with φ(r) = r²·ln r, where
//! d(sᵢ) = dᵢ and Σ wᵢ = Σ wᵢ·xᵢ = Σ wᵢ·yᵢ = 0. Positions are taken from
//! the sources' centre over their largest distance from it: the spline does
//! not depend on the scale (a scale changes only its affine part) and the
//! equations stay well conditioned at national coordinates.

use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::jsmath::{js_hypot, js_max, log};
use crate::op;
use crate::predicates::orientation;
use crate::vec2::Vec2;

/// A link: a point of the drawing and where it is to go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Link {
    pub from: Vec2,
    pub to: Vec2,
}

crate::json_struct!(Link { from, to });

/// The most links a sheet takes: the equations grow with their square.
pub const MAX_LINKS: usize = 1000;

/// Why there is no sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RubberError {
    /// Fewer than three links.
    TooFew,
    /// Two links from one point.
    Duplicate,
    /// The sources on one line.
    Collinear,
    /// The equations have no single solution.
    Singular,
    /// More than `MAX_LINKS`.
    TooMany,
}

impl RubberError {
    pub fn code(self) -> &'static str {
        match self {
            RubberError::TooFew => "too_few",
            RubberError::Duplicate => "duplicate",
            RubberError::Collinear => "collinear",
            RubberError::Singular => "singular",
            RubberError::TooMany => "too_many",
        }
    }
}

/// A solved sheet: the frame positions are taken in, the sources in it,
/// and the spline's numbers for east and north.
#[derive(Clone, Debug, PartialEq)]
pub struct Sheet {
    centre: Vec2,
    /// Metres a unit of the frame is.
    size: f64,
    sources: Vec<Vec2>,
    /// wᵢ for east and north.
    weights: Vec<[f64; 2]>,
    /// a₀, a₁, a₂ for east and north.
    affine: [[f64; 2]; 3],
}

/// φ(r) of a squared distance: r²·ln r = ½·r²·ln r², 0 at 0.
fn phi(r2: f64) -> f64 {
    if r2 > 0.0 { 0.5 * r2 * log(r2) } else { 0.0 }
}

/// A pivot at or below this share of the equations' largest number: they
/// have no single solution (docs/adr/0158 §2).
const SINGULAR: f64 = 1e-12;

impl Sheet {
    /// The sheet through `links`, or why there is none.
    pub fn solve(links: &[Link]) -> Result<Sheet, RubberError> {
        let n = links.len();
        if n < 3 {
            return Err(RubberError::TooFew);
        }
        if n > MAX_LINKS {
            return Err(RubberError::TooMany);
        }
        for (i, a) in links.iter().enumerate() {
            if links[i + 1..].iter().any(|b| b.from == a.from) {
                return Err(RubberError::Duplicate);
            }
        }
        let first = links[0].from;
        let Some(second) = links.iter().map(|l| l.from).find(|&p| p != first) else {
            return Err(RubberError::Collinear);
        };
        if links
            .iter()
            .all(|l| orientation(first, second, l.from) == 0)
        {
            return Err(RubberError::Collinear);
        }
        // The frame: the sources' centre, found from differences to the
        // first (small numbers), over their largest distance from it.
        let (mut sx, mut sy) = (0.0, 0.0);
        for l in links {
            sx += l.from.x - first.x;
            sy += l.from.y - first.y;
        }
        let centre = Vec2::new(first.x + sx / n as f64, first.y + sy / n as f64);
        let mut size: f64 = 0.0;
        for l in links {
            size = js_max(size, js_hypot(l.from.x - centre.x, l.from.y - centre.y));
        }
        let sources: Vec<Vec2> = links
            .iter()
            .map(|l| Vec2::new((l.from.x - centre.x) / size, (l.from.y - centre.y) / size))
            .collect();
        // [K P; Pᵀ 0] [w; a] = [d; 0], both components at once.
        let m = n + 3;
        let mut a = vec![vec![0.0_f64; m]; m];
        let mut b = vec![[0.0_f64; 2]; m];
        for i in 0..n {
            for j in 0..n {
                let (dx, dy) = (sources[i].x - sources[j].x, sources[i].y - sources[j].y);
                a[i][j] = phi(dx * dx + dy * dy);
            }
            let p = sources[i];
            for (k, v) in [1.0, p.x, p.y].into_iter().enumerate() {
                a[i][n + k] = v;
                a[n + k][i] = v;
            }
            b[i] = [
                links[i].to.x - links[i].from.x,
                links[i].to.y - links[i].from.y,
            ];
        }
        let x = gauss(a, b).ok_or(RubberError::Singular)?;
        Ok(Sheet {
            centre,
            size,
            sources,
            weights: x[..n].to_vec(),
            affine: [x[n], x[n + 1], x[n + 2]],
        })
    }

    fn frame(&self, p: Vec2) -> Vec2 {
        Vec2::new(
            (p.x - self.centre.x) / self.size,
            (p.y - self.centre.y) / self.size,
        )
    }

    /// The displacement at `p`, metres.
    pub fn displacement(&self, p: Vec2) -> Vec2 {
        let q = self.frame(p);
        let [a0, a1, a2] = self.affine;
        let mut d = [
            a0[0] + a1[0] * q.x + a2[0] * q.y,
            a0[1] + a1[1] * q.x + a2[1] * q.y,
        ];
        for (s, w) in self.sources.iter().zip(&self.weights) {
            let (dx, dy) = (q.x - s.x, q.y - s.y);
            let f = phi(dx * dx + dy * dy);
            d[0] += w[0] * f;
            d[1] += w[1] * f;
        }
        Vec2::new(d[0], d[1])
    }

    /// Where `p` goes.
    pub fn map(&self, p: Vec2) -> Vec2 {
        let d = self.displacement(p);
        Vec2::new(p.x + d.x, p.y + d.y)
    }

    /// The map's derivative at `p` as [a, b, c, d], its columns (a, b) and
    /// (c, d): the images of a metre east and of a metre north (the core's
    /// `Affine` order). dφ/dx = (x − xᵢ)·(2·ln r + 1), 0 at the source.
    pub fn jacobian(&self, p: Vec2) -> [f64; 4] {
        let q = self.frame(p);
        let [_, a1, a2] = self.affine;
        // ∂d/∂x̂ and ∂d/∂ŷ for east and north.
        let mut gx = [a1[0], a1[1]];
        let mut gy = [a2[0], a2[1]];
        for (s, w) in self.sources.iter().zip(&self.weights) {
            let (dx, dy) = (q.x - s.x, q.y - s.y);
            let r2 = dx * dx + dy * dy;
            if r2 > 0.0 {
                let k = log(r2) + 1.0;
                gx[0] += w[0] * dx * k;
                gx[1] += w[1] * dx * k;
                gy[0] += w[0] * dy * k;
                gy[1] += w[1] * dy * k;
            }
        }
        let s = self.size;
        [1.0 + gx[0] / s, gx[1] / s, gy[0] / s, 1.0 + gy[1] / s]
    }

    /// `map` and `jacobian` at once, one logarithm a source (Raster oturt's
    /// inverse takes both every round of Newton's method).
    pub fn map_and_jacobian(&self, p: Vec2) -> (Vec2, [f64; 4]) {
        let q = self.frame(p);
        let [a0, a1, a2] = self.affine;
        let mut d = [
            a0[0] + a1[0] * q.x + a2[0] * q.y,
            a0[1] + a1[1] * q.x + a2[1] * q.y,
        ];
        let mut gx = [a1[0], a1[1]];
        let mut gy = [a2[0], a2[1]];
        for (s, w) in self.sources.iter().zip(&self.weights) {
            let (dx, dy) = (q.x - s.x, q.y - s.y);
            let r2 = dx * dx + dy * dy;
            if r2 > 0.0 {
                let l = log(r2);
                let f = 0.5 * r2 * l;
                d[0] += w[0] * f;
                d[1] += w[1] * f;
                let k = l + 1.0;
                gx[0] += w[0] * dx * k;
                gx[1] += w[1] * dx * k;
                gy[0] += w[0] * dy * k;
                gy[1] += w[1] * dy * k;
            }
        }
        let s = self.size;
        (
            Vec2::new(p.x + d[0], p.y + d[1]),
            [1.0 + gx[0] / s, gx[1] / s, gy[0] / s, 1.0 + gy[1] / s],
        )
    }
}

/// Gaussian elimination with partial pivoting of `a` against two
/// right-hand sides; none when a pivot is at or below `SINGULAR` of the
/// largest number of `a`.
fn gauss(mut a: Vec<Vec<f64>>, mut b: Vec<[f64; 2]>) -> Option<Vec<[f64; 2]>> {
    let m = a.len();
    let mut big: f64 = 0.0;
    for row in &a {
        for v in row {
            big = js_max(big, v.abs());
        }
    }
    let least = SINGULAR * big;
    for col in 0..m {
        let pivot = (col..m).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        let size = a[pivot][col].abs();
        if size.is_nan() || size <= least {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for r in col + 1..m {
            let f = a[r][col] / a[col][col];
            if f != 0.0 {
                for k in col..m {
                    a[r][k] -= f * a[col][k];
                }
                b[r][0] -= f * b[col][0];
                b[r][1] -= f * b[col][1];
            }
        }
    }
    let mut x = vec![[0.0_f64; 2]; m];
    for i in (0..m).rev() {
        let mut s = [0.0, 0.0];
        for k in i + 1..m {
            s[0] += a[i][k] * x[k][0];
            s[1] += a[i][k] * x[k][1];
        }
        x[i] = [(b[i][0] - s[0]) / a[i][i], (b[i][1] - s[1]) / a[i][i]];
    }
    Some(x)
}

/// What both platforms read: where the probes go and the map's derivative
/// there, or `{error}`.
pub struct RubberAnswer(pub Result<(Vec<Vec2>, Vec<[f64; 4]>), RubberError>);

impl ToJson for RubberAnswer {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok((map, jacobian)) => {
                field(out, &mut first, "map", map);
                field(out, &mut first, "jacobian", jacobian);
            }
            Err(e) => field(out, &mut first, "error", &e.code()),
        }
        out.push('}');
    }
}

pub(crate) static OPS: &[Op] = &[op!("rubberSheet", |links: Vec<Link>, probes: Vec<Vec2>| {
    RubberAnswer(Sheet::solve(&links).map(|sheet| {
        (
            probes.iter().map(|&p| sheet.map(p)).collect(),
            probes.iter().map(|&p| sheet.jacobian(p)).collect(),
        )
    }))
})];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::{FromJson, Json};

    /// The reference's cases (scripts/fixtures/rubber_cases.py, mpmath):
    /// where every probe goes and the derivative there, within the file's
    /// tolerances, or the same error.
    #[test]
    fn every_case_is_solved_as_the_reference_solves_it() {
        let text = include_str!("../../../../../fixtures/fit/v1/rubber.json");
        let file = Json::parse(text).expect("rubber.json reads");
        let tolerance = file.get("tolerance");
        let metres = f64::from_json(tolerance.get("metres")).expect("metres");
        let slope = f64::from_json(tolerance.get("jacobian")).expect("jacobian");
        let Json::Arr(cases) = file.get("cases") else {
            panic!("cases")
        };
        assert!(cases.len() >= 9, "{} cases", cases.len());
        let mut off = Vec::new();
        for case in cases {
            let name = String::from_json(case.get("name")).unwrap_or_default();
            let links = Vec::<Link>::from_json(case.get("links")).expect("links");
            let probes = Vec::<Vec2>::from_json(case.get("probes")).expect("probes");
            let expected = case.get("expected");
            match (
                Sheet::solve(&links),
                String::from_json(expected.get("error")).ok(),
            ) {
                (Err(e), Some(code)) => {
                    if e.code() != code {
                        off.push(format!("{name}: {} ≠ {code}", e.code()));
                    }
                }
                (Ok(sheet), None) => {
                    let map = Vec::<Vec2>::from_json(expected.get("map")).expect("map");
                    let jac =
                        Vec::<[f64; 4]>::from_json(expected.get("jacobian")).expect("jacobian");
                    for (i, &p) in probes.iter().enumerate() {
                        let got = sheet.map(p);
                        let d = js_hypot(got.x - map[i].x, got.y - map[i].y);
                        if d.is_nan() || d > metres {
                            off.push(format!("{name}: probe {i}: {got:?} ≠ {:?} ({d} m)", map[i]));
                        }
                        let j = sheet.jacobian(p);
                        if (0..4).any(|k| {
                            let e = (j[k] - jac[i][k]).abs();
                            e.is_nan() || e > slope
                        }) {
                            off.push(format!("{name}: probe {i}: J {j:?} ≠ {:?}", jac[i]));
                        }
                    }
                }
                (got, want) => off.push(format!("{name}: {:?} ≠ {want:?}", got.map(|_| ()))),
            }
        }
        assert!(off.is_empty(), "{}", off.join("\n"));
    }

    #[test]
    fn too_many_links_are_refused_before_any_work() {
        let links: Vec<Link> = (0..=MAX_LINKS)
            .map(|i| {
                let p = Vec2::new(i as f64, (i * i % 7) as f64);
                Link { from: p, to: p }
            })
            .collect();
        assert_eq!(Sheet::solve(&links), Err(RubberError::TooMany));
    }

    #[test]
    fn the_links_are_passed_exactly_and_fixed_points_stay() {
        let links = [
            Link {
                from: Vec2::new(487_000.0, 4_420_000.0),
                to: Vec2::new(487_000.0, 4_420_000.0),
            },
            Link {
                from: Vec2::new(487_100.0, 4_420_000.0),
                to: Vec2::new(487_100.03, 4_419_999.98),
            },
            Link {
                from: Vec2::new(487_100.0, 4_420_100.0),
                to: Vec2::new(487_100.0, 4_420_100.0),
            },
            Link {
                from: Vec2::new(487_000.0, 4_420_100.0),
                to: Vec2::new(487_000.0, 4_420_100.05),
            },
            Link {
                from: Vec2::new(487_050.0, 4_420_050.0),
                to: Vec2::new(487_050.0, 4_420_050.0),
            },
        ];
        let sheet = Sheet::solve(&links).expect("a sheet");
        for l in links {
            let p = sheet.map(l.from);
            assert!(
                js_hypot(p.x - l.to.x, p.y - l.to.y) < 1e-9,
                "{p:?} ≠ {:?}",
                l.to
            );
        }
    }
}
