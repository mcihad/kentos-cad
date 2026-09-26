//! İçeriğine göre uzayan, sınırına gelince esnek çocuğu kısalan dikey düzen.
//!
//! ```text
//! ┌ Başlık ─────────────────┐   ┌ Başlık ─────────────────┐
//! │ kısa gövde               │   │ uzun gövde            ▲ │
//! │                [Tamam]   │   │ …                     ▼ │
//! └──────────────────────────┘   │                [Tamam]  │
//!   içerik kadar                 └──────────────────────────┘
//!                                  sınır kadar; gövde kayar
//! ```
//!
//! Iced'in sütunu uzunluğu sabit ya da içeriğine göre olan çocukları sırayla
//! yerleştirir: içeriğine göre uzayan bir kaydırma alanı kendisinden sonra
//! gelen düğmelere yer bırakmaz. Sütunu dolduran (`Fill`) kaydırma alanı ise
//! kutuyu hep en büyük boyunda tutar. [`Fit`] önce esnek olmayan çocukları
//! yerleştirir, sonra esnek olanlara (ör. [`Fit::push_elastic`] ile eklenen,
//! yüksekliği `Shrink` bir kaydırma alanı; yüksekliği `Fill` olan çocuklar da
//! esnektir) kalan yeri verir; sonra hepsini eklendikleri sırayla alt alta
//! dizer. Böylece kutu içeriği kadar olur, sığmayınca gövde kayar ve
//! düğmeler hep görünür kalır.

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

/// Çocukları alt alta dizen, esnek çocuklarına kalan yeri veren sütun.
pub struct Fit<'a, Message> {
    children: Vec<Element<'a, Message>>,
    /// Açıkça esnek eklenen çocuklar (yüksekliği `Fill` olanlar ayrıca esnektir).
    elastic: Vec<bool>,
    spacing: f32,
    width: Length,
}

impl<'a, Message: 'a> Default for Fit<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> Fit<'a, Message> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            elastic: Vec::new(),
            spacing: 0.0,
            width: Length::Fill,
        }
    }

    pub fn push(mut self, child: impl Into<Element<'a, Message>>) -> Self {
        self.children.push(child.into());
        self.elastic.push(false);
        self
    }

    /// Öbürleri yerleştikten sonra kalan yeri alan çocuk: yüksekliği
    /// `Shrink` bir kaydırma alanı içeriği kadar, en çok kalan yer kadar olur.
    pub fn push_elastic(mut self, child: impl Into<Element<'a, Message>>) -> Self {
        self.children.push(child.into());
        self.elastic.push(true);
        self
    }

    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    fn is_elastic(&self, i: usize) -> bool {
        self.elastic[i] || self.children[i].as_widget().size().height.is_fill()
    }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Fit<'a, Message> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let limits = limits.width(self.width).height(Length::Shrink);
        let max = limits.max();
        let count = self.children.len();
        let mut used = self.spacing * count.saturating_sub(1) as f32;
        let mut nodes = vec![Node::default(); count];
        // Esnek olmayanlar önce, sonra esnekler; her biri o ana dek kalan yerle.
        let elastic: Vec<bool> = (0..count).map(|i| self.is_elastic(i)).collect();
        for pass in [false, true] {
            for i in (0..count).filter(|i| elastic[*i] == pass) {
                let room = layout::Limits::new(
                    Size::ZERO,
                    Size::new(max.width, (max.height - used).max(0.0)),
                );
                let node =
                    self.children[i]
                        .as_widget_mut()
                        .layout(&mut tree.children[i], renderer, &room);
                used += node.size().height;
                nodes[i] = node;
            }
        }
        let mut y = 0.0;
        let mut width: f32 = 0.0;
        for node in &mut nodes {
            let size = node.size();
            node.move_to_mut((0.0, y));
            y += size.height + self.spacing;
            width = width.max(size.width);
        }
        let height = (y - self.spacing).max(0.0);
        let size = limits.resolve(self.width, Length::Shrink, Size::new(width, height));
        Node::with_children(size, nodes)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, state), layout)| {
                    child
                        .as_widget_mut()
                        .operate(state, layout, renderer, operation);
                });
        });
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
        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
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
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, tree), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
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
        if let Some(viewport) = layout.bounds().intersection(viewport) {
            for ((child, tree), layout) in self
                .children
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
                .filter(|(_, layout)| layout.bounds().intersects(&viewport))
            {
                child
                    .as_widget()
                    .draw(tree, renderer, theme, style, layout, cursor, &viewport);
            }
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
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<Fit<'a, Message>> for Element<'a, Message> {
    fn from(fit: Fit<'a, Message>) -> Self {
        Element::new(fit)
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use iced::advanced::layout::Limits;
    use iced::advanced::widget::Tree;
    use iced::widget::{scrollable, space};
    use iced::{Element, Length, Size};

    use super::Fit;
    use crate::snapshot::Snapshot;

    /// Lays `fit` out within `height`: its height and each child's top and height.
    fn laid_out(fit: Fit<'_, ()>, height: f32) -> (f32, Vec<(f32, f32)>) {
        let snapshot = Snapshot::new(Size::new(400.0, height)).expect("çizici kurulamadı");
        let mut element: Element<'_, ()> = fit.into();
        let mut tree = Tree::new(&element);
        let limits = Limits::new(Size::ZERO, Size::new(400.0, height));
        let node = element
            .as_widget_mut()
            .layout(&mut tree, snapshot.renderer(), &limits);
        let children = node
            .children()
            .iter()
            .map(|c| (c.bounds().y, c.bounds().height))
            .collect();
        (node.size().height, children)
    }

    fn block<'a>(height: f32) -> Element<'a, ()> {
        space::vertical().height(Length::Fixed(height)).into()
    }

    fn body<'a>(height: f32) -> Element<'a, ()> {
        scrollable(space::vertical().height(Length::Fixed(height)))
            .height(Length::Shrink)
            .into()
    }

    fn dialog<'a>(content: f32) -> Fit<'a, ()> {
        Fit::new()
            .spacing(10.0)
            .push(block(30.0))
            .push_elastic(body(content))
            .push(block(40.0))
    }

    #[test]
    fn a_short_body_makes_a_short_box() {
        let (height, children) = laid_out(dialog(100.0), 600.0);
        assert_eq!(height, 30.0 + 10.0 + 100.0 + 10.0 + 40.0);
        assert_eq!(children, [(0.0, 30.0), (40.0, 100.0), (150.0, 40.0)]);
    }

    #[test]
    fn a_long_body_scrolls_and_the_buttons_stay() {
        let (height, children) = laid_out(dialog(2000.0), 600.0);
        assert_eq!(height, 600.0);
        // The body gets what the others leave; the last child ends at the bottom.
        assert_eq!(children[1], (40.0, 600.0 - 30.0 - 40.0 - 20.0));
        assert_eq!(children[2], (560.0, 40.0));
    }

    #[test]
    fn a_fill_child_takes_what_is_left_as_in_a_column() {
        let fit = Fit::new()
            .spacing(10.0)
            .push(block(30.0))
            .push(scrollable(space::vertical().height(Length::Fixed(50.0))).height(Length::Fill))
            .push(block(40.0));
        let (height, children) = laid_out(fit, 500.0);
        assert_eq!(height, 500.0);
        assert_eq!(children[2], (460.0, 40.0));
    }
}
