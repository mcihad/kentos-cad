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
//!
//! Kısa notlar satırın sonunda da durabilir, bir seçeneğin altında da
//! sönük bir ipucu olabilir (işlem penceresinin "Nerede çalışır"ı gibi):
//!
//! ```text
//! (●) Otomatik              şimdi: arka planda
//!     2.000 ya da daha çok nesneli işler arka planda çalışır.
//! ( ) Bu bilgisayarda
//! ```

use iced::widget::button::{Status, Style};
use iced::widget::{Column, Row, button, container, row, space};
use iced::{Alignment, Background, Border, Element, Fill, Theme};

use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};

/// Radyo düğmesinin çapı.
const DOT: f32 = 16.0;

/// Radyo grubu.
pub struct RadioGroup<'a, T, Message> {
    selected: Option<T>,
    on_select: Box<dyn Fn(T) -> Message + 'a>,
    options: Vec<Item<T>>,
    horizontal: bool,
    notes_at_end: bool,
}

/// Bir seçenek.
struct Item<T> {
    value: T,
    name: String,
    description: Option<String>,
    enabled: bool,
    /// Seçeneğin altındaki sönük ipucu.
    hint: Option<String>,
}

impl<'a, T: Copy + PartialEq + 'a, Message: Clone + 'a> RadioGroup<'a, T, Message> {
    pub fn new(selected: impl Into<Option<T>>, on_select: impl Fn(T) -> Message + 'a) -> Self {
        Self {
            selected: selected.into(),
            on_select: Box::new(on_select),
            options: Vec::new(),
            horizontal: false,
            notes_at_end: false,
        }
    }

    /// Seçenek; `description` boşsa yalnızca adı yazar.
    pub fn option(self, value: T, name: impl Into<String>, description: impl Into<String>) -> Self {
        self.push(value, name.into(), description.into(), true)
    }

    /// Seçilemeyen seçenek.
    pub fn disabled(self, value: T, name: impl Into<String>) -> Self {
        self.push(value, name.into(), String::new(), false)
    }

    /// Seçilemeyen seçenek, açıklamasıyla (ör. "yakında").
    pub fn disabled_with(
        self,
        value: T,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.push(value, name.into(), description.into(), false)
    }

    fn push(mut self, value: T, name: String, description: String, enabled: bool) -> Self {
        self.options.push(Item {
            value,
            name,
            description: (!description.is_empty()).then_some(description),
            enabled,
            hint: None,
        });
        self
    }

    /// Son eklenen seçeneğin altına sönük bir ipucu (ör. Otomatik'in kuralı).
    pub fn hint(mut self, text: impl Into<String>) -> Self {
        if let Some(last) = self.options.last_mut() {
            last.hint = Some(text.into());
        }
        self
    }

    /// Açıklamalar adın altında değil, satırın sonunda küçük ve sönük
    /// yazılır; seçenekler bütün genişliği alır.
    pub fn notes_at_end(mut self) -> Self {
        self.notes_at_end = true;
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
        let at_end = group.notes_at_end;
        let mut items: Vec<Element<'a, Message>> = Vec::new();
        for item in group.options {
            let Item {
                value,
                name,
                description,
                enabled,
                hint,
            } = item;
            let selected = group.selected == Some(value);
            let name = if enabled {
                label::body(name)
            } else {
                label::body(name).style(style::text::disabled)
            };
            let content: Element<'a, Message> = if at_end {
                let mut line = row![dot(selected, enabled), name.width(Fill)]
                    .spacing(8)
                    .align_y(Alignment::Center);
                if let Some(description) = description {
                    line = line.push(label::caption(description).style(style::text::muted));
                }
                line.into()
            } else {
                // Açıklamalı seçenekte işaret ilk satıra hizalanır.
                let align = if description.is_some() {
                    Alignment::Start
                } else {
                    Alignment::Center
                };
                let mut text = Column::new().spacing(1).push(name);

                if let Some(description) = description {
                    text = text.push(label::caption(description));
                }

                row![dot(selected, enabled), text]
                    .spacing(8)
                    .align_y(align)
                    .into()
            };

            let choice = button(content)
                .on_press_maybe(enabled.then(|| (group.on_select)(value)))
                .padding([4, 6])
                .style(option);
            items.push(if at_end {
                choice.width(Fill).into()
            } else {
                choice.into()
            });
            if let Some(hint) = hint {
                // Under the name: past the button's padding, the mark and the gap.
                let indent = 6.0 + dot_side() + 8.0;
                items.push(
                    container(label::caption(hint).style(style::text::muted))
                        .padding(iced::Padding {
                            left: indent,
                            right: 6.0,
                            ..iced::Padding::ZERO
                        })
                        .into(),
                );
            }
        }

        if group.horizontal {
            Row::with_children(items).spacing(8).into()
        } else {
            Column::with_children(items).spacing(2).into()
        }
    }
}

/// Radyo işaretinin kenarı, yazı ölçeğiyle.
fn dot_side() -> f32 {
    typography::scaled(DOT).min(DOT + 4.0)
}

/// Radyo işareti: halka, seçiliyse içinde vurgu renginde nokta.
fn dot<'a, Message: 'a>(selected: bool, enabled: bool) -> Element<'a, Message> {
    let side = dot_side();
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
            radius: crate::theme::shape::radius(3.0).into(),
            ..Border::default()
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}
