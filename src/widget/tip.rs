//! Zengin ipuçları.

use iced::Element;
use iced::widget::{column, container, tooltip};

use crate::label;
use crate::style;

/// İpucu içeriği: başlık, isteğe bağlı açıklama ve eş aralıklı bir ayrıntı
/// satırı (ör. komut satırı karşılığı).
///
/// Yalnızca başlığı olan ipucu tek satırlık sade bir etiket olarak gösterilir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tip {
    title: String,
    body: Option<String>,
    detail: Option<String>,
}

impl Tip {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: None,
            detail: None,
        }
    }

    /// Ne yaptığını anlatan açıklama.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Eş aralıklı ayrıntı satırı (ör. "Komut: CIZGI").
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    fn view<'a, Message: 'a>(self) -> Element<'a, Message> {
        if self.body.is_none() && self.detail.is_none() {
            return container(label::caption(self.title).style(style::text::default))
                .padding([4, 8])
                .style(style::container::popover)
                .into();
        }

        let mut content = column![label::strong(self.title)].spacing(3).max_width(260);

        if let Some(body) = self.body {
            content = content.push(label::caption(body));
        }

        if let Some(detail) = self.detail {
            content = content.push(label::mono_caption(detail));
        }

        container(content)
            .padding([7, 10])
            .style(style::container::popover)
            .into()
    }
}

impl From<String> for Tip {
    fn from(title: String) -> Self {
        Self::new(title)
    }
}

impl From<&str> for Tip {
    fn from(title: &str) -> Self {
        Self::new(title)
    }
}

/// `content`'e ipucu ekler.
pub fn tip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    tip: Tip,
    position: tooltip::Position,
) -> Element<'a, Message> {
    tooltip(content, tip.view(), position).gap(5.0).into()
}
