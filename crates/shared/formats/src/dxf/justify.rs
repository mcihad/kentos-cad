//! A text's alignment as DXF says it (docs/adr/0145 §7): a TEXT's horizontal
//! justification (72) and vertical one (73; an ATTRIB's and an ATTDEF's 74)
//! name the point of the text that stands on its alignment point (11).
//! KentOS's twelve points are DXF's four rows (baseline, bottom, middle,
//! top) by three columns; 72 = 4 (Middle) is the middle's centre. Aligned
//! (72 = 3) and fit (5) lay a text between two points and are no KentOS
//! alignment: the reader works their height or width factor out. An MTEXT's
//! attachment point (71) is its lines' alignment.

use kentos_contracts::{TextAlign, Vec2};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::entity::TextPlace;
use kentos_geometry_core::text::Font;

/// 72 = 3: the text runs between its two points at its width factor, its height made to fit.
pub const ALIGNED: i64 = 3;
/// 72 = 4: centred on the middle of its height.
pub const MIDDLE: i64 = 4;
/// 72 = 5: the text runs between its two points at its height, its width factor made to fit.
pub const FIT: i64 = 5;

/// The alignment of a text justified `h` (72) and `v` (73 or 74): none for
/// the left of the baseline, where a text always stood. A value DXF does
/// not know counts as its default, 0.
pub fn align_of(h: i64, v: i64) -> Option<TextAlign> {
    use TextAlign::*;
    if h == MIDDLE {
        return Some(MiddleCenter);
    }
    let row = [
        [None, Some(BaselineCenter), Some(BaselineRight)],
        [Some(BottomLeft), Some(BottomCenter), Some(BottomRight)],
        [Some(MiddleLeft), Some(MiddleCenter), Some(MiddleRight)],
        [Some(TopLeft), Some(TopCenter), Some(TopRight)],
    ];
    let col = match h {
        1 => 1,
        2 => 2,
        _ => 0,
    };
    let v = usize::try_from(v).ok().filter(|v| *v < 4).unwrap_or(0);
    row[v][col]
}

/// The 72 and the 73 (an attribute's 74) that write `a`.
pub fn groups_of(a: TextAlign) -> (i64, i64) {
    use TextAlign::*;
    match a {
        BaselineCenter => (1, 0),
        BaselineRight => (2, 0),
        BottomLeft => (0, 1),
        BottomCenter => (1, 1),
        BottomRight => (2, 1),
        MiddleLeft => (0, 2),
        MiddleCenter => (1, 2),
        MiddleRight => (2, 2),
        TopLeft => (0, 3),
        TopCenter => (1, 3),
        TopRight => (2, 3),
    }
}

/// Where a text `words` standing at `p` with `align` starts (group 10): the
/// left of its baseline, measured in Arimo as the reader measures a text it
/// must place. A program that justifies draws it at its alignment point
/// (11); the start is for those that do not.
pub fn start(
    p: Vec2,
    words: &str,
    height: f64,
    rotation: f64,
    align: Option<TextAlign>,
    width_factor: Option<f64>,
) -> Vec2 {
    let Some(a) = align else {
        return p;
    };
    let o = TextPlace::line(
        CoreVec2::new(p.x, p.y),
        words,
        height,
        rotation,
        crate::blocks::core_align(a),
        width_factor,
    )
    .origin(Font::from_id("arimo"));
    Vec2 { x: o.x, y: o.y }
}

/// The alignment of an MTEXT's lines from its attachment point (71): 1 to 3
/// the top's left, centre and right, 4 to 6 the middle's, 7 to 9 the
/// bottom's; out of range, the nearest.
pub fn attachment(n: i64) -> TextAlign {
    use TextAlign::*;
    match n.clamp(1, 9) {
        1 => TopLeft,
        2 => TopCenter,
        3 => TopRight,
        4 => MiddleLeft,
        5 => MiddleCenter,
        6 => MiddleRight,
        7 => BottomLeft,
        8 => BottomCenter,
        _ => BottomRight,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_alignment_reads_back_as_written() {
        for a in TextAlign::ALL {
            let (h, v) = groups_of(a);
            assert_eq!(align_of(h, v), Some(a), "{a:?}");
        }
        assert_eq!(align_of(0, 0), None);
    }

    #[test]
    fn middle_is_the_middles_centre_and_unknown_values_their_default() {
        assert_eq!(align_of(MIDDLE, 0), Some(TextAlign::MiddleCenter));
        assert_eq!(align_of(MIDDLE, 3), Some(TextAlign::MiddleCenter));
        assert_eq!(align_of(7, 0), None);
        assert_eq!(align_of(1, 9), Some(TextAlign::BaselineCenter));
        assert_eq!(align_of(-1, -1), None);
    }

    #[test]
    fn an_attachment_point_is_its_row_and_column() {
        assert_eq!(attachment(1), TextAlign::TopLeft);
        assert_eq!(attachment(5), TextAlign::MiddleCenter);
        assert_eq!(attachment(9), TextAlign::BottomRight);
        assert_eq!(attachment(0), TextAlign::TopLeft);
        assert_eq!(attachment(12), TextAlign::BottomRight);
    }
}
