//! Transcendental functions from libm, so the browser (WASM) and the server
//! (native) read a file to the same bits (CLAUDE.md §23.4; the std methods
//! call the platform's C library natively and are forbidden by clippy.toml).
//! Angles that are whole multiples of 90° get exact sines and cosines: a
//! block inserted at 90° must not move its points by 10⁻¹⁶ of their size.

pub use std::f64::consts::PI;

pub const TAU: f64 = 2.0 * PI;

pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

pub fn atan(x: f64) -> f64 {
    libm::atan(x)
}

pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}

pub fn hypot(x: f64, y: f64) -> f64 {
    libm::hypot(x, y)
}

/// Degrees to radians as the app converts them (multiply, then divide).
pub fn rad(deg: f64) -> f64 {
    deg * PI / 180.0
}

/// Radians to degrees as the app converts them.
pub fn deg(rad: f64) -> f64 {
    rad * 180.0 / PI
}

/// (sin, cos) of an angle in degrees; exact for whole multiples of 90°.
pub fn sin_cos_deg(d: f64) -> (f64, f64) {
    let q = d.rem_euclid(360.0);
    if q == 0.0 {
        (0.0, 1.0)
    } else if q == 90.0 {
        (1.0, 0.0)
    } else if q == 180.0 {
        (0.0, -1.0)
    } else if q == 270.0 {
        (-1.0, 0.0)
    } else {
        let r = rad(d);
        (sin(r), cos(r))
    }
}

/// An angle in radians brought into [0, 2π).
pub fn norm_angle(a: f64) -> f64 {
    let r = a.rem_euclid(TAU);
    if r >= TAU { 0.0 } else { r }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_turns_are_exact() {
        assert_eq!(sin_cos_deg(90.0), (1.0, 0.0));
        assert_eq!(sin_cos_deg(-90.0), (-1.0, 0.0));
        assert_eq!(sin_cos_deg(450.0), (1.0, 0.0));
        assert_eq!(sin_cos_deg(180.0), (0.0, -1.0));
        let (s, c) = sin_cos_deg(30.0);
        assert!((s - 0.5).abs() < 1e-15 && (c - 0.75f64.sqrt()).abs() < 1e-15);
        assert_eq!(norm_angle(-PI / 2.0), 1.5 * PI);
        assert_eq!(norm_angle(TAU), 0.0);
    }
}
