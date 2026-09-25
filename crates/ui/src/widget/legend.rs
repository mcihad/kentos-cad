//! Lejant: haritadaki katmanların simgeleri ve adları.
//!
//! ```text
//! Lejant                      ⌃
//! Yerleşim
//!   ●  Şehirler              16
//!      ●  Marmara             2
//!      ●  Ege                 1
//! Ulaşım
//!   ━  Karayolları            7
//!   ▭  İlçeler               12
//! Nüfus
//!   ▕▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▏
//!   0             16 M
//! ```
//!
//! Her satırda bir simge ([`Symbol`]: nokta, çizgi, alan, renk kutusu ya da
//! ikon), ad ve isteğe bağlı sayı durur. Bölüm başlıkları satırları
//! gruplar, alt satırlar girintilidir; sürekli renk ölçekleri rampa ve uç
//! değerleriyle gösterilir. Satıra tıklanabilir (ör. katmanı gizlemek);
//! gizli katmanlar sönük yazılır. Lejant başlığına tıklayınca daralabilir.
//!
//! ```ignore
//! Legend::new()
//!     .title("Lejant")
//!     .section("Ulaşım")
//!     .item(Symbol::line(road_color, 2.0), "Karayolları")
//!     .detail("7")
//!     .on_press(Message::LayerToggled(3))
//!     .ramp("Nüfus", &ramp, "0", "16 M")
//! ```

use iced::widget::text::Wrapping;
use iced::widget::{Column, button, column, container, row, space};
use iced::{Background, Border, Center, Color, Element, Fill, Length, Theme};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::color::{self, Ramp};

/// Simgenin kutusu (12 piksellik gövde metnine göre).
const SYMBOL: f32 = 18.0;
/// Alt satırların girintisi.
const INDENT: f32 = 16.0;

/// Lejant simgesi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Symbol {
    /// Nokta: dolu daire, çapı piksel.
    Point { color: Color, size: f32 },
    /// Çizgi: kalınlığı piksel.
    Line { color: Color, width: f32 },
    /// Alan: saydam dolgu ve kenar.
    Area { fill: Color, stroke: Color },
    /// Düz renk kutusu (ör. alt katman).
    Swatch(Color),
    /// İkon, rengiyle.
    Icon(Icon, Color),
}

impl Symbol {
    pub fn point(color: Color) -> Self {
        Symbol::Point { color, size: 9.0 }
    }

    pub fn line(color: Color, width: f32) -> Self {
        Symbol::Line { color, width }
    }

    /// Dolgusu kenarın saydam hâli olan alan.
    pub fn area(color: Color) -> Self {
        Symbol::Area {
            fill: color.scale_alpha(0.35),
            stroke: color,
        }
    }
}

/// Simgenin çizimi; sönükse soluk.
fn symbol<'a, Message: 'a>(symbol: Symbol, muted: bool) -> Element<'a, Message> {
    let alpha = if muted { 0.35 } else { 1.0 };
    let side = typography::scaled(SYMBOL).round();
    let shape = |width: f32, height: f32, background: Color, border: Border| {
        container(space::horizontal())
            .width(width)
            .height(height)
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(background)),
                border,
                ..container::Style::default()
            })
    };

    let content: Element<'a, Message> = match symbol {
        Symbol::Point { color, size } => shape(
            size,
            size,
            color.scale_alpha(alpha),
            Border {
                radius: (size / 2.0).into(),
                ..Border::default()
            },
        )
        .into(),
        Symbol::Line { color, width } => shape(
            side - 2.0,
            width.max(2.0),
            color.scale_alpha(alpha),
            Border {
                radius: (width / 2.0).into(),
                ..Border::default()
            },
        )
        .into(),
        Symbol::Area { fill, stroke } => shape(
            side - 2.0,
            (side * 0.7).round(),
            fill.scale_alpha(alpha),
            Border {
                color: stroke.scale_alpha(alpha),
                width: 1.5,
                radius: 2.0.into(),
            },
        )
        .into(),
        Symbol::Swatch(color) => shape(
            (side * 0.6).round(),
            (side * 0.6).round(),
            color.scale_alpha(alpha),
            Border {
                radius: 2.0.into(),
                ..Border::default()
            },
        )
        .into(),
        Symbol::Icon(glyph, color) => icon(glyph)
            .size(14.0)
            .tone(Tone::Custom(color.scale_alpha(alpha)))
            .into(),
    };

    container(content).center(side).into()
}

/// Lejantın satırı.
enum Line<'a, Message> {
    Section(String),
    Item {
        symbol: Symbol,
        label: String,
        detail: Option<String>,
        indent: u8,
        muted: bool,
        on_press: Option<Message>,
    },
    Ramp {
        title: String,
        ramp: &'a Ramp,
        low: String,
        high: String,
    },
}

/// Lejant.
pub struct Legend<'a, Message> {
    title: Option<String>,
    collapse: Option<(bool, Message)>,
    lines: Vec<Line<'a, Message>>,
    width: Length,
}

impl<'a, Message: Clone + 'a> Default for Legend<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> Legend<'a, Message> {
    pub fn new() -> Self {
        Self {
            title: None,
            collapse: None,
            lines: Vec::new(),
            width: Length::Shrink,
        }
    }

    /// Lejantın başlığı.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Başlığa tıklayınca daralır ya da açılır; başlık gerekir.
    pub fn collapsible(mut self, collapsed: bool, on_toggle: Message) -> Self {
        self.collapse = Some((collapsed, on_toggle));
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Satırları gruplayan başlık.
    pub fn section(mut self, title: impl Into<String>) -> Self {
        self.lines.push(Line::Section(title.into()));
        self
    }

    /// Simgeli satır.
    pub fn item(mut self, symbol: Symbol, label: impl Into<String>) -> Self {
        self.lines.push(Line::Item {
            symbol,
            label: label.into(),
            detail: None,
            indent: 0,
            muted: false,
            on_press: None,
        });
        self
    }

    /// Bir önceki satırın altındaki, girintili satır (ör. alt katman).
    pub fn sub(mut self, symbol: Symbol, label: impl Into<String>) -> Self {
        self.lines.push(Line::Item {
            symbol,
            label: label.into(),
            detail: None,
            indent: 1,
            muted: false,
            on_press: None,
        });
        self
    }

    /// Son satırın sağındaki ayrıntı (ör. öğe sayısı).
    pub fn detail(mut self, text: impl Into<String>) -> Self {
        if let Some(Line::Item { detail, .. }) = self.lines.last_mut() {
            *detail = Some(text.into());
        }
        self
    }

    /// Son satır sönük (ör. gizli katman).
    pub fn muted(mut self, value: bool) -> Self {
        if let Some(Line::Item { muted, .. }) = self.lines.last_mut() {
            *muted = value;
        }
        self
    }

    /// Son satıra tıklanınca gönderilir (ör. katmanı gizle).
    pub fn on_press(mut self, message: Message) -> Self {
        if let Some(Line::Item { on_press, .. }) = self.lines.last_mut() {
            *on_press = Some(message);
        }
        self
    }

    /// Sürekli renk ölçeği: başlık, rampa ve uç değerler.
    pub fn ramp(
        mut self,
        title: impl Into<String>,
        ramp: &'a Ramp,
        low: impl Into<String>,
        high: impl Into<String>,
    ) -> Self {
        self.lines.push(Line::Ramp {
            title: title.into(),
            ramp,
            low: low.into(),
            high: high.into(),
        });
        self
    }
}

impl<'a, Message: Clone + 'a> From<Legend<'a, Message>> for Element<'a, Message> {
    fn from(legend: Legend<'a, Message>) -> Self {
        let collapsed = legend
            .collapse
            .as_ref()
            .is_some_and(|(collapsed, _)| *collapsed);
        let mut body = Column::new().spacing(2);

        if let Some(title) = legend.title {
            let heading = row![label::strong(title), space::horizontal()]
                .spacing(6)
                .align_y(Center);

            let heading: Element<'a, Message> = match legend.collapse {
                Some((collapsed, on_toggle)) => button(
                    heading.push(
                        icon(if collapsed {
                            Icon::ChevronRight
                        } else {
                            Icon::ChevronDown
                        })
                        .size(12.0)
                        .tone(Tone::Muted),
                    ),
                )
                .on_press(on_toggle)
                .padding([2, 4])
                .width(Fill)
                .style(style::button::flat)
                .into(),
                None => container(heading).padding([2, 4]).into(),
            };

            body = body.push(heading);
        }

        if !collapsed {
            for (index, line) in legend.lines.into_iter().enumerate() {
                let line: Element<'a, Message> = match line {
                    Line::Section(title) => container(label::caption(title))
                        .padding(iced::Padding {
                            top: if index == 0 { 2.0 } else { 8.0 },
                            right: 4.0,
                            bottom: 2.0,
                            left: 4.0,
                        })
                        .into(),
                    Line::Item {
                        symbol: glyph,
                        label: name,
                        detail,
                        indent,
                        muted,
                        on_press,
                    } => {
                        let text = if muted {
                            label::body(name).style(style::text::muted)
                        } else {
                            label::body(name)
                        };
                        let mut content = row![
                            space::horizontal()
                                .width(typography::scaled(INDENT) * f32::from(indent)),
                            symbol(glyph, muted),
                            text.wrapping(Wrapping::None),
                            space::horizontal(),
                        ]
                        .spacing(6)
                        .align_y(Center);

                        if let Some(detail) = detail {
                            content = content.push(label::mono_caption(detail));
                        }

                        match on_press {
                            Some(message) => button(content)
                                .on_press(message)
                                .padding([1, 4])
                                .width(Fill)
                                .style(style::button::flat)
                                .into(),
                            None => container(content).padding([1, 4]).into(),
                        }
                    }
                    Line::Ramp {
                        title,
                        ramp,
                        low,
                        high,
                    } => column![
                        label::caption(title),
                        color::preview(ramp, typography::scaled(160.0), 10.0),
                        row![
                            label::mono_caption(low),
                            space::horizontal(),
                            label::mono_caption(high),
                        ]
                        .width(typography::scaled(160.0)),
                    ]
                    .spacing(3)
                    .padding(iced::Padding {
                        top: 8.0,
                        right: 4.0,
                        bottom: 2.0,
                        left: 4.0,
                    })
                    .into(),
                };

                body = body.push(line);
            }
        }

        container(body).width(legend.width).into()
    }
}

/// Lejantın saydam zeminli kutusu: harita üstünde durur.
pub fn frame<'a, Message: 'a>(legend: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(legend)
        .padding([6, 6])
        .style(|theme: &Theme| {
            let t = Tokens::of(theme);

            container::Style {
                background: Some(Background::Color(t.popover.scale_alpha(0.92))),
                border: Border {
                    color: t.border,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..container::Style::default()
            }
        })
        .into()
}
