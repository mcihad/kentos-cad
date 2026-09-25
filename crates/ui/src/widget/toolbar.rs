//! Araç çubuğu: tablo ve panellerin üstündeki küçük eylem satırı (ArcGIS'teki
//! öznitelik tablosu araç çubuğu gibi).
//!
//! ```ignore
//! Toolbar::new()
//!     .search(&self.search, "Tabloda ara", Message::Search)
//!     .separator()
//!     .action(Icon::Filter, "Filtre", Some(Message::OpenFilter))
//!     .toggle(Icon::Check, "Yalnızca seçili", self.selected_only, Message::SelectedOnly)
//!     .spacer()
//!     .button(Icon::Target, "Seçime yakınlaştır", Some(Message::ZoomToSelection))
//! ```

use iced::widget::text::IntoFragment;
use iced::widget::{button, container, row, space, text_input, tooltip};
use iced::{Center, Element, Fill};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::typography;
use crate::widget::{Tip, tip, vertical_divider};

/// Araç çubuğu satırının yüksekliği, 12 piksellik gövde metninde; yazı
/// boyutuyla büyür ([`typography::scaled`]).
pub const HEIGHT: f32 = 30.0;

/// Araç çubuğu.
pub struct Toolbar<'a, Message> {
    items: Vec<Element<'a, Message>>,
}

impl<'a, Message: Clone + 'a> Toolbar<'a, Message> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(mut self, item: impl Into<Element<'a, Message>>) -> Self {
        self.items.push(item.into());
        self
    }

    /// Yalnızca ikonlu düğme; `on_press` yoksa devre dışıdır.
    pub fn button(self, glyph: Icon, description: &'a str, on_press: Option<Message>) -> Self {
        let content = button(icon(glyph))
            .on_press_maybe(on_press)
            .padding([3, 5])
            .style(style::button::flat);

        self.push(tip(
            content,
            Tip::new(description),
            tooltip::Position::Bottom,
        ))
    }

    /// İkonlu ve etiketli düğme; `on_press` yoksa devre dışıdır.
    pub fn action(
        self,
        glyph: Icon,
        text: impl IntoFragment<'a>,
        on_press: Option<Message>,
    ) -> Self {
        self.push(
            button(
                row![icon(glyph), label::body(text)]
                    .spacing(6)
                    .align_y(Center),
            )
            .on_press_maybe(on_press)
            .padding([3, 8])
            .style(style::button::flat),
        )
    }

    /// Açık/kapalı düğme; açıkken vurgulanır.
    pub fn toggle(
        self,
        glyph: Icon,
        text: impl IntoFragment<'a>,
        active: bool,
        on_press: Message,
    ) -> Self {
        self.push(
            button(
                row![
                    icon(glyph).tone(if active {
                        Tone::Highlight
                    } else {
                        Tone::Inherit
                    }),
                    label::body(text)
                ]
                .spacing(6)
                .align_y(Center),
            )
            .on_press(on_press)
            .padding([3, 8])
            .style(style::button::tool(active)),
        )
    }

    /// Arama kutusu.
    pub fn search(
        self,
        value: &'a str,
        placeholder: &'a str,
        on_input: impl Fn(String) -> Message + 'a,
    ) -> Self {
        let field = container(
            row![
                icon(Icon::Search).size(14.0).tone(Tone::Muted),
                text_input(placeholder, value)
                    .on_input(on_input)
                    .font(typography::ui())
                    .size(typography::body())
                    .padding([2, 0])
                    .style(style::field::bare_input),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([0, 8])
        .width(typography::scaled(220.0))
        .height(typography::scaled(24.0))
        .align_y(Center)
        .style(|theme| {
            let mut container = style::container::badge(theme);
            container.background = Some(crate::theme::Tokens::of(theme).field.into());
            container
        });

        self.push(field)
    }

    /// Kısa, dikey bölücü çizgi.
    pub fn separator(self) -> Self {
        self.push(container(vertical_divider()).height(16))
    }

    /// Sonraki öğeleri sağ uca iter.
    pub fn spacer(self) -> Self {
        self.push(space::horizontal())
    }
}

impl<'a, Message: Clone + 'a> Default for Toolbar<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<Toolbar<'a, Message>> for Element<'a, Message> {
    fn from(toolbar: Toolbar<'a, Message>) -> Self {
        container(
            iced::widget::Row::with_children(toolbar.items)
                .spacing(4)
                .align_y(Center),
        )
        .padding([0, 8])
        .height(typography::scaled(HEIGHT))
        .width(Fill)
        .align_y(Center)
        .style(style::container::surface)
        .into()
    }
}
