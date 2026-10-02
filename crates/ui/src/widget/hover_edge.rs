//! Üzerine gelince ince kenar: içeriğin yerini ve boyunu değiştirmeden,
//! imleç üstündeyken çevresine belirgin bir çizgi çizer (ör. şeridin
//! bölünmüş düğmesi: iki parçası ayrı aydınlanır, bütünü çizgiyle bir
//! düğme olduğunu belli eder; DESIGN.md §7.3.1).
//!
//! ```ignore
//! hover_edge(row![main, arrow])
//! ```

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::{
    Background, Border, Color, Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector,
    mouse,
};

use crate::style::button::radius;
use crate::theme::Tokens;

/// Üzerine gelince kenarı çizilen içerik.
pub struct HoverEdge<'a, Message> {
    content: Element<'a, Message>,
}

/// İmleç üstündeyken çevresine belirgin kenar çizen kap.
pub fn hover_edge<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> HoverEdge<'a, Message> {
    HoverEdge {
        content: content.into(),
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct State {
    hovered: bool,
}

impl<Message> Widget<Message, Theme, Renderer> for HoverEdge<'_, Message> {
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
        if let Some(child) = layout.children().next() {
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
        }

        if let Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft) = event {
            let hovered = cursor.is_over(layout.bounds());
            let state = tree.state.downcast_mut::<State>();

            if state.hovered != hovered {
                state.hovered = hovered;
                shell.request_redraw();
            }
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
            .map_or_else(mouse::Interaction::default, |child| {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    child,
                    cursor,
                    viewport,
                    renderer,
                )
            })
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
        let Some(child) = layout.children().next() else {
            return;
        };

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            child,
            cursor,
            viewport,
        );

        if cursor.is_over(layout.bounds()) {
            renderer.fill_quad(
                Quad {
                    bounds: layout.bounds(),
                    border: Border {
                        color: Tokens::of(theme).border_strong(),
                        width: 1.0,
                        radius: radius().into(),
                    },
                    ..Quad::default()
                },
                Background::Color(Color::TRANSPARENT),
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

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let child = layout.children().next()?;

        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            child,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<HoverEdge<'a, Message>> for Element<'a, Message> {
    fn from(edge: HoverEdge<'a, Message>) -> Self {
        Element::new(edge)
    }
}
