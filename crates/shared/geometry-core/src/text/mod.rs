//! Widths of the drawing's text in its own typeface (`ProjectSettings.drawingFont`): the sum of the
//! letters' advances, from tables measured where the overlay draws them (`metrics.rs`, recorded in
//! Chrome with the bundled faces). Kerning is left out; a letter the tables do not hold counts as the
//! face's average lowercase letter.

pub mod edit;
#[rustfmt::skip]
mod metrics;

use metrics::{ADVANCES, FIRST, FONTS, LAST};

/// A drawing typeface, as an index into the tables (0 is Barlow, the default).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Font(u8);

impl Font {
    /// Barlow: files and callers that name no face.
    pub const DEFAULT: Font = Font(0);

    /// The face by its `DrawingFont` id; an unknown id is Barlow.
    pub fn from_id(id: &str) -> Font {
        FONTS
            .iter()
            .position(|f| *f == id)
            .map_or(Font::DEFAULT, |i| Font(i as u8))
    }

    pub fn id(self) -> &'static str {
        FONTS[self.0 as usize]
    }
}

/// The advance of `c` in thousandths of an em.
fn advance(font: Font, c: char) -> u32 {
    let table = &ADVANCES[font.0 as usize];
    let code = c as u32;
    if (FIRST..=LAST).contains(&code) {
        let w = table[(code - FIRST) as usize] as u32;
        // Control codes measure nothing: they count as a letter, as every letter did before.
        if w > 0 {
            return w;
        }
    }
    let lower = &table[('a' as u32 - FIRST) as usize..=('z' as u32 - FIRST) as usize];
    lower.iter().map(|&w| w as u32).sum::<u32>() / 26
}

/// Width of `text` in em. An empty text counts as one average letter, as the boxes always did, so it
/// can still be picked.
pub fn width_em(text: &str, font: Font) -> f64 {
    if text.is_empty() {
        return advance(font, '\u{fffd}') as f64 / 1000.0;
    }
    text.chars().map(|c| advance(font, c)).sum::<u32>() as f64 / 1000.0
}

/// Which point of a text its `p` is (docs/adr/0145 §1, the contract's
/// `TextAlign`): horizontally its left, centre or right; vertically on its
/// baseline, at its bottom (0.2 of its height under the baseline), its middle
/// (half its height over it) or its top (its height over it). The left of the
/// baseline, where a text always stood, is no value but its absence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    BaselineCenter,
    BaselineRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    MiddleLeft,
    MiddleCenter,
    MiddleRight,
    TopLeft,
    TopCenter,
    TopRight,
}

impl TextAlign {
    /// Every alignment, in the contract's order.
    pub const ALL: [TextAlign; 11] = [
        TextAlign::BaselineCenter,
        TextAlign::BaselineRight,
        TextAlign::BottomLeft,
        TextAlign::BottomCenter,
        TextAlign::BottomRight,
        TextAlign::MiddleLeft,
        TextAlign::MiddleCenter,
        TextAlign::MiddleRight,
        TextAlign::TopLeft,
        TextAlign::TopCenter,
        TextAlign::TopRight,
    ];

    /// Its name in the contract (`middleCenter`).
    pub fn name(self) -> &'static str {
        match self {
            TextAlign::BaselineCenter => "baselineCenter",
            TextAlign::BaselineRight => "baselineRight",
            TextAlign::BottomLeft => "bottomLeft",
            TextAlign::BottomCenter => "bottomCenter",
            TextAlign::BottomRight => "bottomRight",
            TextAlign::MiddleLeft => "middleLeft",
            TextAlign::MiddleCenter => "middleCenter",
            TextAlign::MiddleRight => "middleRight",
            TextAlign::TopLeft => "topLeft",
            TextAlign::TopCenter => "topCenter",
            TextAlign::TopRight => "topRight",
        }
    }

    /// The alignment named `name`; none for any other name.
    pub fn from_name(name: &str) -> Option<TextAlign> {
        TextAlign::ALL.into_iter().find(|a| a.name() == name)
    }

    /// Where `p` is along the text, as a share of its width: 0 left, ½ centre, 1 right.
    pub fn along(self) -> f64 {
        match self {
            TextAlign::BottomLeft | TextAlign::MiddleLeft | TextAlign::TopLeft => 0.0,
            TextAlign::BaselineCenter
            | TextAlign::BottomCenter
            | TextAlign::MiddleCenter
            | TextAlign::TopCenter => 0.5,
            TextAlign::BaselineRight
            | TextAlign::BottomRight
            | TextAlign::MiddleRight
            | TextAlign::TopRight => 1.0,
        }
    }

    /// Where `p` is over the baseline, as a share of the text's height: 0 the
    /// baseline, −0.2 the bottom, ½ the middle, 1 the top.
    pub fn up(self) -> f64 {
        match self {
            TextAlign::BaselineCenter | TextAlign::BaselineRight => 0.0,
            TextAlign::BottomLeft | TextAlign::BottomCenter | TextAlign::BottomRight => -0.2,
            TextAlign::MiddleLeft | TextAlign::MiddleCenter | TextAlign::MiddleRight => 0.5,
            TextAlign::TopLeft | TextAlign::TopCenter | TextAlign::TopRight => 1.0,
        }
    }
}

impl crate::api::json::FromJson for TextAlign {
    fn from_json(v: &crate::api::json::Json) -> Result<TextAlign, String> {
        match v {
            crate::api::json::Json::Str(s) => {
                TextAlign::from_name(s).ok_or_else(|| format!("“{s}” yazı hizası bilinmiyor"))
            }
            _ => Err("yazı hizası metin olmalı".into()),
        }
    }
}

impl crate::api::json::ToJson for TextAlign {
    fn write_json(&self, out: &mut String) {
        crate::api::json::write_str(out, self.name());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_by_id_and_default() {
        assert_eq!(Font::from_id("barlow"), Font::DEFAULT);
        assert_eq!(Font::from_id("plex-mono").id(), "plex-mono");
        assert_eq!(Font::from_id("comic-sans"), Font::DEFAULT);
    }

    #[test]
    fn monospace_faces_are_monospace() {
        for id in ["courier-prime", "plex-mono"] {
            let f = Font::from_id(id);
            let w = width_em("i", f);
            assert!((w - 0.6).abs() < 0.002, "{id}: {w}");
            assert_eq!(width_em("iiii", f), width_em("WWWW", f));
            assert_eq!(width_em("ğşıİÇ", f), 5.0 * w);
        }
    }

    #[test]
    fn proportional_faces_measure_letters() {
        let f = Font::DEFAULT;
        assert!(width_em("W", f) > 2.0 * width_em("i", f));
        // Turkish letters are in the tables, not the fallback.
        assert!(width_em("ı", f) < width_em("n", f));
        assert!(width_em("İ", f) < width_em("M", f));
        // Outside the tables: the average letter.
        assert_eq!(width_em("→", f), width_em("\u{fffd}", f));
        assert_eq!(width_em("", f), width_em("\u{fffd}", f));
        assert!(width_em("12", f) > width_em("1", f));
    }
}
