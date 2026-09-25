//! Aralık kaydırıcısı: iki tutamakla alt ve üst sınır. İsteğe bağlı
//! histogram verinin dağılımını gösterir; seçili aralıktaki çubuklar vurgu
//! rengindedir.
//!
//! ```text
//!        ▁▃▆█▇▅▃▂▁
//! ───────●━━━━━━━━●───────
//! ```
//!
//! Tutamaklar sürüklenir; aradaki vurgulu parça sürüklenince aralık bütün
//! olarak kayar. Rayın boş yerine basmak en yakın tutamağı oraya taşır.
//! Tutamaklar birbirinin önüne geçemez.
//!
//! ```ignore
//! RangeSlider::new(0.0..=100.0, self.range, Message::RangeChanged)
//!     .step(1.0)
//!     .histogram(&self.counts)
//! ```

use std::ops::RangeInclusive;

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::{
    Background, Border, Element, Event, Length, Point, Rectangle, Renderer, Shadow, Size, Theme,
    Vector, mouse,
};

use crate::theme::Tokens;

/// Tutamağın çapı, rayın kalınlığı ve histogramın varsayılan yüksekliği.
const THUMB: f32 = 14.0;
const TRACK: f32 = 4.0;
const HISTOGRAM: f32 = 44.0;
/// Histogramla ray arası.
const GAP: f32 = 4.0;

/// Aralık kaydırıcısı.
pub struct RangeSlider<'a, Message> {
    bounds: RangeInclusive<f64>,
    value: (f64, f64),
    on_change: Box<dyn Fn((f64, f64)) -> Message + 'a>,
    on_release: Option<Message>,
    step: f64,
    histogram: Vec<f32>,
    histogram_height: f32,
    width: Length,
}

impl<'a, Message: Clone + 'a> RangeSlider<'a, Message> {
    /// `bounds` seçilebilecek bütün aralık, `value` seçili alt ve üst sınır.
    pub fn new(
        bounds: RangeInclusive<f64>,
        value: (f64, f64),
        on_change: impl Fn((f64, f64)) -> Message + 'a,
    ) -> Self {
        Self {
            bounds,
            value,
            on_change: Box::new(on_change),
            on_release: None,
            step: 0.0,
            histogram: Vec::new(),
            histogram_height: HISTOGRAM,
            width: Length::Fill,
        }
    }

    /// Değerler bu adımın katlarına oturur; 0 adımsızdır.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step.max(0.0);
        self
    }

    /// Dağılım: `bounds`'u eşit bölen aralıklardaki sayılar.
    pub fn histogram(mut self, counts: &[f32]) -> Self {
        self.histogram = counts.to_vec();
        self
    }

    pub fn histogram_height(mut self, height: f32) -> Self {
        self.histogram_height = height;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sürükleme bitince gönderilen mesaj (ör. süzgeci o zaman uygulamak
    /// için).
    pub fn on_release(mut self, message: Message) -> Self {
        self.on_release = Some(message);
        self
    }

    fn span(&self) -> f64 {
        (self.bounds.end() - self.bounds.start()).max(f64::EPSILON)
    }

    /// Rayın ilk ve son noktası (tutamak merkezlerinin gidebileceği yer).
    fn rail(&self, bounds: Rectangle) -> (f32, f32) {
        (
            bounds.x + THUMB / 2.0,
            bounds.x + bounds.width - THUMB / 2.0,
        )
    }

    fn x_of(&self, bounds: Rectangle, value: f64) -> f32 {
        let (start, end) = self.rail(bounds);
        let t = ((value - self.bounds.start()) / self.span()).clamp(0.0, 1.0) as f32;

        start + t * (end - start)
    }

    fn value_at(&self, bounds: Rectangle, x: f32) -> f64 {
        let (start, end) = self.rail(bounds);
        let t = f64::from(((x - start) / (end - start).max(1.0)).clamp(0.0, 1.0));

        self.snap(self.bounds.start() + t * self.span())
    }

    fn snap(&self, value: f64) -> f64 {
        let value = value.clamp(*self.bounds.start(), *self.bounds.end());

        if self.step > 0.0 {
            let steps = ((value - self.bounds.start()) / self.step).round();
            (self.bounds.start() + steps * self.step).min(*self.bounds.end())
        } else {
            value
        }
    }

    fn track_y(&self) -> f32 {
        let histogram = if self.histogram.is_empty() {
            0.0
        } else {
            self.histogram_height + GAP
        };

        histogram + THUMB / 2.0
    }
}

/// Sürüklenen parça.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Grip {
    Low,
    High,
    /// İki tutamak birlikte: başlangıçtaki değerler ve imleç.
    Both {
        origin: f32,
        start: (f64, f64),
    },
}

impl Grip {
    /// Parçanın türü; başlangıç değerlerinden bağımsız karşılaştırma için.
    fn kind(self) -> u8 {
        match self {
            Grip::Low => 0,
            Grip::High => 1,
            Grip::Both { .. } => 2,
        }
    }
}

#[derive(Debug, Default)]
struct State {
    grip: Option<Grip>,
    hovered: Option<Grip>,
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for RangeSlider<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, limits: &layout::Limits) -> Node {
        let height = self.track_y() + THUMB / 2.0 + 1.0;

        layout::atomic(limits, self.width, height)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();
        let (low, high) = self.value;
        let (low_x, high_x) = (self.x_of(bounds, low), self.x_of(bounds, high));
        let track_y = bounds.y + self.track_y();

        // İmlecin altındaki parça: tutamaklar, aradaki vurgulu parça.
        let grip_at = |point: Point| -> Option<Grip> {
            if (point.y - track_y).abs() > THUMB {
                return None;
            }

            let near = |x: f32| (point.x - x).abs() <= THUMB / 2.0 + 2.0;

            // Üst üste binen tutamaklardan imlecin yönündeki seçilir.
            match (near(low_x), near(high_x)) {
                (true, true) => Some(if point.x > low_x {
                    Grip::High
                } else {
                    Grip::Low
                }),
                (true, false) => Some(Grip::Low),
                (false, true) => Some(Grip::High),
                (false, false) if point.x > low_x && point.x < high_x => Some(Grip::Both {
                    origin: point.x,
                    start: (low, high),
                }),
                (false, false) => None,
            }
        };

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(point) = cursor.position_over(bounds) else {
                    return;
                };

                let grip = match grip_at(point) {
                    Some(grip) => grip,
                    // Rayın boş yeri: en yakın tutamak oraya gelir.
                    None if (point.y - track_y).abs() <= THUMB => {
                        let value = self.value_at(bounds, point.x);
                        let (grip, next) = if (point.x - low_x).abs() <= (point.x - high_x).abs() {
                            (Grip::Low, (value.min(high), high))
                        } else {
                            (Grip::High, (low, value.max(low)))
                        };

                        shell.publish((self.on_change)(next));
                        grip
                    }
                    None => return,
                };

                state.grip = Some(grip);
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let Some(grip) = state.grip else {
                    let hovered = cursor.position_over(bounds).and_then(grip_at);

                    if hovered.map(Grip::kind) != state.hovered.map(Grip::kind) {
                        state.hovered = hovered;
                        shell.request_redraw();
                    }

                    return;
                };

                let next = match grip {
                    Grip::Low => (self.value_at(bounds, position.x).min(high), high),
                    Grip::High => (low, self.value_at(bounds, position.x).max(low)),
                    Grip::Both { origin, start } => {
                        let (rail_start, rail_end) = self.rail(bounds);
                        let per_pixel = self.span() / f64::from((rail_end - rail_start).max(1.0));
                        let width = start.1 - start.0;
                        let shifted =
                            self.snap(start.0 + f64::from(position.x - origin) * per_pixel);
                        let low = shifted.clamp(*self.bounds.start(), self.bounds.end() - width);

                        (low, low + width)
                    }
                };

                if next != self.value {
                    shell.publish((self.on_change)(next));
                }

                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.grip.take().is_some() =>
            {
                if let Some(message) = &self.on_release {
                    shell.publish(message.clone());
                }

                shell.capture_event();
                shell.request_redraw();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();

        match (state.grip, state.hovered) {
            (Some(_), _) => mouse::Interaction::Grabbing,
            (None, Some(_)) if cursor.is_over(layout.bounds()) => mouse::Interaction::Grab,
            _ if cursor.is_over(layout.bounds()) => mouse::Interaction::Pointer,
            _ => mouse::Interaction::None,
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let (low, high) = self.value;
        let (low_x, high_x) = (self.x_of(bounds, low), self.x_of(bounds, high));
        let track_y = bounds.y + self.track_y();

        // Histogram: seçili aralıktaki çubuklar vurgu renginde.
        if !self.histogram.is_empty() {
            let peak = self
                .histogram
                .iter()
                .copied()
                .fold(0.0_f32, f32::max)
                .max(1e-6);
            let (rail_start, rail_end) = self.rail(bounds);
            let width = (rail_end - rail_start) / self.histogram.len() as f32;

            for (index, count) in self.histogram.iter().enumerate() {
                let height = (count / peak * self.histogram_height).max(if *count > 0.0 {
                    1.0
                } else {
                    0.0
                });
                let x = rail_start + index as f32 * width;
                let middle = x + width / 2.0;
                let inside = middle >= low_x && middle <= high_x;

                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(
                            Point::new(x + 0.5, bounds.y + self.histogram_height - height),
                            Size::new((width - 1.0).max(1.0), height),
                        ),
                        border: Border {
                            radius: 1.0.into(),
                            ..Border::default()
                        },
                        ..Quad::default()
                    },
                    Background::Color(if inside {
                        t.accent.scale_alpha(0.75)
                    } else {
                        t.muted.scale_alpha(0.3)
                    }),
                );
            }
        }

        let (rail_start, rail_end) = self.rail(bounds);
        let rail = |from: f32, to: f32| {
            Rectangle::new(
                Point::new(from, track_y - TRACK / 2.0),
                Size::new((to - from).max(0.0), TRACK),
            )
        };

        renderer.fill_quad(
            Quad {
                bounds: rail(rail_start, rail_end),
                border: Border {
                    radius: (TRACK / 2.0).into(),
                    ..Border::default()
                },
                ..Quad::default()
            },
            Background::Color(t.border),
        );
        renderer.fill_quad(
            Quad {
                bounds: rail(low_x, high_x),
                border: Border {
                    radius: (TRACK / 2.0).into(),
                    ..Border::default()
                },
                ..Quad::default()
            },
            Background::Color(t.accent),
        );

        for (x, grip) in [(low_x, Grip::Low), (high_x, Grip::High)] {
            let same = |other: Option<Grip>| other.is_some_and(|other| other.kind() == grip.kind());
            let hot = same(state.grip) || same(state.hovered);

            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(
                        Point::new(x - THUMB / 2.0, track_y - THUMB / 2.0),
                        Size::new(THUMB, THUMB),
                    ),
                    border: Border {
                        color: t.accent,
                        width: 2.0,
                        radius: (THUMB / 2.0).into(),
                    },
                    shadow: Shadow {
                        color: t.shadow(),
                        offset: Vector::new(0.0, 1.0),
                        blur_radius: if hot { 6.0 } else { 2.0 },
                    },
                    ..Quad::default()
                },
                Background::Color(if hot { t.text } else { t.popover }),
            );
        }
    }
}

impl<'a, Message: Clone + 'a> From<RangeSlider<'a, Message>> for Element<'a, Message> {
    fn from(slider: RangeSlider<'a, Message>) -> Self {
        Element::new(slider)
    }
}

/// Değerleri `bins` eşit aralığa sayar: histogram için.
pub fn histogram(
    values: impl IntoIterator<Item = f64>,
    bounds: RangeInclusive<f64>,
    bins: usize,
) -> Vec<f32> {
    let bins = bins.max(1);
    let span = (bounds.end() - bounds.start()).max(f64::EPSILON);
    let mut counts = vec![0.0; bins];

    for value in values {
        if value < *bounds.start() || value > *bounds.end() {
            continue;
        }

        let index = (((value - bounds.start()) / span) * bins as f64) as usize;
        counts[index.min(bins - 1)] += 1.0;
    }

    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slider() -> RangeSlider<'static, ()> {
        RangeSlider::new(0.0..=100.0, (20.0, 60.0), |_| ()).step(5.0)
    }

    #[test]
    fn positions_map_to_snapped_values() {
        let slider = slider();
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(114.0, 20.0));

        // Ray tutamağın yarısı kadar içeriden başlar: 7..107.
        assert_eq!(slider.x_of(bounds, 0.0), 7.0);
        assert_eq!(slider.x_of(bounds, 50.0), 57.0);
        assert_eq!(slider.value_at(bounds, 7.0 + 23.0), 25.0);
        assert_eq!(slider.value_at(bounds, 7.0 + 21.0), 20.0);
        assert_eq!(slider.value_at(bounds, -50.0), 0.0);
        assert_eq!(slider.value_at(bounds, 500.0), 100.0);
    }

    #[test]
    fn histograms_count_values_into_bins() {
        let counts = histogram([0.0, 5.0, 10.0, 99.0, 100.0, 150.0], 0.0..=100.0, 10);

        assert_eq!(counts.len(), 10);
        assert_eq!(counts[0], 2.0);
        assert_eq!(counts[1], 1.0);
        assert_eq!(counts[9], 2.0);
        assert_eq!(counts.iter().sum::<f32>(), 5.0);
    }
}
