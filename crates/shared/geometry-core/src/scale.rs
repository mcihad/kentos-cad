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

/// Whether a view at 1:`n` is in a visibility range as QGIS reads one (its "Minimum (exclusive)" and "Maximum
/// (inclusive)"): `farthest` the most zoomed-out scale's denominator, itself not shown (1:10 000: shown from 1:9 999
/// in); `nearest` the most zoomed-in, itself shown (1:500: shown down to 1:500, not at 1:499). Either may be absent.
pub fn visible_at(n: f64, farthest: Option<f64>, nearest: Option<f64>) -> bool {
    farthest.is_none_or(|f| n < f) && nearest.is_none_or(|m| n >= m)
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
    }
}
