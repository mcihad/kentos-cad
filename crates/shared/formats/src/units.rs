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
    Scale, scale_block as block, scale_bounds as bounds, scale_entity as entity,
};

/// Metres in one of a local project's drawing units.
pub fn from_unit(u: DrawingUnit) -> Scale {
    Scale::from_unit(u)
}

/// A local project's drawing unit in metres: what a writer writes.
pub fn to_unit(u: DrawingUnit) -> Scale {
    Scale::to_unit(u)
}

/// The unit an $INSUNITS code names, in metres, and its name; none for a
/// drawing without units (0) and the codes AutoCAD does not define.
pub fn insunits(code: i64) -> Option<(Scale, &'static str)> {
    let s = |mul: f64, div: f64| Scale { mul, div };
    Some(match code {
        1 => (s(254.0, 10_000.0), "inç"),
        2 => (s(3048.0, 10_000.0), "fit"),
        3 => (s(1_609_344.0, 1000.0), "mil"),
        4 => (s(1.0, 1000.0), "milimetre"),
        5 => (s(1.0, 100.0), "santimetre"),
        6 => (Scale::ONE, "metre"),
        7 => (s(1000.0, 1.0), "kilometre"),
        8 => (s(254.0, 1e10), "mikroinç"),
        9 => (s(254.0, 1e7), "binde bir inç"),
        10 => (s(9144.0, 10_000.0), "yarda"),
        11 => (s(1.0, 1e10), "angstrom"),
        12 => (s(1.0, 1e9), "nanometre"),
        13 => (s(1.0, 1e6), "mikrometre"),
        14 => (s(1.0, 10.0), "desimetre"),
        15 => (s(10.0, 1.0), "dekametre"),
        16 => (s(100.0, 1.0), "hektometre"),
        17 => (s(1e9, 1.0), "gigametre"),
        18 => (s(149_597_870_700.0, 1.0), "astronomik birim"),
        19 => (s(9_460_730_472_580_800.0, 1.0), "ışık yılı"),
        // 648 000 / π astronomical units, as the float64 nearest to it.
        20 => (s(3.085_677_581_491_367e16, 1.0), "parsek"),
        21 => (s(1200.0, 3937.0), "ABD ölçme fiti"),
        22 => (s(100.0, 3937.0), "ABD ölçme inci"),
        23 => (s(3600.0, 3937.0), "ABD ölçme yardası"),
        24 => (s(6_336_000.0, 3937.0), "ABD ölçme mili"),
        _ => return None,
    })
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
