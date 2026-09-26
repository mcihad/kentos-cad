//! Parçalı seçim: birbirini dışlayan birkaç seçenekten birini seçtiren
//! bitişik düğmeler (ör. seçim yöntemi: Yeni seçim / Ekle / Çıkar).

use std::fmt::Display;

use iced::widget::{button, container};
use iced::{Element, Length};

use crate::label;
use crate::style;
use crate::theme::typography;

/// Parçalı seçim.
pub struct Segmented<'a, Message> {
    segments: Vec<(String, bool, Option<Message>)>,
    width: Length,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a, Message: Clone + 'a> Segmented<'a, Message> {
    /// Seçenekler `Display` ile yazılır; seçili seçenek vurgulanır.
    pub fn new<T>(
        options: impl IntoIterator<Item = T>,
        selected: T,
        on_select: impl Fn(T) -> Message,
    ) -> Self
    where
        T: Copy + PartialEq + Display,
    {
        Self::new_with(options, selected, on_select, |_| true)
    }

    /// `new` gibi; `enabled` seçeneği kabul etmezse parça sönük ve basılamaz
    /// olur (ör. içinde nesne olmayan kapsam).
    pub fn new_with<T>(
        options: impl IntoIterator<Item = T>,
        selected: T,
        on_select: impl Fn(T) -> Message,
        enabled: impl Fn(T) -> bool,
    ) -> Self
    where
        T: Copy + PartialEq + Display,
    {
        Self {
            segments: options
                .into_iter()
                .map(|option| {
                    let press = enabled(option).then(|| on_select(option));
                    (option.to_string(), option == selected, press)
                })
                .collect(),
            width: Length::Shrink,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// `Fill` verilirse parçalar genişliği eşit paylaşır. Sabit genişlik yazı
    /// boyutuyla büyür.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

impl<'a, Message: Clone + 'a> From<Segmented<'a, Message>> for Element<'a, Message> {
    fn from(segmented: Segmented<'a, Message>) -> Self {
        let fill = segmented.width != Length::Shrink;

        let segments = segmented
            .segments
            .into_iter()
            .map(|(text, selected, on_press)| {
                let content = container(label::body(text));
                let content = if fill {
                    content.width(Length::Fill).center_x(Length::Fill)
                } else {
                    content
                };

                let segment = button(content)
                    .on_press_maybe(on_press)
                    .padding([3, 12])
                    .style(style::button::segment(selected));

                if fill {
                    segment.width(Length::Fill).into()
                } else {
                    segment.into()
                }
            });

        container(iced::widget::Row::with_children(segments).spacing(1))
            .padding(1)
            .width(typography::length(segmented.width))
            .style(style::container::segmented)
            .into()
    }
}
