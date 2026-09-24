//! Bileşen galerisi: kentos-rc'nin kataloğu.
//!
//! Her sayfa, bileşenleri katalog girdileri ([`entry`]) olarak dizer:
//!
//! ```text
//! ┌ Bileşen adı ─────────────────────── kentos_rc::modül::yolu ┐
//! │ Ne işe yaradığı.                                            │
//! │ [canlı örnek]                                               │
//! │ ┌ kullanım kodu ──────────────────────────────────────────┐ │
//! └─────────────────────────────────────────────────────────────┘
//! ```

mod attributes;
mod controls;
mod foundations;
mod frame;
mod spatial;

use iced::widget::{column, container, scrollable, space};
use iced::{Color, Element, Fill};

use kentos_rc::label;
use kentos_rc::style;
use kentos_rc::theme::typography;
use kentos_rc::widget::Panel;

use crate::app::Showcase;
use crate::gallery::{Demo, Page};
use crate::message::Message;

/// Katalog sayfasının en geniş hâli; daha geniş pencerede sola hizalı kalır.
const PAGE_WIDTH: f32 = 1080.0;

impl Showcase {
    pub(super) fn gallery(&self) -> Element<'_, Message> {
        let page = self.gallery.page;

        let entries = match page {
            Page::Colors => self.colors_page(),
            Page::Typography => foundations::typography_page(),
            Page::Icons => foundations::icons_page(),
            Page::Buttons => self.buttons_page(),
            Page::Data => self.data_page(),
            Page::Frame => self.frame_page(),
            Page::Attributes => self.attributes_page(),
            Page::Spatial => self.spatial_page(),
        };

        let header = column![
            label::title(page.label()).size(typography::FIGURE),
            label::muted(page.description()),
        ]
        .spacing(4);

        let content = column![header]
            .extend(entries)
            .spacing(16)
            .padding(20)
            .max_width(PAGE_WIDTH);

        container(
            scrollable(content)
                .direction(style::field::thin_scrollbar())
                .width(Fill)
                .height(Fill),
        )
        .width(Fill)
        .height(Fill)
        .style(style::container::window)
        .into()
    }
}

/// Katalog girdisi: başlık çubuğunda bileşenin adı ve modül yolu; gövdede
/// açıklama, canlı örnek ve varsa kullanım kodu.
fn entry<'a>(
    title: &'a str,
    path: &'a str,
    description: &'a str,
    stage: impl Into<Element<'a, Message>>,
    code: Option<&'a str>,
) -> Element<'a, Message> {
    let mut body = column![label::muted(description), stage.into()]
        .spacing(12)
        .padding([12, 14]);

    if let Some(code) = code {
        body = body.push(
            container(label::mono_caption(code).style(style::text::default))
                .padding([8, 10])
                .width(Fill)
                .style(style::container::field),
        );
    }

    // İç boşluk, başlık çubuğunun zemininin kenar çizgisini örtmesini önler.
    container(Panel::new(title, body).trailing(label::mono_caption(path)))
        .padding(1)
        .width(Fill)
        .style(style::container::bordered)
        .into()
}

/// Galerideki etkisiz örnek düğmelerin mesajı: komut satırına yazılır.
fn pressed(name: &'static str) -> Message {
    Message::Gallery(Demo::Pressed(name))
}

/// Rengi ve ince kenarıyla gösteren örnek kutu.
fn chip<'a>(color: Color, width: f32, height: f32) -> Element<'a, Message> {
    container(space::horizontal())
        .width(width)
        .height(height)
        .style(style::container::swatch(color))
        .into()
}

/// Rengin onaltılık yazımı; saydam renklerde saydamlık da yazılır.
fn hex(color: Color) -> String {
    let [red, green, blue, alpha] = color.into_rgba8();

    if alpha == u8::MAX {
        format!("#{red:02x}{green:02x}{blue:02x}")
    } else {
        format!(
            "#{red:02x}{green:02x}{blue:02x} {:.0}%",
            f32::from(alpha) / 2.55
        )
    }
}
