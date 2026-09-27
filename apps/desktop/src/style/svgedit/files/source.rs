//! The XML source under the canvas (the web's `svgSource.ts`, Inkscape's
//! XML editor as text): the drawing's SVG, one element per line with its
//! id, line numbers beside it. The selected shapes' elements stand out, and
//! a click on an element selects its shape. Edited text is read back with
//! Uygula (Ctrl+Enter) as one undo step; an XML error names its line and
//! column and puts the cursor there. While the text is edited it no longer
//! follows the drawing; Geri al drops the edit.
//!
//! The editor grows with its text inside one scroll area with the line
//! numbers, so the two scroll together; tags, attributes and values are
//! coloured, and the chosen elements take the accent (Iced's editor paints
//! no backgrounds behind text).

use std::ops::Range;

use iced::advanced::text::highlighter::{self, Format};
use iced::keyboard::{self, key::Named};
use iced::widget::text_editor::{self, Action, Binding, Content};
use iced::widget::{
    Column, button, column, container, row, scrollable, space, text_editor as editor,
};
use iced::{Center, Color, Element, Fill, Font, Length, Theme};
use kentos_svg_core::export::{SvgTextOptions, write};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style as ui_style;
use kentos_ui::theme::{Tokens, typography};

use super::super::state::SvgEditor;
use super::super::{Event as EdEvent, change, ev};
use super::read::{read_svg, summary, xml_error};
use super::{Event, FileCmd};
use crate::app::Message;

pub struct SourcePanel {
    pub content: Content,
    /// The text the drawing gave (what an edit is compared with).
    generated: String,
    /// The revision it was made at.
    generated_at: u64,
    /// Each shape's element: its id and its range in UTF-16 code units.
    spans: Vec<(String, usize, usize)>,
    pub edited: bool,
    pub note: Option<(String, bool)>,
}

fn source_text(ed: &SvgEditor) -> (String, Vec<(String, usize, usize)>) {
    match write(&ed.doc.to_obj(), &SvgTextOptions::default(), true) {
        Ok(w) => {
            let spans = w
                .spans
                .into_iter()
                .filter_map(|(id, a, b)| match id {
                    kentos_geometry_core::api::json::Json::Str(s) => Some((s, a, b)),
                    _ => None,
                })
                .collect();
            (w.text, spans)
        }
        Err(_) => (String::new(), Vec::new()),
    }
}

/// A UTF-16 offset as a line and a byte column of `text`.
fn line_col(text: &str, utf16: usize) -> (usize, usize) {
    let (mut line, mut col, mut u) = (0, 0, 0);
    for c in text.chars() {
        if u >= utf16 {
            break;
        }
        u += c.len_utf16();
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += c.len_utf8();
        }
    }
    (line, col)
}

/// A line and a character column as a UTF-16 offset of `text`.
fn offset_of(text: &str, line: usize, column: usize) -> usize {
    let mut u = 0;
    for (i, l) in text.split('\n').enumerate() {
        if i == line {
            return u + l.chars().take(column).map(char::len_utf16).sum::<usize>();
        }
        u += l.encode_utf16().count() + 1;
    }
    u
}

impl SourcePanel {
    pub fn new(ed: &SvgEditor) -> SourcePanel {
        let (text, spans) = source_text(ed);
        SourcePanel {
            content: Content::with_text(&text),
            generated: text,
            generated_at: ed.revision,
            spans,
            edited: false,
            note: None,
        }
    }

    pub fn text(&self) -> String {
        self.content.text()
    }
}

impl SvgEditor {
    /// The source follows the drawing (unless edited, and not during a drag).
    pub fn follow_source(&mut self) {
        if self.op.is_some() || self.nodes.busy() {
            return;
        }
        let rev = self.revision;
        let fresh = match &self.files.source {
            Some(s) if !s.edited && s.generated_at != rev => Some(source_text(self)),
            _ => None,
        };
        if let (Some((text, spans)), Some(s)) = (fresh, self.files.source.as_mut()) {
            if text != s.generated {
                s.content = Content::with_text(&text);
                s.generated = text;
            }
            s.spans = spans;
            s.generated_at = rev;
        }
    }

    fn source_action(&mut self, action: Action) {
        let Some(s) = self.files.source.as_mut() else {
            return;
        };
        let click = matches!(action, Action::Click(_));
        s.content.perform(action);
        let text = s.content.text();
        s.edited = text != s.generated;
        if s.edited {
            s.note = Some((
                "Düzenlendi: Uygula (Ctrl+Enter) ya da Geri al.".to_owned(),
                false,
            ));
        } else if s
            .note
            .as_ref()
            .is_some_and(|n| n.0.starts_with("Düzenlendi"))
        {
            s.note = None;
        }
        // A click on an element selects its shape (while the text follows the drawing).
        if click && !s.edited {
            let pos = s.content.cursor().position;
            let at = offset_of(&text, pos.line, pos.column);
            let hit = s
                .spans
                .iter()
                .find(|(_, a, b)| at >= *a && at <= *b)
                .map(|(id, _, _)| id.clone());
            if let Some(id) = hit {
                self.select(vec![id]);
            }
        }
    }

    /// Uygula: the text read back into the drawing, one undo step.
    pub(crate) fn source_apply(&mut self) {
        let Some(s) = self.files.source.as_mut() else {
            return;
        };
        let text = s.content.text();
        if let Some(err) = xml_error(&text) {
            // The cursor goes where the parser stopped.
            s.content.move_to(text_editor::Cursor {
                position: text_editor::Position {
                    line: err.line.saturating_sub(1),
                    column: err.column.saturating_sub(1),
                },
                selection: None,
            });
            s.note = Some((
                format!("Satır {}, sütun {}: {}", err.line, err.column, err.message),
                true,
            ));
            return;
        }
        let opts = kentos_svg_core::import::ImportOptions {
            symbol_color: kentos_svg_core::import::SymbolColor::Target(None),
            second_color: None,
            editor: true,
        };
        let read = match read_svg(&text, &opts) {
            Ok(r) => r,
            Err(e) => {
                s.note = Some((e, true));
                return;
            }
        };
        let (done, lost) = summary(&read.imported.report);
        let keep: Vec<String> = self
            .selection
            .iter()
            .filter(|id| read.doc.shape(id).is_some())
            .cloned()
            .collect();
        let guides = self.doc.guides.clone();
        let mut doc = read.doc;
        // The source never held the guides: they stay.
        doc.guides = guides;
        self.settle();
        self.edit("source", move |ed| ed.doc = doc);
        self.settle();
        self.select(keep);
        if let Some(s) = self.files.source.as_mut() {
            s.edited = false;
            s.generated_at = 0;
            s.note = Some((
                format!(
                    "Uygulandı: {done}{}. Ctrl+Z geri alır.",
                    if lost.is_empty() {
                        String::new()
                    } else {
                        format!("; {}", lost.join(", "))
                    }
                ),
                !lost.is_empty(),
            ));
        }
        self.follow_source();
    }

    fn source_revert(&mut self) {
        if let Some(s) = self.files.source.as_mut() {
            s.edited = false;
            s.generated_at = 0;
            s.generated.clear();
            s.note = None;
        }
        self.follow_source();
    }
}

/// What a part of a line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Tag,
    Attr,
    Value,
    Chosen,
}

/// The chosen elements' places: line, byte range.
#[derive(Clone, Debug, PartialEq)]
pub struct Marks(pub Vec<(usize, usize, usize)>);

pub struct Xml {
    marks: Vec<(usize, usize, usize)>,
    line: usize,
}

impl highlighter::Highlighter for Xml {
    type Settings = Marks;
    type Highlight = Kind;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Kind)>;

    fn new(settings: &Marks) -> Xml {
        Xml {
            marks: settings.0.clone(),
            line: 0,
        }
    }

    fn update(&mut self, settings: &Marks) {
        self.marks.clone_from(&settings.0);
        self.line = 0;
    }

    fn change_line(&mut self, line: usize) {
        self.line = self.line.min(line);
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let n = self.line;
        self.line += 1;
        let mut out: Vec<(Range<usize>, Kind)> = Vec::new();
        let chosen: Vec<(usize, usize)> = self
            .marks
            .iter()
            .filter(|(l, _, _)| *l == n)
            .map(|(_, a, b)| (*a, (*b).min(line.len())))
            .collect();
        let bytes = line.as_bytes();
        let mut i = 0;
        let mut in_tag = false;
        while i < bytes.len() {
            let c = bytes[i];
            if c == b'<' {
                let start = i;
                i += 1;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                    i += 1;
                }
                out.push((start..i, Kind::Tag));
                in_tag = true;
            } else if in_tag && (c == b'>' || (c == b'/' && bytes.get(i + 1) == Some(&b'>'))) {
                let start = i;
                i += if c == b'/' { 2 } else { 1 };
                out.push((start..i, Kind::Tag));
                in_tag = false;
            } else if in_tag && c == b'"' {
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += 1;
                }
                i = (i + 1).min(bytes.len());
                out.push((start..i, Kind::Value));
            } else if in_tag && (c.is_ascii_alphabetic() || c == b':' || c == b'-') {
                let start = i;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b':' | b'-' | b'_'))
                {
                    i += 1;
                }
                out.push((start..i, Kind::Attr));
            } else {
                i += 1;
            }
        }
        // The chosen elements override the colours where they lie.
        if !chosen.is_empty() {
            let mut merged: Vec<(Range<usize>, Kind)> = Vec::new();
            for (r, k) in out {
                let mut pieces = vec![r];
                for &(a, b) in &chosen {
                    pieces = pieces
                        .into_iter()
                        .flat_map(|p| {
                            let mut v = Vec::new();
                            if p.start < a.min(p.end) {
                                v.push(p.start..a.min(p.end));
                            }
                            if b.max(p.start) < p.end {
                                v.push(b.max(p.start)..p.end);
                            }
                            v
                        })
                        .collect();
                }
                merged.extend(pieces.into_iter().map(|p| (p, k)));
            }
            for &(a, b) in &chosen {
                if a < b {
                    merged.push((a..b, Kind::Chosen));
                }
            }
            merged.sort_by_key(|(r, _)| r.start);
            out = merged;
        }
        out.into_iter()
    }

    fn current_line(&self) -> usize {
        self.line
    }
}

fn format(kind: &Kind, theme: &Theme) -> Format<Font> {
    let t = Tokens::of(theme);
    match kind {
        Kind::Tag => Format {
            color: Some(t.accent.scale_alpha(0.85)),
            font: None,
        },
        Kind::Attr => Format {
            color: Some(t.muted),
            font: None,
        },
        Kind::Value => Format {
            color: Some(t.text),
            font: None,
        },
        Kind::Chosen => Format {
            color: Some(t.accent),
            font: Some(typography::mono_strong()),
        },
    }
}

fn source_change(f: impl Fn(&mut SvgEditor) + Send + Sync + 'static) -> Message {
    change(f)
}

/// The panel under the canvas.
pub fn view<'a>(ed: &'a SvgEditor, s: &'a SourcePanel) -> Element<'a, Message> {
    let text = s.content.text();
    let marks = if s.edited {
        Vec::new()
    } else {
        s.spans
            .iter()
            .filter(|(id, _, _)| ed.is_selected(id))
            .flat_map(|(_, a, b)| {
                // One mark per line the element spans.
                let (la, ca) = line_col(&text, *a);
                let (lb, cb) = line_col(&text, *b);
                (la..=lb)
                    .map(|l| {
                        (
                            l,
                            if l == la { ca } else { 0 },
                            if l == lb { cb } else { usize::MAX },
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    let size = typography::scaled(12.0);
    let line_h = iced::widget::text::LineHeight::Absolute(iced::Pixels(size * 1.45));
    let lines = s.content.line_count().max(1);
    let gutter = Column::with_children((1..=lines).map(|n| {
        label::caption(n.to_string())
            .size(size)
            .line_height(line_h)
            .font(typography::mono())
            .style(ui_style::text::muted)
            .into()
    }))
    .align_x(iced::alignment::Horizontal::Right)
    .padding(iced::Padding {
        top: 4.0,
        right: 8.0,
        bottom: 4.0,
        left: 6.0,
    });
    let code = editor(&s.content)
        .on_action(|a| source_change(move |ed| ed.source_action(a.clone())))
        .font(typography::mono())
        .size(size)
        .line_height(line_h)
        .padding(4)
        .wrapping(iced::widget::text::Wrapping::None)
        .height(Length::Shrink)
        .key_binding(|k| {
            let ctrl = k.modifiers.control() || k.modifiers.logo();
            match &k.key {
                keyboard::Key::Named(Named::Enter) if ctrl => {
                    Some(Binding::Custom(source_change(|ed| ed.source_apply())))
                }
                keyboard::Key::Named(Named::Tab) if !k.modifiers.shift() => {
                    Some(Binding::Sequence(vec![
                        Binding::Insert(' '),
                        Binding::Insert(' '),
                    ]))
                }
                keyboard::Key::Named(Named::Escape) => Some(Binding::Unfocus),
                _ => Binding::from_key_press(k),
            }
        })
        .highlight_with::<Xml>(Marks(marks), format)
        .style(|t: &Theme, _| text_editor::Style {
            background: iced::Background::Color(Color::TRANSPARENT),
            border: iced::Border::default(),
            placeholder: Tokens::of(t).muted,
            value: Tokens::of(t).text,
            selection: Tokens::of(t).accent.scale_alpha(0.3),
        });
    let body = scrollable(
        row![gutter, container(code).width(Length::Shrink)].align_y(iced::Alignment::Start),
    )
    .direction(scrollable::Direction::Both {
        vertical: scrollable::Scrollbar::new().width(6).scroller_width(6),
        horizontal: scrollable::Scrollbar::new().width(6).scroller_width(6),
    })
    .width(Fill)
    .height(Fill);
    let note: Element<'a, Message> = match &s.note {
        Some((t, warn)) => {
            let warn = *warn;
            label::caption(t.clone())
                .style(move |th: &Theme| iced::widget::text::Style {
                    color: Some(if warn {
                        Tokens::of(th).danger
                    } else {
                        Tokens::of(th).muted
                    }),
                })
                .into()
        }
        None => space().into(),
    };
    let edited = s.edited;
    let head = row![
        label::caption("SVG kaynağı").font(typography::ui_strong()),
        container(note).width(Fill),
        button(label::caption("Geri al"))
            .padding([3, 9])
            .style(ui_style::button::secondary)
            .on_press_maybe(edited.then(|| source_change(|ed| ed.source_revert()))),
        button(
            row![
                icon(Icon::Check).size(13.0).tone(Tone::OnAccent),
                label::caption("Uygula").style(ui_style::text::on_accent)
            ]
            .spacing(4)
            .align_y(Center)
        )
        .padding([3, 9])
        .style(ui_style::button::primary)
        .on_press_maybe(edited.then(|| source_change(|ed| ed.source_apply()))),
        button(icon(crate::icons::from_web(Some("copy"))).size(14.0))
            .padding([3, 6])
            .style(ui_style::button::ghost)
            .on_press(ev(EdEvent::File(Event::CopySource))),
        button(icon(Icon::Close).size(14.0))
            .padding([3, 6])
            .style(ui_style::button::ghost)
            .on_press(ev(EdEvent::File(Event::Cmd(FileCmd::ToggleSource)))),
    ]
    .spacing(6)
    .align_y(Center);
    container(column![head, body].spacing(6))
        .padding([6, 8])
        .width(Fill)
        .height(Length::Fill)
        .style(|t: &Theme| container::Style {
            background: Some(iced::Background::Color(Tokens::of(t).surface)),
            border: iced::Border {
                color: Tokens::of(t).border,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}
