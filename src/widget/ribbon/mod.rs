//! Şerit (ribbon): sekme şeridi ve seçili sekmenin araç grupları.
//!
//! ```text
//! ┌──────────────┬───────┬──────┬──────────┬─────────────────── meta ┐
//! │ ■ KentOS ⌄   │ Giriş │ Ekle │ Açıklama │                          │
//! ├──────────────┘       └──────┴──────────┴──────────────────────────┤
//! │ ┌────┐ ┌────┐ ▢ Küçük   │ ┌────┐ ▢ Küçük   │ ...                   │
//! │ │ ⊕  │ │ ⊕  │ ▢ Küçük   │ │ ⊕  │ ▢ Küçük   │                       │
//! │ └────┘ └────┘ ▢ Küçük   │ └────┘           │                       │
//! │        Grup adı         │   Grup adı       │                       │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Grup içeriği 3 satırlık bir ızgaraya oturur: küçük düğmeler ve alanlar
//! bir satır ([`ROW`]), büyük düğmeler üç satır ([`CONTENT`]) yüksekliğindedir.
//! Seçili sekmenin alt çizgisi yoktur ve zemini panelle aynıdır; sekme
//! alttaki panele akar.

mod control;
mod layout;

pub use control::{AppButton, Button};
pub use layout::{Field, Group, Row, Stack};

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, button, column, container, row, space};
use iced::{Element, Fill, Length, Padding, Right};

use crate::label;
use crate::style;
use crate::widget::{horizontal_divider, vertical_divider};

/// Izgara satırının yüksekliği: küçük düğme, alan.
pub const ROW: f32 = 22.0;
/// Izgara satırları arasındaki boşluk.
pub const ROW_GAP: f32 = 1.0;
/// Grup içeriğinin yüksekliği: üç satır.
pub const CONTENT: f32 = ROW * 3.0 + ROW_GAP * 2.0;
/// Küçük düğme ikonlarının boyutu; alanların önündeki sütun genişliği.
pub const ICON: f32 = 16.0;
/// Büyük düğme ikonlarının boyutu.
pub const LARGE_ICON: f32 = 24.0;
/// Büyük düğmelerin genişliği.
pub const LARGE_WIDTH: f32 = 62.0;

/// Grup adının satır yüksekliği.
pub const CAPTION: f32 = 16.0;
const PANEL_PADDING_TOP: f32 = 6.0;
const PANEL_PADDING_BOTTOM: f32 = 2.0;
/// Araç panelinin yüksekliği (alt kenar hariç).
pub const PANEL_HEIGHT: f32 = PANEL_PADDING_TOP + CONTENT + CAPTION + PANEL_PADDING_BOTTOM;

/// Sekme başlıklarının yüksekliği.
pub const TAB_HEIGHT: f32 = 30.0;
/// Seçili sekmenin üstündeki vurgu çizgisi.
const TAB_ACCENT: f32 = 2.0;
/// Sekme şeridinin pencere kenarından uzaklığı.
const STRIP_TOP: f32 = 4.0;
/// Sekme şeridinin toplam yüksekliği; uygulama menüsü bunun hemen altına
/// açılır.
pub const STRIP_HEIGHT: f32 = STRIP_TOP + TAB_HEIGHT;

const EDGE: f32 = 10.0;

/// Şerit.
pub struct Ribbon<'a, Message> {
    application: Option<AppButton<'a, Message>>,
    tabs: Vec<Tab<'a, Message>>,
    trailing: Option<Element<'a, Message>>,
    panel: Panel<'a, Message>,
}

struct Tab<'a, Message> {
    label: Fragment<'a>,
    selected: bool,
    on_press: Message,
}

enum Panel<'a, Message> {
    Groups(Vec<Group<'a, Message>>),
    Placeholder(Fragment<'a>),
}

impl<'a, Message: Clone + 'a> Ribbon<'a, Message> {
    pub fn new() -> Self {
        Self {
            application: None,
            tabs: Vec::new(),
            trailing: None,
            panel: Panel::Groups(Vec::new()),
        }
    }

    /// Sekme şeridinin başındaki uygulama (marka) düğmesi.
    pub fn application(mut self, application: AppButton<'a, Message>) -> Self {
        self.application = Some(application);
        self
    }

    /// Tek bir sekme ekler.
    pub fn tab(mut self, label: impl IntoFragment<'a>, selected: bool, on_press: Message) -> Self {
        self.tabs.push(Tab {
            label: label.into_fragment(),
            selected,
            on_press,
        });
        self
    }

    /// Bir sekme listesi ekler; etiketler `Display` ile yazılır.
    pub fn tabs<T>(
        self,
        tabs: impl IntoIterator<Item = T>,
        selected: T,
        on_select: impl Fn(T) -> Message,
    ) -> Self
    where
        T: Copy + PartialEq + std::fmt::Display,
    {
        tabs.into_iter().fold(self, |ribbon, tab| {
            ribbon.tab(tab.to_string(), tab == selected, on_select(tab))
        })
    }

    /// Sekme şeridinin sağ ucundaki içerik (ör. açık belgenin adı).
    pub fn trailing(mut self, trailing: impl Into<Element<'a, Message>>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }

    /// Seçili sekmenin paneline bir grup ekler.
    pub fn group(mut self, group: Group<'a, Message>) -> Self {
        match &mut self.panel {
            Panel::Groups(groups) => groups.push(group),
            Panel::Placeholder(_) => self.panel = Panel::Groups(vec![group]),
        }
        self
    }

    /// Seçili sekmenin panelini grupsuz, açıklama metniyle gösterir.
    pub fn placeholder(mut self, message: impl IntoFragment<'a>) -> Self {
        self.panel = Panel::Placeholder(message.into_fragment());
        self
    }

    fn strip(
        application: Option<AppButton<'a, Message>>,
        tabs: Vec<Tab<'a, Message>>,
        trailing: Option<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let mut strip = iced::widget::Row::new().height(TAB_HEIGHT + 1.0);

        strip = match application {
            Some(application) => strip.push(underlined(
                row![application, space::horizontal().width(8)],
                Length::Shrink,
            )),
            None => strip.push(underlined(space::horizontal().width(EDGE), Length::Shrink)),
        };

        for tab in tabs {
            strip = strip.push(if tab.selected {
                selected_tab(tab.label)
            } else {
                underlined(
                    button(
                        container(label::body(tab.label))
                            .height(Fill)
                            .align_y(iced::Center),
                    )
                    .on_press(tab.on_press)
                    .height(TAB_HEIGHT)
                    .padding([0, 14])
                    .style(style::button::tab),
                    Length::Shrink,
                )
            });
        }

        let trailing = container(trailing.unwrap_or_else(|| space::horizontal().into()))
            .width(Fill)
            .height(TAB_HEIGHT)
            .padding([0.0, EDGE])
            .align_x(Right)
            .align_y(iced::Center);

        container(column![
            space::vertical().height(STRIP_TOP),
            strip.push(underlined(trailing, Fill)),
        ])
        .width(Fill)
        .style(style::container::window)
        .into()
    }

    fn panel(panel: Panel<'a, Message>) -> Element<'a, Message> {
        let content: Element<'a, Message> = match panel {
            Panel::Groups(groups) => {
                let mut groups_row = iced::widget::Row::new().height(Fill);

                for group in groups {
                    groups_row = groups_row.push(group).push(vertical_divider());
                }

                groups_row.into()
            }
            Panel::Placeholder(message) => container(label::muted(message))
                .padding([0, 16])
                .height(Fill)
                .align_y(iced::Center)
                .into(),
        };

        container(content)
            .width(Fill)
            .height(PANEL_HEIGHT)
            .style(style::container::surface)
            .into()
    }
}

impl<'a, Message: Clone + 'a> Default for Ribbon<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<Ribbon<'a, Message>> for Element<'a, Message> {
    fn from(ribbon: Ribbon<'a, Message>) -> Self {
        let Ribbon {
            application,
            tabs,
            trailing,
            panel,
        } = ribbon;

        Column::new()
            .push(Ribbon::strip(application, tabs, trailing))
            .push(Ribbon::panel(panel))
            .push(horizontal_divider())
            .into()
    }
}

/// Sekme şeridindeki bir öğe; altında şerit boyunca uzanan çizgiyle.
fn underlined<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    width: impl Into<Length>,
) -> Element<'a, Message> {
    column![content.into(), horizontal_divider()]
        .width(width)
        .into()
}

/// Seçili sekme: üstte vurgu çizgisi, yanlarda kenar çizgisi; altta çizgi
/// yoktur ve gövdesi panelle aynı renktedir.
fn selected_tab<'a, Message: 'a>(label: Fragment<'a>) -> Element<'a, Message> {
    row![
        vertical_divider(),
        column![
            container(space::horizontal())
                .width(Fill)
                .height(TAB_ACCENT)
                .style(style::container::accent),
            // Alt boşluk vurgu çizgisini dengeler; yazı diğer sekmelerle aynı
            // taban çizgisinde kalır.
            container(label::strong(label))
                .height(Fill)
                .padding(Padding {
                    top: 0.0,
                    right: 14.0,
                    bottom: TAB_ACCENT + 1.0,
                    left: 14.0,
                })
                .align_y(iced::Center)
                .style(style::container::surface),
        ]
        .width(Length::Shrink),
        vertical_divider(),
    ]
    .height(TAB_HEIGHT + 1.0)
    .into()
}
