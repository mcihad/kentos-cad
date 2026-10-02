//! Keeps child state while an accordion opens or closes over local frames.

use std::time::Duration;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::time::Instant;
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse, window};

use crate::theme::motion;

const DURATION: Duration = Duration::from_millis(240);

pub(super) struct Disclosure<'a, Message> {
    content: Element<'a, Message>,
    expanded: bool,
}

impl<'a, Message> Disclosure<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>, expanded: bool) -> Self {
        Self {
            content: content.into(),
            expanded,
        }
    }
}

struct State {
    from: f32,
    target: f32,
    changed: Instant,
    now: Instant,
    window_active: bool,
}
impl State {
    fn progress(&self) -> f32 {
        self.from + (self.target - self.from) * motion::progress(self.changed, self.now, DURATION)
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Disclosure<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        let target = if self.expanded { 1.0 } else { 0.0 };
        let now = Instant::now();
        tree::State::new(State {
            from: target,
            target,
            changed: now,
            now,
            window_active: true,
        })
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
        let state = tree.state.downcast_mut::<State>();
        let target = if self.expanded { 1.0 } else { 0.0 };
        if state.target != target {
            state.from = state.progress();
            state.target = target;
            state.changed = state.now.max(Instant::now());
        }
    }
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
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
        let progress = tree.state.downcast_ref::<State>().progress();
        let size = Size::new(child.size().width, child.size().height * progress);
        layout::Node::with_children(size, vec![child])
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
        let state = tree.state.downcast_mut::<State>();
        let previous = state.progress();
        match event {
            Event::Window(window::Event::RedrawRequested(now)) => state.now = *now,
            Event::Window(window::Event::Focused) => state.window_active = true,
            Event::Window(window::Event::Unfocused) => state.window_active = false,
            _ => {}
        }
        if (state.progress() - previous).abs() > f32::EPSILON {
            shell.invalidate_layout();
        }
        let animating = motion::running(state.changed, state.now, DURATION);
        if animating
            && state.window_active
            && layout.bounds().expand(1.0).intersection(viewport).is_some()
        {
            shell.invalidate_layout();
            shell.request_redraw_at(state.now + Duration::from_millis(8));
        }
        if self.expanded
            && let Some(clip) = layout.bounds().intersection(viewport)
            && let Some(child) = layout.children().next()
        {
            let cursor = if cursor.is_over(clip) {
                cursor
            } else {
                mouse::Cursor::Unavailable
            };
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                child,
                cursor,
                renderer,
                clipboard,
                shell,
                &clip,
            );
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
        if let Some(clip) = layout.bounds().intersection(viewport)
            && let Some(child) = layout.children().next()
        {
            renderer.with_layer(clip, |renderer| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    child,
                    cursor,
                    &clip,
                )
            });
        }
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.expanded
            && cursor.is_over(layout.bounds())
            && let Some(child) = layout.children().next()
        {
            self.content.as_widget().mouse_interaction(
                &tree.children[0],
                child,
                cursor,
                viewport,
                renderer,
            )
        } else {
            mouse::Interaction::default()
        }
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if self.expanded
            && let Some(child) = layout.children().next()
        {
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
        if self.expanded
            && let Some(child) = layout.children().next()
        {
            self.content.as_widget_mut().overlay(
                &mut tree.children[0],
                child,
                renderer,
                viewport,
                translation,
            )
        } else {
            None
        }
    }
}

impl<'a, Message: 'a> From<Disclosure<'a, Message>> for Element<'a, Message> {
    fn from(value: Disclosure<'a, Message>) -> Self {
        Element::new(value)
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use super::*;
    use crate::snapshot::Snapshot;

    #[test]
    fn accordion_reverses_continuously_and_reaches_its_final_layout() {
        let snapshot = Snapshot::software(Size::new(420.0, 300.0)).unwrap();
        let view = |expanded| -> Element<'_, ()> {
            Disclosure::new(
                iced::widget::container(iced::widget::text("Araç çıktısı")).height(120),
                expanded,
            )
            .into()
        };
        let mut widget = view(false);
        let mut tree = Tree::new(&widget);
        let limits = layout::Limits::new(Size::ZERO, Size::new(420.0, 300.0));
        let mut node = widget
            .as_widget_mut()
            .layout(&mut tree, snapshot.renderer(), &limits);
        assert_eq!(node.size().height, 0.0);
        widget = view(true);
        widget.as_widget().diff(&mut tree);
        let start = tree.state.downcast_ref::<State>().changed;
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        widget.as_widget_mut().update(
            &mut tree,
            &Event::Window(window::Event::RedrawRequested(
                start + Duration::from_millis(120),
            )),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            snapshot.renderer(),
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &Rectangle::with_size(Size::new(420.0, 300.0)),
        );
        assert!(shell.is_layout_invalid());
        node = widget
            .as_widget_mut()
            .layout(&mut tree, snapshot.renderer(), &limits);
        let halfway = node.size().height;
        assert!(halfway > 0.0 && halfway < 120.0);
        widget = view(false);
        widget.as_widget().diff(&mut tree);
        node = widget
            .as_widget_mut()
            .layout(&mut tree, snapshot.renderer(), &limits);
        assert!((node.size().height - halfway).abs() < 0.001);
        let start = tree.state.downcast_ref::<State>().changed;
        let mut shell = Shell::new(&mut messages);
        widget.as_widget_mut().update(
            &mut tree,
            &Event::Window(window::Event::RedrawRequested(
                start + Duration::from_millis(260),
            )),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            snapshot.renderer(),
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &Rectangle::with_size(Size::new(420.0, 300.0)),
        );
        assert!(
            shell.is_layout_invalid(),
            "the last frame must settle the height"
        );
        assert_eq!(shell.redraw_request(), window::RedrawRequest::Wait);
        node = widget
            .as_widget_mut()
            .layout(&mut tree, snapshot.renderer(), &limits);
        assert_eq!(node.size().height, 0.0);
    }
}
