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
        let mut node = self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits);
        // A body that does not fit is at its most already and scrolls: its pictures may take the height shown (a tall
        // form beside a picture in a full screen window left the picture at its least, docs/adr/0937 §2).
        if !(exact || fills) && max.height.is_finite() && node.size().height >= max.height - 0.5 {
            ROOM.with(|room| room.set(Some(max.height)));
            node = self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits);
        }
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

/// A child held to a height (docs/adr/0938 §3):
///
/// - [`Held::within_room`]: no higher than the enclosing [`Reach`]'s room, so a scrolling column inside it (a dialog's
///   form) scrolls by itself instead of making the whole body scroll;
/// - [`Held::natural`]: as high as `natural` where there is room for it, less where the height it is given is less;
///   its content fills the height (a table whose rows then scroll inside it).
pub struct Held<'a, Message> {
    content: Element<'a, Message>,
    natural: Option<f32>,
}

impl<'a, Message: 'a> Held<'a, Message> {
    pub fn within_room(content: impl Into<Element<'a, Message>>) -> Self {
        Self { content: content.into(), natural: None }
    }

    pub fn natural(content: impl Into<Element<'a, Message>>, natural: f32) -> Self {
        Self { content: content.into(), natural: Some(natural.max(0.0)) }
    }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Held<'a, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        let own = self.content.as_widget().size();
        match self.natural {
            Some(_) => Size::new(own.width, Length::Shrink),
            None => own,
        }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let (min, max) = (limits.min(), limits.max());
        let node = match self.natural {
            Some(n) => {
                let h = n.min(max.height).max(min.height.min(n));
                let exact = Limits::new(Size::new(min.width, h), Size::new(max.width, h));
                self.content.as_widget_mut().layout(&mut tree.children[0], renderer, &exact)
            }
            None => {
                let h = room().map_or(max.height, |r| r.min(max.height));
                let held = Limits::new(Size::new(min.width, min.height.min(h)), Size::new(max.width, h));
                self.content.as_widget_mut().layout(&mut tree.children[0], renderer, &held)
            }
        };
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

impl<'a, Message: 'a> From<Held<'a, Message>> for Element<'a, Message> {
    fn from(held: Held<'a, Message>) -> Self {
        Element::new(held)
    }
}

/// A column whose growing children take the height the enclosing [`Reach`] has over, each at least its least
/// height; the others keep their own.
pub struct Grows<'a, Message> {
    children: Vec<Element<'a, Message>>,
    /// A growing child's least height; none for the others.
    least: Vec<Option<f32>>,
    /// Whether a growing child grows no further than its own height (a table: its rows, docs/adr/0938 §3).
    fits: Vec<bool>,
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
        Self { children: Vec::new(), least: Vec::new(), fits: Vec::new(), spacing: 0.0, width: Length::Fill }
    }

    pub fn push(mut self, child: impl Into<Element<'a, Message>>) -> Self {
        self.children.push(child.into());
        self.least.push(None);
        self.fits.push(false);
        self
    }

    /// A child that grows to what the others leave, never under `least` (logical pixels): a picture.
    pub fn push_grow(mut self, child: impl Into<Element<'a, Message>>, least: f32) -> Self {
        self.children.push(child.into());
        self.least.push(Some(least.max(0.0)));
        self.fits.push(false);
        self
    }

    /// A child as high as its own content (laid out without a height bound) when there is room, and given less where
    /// there is not, never under `least` (or its own height when lower): a table whose rows then scroll inside it,
    /// the window keeping its place (docs/adr/0938 §3). What it does not take goes to the others that grow.
    pub fn push_fit(mut self, child: impl Into<Element<'a, Message>>, least: f32) -> Self {
        self.children.push(child.into());
        self.least.push(Some(least.max(0.0)));
        self.fits.push(true);
        self
    }

    /// The gap between children, logical pixels. An `f32`, not `impl Into<f32>`: a float literal then is one
    /// without the `f32: From<{float}>` fallback Rust 1.97 began to warn about (docs/adr/8055 §2).
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

/// [`shares`] with each child's most (infinite: none): what one would take over its most goes to the others, in
/// proportion to their least heights, until none is over or all are at their most.
pub fn shares_capped(least: &[f32], most: &[f32], used: f32, room: Option<f32>) -> Vec<f32> {
    let mut out: Vec<f32> = least.iter().zip(most).map(|(l, m)| l.min(*m)).collect();
    let Some(room) = room else { return out };
    let mut over = (room - used - out.iter().sum::<f32>()).max(0.0);
    let mut open: Vec<usize> = (0..out.len()).filter(|&i| out[i] < most[i]).collect();
    while over > 1e-3 && !open.is_empty() {
        let weight: f32 = open.iter().map(|&i| least[i]).sum();
        let mut given = 0.0;
        let mut still = Vec::new();
        for &i in &open {
            let share = if weight > 0.0 { least[i] / weight } else { 1.0 / open.len() as f32 };
            let take = (over * share).min(most[i] - out[i]);
            out[i] += take;
            given += take;
            if out[i] < most[i] - 1e-3 {
                still.push(i);
            }
        }
        over -= given;
        if still.len() == open.len() {
            break;
        }
        open = still;
    }
    out
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
        // A child that fits is as high as its content at most: laid out without a bound first.
        let most: Vec<f32> = growing
            .iter()
            .map(|&i| {
                if !self.fits[i] {
                    return f32::INFINITY;
                }
                let free = Limits::new(Size::ZERO, Size::new(width, f32::INFINITY));
                self.children[i].as_widget_mut().layout(&mut tree.children[i], renderer, &free).size().height
            })
            .collect();
        let least: Vec<f32> = growing.iter().map(|i| self.least[*i].unwrap_or(0.0)).collect();
        // Without a room a fitting child is its own height (the body grows with it until it reaches the window and the
        // Reach lays it out again with a room); a picture its least.
        let heights: Vec<f32> = if room.is_none() {
            least.iter().zip(&most).map(|(l, m)| if m.is_finite() { *m } else { *l }).collect()
        } else {
            // A pixel kept over: the others' heights are fractional (text), and a body over its room by a rounding
            // shows a scrollbar for nothing (Kübaj, docs/adr/0938 §3).
            shares_capped(&least, &most, used, room.map(|r| r - 1.0))
        };
        for (i, height) in growing.into_iter().zip(heights) {
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
        // A resized dialog gives the body 600: the picture takes 600 − 30 − 50 − 20, a pixel kept over for rounding.
        let exact = Limits::new(Size::new(0.0, 600.0), Size::new(400.0, 600.0));
        let (height, children) = heights(body(), exact);
        assert_eq!(height, 600.0);
        assert_eq!(children, [30.0, 499.0, 50.0]);
    }

    #[test]
    fn a_body_at_its_own_size_keeps_the_least() {
        // A dialog at its own size: loose limits, the picture its least, the body as high as its content.
        let loose = Limits::new(Size::ZERO, Size::new(400.0, 600.0));
        let (height, children) = heights(body(), loose);
        assert_eq!(children, [30.0, 100.0, 50.0]);
        assert_eq!(height, 30.0 + 100.0 + 50.0 + 20.0);
    }

    /// A body taller than its room (a tall form beside the picture) scrolls at its most: the picture takes the height
    /// shown, less what the others in its column take.
    #[test]
    fn a_body_that_scrolls_gives_its_picture_the_height_shown() {
        let tall = Reach::new(
            scrollable(
                iced::widget::row![
                    block(2000.0),
                    Grows::new().spacing(10.0).push(block(30.0)).push_grow(picture(), 100.0).push(block(50.0)),
                ],
            )
            .height(Length::Shrink),
        );
        let snapshot = Snapshot::new(Size::new(400.0, 800.0)).expect("çizici kurulamadı");
        let mut element: Element<'_, ()> = tall.into();
        let mut tree = Tree::new(&element);
        let node = element.as_widget_mut().layout(
            &mut tree,
            snapshot.renderer(),
            &Limits::new(Size::ZERO, Size::new(400.0, 600.0)),
        );
        // Reach › scrollable › row › [block, Grows › children].
        let column = &node.children()[0].children()[0].children()[1];
        assert_eq!(node.size().height, 600.0);
        assert_eq!(column.children()[1].size().height, 600.0 - 30.0 - 50.0 - 20.0 - 1.0);
    }

    #[test]
    fn a_low_body_keeps_the_least_and_scrolls() {
        // Exact but lower than the content: the picture keeps its least, the body scrolls.
        let exact = Limits::new(Size::new(0.0, 120.0), Size::new(400.0, 120.0));
        let (height, children) = heights(body(), exact);
        assert_eq!((height, children), (120.0, vec![30.0, 100.0, 50.0]));
    }

    #[test]
    fn a_table_takes_its_rows_and_the_picture_the_rest() {
        use super::shares_capped;
        // A picture (least 100) and a table (least 60, its rows 150): room enough, the table its rows, the picture the
        // rest; little room, both in proportion; less than the least, the least.
        assert_eq!(shares_capped(&[100.0, 60.0], &[f32::INFINITY, 150.0], 50.0, Some(850.0)), [650.0, 150.0]);
        assert_eq!(shares_capped(&[100.0, 60.0], &[f32::INFINITY, 150.0], 50.0, Some(290.0)), [150.0, 90.0]);
        assert_eq!(shares_capped(&[100.0, 60.0], &[f32::INFINITY, 150.0], 50.0, Some(100.0)), [100.0, 60.0]);
        // A table shorter than its least keeps its own height.
        assert_eq!(shares_capped(&[100.0, 60.0], &[f32::INFINITY, 40.0], 0.0, Some(500.0)), [460.0, 40.0]);
    }

    /// A body taller than the window: its form scrolls by itself, the table takes what the picture leaves, and the
    /// body is the window's height (the table's rows scroll inside it, not the window).
    #[test]
    fn a_long_table_scrolls_inside_and_the_body_keeps_the_window() {
        use super::Held;
        let rows = 2000.0;
        let table = Held::natural(space::vertical().width(Length::Fill).height(Length::Fill), rows);
        let tall = Reach::new(
            scrollable(iced::widget::row![
                Held::within_room(scrollable(block(1500.0)).height(Length::Shrink)),
                Grows::new().spacing(10.0).push(block(30.0)).push_grow(picture(), 100.0).push_fit(table, 60.0),
            ])
            .height(Length::Shrink),
        );
        let snapshot = Snapshot::new(Size::new(400.0, 800.0)).expect("çizici kurulamadı");
        let mut element: Element<'_, ()> = tall.into();
        let mut tree = Tree::new(&element);
        let node = element.as_widget_mut().layout(
            &mut tree,
            snapshot.renderer(),
            &Limits::new(Size::ZERO, Size::new(400.0, 600.0)),
        );
        let row = &node.children()[0].children()[0];
        assert_eq!(node.size().height, 600.0);
        assert_eq!(row.size().height, 600.0, "the body does not scroll");
        assert_eq!(row.children()[0].size().height, 600.0, "the form is held to the window");
        let column = &row.children()[1];
        let (picture_h, table_h) = (column.children()[1].size().height, column.children()[2].size().height);
        assert!((picture_h + table_h - (600.0 - 30.0 - 20.0 - 1.0)).abs() < 0.01, "{picture_h} + {table_h}");
        assert!(table_h < rows && picture_h >= 100.0);
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
