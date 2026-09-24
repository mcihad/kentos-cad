//! Sürüklenerek komşu alanın boyutunu değiştiren ince tutamak (sash).
//!
//! ```text
//!   harita          ┃ yan panel
//!                   ┃◄──► sürükle: panelin genişliği değişir
//!                   ┃     çift tık: varsayılan genişlik
//! ```
//!
//! Tutamak yerleşimde birkaç piksel yer kaplar; kenarında bölücü çizgi
//! bulunur. Üzerine gelince imleç boyutlandırma okuna döner ve çizgi vurgu
//! rengine geçer. Sürüklerken yeni boyut sürekli bildirilir ([`Sash::vertical`]),
//! bırakınca bir kez daha ([`Sash::on_release`]); böylece uygulama boyutu
//! sürüklerken çizer, bırakınca saklar.
//!
//! ```ignore
//! Sash::vertical(width, Message::DockResized)
//!     .reverse() // alan tutamağın sağında: sola sürüklemek büyütür
//!     .range(240.0..=720.0)
//!     .on_release(Message::DockResizeEnded)
//!     .on_double_click(Message::DockReset)
//! ```

use std::ops::RangeInclusive;
use std::time::Duration;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, renderer};
use iced::time::Instant;
use iced::{Background, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

use crate::theme::Tokens;

/// Tutamağın kalınlığı: tutması kolay, görünüşte ince.
pub const THICKNESS: f32 = 5.0;

/// Çift tık sayılacak en uzun aralık.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// Tutamağın yönü.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    /// Dikey çizgi; yandaki alanın genişliğini değiştirir.
    Vertical,
    /// Yatay çizgi; üstteki ya da alttaki alanın yüksekliğini değiştirir.
    Horizontal,
}

/// Boyutlandırma tutamağı.
pub struct Sash<'a, Message> {
    axis: Axis,
    size: f32,
    range: RangeInclusive<f32>,
    reverse: bool,
    /// Tutamağın öbür yanında kalması gereken en az alan.
    keep: f32,
    on_resize: Box<dyn Fn(f32) -> Message + 'a>,
    on_release: Option<Message>,
    on_double_click: Option<Message>,
}

impl<'a, Message: Clone + 'a> Sash<'a, Message> {
    /// Genişliği `size` piksel olan alanın dikey tutamağı; sürüklerken yeni
    /// genişliği `on_resize` ile bildirir. Alan tutamağın sağındaysa
    /// [`reverse`](Self::reverse) verilir.
    pub fn vertical(size: f32, on_resize: impl Fn(f32) -> Message + 'a) -> Self {
        Self::new(Axis::Vertical, size, on_resize)
    }

    /// Yüksekliği `size` piksel olan alanın yatay tutamağı.
    pub fn horizontal(size: f32, on_resize: impl Fn(f32) -> Message + 'a) -> Self {
        Self::new(Axis::Horizontal, size, on_resize)
    }

    fn new(axis: Axis, size: f32, on_resize: impl Fn(f32) -> Message + 'a) -> Self {
        Self {
            axis,
            size,
            range: 0.0..=f32::INFINITY,
            reverse: false,
            keep: 0.0,
            on_resize: Box::new(on_resize),
            on_release: None,
            on_double_click: None,
        }
    }

    /// Alan tutamağın sağında (ya da altında): sola (yukarı) sürüklemek
    /// alanı büyütür.
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// Alanın alabileceği boyutlar (piksel).
    pub fn range(mut self, range: RangeInclusive<f32>) -> Self {
        self.range = range;
        self
    }

    /// Pencerenin geri kalanında en az bu kadar yer kalır (ör. harita
    /// kaybolmasın).
    pub fn keep(mut self, pixels: f32) -> Self {
        self.keep = pixels;
        self
    }

    /// Sürükleme bitince gönderilen mesaj (ör. boyutu saklamak için).
    pub fn on_release(mut self, message: Message) -> Self {
        self.on_release = Some(message);
        self
    }

    /// Çift tıklanınca gönderilen mesaj (ör. varsayılan boyuta dönmek).
    pub fn on_double_click(mut self, message: Message) -> Self {
        self.on_double_click = Some(message);
        self
    }

    /// İmlecin tutamak yönündeki konumu.
    fn along(&self, point: Point) -> f32 {
        match self.axis {
            Axis::Vertical => point.x,
            Axis::Horizontal => point.y,
        }
    }

    /// Sürüklemenin getirdiği boyut: sınırlar içinde, tam piksel.
    fn resized(&self, drag: Drag, position: Point, viewport: &Rectangle) -> f32 {
        let moved = self.along(position) - drag.origin;
        let moved = if self.reverse { -moved } else { moved };

        let room = match self.axis {
            Axis::Vertical => viewport.width,
            Axis::Horizontal => viewport.height,
        } - self.keep
            - THICKNESS;
        let largest = self.range.end().min(room).max(*self.range.start());

        (drag.size + moved)
            .clamp(*self.range.start(), largest)
            .round()
    }
}

/// Süren sürükleme: başladığı imleç konumu ve alanın o anki boyutu.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Drag {
    origin: f32,
    size: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct State {
    drag: Option<Drag>,
    hovered: bool,
    /// Son basışın zamanı; çift tıkı tanımak için.
    last_press: Option<Instant>,
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Sash<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        match self.axis {
            Axis::Vertical => Size::new(Length::Fixed(THICKNESS), Length::Fill),
            Axis::Horizontal => Size::new(Length::Fill, Length::Fixed(THICKNESS)),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = self.size();

        layout::atomic(limits, size.width, size.height)
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
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();

        match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(drag) = state.drag {
                    let size = self.resized(drag, *position, viewport);

                    if (size - self.size).abs() >= 0.5 {
                        shell.publish((self.on_resize)(size));
                    }

                    shell.capture_event();
                }

                let hovered = cursor.is_over(bounds);

                if hovered != state.hovered {
                    state.hovered = hovered;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorLeft) => {
                if state.hovered {
                    state.hovered = false;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor.position_over(bounds) else {
                    return;
                };

                let now = Instant::now();
                let double = state
                    .last_press
                    .is_some_and(|last| now.duration_since(last) <= DOUBLE_CLICK);

                shell.capture_event();
                shell.request_redraw();

                if double && let Some(message) = self.on_double_click.clone() {
                    state.last_press = None;
                    state.drag = None;
                    shell.publish(message);
                    return;
                }

                state.last_press = Some(now);
                state.drag = Some(Drag {
                    origin: self.along(position),
                    size: self.size,
                });
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.drag.is_some() =>
            {
                state.drag = None;

                if let Some(message) = self.on_release.clone() {
                    shell.publish(message);
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

        if state.drag.is_some() || cursor.is_over(layout.bounds()) {
            match self.axis {
                Axis::Vertical => mouse::Interaction::ResizingColumn,
                Axis::Horizontal => mouse::Interaction::ResizingRow,
            }
        } else {
            mouse::Interaction::None
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

        // Tutamak, boyutunu değiştirdiği alanın zeminindedir; çizgi öbür
        // alanla sınırdadır.
        let (line, width) = match (state.drag.is_some(), state.hovered) {
            (true, _) => (t.accent, 2.0),
            (false, true) => (t.accent.scale_alpha(0.7), 2.0),
            (false, false) => (t.border, 1.0),
        };

        let edge = match (self.axis, self.reverse) {
            (Axis::Vertical, true) => Rectangle { width, ..bounds },
            (Axis::Vertical, false) => Rectangle {
                x: bounds.x + bounds.width - width,
                width,
                ..bounds
            },
            (Axis::Horizontal, true) => Rectangle {
                height: width,
                ..bounds
            },
            (Axis::Horizontal, false) => Rectangle {
                y: bounds.y + bounds.height - width,
                height: width,
                ..bounds
            },
        };

        let fill = |renderer: &mut Renderer, bounds: Rectangle, color| {
            renderer::Renderer::fill_quad(
                renderer,
                renderer::Quad {
                    bounds,
                    ..renderer::Quad::default()
                },
                Background::Color(color),
            );
        };

        fill(renderer, bounds, t.surface);
        fill(renderer, edge, line);
    }
}

impl<'a, Message: Clone + 'a> From<Sash<'a, Message>> for Element<'a, Message> {
    fn from(sash: Sash<'a, Message>) -> Self {
        Element::new(sash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sash() -> Sash<'static, f32> {
        Sash::vertical(300.0, |size| size)
            .reverse()
            .range(240.0..=720.0)
            .keep(400.0)
    }

    #[test]
    fn dragging_toward_the_area_shrinks_it() {
        let sash = sash();
        let window = Rectangle::new(Point::ORIGIN, Size::new(1440.0, 900.0));
        let drag = Drag {
            origin: 1000.0,
            size: 300.0,
        };

        // Alan sağda: sola sürüklemek büyütür, sağa sürüklemek küçültür.
        assert_eq!(sash.resized(drag, Point::new(900.0, 0.0), &window), 400.0);
        assert_eq!(sash.resized(drag, Point::new(1030.0, 0.0), &window), 270.0);
    }

    #[test]
    fn size_stays_within_the_range_and_the_window() {
        let sash = sash();
        let window = Rectangle::new(Point::ORIGIN, Size::new(1440.0, 900.0));
        let drag = Drag {
            origin: 1000.0,
            size: 300.0,
        };

        assert_eq!(sash.resized(drag, Point::new(1400.0, 0.0), &window), 240.0);
        assert_eq!(sash.resized(drag, Point::new(0.0, 0.0), &window), 720.0);

        // Dar pencerede geri kalan alana en az 400 piksel bırakılır.
        let narrow = Rectangle::new(Point::ORIGIN, Size::new(900.0, 700.0));
        assert_eq!(sash.resized(drag, Point::new(0.0, 0.0), &narrow), 495.0);
    }
}
