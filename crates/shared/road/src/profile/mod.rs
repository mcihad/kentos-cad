//! A road's longitudinal profile (docs/adr/0902, plan/road.md Y4): the terrain along an alignment, and its vertical
//! alignment. The algorithm's id is `kentos.profile/1`.

pub mod checks;
pub mod compare;
pub mod crossings;
pub mod propose;
pub mod sight;
pub mod lengths;
pub mod terrain;
pub mod trace;
pub mod vertical;

pub use sight::{RESOLUTION as SIGHT_RESOLUTION, Seen, Until};
pub use terrain::{ProfilePoint, TOLERANCE, terrain};
pub use vertical::{CurveReport, Piece, Pvi, VCurve, Vertical, VerticalError};

/// The algorithm's id, written with what it computes.
pub const ALGORITHM: &str = "kentos.profile/1";

/// A grade written as KGM's 2019 technical specification for road projects asks (docs/adr/0978): "Düşey hat eğim
/// hesaplarında eğim yüzde cinsinden ifade edilecektir. Eğimler virgülden sonra üç rakam alınacaktır. (+%5.241,
/// -%3.202)": the sign before the percent sign, always written, three decimals; a grade that rounds to zero has no
/// sign (%0.000). `grade` is rise over run (docs/adr/0977). The point is the drawing's; a window writes a comma.
pub fn grade_text(grade: f64) -> String {
    let thousandths = (grade * 100_000.0).round();
    let p = (thousandths / 1000.0).abs();
    if thousandths > 0.0 {
        format!("+%{p:.3}")
    } else if thousandths < 0.0 {
        format!("-%{p:.3}")
    } else {
        "%0.000".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::grade_text;

    /// KGM's own examples, the zero, and the halves rounding away from zero.
    #[test]
    fn grades_are_written_as_kgm_asks() {
        assert_eq!(grade_text(0.05241), "+%5.241");
        assert_eq!(grade_text(-0.03202), "-%3.202");
        assert_eq!(grade_text(0.0), "%0.000");
        assert_eq!(grade_text(-0.000_004), "%0.000", "−0.0004 % rounds to no grade, not to −%0.000");
        assert_eq!(grade_text(0.000_005), "+%0.001");
        assert_eq!(grade_text(-0.002_941_176_470_588_235_3), "-%0.294");
        assert_eq!(grade_text(0.08), "+%8.000");
    }
}
