//! Özellik ızgarası: CAD programlarındaki "Özellikler" paleti.
//!
//! Anahtar ve değer iki sütunda, kategoriler başlık satırlarıyla gruplanır;
//! hücreler arasında 1 piksellik ızgara çizgileri bulunur.
//!
//! [`PropertySheet`] Öznitelikler panelinin ızgarasıdır (DESIGN.md §7.5):
//! başlığına tıklanınca katlanan bölümler, adı ve değeri olan satırlar.
//! Değer düz yazı ([`value`]), düzenlenene dek yazı gibi görünen hücre
//! ([`EditCell`](crate::widget::EditCell)) ya da açılır liste ([`choice`])
//! olabilir.
//!
//! ```text
//!   ▾ Genel
//!     Tür               Kapalı alan
//!     Katman            ■ Parseller      ▾
//!     Renk              Katmana göre     ▾
//!   ▸ Geometri
//!   ▾ Öznitelik bilgileri
//!     Parsel            12
//! ```

use std::marker::PhantomData;

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, button, container, row, space};
use iced::{Center, Color, Element, Fill, Length, Theme};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::{Menu, MenuButton};

const KEY_WIDTH: f32 = 112.0;
/// A sheet's row and section header heights, 12 px body text (the web's `--row-h`, 30 px).
const SHEET_ROW: f32 = 28.0;
const SHEET_SECTION: f32 = 30.0;
/// The label column's share of the width, in hundredths (the web's 42 %).
const LABEL_SHARE: u16 = 42;

/// Özellik ızgarası.
pub struct PropertyGrid<'a, Message> {
    entries: Vec<Entry<'a>>,
    _message: PhantomData<Message>,
}

enum Entry<'a> {
    Category(Fragment<'a>),
    Property {
        key: Fragment<'a>,
        value: Fragment<'a>,
        mono: bool,
    },
}

impl<'a, Message> PropertyGrid<'a, Message> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            _message: PhantomData,
        }
    }

    /// Sonraki özellikleri gruplayan kategori satırı.
    pub fn category(mut self, title: impl IntoFragment<'a>) -> Self {
        self.entries.push(Entry::Category(title.into_fragment()));
        self
    }

    /// Metin değerli özellik.
    pub fn property(mut self, key: impl IntoFragment<'a>, value: impl IntoFragment<'a>) -> Self {
        self.entries.push(Entry::Property {
            key: key.into_fragment(),
            value: value.into_fragment(),
            mono: false,
        });
        self
    }

    /// Sayısal değerli özellik; değer eş aralıklı yazılır.
    pub fn figure(mut self, key: impl IntoFragment<'a>, value: impl IntoFragment<'a>) -> Self {
        self.entries.push(Entry::Property {
            key: key.into_fragment(),
            value: value.into_fragment(),
            mono: true,
        });
        self
    }
}

impl<'a, Message> Default for PropertyGrid<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<PropertyGrid<'a, Message>> for Element<'a, Message> {
    fn from(grid: PropertyGrid<'a, Message>) -> Self {
        let rows = grid.entries.into_iter().map(|entry| match entry {
            Entry::Category(title) => container(
                row![
                    icon(Icon::ChevronDown).size(10.0).tone(Tone::Muted),
                    label::caption(title)
                        .font(crate::theme::typography::ui_strong())
                        .style(style::text::default),
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding([3, 8])
            .width(Fill)
            .style(style::container::header)
            .into(),
            Entry::Property { key, value, mono } => row![
                container(label::muted(key))
                    .width(typography::scaled(KEY_WIDTH))
                    .padding([3, 8])
                    .style(style::container::surface),
                container(if mono {
                    label::mono(value)
                } else {
                    label::body(value)
                })
                .width(Fill)
                .padding([3, 8])
                .style(style::container::surface_alt),
            ]
            .spacing(1)
            .into(),
        });

        container(Column::with_children(rows).spacing(1))
            .padding([1, 0])
            .width(Fill)
            .style(style::container::grid_lines)
            .into()
    }
}

/// Öznitelikler ızgarası: katlanabilir bölümler ve satırlar.
pub struct PropertySheet<'a, Message> {
    rows: Vec<Element<'a, Message>>,
    /// Whether the current section is open: a closed one's rows are left out.
    open: bool,
}

impl<'a, Message: Clone + 'a> PropertySheet<'a, Message> {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            open: true,
        }
    }

    /// Sonraki satırları toplayan bölüm; başlığına tıklamak `on_toggle`'ı
    /// gönderir. Kapalı bölümün satırları gösterilmez.
    pub fn section(mut self, title: impl IntoFragment<'a>, open: bool, on_toggle: Message) -> Self {
        let first = self.rows.is_empty();
        let head = button(
            container(
                row![
                    icon(if open {
                        Icon::ChevronDown
                    } else {
                        Icon::ChevronRight
                    })
                    .size(14.0)
                    .tone(Tone::Muted),
                    label::caption(title).font(typography::ui_strong()),
                ]
                .spacing(4)
                .align_y(Center),
            )
            .height(Fill)
            .align_y(Center),
        )
        .on_press(on_toggle)
        .width(Fill)
        .height(typography::scaled(SHEET_SECTION))
        .padding([0, 8])
        .style(section_head);
        self.rows.push(if first {
            head.into()
        } else {
            iced::widget::column![rule(), head].into()
        });
        self.open = open;
        self
    }

    /// Adı ve değeri olan satır; değer bir öğedir (düz yazı, hücre, liste).
    pub fn row(
        mut self,
        key: impl IntoFragment<'a>,
        value: impl Into<Element<'a, Message>>,
    ) -> Self {
        if !self.open {
            return self;
        }
        let key = key.into_fragment();
        self.rows.push(
            iced::widget::column![
                row![
                    container(label::muted(key).wrapping(iced::widget::text::Wrapping::None))
                        .width(Length::FillPortion(LABEL_SHARE))
                        .padding([0, 8])
                        .clip(true),
                    container(value)
                        .width(Length::FillPortion(100 - LABEL_SHARE))
                        .padding([0, 8]),
                ]
                .height(typography::scaled(SHEET_ROW))
                .align_y(Center),
                soft_rule(),
            ]
            .into(),
        );
        self
    }
}

impl<'a, Message: Clone + 'a> Default for PropertySheet<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<PropertySheet<'a, Message>> for Element<'a, Message> {
    fn from(sheet: PropertySheet<'a, Message>) -> Self {
        Column::with_children(sheet.rows).width(Fill).into()
    }
}

/// Salt okunur değer: düz yazı, sayıysa eş aralıklı; ardından soluk birim.
pub fn value<'a, Message: 'a>(
    text: impl IntoFragment<'a>,
    numeric: bool,
    unit: Option<String>,
) -> Element<'a, Message> {
    let text = if numeric {
        label::mono(text)
    } else {
        label::body(text)
    };
    let mut content = row![
        container(text.wrapping(iced::widget::text::Wrapping::None))
            .padding([0, 6])
            .clip(true)
    ]
    .align_y(Center);
    if let Some(unit) = unit {
        content = content.push(label::caption(unit).style(style::text::muted));
    }
    content.into()
}

/// Açılır liste hücresi: renk örneği (varsa), seçili değerin adı ve ok;
/// tıklanınca `menu` altında açılır.
pub fn choice<'a, Message: Clone + 'a>(
    text: impl IntoFragment<'a>,
    swatch: Option<Color>,
    menu: impl Fn() -> Menu<Message> + 'a,
) -> Element<'a, Message> {
    let mut face = row![].spacing(6).align_y(Center);
    if let Some(color) = swatch {
        face = face.push(crate::widget::swatch(color));
    }
    face = face
        .push(
            container(label::body(text).wrapping(iced::widget::text::Wrapping::None))
                .width(Fill)
                .clip(true),
        )
        .push(icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted));
    MenuButton::new(
        container(face)
            .padding([0, 5])
            .height(typography::scaled(22.0))
            .align_y(Center)
            .width(Fill),
        menu,
    )
    .into()
}

/// The line above a section header.
fn rule<'a, Message: 'a>() -> Element<'a, Message> {
    container(space::horizontal())
        .width(Fill)
        .height(1)
        .style(|theme: &Theme| container::Style {
            background: Some(Tokens::of(theme).border.into()),
            ..container::Style::default()
        })
        .into()
}

/// The soft line under a row.
fn soft_rule<'a, Message: 'a>() -> Element<'a, Message> {
    container(space::horizontal())
        .width(Fill)
        .height(1)
        .style(|theme: &Theme| container::Style {
            background: Some(Tokens::of(theme).border.scale_alpha(0.5).into()),
            ..container::Style::default()
        })
        .into()
}

/// A section header: secondary text, the main text when hovered.
fn section_head(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::of(theme);
    button::Style {
        background: None,
        text_color: match status {
            button::Status::Hovered | button::Status::Pressed => t.text,
            _ => t.muted,
        },
        ..button::Style::default()
    }
}
