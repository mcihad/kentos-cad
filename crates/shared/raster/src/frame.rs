//! A raster's grid in the drawing (docs/adr/0231 §2): its affine
//! `[x₀, a, b, y₀, c, d]` (x = x₀ + a·i + b·j, y = y₀ + c·i + d·j, pixel
//! corners), and, on a geographic system, how many metres a degree is at a
//! row's latitude (GRS80). The terrain's kernels take a row's pixel axes in
//! metres from here.

use libm::{cos, sin, sqrt};

/// GRS80's semi-major axis (m) and flattening.
pub const GRS80_A: f64 = 6_378_137.0;
pub const GRS80_F: f64 = 1.0 / 298.257_222_101;

/// A raster's grid.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub affine: [f64; 6],
    pub width: u32,
    pub height: u32,
    /// The system's coordinates are degrees (longitude, latitude).
    pub geographic: bool,
}

/// The metres of a degree east and north at latitude `phi` (degrees) on GRS80.
pub fn degree_metres(phi: f64) -> (f64, f64) {
    let e2 = GRS80_F * (2.0 - GRS80_F);
    let r = phi * core::f64::consts::PI / 180.0;
    let s = sin(r);
    let w = 1.0 - e2 * s * s;
    let n = GRS80_A / sqrt(w);
    let m = GRS80_A * (1.0 - e2) / (w * sqrt(w));
    let k = core::f64::consts::PI / 180.0;
    (n * cos(r) * k, m * k)
}

impl Frame {
    /// A grid whose affine turns (its linear part invertible, every number finite).
    pub fn new(
        affine: [f64; 6],
        width: u32,
        height: u32,
        geographic: bool,
    ) -> Result<Frame, String> {
        let [_, a, b, _, c, d] = affine;
        let det = a * d - b * c;
        if affine.iter().any(|v| !v.is_finite()) || det == 0.0 || !det.is_finite() {
            return Err(
                "Rasterin yeri (afin dönüşümü) tersinmiyor; önce Raster oturt ile yerleştirin."
                    .into(),
            );
        }
        if width == 0 || height == 0 {
            return Err("Rasterin boyu sıfır.".into());
        }
        Ok(Frame {
            affine,
            width,
            height,
            geographic,
        })
    }

    /// The drawing point of pixel coordinates (`i`, `j`) (pixel corners; a
    /// centre is (i + ½, j + ½)).
    #[inline]
    pub fn point(&self, i: f64, j: f64) -> (f64, f64) {
        let [x0, a, b, y0, c, d] = self.affine;
        (x0 + a * i + b * j, y0 + c * i + d * j)
    }

    /// The latitude (degrees) a geographic grid's row is measured at: its
    /// middle column's cell centre (docs/adr/0231 §2).
    pub fn row_latitude(&self, j: u32) -> f64 {
        self.point(f64::from(self.width) / 2.0, f64::from(j) + 0.5)
            .1
    }

    /// The row's pixel axes in metres: `[a, b, c, d]`, a step along a row
    /// (a, c) and down a column (b, d) east and north.
    pub fn axes(&self, j: u32) -> [f64; 4] {
        let [_, a, b, _, c, d] = self.affine;
        if self.geographic {
            let (ke, kn) = degree_metres(self.row_latitude(j));
            [a * ke, b * ke, c * kn, d * kn]
        } else {
            [a, b, c, d]
        }
    }
}

/// What turns a row's pixel derivatives (pᵢ, pⱼ) into the drawing's
/// gradient (east, north): g = J⁻ᵀ·p (docs/adr/0231 §3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gradient {
    k1: f64,
    k2: f64,
    k3: f64,
    k4: f64,
}

impl Gradient {
    /// For a row's axes `[a, b, c, d]` (metres) and the z factor.
    pub fn new(axes: [f64; 4], z: f64) -> Gradient {
        let [a, b, c, d] = axes;
        let det = a * d - b * c;
        Gradient {
            k1: z * d / det,
            k2: -z * c / det,
            k3: -z * b / det,
            k4: z * a / det,
        }
    }

    /// The gradient (east, north) of pixel derivatives along a row (`pi`) and down a column (`pj`).
    #[inline]
    pub fn of(&self, pi: f64, pj: f64) -> (f64, f64) {
        (self.k1 * pi + self.k2 * pj, self.k3 * pi + self.k4 * pj)
    }
}
