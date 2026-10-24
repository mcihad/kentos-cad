//! A narrow tab strip reveals the active tab on activation or resize, then leaves manual
//! scrolling alone. Bounds come from the laid-out tab widgets, not estimates of their text.
use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget, operation, tree};
use iced::advanced::{Clipboard, Shell};
use iced::widget::scrollable;
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

pub(super) fn active_tabs<'a, Message: 'a>(
    row: Element<'a, Message>,
    selected: usize,
    name: String,
) -> Element<'a, Message> {
    Element::new(ActiveTabs {
        content: scrollable(row)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .width(Length::Fill)
            .height(super::tab_height())
            .into(),
        selected,
        name,
    })
}

#[derive(Default)]
struct State {
    active: Option<(usize, String)>,
    width: f32,
}
struct ActiveTabs<'a, Message> {
    content: Element<'a, Message>,
    selected: usize,
    name: String,
}

impl<Message> Widget<Message, Theme, Renderer> for ActiveTabs<'_, Message> {
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
    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let state = tree.state.downcast_mut::<State>();
        let active = (self.selected, self.name.clone());
        if state.active.as_ref() != Some(&active) || state.width != node.size().width {
            if let Some(tab) = node
                .children()
                .first()
                .and_then(|row| row.children().get(self.selected))
            {
                let right = tab.bounds().x + tab.bounds().width;
                let offset = (right + 8. - node.size().width).max(0.);
                self.content.as_widget_mut().operate(
                    &mut tree.children[0],
                    Layout::new(&node),
                    renderer,
                    &mut Reveal(offset),
                );
            }
            state.active = Some(active);
            state.width = node.size().width;
        }
        node
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
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
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
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        op: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, op);
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
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}
struct Reveal(f32);
impl Operation for Reveal {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }
    fn scrollable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        _: Rectangle,
        _: Rectangle,
        _: Vector,
        state: &mut dyn operation::Scrollable,
    ) {
        state.scroll_to(operation::scrollable::AbsoluteOffset {
            x: Some(self.0),
            y: None,
        });
    }
}
