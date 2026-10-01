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

    /// An area in the project's unit without the unit (the web's `area(m2, false)`).
    pub fn area_bare(&self, square_metres: f64) -> String {
        let d = self.area_decimals;
        match self.area_unit {
            AreaUnit::Donum => fixed(square_metres / 1000.0, d),
            AreaUnit::Ha => fixed(square_metres / 10_000.0, d),
            AreaUnit::M2 => fixed(square_metres, d),
        }
    }

    /// The area unit's name (the web's `areaUnitLabel`): `m²`, `dönüm` or `ha`.
    pub fn area_unit_label(&self) -> &'static str {
        match self.area_unit {
            AreaUnit::Donum => "dönüm",
            AreaUnit::Ha => "ha",
            AreaUnit::M2 => "m²",
        }
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

    /// A bearing without its unit (the web's `bearing(g, false)`), for a
    /// column whose heading names the unit ([`Format::angle_unit_label`]).
    pub fn bearing_bare(&self, grad: f64) -> String {
        match self.angle_unit {
            AngleUnit::Deg => fixed(grad / GRAD_PER_DEG, 4),
            AngleUnit::Grad => fixed(grad, 4),
        }
    }

    /// The angle unit's mark (the web's `angleUnitLabel`): `°` or `g`.
    pub fn angle_unit_label(&self) -> &'static str {
        match self.angle_unit {
            AngleUnit::Deg => "°",
            AngleUnit::Grad => "g",
        }
    }

    /// An angle in radians (an angular dimension's), in the project's angle
    /// unit: the web's `angle`, a bearing of `rad × 200 / π` grads.
    pub fn angle(&self, rad: f64) -> String {
        self.bearing((rad * 200.0) / std::f64::consts::PI)
    }

    /// An angle typed in the project's angle unit (a direction, a bearing), in radians.
    pub fn angle_from_typed(&self, typed: f64) -> f64 {
        match self.angle_unit {
            AngleUnit::Deg => (typed * std::f64::consts::PI) / 180.0,
            AngleUnit::Grad => (typed * std::f64::consts::PI) / 200.0,
        }
    }

    /// The project's angle unit as a prompt says it: `derece` or `grad`.
    pub fn angle_unit_name(&self) -> &'static str {
        match self.angle_unit {
            AngleUnit::Deg => "derece",
            AngleUnit::Grad => "grad",
        }
    }

    /// The same without its unit (the web's `angle(rad, false)`).
    pub fn angle_bare(&self, rad: f64) -> String {
        self.bearing_bare((rad * 200.0) / std::f64::consts::PI)
    }

    /// A slope in percent, two decimals (docs/adr/0147 §2; the web's `percent`).
    pub fn percent(&self, v: f64) -> String {
        fixed(v, 2)
    }

    /// A dimension's measured value as drawn (the web's `dimensionText`): its
    /// prefix (“R ”, “Ø ”, “Y=”, “X=”, “t=”, “%”), then a length or a
    /// coordinate without its unit, an angle in the project's angle unit or a
    /// percentage.
    pub fn dimension(&self, prefix: &str, unit: &str, value: f64) -> String {
        match unit {
            "angle" => format!("{prefix}{}", self.angle(value)),
            "percent" => format!("{prefix}{}", self.percent(value)),
            "coordinate" => format!("{prefix}{}", self.coord(value)),
            _ => format!("{prefix}{}", self.length_bare(value)),
        }
    }

    /// `Y 487012.000  X 4420000.000`: east first (CLAUDE.md §5).
    pub fn point(&self, p: Vec2) -> String {
        format!("Y {}  X {}", self.coord(p.x), self.coord(p.y))
    }
}

/// `v` with `d` decimals by the display rule (docs/adr/0149): seven
/// decimals first, then the digits shown, a half away from zero; no sign on
/// zero. The shared core's `display::fixed`, the web's `core/displayNumber.ts`.
pub fn fixed(v: f64, d: usize) -> String {
    kentos_geometry_core::display::fixed(v, d)
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

    /// The display rule (docs/adr/0149); every case of it is the core's
    /// `fixtures/numeric/v1/display.json`.
    #[test]
    fn fixed_writes_by_the_display_rule() {
        for (v, d, shown) in [
            (487012.0625, 3, "487012.063"),
            (0.125, 2, "0.13"),
            (-0.125, 2, "-0.13"),
            (2.5, 0, "3"),
            (0.5, 0, "1"),
            // 9.995 and 1.005 are just under a half in binary; written as typed, rounded as on paper.
            (9.995, 2, "10.00"),
            (99.5, 0, "100"),
            (9.9995, 3, "10.000"),
            (1.005, 2, "1.01"),
            // A value that rounds to zero has no sign.
            (-0.001, 2, "0.00"),
            (-0.0, 3, "0.000"),
            (12.0, 3, "12.000"),
            (1234.5678, 2, "1234.57"),
            (5e-324, 3, "0.000"),
        ] {
            assert_eq!(fixed(v, d), shown, "{v} with {d}");
        }
        assert_eq!(fixed(f64::NAN, 2), "NaN");
        assert_eq!(fixed(f64::NEG_INFINITY, 2), "-Infinity");
    }
}
