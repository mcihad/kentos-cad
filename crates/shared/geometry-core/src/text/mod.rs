//! Widths of the drawing's text in its own typeface (`ProjectSettings.drawingFont`): the sum of the
//! letters' advances, from tables measured where the overlay draws them (`metrics.rs`, recorded in
//! a browser with the bundled faces; the desktop's additions in `metrics_more.rs`, from the faces'
//! files). Kerning is left out; a letter the tables do not hold counts as the face's average
//! lowercase letter.

pub mod along;
pub mod edit;
pub mod face;
// The fonts folders' families after the tables' faces (docs/adr/9313).
pub mod families;
pub mod face_info;
pub mod fonts;
pub mod font_data;
pub mod shaping;
pub mod shx;
pub mod natural;
pub mod paragraph;
#[rustfmt::skip]
mod metrics;
// The typefaces the desktop added after the web paused (Arial, Times New Roman; docs/adr/9311), after the web's seven.
#[rustfmt::skip]
mod metrics_more;

use metrics::{ADVANCES, BOLD, FIRST, FONTS, LAST};

/// How many faces the tables hold: the web's, then the desktop's.
const FACES: usize = FONTS.len() + metrics_more::FONTS.len();

/// The id of the face at `i` in `DrawingFont`'s order.
fn id_at(i: usize) -> Option<&'static str> {
    match i.checked_sub(FONTS.len()) {
        None => FONTS.get(i).copied(),
        Some(j) => metrics_more::FONTS.get(j).copied(),
    }
}

/// A drawing typeface: an index into the tables (0 is Barlow, the default), or a fonts folder's family after them
/// (docs/adr/9313, [`families`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Font(u16);

impl Font {
    /// Barlow: files and callers that name no face.
    pub const DEFAULT: Font = Font(0);
    /// Arial, which draws a family this program's fonts folders do not hold (docs/adr/9313).
    pub const ARIAL: Font = Font(FONTS.len() as u16);

    /// The face by its `DrawingFont` id (`arimo`), or the folder family by its name (`Roboto`); a name no family may
    /// have is Barlow.
    pub fn from_id(id: &str) -> Font {
        if let Some(i) = (0..FACES).find(|i| id_at(*i) == Some(id)) {
            return Font(i as u16);
        }
        families::slot(id).map_or(Font::DEFAULT, |k| Font((FACES + k) as u16))
    }

    /// Its id: a table face's (`arimo`), a folder family's name (`Roboto`).
    pub fn id(self) -> &'static str {
        let i = self.0 as usize;
        match i.checked_sub(FACES) {
            None => id_at(i).unwrap_or(FONTS[0]),
            Some(k) => families::name(k).unwrap_or(FONTS[0]),
        }
    }

    /// Its place: in the tables (`DrawingFont`'s order), then the folder families' slots.
    pub fn index(self) -> u16 {
        self.0
    }

    /// The face at `i`; none past the tables and the numbered families.
    pub fn from_index(i: usize) -> Option<Font> {
        (i < FACES + families::count()).then_some(Font(i as u16))
    }

    /// The folder family's slot; none for a table face.
    fn family(self) -> Option<usize> {
        (self.0 as usize).checked_sub(FACES)
    }

    /// Whether it is a folder family this program's fonts folders do not hold (it is drawn in Arial).
    pub fn missing(self) -> bool {
        self.family().is_some_and(|k| !families::is_installed(k))
    }

    /// The face it is drawn and laid out in: itself, or Arial for a family this program's folders do not hold.
    pub fn drawn(self) -> Font {
        if self.missing() { Font::ARIAL } else { self }
    }
}

/// The advance of `c` in thousandths of an em.
fn advance(font: Font, c: char) -> u32 {
    advance_in(row(font, false), c)
}

/// The advance of `c` in thousandths of an em, bold (a multi-line text's
/// bold letters, docs/adr/0182) or not.
pub(crate) fn advance_of(font: Font, c: char, bold: bool) -> u32 {
    advance_in(row(font, bold), c)
}

/// A face's advances, bold (600) or not (500): the web's tables, then the desktop's, then an installed folder family's
/// from its faces (a family this program has not got measures as Arial).
fn row(font: Font, bold: bool) -> &'static [u16; 352] {
    let font = font.drawn();
    if let Some((normal, heavy)) = font.family().and_then(families::rows) {
        return if bold { heavy } else { normal };
    }
    let i = (font.0 as usize).min(FACES - 1);
    match (i.checked_sub(FONTS.len()), bold) {
        (None, false) => &ADVANCES[i],
        (None, true) => &BOLD[i],
        (Some(j), false) => &metrics_more::ADVANCES[j],
        (Some(j), true) => &metrics_more::BOLD[j],
    }
}

fn advance_in(table: &[u16; 352], c: char) -> u32 {
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

/// `width_em`, its letters bold when `bold` (a text in a bold style, docs/adr/0183 §2).
pub fn width_em_in(text: &str, font: Font, bold: bool) -> f64 {
    if !bold {
        return width_em(text, font);
    }
    if text.is_empty() {
        return advance_of(font, '\u{fffd}', true) as f64 / 1000.0;
    }
    text.chars().map(|c| advance_of(font, c, true)).sum::<u32>() as f64 / 1000.0
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
        // A name no face has is a folder family this program has not got: kept, drawn in Arial (docs/adr/9313).
        let missing = Font::from_id("Comic Neue");
        assert_eq!(missing.id(), "Comic Neue");
        assert!(missing.missing() && missing.drawn() == Font::ARIAL);
        assert_eq!(Font::from_id(""), Font::DEFAULT);
        assert_eq!(Font::ARIAL.id(), "arial");
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
