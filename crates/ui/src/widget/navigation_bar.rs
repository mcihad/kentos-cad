//! AutoCAD'deki gezinme çubuğu gibi dar, dikey görünüm denetimleri.

use iced::widget::{Column, button, container, tooltip};
use iced::{Element, Fill};

use crate::icon::{Icon, icon};
use crate::style;
use crate::widget::{Tip, horizontal_divider, tip};

/// Gezinme çubuğunun genişliği.
pub const WIDTH: f32 = 32.0;

const PADDING: f32 = 2.0;
const BUTTON_HEIGHT: f32 = 26.0;
const SEPARATOR_PADDING: f32 = 2.0;
const GAP: f32 = 1.0;

/// Dikey ikon düğmesi çubuğu.
pub struct NavigationBar<'a, Message> {
    items: Vec<Item<'a, Message>>,
}

enum Item<'a, Message> {
    Button {
        icon: Icon,
        tip: &'a str,
        on_press: Message,
    },
    Separator,
}

impl<'a, Message: Clone + 'a> NavigationBar<'a, Message> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// İpucu, düğmenin soluna açılır.
    pub fn button(mut self, icon: Icon, tip: &'a str, on_press: Message) -> Self {
        self.items.push(Item::Button {
            icon,
            tip,
            on_press,
        });
        self
    }

    pub fn separator(mut self) -> Self {
        self.items.push(Item::Separator);
        self
    }

    /// Çubuğun piksel cinsinden yüksekliği. Üzerine yerleştirildiği
    /// yüzeyin (ör. model alanı) bu bölgeyi ayırabilmesi içindir.
    pub fn height(&self) -> f32 {
        let items: f32 = self
            .items
            .iter()
            .map(|item| match item {
                Item::Button { .. } => BUTTON_HEIGHT,
                Item::Separator => 1.0 + SEPARATOR_PADDING * 2.0,
            })
            .sum();

        PADDING * 2.0 + items + GAP * self.items.len().saturating_sub(1) as f32
    }
}

impl<'a, Message: Clone + 'a> Default for NavigationBar<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<NavigationBar<'a, Message>> for Element<'a, Message> {
    fn from(bar: NavigationBar<'a, Message>) -> Self {
        let items = bar.items.into_iter().map(|item| match item {
            Item::Button {
                icon: glyph,
                tip: description,
                on_press,
            } => tip(
                button(container(icon(glyph).size(14.0)).center(Fill))
                    .on_press(on_press)
                    .width(Fill)
                    .height(BUTTON_HEIGHT)
                    .padding(0)
                    .style(style::button::flat),
                Tip::new(description),
                tooltip::Position::Left,
            ),
            Item::Separator => container(horizontal_divider())
                .padding([SEPARATOR_PADDING, 4.0])
                .into(),
        });

        container(Column::with_children(items).spacing(GAP))
            .padding(PADDING)
            .width(WIDTH)
            .style(style::container::floating)
            .into()
    }
}
