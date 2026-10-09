//! The terrain's 3 × 3 kernels (docs/adr/0231 §3–§7): slope, aspect, the
//! shaded relief, curvature and ruggedness of a DEM, a row at a time. A
//! window is z1 … z9 from the top row (z5 the centre); a cell outside the
//! raster has already taken its nearest edge cell's value (the strip's
//! halo), a nodata (NaN) neighbour takes the centre's, a nodata centre
//! gives nodata. The order of the operations is the ADR's, so the
//! independent reference (`scripts/fixtures/terrain_cases.py`) works out the
//! same numbers.

use libm::{atan, atan2, cos, sin, sqrt};

use crate::frame::Gradient;

/// How the gradient is taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// Horn's (gdaldem's and ArcGIS's default).
    Horn,
    /// Zevenbergen and Thorne's.
    ZevenbergenThorne,
}

/// What Eğrilik gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurvatureKind {
    Total,
    Profile,
    Plan,
}

/// What Pürüzlülük gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuggednessKind {
    /// √Σ (nₖ − z5)² (Riley et al. 1999).
    TriRiley,
    /// Σ |nₖ − z5| / 8 (Wilson et al. 2007).
    TriWilson,
    /// z5 − Σ nₖ / 8.
    Tpi,
    /// The window's largest less its least.
    Roughness,
}

/// A kernel and its settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kernel {
    Slope {
        method: Method,
        percent: bool,
        z: f64,
    },
    Aspect {
        method: Method,
        z: f64,
    },
    Hillshade {
        azimuth: f64,
        altitude: f64,
        z: f64,
    },
    Curvature {
        kind: CurvatureKind,
        z: f64,
    },
    Ruggedness {
        kind: RuggednessKind,
    },
}

const DEG: f64 = 180.0 / core::f64::consts::PI;
const RAD: f64 = core::f64::consts::PI / 180.0;

/// The window round column `i` of three rows (each the raster's width and a
/// cell each side), its nodata neighbours the centre's; none when the centre is nodata.
#[inline]
pub fn window(up: &[f64], mid: &[f64], down: &[f64], i: usize) -> Option<[f64; 9]> {
    let c = mid[i + 1];
    if c.is_nan() {
        return None;
    }
    let mut w = [
        up[i],
        up[i + 1],
        up[i + 2],
        mid[i],
        c,
        mid[i + 2],
        down[i],
        down[i + 1],
        down[i + 2],
    ];
    for v in &mut w {
        if v.is_nan() {
            *v = c;
        }
    }
    Some(w)
}

/// The pixel derivatives (pᵢ along the row, pⱼ down the column).
#[inline]
pub fn derivatives(w: &[f64; 9], method: Method) -> (f64, f64) {
    match method {
        Method::Horn => (
            ((w[2] + 2.0 * w[5] + w[8]) - (w[0] + 2.0 * w[3] + w[6])) / 8.0,
            ((w[6] + 2.0 * w[7] + w[8]) - (w[0] + 2.0 * w[1] + w[2])) / 8.0,
        ),
        Method::ZevenbergenThorne => ((w[5] - w[3]) / 2.0, (w[7] - w[1]) / 2.0),
    }
}

/// Slope in degrees or percent from the gradient.
#[inline]
pub fn slope(gx: f64, gy: f64, percent: bool) -> f64 {
    let m = sqrt(gx * gx + gy * gy);
    if percent { 100.0 * m } else { atan(m) * DEG }
}

/// Aspect: the steepest way down from north clockwise, degrees; −1 flat.
#[inline]
pub fn aspect(gx: f64, gy: f64) -> f64 {
    if gx == 0.0 && gy == 0.0 {
        return -1.0;
    }
    let a = atan2(-gx, -gy) * DEG;
    // −0 is 0 (both targets write the same bits).
    (if a < 0.0 { a + 360.0 } else { a }) + 0.0
}

/// The shaded relief's light, worked out once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    sin_alt: f64,
    cos_alt_sin_az: f64,
    cos_alt_cos_az: f64,
}

impl Light {
    pub fn new(azimuth: f64, altitude: f64) -> Light {
        let (az, alt) = (azimuth * RAD, altitude * RAD);
        Light {
            sin_alt: sin(alt),
            cos_alt_sin_az: cos(alt) * sin(az),
            cos_alt_cos_az: cos(alt) * cos(az),
        }
    }

    /// The shade (1–255) of a surface of gradient (`gx`, `gy`), as the file holds it.
    #[inline]
    pub fn shade(&self, gx: f64, gy: f64) -> u8 {
        let c = (self.sin_alt - (gx * self.cos_alt_sin_az + gy * self.cos_alt_cos_az))
            / sqrt(1.0 + gx * gx + gy * gy);
        let v = if c <= 0.0 { 1.0 } else { 1.0 + 254.0 * c };
        // 1 ≤ v ≤ 255 here; the cast cannot wrap.
        libm::floor(v + 0.5) as u8
    }
}

/// Zevenbergen and Thorne's curvature of a window; `lx`, `ly` the pixel's
/// sides (metres), heights times `z`.
#[inline]
pub fn curvature(w: &[f64; 9], lx: f64, ly: f64, z: f64, kind: CurvatureKind) -> f64 {
    let z1 = w[0] * z;
    let z2 = w[1] * z;
    let z3 = w[2] * z;
    let z4 = w[3] * z;
    let z5 = w[4] * z;
    let z6 = w[5] * z;
    let z7 = w[6] * z;
    let z8 = w[7] * z;
    let z9 = w[8] * z;
    let d = ((z4 + z6) / 2.0 - z5) / (lx * lx);
    let e = ((z2 + z8) / 2.0 - z5) / (ly * ly);
    match kind {
        CurvatureKind::Total => -200.0 * (d + e),
        CurvatureKind::Profile | CurvatureKind::Plan => {
            let f = (z3 - z1 + z7 - z9) / (4.0 * lx * ly);
            let g = (z6 - z4) / (2.0 * lx);
            let h = (z2 - z8) / (2.0 * ly);
            let gh = g * g + h * h;
            if gh == 0.0 {
                return 0.0;
            }
            if kind == CurvatureKind::Profile {
                200.0 * (d * g * g + e * h * h + f * g * h) / gh
            } else {
                -200.0 * (d * h * h + e * g * g - f * g * h) / gh
            }
        }
    }
}

/// Ruggedness of a window (heights as they are).
#[inline]
pub fn ruggedness(w: &[f64; 9], kind: RuggednessKind) -> f64 {
    let c = w[4];
    let n = [w[0], w[1], w[2], w[3], w[5], w[6], w[7], w[8]];
    match kind {
        RuggednessKind::TriRiley => {
            let mut s = 0.0;
            for v in n {
                s += (v - c) * (v - c);
            }
            sqrt(s)
        }
        RuggednessKind::TriWilson => {
            let mut s = 0.0;
            for v in n {
                s += (v - c).abs();
            }
            s / 8.0
        }
        RuggednessKind::Tpi => {
            let mut s = 0.0;
            for v in n {
                s += v;
            }
            c - s / 8.0
        }
        RuggednessKind::Roughness => {
            let (mut lo, mut hi) = (c, c);
            for v in n {
                if v < lo {
                    lo = v;
                }
                if v > hi {
                    hi = v;
                }
            }
            hi - lo
        }
    }
}

/// A row's settings: its gradient (the pixel's axes and the z factor) and,
/// for curvature, its pixel's sides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowFrame {
    pub gradient: Gradient,
    pub lx: f64,
    pub ly: f64,
}

impl RowFrame {
    /// For a row's axes `[a, b, c, d]` (metres) and the z factor.
    pub fn new(axes: [f64; 4], z: f64) -> RowFrame {
        let [a, b, c, d] = axes;
        RowFrame {
            gradient: Gradient::new(axes, z),
            lx: sqrt(a * a + c * c),
            ly: sqrt(b * b + d * d),
        }
    }
}

impl Kernel {
    /// The z factor the row's gradient takes (1 for kernels that read heights as they are).
    pub fn z(&self) -> f64 {
        match *self {
            Kernel::Slope { z, .. } | Kernel::Aspect { z, .. } | Kernel::Hillshade { z, .. } => z,
            // Curvature scales the heights itself (its sides stay metres).
            Kernel::Curvature { .. } | Kernel::Ruggedness { .. } => 1.0,
        }
    }

    /// A row of 32-bit results from three rows of heights (NaN nodata).
    pub fn row_f32(&self, frame: &RowFrame, rows: [&[f64]; 3], out: &mut [f32]) {
        let [up, mid, down] = rows;
        for (i, o) in out.iter_mut().enumerate() {
            let Some(w) = window(up, mid, down, i) else {
                *o = f32::NAN;
                continue;
            };
            let v = match *self {
                Kernel::Slope {
                    method, percent, ..
                } => {
                    let (pi, pj) = derivatives(&w, method);
                    let (gx, gy) = frame.gradient.of(pi, pj);
                    slope(gx, gy, percent)
                }
                Kernel::Aspect { method, .. } => {
                    let (pi, pj) = derivatives(&w, method);
                    let (gx, gy) = frame.gradient.of(pi, pj);
                    aspect(gx, gy)
                }
                Kernel::Curvature { kind, z } => curvature(&w, frame.lx, frame.ly, z, kind),
                Kernel::Ruggedness { kind } => ruggedness(&w, kind),
                // An 8-bit result (`row_u8`).
                Kernel::Hillshade { .. } => f64::NAN,
            };
            *o = v as f32;
        }
    }

    /// A row of the shaded relief (1–255; 0 nodata).
    pub fn row_u8(&self, frame: &RowFrame, light: &Light, rows: [&[f64]; 3], out: &mut [u8]) {
        let [up, mid, down] = rows;
        for (i, o) in out.iter_mut().enumerate() {
            *o = match window(up, mid, down, i) {
                Some(w) => {
                    let (pi, pj) = derivatives(&w, Method::Horn);
                    let (gx, gy) = frame.gradient.of(pi, pj);
                    light.shade(gx, gy)
                }
                None => 0,
            };
        }
    }
}
