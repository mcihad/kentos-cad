//! A DXF text's height as other programs read it (docs/adr/8046 §3). A TrueType text's height (a TEXT's 40, an
//! ATTDEF's, a STYLE's fixed height) is its **capital letters' height** there (the ADR names them, in their words).
//! KentOS draws a text's height as its face's **em** (`text::shaping::placed`): the same 40 showed 1 / 0.7158 = 1.397
//! times as large there as in KentOS.
//!
//! So a TrueType text's height goes out as its em times the capital share of the face its STYLE names, and comes back
//! divided by it. An SHX font's height is its capitals' in both (docs/adr/1125): none.

use kentos_geometry_core::text::fonts::{FontMatch, resolve};
use kentos_geometry_core::text::shaping;

/// Arial's capital H: 1466 of 2048 units in each of its four files (arial, arialbd, ariali, arialbi.ttf), what other
/// programs draw for KentOS's Standard (`arial.ttf`). Arimo, KentOS's metric twin, is 1409: as wide, its capitals
/// 4.1 % shorter.
pub const ARIAL: f64 = 1466.0 / 2048.0;
/// Courier New's capital H: 1170 of 2048 (cour.ttf); its italic 1171 (couri.ttf), its bold 1213 (courbd, courbi.ttf).
pub const COURIER_NEW: f64 = 1170.0 / 2048.0;
const COURIER_NEW_ITALIC: f64 = 1171.0 / 2048.0;
const COURIER_NEW_BOLD: f64 = 1213.0 / 2048.0;

/// A bundled face's capital share by its `DrawingFont` id (its OS/2 capital height, else its H; the core's rule).
fn own(id: &str) -> Option<f64> {
    shaping::select_font(id, 400, false).and_then(|f| shaping::cap_em(f).ok())
}

/// The share of a TrueType text's em its capitals take in the face a STYLE names by its `file` (group 3), else by
/// ACAD's `family` (1000) with its `bold` and `italic` flags (1071); none for an SHX font, a file without a TrueType
/// extension (`txt`, `romans`: shape files) or a style that names nothing. Arial, Helvetica: Arial's; Courier
/// New: its own by weight and slope; KentOS's own faces (and Liberation Sans, Arimo's design): their files'; a face
/// KentOS has not got (Times, Calibri …) is drawn in KentOS's default face, Barlow: Barlow's, so its capitals stand as
/// tall in both programs.
pub fn share(file: &str, family: Option<&str>, bold: bool, italic: bool) -> Option<f64> {
    let file = file.trim();
    let lower = file.to_lowercase();
    let leaf = lower.rsplit(['/', '\\']).next().unwrap_or(&lower);
    let truetype = [".ttf", ".otf", ".ttc"].iter().any(|e| leaf.ends_with(e));
    let named = if !file.is_empty() {
        if !truetype {
            return None;
        }
        file.to_owned()
    } else {
        family.map(str::trim).filter(|f| !f.is_empty())?.to_owned()
    };
    let compact: String = named
        .to_lowercase()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    let stem = leaf.split('.').next().unwrap_or(leaf);
    match resolve(&named) {
        (_, FontMatch::Shx) => None,
        (Some("arimo"), FontMatch::Equivalent) if compact.contains("liberation") => own("arimo"),
        (Some("arimo"), FontMatch::Equivalent) => Some(ARIAL),
        (Some("courier-prime"), FontMatch::Equivalent) => {
            let bold = bold || matches!(stem, "courbd" | "courbi") || compact.contains("bold");
            let italic = italic || stem == "couri" || compact.contains("italic");
            Some(if bold {
                COURIER_NEW_BOLD
            } else if italic {
                COURIER_NEW_ITALIC
            } else {
                COURIER_NEW
            })
        }
        (Some(id), _) => own(id),
        (None, _) => own("barlow"),
    }
}

/// A KentOS height (an em) as a DXF's: times the capital share; as it is without one.
pub fn written(height: f64, share: Option<f64>) -> f64 {
    share.map_or(height, |s| height * s)
}

/// A DXF's height as KentOS's em: over the capital share; as it is without one.
pub fn read(height: f64, share: Option<f64>) -> f64 {
    share.map_or(height, |s| height / s)
}

/// The KentOS height `height` to keep in KentOS's data: when the DXF's height divided back does not give it exactly.
pub fn kept(height: f64, share: Option<f64>) -> Option<f64> {
    (read(written(height, share), share) != height).then_some(height)
}

/// KentOS's own height back: `kept` while the file's `height` still is what KentOS wrote of it (an edit in another
/// program wins), else the file's over the share.
pub fn back(height: f64, share: Option<f64>, kept: Option<f64>) -> f64 {
    match kept {
        Some(k) if k > 0.0 && k.is_finite() && written(k, share) == height => k,
        _ => read(height, share),
    }
}

/// An MTEXT's line spacing factor (44) for KentOS's `spacing` (none: 1). Other programs' lines are 5/3 × 40 × 44
/// apart, KentOS's 5/3 × em × spacing (`text::paragraph::PITCH`): with a share, 44 is the spacing over it, within
/// DXF's 0.25 to 4; without one, KentOS's own (none for 1).
pub fn spacing_written(spacing: Option<f64>, share: Option<f64>) -> Option<f64> {
    match share {
        Some(s) => Some((spacing.unwrap_or(1.0) / s).clamp(0.25, 4.0)),
        None => spacing,
    }
}

/// KentOS's line spacing for a file's 44 (`spacing`; 1 where the file has none): times the share; KentOS's own `kept`
/// while the file still has what KentOS wrote of it; none for 1.
pub fn spacing_back(spacing: f64, share: Option<f64>, kept: Option<f64>) -> Option<f64> {
    let s = match (share, kept) {
        (Some(sh), Some(k)) if spacing_written(Some(k), Some(sh)) == Some(spacing) => k,
        (Some(sh), _) => spacing * sh,
        (None, _) => spacing,
    };
    ((s - 1.0).abs() > 1e-9).then_some(s)
}

/// The line spacing to keep in KentOS's data: when the file's 44 does not give it back exactly (a share's division,
/// DXF's range).
pub fn spacing_kept(spacing: Option<f64>, share: Option<f64>) -> Option<f64> {
    let written = spacing_written(spacing, share)?;
    (spacing_back(written, share, None) != spacing).then_some(spacing.unwrap_or(1.0))
}

/// An MTEXT's indents and tab stops (`\px…;`) are in its 40s (ezdxf: "multiples of the default text height"),
/// KentOS's in its em: a KentOS value over the share in the file, a file's value times the share in KentOS.
pub fn in_file(value: f64, share: Option<f64>) -> f64 {
    share.map_or(value, |s| value / s)
}

/// A file's indent or tab stop in KentOS's em.
pub fn from_file(value: f64, share: Option<f64>) -> f64 {
    share.map_or(value, |s| value * s)
}

/// How much lower the file's alignment point stands than KentOS's, along the text's up (docs/adr/8046 §3). Other
/// programs find the baseline a share `up` of the file's height (40: the capitals') under the alignment point
/// (0 baseline, −0.2 bottom, ½ middle, 1 top; an MTEXT's attachment the same way), KentOS a share of its em
/// (`text::TextAlign::up`, `paragraph::rise`; the lines' pitch is the same in both): with the point `up × (1 −
/// share) × em` lower the baseline stands where KentOS draws it. None without a share.
pub fn drop(up: f64, height: f64, share: Option<f64>) -> f64 {
    share.map_or(0.0, |s| up * (1.0 - s) * height)
}

/// The share of an MTEXT's attachment point (71) over its first baseline, as a text alignment's: 1 to 3 the top, 4
/// to 6 the middle, 7 to 9 the bottom (`text::TextAlign::up`).
pub fn attach_up(attach: i64) -> f64 {
    match attach {
        4..=6 => 0.5,
        7..=9 => -0.2,
        _ => 1.0,
    }
}

/// The file's alignment point for KentOS's `p` of a text turned `(sin, cos)` (its up is `(−sin, cos)`), `drop` lower.
pub fn lowered(p: (f64, f64), turn: (f64, f64), drop: f64) -> (f64, f64) {
    (p.0 + turn.0 * drop, p.1 - turn.1 * drop)
}

/// KentOS's point back from the file's: `drop` higher.
pub fn raised(at: (f64, f64), turn: (f64, f64), drop: f64) -> (f64, f64) {
    (at.0 - turn.0 * drop, at.1 + turn.1 * drop)
}

/// KentOS's own point `kept` while the file's `at` still is where KentOS put it (within a nanometre's share of its
/// size: an edit elsewhere wins), else `at` raised.
pub fn point_back(
    at: (f64, f64),
    turn: (f64, f64),
    drop: f64,
    kept: Option<(f64, f64)>,
) -> (f64, f64) {
    match kept {
        Some(k) => {
            let w = lowered(k, turn, drop);
            let near = |a: f64, b: f64| (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0);
            if near(w.0, at.0) && near(w.1, at.1) {
                k
            } else {
                raised(at, turn, drop)
            }
        }
        None => raised(at, turn, drop),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_face_has_its_capitals_share() {
        // Measured from the faces' H (docs/adr/8046 §3).
        assert_eq!(
            share("arial.ttf", None, false, false),
            Some(1466.0 / 2048.0)
        );
        assert_eq!(share("arialbd.ttf", None, false, false), Some(ARIAL));
        assert_eq!(share("", Some("Arial"), true, false), Some(ARIAL));
        assert_eq!(share("cour.ttf", None, false, false), Some(1170.0 / 2048.0));
        assert_eq!(
            share("courbd.ttf", None, false, false),
            Some(1213.0 / 2048.0)
        );
        assert_eq!(
            share("couri.ttf", None, false, false),
            Some(1171.0 / 2048.0)
        );
        assert_eq!(share("Barlow.ttf", None, false, false), Some(0.7));
        assert_eq!(
            share("Arimo.ttf", None, false, false),
            Some(1409.0 / 2048.0)
        );
        assert_eq!(
            share("LiberationSans-Regular.ttf", None, false, false),
            Some(1409.0 / 2048.0)
        );
        assert_eq!(
            share("CourierPrime.ttf", None, false, false),
            Some(1187.0 / 2048.0)
        );
        // A face KentOS draws in Barlow: Barlow's capitals.
        assert_eq!(share("times.ttf", None, false, false), Some(0.7));
        // Shape files and a style naming nothing: their height is their capitals'.
        for none in ["romans.shx", "txt", "ROMANS", "", "isocp.shx"] {
            assert_eq!(share(none, None, false, false), None, "{none}");
        }
        assert_eq!(share("", None, false, false), None);
    }

    #[test]
    fn an_aligned_text_s_point_drops_so_its_baseline_stays() {
        // A middle aligned Arial text of em 2.5 at (10, 20), unturned: the file's point 0.5 × (1 − 0.7158) × 2.5
        // lower; another program's baseline half the file's height under it is KentOS's, half the em under (10, 20).
        let s = Some(ARIAL);
        let d = drop(0.5, 2.5, s);
        let at = lowered((10.0, 20.0), (0.0, 1.0), d);
        assert!((at.1 - (20.0 - 0.5 * (1.0 - ARIAL) * 2.5)).abs() < 1e-12);
        assert!(((at.1 - 0.5 * written(2.5, s)) - (20.0 - 0.5 * 2.5)).abs() < 1e-12);
        // Baseline alignments and SHX fonts do not move.
        assert_eq!(drop(0.0, 2.5, s), 0.0);
        assert_eq!(drop(1.0, 2.5, None), 0.0);
        // Back exactly with KentOS's kept point; an edit elsewhere wins.
        let p = (452_345.123, 4_412_345.678);
        let turn = (0.5, 3f64.sqrt() / 2.0);
        let at = lowered(p, turn, drop(1.0, 0.25, s));
        assert_eq!(point_back(at, turn, drop(1.0, 0.25, s), Some(p)), p);
        let moved = (at.0 + 0.01, at.1);
        assert_ne!(point_back(moved, turn, drop(1.0, 0.25, s), Some(p)), p);
    }

    #[test]
    fn a_line_spacing_goes_out_and_comes_back() {
        let s = Some(ARIAL);
        // None (1): 1 / share in the file, none back.
        assert_eq!(spacing_written(None, s), Some(2048.0 / 1466.0));
        assert_eq!(
            spacing_back(2048.0 / 1466.0, s, spacing_kept(None, s)),
            None
        );
        for sp in [0.25, 0.5, 1.5, 2.0, 1.0 / 3.0 + 1.0, 4.0] {
            let w = spacing_written(Some(sp), s).unwrap();
            assert!((0.25..=4.0).contains(&w), "{w}");
            assert_eq!(
                spacing_back(w, s, spacing_kept(Some(sp), s)),
                Some(sp),
                "{sp}"
            );
        }
        // 4 over the share is past DXF's 4: 4 in the file, KentOS's own kept.
        assert_eq!(spacing_written(Some(4.0), s), Some(4.0));
        assert_eq!(spacing_kept(Some(4.0), s), Some(4.0));
        // A file without 44 (DXF's 1) is KentOS's share.
        assert_eq!(spacing_back(1.0, s, None), Some(ARIAL));
        assert_eq!(spacing_back(1.0, None, None), None);
    }

    #[test]
    fn a_height_goes_out_and_comes_back() {
        let s = Some(ARIAL);
        assert_eq!(written(2.5, s), 2.5 * 1466.0 / 2048.0);
        assert_eq!(written(2.5, None), 2.5);
        for h in [2.5, 0.25, 1.0 / 3.0, 0.18, 3.5e-3, 1234.5678] {
            let file = written(h, s);
            assert_eq!(back(file, s, kept(h, s)), h, "{h}");
        }
        // Another program's edit wins over KentOS's kept height.
        let k = kept(1.0 / 3.0, s).unwrap_or(1.0 / 3.0);
        assert_eq!(back(0.5, s, Some(k)), 0.5 / ARIAL);
    }
}
