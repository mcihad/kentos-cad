//! Solunda bölüm listesi olan özellikler penceresi (QGIS'in katman
//! özellikleri gibi).
//!
//! ```text
//! ┌ Katman özellikleri ─────────────────────────────── Şehirler, nokta ┐
//! ├────────────────┬─────────────────────────────────────────────────────┤
//! │ ⓘ Genel        │ Sembolizasyon                                        │
//! │ ⛁ Kaynak       │ Katmanın rengi, opaklığı ve alt katmanları.          │
//! │▌◧ Sembolizasyon│ ───────────────────────────────────────              │
//! │ T Etiketler    │ bölümün içeriği (kaydırılır)                         │
//! │ ▦ Alanlar      │                                                      │
//! ├────────────────┴─────────────────────────────────────────────────────┤
//! │ ● Kaydedilmemiş değişiklikler var         [İptal] [Uygula] [Tamam]  │
//! └──────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Değişiklikler uygulamanın taslağında tutulur: "Uygula" taslağı katmana
//! yazar ve pencere açık kalır, "Tamam" yazar ve kapatır, "İptal" taslağı
//! atar. Taslak katmandan farklıyken ([`dirty`](PropertiesDialog::dirty))
//! "Uygula" etkindir ve altta bir not görünür. Pencere
//! [`overlay::blocking`](crate::widget::overlay::blocking) ile gösterilir:
//! arkasına tıklamak taslağı atmaz.
//!
//! ```ignore
//! PropertiesDialog::new("Katman özellikleri")
//!     .subtitle("Şehirler, nokta katmanı")
//!     .section(Icon::Info, "Genel", section == Section::General, Message::Section(Section::General))
//!     .section(Icon::Drop, "Sembolizasyon", section == Section::Style, Message::Section(Section::Style))
//!     .body("Sembolizasyon", self.style_section())
//!     .description("Katmanın rengi ve opaklığı.")
//!     .dirty(draft != layer)
//!     .on_apply(Message::Apply)
//!     .on_accept(Message::Accept)
//!     .on_cancel(Message::Cancel)
//! ```

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, button, column, container, row, scrollable, space};
use iced::{Background, Center, Element, Fill, Theme};

use crate::icon::{Icon, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::{horizontal_divider, vertical_divider};

/// Özellikler penceresi.
pub struct PropertiesDialog<'a, Message> {
    title: Fragment<'a>,
    subtitle: Option<Fragment<'a>>,
    sections: Vec<Section<'a, Message>>,
    heading: Option<Fragment<'a>>,
    description: Option<Fragment<'a>>,
    body: Option<Element<'a, Message>>,
    dirty: bool,
    on_apply: Option<Message>,
    on_accept: Option<Message>,
    on_cancel: Option<Message>,
    width: f32,
    height: f32,
}

struct Section<'a, Message> {
    icon: Icon,
    label: Fragment<'a>,
    selected: bool,
    on_select: Message,
}

impl<'a, Message: Clone + 'a> PropertiesDialog<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            subtitle: None,
            sections: Vec::new(),
            heading: None,
            description: None,
            body: None,
            dirty: false,
            on_apply: None,
            on_accept: None,
            on_cancel: None,
            width: 760.0,
            height: 420.0,
        }
    }

    /// Başlığın sağındaki not (ör. katmanın adı ve türü).
    pub fn subtitle(mut self, subtitle: impl IntoFragment<'a>) -> Self {
        self.subtitle = Some(subtitle.into_fragment());
        self
    }

    /// Soldaki listede bir bölüm; seçili bölüm vurgulanır.
    pub fn section(
        mut self,
        icon: Icon,
        label: impl IntoFragment<'a>,
        selected: bool,
        on_select: Message,
    ) -> Self {
        self.sections.push(Section {
            icon,
            label: label.into_fragment(),
            selected,
            on_select,
        });
        self
    }

    /// Seçili bölümün başlığı ve içeriği; içerik sığmazsa kaydırılır.
    pub fn body(
        mut self,
        heading: impl IntoFragment<'a>,
        body: impl Into<Element<'a, Message>>,
    ) -> Self {
        self.heading = Some(heading.into_fragment());
        self.body = Some(body.into());
        self
    }

    /// Bölüm başlığının altındaki kısa açıklama.
    pub fn description(mut self, description: impl IntoFragment<'a>) -> Self {
        self.description = Some(description.into_fragment());
        self
    }

    /// Taslak katmandan farklı: "Uygula" etkin, altta not görünür.
    pub fn dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }

    /// Taslağı yazar; pencere açık kalır.
    pub fn on_apply(mut self, message: Message) -> Self {
        self.on_apply = Some(message);
        self
    }

    /// Taslağı yazar ve pencereyi kapatır.
    pub fn on_accept(mut self, message: Message) -> Self {
        self.on_accept = Some(message);
        self
    }

    /// Taslağı atar ve pencereyi kapatır.
    pub fn on_cancel(mut self, message: Message) -> Self {
        self.on_cancel = Some(message);
        self
    }

    /// Pencerenin genişliği ve içerik alanının yüksekliği, 12 piksellik
    /// gövde metnine göre.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

impl<'a, Message: Clone + 'a> From<PropertiesDialog<'a, Message>> for Element<'a, Message> {
    fn from(dialog: PropertiesDialog<'a, Message>) -> Self {
        let mut header = row![label::title(dialog.title), space::horizontal()].align_y(Center);

        if let Some(subtitle) = dialog.subtitle {
            header = header.push(label::muted(subtitle));
        }

        let navigation = Column::with_children(
            dialog
                .sections
                .into_iter()
                .map(|section| navigation_item(section)),
        )
        .spacing(2)
        .padding(6)
        .width(typography::scaled(184.0));

        let mut content = Column::new().spacing(12).padding([14, 18]).width(Fill);

        if let Some(heading) = dialog.heading {
            let mut top = column![label::heading(heading)].spacing(3);

            if let Some(description) = dialog.description {
                top = top.push(label::muted(description));
            }

            content = content.push(top).push(horizontal_divider());
        }

        if let Some(body) = dialog.body {
            content = content.push(body);
        }

        let middle = row![
            container(navigation)
                .height(Fill)
                .style(style::container::surface_alt),
            vertical_divider(),
            scrollable(content)
                .direction(style::field::thin_scrollbar())
                .width(Fill)
                .height(Fill),
        ]
        .height(typography::scaled(dialog.height));

        let note: Element<'a, Message> = if dialog.dirty {
            row![
                container(space::horizontal().width(0))
                    .center_x(8)
                    .center_y(8)
                    .style(|theme: &Theme| container::Style {
                        background: Some(Background::Color(Tokens::of(theme).accent)),
                        border: iced::border::rounded(4.0),
                        ..container::Style::default()
                    }),
                label::muted("Kaydedilmemiş değişiklikler var"),
            ]
            .spacing(8)
            .align_y(Center)
            .width(Fill)
            .into()
        } else {
            space::horizontal().into()
        };

        let footer = row![
            note,
            button(label::body("İptal"))
                .on_press_maybe(dialog.on_cancel)
                .padding([5, 16])
                .style(style::button::secondary),
            button(label::body("Uygula"))
                .on_press_maybe(dialog.on_apply.filter(|_| dialog.dirty))
                .padding([5, 16])
                .style(style::button::secondary),
            button(label::body("Tamam"))
                .on_press_maybe(dialog.on_accept)
                .padding([5, 16])
                .style(style::button::primary),
        ]
        .spacing(6)
        .align_y(Center);

        container(
            column![
                container(header).padding([14, 18]),
                horizontal_divider(),
                middle,
                horizontal_divider(),
                container(footer).padding([12, 18]),
            ]
            .width(typography::scaled(dialog.width)),
        )
        .style(style::container::popover)
        .into()
    }
}

/// Soldaki listenin satırı: seçili satırın solunda vurgu çizgisi.
fn navigation_item<'a, Message: Clone + 'a>(section: Section<'a, Message>) -> Element<'a, Message> {
    let selected = section.selected;

    let marker = container(space::vertical().height(0))
        .width(2)
        .height(16)
        .style(move |theme: &Theme| container::Style {
            background: selected.then(|| Background::Color(Tokens::of(theme).accent)),
            border: iced::border::rounded(1.0),
            ..container::Style::default()
        });

    let glyph = icon(section.icon).size(14.0).tone(if selected {
        crate::icon::Tone::Accent
    } else {
        crate::icon::Tone::Muted
    });

    button(
        row![
            marker,
            glyph,
            if selected {
                label::strong(section.label)
            } else {
                label::body(section.label)
            },
        ]
        .spacing(8)
        .align_y(Center),
    )
    .on_press(section.on_select)
    .width(Fill)
    .padding([6, 8])
    .style(style::button::navigation(selected))
    .into()
}
