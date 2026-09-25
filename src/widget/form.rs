//! Form düzeni: etiket sütunu, alanlar, bölüm başlıkları, yardım ve hata
//! satırları.
//!
//! ```text
//! Genel ─────────────────────────────────────────
//!   Ad *         [Kadıköy imar planı          ]
//!                Paftanın başlığında görünür.
//!   Ölçek        [1:1000                     ⌄]
//! Konum ─────────────────────────────────────────
//!   Enlem        [91                          ]
//!                ⚠ −90 ile 90 arasında olmalı.
//! ```
//!
//! Etiketler aynı genişlikte bir sütunda, alanın ilk satırına hizalı durur.
//! Zorunlu alanın adının yanında yıldız, alanın altında yardım ya da hata
//! yazar; hata yardımın yerini alır.
//!
//! ```ignore
//! Form::new()
//!     .section("Genel")
//!     .field("Ad", text_input("", &self.name).on_input(Message::Named))
//!     .required()
//!     .help("Paftanın başlığında görünür.")
//!     .field("Enlem", latitude)
//!     .error(self.latitude_error.as_deref())
//! ```

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, column, container, row, rule};
use iced::{Alignment, Element, Fill, Padding};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::typography;

/// Etiket sütununun varsayılan genişliği (12 piksellik gövde metnine göre).
const LABEL: f32 = 110.0;

/// Form.
pub struct Form<'a, Message> {
    rows: Vec<Line<'a, Message>>,
    label_width: f32,
}

enum Line<'a, Message> {
    Section(Fragment<'a>),
    Field {
        label: Fragment<'a>,
        control: Element<'a, Message>,
        required: bool,
        help: Option<Fragment<'a>>,
        error: Option<Fragment<'a>>,
    },
    Full(Element<'a, Message>),
}

impl<'a, Message: 'a> Default for Form<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> Form<'a, Message> {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            label_width: LABEL,
        }
    }

    /// Etiket sütununun genişliği (12 piksellik gövde metnine göre).
    pub fn label_width(mut self, width: f32) -> Self {
        self.label_width = width;
        self
    }

    /// Bölüm başlığı: adı ve satırın sonuna kadar uzanan çizgi.
    pub fn section(mut self, title: impl IntoFragment<'a>) -> Self {
        self.rows.push(Line::Section(title.into_fragment()));
        self
    }

    /// Adlı alan.
    pub fn field(
        mut self,
        label: impl IntoFragment<'a>,
        control: impl Into<Element<'a, Message>>,
    ) -> Self {
        self.rows.push(Line::Field {
            label: label.into_fragment(),
            control: control.into(),
            required: false,
            help: None,
            error: None,
        });
        self
    }

    /// Etiket sütununa hizalı, adsız satır (ör. onay kutusu, düğmeler).
    pub fn row(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.rows.push(Line::Full(content.into()));
        self
    }

    /// Son alan zorunludur: adının yanında yıldız.
    pub fn required(mut self) -> Self {
        if let Some(Line::Field { required, .. }) = self.rows.last_mut() {
            *required = true;
        }
        self
    }

    /// Son alanın altındaki yardım metni.
    pub fn help(mut self, text: impl IntoFragment<'a>) -> Self {
        if let Some(Line::Field { help, .. }) = self.rows.last_mut() {
            *help = Some(text.into_fragment());
        }
        self
    }

    /// Son alanın hatası; varsa yardımın yerinde kırmızı yazar.
    pub fn error(mut self, text: Option<impl IntoFragment<'a>>) -> Self {
        if let Some(Line::Field { error, .. }) = self.rows.last_mut() {
            *error = text.map(IntoFragment::into_fragment);
        }
        self
    }
}

impl<'a, Message: 'a> From<Form<'a, Message>> for Element<'a, Message> {
    fn from(form: Form<'a, Message>) -> Self {
        let label_width = typography::scaled(form.label_width);
        let mut column = Column::new().spacing(10);

        for (index, line) in form.rows.into_iter().enumerate() {
            let line: Element<'a, Message> = match line {
                Line::Section(title) => container(
                    row![
                        label::strong(title),
                        container(rule::horizontal(1).style(style::field::hairline)).width(Fill)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .padding(Padding {
                    top: if index == 0 { 0.0 } else { 8.0 },
                    ..Padding::ZERO
                })
                .into(),
                Line::Field {
                    label: name,
                    control,
                    required,
                    help,
                    error,
                } => {
                    let mut name_row = row![label::body(name)].spacing(3);

                    if required {
                        name_row = name_row.push(label::body("*").style(style::text::danger));
                    }

                    let mut body = column![control].spacing(4);

                    body = match (error, help) {
                        (Some(error), _) => body.push(
                            row![
                                icon(Icon::Warning).size(12.0).tone(Tone::Danger),
                                label::caption(error).style(style::text::danger),
                            ]
                            .spacing(5)
                            .align_y(Alignment::Center),
                        ),
                        (None, Some(help)) => body.push(label::caption(help)),
                        (None, None) => body,
                    };

                    // Ad, alanın ilk satırıyla aynı yükseklikte başlar.
                    row![
                        container(name_row).width(label_width).padding(Padding {
                            top: 4.0,
                            ..Padding::ZERO
                        }),
                        body.width(Fill),
                    ]
                    .spacing(12)
                    .into()
                }
                Line::Full(content) => row![
                    container(iced::widget::space::horizontal()).width(label_width),
                    container(content).width(Fill),
                ]
                .spacing(12)
                .into(),
            };

            column = column.push(line);
        }

        column.into()
    }
}
