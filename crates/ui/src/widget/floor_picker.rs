//! Kat seçici: 3B yapı modelinde bir katı ayırmak (ya da kesit almak)
//! için; katlar yapının kesiti gibi üst üste durur, zemin çizgisinin altında
//! bodrumlar.
//!
//! ```text
//!  Katlar                    [Tümü]
//!  ┌──┐  Çatı
//!  ├──┤  4               +12,80
//!  ├██┤  3   2 birim      +9,60    ← seçili kat
//!  ├──┤  2                +6,40
//!  ├──┤  Z                ±0,00
//!  ═══════════════════════════  zemin
//!  ├░░┤  −1               −3,20
//!  └──┘
//! ```
//!
//! Katlar alttan üste verilir (ilk kat en alttaki); zemin katın sırası
//! ([`FloorPicker::ground`]) bodrumları ayırır. Katın kesit karesi isteğe
//! bağlı bir renk taşıyabilir (ör. afet simülasyonunda katın hasar sınıfı);
//! renk tek başına anlam taşımasın diye satırda notu da yazar. Seçili katı
//! yeniden tıklamak ya da “Tümü” bütün katlara döner.
//!
//! ```ignore
//! FloorPicker::new(&self.floors, self.floor, Message::FloorChosen)
//!     .ground(2)
//!     .title("Katlar")
//! ```

use iced::widget::{Column, button, column, container, row, space};
use iced::{Background, Border, Center, Color, Element, Fill, Theme};

use crate::label;
use crate::style;
use crate::theme::{Tokens, metrics, typography};

/// Kesit sütununun genişliği ve satır yüksekliği, varsayılan yazı boyutunda.
const SECTION: f32 = 30.0;
const ROW: f32 = 26.0;

/// Yapının bir katı.
#[derive(Debug, Clone, PartialEq)]
pub struct Floor {
    /// Kısa ad (ör. “Z”, “3”, “−1”).
    pub label: String,
    /// Döşemenin kotu (metre, zemine göre).
    pub elevation: f32,
    /// Satırdaki kısa not (ör. “4 daire”, “Ağır hasar”).
    pub note: Option<String>,
    /// Kesit karesinin rengi (ör. hasar sınıfı); yoksa nötr.
    pub tone: Option<Color>,
}

impl Floor {
    pub fn new(label: impl Into<String>, elevation: f32) -> Self {
        Self {
            label: label.into(),
            elevation,
            note: None,
            tone: None,
        }
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn tone(mut self, tone: Color) -> Self {
        self.tone = Some(tone);
        self
    }
}

/// Kat seçici.
pub struct FloorPicker<'a, Message> {
    floors: &'a [Floor],
    selected: Option<usize>,
    on_select: Box<dyn Fn(Option<usize>) -> Message + 'a>,
    ground: Option<usize>,
    title: Option<String>,
    roof: bool,
}

impl<'a, Message: Clone + 'a> FloorPicker<'a, Message> {
    /// `floors` alttan üste; `selected` `None` ise bütün katlar.
    pub fn new(
        floors: &'a [Floor],
        selected: Option<usize>,
        on_select: impl Fn(Option<usize>) -> Message + 'a,
    ) -> Self {
        Self {
            floors,
            selected,
            on_select: Box::new(on_select),
            ground: None,
            title: None,
            roof: true,
        }
    }

    /// Zemin katın sırası: altındakiler bodrumdur, aralarına zemin çizgisi
    /// çekilir.
    pub fn ground(mut self, ground: usize) -> Self {
        self.ground = Some(ground);
        self
    }

    /// Başlık (ör. “Katlar”); “Tümü” düğmesi yanında durur.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// En üstte çatı (varsayılan: gösterilir).
    pub fn roof(mut self, roof: bool) -> Self {
        self.roof = roof;
        self
    }
}

impl<'a, Message: Clone + 'a> From<FloorPicker<'a, Message>> for Element<'a, Message> {
    fn from(picker: FloorPicker<'a, Message>) -> Self {
        let FloorPicker {
            floors,
            selected,
            on_select,
            ground,
            title,
            roof,
        } = picker;

        let section = typography::from_default(SECTION);
        let height = typography::from_default(ROW);
        let all = selected.is_none();

        let mut header = row![].spacing(8).align_y(Center);

        if let Some(title) = title {
            header = header.push(label::strong(title));
        }

        header = header.push(space::horizontal()).push(
            button(
                container(label::caption(format!("Tümü ({})", floors.len())).style(
                    move |theme: &Theme| {
                        let t = Tokens::of(theme);

                        iced::widget::text::Style {
                            color: Some(if all { t.accent_hover } else { t.muted }),
                        }
                    },
                ))
                .center_y(Fill),
            )
            .on_press(on_select(None))
            .padding([0, 10])
            .height(metrics::inline())
            .style(style::button::chip(all)),
        );

        let mut rows = Column::new();

        if roof {
            rows = rows.push(
                row![
                    container(space::horizontal())
                        .width(section)
                        .height(5)
                        .style(|theme: &Theme| container::Style {
                            background: Some(Tokens::of(theme).border_strong().into()),
                            border: Border {
                                radius: 2.0.into(),
                                ..Border::default()
                            },
                            ..container::Style::default()
                        }),
                    label::caption("Çatı"),
                ]
                .spacing(10)
                .align_y(Center)
                .height(height * 0.7),
            );
        }

        for index in (0..floors.len()).rev() {
            let floor = &floors[index];
            let chosen = selected == Some(index);
            let basement = ground.is_some_and(|ground| index < ground);
            let tone = floor.tone;

            let cell = container(space::horizontal())
                .width(section)
                .height(height)
                .style(move |theme: &Theme| {
                    let t = Tokens::of(theme);
                    let fill = match (chosen, tone) {
                        (true, _) => t.accent,
                        (false, Some(tone)) => tone.scale_alpha(if basement { 0.55 } else { 0.75 }),
                        (false, None) if basement => t.layer(0.1),
                        (false, None) => t.layer(0.05),
                    };

                    container::Style {
                        background: Some(Background::Color(fill)),
                        border: Border {
                            color: t.border_strong(),
                            width: 1.0,
                            radius: 0.0.into(),
                        },
                        ..container::Style::default()
                    }
                });

            let name = if chosen {
                label::strong(floor.label.clone())
            } else {
                label::body(floor.label.clone())
            };

            let mut line = row![cell, container(name).width(typography::scaled(28.0))]
                .spacing(10)
                .align_y(Center);

            if let Some(note) = &floor.note {
                line = line.push(label::caption(note.clone()));
            }

            line = line
                .push(space::horizontal())
                .push(label::mono_caption(elevation(floor.elevation)));

            rows = rows.push(
                button(line)
                    .on_press(on_select(if chosen { None } else { Some(index) }))
                    .padding(iced::Padding {
                        right: 8.0,
                        ..iced::Padding::ZERO
                    })
                    .width(Fill)
                    .style(floor_row(chosen)),
            );

            // Zemin çizgisi: zemin katın altında.
            if ground == Some(index) && index > 0 {
                rows = rows.push(
                    row![
                        container(space::horizontal()).width(Fill).height(2).style(
                            |theme: &Theme| container::Style {
                                background: Some(Tokens::of(theme).muted.into()),
                                ..container::Style::default()
                            }
                        ),
                        label::caption("zemin"),
                    ]
                    .spacing(6)
                    .align_y(Center)
                    .height(typography::scaled(14.0)),
                );
            }
        }

        column![header, rows].spacing(8).width(Fill).into()
    }
}

/// Katın satırı: seçiliyken yumuşak vurgu zemini, üzerine gelince hafif katman.
fn floor_row(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let t = Tokens::of(theme);
        let background = match (selected, status) {
            (true, _) => Some(Background::Color(t.selection())),
            (false, button::Status::Hovered | button::Status::Pressed) => {
                Some(Background::Color(t.layer(0.05)))
            }
            _ => None,
        };

        button::Style {
            background,
            text_color: t.text,
            border: Border {
                radius: style::button::radius().into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

/// Kotun yazımı: “+12,80”, “±0,00”, “−3,20” (gerçek eksi işaretiyle).
pub fn elevation(value: f32) -> String {
    let text = crate::attribute::number::real(f64::from(value.abs()), 2);

    if value.abs() < 0.005 {
        format!("±{text}")
    } else if value > 0.0 {
        format!("+{text}")
    } else {
        format!("−{text}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elevations_carry_their_sign() {
        assert_eq!(elevation(12.8), "+12,80");
        assert_eq!(elevation(0.0), "±0,00");
        assert_eq!(elevation(-0.001), "±0,00");
        assert_eq!(elevation(-3.2), "−3,20");
    }
}
