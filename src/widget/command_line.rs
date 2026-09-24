//! CAD tarzı komut satırı: son komutların geçmişi ve komut girişi.

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, column, container, row, text_input};
use iced::{Bottom, Center, Element, Fill};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::typography;
use crate::widget::horizontal_divider;

const LINE_HEIGHT: f32 = 17.0;

/// Komut geçmişindeki bir satır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// Kullanıcının yazdığı komut.
    Input(String),
    /// Uygulamanın yanıtı.
    Output(String),
}

/// Komut satırı.
pub struct CommandLine<'a, Message> {
    history: &'a [Entry],
    value: &'a str,
    placeholder: Fragment<'a>,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
    lines: usize,
}

impl<'a, Message: Clone + 'a> CommandLine<'a, Message> {
    pub fn new(history: &'a [Entry], value: &'a str) -> Self {
        Self {
            history,
            value,
            placeholder: Fragment::Borrowed("Komut yazın"),
            on_input: None,
            on_submit: None,
            lines: 3,
        }
    }

    pub fn placeholder(mut self, placeholder: impl IntoFragment<'a>) -> Self {
        self.placeholder = placeholder.into_fragment();
        self
    }

    pub fn on_input(mut self, on_input: impl Fn(String) -> Message + 'a) -> Self {
        self.on_input = Some(Box::new(on_input));
        self
    }

    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }

    /// Gösterilecek geçmiş satırı sayısı.
    pub fn lines(mut self, lines: usize) -> Self {
        self.lines = lines;
        self
    }
}

impl<'a, Message: Clone + 'a> From<CommandLine<'a, Message>> for Element<'a, Message> {
    fn from(command_line: CommandLine<'a, Message>) -> Self {
        let start = command_line
            .history
            .len()
            .saturating_sub(command_line.lines);

        let history = Column::with_children(command_line.history[start..].iter().map(|entry| {
            match entry {
                Entry::Input(command) => label::mono(format!("Komut: {command}")).into(),
                Entry::Output(output) => label::mono(output.as_str())
                    .style(style::text::muted)
                    .into(),
            }
        }))
        .spacing(1);

        let mut input = text_input(&command_line.placeholder, command_line.value)
            .font(typography::MONO)
            .size(12.5)
            .padding([4, 6])
            .width(Fill)
            .style(style::field::bare_input);

        if let Some(on_input) = command_line.on_input {
            input = input.on_input(on_input);
        }

        if let Some(on_submit) = command_line.on_submit {
            input = input.on_submit(on_submit);
        }

        column![
            horizontal_divider(),
            container(history)
                .width(Fill)
                .height(LINE_HEIGHT * command_line.lines as f32 + 3.0)
                .padding([3, 10])
                .align_y(Bottom)
                .style(style::container::surface),
            container(
                row![
                    icon(Icon::ChevronRight).size(12.0).tone(Tone::Accent),
                    input
                ]
                .spacing(4)
                .align_y(Center),
            )
            .padding([2, 8])
            .width(Fill)
            .style(style::container::field),
        ]
        .into()
    }
}
