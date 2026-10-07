//! A multi-line text as DXF (docs/adr/0182 §5): an MTEXT at its point, its
//! attachment its alignment (71: the top's, middle's or bottom's left,
//! centre or right; a baseline alignment, which MTEXT has not, the top's,
//! the point moved there so the text stays where it is), as high as it
//! (40), its box's width (41, 0 without one), left to right (72), its line
//! spacing (44) when it has one, its direction (11) and its mask as the
//! drawing's background (90 3). Its words in MTEXT's notation: `\P` a line
//! break, each run a group (`{…}`) with its formats' switches. The exact
//! turn is KentOS's data when the direction written does not give it back.

use kentos_contracts::{TextAlign, TextEntity, TextRun, TextScript};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::entity::TextPlace;
use kentos_geometry_core::text::Font;

use super::super::super::aci;
use super::super::super::xdata::Meta;
use super::leader::turn_read_back;
use super::{Writer, mtext_chunks};
use crate::blocks::{core_align, core_runs};
use crate::geom::v;
use crate::math::sin_cos_deg;

const WHAT: &str = "Çok satırlı yazı";

/// MTEXT's attachment point (71) of an alignment: 1 to 3 the top's left,
/// centre and right, 4 to 6 the middle's, 7 to 9 the bottom's; none for a
/// baseline alignment (MTEXT has none).
fn attachment(a: Option<TextAlign>) -> Option<i64> {
    use TextAlign::*;
    Some(match a? {
        TopLeft => 1,
        TopCenter => 2,
        TopRight => 3,
        MiddleLeft => 4,
        MiddleCenter => 5,
        MiddleRight => 6,
        BottomLeft => 7,
        BottomCenter => 8,
        BottomRight => 9,
        BaselineCenter | BaselineRight => return None,
    })
}

/// One letter in MTEXT's notation: a line break `\P`, a backslash, brace or
/// caret escaped, `%` as `%%%` in a text that has `%%` (TEXT's control codes
/// read it back), any other control character a space.
fn letter(c: char, percent: bool, out: &mut String) {
    match c {
        '\n' => out.push_str("\\P"),
        c if c.is_control() => out.push(' '),
        '\\' => out.push_str("\\\\"),
        '{' => out.push_str("\\{"),
        '}' => out.push_str("\\}"),
        '^' => out.push_str("^ "),
        '%' if percent => out.push_str("%%%"),
        c => out.push(c),
    }
}

/// A multi-line text's words and formats in MTEXT's notation (docs/adr/0182
/// §5): a width factor other than 1 a `\W` first; each run a group: bold and
/// italic a font switch (Arial, the text style's face), underline `\L`, a
/// colour `\C` (an ACI index, `ink` 7) or `\c` (a true colour, blue in the
/// high byte as AutoCAD reads it), raised letters stacked over nothing
/// (`\S…^ ;`) and lowered ones under it (`\S^ …;`). Also whether a raised or
/// lowered run had a letter a stack cannot hold (`^ / # ; \ { }`, a line
/// break): those are written on the line.
pub(crate) fn paragraph_value(
    text: &str,
    runs: &[TextRun],
    width_factor: Option<f64>,
) -> (String, bool) {
    let percent = text.contains("%%");
    let letters: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 16 * runs.len() + 8);
    if let Some(w) = width_factor.filter(|w| *w != 1.0) {
        out.push_str(&format!("\\W{w};"));
    }
    let mut unstacked = false;
    let mut at = 0;
    for r in runs {
        let (start, end) = (
            (r.start as usize).min(letters.len()),
            (r.end as usize).min(letters.len()),
        );
        for &c in &letters[at.min(start)..start] {
            letter(c, percent, &mut out);
        }
        at = end.max(at);
        if start >= end {
            continue;
        }
        out.push('{');
        if r.bold || r.italic {
            out.push_str(&format!(
                "\\fArial|b{}|i{};",
                u8::from(r.bold),
                u8::from(r.italic)
            ));
        }
        if r.underline {
            out.push_str("\\L");
        }
        if let Some(c) = &r.color {
            let (dxf, _) = aci::from_app(c);
            match dxf.rgb {
                Some(rgb) => {
                    let bgr = ((rgb & 0xFF) << 16) | (rgb & 0xFF00) | ((rgb >> 16) & 0xFF);
                    out.push_str(&format!("\\c{bgr};"));
                }
                None => out.push_str(&format!("\\C{};", dxf.aci)),
            }
        }
        let words = &letters[start..end];
        let stackable = !words
            .iter()
            .any(|c| matches!(c, '^' | '/' | '#' | ';' | '\\' | '{' | '}' | '%') || c.is_control());
        match r.script {
            Some(s) if stackable => {
                let w: String = words.iter().collect();
                match s {
                    TextScript::Super => out.push_str(&format!("\\S{w}^ ;")),
                    TextScript::Sub => out.push_str(&format!("\\S^ {w};")),
                }
            }
            script => {
                unstacked |= script.is_some();
                for &c in words {
                    letter(c, percent, &mut out);
                }
            }
        }
        out.push('}');
    }
    for &c in &letters[at.min(letters.len())..] {
        letter(c, percent, &mut out);
    }
    (out, unstacked)
}

impl Writer<'_> {
    /// A multi-line text (docs/adr/0182 §5) as an MTEXT; `meta` its KentOS data.
    pub(super) fn paragraph(&mut self, t: &TextEntity, mut meta: Meta) -> bool {
        if t.text.trim().is_empty() {
            self.report.skip(WHAT, "boş yazı yazılmadı", 0);
            return false;
        }
        if !(t.height > 0.0) {
            self.report
                .skip(WHAT, "yazı yüksekliği sıfır ya da negatif; yazılmadı", 0);
            return false;
        }
        let (attach, p) = match attachment(t.align) {
            Some(n) => (n, t.p),
            None => {
                // The top of the same side, the point moved there by the text's own box (measured in Arimo,
                // the file's Arial).
                let (n, top) = match t.align {
                    Some(TextAlign::BaselineCenter) => (2, TextAlign::TopCenter),
                    Some(TextAlign::BaselineRight) => (3, TextAlign::TopRight),
                    _ => (1, TextAlign::TopLeft),
                };
                let runs = core_runs(&t.paragraph.runs).unwrap_or_default();
                let place = TextPlace {
                    p: CoreVec2::new(t.p.x, t.p.y),
                    text: &t.text,
                    height: t.height,
                    rotation: t.rotation,
                    align: t.align.and_then(core_align),
                    width_factor: t.width_factor,
                    box_width: t.paragraph.box_width,
                    line_spacing: t.paragraph.line_spacing,
                    runs: &runs,
                    // Measured as the file draws it (Arimo for Arial); bold and slant its own.
                    font: None,
                    bold: t.face.font.is_some() && t.face.bold,
                    lean: crate::blocks::core_face(&t.face).lean(),
                    path: None,
                };
                let q = place.realigned(core_align(top), Font::from_id("arimo"));
                self.report.note(
                    WHAT,
                    "taban hizalı yazı MTEXT'e üst hizasıyla yazıldı (MTEXT'in taban hizası yok); yeri aynı",
                    0,
                );
                (n, v(q.x, q.y))
            }
        };
        let (value, unstacked) = paragraph_value(&t.text, &t.paragraph.runs, t.width_factor);
        if unstacked {
            self.report.note(
                WHAT,
                "üst ya da alt simgesinde MTEXT'in kesir sözdiziminin taşıyamadığı bir harf (^ / # ; \\ { } %) olduğundan satırda yazıldı",
                0,
            );
        }
        // The mask is the MTEXT's own background (90), not KentOS's data.
        meta.mask = false;
        meta.note_turn = (turn_read_back(t.rotation) != t.rotation).then_some(t.rotation);
        self.begin("MTEXT", &t.base);
        self.out.str(100, "AcDbMText");
        self.out.xyz(10, p);
        self.out.real(40, t.height);
        self.out.real(41, t.paragraph.box_width.unwrap_or(0.0));
        self.out.int(71, attach);
        self.out.int(72, 1);
        mtext_chunks(self.out, &value);
        // Its style (docs/adr/0183 §7); its face in its KENTOS data.
        self.out.str(7, self.styles.text(&t.face));
        if !t.face.is_plain() {
            meta.face = serde_json::to_string(&t.face).ok();
        }
        let (s, c) = sin_cos_deg(t.rotation);
        self.out.xyz(11, v(c, s));
        if let Some(spacing) = t.paragraph.line_spacing {
            // At least this spacing (AutoCAD's style 1), the factor itself.
            self.out.int(73, 1);
            self.out.real(44, spacing);
        }
        if t.mask {
            // The drawing's background behind it, a tenth of its height round (the app's margin).
            self.out.int(90, 3);
            self.out.int(63, 256);
            self.out.real(45, 1.1);
            self.out.int(441, 0);
        }
        self.grow(p);
        self.end(meta);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dxf::strings::mtext_content;
    use crate::dxf::xdata::caret_decode;

    fn run(start: u32, end: u32) -> TextRun {
        TextRun {
            start,
            end,
            ..TextRun::default()
        }
    }

    #[test]
    fn a_paragraph_reads_back_as_written() {
        let text = "Parsel 101\nAlan: 450 m2 {a}\\b^c 100%% H2O";
        let runs = vec![
            TextRun {
                bold: true,
                ..run(0, 6)
            },
            TextRun {
                italic: true,
                underline: true,
                color: Some("#E5484D".into()),
                ..run(7, 10)
            },
            TextRun {
                script: Some(TextScript::Super),
                ..run(22, 23)
            },
            TextRun {
                color: Some("ink".into()),
                ..run(24, 27)
            },
            TextRun {
                script: Some(TextScript::Sub),
                ..run(39, 40)
            },
        ];
        let (value, unstacked) = paragraph_value(text, &runs, Some(0.8));
        assert!(!unstacked);
        let back = mtext_content(&caret_decode(&value));
        assert_eq!(back.text, text, "{value}");
        assert_eq!(back.runs, runs, "{value}");
        assert_eq!(back.width_factor, Some(0.8));
        assert!(back.dropped.is_empty(), "{:?}", back.dropped);
    }

    #[test]
    fn a_script_a_stack_cannot_hold_is_written_on_the_line() {
        let runs = vec![TextRun {
            script: Some(TextScript::Super),
            ..run(1, 4)
        }];
        let (value, unstacked) = paragraph_value("a1/2", &runs, None);
        assert!(unstacked);
        assert_eq!(value, "a{1/2}");
    }
}
