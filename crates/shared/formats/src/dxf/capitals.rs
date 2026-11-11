//! A DXF text's height as other programs read it (docs/adr/8046 §3). A TrueType text's height (a TEXT's 40, an
//! ATTDEF's, a STYLE's fixed height) is its **capital letters' height** there: Trimble Tekla's DWG/DXF export turns its
//! em into "CAP" so "the font height … is the same when you open the drawing in Autodesk AutoCAD", BricsCAD's Height
//! is "the height of the uppercase text", QCAD and ezdxf draw the 'A' as tall as the height. KentOS draws a text's
//! height as its face's **em** (`text::shaping::placed`): the same 40 showed 1 / 0.7158 = 1.397 times as large in
//! AutoCAD as in KentOS.
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
/// extension (AutoCAD's `txt`, `romans`: shape files) or a style that names nothing. Arial, Helvetica: Arial's; Courier
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
