//! Kısa bir Markdown notunun okunur görünümü: bir eklentinin değişiklik notu (kurulum incelemesi, kurulu eklentinin
//! ayrıntısı, katalog girdisi).
//!
//! Okunan: başlıklar (`##`, `###`; en üstteki `# ` dosya başlığı atlanır), maddeler (`- `, `* `; girintili devam
//! satırları maddeye katılır, alt alta ayrı satır olarak kalmaz), paragraflar (boş satıra dek birleşir) ve satır içi
//! **kalın** ve `kod`. Kalanı (bağlantı, tablo, kod bloğu) düz metindir: bir not hiçbir zaman kaybolmaz. Önce satırlar
//! tek tek yazılıyordu: maddenin devam satırları ayrı satır, `**` ve ters tırnaklar olduğu gibi görünüyordu.
//!
//! ```text
//!  0.4.0                                 ← başlık
//!  • kacis-uzakligi (yeni): konut …      ← madde, kalın ve kod parçalarıyla, sarılarak
//! ```

use iced::advanced::text::Span;
use iced::widget::{Column, rich_text};
use iced::{Element, Fill};

use crate::label;
use crate::style;
use crate::theme::typography;

/// A piece of a line: its words and how they are written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Piece {
    pub(crate) text: String,
    pub(crate) bold: bool,
    pub(crate) code: bool,
}

/// A block of the notes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Note {
    Heading(String),
    Item(Vec<Piece>),
    Text(Vec<Piece>),
}

/// The notes' blocks: headings, items with their continuation lines, paragraphs.
pub(crate) fn read(source: &str) -> Vec<Note> {
    /// The block gathered so far, an item's or a paragraph's, written out.
    fn close(open: &mut Option<(bool, String)>, out: &mut Vec<Note>) {
        if let Some((item, words)) = open.take() {
            let pieces = pieces(&words);
            out.push(if item {
                Note::Item(pieces)
            } else {
                Note::Text(pieces)
            });
        }
    }
    let mut out = Vec::new();
    // The block being gathered: an item's or a paragraph's words so far.
    let mut open: Option<(bool, String)> = None;
    for raw in source.lines() {
        let line = raw.trim();
        if line.is_empty() {
            close(&mut open, &mut out);
            continue;
        }
        let hashes = line.chars().take_while(|c| *c == '#').count();
        if hashes > 0 && line[hashes..].starts_with(' ') {
            close(&mut open, &mut out);
            if hashes > 1 {
                out.push(Note::Heading(line[hashes..].trim().to_owned()));
            }
            continue;
        }
        if let Some(words) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            close(&mut open, &mut out);
            open = Some((true, words.to_owned()));
            continue;
        }
        match &mut open {
            // A continuation: an item's indented line, or a paragraph's next one.
            Some((item, words)) if !*item || raw.starts_with([' ', '\t']) => {
                words.push(' ');
                words.push_str(line);
            }
            _ => {
                close(&mut open, &mut out);
                open = Some((false, line.to_owned()));
            }
        }
    }
    close(&mut open, &mut out);
    out
}

/// A line's pieces: `**bold**` and `` `code` ``; an unclosed marker stays as written.
fn pieces(line: &str) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    let mut plain = String::new();
    let mut rest = line;
    while !rest.is_empty() {
        let marker = if rest.starts_with("**") {
            Some(("**", true))
        } else if rest.starts_with('`') {
            Some(("`", false))
        } else {
            None
        };
        if let Some((mark, bold)) = marker
            && let Some(end) = rest[mark.len()..].find(mark)
        {
            let inner = &rest[mark.len()..mark.len() + end];
            if !plain.is_empty() {
                out.push(Piece {
                    text: std::mem::take(&mut plain),
                    bold: false,
                    code: false,
                });
            }
            // Bold around code (**`kacis`**) stays bold: the code's marks dropped inside it.
            out.push(Piece {
                text: if bold {
                    inner.replace('`', "")
                } else {
                    inner.to_owned()
                },
                bold,
                code: !bold,
            });
            rest = &rest[mark.len() + end + mark.len()..];
            continue;
        }
        let next = rest.chars().next().map_or(1, char::len_utf8);
        plain.push_str(&rest[..next]);
        rest = &rest[next..];
    }
    if !plain.is_empty() {
        out.push(Piece {
            text: plain,
            bold: false,
            code: false,
        });
    }
    out
}

/// A short Markdown note, read: headings as body text, items with a bullet and paragraphs in the caption's size and
/// colour, bold and code inline.
pub fn notes<'a, Message: 'a>(source: &str) -> Element<'a, Message> {
    let mut column = Column::new().spacing(4).width(Fill);
    for note in read(source) {
        column = column.push(match note {
            Note::Heading(words) => Element::from(label::body(words)),
            Note::Item(mut pieces) => {
                pieces.insert(
                    0,
                    Piece {
                        text: "• ".to_owned(),
                        bold: false,
                        code: false,
                    },
                );
                rich(pieces)
            }
            Note::Text(pieces) => rich(pieces),
        });
    }
    column.into()
}

fn rich<'a, Message: 'a>(pieces: Vec<Piece>) -> Element<'a, Message> {
    let spans: Vec<Span<'a, (), iced::Font>> = pieces
        .into_iter()
        .map(|p| {
            let span = Span::new(p.text);
            if p.bold {
                span.font(typography::ui_strong())
            } else if p.code {
                span.font(typography::mono())
            } else {
                span
            }
        })
        .collect();
    rich_text(spans)
        .font(typography::ui())
        .size(typography::caption())
        .style(style::text::faint)
        .width(Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(text: &str) -> Piece {
        Piece {
            text: text.into(),
            bold: false,
            code: false,
        }
    }

    /// A changelog as packages write it: the file's title left out, a version a heading, an item's indented lines one
    /// item, bold and code inline (bold around code stays bold), a paragraph's lines one paragraph.
    #[test]
    fn a_changelog_is_read_as_headings_items_and_paragraphs() {
        let text = "# Değişiklikler\n\n## 0.4.0\n\n- **`kacis-uzakligi`** (yeni): konut olmayan\n  bölümlerde \
                    `occupancy`.\n- İkinci madde.\n\nBir paragraf\nsürer.\n";
        let read = read(text);
        assert_eq!(
            read,
            [
                Note::Heading("0.4.0".into()),
                Note::Item(vec![
                    Piece {
                        text: "kacis-uzakligi".into(),
                        bold: true,
                        code: false
                    },
                    plain(" (yeni): konut olmayan bölümlerde "),
                    Piece {
                        text: "occupancy".into(),
                        bold: false,
                        code: true
                    },
                    plain("."),
                ]),
                Note::Item(vec![plain("İkinci madde.")]),
                Note::Text(vec![plain("Bir paragraf sürer.")]),
            ]
        );
        // An unclosed marker stays as written.
        assert_eq!(pieces("2 ** 3"), [plain("2 ** 3")]);
    }
}
