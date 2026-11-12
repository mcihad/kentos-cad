//! Liste ve ayrıntı: solda bir listenin bölmesi, sağda listede seçilen öğenin ayrıntısı, yan yana; çok alanlı ayrıntı
//! ad–değer ızgarasıdır, eylemler gösterdikleri şeyin yanındadır (kaynaklar: docs/adr/8050 §6).
//!
//! ```text
//! ┌ liste ───────────┐│┌ ayrıntı ─────────────────────────────┐
//! │ ▣ Banyo        3 ││ │ Banyo                                │
//! │ ▣ Mutfak       1 ││ │ Sürüm      2                         │  ← ad–değer çiftleri (Pairs)
//! │                  ││ │ Nesneler   9                         │
//! │   (kayar)        ││ │ [öğenin eylemleri]                   │  ← işledikleri şeyin yanında
//! ├──────────────────┤│ │ …                (kayar)             │
//! │ listenin eylemi  ││ │                                      │
//! └──────────────────┘│└──────────────────────────────────────┘
//! ```
//!
//! - Liste bölmesi sabit genişliktedir ([`ListDetails::list_width`], öntanımlı 240 piksel) ve kendi içinde kayar;
//!   altındaki listenin kendi eylemi (yeni öğe) hep görünür ([`ListDetails::list_foot`]).
//! - Ayrıntı bölmesi kalan genişliği alır, kendi içinde kayar. Öğenin eylemleri ayrıntının içinde, işledikleri şeyin
//!   yanındadır; pencerenin alt çubuğunda pencerenin kendi eylemleri durur (DESIGN.md §7.9).
//! - İki bölmeyi ince dikey çizgi ayırır. Bileşenin yüksekliği sabittir ([`ListDetails::height`], öntanımlı 420
//!   piksel): seçim değişince pencere zıplamaz, kısa içerik pencerenin dibinde boşluk bırakmaz (sekmeli pencerelerin
//!   kuralı, DESIGN.md §7.9).
//!
//! Ölçüler 12 piksellik gövde metnine göredir, yazı boyutuyla büyür ([`typography::scaled`]).
//!
//! ```ignore
//! ListDetails::new(modules, details)
//!     .list_foot(new_module_form)
//!     .list_width(240.0)
//!     .height(420.0)
//! ```

use iced::widget::{Column, container, row, scrollable};
use iced::{Element, Fill};

use crate::style;
use crate::theme::typography;
use crate::widget::vertical_divider;

/// Liste ve ayrıntı (bkz. modül).
pub struct ListDetails<'a, Message> {
    list: Element<'a, Message>,
    list_foot: Option<Element<'a, Message>>,
    details: Element<'a, Message>,
    list_width: f32,
    height: f32,
}

impl<'a, Message: 'a> ListDetails<'a, Message> {
    /// Solda `list`, sağda `details`.
    pub fn new(
        list: impl Into<Element<'a, Message>>,
        details: impl Into<Element<'a, Message>>,
    ) -> Self {
        Self {
            list: list.into(),
            list_foot: None,
            details: details.into(),
            list_width: 240.0,
            height: 420.0,
        }
    }

    /// Listenin altında, kaymadan duran parça: listenin kendi eylemi (ör. seçimden yeni öğe).
    pub fn list_foot(mut self, foot: impl Into<Element<'a, Message>>) -> Self {
        self.list_foot = Some(foot.into());
        self
    }

    /// Liste bölmesinin genişliği, 12 piksellik gövde metnine göre.
    pub fn list_width(mut self, width: f32) -> Self {
        self.list_width = width;
        self
    }

    /// Bileşenin yüksekliği, 12 piksellik gövde metnine göre.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl<'a, Message: 'a> From<ListDetails<'a, Message>> for Element<'a, Message> {
    fn from(l: ListDetails<'a, Message>) -> Self {
        let mut left = Column::new().spacing(typography::scaled(10.0)).push(
            scrollable(l.list)
                .direction(style::field::body_scrollbar())
                .height(Fill),
        );
        if let Some(foot) = l.list_foot {
            left = left.push(foot);
        }
        let right = scrollable(l.details)
            .direction(style::field::body_scrollbar())
            .height(Fill);
        row![
            container(left)
                .width(typography::scaled(l.list_width))
                .height(Fill),
            vertical_divider(),
            container(right).width(Fill).height(Fill),
        ]
        .spacing(typography::scaled(16.0))
        .height(typography::scaled(l.height))
        .into()
    }
}
