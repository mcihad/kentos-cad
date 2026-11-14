//! Ölçek: the view's scale as 1:N on a 96 dpi screen, the one rule every part reads (the owner, 14 November 2026:
//! "ölçek birçok yerde işimize yarayabilir, her yerden erişilebilir olmalı"): the status bar, the camera, the tools,
//! the labels' and the layers' visibility, the expressions. A view holds its scale as pixels per metre (px/m); its
//! denominator N is 1 / (px/m × 0.26458 mm), a 96 dpi pixel on paper.

/// Metres of paper a screen pixel is at 96 dpi (the web's `METRES_PER_PX`).
pub const METRES_PER_PX: f64 = 0.000_264_58;

/// The N of 1:N for a view of `px_per_metre` pixels a metre.
pub fn denominator(px_per_metre: f64) -> f64 {
    1.0 / (px_per_metre * METRES_PER_PX)
}

/// The pixels a metre at 1:`denominator`.
pub fn px_per_metre(denominator: f64) -> f64 {
    1.0 / (denominator * METRES_PER_PX)
}

/// How near a scale is to a range's end to count as at it, relative: a scale goes through pixels a metre and back (a
/// typed 1:2510 reads 1:2510.0000000000005), and a view set to 1:10 000 from the status bar is at a range's 1:10 000
/// end however the two were worked out.
pub const PRECISION: f64 = 1e-9;

/// Whether a view at 1:`n` is in a visibility range as QGIS reads one (its "Minimum (exclusive)" and "Maximum
/// (inclusive)"): `farthest` the most zoomed-out scale's denominator, itself not shown (1:10 000: shown from 1:9 999
/// in); `nearest` the most zoomed-in, itself shown (1:500: shown down to 1:500, not at 1:499). Either may be absent.
/// A scale within [`PRECISION`] of an end is at it.
pub fn visible_at(n: f64, farthest: Option<f64>, nearest: Option<f64>) -> bool {
    farthest.is_none_or(|f| n < f * (1.0 - PRECISION))
        && nearest.is_none_or(|m| n >= m * (1.0 - PRECISION))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_denominator_and_pixels_a_metre_are_each_others_inverse() {
        let n = denominator(px_per_metre(1000.0));
        assert!((n - 1000.0).abs() < 1e-9, "{n}");
        // 1:1000 on a 96 dpi screen: a metre is 3.78 px.
        assert!((px_per_metre(1000.0) - 3.7795).abs() < 1e-3);
    }

    #[test]
    fn the_range_reads_as_qgis_reads_it() {
        let (farthest, nearest) = (Some(10_000.0), Some(500.0));
        assert!(
            !visible_at(10_000.0, farthest, nearest),
            "the farthest is exclusive"
        );
        assert!(visible_at(9_999.0, farthest, nearest));
        assert!(
            visible_at(500.0, farthest, nearest),
            "the nearest is inclusive"
        );
        assert!(!visible_at(499.0, farthest, nearest));
        assert!(visible_at(1.0, None, None), "no range: every scale");
        assert!(visible_at(1_000_000.0, None, Some(500.0)));
        // A typed end goes through pixels a metre and back; the view at it is still at it.
        let typed = |n: f64| denominator(px_per_metre(n));
        for n in [435.0, 2510.0, 10_000.0, 1_000_000.0] {
            assert!(
                !visible_at(n, Some(typed(n)), None),
                "at the farthest 1:{n}"
            );
            assert!(visible_at(typed(n), None, Some(n)), "at the nearest 1:{n}");
            assert!(visible_at(n - 0.5, Some(typed(n)), None));
        }
    }
}
