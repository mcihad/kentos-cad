//! Büyüyen çizim: büyütülen ya da tam ekran yapılan kutuda çizim kalan bütün yüksekliği alır (docs/adr/0937 §2).
//!
//! ```text
//! ┌ Dever ───────────────────────────┐   ┌ Dever ─────────────────────────────────────────┐
//! │ form   ┌ çizim ───────────────┐  │   │ form   ┌ çizim ─────────────────────────────┐  │
//! │        └──────────────────────┘  │   │        │                                    │  │
//! │        [özet] [özet]  tablo      │   │        │      kalan yerin hepsi             │  │
//! │                 [Kapat] [Kaydet] │   │        └────────────────────────────────────┘  │
//! └──────────────────────────────────┘   │        [özet] [özet]  tablo                    │
//!   kendi boyunda: çizim en az boyunda   │                               [Kapat] [Kaydet] │
//!                                        └────────────────────────────────────────────────┘
//!                                          büyütülmüş: çizim büyür, düğmeler altta
//! ```
//!
//! Bir kutunun kayan gövdesi yükseklik sınırı olmadan yerleşir: iced'in kaydırma alanı içeriğine sınırsız yükseklik
//! verir; içindeki bir çizim kutunun büyüdüğünü bilemez, sabit boyunda kalır ve altında boşluk kalır. [`Reach`]
//! kaydırma alanını sarar: kutu büyütülünce ya da tam ekran yapılınca [`Fit`](crate::widget::fit::Fit) gövdeye
//! kesin bir yükseklik verir (ADR 0746); `Reach` bu yüksekliği içinde yerleşen [`Grows`] sütunlarına söyler, onların
//! büyüyen çocuğu öbürlerinin bıraktığı yeri alır, en az boyunun altına inmez. Kendi boyundaki kutu kesin yükseklik
//! vermez: çizim en az boyunda, kutu içeriği kadar kalır. Öbürleri sığmazsa çizim en az boyunda kalır, gövde kayar.
//!
//! `Grows` gövdenin en üstündeki sütundur (gövdenin satırının bir sütunu da olabilir); gövdeyle arasında dikey
//! boşluk bırakan bir kap olmamalı, yoksa içerik bu kadar taşar ve kayar.

use std::cell::Cell;

use iced::advanced::layout::{Layout, Limits, Node};
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

thread_local! {
    /// The height a [`Reach`] being laid out gives its content: the body's own height in a resized dialog.
    static ROOM: Cell<Option<f32>> = const { Cell::new(None) };
}

/// The height the enclosing [`Reach`] gives its content now; none outside one or in a dialog at its own size.
pub fn room() -> Option<f32> {
    ROOM.with(Cell::get)
}

/// A scrolling body that tells the [`Grows`] inside it the height it is given, when that height is the body's own
/// (exact limits, or a body that fills its height).
pub struct Reach<'a, Message> {
    content: Element<'a, Message>,
}

impl<'a, Message: 'a> Reach<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self { content: content.into() }
    }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Reach<'a, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let (min, max) = (limits.min(), limits.max());
        let exact = max.height.is_finite() && (max.height - min.height).abs() < 0.5;
        let fills = max.height.is_finite() && self.content.as_widget().size().height.is_fill();
        let previous = ROOM.with(|room| room.replace((exact || fills).then_some(max.height)));
        let node = self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits);
        ROOM.with(|room| room.set(previous));
        Node::with_children(node.size(), vec![node])
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        if let Some(child) = layout.children().next() {
            self.content.as_widget_mut().operate(&mut tree.children[0], child, renderer, operation);
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
        if let Some(child) = layout.children().next() {
            self.content
                .as_widget_mut()
                .update(&mut tree.children[0], event, child, cursor, renderer, clipboard, shell, viewport);
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
        layout.children().next().map_or_else(mouse::Interaction::default, |child| {
            self.content.as_widget().mouse_interaction(&tree.children[0], child, cursor, viewport, renderer)
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
        if let Some(child) = layout.children().next() {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, child, cursor, viewport);
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
        self.content.as_widget_mut().overlay(&mut tree.children[0], child, renderer, viewport, translation)
    }
}

impl<'a, Message: 'a> From<Reach<'a, Message>> for Element<'a, Message> {
    fn from(reach: Reach<'a, Message>) -> Self {
        Element::new(reach)
    }
}

/// A column whose growing children take the height the enclosing [`Reach`] has over, each at least its least
/// height; the others keep their own.
pub struct Grows<'a, Message> {
    children: Vec<Element<'a, Message>>,
    /// A growing child's least height; none for the others.
    least: Vec<Option<f32>>,
    spacing: f32,
    width: Length,
}

impl<'a, Message: 'a> Default for Grows<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> Grows<'a, Message> {
    pub fn new() -> Self {
        Self { children: Vec::new(), least: Vec::new(), spacing: 0.0, width: Length::Fill }
    }

    pub fn push(mut self, child: impl Into<Element<'a, Message>>) -> Self {
        self.children.push(child.into());
        self.least.push(None);
        self
    }

    /// A child that grows to what the others leave, never under `least` (logical pixels): a picture.
    pub fn push_grow(mut self, child: impl Into<Element<'a, Message>>, least: f32) -> Self {
        self.children.push(child.into());
        self.least.push(Some(least.max(0.0)));
        self
    }

    pub fn spacing(mut self, spacing: impl Into<f32>) -> Self {
        self.spacing = spacing.into();
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

/// The growing children's heights: each its least, and what `room` has over the others (`used`, spacing included)
/// shared in proportion to the least heights. Without a room, or none over, their least.
pub fn shares(least: &[f32], used: f32, room: Option<f32>) -> Vec<f32> {
    let sum: f32 = least.iter().sum();
    let over = room.map_or(0.0, |room| (room - used - sum).max(0.0));
    least
        .iter()
        .map(|m| {
            let share = if sum > 0.0 { m / sum } else { 1.0 / least.len().max(1) as f32 };
            m + over * share
        })
        .collect()
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Grows<'a, Message> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let limits = limits.width(self.width);
        let width = limits.max().width;
        let count = self.children.len();
        let room = room();
        let mut nodes = vec![Node::default(); count];
        let mut used = self.spacing * count.saturating_sub(1) as f32;
        for i in (0..count).filter(|i| self.least[*i].is_none()) {
            let node = self.children[i].as_widget_mut().layout(
                &mut tree.children[i],
                renderer,
                &Limits::new(Size::ZERO, Size::new(width, f32::INFINITY)),
            );
            used += node.size().height;
            nodes[i] = node;
        }
        let growing: Vec<usize> = (0..count).filter(|i| self.least[*i].is_some()).collect();
        let least: Vec<f32> = growing.iter().map(|i| self.least[*i].unwrap_or(0.0)).collect();
        for (i, height) in growing.into_iter().zip(shares(&least, used, room)) {
            nodes[i] = self.children[i].as_widget_mut().layout(
                &mut tree.children[i],
                renderer,
                &Limits::new(Size::new(0.0, height), Size::new(width, height)),
            );
        }
        let mut y = 0.0;
        let mut widest: f32 = 0.0;
        for node in &mut nodes {
            let size = node.size();
            node.move_to_mut((0.0, y));
            y += size.height + self.spacing;
            widest = widest.max(size.width);
        }
        let height = (y - self.spacing).max(0.0);
        let size = limits.resolve(self.width, Length::Shrink, Size::new(widest, height));
        Node::with_children(size, nodes)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children.iter_mut().zip(&mut tree.children).zip(layout.children()).for_each(|((child, state), layout)| {
                child.as_widget_mut().operate(state, layout, renderer, operation);
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
        for ((child, tree), layout) in self.children.iter_mut().zip(&mut tree.children).zip(layout.children()) {
            child.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
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
            .map(|((child, tree), layout)| child.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer))
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
        for ((child, tree), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .filter(|(_, layout)| layout.bounds().intersects(viewport))
        {
            child.as_widget().draw(tree, renderer, theme, style, layout, cursor, viewport);
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
        overlay::from_children(&mut self.children, tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message: 'a> From<Grows<'a, Message>> for Element<'a, Message> {
    fn from(grows: Grows<'a, Message>) -> Self {
        Element::new(grows)
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use iced::advanced::layout::Limits;
    use iced::advanced::widget::Tree;
    use iced::widget::{scrollable, space};
    use iced::{Element, Length, Size};

    use super::{Grows, Reach, shares};
    use crate::snapshot::Snapshot;

    fn block<'a>(height: f32) -> Element<'a, ()> {
        space::vertical().height(Length::Fixed(height)).into()
    }

    fn picture<'a>() -> Element<'a, ()> {
        space::vertical().width(Length::Fill).height(Length::Fill).into()
    }

    /// The body: a scrolling area round a column with a picture (least 100) and a table 50 high, 10 apart.
    fn body<'a>() -> Element<'a, ()> {
        Reach::new(
            scrollable(Grows::new().spacing(10.0).push(block(30.0)).push_grow(picture(), 100.0).push(block(50.0)))
                .height(Length::Shrink),
        )
        .into()
    }

    /// Lays `element` out within `limits`: the column's children's heights.
    fn heights(mut element: Element<'_, ()>, limits: Limits) -> (f32, Vec<f32>) {
        let snapshot = Snapshot::new(Size::new(400.0, 800.0)).expect("çizici kurulamadı");
        let mut tree = Tree::new(&element);
        let node = element.as_widget_mut().layout(&mut tree, snapshot.renderer(), &limits);
        // Reach › scrollable › Grows › children.
        let column = &node.children()[0].children()[0];
        (node.size().height, column.children().iter().map(|c| c.size().height).collect())
    }

    #[test]
    fn a_body_given_its_height_grows_the_picture() {
        // A resized dialog gives the body 600: the picture takes 600 − 30 − 50 − 20.
        let exact = Limits::new(Size::new(0.0, 600.0), Size::new(400.0, 600.0));
        let (height, children) = heights(body(), exact);
        assert_eq!(height, 600.0);
        assert_eq!(children, [30.0, 500.0, 50.0]);
    }

    #[test]
    fn a_body_at_its_own_size_keeps_the_least() {
        // A dialog at its own size: loose limits, the picture its least, the body as high as its content.
        let loose = Limits::new(Size::ZERO, Size::new(400.0, 600.0));
        let (height, children) = heights(body(), loose);
        assert_eq!(children, [30.0, 100.0, 50.0]);
        assert_eq!(height, 30.0 + 100.0 + 50.0 + 20.0);
    }

    #[test]
    fn a_low_body_keeps_the_least_and_scrolls() {
        // Exact but lower than the content: the picture keeps its least, the body scrolls.
        let exact = Limits::new(Size::new(0.0, 120.0), Size::new(400.0, 120.0));
        let (height, children) = heights(body(), exact);
        assert_eq!((height, children), (120.0, vec![30.0, 100.0, 50.0]));
    }

    #[test]
    fn several_pictures_share_what_is_over_by_their_least() {
        assert_eq!(shares(&[100.0, 300.0], 50.0, Some(850.0)), [200.0, 600.0]);
        assert_eq!(shares(&[100.0], 50.0, None), [100.0]);
        assert_eq!(shares(&[100.0], 900.0, Some(500.0)), [100.0]);
    }

    /// Outside a Reach the column is a column: the picture its least.
    #[test]
    fn outside_a_reach_the_picture_keeps_its_least() {
        let snapshot = Snapshot::new(Size::new(400.0, 800.0)).expect("çizici kurulamadı");
        let mut element: Element<'_, ()> = Grows::new().push_grow(picture(), 80.0).push(block(20.0)).into();
        let mut tree = Tree::new(&element);
        let node = element.as_widget_mut().layout(
            &mut tree,
            snapshot.renderer(),
            &Limits::new(Size::new(0.0, 600.0), Size::new(400.0, 600.0)),
        );
        assert_eq!(node.children()[0].size().height, 80.0);
    }
}
