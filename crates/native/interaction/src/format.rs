//! Numbers as the user reads them, from the project's settings: the web's
//! `Formatter` (`apps/web/src/app/format.ts`). Display only (CLAUDE.md §5,
//! §23.2): nothing here rounds a coordinate that is kept. The decimal
//! separator is the point, as typed input takes it.

use kentos_contracts::{AngleUnit, AreaUnit, ProjectSettings};

use crate::Vec2;

/// Grads per degree.
const GRAD_PER_DEG: f64 = 400.0 / 360.0;

/// The unit settings formatting depends on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Format {
    pub length_decimals: usize,
    pub area_decimals: usize,
    pub area_unit: AreaUnit,
    pub angle_unit: AngleUnit,
}

impl Default for Format {
    /// A new project's settings (web: `DEFAULT_PROJECT_SETTINGS`).
    fn default() -> Self {
        Self {
            length_decimals: 3,
            area_decimals: 2,
            area_unit: AreaUnit::M2,
            angle_unit: AngleUnit::Grad,
        }
    }
}

impl Format {
    pub fn of(settings: &ProjectSettings) -> Self {
        Self {
            length_decimals: (settings.length_decimals as usize).min(12),
            area_decimals: (settings.area_decimals as usize).min(12),
            area_unit: settings.area_unit,
            angle_unit: settings.angle_unit,
        }
    }

    /// A grid coordinate (Y or X) without a unit.
    pub fn coord(&self, v: f64) -> String {
        fixed(v, self.length_decimals)
    }

    pub fn length(&self, metres: f64) -> String {
        format!("{} m", fixed(metres, self.length_decimals))
    }

    /// A length without its unit (the web's `length(m, false)`): `20.000 × 10.000 m`.
    pub fn length_bare(&self, metres: f64) -> String {
        fixed(metres, self.length_decimals)
    }

    pub fn area(&self, square_metres: f64) -> String {
        let d = self.area_decimals;
        match self.area_unit {
            AreaUnit::Donum => format!("{} dönüm", fixed(square_metres / 1000.0, d)),
            AreaUnit::Ha => format!("{} ha", fixed(square_metres / 10_000.0, d)),
            AreaUnit::M2 => format!("{} m²", fixed(square_metres, d)),
        }
    }

    /// A bearing given in grads (the surveying semt), in the project's angle unit.
    pub fn bearing(&self, grad: f64) -> String {
        match self.angle_unit {
            AngleUnit::Deg => format!("{}°", fixed(grad / GRAD_PER_DEG, 4)),
            AngleUnit::Grad => format!("{} g", fixed(grad, 4)),
        }
    }

    /// `Y 487012.000  X 4420000.000`: east first (CLAUDE.md §5).
    pub fn point(&self, p: Vec2) -> String {
        format!("Y {}  X {}", self.coord(p.x), self.coord(p.y))
    }
}

/// JavaScript's `v.toFixed(d)`, so the desktop shows the web's digits: an
/// exact half rounds away from zero (Rust's formatting rounds it to even:
/// 487012.0625 is `…063` on the web, `…062` in Rust), and a negative zero
/// is written without its sign. Display only (CLAUDE.md §23.2).
pub(crate) fn fixed(v: f64, d: usize) -> String {
    if v.is_nan() {
        return "NaN".to_owned();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    let x = v.abs();
    let body = if exact_half(x, d) {
        // The digits up to the half are exact; drop the 5 and add one in the last place.
        let long = format!("{x:.prec$}", prec = d + 1);
        up_one(long[..long.len() - 1].trim_end_matches('.'))
    } else {
        format!("{x:.d$}")
    };
    if v < 0.0 { format!("-{body}") } else { body }
}

/// Whether `x` (finite, not negative) lies exactly halfway between two
/// multiples of 10^-d: `x × 10^d` has a fractional part of exactly one half.
fn exact_half(x: f64, d: usize) -> bool {
    if x == 0.0 || d > 12 {
        return false;
    }
    let bits = x.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i64;
    let fraction = bits & ((1 << 52) - 1);
    // x = mantissa × 2^power, exactly.
    let (mantissa, power) = if exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1 << 52), exponent - 1075)
    };
    if power >= 0 {
        return false;
    }
    // mantissa < 2^53 and 10^12 < 2^40: the product fits in 93 bits, so a
    // half (2^(k-1)) is out of reach once k exceeds 93.
    let k = -power;
    if k > 93 {
        return false;
    }
    let scaled = u128::from(mantissa) * 10u128.pow(d as u32);
    scaled % (1u128 << k) == 1u128 << (k - 1)
}

/// A decimal string plus one unit in its last place: `9.99` → `10.00`.
fn up_one(digits: &str) -> String {
    let mut chars: Vec<char> = digits.chars().collect();
    let mut i = chars.len();
    loop {
        if i == 0 {
            chars.insert(0, '1');
            break;
        }
        i -= 1;
        match chars[i] {
            '.' => {}
            '9' => chars[i] = '0',
            c => {
                chars[i] = char::from(c as u8 + 1);
                break;
            }
        }
    }
    chars.into_iter().collect()
}

/// A number as JavaScript writes it (`String(n)`), for the values the tools
/// show after `+x.toFixed(d)`: the shortest digits, no trailing zeros, no
/// sign on zero. Numbers this small in magnitude are never written with an
/// exponent by either.
pub(crate) fn js_number(v: f64) -> String {
    if v == 0.0 {
        return "0".to_owned();
    }
    format!("{v}")
}

/// An angle in radians as the rectangle tool writes its rotation (the web's
/// `fmtDeg`: `+((rad / DEG) % 360).toFixed(4)` and a degree sign): `30°`, `12.3457°`.
pub(crate) fn short_degrees(rad: f64) -> String {
    let deg = (rad / (kentos_geometry_core::jsmath::PI / 180.0)) % 360.0;
    let rounded = fixed(deg, 4).parse::<f64>().unwrap_or(f64::NAN);
    format!("{}°", js_number(rounded))
}

/// Radians in degrees as the arc tool writes them (the web's `deg`: `rad * 180 / π`).
pub(crate) fn degrees(rad: f64) -> f64 {
    (rad * 180.0) / kentos_geometry_core::jsmath::PI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degrees_read_as_the_web_writes_them() {
        use kentos_geometry_core::jsmath::PI;
        assert_eq!(short_degrees(0.0), "0°");
        assert_eq!(short_degrees(PI / 6.0), "30°");
        assert_eq!(short_degrees(-PI / 2.0), "-90°");
        assert_eq!(short_degrees(1.0), "57.2958°");
        // A tiny negative angle rounds to zero without its sign.
        assert_eq!(short_degrees(-1e-9), "0°");
        assert_eq!(fixed(degrees(PI / 2.0), 4), "90.0000");
    }

    #[test]
    fn numbers_read_as_on_the_web() {
        let f = Format::default();
        assert_eq!(
            f.point(Vec2::new(487012.0, 4420000.0)),
            "Y 487012.000  X 4420000.000"
        );
        assert_eq!(f.length(12.0), "12.000 m");
        assert_eq!(f.area(96.0), "96.00 m²");
        assert_eq!(f.bearing(100.0), "100.0000 g");
        assert_eq!(f.coord(-0.0), "0.000");
        let other = Format {
            area_unit: AreaUnit::Donum,
            angle_unit: AngleUnit::Deg,
            ..f
        };
        assert_eq!(other.area(1500.0), "1.50 dönüm");
        assert_eq!(other.bearing(100.0), "90.0000°");
    }

    /// Values from `Number.prototype.toFixed` in V8.
    #[test]
    fn fixed_rounds_as_javascript_does() {
        for (v, d, js) in [
            (487012.0625, 3, "487012.063"),
            (0.125, 2, "0.13"),
            (-0.125, 2, "-0.13"),
            (2.5, 0, "3"),
            (0.5, 0, "1"),
            (9.995, 2, "9.99"),
            (99.5, 0, "100"),
            (9.9995, 3, "9.999"),
            (1.005, 2, "1.00"),
            (-0.001, 2, "-0.00"),
            (-0.0, 3, "0.000"),
            (12.0, 3, "12.000"),
            (1234.5678, 2, "1234.57"),
            (5e-324, 3, "0.000"),
        ] {
            assert_eq!(fixed(v, d), js, "({v}).toFixed({d})");
        }
        assert_eq!(fixed(f64::NAN, 2), "NaN");
        assert_eq!(fixed(f64::NEG_INFINITY, 2), "-Infinity");
    }
}
