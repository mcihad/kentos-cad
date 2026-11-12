//! Dimension preflight for newly authored parameter expressions (ADR 0840).
//! Evaluation and old files retain their existing numeric meaning; this never converts a value.

use super::{LiteralUnit, ParamExpr, ParamUnits, eval_param_expr};
use crate::ParameterKind;

#[derive(Clone, Copy, Debug)]
enum Dimension {
    /// A bare numeric expression can take the destination's project unit, or act as a multiplier.
    Implicit,
    Quantity {
        length: f64,
        angle: f64,
    },
}

impl Dimension {
    fn of(kind: ParameterKind) -> Self {
        let (length, angle) = match kind {
            ParameterKind::Length => (1.0, 0.0),
            ParameterKind::Angle => (0.0, 1.0),
            ParameterKind::Number => (0.0, 0.0),
        };
        Self::Quantity { length, angle }
    }

    fn powers(self) -> (f64, f64) {
        match self {
            Self::Implicit => (0.0, 0.0),
            Self::Quantity { length, angle } => (length, angle),
        }
    }

    fn same(self, other: Self) -> bool {
        let (a, b) = self.powers();
        let (c, d) = other.powers();
        // Only exponent arithmetic is compared here, never geometric residuals or stored values.
        (a - c).abs() < 1e-12 && (b - d).abs() < 1e-12
    }

    fn words(self) -> String {
        match self {
            Self::Implicit => "birimsiz yazılmış sayı".into(),
            _ if self.same(Self::of(ParameterKind::Length)) => "uzunluk".into(),
            _ if self.same(Self::of(ParameterKind::Angle)) => "açı".into(),
            _ if self.same(Self::of(ParameterKind::Number)) => "birimsiz sayı".into(),
            _ => {
                let (length, angle) = self.powers();
                format!("uzunluk^{length} × açı^{angle}")
            }
        }
    }

    fn common(self, other: Self, operation: &str) -> Result<Self, String> {
        match (self, other) {
            (Self::Implicit, d) | (d, Self::Implicit) => Ok(d),
            (a, b) if a.same(b) => Ok(a),
            (a, b) => Err(format!(
                "“{operation}” için {} ile {} birleştirilemez. Uzunlukları m/cm/mm/ft/in, açıları deg/grad/rad ile yazın.",
                a.words(),
                b.words()
            )),
        }
    }

    fn product(self, other: Self, sign: f64) -> Self {
        if matches!((self, other), (Self::Implicit, Self::Implicit)) {
            return Self::Implicit;
        }
        let (a, b) = self.powers();
        let (c, d) = other.powers();
        Self::Quantity {
            length: a + sign * c,
            angle: b + sign * d,
        }
    }

    fn power(self, exponent: f64) -> Result<Self, String> {
        if matches!(self, Self::Implicit) {
            return Ok(self);
        }
        let (length, angle) = self.powers();
        let result = Self::Quantity {
            length: length * exponent,
            angle: angle * exponent,
        };
        let (l, a) = result.powers();
        if l.is_finite() && a.is_finite() {
            Ok(result)
        } else {
            Err("üsün birim boyutu sonlu değil".into())
        }
    }
}

fn dimension(
    expr: &ParamExpr,
    units: ParamUnits,
    value: &dyn Fn(&str) -> Option<(ParameterKind, f64)>,
) -> Result<Dimension, String> {
    use Dimension::Implicit;
    let scalar = Dimension::of(ParameterKind::Number);
    let angle = Dimension::of(ParameterKind::Angle);
    let walk = |e: &ParamExpr| dimension(e, units, value);
    Ok(match expr {
        ParamExpr::Number(_, None) => Implicit,
        ParamExpr::Number(_, Some(unit)) => Dimension::of(match unit {
            LiteralUnit::Degree | LiteralUnit::Grad | LiteralUnit::Radian => ParameterKind::Angle,
            _ => ParameterKind::Length,
        }),
        // Historical pi means a half-turn in the project angle unit, not the unitless mathematical constant.
        ParamExpr::Pi => angle,
        ParamExpr::Name(n) => Dimension::of(
            value(n)
                .ok_or_else(|| format!("“{n}” diye bir parametre yok"))?
                .0,
        ),
        ParamExpr::Neg(a) => walk(a)?,
        ParamExpr::Add(a, b) => walk(a)?.common(walk(b)?, "+")?,
        ParamExpr::Sub(a, b) => walk(a)?.common(walk(b)?, "−")?,
        ParamExpr::Mul(a, b) => walk(a)?.product(walk(b)?, 1.0),
        ParamExpr::Div(a, b) => walk(a)?.product(walk(b)?, -1.0),
        ParamExpr::Pow(a, b) => {
            let exponent = walk(b)?;
            if !exponent.same(scalar) {
                return Err(format!(
                    "“^” üssü birimsiz olmalı; {} verildi.",
                    exponent.words()
                ));
            }
            let exponent = eval_param_expr(b, units, &|n| value(n).map(|(_, v)| v))?;
            walk(a)?.power(exponent)?
        }
        ParamExpr::Call(f, args) => {
            let x = walk(&args[0])?;
            match *f {
                "sin" | "cos" | "tan" => {
                    if !x.same(angle) && !x.same(scalar) {
                        return Err(format!(
                            "“{f}” açı alır; {} verildi. deg/grad/rad kullanın.",
                            x.words()
                        ));
                    }
                    // Preserve sin(30)'s established use in a bare project-unit expression.
                    if matches!(x, Implicit) {
                        Implicit
                    } else {
                        scalar
                    }
                }
                "asin" | "acos" | "atan" => {
                    if !x.same(scalar) {
                        return Err(format!("“{f}” birimsiz sayı alır; {} verildi.", x.words()));
                    }
                    angle
                }
                "sqrt" => x.power(0.5)?,
                "abs" | "round" | "floor" | "ceil" => x,
                "min" | "max" => x.common(walk(&args[1])?, f)?,
                other => return Err(format!("“{other}” bilinen bir işlev değil")),
            }
        }
    })
}

/// Checks the physical dimension of an authored expression without changing its existing numeric evaluation.
/// Bare literals use project units at the destination and in addition; named Number parameters are explicitly
/// unitless. Intermediate products, ratios, powers and square roots carry length/angle exponents.
/// Old stored expressions must be audited separately rather than silently reinterpreted on open (ADR 0840).
pub fn check_param_expr_dimension(
    expr: &ParamExpr,
    expected: ParameterKind,
    units: ParamUnits,
    value: &dyn Fn(&str) -> Option<(ParameterKind, f64)>,
) -> Result<(), String> {
    let found = dimension(expr, units, value)?;
    let wanted = Dimension::of(expected);
    if matches!(found, Dimension::Implicit) || found.same(wanted) {
        return Ok(());
    }
    Err(format!(
        "Sonuç {}; {} bekleniyor. Uzunlukları m/cm/mm/ft/in, açıları deg/grad/rad ile yazın. Birimsiz oran için aynı tür iki ölçüyü bölün.",
        found.words(),
        wanted.words()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_param_expr;

    fn check(text: &str, kind: ParameterKind) -> Result<(), String> {
        let units = ParamUnits {
            metres_per_unit: 0.001,
            radians_per_unit: std::f64::consts::PI / 200.0,
        };
        check_param_expr_dimension(
            &parse_param_expr(text).unwrap(),
            kind,
            units,
            &|n| match n {
                "width" => Some((ParameterKind::Length, 4000.0)),
                "height" => Some((ParameterKind::Length, 3000.0)),
                "turn" => Some((ParameterKind::Angle, 50.0)),
                "count" => Some((ParameterKind::Number, 2.0)),
                _ => None,
            },
        )
    }

    #[test]
    fn lengths_ratios_products_and_roots_carry_their_physical_dimension() {
        use ParameterKind::*;
        for text in [
            "width + 25",
            "width + 1in",
            "width / count",
            "width * cos(turn)",
            "sqrt(width^2 + height^2)",
            "sqrt(width * height)",
            "(width ^ (1/3))^3",
            "max(width, 100mm)",
            "abs(width - height)",
            "round(width)",
        ] {
            assert_eq!(check(text, Length), Ok(()), "{text}");
        }
        for text in ["width / height", "sin(turn)", "count^2", "min(count, 2)"] {
            assert_eq!(check(text, Number), Ok(()), "{text}");
        }
        for text in [
            "width * height",
            "width / height",
            "sqrt(width)",
            "turn",
            "count",
            "sin(turn)",
        ] {
            assert!(check(text, Length).is_err(), "{text}");
        }
        assert!(
            check("width + count", Length)
                .unwrap_err()
                .contains("birimsiz sayı")
        );
    }

    #[test]
    fn incompatible_addition_function_arguments_and_unitful_exponents_are_explained() {
        use ParameterKind::*;
        for text in [
            "2m + 30deg",
            "width - turn",
            "min(width, turn)",
            "sin(width)",
            "atan(width)",
            "width^turn",
            "width^(2m)",
        ] {
            assert!(check(text, Length).is_err(), "{text}");
        }
        assert!(
            check("sin(width)", Number)
                .unwrap_err()
                .contains("açı alır")
        );
        assert!(
            check("width^turn", Length)
                .unwrap_err()
                .contains("üssü birimsiz")
        );
        assert!(
            check("2m + 30deg", Length)
                .unwrap_err()
                .contains("uzunluk ile açı")
        );
    }

    #[test]
    fn project_unit_bare_values_and_the_legacy_half_turn_are_not_reinterpreted() {
        use ParameterKind::*;
        for kind in [Length, Angle, Number] {
            for text in ["2", "2 + 3", "sqrt(9)", "sin(30)"] {
                assert_eq!(check(text, kind), Ok(()), "{text}, {kind:?}");
            }
        }
        for text in ["pi / 2", "turn + 30deg", "acos(-1)", "atan(count)", "1rad"] {
            assert_eq!(check(text, Angle), Ok(()), "{text}");
        }
        assert!(check("pi", Number).is_err());
        assert_eq!(check("pi / 1rad", Number), Ok(()));
        assert_eq!(check("turn / 1rad", Number), Ok(()));
        assert_eq!(check("width / 1mm", Number), Ok(()));
    }
}
