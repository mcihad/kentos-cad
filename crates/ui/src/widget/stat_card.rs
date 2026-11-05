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
    /// The value's size (none: the figure size); [`StatRow`] fits it to the card.
    value_size: Option<f32>,
    height: Length,
    /// The label on one line, its end “…” where it does not fit (in a [`StatRow`]).
    elide: bool,
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
            value_size: None,
            height: Length::Shrink,
            elide: false,
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

    /// Kartın yüksekliği (ör. bir sıradaki kartlar eşit boyda).
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Başlık tek satırda: sığmazsa sonu “…” ile kısalır.
    pub fn elide_label(mut self, elide: bool) -> Self {
        self.elide = elide;
        self
    }

    /// Değerin yazı boyutu (yoksa figür boyu). Değer hiçbir boyda alt satıra kaymaz.
    pub fn value_size(mut self, size: f32) -> Self {
        self.value_size = Some(size);
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
            value_size,
            height,
            elide,
            ..
        } = card;

        let heading: Element<'a, Message> = if elide {
            crate::widget::Elided::new(title).size(typography::body()).style(style::text::muted).width(Fill).into()
        } else {
            label::muted(title).into()
        };
        let mut top = row![heading].spacing(6).align_y(Center);

        if let Some((severity, _)) = &status {
            top = top
                .push(space::horizontal())
                .push(icon(severity.icon()).size(14.0).tone(severity.tone()));
        }

        // The value stays on one line: a card too narrow for it takes a smaller size (StatRow), never a second line.
        let mut figure = row![
            label::figure(value)
                .size(value_size.unwrap_or_else(typography::figure))
                .wrapping(iced::widget::text::Wrapping::None)
                .style(style::text::default)
        ]
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

        let body = container(content).padding([12, 14]).width(Fill).height(height);

        let card: Element<'a, Message> = match on_press {
            Some(message) => button(body)
                .on_press(message)
                .padding(0)
                .width(width)
                .height(height)
                .style(card_button(selected))
                .into(),
            None => container(body)
                .width(width)
                .height(height)
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

/// Bir değer yazısını sayıya ve birime ayırır: sondaki rakamsız sözcük(ler) birimdir
/// (“1 974 258 m³·m” → “1 974 258”, “m³·m”; “31 451 / 3 335 m³” → “31 451 / 3 335”, “m³”;
/// “Kurulur” → birim yok).
pub fn split_unit(text: &str) -> (String, Option<String>) {
    let t = text.trim();
    match t.rsplit_once(' ') {
        Some((value, unit))
            if !unit.chars().any(|c| c.is_ascii_digit())
                && value.chars().any(|c| c.is_ascii_digit())
                && unit.chars().count() <= 8 =>
        {
            (value.trim_end().to_owned(), Some(unit.to_owned()))
        }
        _ => (t.to_owned(), None),
    }
}

/// Gösterge sırası: bir sonucun birkaç değeri yan yana, eşit genişlikte ve eşit
/// yükseklikte kartlarda (ör. yol pencerelerinde resmin altındaki değerler).
///
/// Değer hiçbir zaman alt satıra kaymaz ve kartın dışına taşmaz: her kart değerine
/// kartta sığan en büyük basamağı verir ([`STEPS`]: figür boyundan gövde boyuna;
/// Grafana stat panelinin “auto” yazı boyu, iOS'un “fit to width”'i). Basamaklar
/// sabit olduğu için yan yana iki değer ya aynı boyda ya da belirgin biçimde
/// farklıdır. Birim değerin yanında küçük yazılır. Başlık tek satırdır, sığmazsa sonu
/// “…” ile kısalır (gerçek yazı tipiyle ölçülür, [`crate::widget::Elided`]): sıranın
/// yüksekliği böylece her genişlikte aynıdır ve kartın kendi yüksekliğine eşittir.
///
/// ```ignore
/// StatRow::new()
///     .figure("Taşıma momenti", "1 974 258 m³·m")   // birim kendiliğinden ayrılır
///     .push("Durum", "Kurulur", None)
/// ```
#[derive(Debug, Clone, Default)]
pub struct StatRow {
    cards: Vec<(String, String, Option<String>)>,
}

/// Değerin yazı boyu basamakları, büyükten küçüğe: figür (gövdenin 5/3'ü), 18, 16,
/// başlık, alt başlık, gövde (12 piksellik gövde metninde).
pub const STEPS: [fn() -> f32; 6] = [
    typography::figure,
    || typography::scaled(18.0),
    || typography::scaled(16.0),
    typography::title,
    typography::heading,
    typography::body,
];

/// Bir yazının yaklaşık genişliği: rakamlar ve harfler ailenin ortalama genişliğinde,
/// boşluk ve ince boşluk, nokta, virgül, eğik çizgi ve orta nokta dar (yazı tipinin
/// kendi oranlarına yakın: rakam ≈ 0,56 em, ince boşluk ≈ 0,2 em, nokta ≈ 0,28 em).
fn span(text: &str, size: f32, strong: bool) -> f32 {
    let ty = typography::current();
    let letter = ty.text_width("x", size, strong);
    text.chars()
        .map(|c| match c {
            '\u{202f}' | '\u{2009}' => 0.2 * size,
            ' ' => 0.28 * size,
            '.' | ',' | '·' | ':' | ';' | '\'' => 0.28 * size,
            '/' | '(' | ')' | '−' | '-' | '+' => 0.4 * size,
            c if c.is_ascii_digit() => 0.6 * size,
            _ => letter,
        })
        .sum()
}

/// Kartın iç boşluğu (iki yanda) ve kenarı; satır arası.
const CARD_PAD_X: f32 = 14.0;
const CARD_PAD_Y: f32 = 12.0;
const ROW_SPACING: f32 = 8.0;
/// Bir satırın yüksekliği, yazı boyunun katı (iced'in varsayılanı).
const LINE: f32 = 1.3;

impl StatRow {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bir kart: başlık, değer ve birim.
    pub fn push(mut self, label: impl Into<String>, value: impl Into<String>, unit: Option<String>) -> Self {
        self.cards.push((label.into(), value.into(), unit));
        self
    }

    /// Bir kart, birimi değer yazısının sonundan ayrılarak ([`split_unit`]).
    pub fn figure(self, label: impl Into<String>, text: impl AsRef<str>) -> Self {
        let (value, unit) = split_unit(text.as_ref());
        self.push(label, value, unit)
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Kartlar `width` genişliğe dizildiğinde her kartın değer boyu.
    pub fn fit(&self, width: f32) -> Vec<f32> {
        let n = self.cards.len().max(1) as f32;
        let room = (width - ROW_SPACING * (n - 1.0)) / n - 2.0 * CARD_PAD_X - 2.0;
        self.cards
            .iter()
            .map(|(_, value, unit)| {
                let unit_w = unit.as_ref().map_or(0.0, |u| 6.0 + span(u, typography::caption(), false));
                STEPS.iter().map(|k| k()).find(|s| span(value, *s, true) + unit_w <= room).unwrap_or_else(typography::body)
            })
            .collect()
    }

    /// Sıranın yüksekliği: kartın iç boşluğu, başlığın satırları ve figür boyunda bir satır.
    fn height(lines: usize) -> f32 {
        2.0 * CARD_PAD_Y + 2.0 + lines as f32 * LINE * typography::body() + 4.0 + LINE * typography::figure()
    }
}

impl<'a, Message: Clone + 'a> From<StatRow> for Element<'a, Message> {
    fn from(row_of: StatRow) -> Self {
        if row_of.cards.is_empty() {
            return space::vertical().height(0).into();
        }
        // The height is set before the width is known (`responsive` needs a bounded box): one line of label, one of
        // value at the figure size.
        let height = StatRow::height(1);
        let content = iced::widget::responsive(move |size| {
            let sizes = row_of.fit(size.width);
            let mut cards = iced::widget::Row::new().spacing(ROW_SPACING).height(Fill);
            for ((label, value, unit), value_size) in row_of.cards.iter().cloned().zip(sizes) {
                let mut card = StatCard::new(label, value).value_size(value_size).width(Fill).height(Fill).elide_label(true);
                if let Some(u) = unit {
                    card = card.unit(u);
                }
                cards = cards.push(card);
            }
            cards.into()
        });
        container(content).width(Fill).height(Length::Fixed(height)).into()
    }
}
