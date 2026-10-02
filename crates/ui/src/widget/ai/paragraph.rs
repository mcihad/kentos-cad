//! A stable rich-text child for both streamed and completed paragraphs.
//! Finishing a paragraph retains its shape, width, widget state and links.

use crate::theme::motion;
use iced::advanced::{
    Clipboard, Shell,
    layout::{self, Layout},
    overlay, renderer,
    widget::{Operation, Tree, Widget, tree},
};
use iced::time::Instant;
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse, window};
use std::{rc::Rc, time::Duration};

pub(super) struct StreamingParagraph<'a, Message> {
    source: &'a str,
    active: bool,
    link: Option<Rc<dyn Fn(String) -> Message + 'a>>,
    child: Element<'a, Message>,
    child_shown: usize,
}
impl<'a, Message: 'a> StreamingParagraph<'a, Message> {
    pub fn new(
        source: &'a str,
        active: bool,
        link: Option<Rc<dyn Fn(String) -> Message + 'a>>,
    ) -> Self {
        Self {
            source,
            active,
            child: super::content::rich_paragraph(source, link.clone(), active),
            child_shown: source.len(),
            link,
        }
    }
}
struct State {
    target: String,
    shown: usize,
    credit: f32,
    now: Instant,
    window_active: bool,
}
impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for StreamingParagraph<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(State {
            target: self.source.to_owned(),
            shown: if self.active && !motion::reduced() {
                0
            } else {
                self.source.len()
            },
            credit: 0.0,
            now: Instant::now(),
            window_active: true,
        })
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.child)]
    }
    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        if !self.source.starts_with(&state.target) {
            state.shown = 0;
            state.credit = 0.0;
        }
        self.source.clone_into(&mut state.target);
        if !self.active || motion::reduced() {
            state.shown = self.source.len();
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
        let shown = tree.state.downcast_ref::<State>().shown;
        if self.child_shown != shown {
            self.child = super::content::rich_paragraph(
                &self.source[..shown],
                self.link.clone(),
                self.active,
            );
            self.child_shown = shown;
        }
        tree.children[0].diff(&self.child);
        let child = self
            .child
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
        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Window(window::Event::Focused) => state.window_active = true,
            Event::Window(window::Event::Unfocused) => state.window_active = false,
            Event::Window(window::Event::RedrawRequested(now)) => {
                let delta = now
                    .saturating_duration_since(state.now)
                    .as_secs_f32()
                    .min(0.064);
                state.now = *now;
                if state.shown < self.source.len() {
                    if !self.active || motion::reduced() {
                        state.shown = self.source.len();
                    } else {
                        state.credit += delta
                            * (self.source[state.shown..].chars().count() as f32 / 0.15).max(70.0);
                        let count = state.credit.floor() as usize;
                        state.credit -= count as f32;
                        state.shown = self.source[state.shown..]
                            .char_indices()
                            .nth(count)
                            .map_or(self.source.len(), |(offset, _)| state.shown + offset);
                    }
                    shell.invalidate_layout();
                }
            }
            _ => {}
        }
        if self.active
            && state.shown < self.source.len()
            && state.window_active
            && !motion::reduced()
            && layout.bounds().expand(1.0).intersection(viewport).is_some()
        {
            shell.request_redraw_at(state.now + Duration::from_millis(8));
        }
        if let Some(child) = layout.children().next() {
            self.child.as_widget_mut().update(
                &mut tree.children[0],
                event,
                child,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
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
        if let Some(child) = layout.children().next() {
            self.child.as_widget().draw(
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
                self.child.as_widget().mouse_interaction(
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
            self.child
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
        self.child.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next()?,
            renderer,
            viewport,
            translation,
        )
    }
}
impl<'a, Message: 'a> From<StreamingParagraph<'a, Message>> for Element<'a, Message> {
    fn from(value: StreamingParagraph<'a, Message>) -> Self {
        Element::new(value)
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use super::*;
    use crate::snapshot::Snapshot;

    #[test]
    fn reveal_preserves_utf8_and_completion_retains_the_same_rich_text_layout() {
        let snapshot = Snapshot::software(Size::new(420.0, 400.0)).unwrap();
        let source = "Çizim 🦀 incelendi. **Alanlar** hesaplandı; `parcel.area` ve [belge](drawing://7) hazır.";
        let mut widget: Element<'_, ()> = StreamingParagraph::new(source, true, None).into();
        let mut tree = Tree::new(&widget);
        let limits = layout::Limits::new(Size::ZERO, Size::new(420.0, 400.0));
        let mut node = widget
            .as_widget_mut()
            .layout(&mut tree, snapshot.renderer(), &limits);
        let tag = tree.children[0].tag;
        let start = tree.state.downcast_ref::<State>().now;
        let mut previous = 0;
        for frame in 1..=100 {
            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);
            widget.as_widget_mut().update(
                &mut tree,
                &Event::Window(window::Event::RedrawRequested(
                    start + Duration::from_millis(frame * 16),
                )),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                snapshot.renderer(),
                &mut iced::advanced::clipboard::Null,
                &mut shell,
                &Rectangle::with_size(Size::new(420.0, 400.0)),
            );
            let shown = tree.state.downcast_ref::<State>().shown;
            assert!(shown >= previous && source.is_char_boundary(shown));
            previous = shown;
            node = widget
                .as_widget_mut()
                .layout(&mut tree, snapshot.renderer(), &limits);
        }
        assert_eq!(previous, source.len());
        let streamed_size = node.size();
        widget = StreamingParagraph::new(source, false, None).into();
        widget.as_widget().diff(&mut tree);
        node = widget
            .as_widget_mut()
            .layout(&mut tree, snapshot.renderer(), &limits);
        assert_eq!(tree.children[0].tag, tag);
        assert_eq!(node.size(), streamed_size);
        assert_eq!(tree.state.downcast_ref::<State>().shown, source.len());
    }
}
