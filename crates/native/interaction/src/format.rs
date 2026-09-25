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

/// `v.toFixed(d)` for display: a negative zero is written without its sign,
/// as JavaScript writes it.
fn fixed(v: f64, d: usize) -> String {
    let v = if v == 0.0 { 0.0 } else { v };
    format!("{v:.d$}")
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
