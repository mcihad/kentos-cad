//! İletişim kutusu ve kısayol listesi.
//!
//! [`Dialog`] yalnızca kutunun kendisidir; ortalamak ve arkasını karartmak
//! için [`overlay::modal`](crate::widget::overlay::modal) ile gösterilir.

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, column, container, row, space};
use iced::{Center, Element};

use crate::label;
use crate::style;
use crate::widget::horizontal_divider;

/// Başlık, gövde ve sağa hizalı eylem düğmelerinden oluşan kutu.
pub struct Dialog<'a, Message> {
    title: Fragment<'a>,
    hint: Option<Fragment<'a>>,
    body: Vec<Element<'a, Message>>,
    actions: Vec<Element<'a, Message>>,
    width: f32,
}

impl<'a, Message: 'a> Dialog<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            hint: None,
            body: Vec::new(),
            actions: Vec::new(),
            width: 500.0,
        }
    }

    /// Başlığın sağındaki kısa not (ör. kutuyu açan kısayol).
    pub fn hint(mut self, hint: impl IntoFragment<'a>) -> Self {
        self.hint = Some(hint.into_fragment());
        self
    }

    pub fn push(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.body.push(content.into());
        self
    }

    pub fn action(mut self, action: impl Into<Element<'a, Message>>) -> Self {
        self.actions.push(action.into());
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

impl<'a, Message: 'a> From<Dialog<'a, Message>> for Element<'a, Message> {
    fn from(dialog: Dialog<'a, Message>) -> Self {
        let mut header = row![label::title(dialog.title), space::horizontal()].align_y(Center);

        if let Some(hint) = dialog.hint {
            header = header.push(label::mono_caption(hint));
        }

        let mut content = column![header, horizontal_divider()].spacing(12);

        for part in dialog.body {
            content = content.push(part);
        }

        if !dialog.actions.is_empty() {
            let mut actions = row![space::horizontal()].spacing(6);

            for action in dialog.actions {
                actions = actions.push(action);
            }

            content = content.push(actions);
        }

        container(content)
            .width(dialog.width)
            .padding(18)
            .style(style::container::popover)
            .into()
    }
}

/// Kısayol ve açıklamalarını iki sütunda listeler.
pub struct ShortcutList<'a> {
    items: Vec<(Fragment<'a>, Fragment<'a>)>,
    key_width: f32,
}

impl<'a> ShortcutList<'a> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            key_width: 160.0,
        }
    }

    pub fn item(mut self, keys: impl IntoFragment<'a>, description: impl IntoFragment<'a>) -> Self {
        self.items
            .push((keys.into_fragment(), description.into_fragment()));
        self
    }
}

impl<'a> Default for ShortcutList<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<ShortcutList<'a>> for Element<'a, Message> {
    fn from(list: ShortcutList<'a>) -> Self {
        Column::with_children(list.items.into_iter().map(|(keys, description)| {
            row![
                label::mono(keys).width(list.key_width),
                label::muted(description),
            ]
            .spacing(12)
            .into()
        }))
        .spacing(7)
        .into()
    }
}
