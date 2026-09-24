//! Şerit grupları ve grup içi yerleşim yardımcıları.

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, column, container, row};
use iced::{Center, Element, Fill, Length, Padding, Top};

use super::{CAPTION, CONTENT, ICON, ROW, ROW_GAP};
use crate::label;

/// Başlıklı araç grubu. Öğeler yan yana dizilir; altta ortalanmış grup
/// adı bulunur.
pub struct Group<'a, Message> {
    title: Fragment<'a>,
    items: Vec<Element<'a, Message>>,
}

impl<'a, Message: 'a> Group<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            items: Vec::new(),
        }
    }

    /// Büyük düğme, [`Stack`] veya [`Row`] ekler.
    pub fn push(mut self, item: impl Into<Element<'a, Message>>) -> Self {
        self.items.push(item.into());
        self
    }
}

impl<'a, Message: 'a> From<Group<'a, Message>> for Element<'a, Message> {
    fn from(group: Group<'a, Message>) -> Self {
        column![
            container(iced::widget::Row::with_children(group.items).spacing(2))
                .height(CONTENT)
                .align_y(Top),
            container(label::caption(group.title))
                .height(CAPTION)
                .align_y(Center),
        ]
        .padding(Padding {
            top: super::PANEL_PADDING_TOP,
            right: 8.0,
            bottom: super::PANEL_PADDING_BOTTOM,
            left: 8.0,
        })
        .height(Fill)
        .align_x(Center)
        .into()
    }
}

/// Öğeleri ızgara satırlarına üstten hizalı, alt alta dizer. Üçten az
/// öğeli yığınlar da üst satırlara oturur; böylece gruplar arasında satırlar
/// hizalı kalır.
pub struct Stack<'a, Message> {
    items: Vec<Element<'a, Message>>,
    width: Length,
}

impl<'a, Message: 'a> Stack<'a, Message> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            width: Length::Shrink,
        }
    }

    /// Küçük düğme, [`Field`] veya [`Row`] ekler.
    pub fn push(mut self, item: impl Into<Element<'a, Message>>) -> Self {
        self.items.push(item.into());
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

impl<'a, Message: 'a> Default for Stack<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<Stack<'a, Message>> for Element<'a, Message> {
    fn from(stack: Stack<'a, Message>) -> Self {
        Column::with_children(stack.items)
            .spacing(ROW_GAP)
            .width(stack.width)
            .into()
    }
}

/// Bir ızgara satırında eşit genişlikli hücreler (ör. Göster, Gizle,
/// Sığdır).
pub struct Row<'a, Message> {
    items: Vec<Element<'a, Message>>,
}

impl<'a, Message: 'a> Row<'a, Message> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(mut self, item: impl Into<Element<'a, Message>>) -> Self {
        self.items.push(item.into());
        self
    }
}

impl<'a, Message: 'a> Default for Row<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<Row<'a, Message>> for Element<'a, Message> {
    fn from(cells: Row<'a, Message>) -> Self {
        iced::widget::Row::with_children(
            cells
                .items
                .into_iter()
                .map(|item| container(item).width(Fill).into()),
        )
        .height(ROW)
        .into()
    }
}

/// Bir ızgara satırında önde küçük bir işaret (ör. renk örneği) ve bir
/// kontrol (ör. açılır liste). İşaret küçük düğmelerin ikon sütununa,
/// kontrol ise etiket sütununa hizalanır.
pub struct Field<'a, Message> {
    leading: Element<'a, Message>,
    control: Element<'a, Message>,
}

impl<'a, Message: 'a> Field<'a, Message> {
    pub fn new(
        leading: impl Into<Element<'a, Message>>,
        control: impl Into<Element<'a, Message>>,
    ) -> Self {
        Self {
            leading: leading.into(),
            control: control.into(),
        }
    }
}

impl<'a, Message: 'a> From<Field<'a, Message>> for Element<'a, Message> {
    fn from(field: Field<'a, Message>) -> Self {
        row![
            container(field.leading).width(ICON).align_x(Center),
            field.control,
        ]
        .spacing(6)
        .padding([0, 6])
        .height(ROW)
        .align_y(Center)
        .into()
    }
}
