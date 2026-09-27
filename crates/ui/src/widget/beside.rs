//! İmlecin yanında duran kart (bilgi kartı, imleç yanındaki değer alanı):
//! kendi alanından hiç çıkmaz (DESIGN.md §7.4.2). Kural web'in
//! `ui/widgets/placeBeside.ts`'indekiyle aynıdır:
//!
//! - Kart imlecin `offset` kadar sağında ve altında durur (sağ ve alt artı).
//! - Sağ kenarı geçecekse aynı uzaklıkta imlecin soluna geçer. İmlecin
//!   altında duran kart alt kenarı geçecekse imlecin üstüne çıkar.
//! - Sonra alanın her kenarından [`EDGE_MARGIN`] içeride tutulur. Alandan
//!   büyük kartın sol üst köşesi içeride kalır.
//!
//! ```text
//! ┌ alan ──────────────────────────────────┐
//! │  ┼┐                          ┌────────┐ │
//! │   └ ┌────────┐               │  kart  │ │
//! │     │  kart  │               └────────┘┼│ ← sağ kenarda imlecin solunda
//! │     └────────┘                          │
//! └─────────────────────────────────────────┘
//! ```
//!
//! [`beside`] bulunduğu yeri doldurur (ör. çizim alanının üstündeki katman)
//! ve orayı alan sayar. İçerik o alanın iki kenar boşluğu dar genişliğine
//! kadar kendi boyunda yerleşir, sonra kurala göre konur.

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::{Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

/// Kartla alanın kenarı arasında kalan boşluk.
pub const EDGE_MARGIN: f32 = 8.0;

/// `card` boyutundaki kartın sol üst köşesi: `at`'ın yanında, sol üst köşesi
/// (0, 0) olan `area` içinde (bkz. modül; web'in `besidePointer`'ı).
pub fn beside_pointer(at: Point, card: Size, area: Size, offset: Vector) -> Point {
    let mut x = at.x + offset.x;
    if x + card.width > area.width - EDGE_MARGIN {
        x = at.x - offset.x - card.width;
    }
    let mut y = at.y + offset.y;
    if offset.y >= 0.0 && y + card.height > area.height - EDGE_MARGIN {
        y = at.y - offset.y - card.height;
    }
    // Between the edges; the top left when the card is larger than the area.
    let hold = |v: f32, low: f32, high: f32| v.min(high).max(low);
    Point::new(
        hold(x, EDGE_MARGIN, area.width - EDGE_MARGIN - card.width).round(),
        hold(y, EDGE_MARGIN, area.height - EDGE_MARGIN - card.height).round(),
    )
}

/// `content`, `at`'ın yanında (bkz. modül). `at` bileşenin kendi sol üst
/// köşesine göredir.
pub fn beside<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    at: Point,
    offset: Vector,
) -> Element<'a, Message> {
    Element::new(Beside {
        content: content.into(),
        at,
        offset,
    })
}

struct Beside<'a, Message> {
    content: Element<'a, Message>,
    at: Point,
    offset: Vector,
}

impl<Message> Widget<Message, Theme, Renderer> for Beside<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.content.as_widget()]);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let area = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        // Never wider than the area less its margins; as tall as it needs.
        let room = Size::new((area.width - 2.0 * EDGE_MARGIN).max(0.0), f32::INFINITY);
        let card = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, room),
        );
        let at = beside_pointer(self.at, card.size(), area, self.offset);
        Node::with_children(area, vec![card.move_to(at)])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(card) = layout.children().next() {
            self.content
                .as_widget_mut()
                .operate(&mut tree.children[0], card, renderer, operation);
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
        if let Some(card) = layout.children().next() {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                card,
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
        layout
            .children()
            .next()
            .map_or_else(mouse::Interaction::default, |card| {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    card,
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
        if let Some(card) = layout.children().next() {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                card,
                cursor,
                viewport,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size, Vector};

    use super::{EDGE_MARGIN, beside_pointer};

    const AREA: Size = Size::new(800.0, 600.0);
    const CARD: Size = Size::new(280.0, 120.0);
    const BELOW: Vector = Vector::new(18.0, 20.0);

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// The web's cases (placeBeside.test.ts), with its numbers.
    #[test]
    fn right_and_below_the_pointer_else_its_other_side() {
        assert_eq!(
            beside_pointer(at(100.0, 100.0), CARD, AREA, BELOW),
            at(118.0, 120.0)
        );
        // The right edge: left of the pointer, as far.
        assert_eq!(
            beside_pointer(at(700.0, 100.0), CARD, AREA, BELOW),
            at(700.0 - 18.0 - 280.0, 120.0)
        );
        // The bottom edge: above it.
        assert_eq!(
            beside_pointer(at(100.0, 560.0), CARD, AREA, BELOW),
            at(118.0, 560.0 - 20.0 - 120.0)
        );
        // The bottom right corner: both.
        assert_eq!(
            beside_pointer(at(790.0, 590.0), CARD, AREA, BELOW),
            at(790.0 - 18.0 - 280.0, 590.0 - 20.0 - 120.0)
        );
    }

    #[test]
    fn held_inside_when_neither_side_has_room() {
        let narrow = Size::new(300.0, 600.0);
        assert_eq!(
            beside_pointer(at(290.0, 100.0), CARD, narrow, BELOW),
            at(EDGE_MARGIN, 120.0)
        );
        let low = Size::new(800.0, 200.0);
        assert_eq!(
            beside_pointer(at(100.0, 130.0), CARD, low, BELOW),
            at(118.0, EDGE_MARGIN)
        );
        // Larger than the area: its top left corner stays in.
        let big = Size::new(500.0, 400.0);
        let small = Size::new(300.0, 200.0);
        assert_eq!(
            beside_pointer(at(50.0, 50.0), big, small, BELOW),
            at(EDGE_MARGIN, EDGE_MARGIN)
        );
    }

    /// The value field, 58 px above the cursor: never flips up or down; near
    /// the top it slides down, near the right edge it goes left.
    #[test]
    fn a_card_above_the_pointer_slides_down_at_the_top() {
        let above = Vector::new(18.0, -58.0);
        let field = Size::new(190.0, 50.0);
        assert_eq!(
            beside_pointer(at(100.0, 300.0), field, AREA, above),
            at(118.0, 242.0)
        );
        assert_eq!(
            beside_pointer(at(100.0, 20.0), field, AREA, above),
            at(118.0, EDGE_MARGIN)
        );
        assert_eq!(
            beside_pointer(at(780.0, 300.0), field, AREA, above),
            at(780.0 - 18.0 - 190.0, 242.0)
        );
    }
}
