//! Follow visual text growth while preserving a reader's manual scroll position.
//! Iced also reports viewport changes on layout/redraw; those are not gestures.

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Id, Operation, Tree, Widget, operation, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse, touch};

pub(super) struct ScrollFollow<'a, Message> {
    content: Element<'a, Message>,
    following: bool,
    on_follow: Box<dyn Fn(bool) -> Message + 'a>,
}

impl<'a, Message> ScrollFollow<'a, Message> {
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        following: bool,
        on_follow: impl Fn(bool) -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            following,
            on_follow: Box::new(on_follow),
        }
    }
}

#[derive(Default)]
struct State {
    auto_scroll: bool,
}

#[derive(Default)]
struct Inspect {
    offset: f32,
    end: f32,
    snap: bool,
}
impl Operation for Inspect {
    // Only the outer conversation scroller participates; code blocks keep
    // their independent horizontal position.
    fn traverse(&mut self, _: &mut dyn FnMut(&mut dyn Operation)) {}
    fn scrollable(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        content: Rectangle,
        translation: Vector,
        state: &mut dyn operation::Scrollable,
    ) {
        self.offset = translation.y;
        self.end = (content.height - bounds.height).max(0.0);
        if self.snap {
            state.snap_to(operation::scrollable::RelativeOffset {
                x: None,
                y: Some(1.0),
            });
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for ScrollFollow<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
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
        if self.following {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                Layout::new(&child),
                renderer,
                &mut Inspect {
                    snap: true,
                    ..Inspect::default()
                },
            );
        }
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
        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle))
                if cursor.is_over(layout.bounds()) =>
            {
                state.auto_scroll = !state.auto_scroll
            }
            Event::Mouse(mouse::Event::ButtonPressed(_))
            | Event::Window(iced::window::Event::Unfocused) => state.auto_scroll = false,
            Event::Keyboard(_) => state.auto_scroll = false,
            _ => {}
        }
        let gesture = state.auto_scroll
            || matches!(
                event,
                Event::Mouse(
                    mouse::Event::WheelScrolled { .. }
                        | mouse::Event::CursorMoved { .. }
                        | mouse::Event::ButtonPressed(_)
                ) | Event::Touch(
                    touch::Event::FingerMoved { .. } | touch::Event::FingerPressed { .. }
                )
            );
        let mut before = Inspect::default();
        if gesture {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                child,
                renderer,
                &mut before,
            );
        }
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
        if gesture {
            let mut after = Inspect::default();
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                child,
                renderer,
                &mut after,
            );
            if (after.offset - before.offset).abs() > f32::EPSILON {
                shell.publish((self.on_follow)(after.end - after.offset <= 3.0));
            }
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
        if let Some(child) = layout.children().next() {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                child,
                cursor,
                viewport,
            );
        }
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
impl<'a, Message: 'a> From<ScrollFollow<'a, Message>> for Element<'a, Message> {
    fn from(value: ScrollFollow<'a, Message>) -> Self {
        Element::new(value)
    }
}
