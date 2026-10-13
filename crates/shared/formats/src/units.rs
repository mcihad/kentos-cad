//! A file's length unit against a local project's (docs/adr/0165 §2): the
//! units DXF's $INSUNITS names, as metres, and one scale put on every
//! coordinate and length of a drawing's objects and block definitions, so
//! that the drawing keeps metres whatever the file was written in.
//!
//! A scale is a ratio of whole numbers (`mul ÷ div`), each exact in a
//! float64: a millimetre is ÷1000 and an inch ×254 ÷10000, as a value typed
//! in a local project's unit is turned into metres (`Format::to_metres`,
//! the web's `toMetres`). Angles, ratios, an insert's own scale and pen
//! widths (millimetres on paper) are not lengths in the drawing and stay.

use kentos_contracts::DrawingUnit;
/// The scale and the scaling of a drawing's objects are the contracts'
/// (`kentos_contracts::units`): one rule for DXF's units and for changing a
/// project's unit (docs/adr/0256 §1).
pub use kentos_contracts::units::{
    Scale, insunits, scale_block as block, scale_bounds as bounds, scale_entity as entity,
};

/// Metres in one of a local project's drawing units.
pub fn from_unit(u: DrawingUnit) -> Scale {
    Scale::from_unit(u)
}

/// A local project's drawing unit in metres: what a writer writes.
pub fn to_unit(u: DrawingUnit) -> Scale {
    Scale::to_unit(u)
}

/// The $INSUNITS code of a local project's drawing unit.
pub fn insunits_of(u: DrawingUnit) -> i64 {
    match u {
        DrawingUnit::Mm => 4,
        DrawingUnit::Cm => 5,
        DrawingUnit::M => 6,
    }
}

/// The unit's name as a report says it: “milimetre”.
pub fn name_of(u: DrawingUnit) -> &'static str {
    match u {
        DrawingUnit::Mm => "milimetre",
        DrawingUnit::Cm => "santimetre",
        DrawingUnit::M => "metre",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ratios are the definitions: an inch is 25.4 mm, a US survey foot
    /// 1200/3937 m; a millimetre is divided, as a typed one is.
    #[test]
    fn units_are_their_definitions() {
        let metres = |code: i64, v: f64| insunits(code).map(|(s, _)| s.apply(v));
        assert_eq!(metres(4, 250.0), Some(0.25));
        assert_eq!(metres(4, 0.1), Some(0.1 / 1000.0));
        assert_eq!(metres(5, 12.5), Some(0.125));
        assert_eq!(metres(6, 12.5), Some(12.5));
        assert_eq!(metres(7, 1.5), Some(1500.0));
        assert_eq!(metres(1, 100.0), Some(2.54));
        assert_eq!(metres(2, 10.0), Some(3.048));
        assert_eq!(metres(21, 3937.0), Some(1200.0));
        assert_eq!(metres(0, 1.0), None);
        assert_eq!(metres(25, 1.0), None);
        assert_eq!(metres(-1, 1.0), None);
        for u in [DrawingUnit::Mm, DrawingUnit::Cm, DrawingUnit::M] {
            let (scale, name) = insunits(insunits_of(u)).expect("a unit");
            assert_eq!((scale, name), (from_unit(u), name_of(u)));
            assert_eq!(to_unit(u).apply(from_unit(u).apply(12.0)), 12.0);
        }
        assert!(from_unit(DrawingUnit::M).is_one() && !to_unit(DrawingUnit::Mm).is_one());
    }
}
