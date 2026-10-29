//! Geniş ya da dar: geniş düzen yerine sığarsa o, sığmazsa dar olanı (ör.
//! araç çubuğunda adlı düğmeler, dar yerde yalnız ikonları; adları ipucunda).
//!
//! ```text
//! ┌──────────────────────────────┐   ┌───────────────┐
//! │ ▭ Sekme  ▤ Grup  ⓘ Not  — ⚭ │   │ ▭ ▤ ⓘ — ⚭    │
//! └──────────────────────────────┘   └───────────────┘
//!   sığıyor: geniş                     sığmıyor: dar
//! ```
//!
//! Geniş düzen genişliği sınırsız yerde ölçülür; bu yüzden içinde genişliği
//! `Fill` olan bir parça olmamalıdır (yayılacak boşluğu düzenin dışına koyun).
//! İkisi de ağaçta durur: hangisinin gösterildiği değişince durumları (ör.
//! üzerine gelinmiş düğme) kaybolmaz.
//!
//! ```ignore
//! fallback(named_tools, icon_tools)
//! ```

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

/// Geniş ya da dar düzen.
pub struct Fallback<'a, Message> {
    /// The wide one and the narrow one.
    children: [Element<'a, Message>; 2],
}

/// Geniş düzen sığarsa o, sığmazsa dar olan.
pub fn fallback<'a, Message: 'a>(
    wide: impl Into<Element<'a, Message>>,
    narrow: impl Into<Element<'a, Message>>,
) -> Fallback<'a, Message> {
    Fallback {
        children: [wide.into(), narrow.into()],
    }
}

/// Which one is shown: 0 the wide, 1 the narrow.
#[derive(Default)]
struct State {
    shown: usize,
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Fallback<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let max = limits.max();
        let free = layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, max.height));
        let wide = self.children[0]
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &free);
        let shown = usize::from(wide.size().width > max.width + 0.5);
        tree.state.downcast_mut::<State>().shown = shown;
        let node = if shown == 0 {
            wide
        } else {
            self.children[1]
                .as_widget_mut()
                .layout(&mut tree.children[1], renderer, limits)
        };
        let size = limits.resolve(Length::Shrink, Length::Shrink, node.size());
        Node::with_children(size, vec![node])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let shown = tree.state.downcast_ref::<State>().shown;
        if let Some(child) = layout.children().next() {
            self.children[shown].as_widget_mut().operate(
                &mut tree.children[shown],
                child,
                renderer,
                operation,
            );
        }
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
        let shown = tree.state.downcast_ref::<State>().shown;
        if let Some(child) = layout.children().next() {
            self.children[shown].as_widget_mut().update(
                &mut tree.children[shown],
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

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let shown = tree.state.downcast_ref::<State>().shown;
        layout
            .children()
            .next()
            .map_or(mouse::Interaction::default(), |child| {
                self.children[shown].as_widget().mouse_interaction(
                    &tree.children[shown],
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
        let shown = tree.state.downcast_ref::<State>().shown;
        if let Some(child) = layout.children().next() {
            self.children[shown].as_widget().draw(
                &tree.children[shown],
                renderer,
                theme,
                style,
                child,
                cursor,
                viewport,
            );
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
        let shown = tree.state.downcast_ref::<State>().shown;
        let child = layout.children().next()?;
        let [wide, narrow] = &mut self.children;
        let chosen = if shown == 0 { wide } else { narrow };
        chosen.as_widget_mut().overlay(
            &mut tree.children[shown],
            child,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<Fallback<'a, Message>> for Element<'a, Message> {
    fn from(fallback: Fallback<'a, Message>) -> Self {
        Element::new(fallback)
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use iced::advanced::layout::Limits;
    use iced::advanced::widget::Tree;
    use iced::widget::space;
    use iced::{Element, Length, Size};

    use super::fallback;
    use crate::snapshot::Snapshot;

    /// `fallback(wide, narrow)` laid out `room` wide: its width.
    fn width(room: f32) -> f32 {
        let snapshot = Snapshot::new(Size::new(400.0, 100.0)).expect("a renderer");
        let wide = space::horizontal().width(Length::Fixed(300.0));
        let narrow = space::horizontal().width(Length::Fixed(120.0));
        let mut element: Element<'_, ()> = fallback(wide, narrow).into();
        let mut tree = Tree::new(&element);
        let limits = Limits::new(Size::ZERO, Size::new(room, 100.0));
        element
            .as_widget_mut()
            .layout(&mut tree, snapshot.renderer(), &limits)
            .size()
            .width
    }

    #[test]
    fn the_wide_one_when_it_fits_else_the_narrow_one() {
        assert_eq!(width(400.0), 300.0);
        assert_eq!(width(300.0), 300.0);
        assert_eq!(width(299.0), 120.0);
        assert_eq!(
            width(100.0),
            100.0,
            "the narrow one too wide: within the room"
        );
    }
}
