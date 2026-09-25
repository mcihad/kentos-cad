//! Yan panel yuvası (dock) ve başlıklı paneller.
//!
//! ```text
//! ┃ ▾ Katmanlar           6 katman  ⌄ ⌃ │
//! ┃   …                                  │
//! ┃ ▸ Özellikler          2 öğe seçili   │  ← kapalı panel: yalnızca başlık
//! ◄─► sol kenar sürüklenir, çift tık varsayılan genişlik
//! ```
//!
//! Yuvanın genişliği ve panellerin açık ya da kapalı olması uygulamanın
//! durumudur; yuva değişiklikleri mesajla bildirir:
//!
//! ```ignore
//! Dock::new(self.dock_width)
//!     .resizable(DEFAULT_WIDTH, Message::DockResized)
//!     .on_resize_end(Message::DockResizeEnded)
//!     .push(
//!         Panel::new("Katmanlar", tree)
//!             .collapsible(self.layers_collapsed, Message::LayersToggled)
//!             .height(FillPortion(5)),
//!     )
//! ```

use std::ops::RangeInclusive;

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, button, container, row, scrollable, space};
use iced::{Center, Element, Fill, Length};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::typography;
use crate::widget::horizontal_divider;
use crate::widget::sash::Sash;

/// Sürüklenerek verilebilecek genişlikler (12 piksellik gövde metninde).
const WIDTHS: RangeInclusive<f32> = 240.0..=720.0;

/// Genişletilirken ana alana (haritaya) bırakılan en az yer.
const KEEP: f32 = 420.0;

/// Panelleri alt alta dizen yan yuva. Paneller arasına bölücü çizgi konur;
/// [`resizable`](Dock::resizable) verilirse sol kenarı sürüklenerek
/// genişletilir.
pub struct Dock<'a, Message> {
    panels: Vec<Panel<'a, Message>>,
    width: f32,
    resize: Option<Resize<'a, Message>>,
}

struct Resize<'a, Message> {
    default: f32,
    on_resize: Box<dyn Fn(f32) -> Message + 'a>,
    on_end: Option<Message>,
}

impl<'a, Message: Clone + 'a> Dock<'a, Message> {
    /// `width` 12 piksellik gövde metnine göredir; yazı boyutuyla büyür.
    pub fn new(width: f32) -> Self {
        Self {
            panels: Vec::new(),
            width,
            resize: None,
        }
    }

    pub fn push(mut self, panel: Panel<'a, Message>) -> Self {
        self.panels.push(panel);
        self
    }

    /// Sol kenar sürüklenerek genişletilir; yeni genişlik `on_resize` ile,
    /// [`new`](Dock::new)'deki gibi 12 piksellik gövde metnine göre bildirilir.
    /// Kenara çift tıklamak `default` genişliğe döndürür.
    pub fn resizable(mut self, default: f32, on_resize: impl Fn(f32) -> Message + 'a) -> Self {
        self.resize = Some(Resize {
            default,
            on_resize: Box::new(on_resize),
            on_end: None,
        });
        self
    }

    /// Sürükleme bitince (ve çift tıkla sıfırlanınca) gönderilen mesaj; ör.
    /// genişliği saklamak için. [`resizable`](Dock::resizable) ile birlikte
    /// kullanılır.
    pub fn on_resize_end(mut self, message: Message) -> Self {
        if let Some(resize) = &mut self.resize {
            resize.on_end = Some(message);
        }
        self
    }
}

impl<'a, Message: Clone + 'a> From<Dock<'a, Message>> for Element<'a, Message> {
    fn from(dock: Dock<'a, Message>) -> Self {
        let mut column = Column::new().height(Fill);

        for (index, panel) in dock.panels.into_iter().enumerate() {
            if index > 0 {
                column = column.push(horizontal_divider());
            }

            column = column.push(panel);
        }

        let width = typography::scaled(dock.width);
        let body = container(column)
            .width(width)
            .height(Fill)
            .style(style::container::surface);

        let Some(resize) = dock.resize else {
            return body.into();
        };

        let on_resize = resize.on_resize;
        let reset = on_resize(resize.default);
        let mut sash = Sash::vertical(width, move |px| on_resize(typography::unscaled(px)))
            .reverse()
            .range(typography::scaled(*WIDTHS.start())..=typography::scaled(*WIDTHS.end()))
            .keep(KEEP)
            .on_double_click(reset);

        if let Some(end) = resize.on_end {
            sash = sash.on_release(end);
        }

        row![sash, body].height(Fill).into()
    }
}

/// Başlık çubuğu ve gövdeden oluşan panel.
pub struct Panel<'a, Message> {
    title: Fragment<'a>,
    meta: Option<Fragment<'a>>,
    trailing: Option<Element<'a, Message>>,
    body: Element<'a, Message>,
    height: Length,
    scrollable: bool,
    collapse: Option<(bool, Message)>,
}

impl<'a, Message: Clone + 'a> Panel<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>, body: impl Into<Element<'a, Message>>) -> Self {
        Self {
            title: title.into_fragment(),
            meta: None,
            trailing: None,
            body: body.into(),
            height: Length::Shrink,
            scrollable: false,
            collapse: None,
        }
    }

    /// Başlık çubuğunun sağındaki meta bilgisi (ör. "6 katman").
    pub fn meta(mut self, meta: impl IntoFragment<'a>) -> Self {
        self.meta = Some(meta.into_fragment());
        self
    }

    /// Başlık çubuğunun sağ ucundaki öğe (ör. bir eylem düğmesi); meta
    /// bilgisinin sağında yer alır.
    pub fn trailing(mut self, trailing: impl Into<Element<'a, Message>>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }

    /// Gövdenin yüksekliği; birden çok esnek paneli oranlamak için
    /// `FillPortion` kullanılabilir. Sabit yükseklik yazı boyutuyla büyür.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Gövdeyi ince kaydırma çubuğuyla kaydırılabilir yapar.
    pub fn scrollable(mut self) -> Self {
        self.scrollable = true;
        self
    }

    /// Başlığa tıklanınca panel açılır ya da kapanır; başlığın solundaki ok
    /// durumu gösterir. Kapalı panelde yalnızca başlık kalır, boşalan yeri
    /// açık paneller doldurur.
    pub fn collapsible(mut self, collapsed: bool, on_toggle: Message) -> Self {
        self.collapse = Some((collapsed, on_toggle));
        self
    }
}

impl<'a, Message: Clone + 'a> From<Panel<'a, Message>> for Element<'a, Message> {
    fn from(panel: Panel<'a, Message>) -> Self {
        let collapsed = panel
            .collapse
            .as_ref()
            .is_some_and(|(collapsed, _)| *collapsed);

        let mut heading = row![].spacing(6).align_y(Center);

        if panel.collapse.is_some() {
            heading = heading.push(
                icon(if collapsed {
                    Icon::ChevronRight
                } else {
                    Icon::ChevronDown
                })
                .size(12.0)
                .tone(Tone::Muted),
            );
        }

        heading = heading
            .push(label::strong(panel.title))
            .push(space::horizontal());

        if let Some(meta) = panel.meta {
            heading = heading.push(label::caption(meta));
        }

        // Açılıp kapanan panelde başlığın tamamı düğmedir; sağdaki eylemler
        // düğmenin dışında kalır ve paneli açıp kapatmaz.
        let heading: Element<'a, Message> = match panel.collapse {
            Some((_, on_toggle)) => button(heading)
                .on_press(on_toggle)
                .width(Fill)
                .padding([5, 10])
                .style(style::button::panel_header)
                .into(),
            None => container(heading).width(Fill).padding([5, 10]).into(),
        };

        let mut header = row![heading].align_y(Center);

        if let Some(trailing) = panel.trailing {
            header = header.push(container(trailing).padding(iced::padding::right(8)));
        }

        let header = container(header)
            .width(Fill)
            .style(style::container::header);

        if collapsed {
            return header.into();
        }

        // Esnek yükseklikli panelde gövde, başlıktan kalan alanı doldurur.
        let body_height = if panel.height == Length::Shrink {
            Length::Shrink
        } else {
            Length::Fill
        };

        // Kaydırma çubuğu içeriğin üstüne binmez; yalnızca gerektiğinde yer
        // kaplar.
        let body: Element<'a, Message> = if panel.scrollable {
            scrollable(panel.body)
                .direction(style::field::thin_scrollbar())
                .spacing(0)
                .width(Fill)
                .height(body_height)
                .into()
        } else {
            container(panel.body).width(Fill).height(body_height).into()
        };

        Column::new()
            .push(header)
            .push(body)
            .height(typography::length(panel.height))
            .into()
    }
}
