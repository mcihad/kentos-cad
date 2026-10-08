//! Raster oturt's transforms (docs/adr/0204 §6), one for both platforms (the
//! web through WASM): from a raster's pixels (column, row) to the drawing,
//! fitted to control points. Helmert, affine and projective are Vektör
//! oturtma's least squares (`ops::fit`); polynomials of order 2 and 3 are
//! least squares in the points' centred and scaled frames; the thin plate is
//! Kauçuk levha's spline (`ops::rubber`), through every point. The inverse,
//! from the drawing to the pixels, is closed (Helmert, affine, projective);
//! the polynomials' and the thin plate's is Newton's method on the forward
//! from a guess fitted apart to the swapped points, so that a pixel sent
//! and brought back is where it was (GDAL refines its thin plate's reverse
//! so; its polynomials keep the reverse fitted apart, which is not the
//! forward's inverse: their warps differ from GDAL's by that fit's error).
//!
//! A raster's rows go down and the drawing's north up, so the pixel frame is
//! a mirror of the drawing's: Helmert (a similarity, which keeps the turn) is
//! fitted to (column, −row); the others take the mirror in their numbers.
//!
//! Every point's residual is the transformed pixel less its target (metres);
//! m0 = √(Σ v² / (2n − u)) with u = 4, 6, 8, 12, 20; the thin plate has none.

use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::jsmath::{js_hypot, js_max};
use crate::op;
use crate::ops::fit::{self, Fit, FitKind, FitPair};
use crate::ops::rubber::{Link, Sheet};
use crate::predicates::orientation;
use crate::vec2::Vec2;

/// A transform's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Helmert,
    Affine,
    Projective,
    Poly2,
    Poly3,
    ThinPlate,
}

impl Method {
    pub fn from_name(name: &str) -> Option<Method> {
        Some(match name {
            "helmert" => Method::Helmert,
            "affine" => Method::Affine,
            "projective" => Method::Projective,
            "poly2" => Method::Poly2,
            "poly3" => Method::Poly3,
            "thinPlate" => Method::ThinPlate,
            _ => return None,
        })
    }

    /// The fewest used points.
    pub fn need(self) -> usize {
        match self {
            Method::Helmert => 2,
            Method::Affine | Method::ThinPlate => 3,
            Method::Projective => 4,
            Method::Poly2 => 6,
            Method::Poly3 => 10,
        }
    }

    /// Numbers the solution has (u in m0's 2n − u).
    pub fn unknowns(self) -> usize {
        match self {
            Method::Helmert => 4,
            Method::Affine => 6,
            Method::Projective => 8,
            Method::Poly2 => 12,
            Method::Poly3 => 20,
            Method::ThinPlate => 0,
        }
    }

    /// Whether it places a raster without resampling (an affine).
    pub fn affine(self) -> bool {
        matches!(self, Method::Helmert | Method::Affine)
    }
}

/// A control point: a raster pixel (column, row), where it is to go, whether used.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gcp {
    pub pixel: Vec2,
    pub target: Vec2,
    pub used: bool,
}

crate::json_struct!(Gcp {
    pixel,
    target,
    used
});

/// Why there is no transform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeorefError {
    TooFew(usize),
    /// The pixels on one line or in one place.
    Collinear,
    Singular,
    /// Two used points at one pixel.
    Duplicate,
}

impl GeorefError {
    pub fn code(self) -> &'static str {
        match self {
            GeorefError::TooFew(_) => "too_few",
            GeorefError::Collinear => "collinear",
            GeorefError::Singular => "singular",
            GeorefError::Duplicate => "duplicate",
        }
    }
}

/// A polynomial map: centre and scale of its inputs, and the coefficients
/// of x and of y over the terms 1, x, y, x², xy, y², x³, x²y, xy², y³.
#[derive(Clone, Debug, PartialEq)]
pub struct Poly {
    order: u8,
    from: Vec2,
    scale: f64,
    to: Vec2,
    cx: Vec<f64>,
    cy: Vec<f64>,
}

/// The terms at (x, y); a second-order polynomial reads the first six.
fn terms(x: f64, y: f64) -> [f64; 10] {
    [
        1.0,
        x,
        y,
        x * x,
        x * y,
        y * y,
        x * x * x,
        x * x * y,
        x * y * y,
        y * y * y,
    ]
}

/// The terms' derivatives by x and by y.
fn slopes(x: f64, y: f64) -> ([f64; 10], [f64; 10]) {
    (
        [
            0.0,
            1.0,
            0.0,
            2.0 * x,
            y,
            0.0,
            3.0 * x * x,
            2.0 * x * y,
            y * y,
            0.0,
        ],
        [
            0.0,
            0.0,
            1.0,
            0.0,
            x,
            2.0 * y,
            0.0,
            x * x,
            2.0 * x * y,
            3.0 * y * y,
        ],
    )
}

fn dot(c: &[f64], t: &[f64; 10]) -> f64 {
    c.iter().zip(t).map(|(a, b)| a * b).sum::<f64>()
}

impl Poly {
    fn local(&self, p: Vec2) -> (f64, f64) {
        (
            (p.x - self.from.x) / self.scale,
            (p.y - self.from.y) / self.scale,
        )
    }

    fn map(&self, p: Vec2) -> Vec2 {
        let (x, y) = self.local(p);
        let t = terms(x, y);
        Vec2::new(self.to.x + dot(&self.cx, &t), self.to.y + dot(&self.cy, &t))
    }

    /// The derivative at `p` as [∂X/∂x, ∂Y/∂x, ∂X/∂y, ∂Y/∂y] (`Sheet::jacobian`'s order).
    fn jacobian(&self, p: Vec2) -> [f64; 4] {
        let (x, y) = self.local(p);
        let (dx, dy) = slopes(x, y);
        let s = self.scale;
        [
            dot(&self.cx, &dx) / s,
            dot(&self.cy, &dx) / s,
            dot(&self.cx, &dy) / s,
            dot(&self.cy, &dy) / s,
        ]
    }

    /// The least squares polynomial of `order` from `src` to `dst`.
    fn fit(order: u8, src: &[Vec2], dst: &[Vec2]) -> Result<Poly, GeorefError> {
        let n = src.len() as f64;
        let mean = |v: &[Vec2]| {
            let (b, mut sx, mut sy) = (v[0], 0.0, 0.0);
            for p in v {
                sx += p.x - b.x;
                sy += p.y - b.y;
            }
            Vec2::new(b.x + sx / n, b.y + sy / n)
        };
        let (from, to) = (mean(src), mean(dst));
        let scale = src
            .iter()
            .fold(0.0, |m, p| js_max(m, js_hypot(p.x - from.x, p.y - from.y)));
        if !(scale > 0.0) {
            return Err(GeorefError::Collinear);
        }
        let k = if order >= 3 { 10 } else { 6 };
        let mut ata = vec![vec![0.0f64; k]; k];
        let mut atx = vec![0.0f64; k];
        let mut aty = vec![0.0f64; k];
        for (s, d) in src.iter().zip(dst) {
            let t = terms((s.x - from.x) / scale, (s.y - from.y) / scale);
            let (tx, ty) = (d.x - to.x, d.y - to.y);
            for i in 0..k {
                atx[i] += t[i] * tx;
                aty[i] += t[i] * ty;
                for j in 0..k {
                    ata[i][j] += t[i] * t[j];
                }
            }
        }
        let cx = gauss(ata.clone(), atx).ok_or(GeorefError::Singular)?;
        let cy = gauss(ata, aty).ok_or(GeorefError::Singular)?;
        Ok(Poly {
            order,
            from,
            scale,
            to,
            cx,
            cy,
        })
    }
}

/// A pivot below this share of the largest diagonal: no single solution.
const SINGULAR: f64 = 1e-12;

/// Gaussian elimination with partial pivoting.
fn gauss(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    let largest = (0..n).fold(0.0, |m, i| js_max(m, a[i][i].abs()));
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() <= SINGULAR * largest {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for r in col + 1..n {
            let f = a[r][col] / a[col][col];
            if f == 0.0 {
                continue;
            }
            for c in col..n {
                a[r][c] -= f * a[col][c];
            }
            b[r] -= f * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let mut s = b[r];
        for c in r + 1..n {
            s -= a[r][c] * x[c];
        }
        x[r] = s / a[r][r];
    }
    Some(x)
}

/// One direction of a transform.
#[derive(Clone, Debug, PartialEq)]
enum Map {
    Fit(Fit),
    Poly(Poly),
    Sheet(Sheet),
}

impl Map {
    fn map(&self, p: Vec2) -> Option<Vec2> {
        match self {
            Map::Fit(f) => f.map(p),
            Map::Poly(q) => Some(q.map(p)),
            Map::Sheet(s) => Some(s.map(p)),
        }
    }

    /// Where `p` goes and the derivative there (`Sheet::jacobian`'s order).
    fn map_and_jacobian(&self, p: Vec2) -> Option<(Vec2, [f64; 4])> {
        match self {
            Map::Fit(_) => None,
            Map::Poly(q) => Some((q.map(p), q.jacobian(p))),
            Map::Sheet(s) => Some(s.map_and_jacobian(p)),
        }
    }
}

/// Newton's rounds at most before the inverse gives up.
const ROUNDS: usize = 30;

/// A step below this share of (1 + the pixel's largest coordinate) ends Newton's method.
const SETTLED: f64 = 1e-10;

/// A step that no longer halves is the forward's rounding (the drawing's
/// coordinates are large: 4 420 300 m's last bit is a nanometre, two
/// nanopixels at half a metre a pixel) when it is below this many pixels.
const NOISE: f64 = 1e-6;

/// The pixel `forward` sends to `target`, by Newton's method from `guess`;
/// none if it does not settle (beyond a fold of the map).
fn refine(forward: &Map, guess: Vec2, target: Vec2) -> Option<Vec2> {
    let mut p = guess;
    let mut last = f64::INFINITY;
    for _ in 0..ROUNDS {
        let (f, [a, b, c, d]) = forward.map_and_jacobian(p)?;
        // J = [a c; b d] (columns: a pixel right, a pixel down); the step is J⁻¹·(F(p) − target).
        let (rx, ry) = (f.x - target.x, f.y - target.y);
        let det = a * d - b * c;
        let (sx, sy) = ((d * rx - c * ry) / det, (a * ry - b * rx) / det);
        if !(sx.is_finite() && sy.is_finite()) {
            return None;
        }
        p = Vec2::new(p.x - sx, p.y - sy);
        let size = js_hypot(sx, sy);
        if size <= SETTLED * (1.0 + js_max(p.x.abs(), p.y.abs()))
            || (size > 0.5 * last && size <= NOISE)
        {
            return Some(p);
        }
        last = size;
    }
    None
}

/// A fitted transform.
#[derive(Clone, Debug, PartialEq)]
pub struct Georef {
    pub method: Method,
    /// The pixels are taken as (column, −row) (Helmert).
    flip: bool,
    forward: Map,
    /// The guess Newton's method starts from (polynomials, thin plate).
    inverse: Option<Map>,
    /// Every point's residual (x, y, length), metres; NaN beyond a projective's horizon.
    pub residuals: Vec<[f64; 3]>,
    pub m0: Option<f64>,
    /// Helmert's and the affine's `[x₀, a, b, y₀, c, d]` (pixel corners).
    pub affine: Option<[f64; 6]>,
}

impl Georef {
    /// Where a pixel (column, row) lands; none beyond a projective's horizon.
    pub fn forward(&self, p: Vec2) -> Option<Vec2> {
        self.forward.map(self.mirror(p))
    }

    fn mirror(&self, p: Vec2) -> Vec2 {
        if self.flip { Vec2::new(p.x, -p.y) } else { p }
    }

    /// The pixel a point of the drawing comes from: the forward's inverse.
    pub fn inverse(&self, p: Vec2) -> Option<Vec2> {
        let q = match (&self.inverse, &self.forward) {
            (Some(guess), forward) => refine(forward, guess.map(p)?, p),
            (None, Map::Fit(f)) => fit_inverse(f, p),
            _ => None,
        }?;
        Some(self.mirror(q))
    }
}

/// The pixel a closed-form fit sends to `p` (affine: the 2 × 2 inverse;
/// projective: the 2 × 2 system its equations make at `p`).
fn fit_inverse(f: &Fit, p: Vec2) -> Option<Vec2> {
    let (tx, ty) = (p.x - f.to.x, p.y - f.to.y);
    let (x, y) = match (f.kind, f.params.as_slice()) {
        (FitKind::Helmert, &[a, b]) => {
            let det = a * a + b * b;
            ((a * tx + b * ty) / det, (a * ty - b * tx) / det)
        }
        (FitKind::Affine, &[a, b, c, d]) => {
            let det = a * d - b * c;
            ((d * tx - c * ty) / det, (a * ty - b * tx) / det)
        }
        (FitKind::Projective, &[a1, a2, a3, b1, b2, b3, c1, c2]) => {
            let (m00, m01, r0) = (a1 - tx * c1, a2 - tx * c2, tx - a3);
            let (m10, m11, r1) = (b1 - ty * c1, b2 - ty * c2, ty - b3);
            let det = m00 * m11 - m01 * m10;
            ((m11 * r0 - m01 * r1) / det, (m00 * r1 - m10 * r0) / det)
        }
        _ => return None,
    };
    (x.is_finite() && y.is_finite()).then(|| Vec2::new(f.from.x + x, f.from.y + y))
}

/// The transform of `method` the used points give.
pub fn solve(points: &[Gcp], method: Method) -> Result<Georef, GeorefError> {
    let used: Vec<Gcp> = points.iter().copied().filter(|p| p.used).collect();
    if used.len() < method.need() {
        return Err(GeorefError::TooFew(method.need()));
    }
    for (i, a) in used.iter().enumerate() {
        if used[i + 1..].iter().any(|b| b.pixel == a.pixel) {
            return Err(GeorefError::Duplicate);
        }
    }
    let flip = method == Method::Helmert;
    let src: Vec<Vec2> = used
        .iter()
        .map(|p| {
            if flip {
                Vec2::new(p.pixel.x, -p.pixel.y)
            } else {
                p.pixel
            }
        })
        .collect();
    let dst: Vec<Vec2> = used.iter().map(|p| p.target).collect();
    if method != Method::Helmert {
        let (a, b) = (
            src[0],
            src.iter()
                .copied()
                .find(|&p| p != src[0])
                .ok_or(GeorefError::Collinear)?,
        );
        if src.iter().all(|&p| orientation(a, b, p) == 0) {
            return Err(GeorefError::Collinear);
        }
    }
    let pairs = |s: &[Vec2], d: &[Vec2]| -> Vec<FitPair> {
        s.iter()
            .zip(d)
            .map(|(&source, &target)| FitPair {
                source,
                target,
                used: true,
            })
            .collect()
    };
    let fit_err = |e: fit::FitError| match e {
        fit::FitError::TooFew(n) => GeorefError::TooFew(n),
        fit::FitError::Coincident | fit::FitError::Collinear => GeorefError::Collinear,
        fit::FitError::Singular => GeorefError::Singular,
    };
    let (forward, inverse) = match method {
        Method::Helmert | Method::Affine | Method::Projective => {
            let kind = match method {
                Method::Helmert => FitKind::Helmert,
                Method::Affine => FitKind::Affine,
                _ => FitKind::Projective,
            };
            (
                Map::Fit(fit::fit(&pairs(&src, &dst), kind).map_err(fit_err)?),
                None,
            )
        }
        Method::Poly2 | Method::Poly3 => {
            let order = if method == Method::Poly2 { 2 } else { 3 };
            (
                Map::Poly(Poly::fit(order, &src, &dst)?),
                Some(Map::Poly(Poly::fit(order, &dst, &src)?)),
            )
        }
        Method::ThinPlate => {
            let links = |s: &[Vec2], d: &[Vec2]| -> Vec<Link> {
                s.iter()
                    .zip(d)
                    .map(|(&from, &to)| Link { from, to })
                    .collect()
            };
            let sheet = |l: Vec<Link>| Sheet::solve(&l).map_err(|_| GeorefError::Singular);
            (
                Map::Sheet(sheet(links(&src, &dst))?),
                Some(Map::Sheet(sheet(links(&dst, &src))?)),
            )
        }
    };
    let mirror = |p: Vec2| if flip { Vec2::new(p.x, -p.y) } else { p };
    let mut vv = 0.0;
    let residuals: Vec<[f64; 3]> = points
        .iter()
        .map(|p| {
            let (vx, vy) = match forward.map(mirror(p.pixel)) {
                Some(q) => (q.x - p.target.x, q.y - p.target.y),
                None => (f64::NAN, f64::NAN),
            };
            if p.used {
                vv += vx * vx + vy * vy;
            }
            [vx, vy, js_hypot(vx, vy)]
        })
        .collect();
    let dof = (2 * used.len()).saturating_sub(method.unknowns());
    let m0 = (method != Method::ThinPlate && dof > 0).then(|| (vv / dof as f64).sqrt());
    let affine = match &forward {
        Map::Fit(f) => match (f.kind, f.params.as_slice()) {
            // Fitted to (column, −row): X = to + [a −b; b a]·((c, −r) − from), `from` in that frame.
            (FitKind::Helmert, &[a, b]) => {
                let (fx, fy) = (f.from.x, f.from.y);
                Some([
                    f.to.x - a * fx + b * fy,
                    a,
                    b,
                    f.to.y - b * fx - a * fy,
                    b,
                    -a,
                ])
            }
            (FitKind::Affine, &[a, b, c, d]) => Some(affine_of(f, a, c, b, d)),
            _ => None,
        },
        _ => None,
    };
    Ok(Georef {
        method,
        flip,
        forward,
        inverse,
        residuals,
        m0,
        affine,
    })
}

/// The pixel-to-drawing affine of a centred linear map
/// X = to + [m00 m01; m10 m11]·(p − from).
fn affine_of(f: &Fit, m00: f64, m01: f64, m10: f64, m11: f64) -> [f64; 6] {
    [
        f.to.x - (m00 * f.from.x + m01 * f.from.y),
        m00,
        m01,
        f.to.y - (m10 * f.from.x + m11 * f.from.y),
        m10,
        m11,
    ]
}

/// What both platforms read: residuals, m0, the affine (Helmert, affine), where
/// the probes go (pixels to the drawing) and where the back-probes come from,
/// or `{error, need}`.
pub struct GeorefAnswer(pub Result<Probed, GeorefError>);

/// A transform with where its probes went.
pub type Probed = (Georef, Vec<Option<Vec2>>, Vec<Option<Vec2>>);

impl ToJson for GeorefAnswer {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok((g, fwd, back)) => {
                field(out, &mut first, "residuals", &g.residuals);
                field(out, &mut first, "m0", &g.m0);
                field(out, &mut first, "affine", &g.affine.map(|a| a.to_vec()));
                field(out, &mut first, "forward", fwd);
                field(out, &mut first, "inverse", back);
            }
            Err(e) => {
                field(out, &mut first, "error", &e.code());
                if let GeorefError::TooFew(n) = e {
                    field(out, &mut first, "need", &(*n as f64));
                }
            }
        }
        out.push('}');
    }
}

pub(crate) static OPS: &[Op] = &[op!(
    "rasterGeoref",
    |points: Vec<Gcp>, method: String, probes: Vec<Vec2>, back: Vec<Vec2>| {
        GeorefAnswer(
            Method::from_name(&method)
                .ok_or(GeorefError::Singular)
                .and_then(|m| solve(&points, m))
                .map(|g| {
                    let fwd = probes.iter().map(|&p| g.forward(p)).collect();
                    let inv = back.iter().map(|&p| g.inverse(p)).collect();
                    (g, fwd, inv)
                }),
        )
    }
)];
