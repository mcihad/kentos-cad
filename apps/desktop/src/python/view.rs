//! The Python tab: what the code printed above, the code below with its
//! buttons. Enter runs code that is whole (a block ends with an empty line),
//! Shift+Enter starts a new line, Ctrl+Enter always runs; ↑ and ↓ in a
//! one-line box, or with Ctrl, go through the runs.

use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::text_editor::{Binding, KeyPress, Status};
use iced::widget::{Column, button, column, container, row, scrollable, text, text_editor};
use iced::{Element, Fill, Length, Padding, Task, Theme};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Elided, Tip, tip};

use super::{Event, Kind, Line};
use crate::app::{App, Message};

const OUTPUT: &str = "python-cikti";
/// The code box grows with its lines up to this many.
const MOST_ROWS: usize = 6;

/// The output follows its newest line.
pub fn follow() -> Task<Message> {
    iced::widget::operation::snap_to_end(OUTPUT)
}

fn ev(e: Event) -> Message {
    Message::Python(e)
}

/// Whether Enter runs the code: its brackets closed, no block left open
/// (a line ending in `:` or `\`, or a block's last line not yet empty).
pub(super) fn whole(code: &str) -> bool {
    if code.trim().is_empty() {
        return false;
    }
    let mut depth = 0_i32;
    let mut quote: Option<char> = None;
    let mut chars = code.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                if c == '\\' {
                    chars.next();
                } else if c == q {
                    quote = None;
                }
            }
            None => match c {
                '#' => {
                    // A comment runs to the line's end.
                    for c in chars.by_ref() {
                        if c == '\n' {
                            break;
                        }
                    }
                }
                '\'' | '"' => quote = Some(c),
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            },
        }
    }
    if depth > 0 || quote.is_some() {
        return false;
    }
    let last = code.lines().last().unwrap_or_default().trim_end();
    if last.ends_with(':') || last.ends_with('\\') {
        return false;
    }
    let block = code.lines().any(|l| l.trim_end().ends_with(':'));
    !block || code.ends_with('\n')
}

/// What the keys depend on, known when the box is drawn.
#[derive(Clone, Copy)]
struct Keys {
    /// Enter runs: the code is whole.
    whole: bool,
    one_line: bool,
    /// The completion list is open.
    list: bool,
    /// A name or a dot is just before the cursor: Tab completes it.
    word: bool,
}

fn keys(kp: KeyPress, k: Keys) -> Option<Binding<Message>> {
    if !matches!(kp.status, Status::Focused { .. }) {
        return None;
    }
    let custom = |e: Event| Some(Binding::Custom(ev(e)));
    let control = kp.modifiers.control();
    let shift = kp.modifiers.shift();
    match kp.key.as_ref() {
        Key::Named(Named::Escape) if k.list => custom(Event::CloseList),
        Key::Named(Named::ArrowUp) if k.list => custom(Event::Step(-1)),
        Key::Named(Named::ArrowDown) if k.list => custom(Event::Step(1)),
        Key::Named(Named::Enter | Named::Tab) if k.list => custom(Event::Accept(None)),
        Key::Named(Named::Space) if control => custom(Event::Complete),
        Key::Named(Named::Tab) if !shift && k.word => custom(Event::Complete),
        Key::Named(Named::Tab) if !shift => custom(Event::Indent),
        Key::Named(Named::Enter) if control => custom(Event::Run),
        Key::Named(Named::Enter) if !shift && k.whole => custom(Event::Run),
        Key::Named(Named::ArrowUp) if control || k.one_line => custom(Event::History(-1)),
        Key::Named(Named::ArrowDown) if control || k.one_line => custom(Event::History(1)),
        _ => Binding::from_key_press(kp),
    }
}

/// The most entries of the list on screen at once, around the active one.
const LIST_ROWS: usize = 8;

/// The completion list, over the output's lower left corner.
fn list_view(list: &super::assist::List) -> Element<'_, Message> {
    let first = list.active.saturating_sub(LIST_ROWS - 1);
    let rows = list
        .entries()
        .enumerate()
        .skip(first)
        .take(LIST_ROWS)
        .map(|(i, choice)| {
            // What it is, in the interface's words.
            let kind = match choice.kind.as_str() {
                "module" => "modül",
                "class" => "sınıf",
                "function" => "işlev",
                "property" => "özellik",
                "keyword" => "anahtar sözcük",
                _ => "değer",
            };
            let line = row![
                text(choice.text.as_str())
                    .font(typography::mono())
                    .size(typography::body()),
                text(kind)
                    .size(typography::caption())
                    .style(|theme: &Theme| text::Style {
                        color: Some(Tokens::of(theme).faint),
                    }),
                container(
                    Elided::new(choice.detail.as_str())
                        .size(typography::caption())
                        .style(|theme: &Theme| text::Style {
                            color: Some(Tokens::of(theme).muted),
                        })
                )
                .width(Fill),
            ]
            .spacing(10)
            .align_y(iced::Center);
            button(line)
                .on_press(ev(Event::Accept(Some(i))))
                .padding([3, 8])
                .width(Fill)
                .style(style::button::list_item(i == list.active))
                .into()
        });
    container(Column::with_children(rows))
        .width(Length::Fixed(typography::scaled(560.0)))
        .padding(4)
        .style(style::container::popover)
        .into()
}

/// The call the cursor is in: its label, the parameter being written and its help.
fn signature_view(s: &super::assist::Signature) -> Element<'_, Message> {
    let mut lines = Column::new().spacing(2).push(
        Elided::new(s.label.as_str())
            .font(typography::mono())
            .size(typography::body()),
    );
    if let Some(argument) = &s.argument {
        lines = lines.push(
            text(format!("▸ {argument}"))
                .font(typography::mono())
                .size(typography::caption())
                .style(|theme: &Theme| text::Style {
                    color: Some(Tokens::of(theme).accent),
                }),
        );
    }
    if !s.doc.is_empty() {
        lines = lines.push(
            Elided::new(s.doc.as_str())
                .size(typography::caption())
                .style(|theme: &Theme| text::Style {
                    color: Some(Tokens::of(theme).muted),
                }),
        );
    }
    container(lines)
        .width(Fill)
        .padding([6, 10])
        .style(style::container::popover)
        .into()
}

fn line_view(line: &Line) -> Element<'_, Message> {
    let kind = line.kind;
    let words = match kind {
        Kind::Note => text(line.text.as_str()).size(typography::caption()),
        _ => text(line.text.as_str())
            .font(typography::mono())
            .size(typography::body()),
    };
    words
        .style(move |theme: &Theme| {
            let t = Tokens::of(theme);
            text::Style {
                color: Some(match kind {
                    Kind::Input => t.muted,
                    Kind::Out => t.text,
                    Kind::Err => t.warning,
                    Kind::Error => t.danger,
                    Kind::Note => t.faint,
                }),
            }
        })
        .into()
}

fn small(glyph: Icon, about: Tip, message: Option<Message>) -> Element<'static, Message> {
    tip(
        button(icon(glyph).size(15.0))
            .on_press_maybe(message)
            .padding([5, 6])
            .style(style::button::flat),
        about,
        iced::widget::tooltip::Position::Top,
    )
}

impl App {
    /// The bottom panel's Python tab.
    pub(crate) fn python_tab(&self) -> Element<'_, Message> {
        let c = &self.python;
        let output: Element<'_, Message> = if c.lines.is_empty() {
            let intro = match &c.ready {
                Some(ready) => format!("{ready}. "),
                None => String::new(),
            };
            container(
                text(format!(
                    "{intro}Kod kendi Python'unda çalışır: doc açık çizim, cad kentos.cad. \
                     Enter çalıştırır, Shift+Enter yeni satır açar, ↑ ve ↓ önceki kodları getirir. \
                     Her çalıştırma tek geri alma adımıdır; hata ya da Durdur yazdıklarını geri alır."
                ))
                .size(typography::body())
                .style(|theme: &Theme| text::Style {
                    color: Some(Tokens::of(theme).faint),
                }),
            )
            .padding(Padding::new(14.0).bottom(12.0))
            .into()
        } else {
            scrollable(
                Column::with_children(c.lines.iter().map(line_view))
                    .spacing(1)
                    .padding([6, 12]),
            )
            .id(OUTPUT)
            .direction(style::field::body_scrollbar())
            .width(Fill)
            .height(Fill)
            .into()
        };
        let code = c.input.text();
        let (all, cursor) = super::assist::caret(&c.input);
        let word = all
            .chars()
            .nth(cursor.wrapping_sub(1))
            .is_some_and(|ch| ch.is_alphanumeric() || ch == '_' || ch == '.');
        // Two rows at least: a long line wraps into the second.
        let rows = c.input.line_count().clamp(2, MOST_ROWS);
        let line_height = typography::body() * 1.3;
        let k = Keys {
            whole: whole(&code),
            one_line: c.input.line_count() <= 1,
            list: c.completion.is_some(),
            word: cursor > 0 && word,
        };
        let editor = text_editor(&c.input)
            .placeholder("Python kodu: cad.polygon.create(doc, layer_id=…, pts=[…])")
            .on_action(|a| ev(Event::Edit(a)))
            .font(typography::mono())
            .size(typography::body())
            .height(Length::Fixed(line_height * rows as f32 + 12.0))
            .padding(Padding::from([6, 8]))
            .key_binding(move |kp| keys(kp, k))
            .style(style::field::text_area);
        let running = c.running.is_some();
        let go = if running {
            button(
                row![
                    icon(Icon::Stop).size(14.0),
                    text("Durdur").size(typography::body())
                ]
                .spacing(6)
                .align_y(iced::Center),
            )
            .on_press(ev(Event::Stop))
            .padding([5, 12])
            .style(style::button::danger)
        } else {
            button(
                row![
                    icon(Icon::Play).size(14.0),
                    text("Çalıştır").size(typography::body())
                ]
                .spacing(6)
                .align_y(iced::Center),
            )
            .on_press_maybe((!code.trim().is_empty()).then_some(ev(Event::Run)))
            .padding([5, 12])
            .style(style::button::primary)
        };
        let restart_tip = match &c.ready {
            Some(ready) => Tip::new("Yeniden başlat").detail(ready.clone()),
            None => Tip::new("Yeniden başlat"),
        };
        let buttons = row![
            tip(
                go,
                if running {
                    Tip::new("Durdur").detail("Python'u kapatır; yazdıkları geri alınır")
                } else {
                    Tip::new("Çalıştır").detail("Enter · Ctrl+Enter")
                },
                iced::widget::tooltip::Position::Top,
            ),
            small(
                Icon::Open,
                Tip::new("Betik aç…").detail(".py dosyasını çalıştırır"),
                (!running).then_some(ev(Event::Open)),
            ),
            small(
                Icon::Retry,
                restart_tip,
                (c.started() || running).then_some(ev(Event::Restart)),
            ),
            small(
                Icon::Eraser,
                Tip::new("Çıktıyı temizle"),
                (!c.lines.is_empty()).then_some(ev(Event::Clear)),
            ),
        ]
        .spacing(4)
        .align_y(iced::Center);
        let input = row![container(editor).width(Fill), buttons]
            .spacing(8)
            .padding([6, 12])
            .align_y(iced::Center);
        // The list and the signature over the output's lower edge, just above the code.
        let mut over = Column::new().spacing(4).padding([4, 12]);
        if let Some(list) = &c.completion {
            over = over.push(list_view(list));
        }
        if let Some(signature) = &c.signature
            && c.running.is_none()
        {
            over = over.push(signature_view(signature));
        }
        let area = iced::widget::Stack::with_children([
            container(output).width(Fill).height(Fill).into(),
            container(over)
                .width(Fill)
                .height(Fill)
                .align_bottom(Fill)
                .into(),
        ]);
        column![area, kentos_ui::widget::horizontal_divider(), input].into()
    }
}
