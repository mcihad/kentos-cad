//! Gösterge kartı: bir sonucun değeri, birimi, önceki duruma göre değişimi
//! ve küçük eğilim çizgisi (ör. afet senaryosunda etkilenen yapı sayısı,
//! tahliye süresi).
//!
//! ```text
//!  ┌──────────────────────────────┐
//!  │ Etkilenen yapı            ⚠  │
//!  │ 1.284  yapı                  │
//!  │ ⌃ +8,2 %  önceki senaryoya göre│
//!  │ ╱╲__╱‾‾╲_╱‾                  │
//!  └──────────────────────────────┘
//! ```
//!
//! Değişimin rengi yönünden değil anlamından gelir ([`Better`]): etkilenen
//! yapı için artış kötü, toplanma alanı kapasitesi için iyidir. Renk tek
//! başına anlam taşımaz: ok ve işaretli değer her zaman yazar.
//!
//! ```ignore
//! StatCard::new("Etkilenen yapı", "1.284")
//!     .unit("yapı")
//!     .delta(Delta::new(8.2, "+8,2 %").better(Better::Down))
//!     .note("önceki senaryoya göre")
//!     .trend(&self.affected_by_hour)
//! ```

use std::cell::RefCell;

use iced::widget::canvas::{self, Cache, Canvas, Frame, Geometry, Path, Stroke, gradient};
use iced::widget::{Column, button, column, container, row, space};
use iced::{Center, Color, Element, Fill, Length, Point, Rectangle, Renderer, Theme, mouse};

use crate::icon::{Icon, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::Severity;

/// Değişimin hangi yönü iyidir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Better {
    /// Artış iyidir (ör. kapasite).
    #[default]
    Up,
    /// Azalış iyidir (ör. hasar, süre).
    Down,
    /// Yönün iyi ya da kötü anlamı yok.
    Neither,
}

/// Önceki duruma göre değişim.
#[derive(Debug, Clone, PartialEq)]
pub struct Delta {
    /// İşaretli değer: yönü ve rengi belirler.
    pub value: f32,
    /// Yazımı (ör. “+8,2 %”, “−3 dk”).
    pub text: String,
    pub better: Better,
}

impl Delta {
    pub fn new(value: f32, text: impl Into<String>) -> Self {
        Self {
            value,
            text: text.into(),
            better: Better::default(),
        }
    }

    pub fn better(mut self, better: Better) -> Self {
        self.better = better;
        self
    }

    /// Değişimin anlamı: iyi, kötü ya da yansız.
    fn severity(&self) -> Option<Severity> {
        if self.value.abs() < f32::EPSILON {
            return None;
        }

        match (self.better, self.value > 0.0) {
            (Better::Neither, _) => None,
            (Better::Up, true) | (Better::Down, false) => Some(Severity::Success),
            (Better::Up, false) | (Better::Down, true) => Some(Severity::Error),
        }
    }
}

/// Gösterge kartı.
pub struct StatCard<'a, Message> {
    label: String,
    value: String,
    unit: Option<String>,
    delta: Option<Delta>,
    note: Option<String>,
    trend: Option<Vec<f32>>,
    status: Option<(Severity, String)>,
    on_press: Option<Message>,
    selected: bool,
    width: Length,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a, Message: Clone + 'a> StatCard<'a, Message> {
    /// `value` yazılmış hâliyle (ör. “1.284”, “42”).
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            unit: None,
            delta: None,
            note: None,
            trend: None,
            status: None,
            on_press: None,
            selected: false,
            width: Length::Fill,
            _lifetime: std::marker::PhantomData,
        }
    }

    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn delta(mut self, delta: Delta) -> Self {
        self.delta = Some(delta);
        self
    }

    /// Değişimin yanındaki kısa açıklama (ör. “önceki senaryoya göre”).
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Eğilim çizgisinin değerleri, zaman sırasıyla.
    pub fn trend(mut self, values: &[f32]) -> Self {
        self.trend = Some(values.to_vec());
        self
    }

    /// Kartın durumu: başlığın yanında ikon; ipucu yerine açıklaması altta.
    pub fn status(mut self, severity: Severity, text: impl Into<String>) -> Self {
        self.status = Some((severity, text.into()));
        self
    }

    /// Tıklanınca (ör. haritada ilgili katmanı göster).
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Seçili kart (ör. haritada gösterilen gösterge).
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

impl<'a, Message: Clone + 'a> From<StatCard<'a, Message>> for Element<'a, Message> {
    fn from(card: StatCard<'a, Message>) -> Self {
        let StatCard {
            label: title,
            value,
            unit,
            delta,
            note,
            trend,
            status,
            on_press,
            selected,
            width,
            ..
        } = card;

        let mut top = row![label::muted(title)].spacing(6).align_y(Center);

        if let Some((severity, _)) = &status {
            top = top
                .push(space::horizontal())
                .push(icon(severity.icon()).size(14.0).tone(severity.tone()));
        }

        let mut figure = row![label::figure(value).style(style::text::default)]
            .spacing(6)
            .align_y(iced::Bottom);

        if let Some(unit) = unit {
            figure = figure.push(container(label::caption(unit)).padding(iced::Padding {
                bottom: typography::scaled(3.0),
                ..iced::Padding::ZERO
            }));
        }

        let mut content: Column<'a, Message> = column![top, figure].spacing(4);
        let tone = delta.as_ref().and_then(Delta::severity);

        if delta.is_some() || note.is_some() {
            let mut line = row![].spacing(6).align_y(Center);

            if let Some(delta) = &delta {
                let glyph = if delta.value > 0.0 {
                    Icon::ChevronUp
                } else if delta.value < 0.0 {
                    Icon::ChevronDown
                } else {
                    Icon::Minus
                };

                line = line.push(
                    container(
                        row![
                            icon(glyph).size(10.0),
                            label::caption(delta.text.clone()).style(move |theme: &Theme| {
                                let t = Tokens::of(theme);

                                iced::widget::text::Style {
                                    color: Some(tone.map_or(t.text, |tone| tone.color(&t))),
                                }
                            }),
                        ]
                        .spacing(3)
                        .align_y(Center),
                    )
                    .padding([1, 6])
                    .style(move |theme: &Theme| {
                        let t = Tokens::of(theme);
                        let color = tone.map_or(t.muted, |tone| tone.color(&t));

                        container::Style {
                            text_color: Some(color),
                            background: Some(color.scale_alpha(0.14).into()),
                            border: iced::Border {
                                color: color.scale_alpha(0.45),
                                width: 1.0,
                                radius: 999.0.into(),
                            },
                            ..container::Style::default()
                        }
                    }),
                );
            }

            if let Some(note) = note {
                line = line.push(label::caption(note));
            }

            content = content.push(line);
        }

        if let Some(values) = trend.filter(|values| values.len() > 1) {
            content = content.push(
                Canvas::new(Sparkline { values, tone })
                    .width(Fill)
                    .height(typography::scaled(30.0)),
            );
        }

        if let Some((_, text)) = status {
            content = content.push(label::caption(text));
        }

        let body = container(content).padding([12, 14]).width(Fill);

        let card: Element<'a, Message> = match on_press {
            Some(message) => button(body)
                .on_press(message)
                .padding(0)
                .width(width)
                .style(card_button(selected))
                .into(),
            None => container(body)
                .width(width)
                .style(style::container::bordered)
                .into(),
        };

        card
    }
}

/// Tıklanabilen kart: kart kenarı, üzerine gelince belirgin, seçiliyken vurgu.
fn card_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let t = Tokens::of(theme);
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);

        button::Style {
            background: Some(if selected { t.selection() } else { t.surface }.into()),
            text_color: t.text,
            border: iced::Border {
                color: match (selected, hovered) {
                    (true, _) => t.accent_line(),
                    (false, true) => t.border_strong(),
                    (false, false) => t.border,
                },
                width: 1.0,
                radius: crate::theme::shape::md().into(),
            },
            ..button::Style::default()
        }
    }
}

/// Eğilim çizgisi: değerler kartın genişliğine yayılır, altı hafif dolgulu,
/// son değer noktayla.
struct Sparkline {
    values: Vec<f32>,
    tone: Option<Severity>,
}

/// Çizimin neyle yapıldığı: değerlerin bitleri, ton ve tema koyuluğu.
type SparklineKey = (Vec<u32>, Option<Severity>, bool);

#[derive(Default)]
struct SparklineState {
    cache: Cache,
    key: RefCell<Option<SparklineKey>>,
}

impl<Message> canvas::Program<Message> for Sparkline {
    type State = SparklineState;

    fn draw(
        &self,
        state: &SparklineState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let t = Tokens::of(theme);
        let key = (
            self.values.iter().map(|value| value.to_bits()).collect(),
            self.tone,
            t.is_dark,
        );

        if state.key.borrow().as_ref() != Some(&key) {
            state.cache.clear();
            *state.key.borrow_mut() = Some(key);
        }

        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            let color = self.tone.map_or(t.accent, |tone| tone.color(&t));
            line(frame, &self.values, color);
        });

        vec![geometry]
    }
}

fn line(frame: &mut Frame, values: &[f32], color: Color) {
    let size = frame.size();
    let (low, high) = values
        .iter()
        .fold((f32::MAX, f32::MIN), |(low, high), value| {
            (low.min(*value), high.max(*value))
        });
    let span = (high - low).max(f32::EPSILON);
    let pad = 3.0;
    let points: Vec<Point> = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            Point::new(
                pad + (size.width - 2.0 * pad) * index as f32 / (values.len() - 1) as f32,
                pad + (size.height - 2.0 * pad) * (1.0 - (value - low) / span),
            )
        })
        .collect();

    let area = Path::new(|path| {
        path.move_to(Point::new(points[0].x, size.height));
        for point in &points {
            path.line_to(*point);
        }
        path.line_to(Point::new(points[points.len() - 1].x, size.height));
        path.close();
    });
    let fade = gradient::Linear::new(Point::new(0.0, 0.0), Point::new(0.0, size.height))
        .add_stop(0.0, color.scale_alpha(0.22))
        .add_stop(1.0, color.scale_alpha(0.0));
    frame.fill(&area, canvas::Gradient::Linear(fade));

    let stroke = Path::new(|path| {
        path.move_to(points[0]);
        for point in &points[1..] {
            path.line_to(*point);
        }
    });
    frame.stroke(
        &stroke,
        Stroke::default()
            .with_color(color)
            .with_width(1.5)
            .with_line_join(canvas::LineJoin::Round),
    );

    if let Some(last) = points.last() {
        frame.fill(&Path::circle(*last, 2.5), color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_colour_of_a_change_follows_its_meaning() {
        let worse = Delta::new(8.2, "+8,2 %").better(Better::Down);
        let better = Delta::new(-3.0, "−3 dk").better(Better::Down);
        let gain = Delta::new(4.0, "+4").better(Better::Up);
        let neutral = Delta::new(4.0, "+4").better(Better::Neither);

        assert_eq!(worse.severity(), Some(Severity::Error));
        assert_eq!(better.severity(), Some(Severity::Success));
        assert_eq!(gain.severity(), Some(Severity::Success));
        assert_eq!(neutral.severity(), None);
        assert_eq!(Delta::new(0.0, "0").severity(), None);
    }
}
