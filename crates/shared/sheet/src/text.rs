//! Text on the sheet (design §6): widths from the drawing typefaces' own
//! metrics (`data/font-metrics.json`, written by `scripts/fonts/sheet_metrics.py`),
//! so lines break at the same word on every platform. A line is laid out
//! as CSS lays it out: its height is the size times the line height, the
//! glyphs' ascent and descent centred in it. Kerning is left out (phase 1):
//! for Latin and Turkish text the difference does not show.
//!
//! The table is also the faces' coverage, and no character is drawn as a
//! box (design §6, “Eksik karakter”): one the face lacks is drawn by the
//! first of the drawing's typefaces that has it ([`glyph`]); one no face has
//! is a “?”; the preflight names both (`glyph_missing`).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::kinds::TextFit;
use crate::style::{HAlign, TextStyle, VAlign};
use crate::units::{RectUm, Um};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceMetrics {
    pub font: String,
    pub weight: u16,
    pub italic: bool,
    pub units_per_em: u32,
    pub ascent: i32,
    pub descent: i32,
    pub line_gap: i32,
    pub cap_height: i32,
    pub x_height: i32,
    /// The average lowercase letter (what a CID font's default width is; every character is
    /// measured as the glyph that draws it, [`glyph`]).
    pub fallback: i32,
    pub advances: Vec<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Table {
    #[serde(default)]
    chars: String,
    #[serde(default)]
    faces: Vec<FaceMetrics>,
    #[serde(skip)]
    index: Vec<(char, usize)>,
}

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| {
        // The crate's own file, read by a test; a broken one leaves no faces (every text falls back), never a panic.
        let mut t: Table =
            serde_json::from_str(include_str!("../data/font-metrics.json")).unwrap_or_default();
        t.index = t.chars.chars().enumerate().map(|(i, c)| (c, i)).collect();
        t.index.sort_unstable();
        t
    })
}

/// The typefaces the table has (`DRAWING_FONTS` ids), in its order.
pub fn fonts() -> Vec<&'static str> {
    let mut out: Vec<&str> = Vec::new();
    for f in &table().faces {
        if !out.contains(&f.font.as_str()) {
            out.push(&f.font);
        }
    }
    out
}

/// Whether the table measures `c` (a character it lacks counts as the face's average lowercase letter).
pub fn measures(c: char) -> bool {
    table().index.binary_search_by(|(k, _)| k.cmp(&c)).is_ok()
}

pub fn has_font(font: &str) -> bool {
    table().faces.iter().any(|f| f.font == font)
}

/// The weight CSS picks for `want` among `have` (CSS Fonts 4, §5.2 step 4).
fn css_weight(want: u16, have: &[u16]) -> Option<u16> {
    if have.contains(&want) {
        return Some(want);
    }
    let up = |lo: u16, hi: u16| have.iter().copied().filter(|w| *w > lo && *w <= hi).min();
    let down = |hi: u16| have.iter().copied().filter(|w| *w < hi).max();
    if (400..=500).contains(&want) {
        up(want, 500)
            .or_else(|| down(want))
            .or_else(|| up(500, u16::MAX))
    } else if want < 400 {
        down(want).or_else(|| up(want, u16::MAX))
    } else {
        up(want, u16::MAX).or_else(|| down(want))
    }
}

/// The face a style is drawn with: its typeface (Barlow when the table lacks it), the weight CSS would pick, italic when there is one.
pub fn face(style: &TextStyle) -> Option<&'static FaceMetrics> {
    let family = if has_font(&style.font) {
        style.font.as_str()
    } else {
        "barlow"
    };
    face_in(family, style.weight, style.italic)
}

/// The face of `family` CSS picks for `weight`, italic when the family has one.
fn face_in(family: &str, weight: u16, italic: bool) -> Option<&'static FaceMetrics> {
    let t = table();
    let pick = |italic: bool| -> Option<&'static FaceMetrics> {
        let ws: Vec<u16> = t
            .faces
            .iter()
            .filter(|f| f.font == family && f.italic == italic)
            .map(|f| f.weight)
            .collect();
        let w = css_weight(weight, &ws)?;
        t.faces
            .iter()
            .find(|f| f.font == family && f.italic == italic && f.weight == w)
    };
    if italic {
        pick(true).or_else(|| pick(false))
    } else {
        pick(false)
    }
}

/// A typeface's name as the interface writes it (`DRAWING_FONTS`).
pub fn family_name(font: &str) -> &str {
    match font {
        "barlow" => "Barlow",
        "arimo" => "Arimo",
        "overpass" => "Overpass",
        "quicksand" => "Quicksand",
        "architects-daughter" => "Architects Daughter",
        "courier-prime" => "Courier Prime",
        "plex-mono" => "IBM Plex Mono",
        other => other,
    }
}

/// How a text written in a face draws one of its characters.
#[derive(Clone, Copy, Debug)]
pub enum Glyph {
    /// The face's own glyph (a control character is drawn as a space).
    Own(char),
    /// Another drawing face's glyph: the face lacks the character.
    Other(char, &'static FaceMetrics),
    /// No drawing face has the character: the face's “?” is drawn.
    Missing(char),
}

/// How a text in face `f` draws `c` (design §6, “Eksik karakter”): with `f` when the table has
/// the character for it; else with the first of the table's typefaces (Barlow, Arimo, Overpass,
/// Quicksand, Architects Daughter, Courier Prime, IBM Plex Mono) other than `f`'s whose face
/// at `f`'s weight and slant has it; else as a “?”. The table's characters are the coverage:
/// one it does not measure is in no face.
pub fn glyph(f: &FaceMetrics, c: char) -> Glyph {
    if c.is_control() {
        return Glyph::Own(' ');
    }
    if f.advance_of(c).is_some() {
        return Glyph::Own(c);
    }
    for family in fonts() {
        if family == f.font {
            continue;
        }
        if let Some(g) = face_in(family, f.weight, f.italic)
            && g.advance_of(c).is_some()
        {
            return Glyph::Other(c, g);
        }
    }
    Glyph::Missing(c)
}

/// A piece of a line that one face draws.
#[derive(Clone, Debug)]
pub struct Run {
    pub text: String,
    pub face: &'static FaceMetrics,
}

/// A line of text in face `f` as its faces draw it ([`glyph`]): the pieces in order, a
/// character no face has as a “?” of `f`, a control character as a space.
pub fn runs(f: &'static FaceMetrics, text: &str) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for c in text.chars() {
        let (d, g) = match glyph(f, c) {
            Glyph::Own(d) => (d, f),
            Glyph::Other(d, g) => (d, g),
            Glyph::Missing(_) => ('?', f),
        };
        match out.last_mut() {
            Some(r) if std::ptr::eq(r.face, g) => r.text.push(d),
            _ => out.push(Run {
                text: d.to_string(),
                face: g,
            }),
        }
    }
    out
}

/// A piece of a text with the face that draws it (`textRuns`): how a host writes a line so that
/// every letter comes from the drawing's faces, as the PDF writes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TextRun {
    pub text: String,
    /// A `DRAWING_FONTS` id.
    pub font: String,
    pub weight: u16,
    pub italic: bool,
}

/// [`runs`] of a text in `font` (an unknown one is Barlow), as data; none for an empty table.
pub fn text_runs(font: &str, weight: u16, italic: bool, text: &str) -> Vec<TextRun> {
    let family = if has_font(font) { font } else { "barlow" };
    let Some(f) = face_in(family, weight, italic) else {
        return Vec::new();
    };
    runs(f, text)
        .into_iter()
        .map(|r| TextRun {
            text: r.text,
            font: r.face.font.clone(),
            weight: r.face.weight,
            italic: r.face.italic,
        })
        .collect()
}

impl FaceMetrics {
    /// A character's advance in font units, as the glyph that draws it ([`glyph`]): another
    /// face's in this face's units, a “?” for a character no face has.
    pub fn advance(&self, c: char) -> i64 {
        let own = |c: char| self.advance_of(c).unwrap_or(i64::from(self.fallback));
        match glyph(self, c) {
            Glyph::Own(d) => own(d),
            Glyph::Other(d, g) => {
                let a = g.advance_of(d).unwrap_or(i64::from(g.fallback)).max(0);
                let (num, den) = (
                    a * i64::from(self.units_per_em.max(1)),
                    i64::from(g.units_per_em.max(1)),
                );
                (num + den / 2) / den
            }
            Glyph::Missing(_) => own('?'),
        }
    }

    /// “Barlow 400”, “Courier Prime 400 italik”: the face in a message.
    pub fn label(&self) -> String {
        format!(
            "{} {}{}",
            family_name(&self.font),
            self.weight,
            if self.italic { " italik" } else { "" }
        )
    }

    /// A character's advance in font units when the table has it for this face.
    pub fn advance_of(&self, c: char) -> Option<i64> {
        let t = table();
        let k = t.index.binary_search_by(|(k, _)| k.cmp(&c)).ok()?;
        self.advances
            .get(t.index[k].1)
            .filter(|a| **a >= 0)
            .map(|a| i64::from(*a))
    }

    /// Font units scaled to micrometres at `size`, a half away from zero.
    pub fn scale(&self, units: i64, size: Um) -> i64 {
        let em = i64::from(self.units_per_em.max(1));
        let n = units * i64::from(size);
        if n >= 0 {
            (n + em / 2) / em
        } else {
            (n - em / 2) / em
        }
    }

    /// The width of `text` at `size`, in micrometres.
    pub fn width(&self, text: &str, size: Um) -> i64 {
        let units: i64 = text.chars().map(|c| self.advance(c)).sum();
        self.scale(units, size)
    }
}

/// The width of `text` in `style`, in micrometres.
pub fn text_width(text: &str, style: &TextStyle) -> i64 {
    face(style).map_or(0, |f| f.width(text, style.size))
}

/// The baseline's distance below the top of a line box of height `advance` (CSS's half leading).
pub fn baseline_in_line(f: &FaceMetrics, size: Um, advance: i64) -> i64 {
    let ascent = f.scale(i64::from(f.ascent), size);
    let descent = f.scale(i64::from(f.descent), size);
    let content = ascent + descent;
    (advance - content).div_euclid(2) + ascent
}

/// A laid-out line: its text, where it starts and its baseline, and its width.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub x: i64,
    pub baseline: i64,
    pub width: i64,
}

/// A block of laid-out text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Block {
    pub lines: Vec<Line>,
    /// The letter size used (smaller than the style's after shrinking).
    pub size: Um,
    /// Something did not fit the box (lines cut off at the bottom or wider than it).
    pub overflow: bool,
    /// The block's height.
    pub height: i64,
    /// The widest line.
    pub width: i64,
}

/// Lines of `text` broken to `max` micrometres (none: only at line feeds); a word longer than a line stands on its own and overflows.
pub fn break_lines(f: &FaceMetrics, text: &str, size: Um, max: Option<i64>) -> Vec<(String, i64)> {
    let mut out = Vec::new();
    for para in text.split('\n') {
        let para = para.strip_suffix('\r').unwrap_or(para);
        let Some(max) = max else {
            out.push((para.to_owned(), f.width(para.trim_end(), size)));
            continue;
        };
        // Words with what follows them (spaces, or a hyphen kept on the line).
        let mut tokens: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut chars = para.chars().peekable();
        while let Some(c) = chars.next() {
            cur.push(c);
            let next_is_space = chars.peek().is_some_and(|n| *n == ' ');
            if (c == ' ' && !next_is_space) || (c == '-' && chars.peek().is_some_and(|n| *n != ' '))
            {
                tokens.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            tokens.push(cur);
        }
        if tokens.is_empty() {
            out.push((String::new(), 0));
            continue;
        }
        let mut line = String::new();
        for tok in tokens {
            if !line.is_empty() && f.width(format!("{line}{tok}").trim_end(), size) > max {
                let t = line.trim_end().to_owned();
                let w = f.width(&t, size);
                out.push((t, w));
                line.clear();
            }
            if line.is_empty() && f.width(tok.trim_end(), size) > max {
                // A word wider than the line stands alone and overflows (the layout says so); it is never broken between letters.
                let t = tok.trim_end().to_owned();
                let w = f.width(&t, size);
                out.push((t, w));
            } else {
                line.push_str(&tok);
            }
        }
        let t = line.trim_end().to_owned();
        let w = f.width(&t, size);
        out.push((t, w));
    }
    out
}

/// How a block of text is laid out in a box.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub align: HAlign,
    pub valign: VAlign,
    /// Percent of the size.
    pub line_height: u16,
    pub wrap: bool,
    pub fit: TextFit,
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            align: HAlign::Left,
            valign: VAlign::Top,
            line_height: 120,
            wrap: true,
            fit: TextFit::None,
        }
    }
}

fn line_advance(size: Um, line_height: u16) -> i64 {
    let n = i64::from(size) * i64::from(line_height);
    (n + 50) / 100
}

fn place(f: &FaceMetrics, text: &str, size: Um, b: &RectUm, l: &Layout) -> Block {
    let max = l.wrap.then_some(i64::from(b.width));
    let lines = break_lines(f, text, size, max);
    let adv = line_advance(size, l.line_height);
    let height = adv * lines.len() as i64;
    let top = match l.valign {
        VAlign::Top => i64::from(b.top),
        VAlign::Middle => i64::from(b.top) + (i64::from(b.height) - height).div_euclid(2),
        VAlign::Bottom => b.bottom() - height,
    };
    let base = baseline_in_line(f, size, adv);
    let mut width = 0;
    let mut overflow = height > i64::from(b.height);
    let laid = lines
        .into_iter()
        .enumerate()
        .map(|(i, (t, w))| {
            width = width.max(w);
            if w > i64::from(b.width) {
                overflow = true;
            }
            let x = match l.align {
                HAlign::Left => i64::from(b.left),
                HAlign::Center => i64::from(b.left) + (i64::from(b.width) - w).div_euclid(2),
                HAlign::Right => b.right() - w,
            };
            Line {
                text: t,
                x,
                baseline: top + i as i64 * adv + base,
                width: w,
            }
        })
        .collect();
    Block {
        lines: laid,
        size,
        overflow,
        height,
        width,
    }
}

/// The smallest letter size the core shrinks a text it writes itself to (the north diagram's
/// angles, a north arrow's note): 1.5 mm, about 4 pt, the least that still reads on paper. A
/// text that does not fit at this size is said (`text_overflow`), never made smaller.
pub const LEGIBLE_MIN: Um = 1_500;

/// `text` in `style` laid out in box `b`; with `ShrinkToFit` the size goes down (to half, in 0.01 mm steps) until it fits.
pub fn layout(text: &str, style: &TextStyle, b: &RectUm, l: &Layout) -> Block {
    layout_down_to(text, style, b, l, (style.size / 2).max(200))
}

/// [`layout`] with the smallest size `ShrinkToFit` may go to (a text the core writes itself
/// stops at [`LEGIBLE_MIN`]); a block that does not fit even then is that size's, `overflow` set.
pub fn layout_down_to(text: &str, style: &TextStyle, b: &RectUm, l: &Layout, floor: Um) -> Block {
    let Some(f) = face(style) else {
        return Block::default();
    };
    let first = place(f, text, style.size, b, l);
    if !first.overflow || l.fit != TextFit::ShrinkToFit {
        return first;
    }
    // The largest size (in steps of 10 µm) that fits, down to the floor.
    let floor = floor.clamp(200, style.size.max(200));
    let (mut lo, mut hi) = (floor / 10, (style.size + 9) / 10);
    if place(f, text, lo * 10, b, l).overflow {
        return place(f, text, lo * 10, b, l);
    }
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if place(f, text, mid * 10, b, l).overflow {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    place(f, text, lo * 10, b, l)
}

/// The height of one line of `style` (no wrapping) at `line_height` percent.
pub fn line_height_of(style: &TextStyle, line_height: u16) -> i64 {
    line_advance(style.size, line_height)
}

/// The cap height of `style`, in micrometres.
pub fn cap_height(style: &TextStyle) -> i64 {
    face(style).map_or(i64::from(style.size) * 7 / 10, |f| {
        f.scale(i64::from(f.cap_height), style.size)
    })
}

/// The ascent of `style`'s face, in micrometres.
pub fn ascent(style: &TextStyle) -> i64 {
    face(style).map_or(i64::from(style.size), |f| {
        f.scale(i64::from(f.ascent), style.size)
    })
}

/// The descent of `style`'s face, in micrometres.
pub fn descent(style: &TextStyle) -> i64 {
    face(style).map_or(i64::from(style.size) / 4, |f| {
        f.scale(i64::from(f.descent), style.size)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_has_every_drawing_face() {
        assert_eq!(
            fonts(),
            [
                "barlow",
                "arimo",
                "overpass",
                "quicksand",
                "architects-daughter",
                "courier-prime",
                "plex-mono"
            ]
        );
        assert_eq!(table().faces.len(), 22);
        assert_eq!(
            table().chars.chars().count(),
            table().faces[0].advances.len()
        );
    }

    #[test]
    fn weights_are_matched_as_css_does() {
        assert_eq!(css_weight(700, &[400, 500, 600]), Some(600));
        assert_eq!(css_weight(450, &[400, 600]), Some(400));
        assert_eq!(css_weight(500, &[400, 600]), Some(400));
        assert_eq!(css_weight(300, &[400, 600]), Some(400));
        assert_eq!(css_weight(600, &[400, 700]), Some(700));
        let bold = TextStyle::new(2_500, 700);
        assert_eq!(face(&bold).map(|f| f.weight), Some(600));
        let mut it = TextStyle::new(2_500, 400);
        it.italic = true;
        assert!(face(&it).is_some_and(|f| f.italic));
        it.font = "nope".into();
        assert_eq!(face(&it).map(|f| f.font.as_str()), Some("barlow"));
    }

    #[test]
    fn widths_follow_the_advances() {
        // Barlow 400: “H” 645, “ı” 221 thousandths of an em (the font's own advances).
        let s = TextStyle::new(10_000, 400);
        let f = face(&s).unwrap();
        assert_eq!(f.units_per_em, 1000);
        assert_eq!(f.width("H", 10_000), f.advance('H') * 10);
        assert_eq!(text_width("", &s), 0);
        assert!(text_width("Şğİı", &s) > 0);
        // A character no face has is drawn, and measured, as a “?”; a control one as a space.
        assert_eq!(f.advance('⟨'), f.advance('?'));
        assert_eq!(f.advance('\t'), f.advance(' '));
    }

    /// No silent box (design §6): a character the face lacks is another drawing face's, the
    /// first in the table's order at the same weight and slant; one no face has is a “?”.
    #[test]
    fn a_character_the_face_lacks_is_another_face_s_or_a_question_mark() {
        let barlow = face(&TextStyle::new(2_500, 400)).unwrap();
        // Barlow has no arrows up or down; Arimo (the next typeface) has them.
        let Glyph::Other('↑', g) = glyph(barlow, '↑') else {
            panic!("{:?}", glyph(barlow, '↑'));
        };
        assert_eq!((g.font.as_str(), g.weight, g.italic), ("arimo", 400, false));
        // Arimo's advance in Barlow's units (1000 an em), a half away from zero.
        let (a, em) = (g.advance_of('↑').unwrap(), i64::from(g.units_per_em));
        assert_ne!(em, 1000, "the faces' ems differ");
        assert_eq!(barlow.advance('↑'), (a * 1000 + em / 2) / em);
        // No drawing face has “≤”: a “?”.
        assert!(matches!(glyph(barlow, '≤'), Glyph::Missing('≤')));
        let mut bold = TextStyle::new(2_500, 600);
        bold.font = "courier-prime".into();
        let courier = face(&bold).unwrap();
        // Courier Prime has no lira sign: Barlow's, at the weight CSS picks for Courier's 700.
        let Glyph::Other('₺', g) = glyph(courier, '₺') else {
            panic!("{:?}", glyph(courier, '₺'));
        };
        assert_eq!((g.font.as_str(), g.weight), ("barlow", 600));
        // The pieces of a line: its face's, another's, a “?”, its face's again.
        let r = runs(barlow, "Ok ↑ ≤ 5");
        let pieces: Vec<(&str, &str)> = r
            .iter()
            .map(|r| (r.text.as_str(), r.face.font.as_str()))
            .collect();
        assert_eq!(
            pieces,
            [("Ok ", "barlow"), ("↑", "arimo"), (" ? 5", "barlow")]
        );
        assert_eq!(barlow.label(), "Barlow 400");
        assert_eq!(
            text_runs("nope", 400, false, "a↑"),
            [
                TextRun {
                    text: "a".into(),
                    font: "barlow".into(),
                    weight: 400,
                    italic: false
                },
                TextRun {
                    text: "↑".into(),
                    font: "arimo".into(),
                    weight: 400,
                    italic: false
                },
            ]
        );
        // The missing-value mark's angle quotes are in every face (design §7).
        for f in fonts() {
            let m = face_in(f, 400, false).unwrap();
            assert!(
                matches!(glyph(m, '‹'), Glyph::Own('‹'))
                    && matches!(glyph(m, '›'), Glyph::Own('›')),
                "{f}"
            );
        }
    }

    #[test]
    fn lines_break_at_spaces_and_fit_the_box() {
        let s = TextStyle::new(2_500, 400);
        let f = face(&s).unwrap();
        // 28.1 mm in one line at 2.5 mm Barlow: two lines in 20 mm.
        let lines = break_lines(f, "Kadastro müdürlüğü onayı", 2_500, Some(20_000));
        assert!(lines.len() >= 2, "{lines:?}");
        assert!(lines.iter().all(|(_, w)| *w <= 20_000));
        let one = break_lines(f, "a\nb", 2_500, None);
        assert_eq!(one.len(), 2);
        // A word longer than the line stands alone and overflows; it is not broken.
        let long = break_lines(f, "a Çokuzunbirsözcükburadabölünmez b", 2_500, Some(10_000));
        assert_eq!(
            long.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(),
            ["a", "Çokuzunbirsözcükburadabölünmez", "b"]
        );
        assert!(long[1].1 > 10_000);
    }

    #[test]
    fn shrink_to_fit_makes_the_text_fit() {
        let s = TextStyle::new(5_000, 600);
        let b = RectUm::new(0, 0, 40_000, 7_000);
        let l = Layout {
            wrap: false,
            fit: TextFit::ShrinkToFit,
            ..Layout::default()
        };
        let block = layout("İFRAZ VE TEVHİT PAFTASI", &s, &b, &l);
        assert!(!block.overflow);
        assert!(block.size < 5_000);
        assert!(block.width <= 40_000);
        let plain = layout(
            "İFRAZ VE TEVHİT PAFTASI",
            &s,
            &b,
            &Layout {
                wrap: false,
                ..Layout::default()
            },
        );
        assert!(plain.overflow);
    }

    /// A text the core writes itself wraps at spaces first, gets smaller only as far as it must,
    /// and stops at the legible size: below that it is said, not shrunk further.
    #[test]
    fn a_written_text_wraps_then_shrinks_to_the_legible_size_and_no_further() {
        let s = TextStyle::new(1_800, 400);
        let l = Layout {
            align: HAlign::Center,
            wrap: true,
            fit: TextFit::ShrinkToFit,
            ..Layout::default()
        };
        let line = "CK–MK manyetik sapma 6°19' D (WMM2025, 2026-10)";
        // 34 mm wide, room for two lines: wrapped at a space, the size kept.
        let b = layout_down_to(line, &s, &RectUm::new(0, 0, 34_000, 5_000), &l, LEGIBLE_MIN);
        assert!(
            !b.overflow && b.size == 1_800 && b.lines.len() == 2,
            "{b:?}"
        );
        assert!(b.lines.iter().all(|x| x.width <= 34_000));
        // Wider, room for one line only: one line, smaller, not below 1.5 mm.
        let b = layout_down_to(line, &s, &RectUm::new(0, 0, 40_000, 2_000), &l, LEGIBLE_MIN);
        assert!(!b.overflow && b.lines.len() == 1, "{b:?}");
        assert!(b.size < 1_800 && b.size >= LEGIBLE_MIN, "{b:?}");
        // No room at all: the legible size, and said.
        let b = layout_down_to(line, &s, &RectUm::new(0, 0, 9_000, 2_000), &l, LEGIBLE_MIN);
        assert!(b.overflow && b.size == LEGIBLE_MIN, "{b:?}");
    }
}
