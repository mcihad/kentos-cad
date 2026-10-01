//! Local animation clocks. They request frames only while visible and active;
//! no application subscription is required for activity or character reveal.

use std::time::Duration;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::text::{self, Paragraph as _, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::time::Instant;
use iced::{
    Border, Element, Event, Fill, Length, Point, Rectangle, Renderer, Shadow, Size, Theme, Vector,
    mouse, window,
};

use crate::theme::{Tokens, motion, typography};

type Paragraph = <Renderer as text::Renderer>::Paragraph;
const FRAME: Duration = Duration::from_millis(8);

/// An orbit of glowing particles that becomes a quiet four point star at
/// rest. Usable on its own in an agent avatar, a tool row or a thinking header.
pub struct AiActivity {
    active: bool,
    size: f32,
}

impl AiActivity {
    pub fn new(active: bool) -> Self {
        Self { active, size: 22.0 }
    }
    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(8.0);
        self
    }
}

struct Clock {
    started: Instant,
    now: Instant,
    window_active: bool,
}
impl Default for Clock {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            started: now,
            now,
            window_active: true,
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for AiActivity {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Clock>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(Clock::default())
    }
    fn size(&self) -> Size<Length> {
        Size::new(self.size.into(), self.size.into())
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
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let clock = tree.state.downcast_mut::<Clock>();
        match event {
            Event::Window(window::Event::RedrawRequested(now)) => clock.now = *now,
            Event::Window(window::Event::Focused) => clock.window_active = true,
            Event::Window(window::Event::Unfocused) => clock.window_active = false,
            _ => {}
        }
        if self.active
            && clock.window_active
            && !motion::reduced()
            && layout.bounds().intersection(viewport).is_some()
        {
            shell.request_redraw_at(clock.now + FRAME);
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
        viewport: &Rectangle,
    ) {
        let Some(clip) = layout.bounds().expand(6.0).intersection(viewport) else {
            return;
        };
        let t = Tokens::of(theme);
        let clock = tree.state.downcast_ref::<Clock>();
        let elapsed = clock
            .now
            .saturating_duration_since(clock.started)
            .as_secs_f32();
        let animate = self.active && !motion::reduced();
        let angle = if animate {
            elapsed * 2.1
        } else {
            std::f32::consts::FRAC_PI_4
        };
        let center = layout.bounds().center();
        renderer.with_layer(clip, |renderer| {
            for i in 0..8 {
                let a = angle + i as f32 * std::f32::consts::TAU / 8.0;
                let radius = self.size
                    * if animate {
                        0.3 + 0.055 * (elapsed * 3.0 + i as f32 * 0.6).sin()
                    } else if i % 2 == 0 {
                        0.33
                    } else {
                        0.17
                    };
                let size = if animate {
                    2.3 + 1.0 * (a - angle).cos()
                } else if i % 2 == 0 {
                    3.0
                } else {
                    2.0
                };
                let color = crate::widget::animated_surface::mix(
                    t.accent_hover,
                    t.info,
                    (i as f32 / 7.0).min(1.0),
                );
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(
                            Point::new(
                                center.x + a.cos() * radius - size / 2.0,
                                center.y + a.sin() * radius - size / 2.0,
                            ),
                            Size::new(size, size),
                        ),
                        border: Border {
                            radius: size.into(),
                            ..Border::default()
                        },
                        shadow: Shadow {
                            color: color.scale_alpha(if animate { 0.55 } else { 0.15 }),
                            offset: Vector::ZERO,
                            blur_radius: if animate { 5.0 } else { 2.0 },
                        },
                        ..Quad::default()
                    },
                    color.scale_alpha(if animate {
                        0.42 + 0.58 * (i as f32 / 7.0)
                    } else {
                        0.85
                    }),
                );
            }
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(center - Vector::new(1.6, 1.6), Size::new(3.2, 3.2)),
                    border: Border {
                        radius: 2.0.into(),
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                t.accent_hover,
            );
        });
    }
}

impl<'a, Message: 'a> From<AiActivity> for Element<'a, Message> {
    fn from(activity: AiActivity) -> Self {
        Self::new(activity)
    }
}

/// Smooth character reveal for a growing UTF-8 source. The source can arrive
/// in large or small chunks; the local clock catches up over about 150 ms.
/// Finished text is drawn immediately and stops requesting animation frames.
pub struct AiStream<'a> {
    source: &'a str,
    active: bool,
    size: f32,
}

impl<'a> AiStream<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            active: true,
            size: typography::body() + 2.0,
        }
    }
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(8.0);
        self
    }
}

struct StreamState {
    paragraph: text::paragraph::Plain<Paragraph>,
    target: String,
    shown: usize,
    credit: f32,
    clock: Clock,
}

impl<Message> Widget<Message, Theme, Renderer> for AiStream<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<StreamState>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(StreamState {
            paragraph: text::paragraph::Plain::default(),
            target: self.source.to_owned(),
            shown: if self.active && !motion::reduced() {
                0
            } else {
                self.source.len()
            },
            credit: 0.0,
            clock: Clock::default(),
        })
    }
    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<StreamState>();
        if !self.source.starts_with(&state.target) {
            state.shown = if self.active { 0 } else { self.source.len() };
            state.credit = 0.0;
        }
        self.source.clone_into(&mut state.target);
        if !self.active || motion::reduced() {
            state.shown = self.source.len();
        }
    }
    fn size(&self) -> Size<Length> {
        Size::new(Fill, Length::Shrink)
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<StreamState>();
        state.paragraph.update(text::Text {
            content: &state.target[..state.shown],
            bounds: Size::new((limits.max().width - 12.0).max(0.0), f32::INFINITY),
            size: self.size.into(),
            line_height: text::LineHeight::Relative(1.65),
            font: typography::ui(),
            align_x: text::Alignment::Default,
            align_y: iced::alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::WordOrGlyph,
        });
        let bounds = state.paragraph.min_bounds();
        layout::Node::new(limits.resolve(
            Fill,
            Length::Shrink,
            Size::new(bounds.width + 12.0, bounds.height.max(self.size * 1.65)),
        ))
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<StreamState>();
        match event {
            Event::Window(window::Event::Focused) => state.clock.window_active = true,
            Event::Window(window::Event::Unfocused) => state.clock.window_active = false,
            Event::Window(window::Event::RedrawRequested(now)) => {
                let delta = now
                    .saturating_duration_since(state.clock.now)
                    .as_secs_f32()
                    .min(0.064);
                state.clock.now = *now;
                if state.shown < state.target.len() {
                    if motion::reduced() || !self.active {
                        state.shown = state.target.len();
                    } else {
                        let backlog = state.target[state.shown..].chars().count();
                        state.credit += delta * (backlog as f32 / 0.15).max(70.0);
                        let count = state.credit.floor() as usize;
                        state.credit -= count as f32;
                        state.shown = advance(&state.target, state.shown, count);
                    }
                    shell.invalidate_layout();
                }
            }
            _ => {}
        }
        if self.active
            && !motion::reduced()
            && state.clock.window_active
            && layout.bounds().intersection(viewport).is_some()
        {
            shell.request_redraw_at(state.clock.now + FRAME);
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
        viewport: &Rectangle,
    ) {
        let Some(clip) = layout.bounds().intersection(viewport) else {
            return;
        };
        let state = tree.state.downcast_ref::<StreamState>();
        let t = Tokens::of(theme);
        renderer.fill_paragraph(state.paragraph.raw(), layout.position(), t.text, clip);
        if self.active {
            let height = self.size * 1.65;
            let row = (state.paragraph.min_height() / height).ceil().max(1.0) as usize - 1;
            let x = state
                .paragraph
                .raw()
                .grapheme_position(row, usize::MAX)
                .map_or(0.0, |point| point.x);
            let elapsed = state
                .clock
                .now
                .saturating_duration_since(state.clock.started)
                .as_secs_f32();
            let pulse = if motion::reduced() {
                0.8
            } else {
                0.55 + 0.45 * (elapsed * 4.0).sin().abs()
            };
            renderer.with_layer(clip, |renderer| {
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(
                            Point::new(
                                layout.bounds().x + x + 4.0,
                                layout.bounds().y + row as f32 * height + 4.0,
                            ),
                            Size::new(3.0, self.size),
                        ),
                        border: Border {
                            radius: 2.0.into(),
                            ..Border::default()
                        },
                        shadow: Shadow {
                            color: t.accent.scale_alpha(pulse * 0.55),
                            offset: Vector::ZERO,
                            blur_radius: 7.0,
                        },
                        ..Quad::default()
                    },
                    t.accent_hover.scale_alpha(pulse),
                )
            });
        }
    }
    fn operate(
        &mut self,
        _tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.text(None, layout.bounds(), self.source);
    }
}

fn advance(source: &str, start: usize, count: usize) -> usize {
    source[start..]
        .char_indices()
        .nth(count)
        .map_or(source.len(), |(offset, _)| start + offset)
}

impl<'a, Message: 'a> From<AiStream<'a>> for Element<'a, Message> {
    fn from(stream: AiStream<'a>) -> Self {
        Self::new(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::advance;
    #[test]
    fn character_reveal_never_slices_a_utf8_character() {
        let source = "çizim 🦀 東京";
        let mut position = 0;
        while position < source.len() {
            position = advance(source, position, 1);
            assert!(source.is_char_boundary(position));
        }
        assert_eq!(advance(source, 0, 0), 0);
        assert_eq!(advance(source, 0, 100), source.len());
    }

    #[cfg(feature = "snapshot")]
    #[test]
    fn stream_reveals_on_frames_and_culls_frame_requests_when_hidden_or_stopped() {
        use super::*;
        use crate::snapshot::Snapshot;

        let snapshot = Snapshot::software(Size::new(420.0, 210.0)).unwrap();
        let source = "Çizim 🦀 inceleniyor; alanlar ve katmanlar hazırlanıyor.";
        let mut widget: Element<'_, ()> = AiStream::new(source).into();
        let mut tree = Tree::new(&widget);
        let limits = layout::Limits::new(Size::ZERO, Size::new(420.0, 210.0));
        let node = widget
            .as_widget_mut()
            .layout(&mut tree, snapshot.renderer(), &limits);
        let start = tree.state.downcast_ref::<StreamState>().clock.now;
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        widget.as_widget_mut().update(
            &mut tree,
            &Event::Window(window::Event::RedrawRequested(
                start + Duration::from_millis(32),
            )),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            snapshot.renderer(),
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &Rectangle::with_size(Size::new(420.0, 210.0)),
        );
        let shown = tree.state.downcast_ref::<StreamState>().shown;
        assert!(shown > 0 && shown < source.len());
        assert!(source.is_char_boundary(shown));
        assert_ne!(shell.redraw_request(), window::RedrawRequest::Wait);

        let mut shell = Shell::new(&mut messages);
        widget.as_widget_mut().update(
            &mut tree,
            &Event::Window(window::Event::RedrawRequested(
                start + Duration::from_millis(64),
            )),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            snapshot.renderer(),
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &Rectangle::new(Point::new(1000.0, 1000.0), Size::new(100.0, 100.0)),
        );
        assert_eq!(shell.redraw_request(), window::RedrawRequest::Wait);

        widget = AiStream::new(source).active(false).into();
        widget.as_widget().diff(&mut tree);
        assert_eq!(tree.state.downcast_ref::<StreamState>().shown, source.len());
        let mut shell = Shell::new(&mut messages);
        widget.as_widget_mut().update(
            &mut tree,
            &Event::Window(window::Event::RedrawRequested(
                start + Duration::from_millis(96),
            )),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            snapshot.renderer(),
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &Rectangle::with_size(Size::new(420.0, 210.0)),
        );
        assert_eq!(shell.redraw_request(), window::RedrawRequest::Wait);
    }
}
