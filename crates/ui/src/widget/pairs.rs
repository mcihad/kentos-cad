//! Ad–değer çiftleri iki sütunda, CSS'in `grid-template-columns: auto 1fr`'ı
//! gibi. Adlar en geniş adın genişliğindedir, değerler kalan yerdedir. Uzun
//! bir değer kayar ve satırını yükseltir; alttaki adlar değerleriyle hizada
//! kalır (iki ayrı sütunda olsalardı kayardı).
//!
//! ```text
//! Ada                        101
//! Nitelik     Konut alanı, ayrık
//!                   nizam, 4 kat
//! Alan               1.234,56 m²
//! ```
//!
//! Bileşen içeriğine göre daralır ve kendisine verilen en büyük genişliği
//! aşmaz (ör. bir kartın `max_width`'i); değerler o genişlikte kayar.
//!
//! ```ignore
//! Pairs::new()
//!     .push(label::caption("Ada"), label::body("101"))
//!     .push(label::caption("Nitelik"), label::body(nitelik).wrapping(Wrapping::WordOrGlyph))
//!     .align_values(Horizontal::Right)
//! ```

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::alignment::Horizontal;
use iced::{Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

/// Ad–değer çiftleri (bkz. modül).
pub struct Pairs<'a, Message> {
    /// Sırayla ad, değer, ad, değer…
    cells: Vec<Element<'a, Message>>,
    spacing_x: f32,
    spacing_y: f32,
    align: Horizontal,
}

impl<Message> Default for Pairs<'_, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message> Pairs<'a, Message> {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            spacing_x: 12.0,
            spacing_y: 2.0,
            align: Horizontal::Left,
        }
    }

    /// Bir satır: adı ve değeri.
    #[must_use]
    pub fn push(
        mut self,
        name: impl Into<Element<'a, Message>>,
        value: impl Into<Element<'a, Message>>,
    ) -> Self {
        self.cells.push(name.into());
        self.cells.push(value.into());
        self
    }

    /// Ad ile değer arasındaki boşluk.
    #[must_use]
    pub fn spacing_x(mut self, spacing: f32) -> Self {
        self.spacing_x = spacing;
        self
    }

    /// Satırlar arasındaki boşluk.
    #[must_use]
    pub fn spacing_y(mut self, spacing: f32) -> Self {
        self.spacing_y = spacing;
        self
    }

    /// Değerlerin kendi sütunlarındaki yeri (web'in `text-align`'ı).
    #[must_use]
    pub fn align_values(mut self, align: Horizontal) -> Self {
        self.align = align;
        self
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

impl<'a, Message: 'a> From<Pairs<'a, Message>> for Element<'a, Message> {
    fn from(pairs: Pairs<'a, Message>) -> Self {
        Element::new(pairs)
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Pairs<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        self.cells.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.cells);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let max = limits.max();
        let open = |width: f32| layout::Limits::new(Size::ZERO, Size::new(width, f32::INFINITY));
        // The names first: the widest one sets their column.
        let mut names = Vec::with_capacity(self.cells.len() / 2);
        let mut values = Vec::with_capacity(self.cells.len() / 2);
        for (cell, state) in self.cells.iter_mut().zip(&mut tree.children).step_by(2) {
            names.push(
                cell.as_widget_mut()
                    .layout(state, renderer, &open(max.width)),
            );
        }
        let name_width = names.iter().map(|n| n.size().width).fold(0.0, f32::max);
        let x = name_width + self.spacing_x;
        let room = open((max.width - x).max(0.0));
        for (cell, state) in self
            .cells
            .iter_mut()
            .zip(&mut tree.children)
            .skip(1)
            .step_by(2)
        {
            values.push(cell.as_widget_mut().layout(state, renderer, &room));
        }
        let value_width = values.iter().map(|v| v.size().width).fold(0.0, f32::max);
        let rows: f32 = names
            .iter()
            .zip(&values)
            .map(|(n, v)| n.size().height.max(v.size().height))
            .sum();
        let natural = if names.is_empty() {
            Size::ZERO
        } else {
            let gaps = self.spacing_y * (names.len() - 1) as f32;
            Size::new(x + value_width, rows + gaps)
        };
        // Given more width than it needs (a card's), the values' column
        // reaches the right edge (the web's `1fr`).
        let size = limits.resolve(Length::Shrink, Length::Shrink, natural);
        let column = (size.width - x).max(value_width);
        let mut nodes = Vec::with_capacity(self.cells.len());
        let mut y = 0.0;
        for (name, value) in names.into_iter().zip(values) {
            let height = name.size().height.max(value.size().height);
            let free = column - value.size().width;
            let at = match self.align {
                Horizontal::Left => x,
                Horizontal::Center => x + free / 2.0,
                Horizontal::Right => x + free,
            };
            nodes.push(name.move_to(Point::new(0.0, y)));
            nodes.push(value.move_to(Point::new(at, y)));
            y += height + self.spacing_y;
        }
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
            for ((cell, state), layout) in self
                .cells
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                cell.as_widget_mut()
                    .operate(state, layout, renderer, operation);
            }
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
        for ((cell, state), layout) in self
            .cells
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            cell.as_widget_mut().update(
                state, event, layout, cursor, renderer, clipboard, shell, viewport,
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
        self.cells
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((cell, state), layout)| {
                cell.as_widget()
                    .mouse_interaction(state, layout, cursor, viewport, renderer)
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
        for ((cell, state), layout) in self.cells.iter().zip(&tree.children).zip(layout.children())
        {
            cell.as_widget()
                .draw(state, renderer, theme, style, layout, cursor, viewport);
        }
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use iced::Size;
    use iced::advanced::layout::Limits;
    use iced::advanced::widget::{Tree, Widget};
    use iced::alignment::Horizontal;
    use iced::widget::space;

    use super::Pairs;
    use crate::snapshot::Snapshot;

    /// Boxes of known sizes stand for the texts: the names' column is the
    /// widest name, a tall value makes its row tall, values go right.
    #[test]
    fn names_share_the_widest_column_and_rows_take_their_tallest_cell() {
        let mut pairs: Pairs<'_, ()> = Pairs::new()
            .push(space().width(30).height(14), space().width(20).height(14))
            .push(space().width(50).height(14), space().width(80).height(42))
            .push(space().width(20).height(14), space().width(60).height(14))
            .spacing_x(12.0)
            .spacing_y(2.0)
            .align_values(Horizontal::Right);
        let mut tree = Tree::new(&pairs as &dyn Widget<(), _, _>);
        let snapshot = Snapshot::software(Size::new(300.0, 200.0)).expect("a renderer");
        let node = pairs.layout(
            &mut tree,
            snapshot.renderer(),
            &Limits::new(Size::ZERO, Size::new(260.0, 400.0)),
        );
        assert_eq!(
            node.size(),
            Size::new(50.0 + 12.0 + 80.0, 14.0 + 2.0 + 42.0 + 2.0 + 14.0)
        );
        let at: Vec<(f32, f32)> = node
            .children()
            .iter()
            .map(|c| (c.bounds().x, c.bounds().y))
            .collect();
        assert_eq!(
            at,
            [
                (0.0, 0.0),
                (62.0 + 60.0, 0.0),
                (0.0, 16.0),
                (62.0, 16.0),
                (0.0, 60.0),
                (62.0 + 20.0, 60.0),
            ]
        );
        // Given a card's width, the values end at its right edge.
        let node = pairs.layout(
            &mut tree,
            snapshot.renderer(),
            &Limits::new(Size::new(200.0, 0.0), Size::new(200.0, 400.0)),
        );
        assert_eq!(node.size().width, 200.0);
        let right: Vec<f32> = node
            .children()
            .iter()
            .skip(1)
            .step_by(2)
            .map(|c| c.bounds().x + c.bounds().width)
            .collect();
        assert_eq!(right, [200.0, 200.0, 200.0]);
    }
}
