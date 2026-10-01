use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::text::{self as advanced_text, Renderer as _};
use iced::advanced::widget::{Tree, Widget};
use iced::keyboard::{Key, key::Named};
use iced::widget::text_editor::{Binding, Status};
use iced::widget::{Column, button, column, container, row, scrollable, space, text};
use iced::{Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

use crate::icon::{Icon, icon};
use crate::style;
use crate::theme::{Tokens, typography};

use super::surface::{Surface, status_color};
use super::{EntryKind, PythonEditor, ReplEvent, ReplState, RunStatus, font, strong_font, syntax};

/// Python console: colourized source history, differentiated output and errors,
/// multiline input, command history, and animated execution/status feedback.
pub struct PythonRepl<'a, Message> {
    state: &'a ReplState,
    on_event: Box<dyn Fn(ReplEvent) -> Message + 'a>,
    height: Length,
    output_id: Option<iced::widget::Id>,
    title: &'a str,
}

impl<'a, Message: Clone + 'a> PythonRepl<'a, Message> {
    pub fn new(state: &'a ReplState, on_event: impl Fn(ReplEvent) -> Message + 'a) -> Self {
        Self {
            state,
            on_event: Box::new(on_event),
            height: Fill,
            output_id: None,
            title: "Python REPL",
        }
    }
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }
    /// Use `iced::widget::operation::snap_to_end(id)` after new output arrives.
    pub fn output_id(mut self, id: impl Into<iced::widget::Id>) -> Self {
        self.output_id = Some(id.into());
        self
    }

    fn view(self) -> Element<'a, Message> {
        let on_event: std::rc::Rc<dyn Fn(ReplEvent) -> Message + 'a> =
            std::rc::Rc::from(self.on_event);
        let state = self.state;
        let status = state.status();
        let running = status == RunStatus::Running;
        let submit = (on_event)(ReplEvent::Submit);
        let interrupt = (on_event)(ReplEvent::Interrupt);
        let up = (on_event)(ReplEvent::History(-1));
        let down = (on_event)(ReplEvent::History(1));
        let undo = (on_event)(ReplEvent::Undo);
        let redo = (on_event)(ReplEvent::Redo);
        let ready = syntax::ready(&state.input.content.text());
        let one_line = state.input.content.line_count() == 1;
        let header = row![
            icon(Icon::Terminal).size(16.0),
            text(self.title)
                .font(strong_font())
                .size(typography::body()),
            space::horizontal(),
            text(format!("●  {}", status.label()))
                .font(font())
                .size(typography::caption())
                .style(move |theme: &Theme| text::Style {
                    color: Some(status_color(status, &Tokens::of(theme)))
                }),
            button(
                row![
                    icon(Icon::Eraser).size(13.0),
                    text("Temizle").font(font()).size(typography::caption())
                ]
                .spacing(5)
                .align_y(iced::Center)
            )
            .on_press((on_event)(ReplEvent::Clear))
            .padding([4, 8])
            .style(style::button::flat),
        ]
        .spacing(10)
        .align_y(iced::Center);
        let mut transcript = Column::new().spacing(8).padding([14, 16]);
        if state.entries().len() == 0 {
            transcript = transcript.push(
                column![
                    text(">>>").font(strong_font()).size(typography::figure()).style(|theme: &Theme| text::Style { color: Some(Tokens::of(theme).accent) }),
                    text("Python oturumunuz hazır.").font(font()).size(typography::body()),
                    text("Bir ifade yazın. Enter çalıştırır; Shift+Enter yeni satır açar.\n↑ ve ↓ önceki komutları getirir.").font(font()).size(typography::caption()).style(style::text::muted),
                ].spacing(8)
            );
        }
        for entry in state.entries() {
            let input = entry.kind == EntryKind::Input;
            let marker = match entry.kind {
                EntryKind::Input => format!("In [{}]", entry.execution),
                EntryKind::Output => "Out".into(),
                EntryKind::Error => "Hata".into(),
                EntryKind::Note => "•".into(),
            };
            let kind = entry.kind;
            let label = text(marker)
                .font(strong_font())
                .size(typography::caption())
                .width(64)
                .style(move |theme: &Theme| {
                    let t = Tokens::of(theme);
                    text::Style {
                        color: Some(match kind {
                            EntryKind::Input => t.accent_hover,
                            EntryKind::Output => t.success,
                            EntryKind::Error => t.danger,
                            EntryKind::Note => t.muted,
                        }),
                    }
                });
            let content: Element<'_, Message> = if input {
                scrollable(CodeView::new(&entry.text, typography::body() + 1.0))
                    .direction(scrollable::Direction::Horizontal(
                        scrollable::Scrollbar::new().width(4).scroller_width(4),
                    ))
                    .height(
                        entry.text.split('\n').count() as f32 * (typography::body() + 1.0) * 1.65
                            + 5.0,
                    )
                    .into()
            } else {
                text(entry.text.trim_end_matches('\n'))
                    .font(font())
                    .size(typography::body() + 1.0)
                    .line_height(advanced_text::LineHeight::Relative(1.65))
                    .style(move |theme: &Theme| text::Style {
                        color: Some(match kind {
                            EntryKind::Error => Tokens::of(theme).danger,
                            EntryKind::Note => Tokens::of(theme).muted,
                            _ => Tokens::of(theme).text,
                        }),
                    })
                    .into()
            };
            transcript = transcript.push(
                row![label, container(content).width(Fill)]
                    .spacing(10)
                    .align_y(iced::Top),
            );
        }
        let mut output = scrollable(transcript).width(Fill).height(Fill).direction(
            scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(6).scroller_width(6),
            ),
        );
        if let Some(id) = self.output_id {
            output = output.id(id);
        }
        let size = typography::body() + 1.0;
        let rows = state.input.content.line_count().clamp(2, 7);
        let edit_event = on_event.clone();
        let key_event = on_event.clone();
        let completion_event = on_event.clone();
        let input = PythonEditor::new(&state.input.content, move |action| {
            (edit_event)(ReplEvent::Edit(action))
        })
        .header(false)
        .footer(false)
        .prompt()
        .size(size)
        .placeholder("print(\"Merhaba, KentOS\")")
        .height(rows as f32 * size * 1.65 + 26.0)
        .completions(&state.input.completion, move |event| {
            completion_event(ReplEvent::Complete(event))
        })
        .key_binding(move |kp| {
            if !matches!(kp.status, Status::Focused { .. }) {
                return None;
            }
            let command = kp.modifiers.command();
            let shift = kp.modifiers.shift();
            let custom = |message| Some(Binding::Custom(message));
            match kp.key.as_ref() {
                Key::Named(Named::Enter) if !running && (command || (!shift && ready)) => {
                    custom(submit.clone())
                }
                Key::Named(Named::ArrowUp) if !shift && (command || one_line) => custom(up.clone()),
                Key::Named(Named::ArrowDown) if !shift && (command || one_line) => {
                    custom(down.clone())
                }
                Key::Character(ch) if command && ch.eq_ignore_ascii_case("z") => {
                    custom(if shift { redo.clone() } else { undo.clone() })
                }
                Key::Character(ch) if command && ch.eq_ignore_ascii_case("y") => {
                    custom(redo.clone())
                }
                Key::Named(Named::Tab) => custom((key_event)(ReplEvent::Edit(
                    iced::widget::text_editor::Action::Edit(if shift {
                        iced::widget::text_editor::Edit::Unindent
                    } else {
                        iced::widget::text_editor::Edit::Indent
                    }),
                ))),
                _ => Binding::from_key_press(kp),
            }
        });
        let action = if running {
            interrupt
        } else {
            (on_event)(ReplEvent::Submit)
        };
        let go = button(
            row![
                icon(if running { Icon::Stop } else { Icon::Play }).size(12.0),
                text(if running { "Durdur" } else { "Çalıştır" })
                    .font(font())
                    .size(typography::caption())
            ]
            .spacing(6)
            .align_y(iced::Center),
        )
        .on_press_maybe((running || !state.input.content.is_empty()).then_some(action))
        .padding([6, 11])
        .style(if running {
            style::button::danger
        } else {
            style::button::primary
        });
        let timing = state.elapsed().map_or_else(String::new, |duration| {
            format!("{:.1} ms", duration.as_secs_f64() * 1000.0)
        });
        let footer = row![
            text("Enter çalıştır · Shift+Enter satır · ↑↓ geçmiş")
                .font(font())
                .size(typography::caption())
                .width(Fill)
                .style(style::text::muted),
            text(timing)
                .font(font())
                .size(typography::caption())
                .style(style::text::muted),
            go,
        ]
        .spacing(8)
        .align_y(iced::Center);
        let content = column![
            container(header).padding([9, 14]).width(Fill),
            super::super::horizontal_divider(),
            output,
            super::super::horizontal_divider(),
            container(input).padding([8, 10]).width(Fill),
            container(footer)
                .padding([0, 14])
                .padding(iced::Padding::new(0.0).left(14.0).right(14.0).bottom(12.0))
                .width(Fill),
        ]
        .height(self.height);
        Surface::new(
            container(content)
                .padding(1)
                .width(Fill)
                .height(self.height),
            status,
            state.revision(),
        )
        .into()
    }
}

impl<'a, Message: Clone + 'a> From<PythonRepl<'a, Message>> for Element<'a, Message> {
    fn from(repl: PythonRepl<'a, Message>) -> Self {
        repl.view()
    }
}

/// Static code in the transcript: tokenize once when the view is rebuilt and
/// draw only visible lines. JetBrains Mono's advance is 0.6 em in both weights.
pub struct CodeView<'a> {
    lines: Vec<std::borrow::Cow<'a, str>>,
    spans: Vec<Vec<(std::ops::Range<usize>, syntax::Kind)>>,
    size: f32,
}

impl<'a> CodeView<'a> {
    pub fn new(source: &'a str, size: f32) -> Self {
        Self {
            lines: source.split('\n').map(std::borrow::Cow::Borrowed).collect(),
            spans: syntax::lines(source),
            size,
        }
    }

    pub fn owned(source: String, size: f32) -> CodeView<'static> {
        CodeView {
            spans: syntax::lines(&source),
            lines: source
                .split('\n')
                .map(|line| std::borrow::Cow::Owned(line.to_owned()))
                .collect(),
            size,
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for CodeView<'_> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }
    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let width = self
            .lines
            .iter()
            .map(|line| {
                line.chars()
                    .map(|c| if c == '\t' { 4 } else { 1 })
                    .sum::<usize>()
            })
            .max()
            .unwrap_or(1) as f32
            * self.size
            * 0.6;
        layout::Node::new(limits.resolve(
            Length::Shrink,
            Length::Shrink,
            Size::new(width, self.lines.len() as f32 * self.size * 1.65),
        ))
    }
    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some(clip) = layout.bounds().intersection(viewport) else {
            return;
        };
        let t = Tokens::of(theme);
        let height = self.size * 1.65;
        for (n, (line, spans)) in self.lines.iter().zip(&self.spans).enumerate() {
            let y = layout.bounds().y + n as f32 * height;
            if y + height < clip.y || y > clip.y + clip.height {
                continue;
            }
            let mut start = 0;
            let mut x = layout.bounds().x;
            let mut draw = |words: &str, kind: Option<syntax::Kind>| {
                if words.is_empty() {
                    return;
                }
                let words = words.replace('\t', "    ");
                let width = words.chars().count() as f32 * self.size * 0.6;
                renderer.fill_text(
                    advanced_text::Text {
                        content: words,
                        bounds: Size::new(width + 1.0, height),
                        size: self.size.into(),
                        line_height: advanced_text::LineHeight::Relative(1.65),
                        font: if matches!(
                            kind,
                            Some(syntax::Kind::Keyword | syntax::Kind::Definition)
                        ) {
                            strong_font()
                        } else {
                            font()
                        },
                        align_x: advanced_text::Alignment::Default,
                        align_y: iced::alignment::Vertical::Top,
                        shaping: advanced_text::Shaping::Advanced,
                        wrapping: advanced_text::Wrapping::None,
                    },
                    Point::new(x, y),
                    kind.map_or(t.text, |kind| syntax::color(kind, theme)),
                    clip,
                );
                x += width;
            };
            for (range, kind) in spans {
                draw(&line[start..range.start], None);
                draw(&line[range.clone()], Some(*kind));
                start = range.end;
            }
            draw(&line[start..], None);
        }
    }
}

impl<'a, Message: 'a> From<CodeView<'a>> for Element<'a, Message> {
    fn from(code: CodeView<'a>) -> Self {
        Self::new(code)
    }
}
