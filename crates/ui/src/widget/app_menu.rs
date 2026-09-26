//! Uygulama menüsü: Office'teki "Dosya" menüsü gibi, şeridin marka
//! düğmesinden açılan büyük menü (DESIGN.md §7.1.1).
//!
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │ ▣ KentOS CAD  Örnek pafta •                            │
//! ├─────────────────────────┬──────────────────────────────┤
//! │ ▢  Yeni          Ctrl+N │ Son kullanılanlar            │
//! │    Görünümü sıfırlar    │ Bir çizimi açmak için...     │
//! │ ▢  Dışa aktar         › │ ──────────────────────────── │
//! │    PDF, PNG, DXF        │ ▢ Ankara imar planı     Dün  │
//! ├─────────────────────────┴──────────────────────────────┤
//! │ [⚙ Ayarlar] [? Kısayollar]                  KentOS CAD │
//! └────────────────────────────────────────────────────────┘
//! ```
//!
//! Üstte isteğe bağlı başlık, solda büyük komutlar ([`Entry`]), sağda
//! ayrıntı bölmesi ([`Pane`]) ve altta eylemler ([`Action`]) bulunur. Alt
//! menüsü olan komutun üzerine gelmek, uygulamanın ayrıntı bölmesini
//! değiştirmesi için `on_hover` mesajını üretir. Klavyeyle gezinmeyi
//! uygulama yürütür: seçili satırı [`Entry::current`] ve
//! [`Item::current`] gösterir.

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, button, column, container, mouse_area, row, space};
use iced::{Center, Element, Fill, Point};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::typography;
use crate::widget::{horizontal_divider, overlay, ribbon, vertical_divider};

const WIDTH: f32 = 680.0;
const COMMAND_COLUMN: f32 = 290.0;
const ENTRY_HEIGHT: f32 = 50.0;
const ENTRY_GAP: f32 = 2.0;
const PADDING: f32 = 6.0;
/// Komut simgesinin zemini (DESIGN.md §7.1.1: 34 px).
const ICON_TILE: f32 = 34.0;

/// Uygulama menüsü.
pub struct AppMenu<'a, Message> {
    header: Option<Element<'a, Message>>,
    entries: Vec<Entry<'a, Message>>,
    detail: Option<Element<'a, Message>>,
    note: Option<Fragment<'a>>,
    actions: Vec<Action<'a, Message>>,
    width: f32,
    on_dismiss: Message,
}

impl<'a, Message: Clone + 'a> AppMenu<'a, Message> {
    /// Menünün dışına tıklamak `on_dismiss` mesajını üretir.
    pub fn new(on_dismiss: Message) -> Self {
        Self {
            header: None,
            entries: Vec::new(),
            detail: None,
            note: None,
            actions: Vec::new(),
            width: WIDTH,
            on_dismiss,
        }
    }

    /// Menünün üstündeki başlık (ör. marka ve açık çizimin adı).
    pub fn header(mut self, header: impl Into<Element<'a, Message>>) -> Self {
        self.header = Some(header.into());
        self
    }

    pub fn entry(mut self, entry: Entry<'a, Message>) -> Self {
        self.entries.push(entry);
        self
    }

    /// Sağdaki ayrıntı bölmesi (genellikle bir [`Pane`]). Komut sütunu
    /// kadar yüksektir; uzun içerik kendi kaydırmasını getirir.
    pub fn detail(mut self, detail: impl Into<Element<'a, Message>>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Alt çubuğun sağındaki not (ör. ürünün adı ya da sürümü).
    pub fn note(mut self, note: impl IntoFragment<'a>) -> Self {
        self.note = Some(note.into_fragment());
        self
    }

    /// Alt çubuğa eylem düğmesi ekler.
    pub fn action(mut self, action: Action<'a, Message>) -> Self {
        self.actions.push(action);
        self
    }

    /// Menünün genişliği (yazı ölçeğiyle büyür); varsayılan 680.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

impl<'a, Message: Clone + 'a> From<AppMenu<'a, Message>> for Element<'a, Message> {
    fn from(menu: AppMenu<'a, Message>) -> Self {
        let entry_count = menu.entries.len().max(1) as f32;
        let body_height = typography::scaled(ENTRY_HEIGHT) * entry_count
            + ENTRY_GAP * (entry_count - 1.0)
            + PADDING * 2.0;

        let entries =
            Column::with_children(menu.entries.into_iter().map(Element::from)).spacing(ENTRY_GAP);

        let body = row![
            container(entries)
                .width(typography::scaled(COMMAND_COLUMN))
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

        // DESIGN.md §7.1.1: eylemler solda, not sağda.
        let mut footer = iced::widget::Row::new().spacing(6).align_y(Center);

        for action in menu.actions {
            footer = footer.push(action);
        }

        footer = footer.push(space::horizontal());

        if let Some(note) = menu.note {
            footer = footer.push(label::caption(note));
        }

        let mut panel = Column::new();

        if let Some(header) = menu.header {
            panel = panel
                .push(
                    container(header)
                        .padding([10, 14])
                        .width(Fill)
                        .style(style::container::header),
                )
                .push(horizontal_divider());
        }

        let panel = container(
            panel.push(body).push(horizontal_divider()).push(
                container(footer)
                    .padding([8, 10])
                    .width(Fill)
                    .style(style::container::surface),
            ),
        )
        .width(typography::scaled(menu.width))
        .style(style::container::popover);

        overlay::popover(
            panel,
            Point::new(0.0, ribbon::strip_height()),
            menu.on_dismiss,
        )
    }
}

/// Menünün solundaki büyük komut: ikon, başlık ve tek satırlık açıklama.
/// `on_press` verilmeyen komut soluk durur (ör. bu platformda henüz yok).
pub struct Entry<'a, Message> {
    icon: Icon,
    title: Fragment<'a>,
    description: Fragment<'a>,
    shortcut: Option<Fragment<'a>>,
    on_press: Option<Message>,
    on_hover: Option<Message>,
    submenu: Option<bool>,
    current: bool,
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
            shortcut: None,
            on_press: None,
            on_hover: None,
            submenu: None,
            current: false,
        }
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// `Some` ise `on_press`; `None` komutu soluk bırakır.
    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    /// İmleç komutun üzerine geldiğinde üretilecek mesaj.
    pub fn on_hover(mut self, message: Message) -> Self {
        self.on_hover = Some(message);
        self
    }

    /// Komutun kısayolu (ör. `Ctrl+S`); alt menüsü olmayan komutun sağında.
    pub fn shortcut(mut self, shortcut: impl IntoFragment<'a>) -> Self {
        self.shortcut = Some(shortcut.into_fragment());
        self
    }

    /// Komutun bir alt menüsü olduğunu gösterir (sağda ok). `expanded`
    /// ise komut vurgulanır; ayrıntı bölmesi onun alt menüsünü gösterir.
    pub fn submenu(mut self, expanded: bool) -> Self {
        self.submenu = Some(expanded);
        self
    }

    /// Klavyenin üzerinde durduğu komut: vurgulanır.
    pub fn current(mut self, current: bool) -> Self {
        self.current = current;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Entry<'a, Message>> for Element<'a, Message> {
    fn from(entry: Entry<'a, Message>) -> Self {
        let expanded = entry.submenu == Some(true);
        let enabled = entry.on_press.is_some();
        let highlighted = expanded || entry.current;

        // The chevron beside the row; a shortcut on the title's line, so the
        // description below keeps the row's whole width.
        let trailing: Element<'a, Message> = match entry.submenu {
            Some(_) => icon(Icon::ChevronRight).size(12.0).tone(Tone::Muted).into(),
            None => space::horizontal().width(4).into(),
        };

        let tile = container(icon(entry.icon).size(20.0).tone(if !enabled {
            Tone::Disabled
        } else if highlighted {
            Tone::Accent
        } else {
            Tone::Muted
        }))
        .center(typography::scaled(ICON_TILE))
        .style(style::container::tile);

        let title = label::heading(entry.title);
        let title = if enabled {
            title
        } else {
            title.style(style::text::disabled)
        };

        let heading: Element<'a, Message> = match entry.shortcut {
            Some(shortcut) => row![title, space::horizontal(), label::mono_caption(shortcut)]
                .align_y(Center)
                .into(),
            None => title.into(),
        };

        let content = button(
            row![
                tile,
                column![heading, label::caption(entry.description)]
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
        .height(typography::scaled(ENTRY_HEIGHT))
        .padding([0, 8])
        .style(style::button::list_item(highlighted));

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
/// sondaki bilgi. `on_press` verilmeyen satır soluk durur.
pub struct Item<'a, Message> {
    title: Fragment<'a>,
    subtitle: Option<Fragment<'a>>,
    leading: Option<Element<'a, Message>>,
    trailing: Option<Element<'a, Message>>,
    on_press: Option<Message>,
    current: bool,
}

impl<'a, Message: Clone + 'a> Item<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            subtitle: None,
            leading: None,
            trailing: None,
            on_press: None,
            current: false,
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

    /// `Some` ise `on_press`; `None` satırı soluk bırakır.
    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    /// Klavyenin üzerinde durduğu satır: vurgulanır.
    pub fn current(mut self, current: bool) -> Self {
        self.current = current;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Item<'a, Message>> for Element<'a, Message> {
    fn from(item: Item<'a, Message>) -> Self {
        let enabled = item.on_press.is_some();
        let title = label::body(item.title);
        let title = if enabled {
            title
        } else {
            title.style(style::text::disabled)
        };
        let mut text = column![title].spacing(1).width(Fill);

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
            .style(style::button::list_item(item.current))
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
