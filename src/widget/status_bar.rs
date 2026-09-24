//! Durum çubuğu ve açık/kapalı anahtarları.
//!
//! ```ignore
//! StatusBar::new()
//!     .push(label::mono(coordinates))
//!     .separator()
//!     .push(Toggle::new("Izgara", grid).shortcut("F7").on_press(Message::ToggleGrid))
//!     .spacer()
//!     .push(label::mono_caption("EPSG:3857"))
//! ```

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{button, column, container, space, text, tooltip};
use iced::{Center, Element, Fill};

use crate::style;
use crate::theme::typography;
use crate::widget::{Tip, horizontal_divider, tip, vertical_divider};

/// Durum çubuğunun yüksekliği.
pub const HEIGHT: f32 = 26.0;

/// Pencerenin altındaki durum çubuğu; üst kenarında bölücü çizgi bulunur.
pub struct StatusBar<'a, Message> {
    items: Vec<Element<'a, Message>>,
}

impl<'a, Message: 'a> StatusBar<'a, Message> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(mut self, item: impl Into<Element<'a, Message>>) -> Self {
        self.items.push(item.into());
        self
    }

    /// Kısa, dikey bölücü çizgi.
    pub fn separator(self) -> Self {
        self.push(container(vertical_divider()).height(14))
    }

    /// Sonraki öğeleri çubuğun sağ ucuna iter.
    pub fn spacer(self) -> Self {
        self.push(space::horizontal())
    }
}

impl<'a, Message: 'a> Default for StatusBar<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<StatusBar<'a, Message>> for Element<'a, Message> {
    fn from(bar: StatusBar<'a, Message>) -> Self {
        column![
            horizontal_divider(),
            container(
                iced::widget::Row::with_children(bar.items)
                    .spacing(6)
                    .align_y(Center),
            )
            .height(HEIGHT)
            .padding([0, 10])
            .width(Fill)
            .style(style::container::window),
        ]
        .into()
    }
}

/// Durum çubuğundaki açık/kapalı anahtar (ör. Izgara, Yakalama). İpucu
/// anahtarın durumunu ve kısayolunu gösterir.
pub struct Toggle<'a, Message> {
    label: Fragment<'a>,
    active: bool,
    shortcut: Option<Fragment<'a>>,
    on_press: Option<Message>,
}

impl<'a, Message: Clone + 'a> Toggle<'a, Message> {
    pub fn new(label: impl IntoFragment<'a>, active: bool) -> Self {
        Self {
            label: label.into_fragment(),
            active,
            shortcut: None,
            on_press: None,
        }
    }

    pub fn shortcut(mut self, shortcut: impl IntoFragment<'a>) -> Self {
        self.shortcut = Some(shortcut.into_fragment());
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Toggle<'a, Message>> for Element<'a, Message> {
    fn from(toggle: Toggle<'a, Message>) -> Self {
        let state = if toggle.active { "açık" } else { "kapalı" };
        let description = match &toggle.shortcut {
            Some(shortcut) => format!("{}: {state}  ({shortcut})", toggle.label),
            None => format!("{}: {state}", toggle.label),
        };

        tip(
            button(text(toggle.label).size(typography::CAPTION))
                .on_press_maybe(toggle.on_press)
                .padding([2, 7])
                .style(style::button::toggle(toggle.active)),
            Tip::new(description),
            tooltip::Position::Top,
        )
    }
}
