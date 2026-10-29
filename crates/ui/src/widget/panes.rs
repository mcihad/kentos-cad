//! Üç bölmeli düzen: solda liste, ortada çalışma alanı, sağda denetçi (ör. form
//! tasarımcısı).
//!
//! ```text
//! ┌ liste ──┐ ┌ çalışma alanı ─────────────┐ ┌ denetçi ──────┐
//! │ %20     │ │ kalan                      │ │ %36           │
//! │ en az … │ │ en az …                    │ │ en az …       │
//! └─────────┘ └────────────────────────────┘ └───────────────┘
//! ```
//!
//! Yan bölmeler kutunun genişliğinden paylarını alır, en az ve en çok
//! sınırları içinde; orta bölme kalanı alır. Orta bölmeye en azı kalmazsa
//! yan bölmeler kendi en azlarına dek geri verir, önce sağdaki. Genişlikler
//! içerikten değil yerden bulunur: denetçinin sekmesi değişince bölmeler
//! yerinden oynamaz. Sınırlar 12 piksellik gövde metnine göre verilir, yazı
//! boyutuyla büyür ([`typography::scaled`]); yazısı ölçülen bir şeyin
//! sığması gerekiyorsa (ör. sekmeler, [`segmented::natural_width`])
//! genişliği en az olarak verilir ([`Panes::least`]).
//!
//! ```ignore
//! Panes::new(list, tree, inspector)
//!     .left(0.2, 200.0, 260.0)
//!     .right(0.36, 340.0, 480.0)
//!     .least(Side::Right, segmented::natural_width(&TABS, false))
//! ```
//!
//! [`segmented::natural_width`]: crate::widget::segmented::natural_width

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::theme::typography;

/// Bir yan bölme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// Bir yan bölmenin payı ve sınırları (sınırlar 12 piksellik metne göre).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Share {
    share: f32,
    least: f32,
    most: f32,
    /// The least in pixels a measured text needs (not scaled again).
    fits: f32,
}

/// Üç bölmeli düzen.
pub struct Panes<'a, Message> {
    children: [Element<'a, Message>; 3],
    sides: [Share; 2],
    /// The middle's least width (at 12 px text).
    middle: f32,
    spacing: f32,
    height: Length,
}

impl<'a, Message: 'a> Panes<'a, Message> {
    pub fn new(
        left: impl Into<Element<'a, Message>>,
        middle: impl Into<Element<'a, Message>>,
        right: impl Into<Element<'a, Message>>,
    ) -> Self {
        Self {
            children: [left.into(), middle.into(), right.into()],
            sides: [
                Share {
                    share: 0.2,
                    least: 200.0,
                    most: 260.0,
                    fits: 0.0,
                },
                Share {
                    share: 0.36,
                    least: 320.0,
                    most: 480.0,
                    fits: 0.0,
                },
            ],
            middle: 280.0,
            spacing: 12.0,
            height: Length::Fill,
        }
    }

    /// Sol bölmenin payı (kutunun genişliğinin kesri), en az ve en çok genişliği.
    pub fn left(mut self, share: f32, least: f32, most: f32) -> Self {
        self.sides[0] = Share {
            share,
            least,
            most,
            ..self.sides[0]
        };
        self
    }

    /// Sağ bölmenin payı, en az ve en çok genişliği.
    pub fn right(mut self, share: f32, least: f32, most: f32) -> Self {
        self.sides[1] = Share {
            share,
            least,
            most,
            ..self.sides[1]
        };
        self
    }

    /// Yan bölmenin içindeki ölçülmüş bir şeyin (ör. sekmeler) sığması için
    /// gereken genişlik, piksel olarak: bölme bundan dar olmaz (yer varsa).
    pub fn least(mut self, side: Side, pixels: f32) -> Self {
        let i = match side {
            Side::Left => 0,
            Side::Right => 1,
        };
        self.sides[i].fits = self.sides[i].fits.max(pixels);
        self
    }

    /// Orta bölmeye kalması istenen en az genişlik.
    pub fn middle(mut self, least: f32) -> Self {
        self.middle = least;
        self
    }

    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

/// The side panes' widths in `width` (docs/adr/0749): each its share between its least and most (a measured
/// least wins over the most), the middle keeping its least while the sides can give back, the right first.
pub fn widths(width: f32, left: [f32; 4], right: [f32; 4], middle: f32, spacing: f32) -> [f32; 3] {
    let bounds = |[share, least, most, fits]: [f32; 4]| {
        let least = least.max(fits);
        ((share * width).clamp(least, most.max(least)), least)
    };
    let ((mut l, l_least), (mut r, r_least)) = (bounds(left), bounds(right));
    let gaps = 2.0 * spacing;
    let mut over = l + r + gaps + middle - width;
    if over > 0.0 {
        let give = (r - r_least).clamp(0.0, over);
        r -= give;
        over -= give;
        let give = (l - l_least).clamp(0.0, over);
        l -= give;
    }
    [l, (width - l - r - gaps).max(0.0), r]
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Panes<'a, Message> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, self.height)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let limits = limits.width(Length::Fill).height(self.height);
        let max = limits.max();
        let s = typography::scaled;
        let side = |sh: Share| [sh.share, s(sh.least), s(sh.most), sh.fits];
        let spacing = s(self.spacing);
        let w = widths(
            max.width,
            side(self.sides[0]),
            side(self.sides[1]),
            s(self.middle),
            spacing,
        );
        let mut nodes = Vec::with_capacity(3);
        let mut x = 0.0;
        let mut tallest: f32 = 0.0;
        for (i, (child, tree)) in self.children.iter_mut().zip(&mut tree.children).enumerate() {
            let room = layout::Limits::new(Size::new(w[i], 0.0), Size::new(w[i], max.height));
            let node = child
                .as_widget_mut()
                .layout(tree, renderer, &room)
                .move_to((x, 0.0));
            tallest = tallest.max(node.size().height);
            x += w[i] + spacing;
            nodes.push(node);
        }
        let size = limits.resolve(Length::Fill, self.height, Size::new(max.width, tallest));
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

impl<'a, Message: 'a> From<Panes<'a, Message>> for Element<'a, Message> {
    fn from(panes: Panes<'a, Message>) -> Self {
        Element::new(panes)
    }
}

#[cfg(test)]
mod tests {
    use super::widths;

    const LEFT: [f32; 4] = [0.2, 200.0, 260.0, 0.0];
    const RIGHT: [f32; 4] = [0.36, 320.0, 480.0, 0.0];

    #[test]
    fn the_sides_take_their_shares_within_their_bounds_and_the_middle_the_rest() {
        // A wide box: the sides at their shares.
        assert_eq!(
            widths(1000.0, LEFT, RIGHT, 280.0, 12.0),
            [200.0, 416.0, 360.0]
        );
        // A very wide one: the sides at their most, the middle gets the room.
        assert_eq!(
            widths(2000.0, LEFT, RIGHT, 280.0, 12.0),
            [260.0, 1236.0, 480.0]
        );
        // A measured least wins over the share (and over the most).
        assert_eq!(
            widths(1000.0, LEFT, [0.36, 320.0, 480.0, 500.0], 280.0, 12.0),
            [200.0, 276.0, 500.0]
        );
    }

    #[test]
    fn short_of_room_the_right_gives_back_first_then_the_left() {
        // 900: left 200 (its least), right 324, middle 900 − 200 − 324 − 24 = 352.
        assert_eq!(
            widths(900.0, LEFT, RIGHT, 280.0, 12.0),
            [200.0, 352.0, 324.0]
        );
        // 950 with a middle of at least 400: the right (342) gives 16 back.
        assert_eq!(
            widths(950.0, LEFT, RIGHT, 400.0, 12.0),
            [200.0, 400.0, 326.0]
        );
        // 1000 with a middle of at least 500: the right (360) down to its least 320, then the left (250)
        // down to its least 200; the middle has 456, not 500.
        assert_eq!(
            widths(1000.0, [0.25, 200.0, 260.0, 0.0], RIGHT, 500.0, 12.0),
            [200.0, 456.0, 320.0]
        );
        // Narrower than the leasts: the sides at their least, the middle what is left.
        assert_eq!(
            widths(700.0, LEFT, RIGHT, 300.0, 12.0),
            [200.0, 156.0, 320.0]
        );
    }
}
