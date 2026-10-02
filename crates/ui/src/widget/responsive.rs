//! Responsive layout that retains its child during local animation relayouts.
//!
//! A new application view still builds fresh content. Within that view, equal
//! constraints preserve the widget values (including Iced button hover/status),
//! so a character reveal cannot turn controls dim for one frame.

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

pub struct Responsive<'a, Message> {
    view: Box<dyn Fn(Size) -> Element<'a, Message> + 'a>,
    width: Length,
    height: Length,
    bounds: Option<Size>,
    content: Element<'a, Message>,
}

impl<'a, Message: 'a> Responsive<'a, Message> {
    pub fn new(view: impl Fn(Size) -> Element<'a, Message> + 'a) -> Self {
        Self {
            view: Box::new(view),
            width: Length::Fill,
            height: Length::Fill,
            bounds: None,
            content: iced::widget::space().into(),
        }
    }
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Responsive<'_, Message> {
    fn diff(&self, _: &mut Tree) {} // The child's type depends on layout bounds.
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        let bounds = limits.max();
        if self.bounds != Some(bounds) {
            self.content = (self.view)(bounds);
            tree.diff_children(std::slice::from_ref(&self.content));
            self.bounds = Some(bounds);
        }
        let child =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &limits.loose());
        layout::Node::with_children(
            limits.resolve(self.width, self.height, child.size()),
            vec![child],
        )
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
impl<'a, Message: 'a> From<Responsive<'a, Message>> for Element<'a, Message> {
    fn from(value: Responsive<'a, Message>) -> Self {
        Element::new(value)
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use super::*;
    use crate::{style, theme::typography};
    use iced::advanced::renderer::{Headless, Renderer as _};

    #[test]
    fn local_relayout_preserves_button_paint_after_its_redraw_event() {
        typography::load();
        let mut renderer = iced::futures::executor::block_on(<Renderer as Headless>::new(
            typography::ui(),
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        let mut widget: Element<'_, ()> = Responsive::new(|_| {
            iced::widget::button("Araç ayrıntıları")
                .on_press(())
                .style(style::button::flat)
                .into()
        })
        .height(Length::Shrink)
        .into();
        let mut tree = Tree::new(&widget);
        let limits = layout::Limits::new(Size::ZERO, Size::new(420.0, 200.0));
        let node = widget.as_widget_mut().layout(&mut tree, &renderer, &limits);
        let mut messages = Vec::new();
        widget.as_widget_mut().update(
            &mut tree,
            &Event::Window(iced::window::Event::RedrawRequested(
                iced::time::Instant::now(),
            )),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut Shell::new(&mut messages),
            &Rectangle::with_size(Size::new(420.0, 200.0)),
        );
        // Draw the cached widget, then repeat the relayout an accordion/stream
        // requests in that same frame. Its enabled control must paint identically.
        let theme = crate::theme::theme(crate::theme::Mode::Dark, crate::theme::Accent::default());
        let draw = |widget: &Element<'_, ()>,
                    tree: &Tree,
                    node: &layout::Node,
                    renderer: &mut Renderer| {
            renderer.reset(Rectangle::with_size(Size::new(420.0, 200.0)));
            widget.as_widget().draw(
                tree,
                renderer,
                &theme,
                &renderer::Style {
                    text_color: theme.palette().text,
                },
                Layout::new(node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(Size::new(420.0, 200.0)),
            );
            renderer.screenshot(Size::new(420, 200), 1.0, theme.palette().background)
        };
        let before = draw(&widget, &tree, &node, &mut renderer);
        let node = widget.as_widget_mut().layout(&mut tree, &renderer, &limits);
        assert!(
            draw(&widget, &tree, &node, &mut renderer) == before,
            "enabled controls changed color during relayout"
        );
    }
}
