//! Özellik ızgarası: CAD programlarındaki "Özellikler" paleti.
//!
//! Anahtar ve değer iki sütunda, kategoriler başlık satırlarıyla gruplanır;
//! hücreler arasında 1 piksellik ızgara çizgileri bulunur.

use std::marker::PhantomData;

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, container, row};
use iced::{Center, Element, Fill};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;

const KEY_WIDTH: f32 = 112.0;

/// Özellik ızgarası.
pub struct PropertyGrid<'a, Message> {
    entries: Vec<Entry<'a>>,
    _message: PhantomData<Message>,
}

enum Entry<'a> {
    Category(Fragment<'a>),
    Property {
        key: Fragment<'a>,
        value: Fragment<'a>,
        mono: bool,
    },
}

impl<'a, Message> PropertyGrid<'a, Message> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            _message: PhantomData,
        }
    }

    /// Sonraki özellikleri gruplayan kategori satırı.
    pub fn category(mut self, title: impl IntoFragment<'a>) -> Self {
        self.entries.push(Entry::Category(title.into_fragment()));
        self
    }

    /// Metin değerli özellik.
    pub fn property(mut self, key: impl IntoFragment<'a>, value: impl IntoFragment<'a>) -> Self {
        self.entries.push(Entry::Property {
            key: key.into_fragment(),
            value: value.into_fragment(),
            mono: false,
        });
        self
    }

    /// Sayısal değerli özellik; değer eş aralıklı yazılır.
    pub fn figure(mut self, key: impl IntoFragment<'a>, value: impl IntoFragment<'a>) -> Self {
        self.entries.push(Entry::Property {
            key: key.into_fragment(),
            value: value.into_fragment(),
            mono: true,
        });
        self
    }
}

impl<'a, Message> Default for PropertyGrid<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<PropertyGrid<'a, Message>> for Element<'a, Message> {
    fn from(grid: PropertyGrid<'a, Message>) -> Self {
        let rows = grid.entries.into_iter().map(|entry| match entry {
            Entry::Category(title) => container(
                row![
                    icon(Icon::ChevronDown).size(10.0).tone(Tone::Muted),
                    label::caption(title)
                        .font(crate::theme::typography::UI_STRONG)
                        .style(style::text::default),
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding([3, 8])
            .width(Fill)
            .style(style::container::header)
            .into(),
            Entry::Property { key, value, mono } => row![
                container(label::muted(key))
                    .width(KEY_WIDTH)
                    .padding([3, 8])
                    .style(style::container::surface),
                container(if mono {
                    label::mono(value)
                } else {
                    label::body(value)
                })
                .width(Fill)
                .padding([3, 8])
                .style(style::container::surface_alt),
            ]
            .spacing(1)
            .into(),
        });

        container(Column::with_children(rows).spacing(1))
            .padding([1, 0])
            .width(Fill)
            .style(style::container::grid_lines)
            .into()
    }
}
