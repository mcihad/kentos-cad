//! Widths of the drawing's text in its own typeface (`ProjectSettings.drawingFont`): the sum of the
//! letters' advances, from tables measured where the overlay draws them (`metrics.rs`, recorded in
//! Chrome with the bundled faces). Kerning is left out; a letter the tables do not hold counts as the
//! face's average lowercase letter.

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
