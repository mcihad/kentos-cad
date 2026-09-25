//! İlerleme: ilerleme çubuğu, dönen gösterge ve iptal edilebilen görev
//! listesi.
//!
//! ```text
//! ⠶  GeoJSON olarak dışa aktar                         %45  ■
//!    Türkiye.geojson: 27 / 60 öğe
//!    ▬▬▬▬▬▬▬▬▬▬▬▬▬▬────────────────────
//! ◷  Uzamsal dizin oluştur                          Sırada  ×
//! ✓  Paftaları içe aktar                             Bitti  ×
//! ⓧ  DXF olarak dışa aktar                      Başarısız  ×
//!    Çizgi tipi desteklenmiyor.  Yeniden dene
//! ```
//!
//! - [`bar`] ince ilerleme çubuğudur. Oranı bilinmeyen işte (`None`) çubuğun
//!   üzerinde bir parça soldan sağa kayar.
//! - [`spinner`] süren işin küçük, dönen göstergesidir: çember üzerinde
//!   sekiz nokta, öndeki en koyu.
//! - [`TaskList`] arka plandaki işleri durumlarıyla sıralar: sürüyor,
//!   sırada, bitti, başarısız, iptal edildi. Süren ve sıradaki iş iptal
//!   edilir, başarısız iş yeniden denenir, biten iş listeden kaldırılır.
//!
//! ```ignore
//! TaskList::new()
//!     .push(
//!         Task::new("GeoJSON olarak dışa aktar")
//!             .detail("Türkiye.geojson: 27 / 60 öğe")
//!             .running(Some(0.45))
//!             .on_cancel(Message::Cancel(id)),
//!     )
//!     .push(Task::new("Uzamsal dizin oluştur").on_cancel(Message::Cancel(other)))
//! ```

use std::f32::consts::TAU;
use std::time::Duration;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::time::Instant;
use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, Row, button, column, container, row, space, tooltip};
use iced::{
    Background, Center, Color, Element, Event, Fill, Length, Point, Rectangle, Renderer, Size,
    Theme, border, mouse, window,
};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::Tokens;
use crate::widget::{Tip, horizontal_divider, tip};

/// Canlandırmada iki kare arası.
const FRAME: Duration = Duration::from_millis(33);

/// Oranı bilinmeyen çubukta kayan parçanın bir geçişi.
const SWEEP: Duration = Duration::from_millis(1400);

/// Dönen göstergenin bir tam turu.
const TURN: Duration = Duration::from_millis(900);

/// Çubuğun ve göstergenin rengi.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Tint {
    /// Vurgu rengi: süren iş.
    #[default]
    Accent,
    /// Başarıyla biten iş.
    Success,
    /// Başarısız iş.
    Danger,
    /// Sönük: duraklayan ya da iptal edilen iş.
    Muted,
}

impl Tint {
    fn color(self, t: &Tokens) -> Color {
        match self {
            Tint::Accent => t.accent,
            Tint::Success => t.success,
            Tint::Danger => t.danger,
            Tint::Muted => t.muted,
        }
    }
}

/// İnce ilerleme çubuğu: `Some(oran)` 0..=1 arası, `None` oranı bilinmeyen
/// iş.
pub fn bar(value: Option<f32>) -> ProgressBar {
    ProgressBar {
        value: value.map(|value| value.clamp(0.0, 1.0)),
        width: Length::Fill,
        height: 4.0,
        tint: Tint::Accent,
    }
}

/// İlerleme çubuğu; bkz. [`bar`].
#[derive(Debug, Clone, Copy)]
pub struct ProgressBar {
    value: Option<f32>,
    width: Length,
    height: f32,
    tint: Tint,
}

impl ProgressBar {
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Kalınlık (piksel); varsayılanı 4.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn tint(mut self, tint: Tint) -> Self {
        self.tint = tint;
        self
    }
}

/// Canlandırmanın başladığı an; kayan parçanın ve dönen göstergenin
/// yerini zaman belirler.
struct Clock {
    started: Instant,
}

impl Clock {
    fn new() -> Self {
        Self {
            started: Instant::now(),
        }
    }

    /// `period` uzunluğundaki döngüde nerede olunduğu: 0..1.
    fn phase(&self, now: Instant, period: Duration) -> f32 {
        let elapsed = now.saturating_duration_since(self.started).as_secs_f32();

        (elapsed / period.as_secs_f32()).fract()
    }
}

impl<Message> Widget<Message, Theme, Renderer> for ProgressBar {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Clock>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Clock::new())
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Fixed(self.height))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.width, self.height)
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        // Oranı bilinmeyen çubuk kendi kendine kayar.
        if self.value.is_none()
            && let Event::Window(window::Event::RedrawRequested(now)) = event
        {
            shell.request_redraw_at(*now + FRAME);
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
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let radius = bounds.height / 2.0;

        rounded(renderer, bounds, radius, t.layer(0.1));

        let fill = match self.value {
            Some(value) => Rectangle {
                width: (bounds.width * value).round(),
                ..bounds
            },
            None => {
                // Kayan parça: çubuğun üçte biri, yavaşlayarak girip çıkar.
                // İlk karede de görünsün diye geçişin ortasından başlar.
                let phase = (tree
                    .state
                    .downcast_ref::<Clock>()
                    .phase(Instant::now(), SWEEP)
                    + 0.4)
                    .fract();
                let eased = 0.5 - 0.5 * (phase * std::f32::consts::PI).cos();
                let length = bounds.width / 3.0;
                let start = bounds.x - length + (bounds.width + length) * eased;
                let end = (start + length).min(bounds.x + bounds.width);
                let start = start.max(bounds.x);

                Rectangle {
                    x: start,
                    width: (end - start).max(0.0),
                    ..bounds
                }
            }
        };

        if fill.width > 0.0 {
            rounded(renderer, fill, radius, self.tint.color(&t));
        }
    }
}

impl<'a, Message> From<ProgressBar> for Element<'a, Message> {
    fn from(bar: ProgressBar) -> Self {
        Element::new(bar)
    }
}

/// Süren işin dönen göstergesi; varsayılan boyu 14 piksel.
pub fn spinner() -> Spinner {
    Spinner {
        size: 14.0,
        tint: Tint::Accent,
    }
}

/// Dönen gösterge; bkz. [`spinner`].
#[derive(Debug, Clone, Copy)]
pub struct Spinner {
    size: f32,
    tint: Tint,
}

impl Spinner {
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn tint(mut self, tint: Tint) -> Self {
        self.tint = tint;
        self
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Spinner {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Clock>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Clock::new())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.size), Length::Fixed(self.size))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.size, self.size)
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            shell.request_redraw_at(*now + FRAME);
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
        const DOTS: usize = 8;

        let t = Tokens::of(theme);
        let color = self.tint.color(&t);
        let bounds = layout.bounds();
        let center = bounds.center();
        let dot = (bounds.width * 0.11).max(1.0);
        let radius = bounds.width / 2.0 - dot;

        // Öndeki nokta tam renkte; arkadakiler giderek silikleşir.
        let lead = (tree
            .state
            .downcast_ref::<Clock>()
            .phase(Instant::now(), TURN)
            * DOTS as f32)
            .floor() as usize;

        for index in 0..DOTS {
            let behind = (lead + DOTS - index) % DOTS;
            let alpha = 1.0 - behind as f32 / DOTS as f32 * 0.85;
            let angle = index as f32 / DOTS as f32 * TAU - TAU / 4.0;
            let position = Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            );

            rounded(
                renderer,
                Rectangle::new(
                    Point::new(position.x - dot, position.y - dot),
                    Size::new(2.0 * dot, 2.0 * dot),
                ),
                dot,
                color.scale_alpha(alpha),
            );
        }
    }
}

impl<'a, Message> From<Spinner> for Element<'a, Message> {
    fn from(spinner: Spinner) -> Self {
        Element::new(spinner)
    }
}

fn rounded(renderer: &mut Renderer, bounds: Rectangle, radius: f32, color: Color) {
    renderer.fill_quad(
        Quad {
            bounds,
            border: border::rounded(radius),
            ..Quad::default()
        },
        Background::Color(color),
    );
}

/// Görevin durumu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    /// Sırasını bekliyor.
    Queued,
    /// Sürüyor; oranı bilinmiyorsa `None`.
    Running(Option<f32>),
    Done,
    Failed,
    Cancelled,
}

/// Görev listesinin bir satırı.
pub struct Task<'a, Message> {
    title: Fragment<'a>,
    detail: Option<Fragment<'a>>,
    state: State,
    on_cancel: Option<Message>,
    on_retry: Option<Message>,
    on_dismiss: Option<Message>,
}

impl<'a, Message: Clone + 'a> Task<'a, Message> {
    /// Sıradaki görev.
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            detail: None,
            state: State::Queued,
            on_cancel: None,
            on_retry: None,
            on_dismiss: None,
        }
    }

    /// Başlığın altındaki ayrıntı (ör. "27 / 60 öğe" ya da hatanın nedeni).
    pub fn detail(mut self, detail: impl IntoFragment<'a>) -> Self {
        self.detail = Some(detail.into_fragment());
        self
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }

    /// Sürüyor; oranı bilinmiyorsa `None`.
    pub fn running(self, progress: Option<f32>) -> Self {
        self.state(State::Running(progress))
    }

    /// Süren ya da sıradaki görevin iptal düğmesi.
    pub fn on_cancel(mut self, message: Message) -> Self {
        self.on_cancel = Some(message);
        self
    }

    /// Başarısız görevin "Yeniden dene" bağlantısı.
    pub fn on_retry(mut self, message: Message) -> Self {
        self.on_retry = Some(message);
        self
    }

    /// Biten, başarısız ya da iptal edilen görevi listeden kaldıran düğme.
    pub fn on_dismiss(mut self, message: Message) -> Self {
        self.on_dismiss = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Task<'a, Message>> for Element<'a, Message> {
    fn from(task: Task<'a, Message>) -> Self {
        let (status, note, tone): (Element<'a, Message>, String, Tone) = match task.state {
            State::Queued => (
                icon(Icon::Clock).size(14.0).tone(Tone::Muted).into(),
                "Sırada".to_owned(),
                Tone::Muted,
            ),
            State::Running(progress) => (
                spinner().into(),
                progress.map_or_else(
                    || "Sürüyor".to_owned(),
                    |progress| format!("%{:.0}", progress * 100.0),
                ),
                Tone::Text,
            ),
            State::Done => (
                icon(Icon::Success).size(14.0).tone(Tone::Success).into(),
                "Bitti".to_owned(),
                Tone::Muted,
            ),
            State::Failed => (
                icon(Icon::Error).size(14.0).tone(Tone::Danger).into(),
                "Başarısız".to_owned(),
                Tone::Danger,
            ),
            State::Cancelled => (
                icon(Icon::Stop).size(14.0).tone(Tone::Muted).into(),
                "İptal edildi".to_owned(),
                Tone::Muted,
            ),
        };

        let active = matches!(task.state, State::Queued | State::Running(_));

        let title = label::strong(task.title).style(if task.state == State::Cancelled {
            style::text::muted
        } else {
            style::text::default
        });

        let note = if matches!(task.state, State::Running(Some(_))) {
            label::mono_caption(note)
        } else {
            label::caption(note)
        }
        .style(move |theme: &Theme| {
            let t = Tokens::of(theme);

            iced::widget::text::Style {
                color: Some(match tone {
                    Tone::Danger => t.danger,
                    Tone::Text => t.text,
                    _ => t.muted,
                }),
            }
        });

        let mut text = Column::new()
            .push(row![title.width(Fill), note].spacing(8).align_y(Center))
            .spacing(3)
            .width(Fill);

        let mut detail_row = Row::new().spacing(10).align_y(Center);

        if let Some(detail) = task.detail {
            detail_row = detail_row.push(label::caption(detail).width(Fill));
        } else {
            detail_row = detail_row.push(space::horizontal());
        }

        if task.state == State::Failed
            && let Some(retry) = task.on_retry
        {
            detail_row = detail_row.push(
                button(label::caption("Yeniden dene").style(style::text::accent))
                    .on_press(retry)
                    .padding([1, 4])
                    .style(style::button::ghost),
            );
        }

        text = text.push(detail_row);

        if let State::Running(progress) = task.state {
            text = text.push(container(bar(progress)).padding(iced::padding::top(3)));
        }

        let action = if active {
            task.on_cancel
                .map(|message| control(Icon::Stop, "İptal et", message))
        } else {
            task.on_dismiss
                .map(|message| control(Icon::Close, "Listeden kaldır", message))
        };

        let mut line = row![
            container(status)
                .width(16)
                .height(18)
                .center_x(16)
                .center_y(18),
            text,
        ]
        .spacing(10);

        line = match action {
            Some(action) => line.push(action),
            None => line.push(space::horizontal().width(22)),
        };

        container(line).padding([8, 10]).width(Fill).into()
    }
}

/// Satırın sağındaki küçük ikon düğmesi.
fn control<'a, Message: Clone + 'a>(
    glyph: Icon,
    description: &'static str,
    message: Message,
) -> Element<'a, Message> {
    tip(
        button(icon(glyph).size(12.0))
            .on_press(message)
            .padding(4)
            .style(style::button::ghost),
        Tip::new(description),
        tooltip::Position::Left,
    )
}

/// Arka plandaki işlerin listesi; satırlar arasında bölücü çizgi bulunur.
pub struct TaskList<'a, Message> {
    tasks: Vec<Task<'a, Message>>,
}

impl<'a, Message: Clone + 'a> TaskList<'a, Message> {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn push(mut self, task: Task<'a, Message>) -> Self {
        self.tasks.push(task);
        self
    }

    pub fn extend(mut self, tasks: impl IntoIterator<Item = Task<'a, Message>>) -> Self {
        self.tasks.extend(tasks);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

impl<'a, Message: Clone + 'a> Default for TaskList<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<TaskList<'a, Message>> for Element<'a, Message> {
    fn from(list: TaskList<'a, Message>) -> Self {
        let mut rows = column![].width(Fill);

        for (index, task) in list.tasks.into_iter().enumerate() {
            if index > 0 {
                rows = rows.push(horizontal_divider());
            }

            rows = rows.push(task);
        }

        rows.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_wrap_around() {
        let clock = Clock::new();
        let later = clock.started + Duration::from_millis(2100);

        assert!((clock.phase(later, SWEEP) - 0.5).abs() < 1e-3);
        assert!((clock.phase(clock.started, TURN)).abs() < 1e-6);
    }

    #[test]
    fn bars_clamp_their_value() {
        assert_eq!(bar(Some(1.4)).value, Some(1.0));
        assert_eq!(bar(Some(-0.2)).value, Some(0.0));
        assert_eq!(bar(None).value, None);
    }
}
