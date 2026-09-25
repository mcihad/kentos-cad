//! Pusula: görünümün döndüğü yönü gösteren kuzey oku; tıklanınca kuzeye
//! döndürür.
//!
//! ```text
//!      ╭─────╮            K
//!      │  K  │            ▲
//!      │  ▲  │           ╱█╲
//!      │  ▼  │          ╱██ ╲
//!      ╰─────╯         ╱▀▀ ▀▀╲
//!   harita düğmesi    pafta oku (sade)
//! ```
//!
//! Harita ve 3B görünümlerin köşesinde durur: görünüm döndükçe iğne kuzeyi
//! gösterir, harf hep dik kalır. Tıklanınca `on_press` (ör. kuzeye döndür)
//! gönderilir. [`Compass::plain`] zeminsiz, klasik pafta kuzey okunu çizer;
//! kâğıt düzenlerinde kullanılır. Harf Türkçede "K"dir, [`Compass::letter`]
//! ile değişir.
//!
//! ```ignore
//! Compass::new(self.north).on_press(Message::NorthReset)
//! ```

use std::cell::Cell;

use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::alignment::Vertical;
use iced::widget::canvas::{self, Path};
use iced::widget::text::Alignment;
use iced::{
    Background, Border, Color, Element, Event, Length, Pixels, Point, Rectangle, Renderer, Shadow,
    Size, Theme, Vector, mouse,
};

use crate::theme::{Tokens, typography};

/// Varsayılan boyut.
const SIZE: f32 = 40.0;

/// Pusula.
pub struct Compass<Message> {
    north: f32,
    size: f32,
    plain: bool,
    letter: &'static str,
    on_press: Option<Message>,
}

impl<Message: Clone> Compass<Message> {
    /// `north`: kuzeyin ekrandaki yönü, yukarıdan saat yönünde derece (ör.
    /// harita 30° saat yönünde döndürüldüyse 30).
    pub fn new(north: f32) -> Self {
        Self {
            north,
            size: SIZE,
            plain: false,
            letter: "K",
            on_press: None,
        }
    }

    /// Kenar uzunluğu (piksel).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Zeminsiz, klasik pafta kuzey oku.
    pub fn plain(mut self) -> Self {
        self.plain = true;
        self
    }

    /// Kuzeyin harfi (varsayılan "K").
    pub fn letter(mut self, letter: &'static str) -> Self {
        self.letter = letter;
        self
    }

    /// Tıklanınca gönderilir (ör. görünümü kuzeye döndür).
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

#[derive(Default)]
struct State {
    cache: canvas::Cache,
    /// Önbellekteki çizimin açısı, boyutu ve renkleri.
    drawn: Cell<Option<(f32, f32, [Color; 3])>>,
    hovered: bool,
    pressed: bool,
}

/// Açının yönü: x sağa, y aşağı.
fn heading(degrees: f32) -> Vector {
    let radians = degrees.to_radians();

    Vector::new(radians.sin(), -radians.cos())
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for Compass<Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
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
        let over = cursor.is_over(layout.bounds());

        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) if over != state.hovered => {
                state.hovered = over;
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if over && self.on_press.is_some() =>
            {
                state.pressed = true;
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if std::mem::take(&mut state.pressed) =>
            {
                if over && let Some(message) = &self.on_press {
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
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.on_press.is_some() && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
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
        let size = bounds.width.min(bounds.height);

        if !self.plain {
            let hovered = state.hovered && self.on_press.is_some();

            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        color: if hovered { t.accent } else { t.border },
                        width: 1.0,
                        radius: (size / 2.0).into(),
                    },
                    shadow: Shadow {
                        color: t.shadow(),
                        offset: Vector::new(0.0, 1.0),
                        blur_radius: 4.0,
                    },
                    ..Quad::default()
                },
                Background::Color(if state.pressed && hovered {
                    t.surface_hover
                } else {
                    t.popover
                }),
            );
        }

        // Kuzey yarısı kırmızı, güney yarısı soluk; sade ok yazı renginde.
        let colors = if self.plain {
            [t.text, t.text, t.text]
        } else {
            [t.danger, t.muted, t.text]
        };

        if state.drawn.get() != Some((self.north, size, colors)) {
            state.cache.clear();
            state.drawn.set(Some((self.north, size, colors)));
        }

        let plain = self.plain;
        let north = self.north;
        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            let center = Point::new(size / 2.0, size / 2.0);
            let direction = heading(north);
            let side = Vector::new(-direction.y, direction.x);
            let at = |along: f32, across: f32| {
                center + direction * (along * size) + side * (across * size)
            };

            if plain {
                // Klasik kuzey oku: ucu kuzeyde, sol yarısı dolu.
                let left = Path::new(|builder| {
                    builder.move_to(at(0.28, 0.0));
                    builder.line_to(at(-0.3, -0.2));
                    builder.line_to(at(-0.14, 0.0));
                    builder.close();
                });
                let outline = Path::new(|builder| {
                    builder.move_to(at(0.28, 0.0));
                    builder.line_to(at(-0.3, 0.2));
                    builder.line_to(at(-0.14, 0.0));
                    builder.line_to(at(-0.3, -0.2));
                    builder.close();
                });

                frame.fill(&left, colors[0]);
                frame.stroke(
                    &outline,
                    canvas::Stroke::default()
                        .with_color(colors[2])
                        .with_width((size / 36.0).max(1.0)),
                );
            } else {
                let north_half = Path::new(|builder| {
                    builder.move_to(at(0.26, 0.0));
                    builder.line_to(at(0.0, 0.09));
                    builder.line_to(at(0.0, -0.09));
                    builder.close();
                });
                let south_half = Path::new(|builder| {
                    builder.move_to(at(-0.26, 0.0));
                    builder.line_to(at(0.0, 0.09));
                    builder.line_to(at(0.0, -0.09));
                    builder.close();
                });

                frame.fill(&north_half, colors[0]);
                frame.fill(&south_half, colors[1]);
            }
        });

        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });

        // Harf hep dik: iğnenin ucunun ötesinde.
        let direction = heading(self.north);
        let reach = if self.plain { 0.4 } else { 0.38 };
        let letter_size = (size * if self.plain { 0.26 } else { 0.22 }).round();
        let position =
            Point::new(bounds.center_x(), bounds.center_y()) + direction * (reach * size);

        iced::advanced::text::Renderer::fill_text(
            renderer,
            iced::advanced::Text {
                content: self.letter.to_owned(),
                bounds: Size::new(size, size),
                size: Pixels(letter_size.max(8.0)),
                line_height: iced::widget::text::LineHeight::Relative(1.0),
                font: typography::ui_strong(),
                align_x: Alignment::Center,
                align_y: Vertical::Center,
                shaping: iced::widget::text::Shaping::Basic,
                wrapping: iced::widget::text::Wrapping::None,
            },
            position,
            if self.plain { t.text } else { t.danger },
            Rectangle::new(
                Point::new(bounds.x - size, bounds.y - size),
                Size::new(size * 3.0, size * 3.0),
            ),
        );
    }
}

impl<'a, Message: Clone + 'a> From<Compass<Message>> for Element<'a, Message> {
    fn from(compass: Compass<Message>) -> Self {
        Element::new(compass)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_turn_clockwise_from_up() {
        let close = |a: Vector, b: Vector| (a.x - b.x).abs() < 1e-5 && (a.y - b.y).abs() < 1e-5;

        assert!(close(heading(0.0), Vector::new(0.0, -1.0)));
        assert!(close(heading(90.0), Vector::new(1.0, 0.0)));
        assert!(close(heading(180.0), Vector::new(0.0, 1.0)));
        assert!(close(heading(-90.0), Vector::new(-1.0, 0.0)));
    }
}

/// Gerçek olaylarla: tıklanınca mesaj gönderir.
#[cfg(all(test, feature = "snapshot"))]
mod interaction {
    use iced::{Element, Point, Size};

    use super::Compass;
    use crate::snapshot::{Input, Snapshot};

    type Presses = Vec<()>;

    fn view(_presses: &Presses) -> Element<'_, ()> {
        Compass::new(30.0).on_press(()).into()
    }

    #[test]
    fn clicking_the_compass_resets_north() {
        let mut snapshot = Snapshot::new(Size::new(100.0, 100.0)).expect("çizici kurulamadı");
        let mut presses: Presses = Vec::new();
        let mut update = |presses: &mut Presses, message| presses.push(message);

        snapshot.input(
            &mut presses,
            view,
            &mut update,
            Input::Click(Point::new(20.0, 20.0)),
        );
        snapshot.input(
            &mut presses,
            view,
            &mut update,
            Input::Click(Point::new(80.0, 80.0)),
        );
        assert_eq!(presses.len(), 1);
    }
}
