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
use kentos_ui::widget::{Tip, tip};

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

fn keys(kp: KeyPress, whole: bool, one_line: bool) -> Option<Binding<Message>> {
    if !matches!(kp.status, Status::Focused { .. }) {
        return None;
    }
    let custom = |e: Event| Some(Binding::Custom(ev(e)));
    let control = kp.modifiers.control();
    match kp.key.as_ref() {
        Key::Named(Named::Enter) if control => custom(Event::Run),
        Key::Named(Named::Enter) if !kp.modifiers.shift() && whole => custom(Event::Run),
        Key::Named(Named::ArrowUp) if control || one_line => custom(Event::History(-1)),
        Key::Named(Named::ArrowDown) if control || one_line => custom(Event::History(1)),
        _ => Binding::from_key_press(kp),
    }
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
        // Two rows at least: a long line wraps into the second.
        let rows = c.input.line_count().clamp(2, MOST_ROWS);
        let line_height = typography::body() * 1.3;
        let whole = whole(&code);
        let one_line = c.input.line_count() <= 1;
        let editor = text_editor(&c.input)
            .placeholder("Python kodu: cad.polygon.create(doc, layer_id=…, pts=[…])")
            .on_action(|a| ev(Event::Edit(a)))
            .font(typography::mono())
            .size(typography::body())
            .height(Length::Fixed(line_height * rows as f32 + 12.0))
            .padding(Padding::from([6, 8]))
            .key_binding(move |kp| keys(kp, whole, one_line))
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
        column![
            container(output).width(Fill).height(Fill),
            kentos_ui::widget::horizontal_divider(),
            input
        ]
        .into()
    }
}
