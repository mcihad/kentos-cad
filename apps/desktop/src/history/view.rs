//! Komut geçmişi with the calls (docs/adr/0284): the log's lines and the
//! calls in the order they came; a call is its title and command, its input
//! in words, what it wrote or why it wrote nothing, its warnings, and its
//! buttons (Python shown, Python copied, the replay input copied or saved).

use iced::widget::text::{LineHeight, Wrapping};
use iced::widget::{Column, button, column, container, row, space, text};
use iced::{Center, Color, Element, Fill, Padding, Theme};
use kentos_commands::{CallKind, CallStatus};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Tip, tip};

use super::{Call, Entry, Event, text as words};
use crate::app::{App, Message};
use crate::cloud::local_time::Zone;
use crate::log_plan::{self as plan, Line};

/// What a row of the list is: a line of the log or a call.
enum Item<'a> {
    Line(&'a Line),
    Call(&'a Entry),
}

impl App {
    /// Komut geçmişi's rows: each call after the log line that was newest
    /// when it came; `line` draws a line of the log.
    pub(crate) fn history_rows<'a>(
        &'a self,
        lines: Vec<&'a Line>,
        zone: &Zone,
        line: impl Fn(&'a Line, &Zone) -> Element<'a, Message>,
    ) -> Vec<Element<'a, Message>> {
        let mut items: Vec<Item<'a>> = Vec::new();
        let mut calls = self.history.entries.iter().peekable();
        for l in lines {
            while let Some(e) = calls.next_if(|e| e.after < l.id) {
                items.push(Item::Call(e));
            }
            items.push(Item::Line(l));
        }
        items.extend(calls.map(Item::Call));
        let mut rows = Vec::new();
        let mut step: Option<(u64, u64)> = None;
        for item in items {
            match item {
                Item::Line(l) => {
                    step = None;
                    rows.push(line(l, zone));
                }
                Item::Call(e) => {
                    let this = self.row_now(e).and_then(|r| r.step).map(|s| (e.doc, s));
                    let grouped = this.is_some() && this == step;
                    step = this;
                    rows.push(self.call_row(e, zone, grouped));
                }
            }
        }
        rows
    }

    /// Whether Komut geçmişi has calls to list.
    pub(crate) fn history_has_calls(&self) -> bool {
        !self.history.entries.is_empty()
    }

    /// A call: its head (time, title, command, buttons), its input, its
    /// outcome, its warnings and, when open, its Python. A call of the same
    /// undo step as the one before goes under it, without its time.
    fn call_row<'a>(
        &'a self,
        entry: &'a Entry,
        zone: &Zone,
        grouped: bool,
    ) -> Element<'a, Message> {
        let size = typography::caption();
        let height = LineHeight::Relative(1.5);
        let row_now = self.row_now(entry);
        let (title, command, input, outcome, tone, warnings) = match (&entry.call, row_now) {
            (Call::Row(_), Some(r)) => {
                let tone = match (r.kind, r.status) {
                    (_, CallStatus::Undone) => Tone::Faint,
                    (CallKind::Edit, _) => Tone::Muted,
                    _ => Tone::Done,
                };
                let layer = |id: &str| self.layer_name_of(id);
                (
                    words::row_title(r),
                    match (&r.command, r.version, r.step) {
                        (Some(c), Some(v), Some(s)) => format!("{c} v{v} · adım {s}"),
                        (Some(c), Some(v), None) => format!("{c} v{v}"),
                        _ => String::new(),
                    },
                    r.input
                        .as_ref()
                        .map(|i| words::inputs(i, &layer))
                        .unwrap_or_default(),
                    words::outcome(r),
                    tone,
                    r.warnings.iter().map(words::warning).collect::<Vec<_>>(),
                )
            }
            (Call::Refused(r), _) => {
                let layer = |id: &str| self.layer_name_of(id);
                (
                    words::title_of(&r.command).to_owned(),
                    format!("{} v{}", r.command, r.version),
                    words::inputs(&r.input, &layer),
                    words::refusal(r),
                    Tone::Refused,
                    Vec::new(),
                )
            }
            _ => return space::horizontal().into(),
        };
        let has_python = self.python_of(entry).is_some();
        let open = self.history.open == Some(entry.id) && has_python;

        let time: Element<'a, Message> = if grouped {
            text("↳")
                .font(typography::mono())
                .size(size)
                .style(faint)
                .into()
        } else {
            text(plan::log_time(entry.at, zone))
                .font(typography::mono())
                .size(size)
                .line_height(height)
                .style(faint)
                .into()
        };
        let mark = container(
            icon(match tone {
                Tone::Refused => Icon::Error,
                _ => Icon::Terminal,
            })
            .size(14.0),
        )
        .padding(Padding::ZERO.top(2.0))
        .style(move |theme: &Theme| container::Style {
            text_color: Some(tone.icon(theme)),
            ..container::Style::default()
        });
        let mut head = row![
            text(title)
                .font(typography::mono_strong())
                .size(size)
                .line_height(height)
                .style(move |theme: &Theme| text::Style {
                    color: Some(tone.text(theme)),
                }),
            text(command)
                .font(typography::mono())
                .size(size)
                .line_height(height)
                .style(faint),
            space::horizontal(),
        ]
        .spacing(10)
        .align_y(Center);
        if let Some(buttons) = self.call_buttons(entry, open) {
            head = head.push(buttons);
        }
        let mut body = Column::new().push(head);
        if !input.is_empty() {
            body = body.push(small(input, size, height, Tone::Muted));
        }
        body = body.push(small(outcome, size, height, tone));
        for w in warnings {
            body = body.push(small(format!("Uyarı {w}"), size, height, Tone::Warning));
        }
        if open && let Some(code) = self.python_of(entry) {
            body = body.push(code_block(code));
        }
        let lead = if grouped { 76.0 } else { 64.0 };
        container(row![
            container(time).width(typography::from_default(lead)),
            container(mark).width(18),
            container(body.spacing(1)).width(Fill),
        ])
        .padding([3, 12])
        .width(Fill)
        .into()
    }

    /// A call's buttons: Python shown, Python copied, the replay input copied and saved; none for an edit made
    /// without a command (it has no Python and no replay input).
    pub(crate) fn call_buttons(&self, entry: &Entry, open: bool) -> Option<Element<'_, Message>> {
        self.python_of(entry)?;
        let id = entry.id;
        let action = |glyph: Icon, event: Event, about: Tip| {
            tip(
                button(icon(glyph).size(13.0))
                    .on_press(Message::History(event))
                    .padding([2, 4])
                    .style(kentos_ui::style::button::flat),
                about,
                iced::widget::tooltip::Position::Top,
            )
        };
        let show = button(
            row![
                icon(if open {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                })
                .size(12.0),
                text("Python").size(typography::caption()),
            ]
            .spacing(3)
            .align_y(Center),
        )
        .on_press(Message::History(Event::Toggle(id)))
        .padding([1, 6])
        .style(kentos_ui::style::button::flat);
        let buttons = row![
            show,
            action(
                Icon::Copy,
                Event::CopyPython(id),
                Tip::new("Python kodunu kopyala").body("kentos.cad ile aynı komut, aynı girdiyle"),
            ),
            action(
                Icon::Play,
                Event::CopyReplay(id),
                Tip::new("Tekrar oynatma girdisini kopyala")
                    .body("kentos.replay/1; doc.replay(...) ile oynatılır"),
            ),
            action(
                Icon::Save,
                Event::SaveReplay(id),
                Tip::new("Tekrar oynatma girdisini kaydet…").body("kentos.replay/1 JSON dosyası"),
            ),
        ]
        .spacing(2)
        .align_y(Center);
        Some(buttons.into())
    }

    /// A layer's name by its id, in the open drawing.
    fn layer_name_of(&self, id: &str) -> Option<String> {
        let doc = self.document.as_ref()?;
        doc.model.layers().get(id).map(|n| n.name.clone())
    }
}

/// How a call's words are coloured.
#[derive(Clone, Copy)]
enum Tone {
    Done,
    Muted,
    Faint,
    Warning,
    Refused,
}

impl Tone {
    fn text(self, theme: &Theme) -> Color {
        let t = Tokens::of(theme);
        match self {
            Tone::Done => t.text,
            Tone::Muted => t.muted,
            Tone::Faint => t.muted.scale_alpha(0.7),
            Tone::Warning => t.warning,
            Tone::Refused => t.danger,
        }
    }

    fn icon(self, theme: &Theme) -> Color {
        let t = Tokens::of(theme);
        match self {
            Tone::Refused => t.danger,
            Tone::Warning => t.warning,
            _ => t.muted,
        }
    }
}

fn faint(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(Tokens::of(theme).muted.scale_alpha(0.75)),
    }
}

fn small<'a>(words: String, size: f32, height: LineHeight, tone: Tone) -> Element<'a, Message> {
    text(words)
        .font(typography::mono())
        .size(size)
        .line_height(height)
        .wrapping(Wrapping::WordOrGlyph)
        .style(move |theme: &Theme| text::Style {
            color: Some(tone.text(theme)),
        })
        .into()
}

/// The call's Python, on the field's ground.
fn code_block<'a>(code: String) -> Element<'a, Message> {
    container(
        column![
            text(code)
                .font(typography::mono())
                .size(typography::caption())
                .wrapping(Wrapping::WordOrGlyph)
        ]
        .width(Fill),
    )
    .padding([6, 8])
    .width(Fill)
    .style(|theme: &Theme| {
        let t = Tokens::of(theme);
        container::Style {
            background: Some(t.surface_alt.into()),
            border: iced::border::rounded(kentos_ui::theme::shape::radius(4.0))
                .width(1.0)
                .color(t.border),
            text_color: Some(t.text),
            ..container::Style::default()
        }
    })
    .into()
}
