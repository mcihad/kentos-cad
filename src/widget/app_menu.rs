//! Uygulama menüsü: Office'teki "Dosya" menüsü gibi, şeridin marka
//! düğmesinden açılan büyük menü.
//!
//! ```text
//! ┌─────────────────────────┬──────────────────────────────┐
//! │ ▢  Yeni                 │ Son kullanılanlar            │
//! │    Görünümü sıfırlar    │ Bir çizimi açmak için...     │
//! │ ▢  Dışa aktar         › │ ──────────────────────────── │
//! │    PDF, PNG, DXF        │ ▢ Ankara imar planı     Dün  │
//! ├─────────────────────────┴──────────────────────────────┤
//! │ KentOS CAD 0.1               [◐ Aydınlık tema] [⏻ Çık] │
//! └────────────────────────────────────────────────────────┘
//! ```
//!
//! Solda büyük komutlar ([`Entry`]), sağda ayrıntı bölmesi ([`Pane`]) ve
//! altta eylemler ([`Action`]) bulunur. Alt menüsü olan komutun üzerine
//! gelmek, uygulamanın ayrıntı bölmesini değiştirmesi için `on_hover`
//! mesajını üretir.

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, button, column, container, mouse_area, row, space};
use iced::{Center, Element, Fill, Point};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::widget::{horizontal_divider, overlay, ribbon, vertical_divider};

const WIDTH: f32 = 680.0;
const COMMAND_COLUMN: f32 = 290.0;
const ENTRY_HEIGHT: f32 = 50.0;
const ENTRY_GAP: f32 = 2.0;
const PADDING: f32 = 6.0;

/// Uygulama menüsü.
pub struct AppMenu<'a, Message> {
    entries: Vec<Entry<'a, Message>>,
    detail: Option<Element<'a, Message>>,
    note: Option<Fragment<'a>>,
    actions: Vec<Action<'a, Message>>,
    on_dismiss: Message,
}

impl<'a, Message: Clone + 'a> AppMenu<'a, Message> {
    /// Menünün dışına tıklamak `on_dismiss` mesajını üretir.
    pub fn new(on_dismiss: Message) -> Self {
        Self {
            entries: Vec::new(),
            detail: None,
            note: None,
            actions: Vec::new(),
            on_dismiss,
        }
    }

    pub fn entry(mut self, entry: Entry<'a, Message>) -> Self {
        self.entries.push(entry);
        self
    }

    /// Sağdaki ayrıntı bölmesi (genellikle bir [`Pane`]).
    pub fn detail(mut self, detail: impl Into<Element<'a, Message>>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Alt çubuğun solundaki not (ör. sürüm).
    pub fn note(mut self, note: impl IntoFragment<'a>) -> Self {
        self.note = Some(note.into_fragment());
        self
    }

    /// Alt çubuğa eylem düğmesi ekler.
    pub fn action(mut self, action: Action<'a, Message>) -> Self {
        self.actions.push(action);
        self
    }
}

impl<'a, Message: Clone + 'a> From<AppMenu<'a, Message>> for Element<'a, Message> {
    fn from(menu: AppMenu<'a, Message>) -> Self {
        let entry_count = menu.entries.len().max(1) as f32;
        let body_height =
            ENTRY_HEIGHT * entry_count + ENTRY_GAP * (entry_count - 1.0) + PADDING * 2.0;

        let entries =
            Column::with_children(menu.entries.into_iter().map(Element::from)).spacing(ENTRY_GAP);

        let body = row![
            container(entries)
                .width(COMMAND_COLUMN)
                .height(Fill)
                .padding(PADDING)
                .style(style::container::surface),
            vertical_divider(),
            container(menu.detail.unwrap_or_else(|| space::horizontal().into()))
                .width(Fill)
                .height(Fill)
                .padding([12, 14])
                .style(style::container::surface_alt),
        ]
        .height(body_height);

        let mut footer = iced::widget::Row::new().spacing(6).align_y(Center);

        if let Some(note) = menu.note {
            footer = footer.push(label::caption(note));
        }

        footer = footer.push(space::horizontal());

        for action in menu.actions {
            footer = footer.push(action);
        }

        let panel = container(column![
            body,
            horizontal_divider(),
            container(footer)
                .padding([8, 10])
                .width(Fill)
                .style(style::container::surface),
        ])
        .width(WIDTH)
        .style(style::container::popover);

        overlay::popover(
            panel,
            Point::new(0.0, ribbon::STRIP_HEIGHT),
            menu.on_dismiss,
        )
    }
}

/// Menünün solundaki büyük komut: ikon, başlık ve tek satırlık açıklama.
pub struct Entry<'a, Message> {
    icon: Icon,
    title: Fragment<'a>,
    description: Fragment<'a>,
    on_press: Option<Message>,
    on_hover: Option<Message>,
    submenu: Option<bool>,
}

impl<'a, Message: Clone + 'a> Entry<'a, Message> {
    pub fn new(
        icon: Icon,
        title: impl IntoFragment<'a>,
        description: impl IntoFragment<'a>,
    ) -> Self {
        Self {
            icon,
            title: title.into_fragment(),
            description: description.into_fragment(),
            on_press: None,
            on_hover: None,
            submenu: None,
        }
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// İmleç komutun üzerine geldiğinde üretilecek mesaj.
    pub fn on_hover(mut self, message: Message) -> Self {
        self.on_hover = Some(message);
        self
    }

    /// Komutun bir alt menüsü olduğunu gösterir (sağda ok). `expanded`
    /// ise komut vurgulanır; ayrıntı bölmesi onun alt menüsünü gösterir.
    pub fn submenu(mut self, expanded: bool) -> Self {
        self.submenu = Some(expanded);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Entry<'a, Message>> for Element<'a, Message> {
    fn from(entry: Entry<'a, Message>) -> Self {
        let expanded = entry.submenu == Some(true);

        let trailing: Element<'a, Message> = if entry.submenu.is_some() {
            icon(Icon::ChevronRight).size(12.0).tone(Tone::Muted).into()
        } else {
            space::horizontal().width(12).into()
        };

        let content = button(
            row![
                icon(entry.icon).size(24.0).tone(if expanded {
                    Tone::Accent
                } else {
                    Tone::Inherit
                }),
                column![
                    label::heading(entry.title),
                    label::caption(entry.description)
                ]
                .spacing(1)
                .width(Fill),
                trailing,
            ]
            .spacing(12)
            .height(Fill)
            .align_y(Center),
        )
        .on_press_maybe(entry.on_press)
        .width(Fill)
        .height(ENTRY_HEIGHT)
        .padding([0, 10])
        .style(style::button::list_item(expanded));

        match entry.on_hover {
            Some(on_hover) => mouse_area(content).on_enter(on_hover).into(),
            None => content.into(),
        }
    }
}

/// Ayrıntı bölmesi: başlık, alt başlık ve öğe listesi.
pub struct Pane<'a, Message> {
    title: Fragment<'a>,
    subtitle: Fragment<'a>,
    items: Vec<Element<'a, Message>>,
}

impl<'a, Message: Clone + 'a> Pane<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>, subtitle: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            subtitle: subtitle.into_fragment(),
            items: Vec::new(),
        }
    }

    pub fn push(mut self, item: impl Into<Element<'a, Message>>) -> Self {
        self.items.push(item.into());
        self
    }
}

impl<'a, Message: Clone + 'a> From<Pane<'a, Message>> for Element<'a, Message> {
    fn from(pane: Pane<'a, Message>) -> Self {
        column![
            column![label::heading(pane.title), label::caption(pane.subtitle)].spacing(2),
            horizontal_divider(),
            Column::with_children(pane.items).spacing(1),
        ]
        .spacing(8)
        .into()
    }
}

/// Ayrıntı bölmesindeki bir satır: öndeki işaret, başlık, alt başlık ve
/// sondaki bilgi.
pub struct Item<'a, Message> {
    title: Fragment<'a>,
    subtitle: Option<Fragment<'a>>,
    leading: Option<Element<'a, Message>>,
    trailing: Option<Element<'a, Message>>,
    on_press: Option<Message>,
}

impl<'a, Message: Clone + 'a> Item<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            subtitle: None,
            leading: None,
            trailing: None,
            on_press: None,
        }
    }

    pub fn subtitle(mut self, subtitle: impl IntoFragment<'a>) -> Self {
        self.subtitle = Some(subtitle.into_fragment());
        self
    }

    pub fn leading(mut self, leading: impl Into<Element<'a, Message>>) -> Self {
        self.leading = Some(leading.into());
        self
    }

    pub fn trailing(mut self, trailing: impl Into<Element<'a, Message>>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Item<'a, Message>> for Element<'a, Message> {
    fn from(item: Item<'a, Message>) -> Self {
        let mut text = column![label::body(item.title)].spacing(1).width(Fill);

        if let Some(subtitle) = item.subtitle {
            text = text.push(label::caption(subtitle));
        }

        let mut content = iced::widget::Row::new().spacing(10).align_y(Center);

        if let Some(leading) = item.leading {
            content = content.push(leading);
        }

        content = content.push(text);

        if let Some(trailing) = item.trailing {
            content = content.push(trailing);
        }

        button(content)
            .on_press_maybe(item.on_press)
            .width(Fill)
            .padding([6, 8])
            .style(style::button::list_item(false))
            .into()
    }
}

/// Alt çubuktaki eylem düğmesi.
pub struct Action<'a, Message> {
    icon: Icon,
    label: Fragment<'a>,
    on_press: Message,
    primary: bool,
}

impl<'a, Message: Clone + 'a> Action<'a, Message> {
    /// Vurgulu eylem (ör. uygulamadan çıkış).
    pub fn primary(icon: Icon, label: impl IntoFragment<'a>, on_press: Message) -> Self {
        Self {
            icon,
            label: label.into_fragment(),
            on_press,
            primary: true,
        }
    }

    /// Kenarlı ikincil eylem.
    pub fn secondary(icon: Icon, label: impl IntoFragment<'a>, on_press: Message) -> Self {
        Self {
            primary: false,
            ..Self::primary(icon, label, on_press)
        }
    }
}

impl<'a, Message: Clone + 'a> From<Action<'a, Message>> for Element<'a, Message> {
    fn from(action: Action<'a, Message>) -> Self {
        let content = button(
            row![icon(action.icon).size(14.0), label::body(action.label)]
                .spacing(7)
                .align_y(Center),
        )
        .on_press(action.on_press)
        .padding([5, 12]);

        if action.primary {
            content.style(style::button::primary).into()
        } else {
            content.style(style::button::secondary).into()
        }
    }
}
