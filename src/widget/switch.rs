//! Anahtar: açık/kapalı ayar; düğmesi kayarak konum değiştirir.
//!
//! ```text
//! (●──)  Yakalama        kapalı
//! (──●)  Yakalama        açık (vurgu renginde)
//! ```
//!
//! Onay kutusundan farkı, değişikliğin hemen uygulanmasıdır (ör. ızgarayı
//! açmak). Etiket de tıklanabilir.
//!
//! ```ignore
//! Switch::new(self.snap, Message::SnapToggled).label("Nesne yakalama")
//! ```

use std::time::Duration;

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::time::Instant;
use iced::widget::text::{Fragment, IntoFragment, Wrapping};
use iced::widget::{Text, text};
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Shadow, Size,
    Theme, Vector, mouse, window,
};

use crate::theme::{Tokens, typography};

/// Rayın ölçüleri ve düğmenin kayma süresi.
const TRACK: Size = Size::new(32.0, 18.0);
const GAP: f32 = 8.0;
const SLIDE: Duration = Duration::from_millis(140);

/// Anahtar.
pub struct Switch<'a, Message> {
    on: bool,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
    label: Option<Text<'a>>,
}

impl<'a, Message: 'a> Switch<'a, Message> {
    /// `on_toggle` yeni durumu alır.
    pub fn new(on: bool, on_toggle: impl Fn(bool) -> Message + 'a) -> Self {
        Self {
            on,
            on_toggle: Some(Box::new(on_toggle)),
            label: None,
        }
    }

    /// Devre dışı anahtar: durumu gösterir, değiştirilemez.
    pub fn disabled(on: bool) -> Self {
        Self {
            on,
            on_toggle: None,
            label: None,
        }
    }

    pub fn label(mut self, label: impl IntoFragment<'a>) -> Self {
        let label: Fragment<'a> = label.into_fragment();

        self.label = Some(
            text(label)
                .font(typography::ui())
                .size(typography::body())
                .wrapping(Wrapping::None),
        );
        self
    }
}

#[derive(Debug)]
struct State {
    /// Son çizilen durum ve değiştiği an: düğme eski yerinden kayar.
    shown: bool,
    changed: Option<Instant>,
    now: Instant,
    hovered: bool,
    pressed: bool,
}

impl State {
    /// Düğmenin konumu: 0 solda (kapalı), 1 sağda (açık).
    fn position(&self) -> f32 {
        let target = if self.shown { 1.0 } else { 0.0 };

        match self.changed {
            Some(changed) => {
                let t = (self.now.saturating_duration_since(changed).as_secs_f32()
                    / SLIDE.as_secs_f32())
                .clamp(0.0, 1.0);
                // Yavaşlayarak durur.
                let eased = 1.0 - (1.0 - t) * (1.0 - t);

                if self.shown { eased } else { 1.0 - eased }
            }
            None => target,
        }
    }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Switch<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        let now = Instant::now();

        tree::State::new(State {
            shown: self.on,
            changed: None,
            now,
            hovered: false,
            pressed: false,
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.label
            .iter()
            .map(|label| Tree::new(label as &dyn Widget<Message, Theme, Renderer>))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        match &self.label {
            Some(label) => {
                if tree.children.len() == 1 {
                    tree.children[0].diff(label as &dyn Widget<Message, Theme, Renderer>);
                } else {
                    tree.children = vec![Tree::new(label as &dyn Widget<Message, Theme, Renderer>)];
                }
            }
            None => tree.children.clear(),
        }

        // Durum dışarıdan değişince düğme kayar.
        let state = tree.state.downcast_mut::<State>();

        if state.shown != self.on {
            state.shown = self.on;
            state.changed = Some(Instant::now());
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let track = Node::new(TRACK);

        let Some(label) = &mut self.label else {
            return Node::with_children(TRACK, vec![track]);
        };

        let label = <Text<'a> as Widget<Message, Theme, Renderer>>::layout(
            label,
            &mut tree.children[0],
            renderer,
            &limits.loose(),
        );
        let height = TRACK.height.max(label.size().height);
        let width = TRACK.width + GAP + label.size().width;

        Node::with_children(
            Size::new(width, height),
            vec![
                track.move_to(Point::new(0.0, ((height - TRACK.height) / 2.0).round())),
                label.clone().move_to(Point::new(
                    TRACK.width + GAP,
                    ((height - label.size().height) / 2.0).round(),
                )),
            ],
        )
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

        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                state.now = *now;

                if let Some(changed) = state.changed {
                    if now.saturating_duration_since(changed) < SLIDE {
                        shell.request_redraw();
                    } else {
                        state.changed = None;
                    }
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let hovered = cursor.is_over(bounds);

                if hovered != state.hovered {
                    state.hovered = hovered;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if self.on_toggle.is_some() && cursor.is_over(bounds) {
                    state.pressed = true;
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if std::mem::take(&mut state.pressed) && cursor.is_over(bounds) =>
            {
                if let Some(on_toggle) = &self.on_toggle {
                    shell.publish(on_toggle(!self.on));
                }

                shell.capture_event();
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
        if self.on_toggle.is_some() && cursor.is_over(layout.bounds()) {
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
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let t = Tokens::of(theme);
        let enabled = self.on_toggle.is_some();
        let mut children = layout.children();
        let Some(track) = children.next().map(|track| track.bounds()) else {
            return;
        };
        let position = state.position();
        let radius = track.height / 2.0;

        // Ray kapalıdan açığa doğru vurgu rengine döner.
        let off = if state.hovered && enabled {
            t.surface_hover
        } else {
            t.field
        };
        let fill = mix(off, t.accent, position);
        let fill = if enabled { fill } else { fill.scale_alpha(0.5) };

        renderer.fill_quad(
            Quad {
                bounds: track,
                border: Border {
                    color: mix(t.border, t.accent, position),
                    width: 1.0,
                    radius: radius.into(),
                },
                ..Quad::default()
            },
            Background::Color(fill),
        );

        let knob = track.height - 4.0;
        let x = track.x + 2.0 + position * (track.width - knob - 4.0);
        let knob_color = if position > 0.5 { t.on_accent } else { t.text };

        renderer.fill_quad(
            Quad {
                bounds: Rectangle::new(Point::new(x, track.y + 2.0), Size::new(knob, knob)),
                border: Border {
                    radius: (knob / 2.0).into(),
                    ..Border::default()
                },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                    offset: Vector::new(0.0, 1.0),
                    blur_radius: 2.0,
                },
                ..Quad::default()
            },
            Background::Color(if enabled {
                knob_color
            } else {
                knob_color.scale_alpha(0.6)
            }),
        );

        if let (Some(label), Some(label_layout)) = (&self.label, children.next()) {
            <Text<'a> as Widget<Message, Theme, Renderer>>::draw(
                label,
                &tree.children[0],
                renderer,
                theme,
                &renderer::Style {
                    text_color: if enabled { t.text } else { t.disabled() },
                },
                label_layout,
                cursor,
                viewport,
            );
        }
    }
}

impl<'a, Message: 'a> From<Switch<'a, Message>> for Element<'a, Message> {
    fn from(switch: Switch<'a, Message>) -> Self {
        Element::new(switch)
    }
}

fn mix(from: Color, to: Color, amount: f32) -> Color {
    crate::theme::accent::mix(from, to, amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_knob_slides_toward_the_new_state() {
        let start = Instant::now();
        let mut state = State {
            shown: true,
            changed: Some(start),
            now: start,
            hovered: false,
            pressed: false,
        };

        assert_eq!(state.position(), 0.0);

        state.now = start + SLIDE / 2;
        assert!((state.position() - 0.75).abs() < 1e-3);

        state.now = start + SLIDE * 2;
        assert_eq!(state.position(), 1.0);

        state.shown = false;
        state.changed = None;
        assert_eq!(state.position(), 0.0);
    }
}
