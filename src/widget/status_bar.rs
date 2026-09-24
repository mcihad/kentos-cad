//! Durum çubuğu: göstergeler ve açık/kapalı anahtarlar.
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────────────┐
//! │ ⌖ 41.00820° K  28.97840° D ˄ │ ↖ 2 seçili ˄       ▦ Izgara  ⋔ Yakalama │ 1:25.000 ˄ │
//! └────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Göstergeler ([`Readout`]) bir değer gösterir. Menüsü olan gösterge
//! tıklanınca yukarı doğru açılan bir menü sunar (ör. koordinat biçimi,
//! ölçek); bunu sağındaki küçük ok belli eder. Anahtarlar ([`Toggle`]) açıkken
//! ikonlarıyla vurgulanır. Çubukta hap ya da kutu yoktur: zemin yalnızca
//! üzerine gelince belirir.
//!
//! İmleçle değişen değerler (koordinat) sabit genişlikte tutulur; böylece
//! imleç hareket ettikçe çubuktaki öğeler kıpırdamaz.
//!
//! ```ignore
//! StatusBar::new()
//!     .push(
//!         Readout::new(label::mono(coordinates))
//!             .icon(Icon::Target)
//!             .width(236.0)
//!             .menu(|| coordinate_formats()),
//!     )
//!     .separator()
//!     .push(
//!         Toggle::new("Izgara", grid)
//!             .icon(Icon::Grid)
//!             .shortcut("F7")
//!             .on_press(Message::ToggleGrid),
//!     )
//!     .spacer()
//!     .push(Readout::new(label::mono_caption("EPSG:3857")).icon(Icon::Globe))
//! ```

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{button, column, container, row, space, tooltip};
use iced::{Center, Element, Fill, Length};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::typography;
use crate::widget::context_menu::{Menu, MenuButton};
use crate::widget::{Tip, horizontal_divider, tip, vertical_divider};

/// Durum çubuğunun yüksekliği, 12 piksellik gövde metninde; yazı boyutuyla
/// büyür ([`height`]).
pub const HEIGHT: f32 = 28.0;

/// Öğelerin yüksekliği; üzerine gelme zemini çubuğun üst ve altında eşit
/// boşluk bırakır.
const ITEM_HEIGHT: f32 = 22.0;

/// Geçerli yazı boyutunda durum çubuğunun yüksekliği.
pub fn height() -> f32 {
    typography::scaled(HEIGHT)
}

fn item_height() -> f32 {
    typography::scaled(ITEM_HEIGHT)
}

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
        self.push(container(vertical_divider()).height(14).padding([0, 4]))
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
                    .spacing(2)
                    .align_y(Center),
            )
            .height(height())
            .padding([0, 6])
            .width(Fill)
            .align_y(Center)
            .style(style::container::window),
        ]
        .into()
    }
}

/// Durum çubuğunda bir değer: koordinat, seçim sayısı, ölçek, koordinat
/// sistemi. Menüsü varsa tıklanınca menüyü yukarı doğru açar.
pub struct Readout<'a, Message> {
    content: Element<'a, Message>,
    icon: Option<Icon>,
    menu: Option<Box<dyn Fn() -> Menu<Message> + 'a>>,
    tip: Option<Tip>,
    width: Option<f32>,
}

impl<'a, Message: Clone + 'a> Readout<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            icon: None,
            menu: None,
            tip: None,
            width: None,
        }
    }

    /// Değerin solundaki sönük ikon.
    pub fn icon(mut self, glyph: Icon) -> Self {
        self.icon = Some(glyph);
        self
    }

    /// Tıklanınca açılan menü; her açılışta yeniden kurulur.
    pub fn menu(mut self, menu: impl Fn() -> Menu<Message> + 'a) -> Self {
        self.menu = Some(Box::new(menu));
        self
    }

    /// Üzerine gelince gösterilen açıklama. Menüsü olan göstergede
    /// gösterilmez; menü aynı yere açılır.
    pub fn tip(mut self, tip: impl Into<Tip>) -> Self {
        self.tip = Some(tip.into());
        self
    }

    /// Değerin sabit genişliği: değer değiştikçe çubuk kıpırdamasın. 12
    /// piksellik gövde metnine göre verilir ve yazı boyutuyla büyür.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Readout<'a, Message>> for Element<'a, Message> {
    fn from(readout: Readout<'a, Message>) -> Self {
        let mut content = row![].spacing(6).align_y(Center);

        if let Some(glyph) = readout.icon {
            content = content.push(icon(glyph).size(13.0).tone(Tone::Muted));
        }

        content = content.push(
            container(readout.content)
                .width(readout.width.map_or(Length::Shrink, |width| {
                    Length::Fixed(typography::scaled(width))
                }))
                .clip(true),
        );

        if readout.menu.is_some() {
            content = content.push(icon(Icon::ChevronUp).size(10.0).tone(Tone::Muted));
        }

        let body = container(content)
            .padding([0, 8])
            .height(item_height())
            .align_y(Center);

        match (readout.menu, readout.tip) {
            (Some(menu), _) => MenuButton::new(body, menu).into(),
            (None, Some(description)) => tip(body, description, tooltip::Position::Top),
            (None, None) => body.into(),
        }
    }
}

/// Durum çubuğundaki açık/kapalı anahtar (ör. Izgara, Yakalama). Açıkken
/// ikonu vurgu renginde, adı tam renkte yazılır; kapalıyken ikisi de
/// sönüktür. İpucu anahtarın durumunu ve kısayolunu gösterir.
pub struct Toggle<'a, Message> {
    label: Fragment<'a>,
    active: bool,
    icon: Option<Icon>,
    shortcut: Option<Fragment<'a>>,
    on_press: Option<Message>,
}

impl<'a, Message: Clone + 'a> Toggle<'a, Message> {
    pub fn new(label: impl IntoFragment<'a>, active: bool) -> Self {
        Self {
            label: label.into_fragment(),
            active,
            icon: None,
            shortcut: None,
            on_press: None,
        }
    }

    pub fn icon(mut self, glyph: Icon) -> Self {
        self.icon = Some(glyph);
        self
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
        let mut description = Tip::new(format!("{}: {state}", toggle.label));

        if let Some(shortcut) = &toggle.shortcut {
            description = description.detail(shortcut.to_string());
        }

        let mut content = row![].spacing(6).height(Fill).align_y(Center);

        if let Some(glyph) = toggle.icon {
            content = content.push(icon(glyph).size(14.0).tone(if toggle.active {
                Tone::Accent
            } else {
                Tone::Inherit
            }));
        }

        tip(
            button(content.push(label::body(toggle.label)))
                .on_press_maybe(toggle.on_press)
                .padding([0, 8])
                .height(item_height())
                .style(style::button::status_toggle(toggle.active)),
            description,
            tooltip::Position::Top,
        )
    }
}
