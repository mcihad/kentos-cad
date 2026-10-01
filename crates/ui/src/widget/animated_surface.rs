//! GPU friendly decoration: Iced batches these rounded quads and gradients
//! through its wgpu renderer. No per-frame application messages are needed.

use std::time::Duration;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::gradient::Linear;
use iced::time::Instant;
use iced::{
    Background, Border, Color, Degrees, Element, Event, Gradient, Length, Rectangle, Renderer,
    Shadow, Size, Theme, Vector, mouse, window,
};

use crate::theme::{Tokens, motion, shape};

use super::python::syntax;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Activity {
    #[default]
    Ready,
    Running,
    Success,
    Error,
}

const FRAME: Duration = Duration::from_millis(8);
const FOCUS: Duration = Duration::from_millis(260);
const RESPONSE: Duration = Duration::from_millis(650);

pub(crate) fn focused(tree: &Tree) -> bool {
    (tree.tag == tree::Tag::of::<iced::widget::text_editor::State<syntax::Python>>()
        && tree
            .state
            .downcast_ref::<iced::widget::text_editor::State<syntax::Python>>()
            .is_focused())
        || (tree.tag == tree::Tag::of::<iced::widget::text_editor::State<iced::advanced::text::highlighter::PlainText>>()
            && tree.state.downcast_ref::<iced::widget::text_editor::State<iced::advanced::text::highlighter::PlainText>>().is_focused())
        || (tree.tag == tree::Tag::of::<iced::widget::text_input::State<<Renderer as iced::advanced::text::Renderer>::Paragraph>>()
            && tree.state.downcast_ref::<iced::widget::text_input::State<<Renderer as iced::advanced::text::Renderer>::Paragraph>>().is_focused())
        || tree.children.iter().any(focused)
}

pub(crate) fn status_color(status: impl Into<Activity>, t: &Tokens) -> Color {
    match status.into() {
        Activity::Ready => t.accent,
        Activity::Running => t.info,
        Activity::Success => t.success,
        Activity::Error => t.danger,
    }
}

pub(crate) struct Surface<'a, Message> {
    content: Element<'a, Message>,
    status: Activity,
    revision: u64,
}

impl<'a, Message> Surface<'a, Message> {
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        status: impl Into<Activity>,
        revision: u64,
    ) -> Self {
        Self {
            content: content.into(),
            status: status.into(),
            revision,
        }
    }
}

struct State {
    started: Instant,
    now: Instant,
    changed: Instant,
    status: Activity,
    revision: u64,
    focus_from: f32,
    focus_target: f32,
    focus_changed: Instant,
    window_active: bool,
}

impl State {
    fn focus(&self) -> f32 {
        let p = motion::progress(self.focus_changed, self.now, FOCUS);
        self.focus_from + (self.focus_target - self.focus_from) * p
    }

    fn animating(&self) -> bool {
        !motion::reduced()
            && self.window_active
            && (self.status == Activity::Running
                || motion::running(self.focus_changed, self.now, FOCUS)
                || motion::running(self.changed, self.now, RESPONSE))
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Surface<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        let now = Instant::now();
        tree::State::new(State {
            started: now,
            now,
            changed: now,
            status: self.status,
            revision: self.revision,
            focus_from: 0.0,
            focus_target: 0.0,
            focus_changed: now,
            window_active: true,
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
        let state = tree.state.downcast_mut::<State>();
        if state.status != self.status || state.revision != self.revision {
            state.status = self.status;
            state.revision = self.revision;
            state.changed = Instant::now();
        }
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        layout::Node::with_children(child.size(), vec![child])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Some(child) = layout.children().next() else {
            return;
        };
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            child,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        let target = if focused(&tree.children[0]) {
            1.0
        } else if cursor.is_over(layout.bounds()) {
            0.28
        } else {
            0.0
        };
        let state = tree.state.downcast_mut::<State>();
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            state.now = *now;
        }
        if let Event::Window(window::Event::Unfocused) = event {
            state.window_active = false;
        }
        if let Event::Window(window::Event::Focused) = event {
            state.window_active = true;
        }
        if state.focus_target != target {
            state.focus_from = state.focus();
            state.focus_target = target;
            state.focus_changed = Instant::now();
            shell.request_redraw();
        }
        if layout.bounds().intersection(viewport).is_some() && state.animating() {
            shell.request_redraw_at(state.now + FRAME);
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };
        let Some(child) = layout.children().next() else {
            return;
        };
        let t = Tokens::of(theme);
        let state = tree.state.downcast_ref::<State>();
        let focus = state.focus();
        let color = status_color(self.status, &t);
        let response = if motion::reduced() {
            0.0
        } else {
            1.0 - motion::progress(state.changed, state.now, RESPONSE)
        };
        let elapsed = state
            .now
            .saturating_duration_since(state.started)
            .as_secs_f32();
        let pulse = if self.status == Activity::Running && !motion::reduced() {
            0.5 + 0.5 * (elapsed * 3.6).sin()
        } else {
            0.0
        };
        let glow = (focus * 0.2 + response * 0.18 + pulse * 0.1).min(0.45);
        renderer.with_layer(*viewport, |renderer| {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        color: mix(t.border, color, focus * 0.7 + response * 0.25),
                        width: 1.0,
                        radius: shape::lg().into(),
                    },
                    shadow: Shadow {
                        color: color.scale_alpha(if t.is_dark { glow } else { glow * 0.3 }),
                        offset: Vector::ZERO,
                        blur_radius: 18.0 + 14.0 * focus,
                    },
                    ..Quad::default()
                },
                t.field,
            );
        });
        renderer.with_layer(clip, |renderer| {
            renderer.fill_quad(
                Quad {
                    bounds: bounds.shrink(1.0),
                    border: Border {
                        radius: (shape::lg() - 1.0).max(0.0).into(),
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                Background::Gradient(Gradient::Linear(
                    Linear::new(Degrees(180.0))
                        .add_stop(0.0, color.scale_alpha(0.035 + focus * 0.035))
                        .add_stop(1.0, Color::TRANSPARENT),
                )),
            );
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                child,
                cursor,
                viewport,
            );
            // Execution beam: a luminous head with a long fading trail. The
            // clipped beam crosses the top edge, without covering code.
            // The top edge is straight only between the rounded corners.
            let edge = Rectangle {
                x: bounds.x + shape::lg(),
                y: bounds.y,
                width: (bounds.width - 2.0 * shape::lg()).max(0.0),
                height: 2.0,
            };
            renderer.with_layer(edge, |renderer| {
                if self.status == Activity::Running {
                    let phase = if motion::reduced() {
                        0.5
                    } else {
                        (elapsed / 1.65).fract()
                    };
                    let width = bounds.width * 0.5;
                    let x = bounds.x - width + (bounds.width + width) * phase;
                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle {
                                x,
                                y: bounds.y,
                                width,
                                height: 2.0,
                            },
                            ..Quad::default()
                        },
                        Background::Gradient(Gradient::Linear(
                            Linear::new(Degrees(90.0))
                                .add_stop(0.0, color.scale_alpha(0.0))
                                .add_stop(0.68, color.scale_alpha(0.5))
                                .add_stop(0.95, color)
                                .add_stop(1.0, color.scale_alpha(0.0)),
                        )),
                    );
                } else if response > 0.001 {
                    let width = bounds.width * (1.0 - response);
                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle {
                                x: bounds.center_x() - width / 2.0,
                                y: bounds.y,
                                width,
                                height: 2.0,
                            },
                            ..Quad::default()
                        },
                        color.scale_alpha(response),
                    );
                }
            });
        });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        layout
            .children()
            .next()
            .map_or(mouse::Interaction::default(), |child| {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    child,
                    cursor,
                    viewport,
                    renderer,
                )
            })
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(child) = layout.children().next() {
            self.content
                .as_widget_mut()
                .operate(&mut tree.children[0], child, renderer, operation);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next()?,
            renderer,
            viewport,
            translation,
        )
    }
}

pub(crate) fn mix(a: Color, b: Color, amount: f32) -> Color {
    let p = amount.clamp(0.0, 1.0);
    Color::from_rgba(
        a.r + (b.r - a.r) * p,
        a.g + (b.g - a.g) * p,
        a.b + (b.b - a.b) * p,
        a.a + (b.a - a.a) * p,
    )
}

impl<'a, Message: 'a> From<Surface<'a, Message>> for Element<'a, Message> {
    fn from(surface: Surface<'a, Message>) -> Self {
        Self::new(surface)
    }
}
