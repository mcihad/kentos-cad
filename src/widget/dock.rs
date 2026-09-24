//! Yan panel yuvası (dock) ve başlıklı paneller.

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Length};

use crate::label;
use crate::style;
use crate::widget::horizontal_divider;

/// Sabit genişlikli, panelleri alt alta dizen yuva. Paneller arasına
/// bölücü çizgi konur.
pub struct Dock<'a, Message> {
    panels: Vec<Panel<'a, Message>>,
    width: f32,
}

impl<'a, Message: 'a> Dock<'a, Message> {
    pub fn new(width: f32) -> Self {
        Self {
            panels: Vec::new(),
            width,
        }
    }

    pub fn push(mut self, panel: Panel<'a, Message>) -> Self {
        self.panels.push(panel);
        self
    }
}

impl<'a, Message: 'a> From<Dock<'a, Message>> for Element<'a, Message> {
    fn from(dock: Dock<'a, Message>) -> Self {
        let mut column = Column::new().height(Fill);

        for (index, panel) in dock.panels.into_iter().enumerate() {
            if index > 0 {
                column = column.push(horizontal_divider());
            }

            column = column.push(panel);
        }

        container(column)
            .width(dock.width)
            .height(Fill)
            .style(style::container::surface)
            .into()
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
}

impl<'a, Message: 'a> Panel<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>, body: impl Into<Element<'a, Message>>) -> Self {
        Self {
            title: title.into_fragment(),
            meta: None,
            trailing: None,
            body: body.into(),
            height: Length::Shrink,
            scrollable: false,
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
    /// `FillPortion` kullanılabilir.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Gövdeyi ince kaydırma çubuğuyla kaydırılabilir yapar.
    pub fn scrollable(mut self) -> Self {
        self.scrollable = true;
        self
    }
}

impl<'a, Message: 'a> From<Panel<'a, Message>> for Element<'a, Message> {
    fn from(panel: Panel<'a, Message>) -> Self {
        let mut header_row = row![label::strong(panel.title), space::horizontal()]
            .spacing(8)
            .align_y(Center);

        if let Some(meta) = panel.meta {
            header_row = header_row.push(label::caption(meta));
        }

        if let Some(trailing) = panel.trailing {
            header_row = header_row.push(trailing);
        }

        let header = container(header_row)
            .padding([5, 10])
            .width(Fill)
            .style(style::container::header);

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
            .height(panel.height)
            .into()
    }
}
