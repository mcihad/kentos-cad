//! Bilgi kartının içi (DESIGN.md §7.4.2): başlık ve sağında bir etiket (ör.
//! nesnenin katmanı), altında ince bir çizgi ve satırlar (ör.
//! [`Pairs`](super::Pairs)). Başlıkla etiket bir satıra sığmazsa etiket
//! başlığın altına iner ve orada kayar (web'in `flex-wrap`'ı).
//!
//! ```text
//! ┌─────────────────────────────┐   ┌─────────────────────────────┐
//! │ Daire              ■ Bina   │   │ Kapalı alan                 │
//! │─────────────────────────────│   │ ■ Ortak gösterimler · Koru- │
//! │ Çevre            20.420 m   │   │   nacak alanlar · …         │
//! │ Yarıçap           3.250 m   │   │─────────────────────────────│
//! └─────────────────────────────┘   │ Alan             800.00 m²  │
//!                                   └─────────────────────────────┘
//! ```
//!
//! Kart içeriğine göre genişler: en az [`InfoCard::min_width`], en çok
//! kendisine verilen genişlik (kabın `max_width`'i); tek satıra sığmayan
//! başlık ve etiket o genişliğe çıkarır. Satırlar kartın genişliğine yayılır.
//!
//! ```ignore
//! container(
//!     InfoCard::new(label::strong("Kapalı alan"))
//!         .tag(row![swatch(color), label::caption(layer)].spacing(5))
//!         .body(Pairs::new().push(label::caption("Alan"), label::mono("800.00 m²")))
//!         .min_width(140.0),
//! )
//! .max_width(280.0)
//! ```

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::{Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

use crate::widget::horizontal_divider;

/// Başlıkla aynı satırdaki etiketin başlıktan uzaklığı.
const GAP: f32 = 12.0;
/// Başlığın altına inen etiketin başlıktan uzaklığı.
const WRAP_GAP: f32 = 2.0;
/// Çizginin üstünde ve altında kalan boşluk.
const RULE_SPACE: f32 = 6.0;

/// Bilgi kartının içi (bkz. modül).
pub struct InfoCard<'a, Message> {
    title: Element<'a, Message>,
    tag: Option<Element<'a, Message>>,
    body: Option<Element<'a, Message>>,
    min_width: f32,
}

impl<'a, Message: 'a> InfoCard<'a, Message> {
    pub fn new(title: impl Into<Element<'a, Message>>) -> Self {
        Self {
            title: title.into(),
            tag: None,
            body: None,
            min_width: 0.0,
        }
    }

    /// Başlığın sağında, sığmazsa altında duran etiket.
    #[must_use]
    pub fn tag(mut self, tag: impl Into<Element<'a, Message>>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Çizginin altındaki satırlar.
    #[must_use]
    pub fn body(mut self, body: impl Into<Element<'a, Message>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Kartın içinin en az genişliği.
    #[must_use]
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }
}

impl<'a, Message: 'a> From<InfoCard<'a, Message>> for Element<'a, Message> {
    fn from(card: InfoCard<'a, Message>) -> Self {
        let tag = card.tag.is_some();
        let body = card.body.is_some();
        let mut cells = vec![card.title];
        cells.extend(card.tag);
        if let Some(rows) = card.body {
            cells.push(horizontal_divider().into());
            cells.push(rows);
        }
        Element::new(Card {
            cells,
            tag,
            body,
            min_width: card.min_width,
        })
    }
}

/// The card's cells in order: the title, the tag, the line and the body.
struct Card<'a, Message> {
    cells: Vec<Element<'a, Message>>,
    tag: bool,
    body: bool,
    min_width: f32,
}

impl<Message> Card<'_, Message> {
    fn lay(
        &mut self,
        index: usize,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> Node {
        self.cells[index]
            .as_widget_mut()
            .layout(&mut tree.children[index], renderer, limits)
    }
}

/// Up to `width` wide, as tall as it needs.
fn open(width: f32) -> layout::Limits {
    layout::Limits::new(Size::ZERO, Size::new(width, f32::INFINITY))
}

/// Exactly `width` wide.
fn exact(width: f32) -> layout::Limits {
    layout::Limits::new(Size::new(width, 0.0), Size::new(width, f32::INFINITY))
}

impl<Message> Widget<Message, Theme, Renderer> for Card<'_, Message> {
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
        let max = limits.max().width;
        let tag = self.tag.then_some(1);
        let body = self.body.then(|| self.cells.len() - 1);
        let title = self.lay(0, tree, renderer, &open(max));
        let tag_node = tag.map(|i| self.lay(i, tree, renderer, &open(max)));
        let body_width = body.map_or(0.0, |i| {
            self.lay(i, tree, renderer, &open(max)).size().width
        });
        // As wide as the head on one line or the body, within the limits.
        let head = title.size().width + tag_node.as_ref().map_or(0.0, |t| GAP + t.size().width);
        let width = head
            .max(body_width)
            .max(self.min_width)
            .max(limits.min().width)
            .min(max);
        let mut nodes = Vec::with_capacity(self.cells.len());
        let head_height = match (tag, tag_node) {
            // On one line: their bottoms level (near enough the web's
            // baselines), the tag at the right edge.
            (Some(_), Some(t)) if head <= width => {
                let height = title.size().height.max(t.size().height);
                let (title_size, t_size) = (title.size(), t.size());
                nodes.push(title.move_to(Point::new(0.0, height - title_size.height)));
                nodes.push(t.move_to(Point::new(width - t_size.width, height - t_size.height)));
                height
            }
            // Under the title, wrapping at the card's width.
            (Some(i), Some(_)) => {
                let t = self.lay(i, tree, renderer, &open(width));
                let below = title.size().height + WRAP_GAP;
                let height = below + t.size().height;
                nodes.push(title);
                nodes.push(t.move_to(Point::new(0.0, below)));
                height
            }
            _ => {
                let height = title.size().height;
                nodes.push(title);
                height
            }
        };
        let mut height = head_height;
        if let Some(i) = body {
            let rule = self.lay(i - 1, tree, renderer, &exact(width));
            let rule_height = rule.size().height;
            nodes.push(rule.move_to(Point::new(0.0, head_height + RULE_SPACE)));
            let rows = self.lay(i, tree, renderer, &exact(width));
            let top = head_height + RULE_SPACE * 2.0 + rule_height;
            height = top + rows.size().height;
            nodes.push(rows.move_to(Point::new(0.0, top)));
        }
        Node::with_children(Size::new(width, height), nodes)
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
    use iced::advanced::layout::{Limits, Node};
    use iced::advanced::widget::Tree;
    use iced::widget::space;
    use iced::{Element, Size};

    use super::InfoCard;
    use crate::snapshot::Snapshot;

    /// Boxes of known sizes stand for the texts.
    fn laid(card: InfoCard<'_, ()>, max: f32) -> Node {
        let mut card: Element<'_, ()> = card.into();
        let mut tree = Tree::new(card.as_widget());
        let snapshot = Snapshot::software(Size::new(400.0, 300.0)).expect("a renderer");
        card.as_widget_mut().layout(
            &mut tree,
            snapshot.renderer(),
            &Limits::new(Size::ZERO, Size::new(max, 400.0)),
        )
    }

    fn origins(node: &Node) -> Vec<(f32, f32)> {
        node.children()
            .iter()
            .map(|c| (c.bounds().x, c.bounds().y))
            .collect()
    }

    #[test]
    fn the_tag_goes_right_of_the_title_when_both_fit_and_under_it_when_not() {
        let rows = || space().width(150).height(40);
        // Title 60 + 12 + tag 50 fit: one line, the tag at the right edge of the rows' width.
        let node = laid(
            InfoCard::new(space().width(60).height(16))
                .tag(space().width(50).height(14))
                .body(rows())
                .min_width(140.0),
            260.0,
        );
        assert_eq!(node.size().width, 150.0);
        assert_eq!(
            origins(&node),
            [(0.0, 0.0), (100.0, 2.0), (0.0, 22.0), (0.0, 29.0)]
        );
        assert_eq!(node.size().height, 16.0 + 6.0 + 1.0 + 6.0 + 40.0);
        // A tag too long for the line goes under the title, the card at its most.
        let node = laid(
            InfoCard::new(space().width(60).height(16))
                .tag(space().width(230).height(14))
                .body(rows()),
            260.0,
        );
        assert_eq!(node.size().width, 260.0);
        assert_eq!(origins(&node)[..2], [(0.0, 0.0), (0.0, 18.0)]);
        // No rows: the least width holds.
        let node = laid(
            InfoCard::new(space().width(40).height(16)).min_width(140.0),
            260.0,
        );
        assert_eq!(node.size(), Size::new(140.0, 16.0));
    }
}
