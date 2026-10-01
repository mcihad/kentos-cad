//! A small native Markdown presentation layer. Supports paragraphs, headings,
//! emphasis, inline code, links, lists, quotes, tables and fenced code. It does
//! not fetch links, images or execute HTML. Unclosed fences work while streaming.

use std::rc::Rc;

use iced::advanced::text::{LineHeight, Span, Wrapping};
use iced::widget::{Column, button, container, rich_text, row, scrollable, space, text};
use iced::{Border, Element, Fill, Theme};

use crate::icon::{Icon, icon};
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::python::{CodeView, font as code_font};

use super::paragraph::StreamingParagraph;

/// Native response content. Unsupported Markdown is shown as text, so streaming
/// or malformed input never disappears. The host handles copy and link actions.
pub struct AiContent<'a, Message> {
    source: &'a str,
    streaming: bool,
    link: Option<Rc<dyn Fn(String) -> Message + 'a>>,
    copy: Option<Rc<dyn Fn(String) -> Message + 'a>>,
}

impl<'a, Message: Clone + 'a> AiContent<'a, Message> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            streaming: false,
            link: None,
            copy: None,
        }
    }
    pub fn streaming(mut self, streaming: bool) -> Self {
        self.streaming = streaming;
        self
    }
    pub fn on_link(mut self, callback: impl Fn(String) -> Message + 'a) -> Self {
        self.link = Some(Rc::new(callback));
        self
    }
    pub fn on_copy(mut self, callback: impl Fn(String) -> Message + 'a) -> Self {
        self.copy = Some(Rc::new(callback));
        self
    }

    fn view(self) -> Element<'a, Message> {
        let blocks = parse(self.source);
        let last = blocks.len().saturating_sub(1);
        let mut content = Column::new().spacing(10).width(Fill);
        for (index, block) in blocks.into_iter().enumerate() {
            match block {
                Block::Text(words) => {
                    content = content.push(StreamingParagraph::new(
                        words,
                        self.streaming && index == last,
                        self.link.clone(),
                    ))
                }
                Block::Heading(level, words) => {
                    content = content.push(
                        text(words)
                            .font(typography::ui_strong())
                            .size(typography::body() + if level == 1 { 6.0 } else { 3.0 }),
                    )
                }
                Block::Item(marker, words) => {
                    content = content.push(
                        row![
                            text(marker)
                                .size(typography::body() + 2.0)
                                .width(20)
                                .style(style::text::muted),
                            rich_paragraph(words, self.link.clone(), false)
                        ]
                        .spacing(8)
                        .align_y(iced::Top),
                    )
                }
                Block::Quote(words) => {
                    content = content.push(
                        container(rich_paragraph(words, self.link.clone(), false))
                            .padding([8, 12])
                            .width(Fill)
                            .style(|theme: &Theme| {
                                let t = Tokens::of(theme);
                                container::Style {
                                    background: Some(t.accent.scale_alpha(0.045).into()),
                                    border: Border {
                                        color: t.accent.scale_alpha(0.3),
                                        width: 1.0,
                                        radius: crate::theme::shape::sm().into(),
                                    },
                                    ..container::Style::default()
                                }
                            }),
                    )
                }
                Block::Code(language, code) => {
                    let mut bar = row![
                        text(if language.is_empty() { "Kod" } else { language })
                            .font(code_font())
                            .size(typography::caption())
                            .style(style::text::muted),
                        space::horizontal()
                    ]
                    .align_y(iced::Center);
                    if let Some(copy) = &self.copy {
                        bar = bar.push(
                            button(
                                row![
                                    icon(Icon::Copy).size(12.0),
                                    text("Kopyala").size(typography::caption())
                                ]
                                .spacing(5),
                            )
                            .on_press(copy(code.clone()))
                            .padding([3, 7])
                            .style(style::button::flat),
                        );
                    }
                    let lines = code.split('\n').count();
                    let body: Element<'a, Message> = if matches!(language, "python" | "py") {
                        CodeView::owned(code, typography::body() + 1.0).into()
                    } else {
                        text(code)
                            .font(code_font())
                            .size(typography::body() + 1.0)
                            .line_height(LineHeight::Relative(1.65))
                            .wrapping(Wrapping::None)
                            .into()
                    };
                    content = content.push(
                        container(iced::widget::column![
                            container(bar).padding([6, 10]).width(Fill),
                            crate::widget::horizontal_divider(),
                            container(scrollable(body).direction(
                                scrollable::Direction::Horizontal(
                                    scrollable::Scrollbar::new().width(5).scroller_width(5)
                                )
                            ))
                            .padding([10, 12])
                            .height(lines as f32 * (typography::body() + 1.0) * 1.65 + 25.0)
                            .width(Fill),
                        ])
                        .width(Fill)
                        .style(|theme: &Theme| {
                            let t = Tokens::of(theme);
                            container::Style {
                                background: Some(t.field.into()),
                                border: Border {
                                    color: t.border,
                                    width: 1.0,
                                    radius: crate::theme::shape::md().into(),
                                },
                                ..container::Style::default()
                            }
                        }),
                    );
                }
                Block::Table(rows) => {
                    let mut table = Column::new().spacing(0).width(Fill);
                    let row_count = rows.len();
                    for (index, cells) in rows.into_iter().enumerate() {
                        let mut line = iced::widget::Row::new().spacing(12);
                        for cell in cells {
                            line = line.push(
                                text(cell)
                                    .font(if index == 0 {
                                        typography::ui_strong()
                                    } else {
                                        typography::ui()
                                    })
                                    .size(typography::body() + 1.0)
                                    .width(Fill),
                            );
                        }
                        table = table.push(container(line).padding([7, 10]).width(Fill).style(
                            move |theme: &Theme| container::Style {
                                background:
                                    (index % 2 == 0).then(|| Tokens::of(theme).surface_alt.into()),
                                border: Border {
                                    radius: iced::border::Radius {
                                        top_left: if index == 0 {
                                            (crate::theme::shape::md() - 1.0).max(0.0)
                                        } else {
                                            0.0
                                        },
                                        top_right: if index == 0 {
                                            (crate::theme::shape::md() - 1.0).max(0.0)
                                        } else {
                                            0.0
                                        },
                                        bottom_left: if index + 1 == row_count {
                                            (crate::theme::shape::md() - 1.0).max(0.0)
                                        } else {
                                            0.0
                                        },
                                        bottom_right: if index + 1 == row_count {
                                            (crate::theme::shape::md() - 1.0).max(0.0)
                                        } else {
                                            0.0
                                        },
                                    },
                                    ..Border::default()
                                },
                                ..container::Style::default()
                            },
                        ));
                    }
                    content = content.push(
                        container(table)
                            .padding(1)
                            .width(Fill)
                            .style(style::container::bordered),
                    );
                }
            }
        }
        if self.source.is_empty() && self.streaming {
            content = content.push(StreamingParagraph::new("", true, self.link.clone()));
        }
        content.into()
    }
}

impl<'a, Message: Clone + 'a> From<AiContent<'a, Message>> for Element<'a, Message> {
    fn from(content: AiContent<'a, Message>) -> Self {
        content.view()
    }
}

pub(super) fn rich_paragraph<'a, Message: 'a>(
    words: &'a str,
    link: Option<Rc<dyn Fn(String) -> Message + 'a>>,
    partial: bool,
) -> Element<'a, Message> {
    let mut paragraph = rich_text(inline(words, partial))
        .font(typography::ui())
        .size(typography::body() + 2.0)
        .line_height(LineHeight::Relative(1.65))
        .width(Fill)
        .wrapping(Wrapping::WordOrGlyph);
    if let Some(link) = link {
        paragraph = paragraph.on_link_click(move |location| link(location));
    }
    paragraph.into()
}

fn inline(source: &str, partial: bool) -> Vec<Span<'_, String>> {
    let mut spans = Vec::new();
    let mut source = source;
    while !source.is_empty() {
        let special = source.find(['*', '`', '[']).unwrap_or(source.len());
        if special > 0 {
            spans.push(Span::new(&source[..special]));
            source = &source[special..];
        }
        if source.is_empty() {
            break;
        }
        let (marker, font) = if source.starts_with("**") {
            ("**", typography::ui_strong())
        } else if source.starts_with('`') {
            ("`", code_font())
        } else {
            (
                "*",
                iced::Font {
                    style: iced::font::Style::Italic,
                    ..typography::ui()
                },
            )
        };
        if source.starts_with('[') {
            if let Some(label_end) = source.find("](")
                && let Some(end) = source[label_end + 2..].find(')')
            {
                let end = label_end + 2 + end;
                spans.push(
                    Span::new(&source[1..label_end])
                        .link(source[label_end + 2..end].to_owned())
                        .underline(true),
                );
                source = &source[end + 1..];
                continue;
            }
            if partial {
                // A link destination can arrive over many provider chunks.
                // Keep its label visible while withholding incomplete metadata.
                let end = source.find(']').unwrap_or(source.len());
                spans.push(Span::new(&source[1..end]));
                break;
            }
        } else if let Some(end) = source[marker.len()..].find(marker) {
            let end = marker.len() + end;
            spans.push(Span::new(&source[marker.len()..end]).font(font));
            source = &source[end + marker.len()..];
            continue;
        } else if partial {
            // Apply the opening style immediately. Closing Markdown markers
            // then leave the text width and font unchanged.
            spans.push(Span::new(&source[marker.len()..]).font(font));
            break;
        }
        let first = source.chars().next().map_or(1, char::len_utf8);
        spans.push(Span::new(&source[..first]));
        source = &source[first..];
    }
    spans
}

#[derive(Debug)]
enum Block<'a> {
    Text(&'a str),
    Heading(usize, &'a str),
    Item(String, &'a str),
    Quote(&'a str),
    Code(&'a str, String),
    Table(Vec<Vec<&'a str>>),
}

fn parse(source: &str) -> Vec<Block<'_>> {
    let lines: Vec<_> = source.split_inclusive('\n').collect();
    let mut blocks = Vec::new();
    let mut index = 0;
    let mut offset = 0;
    while index < lines.len() {
        let raw = lines[index];
        let line = raw.trim_end_matches(['\r', '\n']);
        if line.trim().is_empty() {
            index += 1;
            offset += raw.len();
            continue;
        }
        if let Some(language) = line.trim_start().strip_prefix("```") {
            index += 1;
            offset += raw.len();
            let mut code = String::new();
            while index < lines.len() && !lines[index].trim_start().starts_with("```") {
                code.push_str(lines[index]);
                offset += lines[index].len();
                index += 1;
            }
            if index < lines.len() {
                offset += lines[index].len();
                index += 1;
            }
            blocks.push(Block::Code(
                language.trim(),
                code.trim_end_matches('\n').to_owned(),
            ));
            continue;
        }
        if line.starts_with('|')
            && index + 1 < lines.len()
            && lines[index + 1]
                .chars()
                .all(|ch| matches!(ch, '|' | '-' | ':' | ' ' | '\r' | '\n'))
        {
            let mut rows = vec![cells(line)];
            offset += raw.len() + lines[index + 1].len();
            index += 2;
            while index < lines.len() && lines[index].starts_with('|') {
                rows.push(cells(lines[index].trim()));
                offset += lines[index].len();
                index += 1;
            }
            blocks.push(Block::Table(rows));
            continue;
        }
        let hashes = line.chars().take_while(|ch| *ch == '#').count();
        if (1..=3).contains(&hashes) && line.as_bytes().get(hashes) == Some(&b' ') {
            blocks.push(Block::Heading(hashes, line[hashes + 1..].trim()));
        } else if let Some(words) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            blocks.push(Block::Item("•".into(), words));
        } else if let Some(words) = line.strip_prefix("> ") {
            blocks.push(Block::Quote(words));
        } else if let Some((number, words)) = line.split_once(". ").filter(|(number, _)| {
            !number.is_empty() && number.chars().all(|ch| ch.is_ascii_digit())
        }) {
            blocks.push(Block::Item(format!("{number}."), words));
        } else {
            let start = offset;
            let mut end = offset + line.len();
            offset += raw.len();
            index += 1;
            while index < lines.len()
                && !lines[index].trim().is_empty()
                && !lines[index].starts_with(['#', '-', '*', '>', '|'])
                && !lines[index].trim_start().starts_with("```")
            {
                end = offset + lines[index].trim_end_matches(['\r', '\n']).len();
                offset += lines[index].len();
                index += 1;
            }
            blocks.push(Block::Text(&source[start..end]));
            continue;
        }
        offset += raw.len();
        index += 1;
    }
    blocks
}

fn cells(line: &str) -> Vec<&str> {
    line.trim_matches('|').split('|').map(str::trim).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_code_fences_tables_and_unicode_are_kept() {
        let blocks = parse(
            "## Sonuç\n\nÇevre **100 m**.\n\n| A | B |\n| --- | --- |\n| ç | 🦀 |\n\n```python\nprint('çizim')\n",
        );
        assert!(matches!(blocks[0], Block::Heading(2, "Sonuç")));
        assert!(matches!(&blocks[2], Block::Table(rows) if rows.len() == 2));
        assert!(matches!(&blocks[3], Block::Code("python", code) if code == "print('çizim')"));
        for source in [
            "a [eksik",
            "**kalın",
            "`kod",
            "*",
            "東京 [belge](drawing://7)",
        ] {
            assert!(!inline(source, false).is_empty());
        }
    }

    #[test]
    fn streamed_inline_markup_keeps_the_opening_style_and_hides_link_metadata() {
        let visible = |source: &str| {
            inline(source, true)
                .iter()
                .map(|span| span.text.as_ref())
                .collect::<String>()
        };
        assert_eq!(visible("Üç **parsel"), "Üç parsel");
        assert_eq!(visible("Üç **parsel**"), "Üç parsel");
        assert_eq!(visible("`parcel.area"), "parcel.area");
        assert_eq!(visible("[Belge](drawing://par"), "Belge");
        assert_eq!(visible("[Belge](drawing://parcels)"), "Belge");
        assert_eq!(
            inline("**parsel", true)[0].font,
            inline("**parsel**", false)[0].font
        );
    }
}
