//! Radyo grubu: birbirini dışlayan seçenekler; her seçeneğin isteğe bağlı
//! açıklaması olur.
//!
//! ```text
//! (●) Pencere          Tamamen içinde kalan öğeler seçilir.
//! ( ) Kesişen          Pencereye değen öğeler de seçilir.
//! ```
//!
//! ```ignore
//! RadioGroup::new(self.mode, Message::ModeSelected)
//!     .option(Mode::Window, "Pencere", "Tamamen içinde kalan öğeler seçilir.")
//!     .option(Mode::Crossing, "Kesişen", "Pencereye değen öğeler de seçilir.")
//! ```

use iced::widget::button::{Status, Style};
use iced::widget::{Column, Row, button, container, row, space};
use iced::{Alignment, Background, Border, Element, Theme};

use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};

/// Radyo düğmesinin çapı.
const DOT: f32 = 16.0;

/// Radyo grubu.
pub struct RadioGroup<'a, T, Message> {
    selected: Option<T>,
    on_select: Box<dyn Fn(T) -> Message + 'a>,
    options: Vec<(T, String, Option<String>, bool)>,
    horizontal: bool,
}

impl<'a, T: Copy + PartialEq + 'a, Message: Clone + 'a> RadioGroup<'a, T, Message> {
    pub fn new(selected: impl Into<Option<T>>, on_select: impl Fn(T) -> Message + 'a) -> Self {
        Self {
            selected: selected.into(),
            on_select: Box::new(on_select),
            options: Vec::new(),
            horizontal: false,
        }
    }

    /// Seçenek; `description` boşsa yalnızca adı yazar.
    pub fn option(
        mut self,
        value: T,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let description: String = description.into();

        self.options.push((
            value,
            name.into(),
            (!description.is_empty()).then_some(description),
            true,
        ));
        self
    }

    /// Seçilemeyen seçenek.
    pub fn disabled(mut self, value: T, name: impl Into<String>) -> Self {
        self.options.push((value, name.into(), None, false));
        self
    }

    /// Seçenekler yan yana dizilir (açıklamasız kısa seçenekler için).
    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self
    }
}

impl<'a, T: Copy + PartialEq + 'a, Message: Clone + 'a> From<RadioGroup<'a, T, Message>>
    for Element<'a, Message>
{
    fn from(group: RadioGroup<'a, T, Message>) -> Self {
        let items = group
            .options
            .into_iter()
            .map(|(value, name, description, enabled)| {
                let selected = group.selected == Some(value);
                // Açıklamalı seçenekte işaret ilk satıra hizalanır.
                let align = if description.is_some() {
                    Alignment::Start
                } else {
                    Alignment::Center
                };
                let mut text = Column::new().spacing(1).push(if enabled {
                    label::body(name)
                } else {
                    label::body(name).style(style::text::disabled)
                });

                if let Some(description) = description {
                    text = text.push(label::caption(description));
                }

                let content: Element<'a, Message> = row![dot(selected, enabled), text]
                    .spacing(8)
                    .align_y(align)
                    .into();

                button(content)
                    .on_press_maybe(enabled.then(|| (group.on_select)(value)))
                    .padding([4, 6])
                    .style(option)
                    .into()
            });

        if group.horizontal {
            Row::with_children(items).spacing(8).into()
        } else {
            Column::with_children(items).spacing(2).into()
        }
    }
}

/// Radyo işareti: halka, seçiliyse içinde vurgu renginde nokta.
fn dot<'a, Message: 'a>(selected: bool, enabled: bool) -> Element<'a, Message> {
    let side = typography::scaled(DOT).min(DOT + 4.0);
    let inner = (side * 0.45).round();

    container(
        container(space::horizontal())
            .width(inner)
            .height(inner)
            .style(move |theme: &Theme| {
                let t = Tokens::of(theme);

                container::Style {
                    background: selected
                        .then(|| Background::Color(if enabled { t.accent } else { t.disabled() })),
                    border: Border {
                        radius: (inner / 2.0).into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                }
            }),
    )
    .center_x(side)
    .center_y(side)
    .style(move |theme: &Theme| {
        let t = Tokens::of(theme);

        container::Style {
            background: Some(Background::Color(t.field)),
            border: Border {
                color: match (selected, enabled) {
                    (_, false) => t.border.scale_alpha(0.6),
                    (true, true) => t.accent,
                    (false, true) => t.muted,
                },
                width: if selected { 1.5 } else { 1.0 },
                radius: (side / 2.0).into(),
            },
            ..container::Style::default()
        }
    })
    .into()
}

/// Seçenek satırı: üzerine gelince hafif zemin.
fn option(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    Style {
        background: match status {
            Status::Hovered | Status::Pressed => Some(Background::Color(t.layer(0.05))),
            Status::Active | Status::Disabled => None,
        },
        text_color: t.text,
        border: Border {
            radius: 3.0.into(),
            ..Border::default()
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}
