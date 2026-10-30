//! Izgaranın bir satırı (docs/adr/0747, 0804): ad hücresi, sütun çizgisi, değer hücresi ve altında çizgi.
//!
//! ```text
//!   Yazı yüksekliği │ 2.500                 m
//! ──────────────────┼───────────────────────
//! ```
//!
//! Satır göz için bir birimdir: üzerine gelinen satırın adı ile değeri birlikte aydınlanır. Düzenlenen değerin işareti
//! hücrenin kendi kenarıdır ([`Cell`](super::cell::Cell)). Sütunlar tam pikselde durur: çizgiler ve kenarlar hiçbir
//! genişlikte bulanmaz.

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::{
    Background, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse,
};

use crate::theme::Tokens;

/// Ad sütununun genişliği.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum KeyWidth {
    /// Satırın yüzde kaçı.
    Share(u16),
    /// Piksel (kullanıcının sürüklediği).
    Fixed(f32),
}

#[derive(Default)]
struct State {
    hovered: bool,
}

/// Adı ve değeri olan satır: üzerine gelince aydınlanır.
pub(crate) struct SheetRow<'a, Message> {
    key: Element<'a, Message>,
    value: Element<'a, Message>,
    width: KeyWidth,
    /// Satırın yüksekliği; altındaki çizgi hariç.
    height: f32,
}

impl<'a, Message: 'a> SheetRow<'a, Message> {
    pub(crate) fn new(
        key: impl Into<Element<'a, Message>>,
        value: impl Into<Element<'a, Message>>,
        width: KeyWidth,
        height: f32,
    ) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            width,
            height,
        }
    }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for SheetRow<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.key), Tree::new(&self.value)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.key, &self.value]);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(self.height + 1.0))
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let whole = self.height + 1.0;
        let size = limits.resolve(Length::Fill, whole, Size::new(0.0, whole));
        // Whole pixels: the lines and the cells' edges stay sharp at every width.
        let width = size.width.floor();
        let key_width = match self.width {
            KeyWidth::Share(share) => (width * f32::from(share) / 100.0).floor(),
            KeyWidth::Fixed(fixed) => fixed.round(),
        }
        .clamp(0.0, width);
        let value_width = (width - key_width - 1.0).max(0.0);
        let cell =
            |w: f32| layout::Limits::new(Size::new(w, self.height), Size::new(w, self.height));
        let key =
            self.key
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &cell(key_width));
        let value =
            self.value
                .as_widget_mut()
                .layout(&mut tree.children[1], renderer, &cell(value_width));
        Node::with_children(
            Size::new(size.width, whole),
            vec![key, value.move_to(Point::new(key_width + 1.0, 0.0))],
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let mut parts = layout.children();
        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
            self.key
                .as_widget_mut()
                .operate(&mut tree.children[0], key, renderer, operation);
            self.value
                .as_widget_mut()
                .operate(&mut tree.children[1], value, renderer, operation);
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
        let mut parts = layout.children();
        let (Some(key), Some(value)) = (parts.next(), parts.next()) else {
            return;
        };
        self.key.as_widget_mut().update(
            &mut tree.children[0],
            event,
            key,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        self.value.as_widget_mut().update(
            &mut tree.children[1],
            event,
            value,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        let hovered = cursor.is_over(lit(layout.bounds(), self.height));
        let state = tree.state.downcast_mut::<State>();
        if hovered != state.hovered {
            state.hovered = hovered;
            shell.request_redraw();
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
        let mut parts = layout.children();
        let (Some(key), Some(value)) = (parts.next(), parts.next()) else {
            return mouse::Interaction::default();
        };
        self.key
            .as_widget()
            .mouse_interaction(&tree.children[0], key, cursor, viewport, renderer)
            .max(self.value.as_widget().mouse_interaction(
                &tree.children[1],
                value,
                cursor,
                viewport,
                renderer,
            ))
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
        let mut parts = layout.children();
        let (Some(key), Some(value)) = (parts.next(), parts.next()) else {
            return;
        };
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let row = lit(bounds, self.height);
        if tree.state.downcast_ref::<State>().hovered {
            renderer.fill_quad(
                Quad {
                    bounds: row,
                    ..Quad::default()
                },
                Background::Color(t.layer(0.035)),
            );
        }
        self.key.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            key,
            cursor,
            viewport,
        );
        self.value.as_widget().draw(
            &tree.children[1],
            renderer,
            theme,
            style,
            value,
            cursor,
            viewport,
        );
        let rule = Background::Color(t.rule());
        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x: value.bounds().x - 1.0,
                    width: 1.0,
                    ..row
                },
                ..Quad::default()
            },
            rule,
        );
        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    y: row.y + row.height,
                    height: 1.0,
                    ..bounds
                },
                ..Quad::default()
            },
            rule,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let mut parts = layout.children();
        let (key, value) = (parts.next()?, parts.next()?);
        let (first, second) = tree.children.split_at_mut(1);
        let from_value = self.value.as_widget_mut().overlay(
            &mut second[0],
            value,
            renderer,
            viewport,
            translation,
        );
        if from_value.is_some() {
            return from_value;
        }
        self.key
            .as_widget_mut()
            .overlay(&mut first[0], key, renderer, viewport, translation)
    }
}

/// The lit part of a row: its top `height` (the line under it is not lit).
fn lit(bounds: Rectangle, height: f32) -> Rectangle {
    Rectangle {
        height: height.min(bounds.height),
        ..bounds
    }
}

impl<'a, Message: 'a> From<SheetRow<'a, Message>> for Element<'a, Message> {
    fn from(row: SheetRow<'a, Message>) -> Self {
        Element::new(row)
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use iced::keyboard::key::Named;
    use iced::{Element, Point, Size, Theme};

    use crate::snapshot::{Image, Input, Snapshot};
    use crate::theme::Tokens;
    use crate::widget::EditCell;
    use crate::widget::property_grid::{PropertySheet, value};

    fn view(_: &()) -> Element<'_, ()> {
        PropertySheet::new()
            .row("Yazı", EditCell::new("Ada 5", |_| ()))
            .row("Yükseklik", value("2.500", true, Some("m".into())))
            .into()
    }

    fn pixel(image: &Image, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * image.width + x) * 4) as usize;
        [image.rgba[i], image.rgba[i + 1], image.rgba[i + 2]]
    }

    fn near(a: [u8; 3], b: iced::Color) -> bool {
        let b = [b.r, b.g, b.b].map(|c| (c * 255.0).round() as i32);
        a.iter().zip(b).all(|(a, b)| (i32::from(*a) - b).abs() <= 3)
    }

    #[test]
    fn the_cell_edited_has_the_accent_edge_and_a_row_under_the_pointer_is_lit() {
        let mut snapshot = Snapshot::new(Size::new(320.0, 120.0)).expect("çizici kurulamadı");
        let mut update = |_: &mut (), _: ()| {};
        let theme = Theme::Dark;
        let t = Tokens::of(&theme);
        // Rows of 28 px and a 1 px line; the values from 42 % of the width and a 1 px line.
        let middle_x = 250;
        let image = snapshot.render(view(&()), &theme);
        assert!(
            near(pixel(&image, middle_x, 14), t.cell()),
            "a writable cell's ground at rest"
        );
        assert!(
            !near(pixel(&image, middle_x, 43), t.cell()),
            "a read-only one has none"
        );
        assert!(
            !(0..320).any(|x| near(pixel(&image, x, 14), t.accent)),
            "no edge before the edit"
        );
        // A press on the value starts the edit: the field's ground from line to line, the accent on its edge.
        snapshot.input(
            &mut (),
            view,
            &mut update,
            Input::Click(Point::new(250.0, 14.0)),
        );
        let image = snapshot.render(view(&()), &theme);
        let edge_x = (120..150)
            .find(|&x| near(pixel(&image, x, 14), t.accent))
            .expect("the edge while editing");
        assert!(
            (130..=138).contains(&edge_x),
            "at the column's line: {edge_x}"
        );
        assert!(
            near(pixel(&image, 319, 14), t.accent),
            "to the cell's right end"
        );
        assert!(near(pixel(&image, middle_x, 0), t.accent), "from its top");
        assert!(near(pixel(&image, middle_x, 27), t.accent), "to its bottom");
        assert!(
            near(pixel(&image, 300, 14), t.field),
            "on the field's ground"
        );
        assert!(
            !near(pixel(&image, edge_x, 43), t.accent),
            "the other row has none"
        );
        // Esc ends it: no edge.
        snapshot.input(&mut (), view, &mut update, Input::Key(Named::Escape));
        snapshot.input(
            &mut (),
            view,
            &mut update,
            Input::Move(Point::new(100.0, 43.0)),
        );
        let image = snapshot.render(view(&()), &theme);
        assert!(
            !near(pixel(&image, edge_x, 14), t.accent),
            "no edge after Esc"
        );
        // The second row is lit under the pointer, the first is not.
        assert_ne!(pixel(&image, 60, 43), pixel(&image, 60, 14));
    }

    #[test]
    fn a_text_stays_where_it_is_when_its_cell_is_edited() {
        let mut snapshot = Snapshot::new(Size::new(320.0, 120.0)).expect("çizici kurulamadı");
        let mut update = |_: &mut (), _: ()| {};
        let theme = Theme::Dark;
        let t = Tokens::of(&theme);
        // Where the first letter's ink starts, on the row's middle lines.
        let first_ink = |image: &Image, ground: iced::Color| {
            (136..320).find(|&x| (8..20).any(|y| !near(pixel(image, x, y), ground)))
        };
        let image = snapshot.render(view(&()), &theme);
        let at_rest = first_ink(&image, t.cell()).expect("the value's letters");
        snapshot.input(
            &mut (),
            view,
            &mut update,
            Input::Click(Point::new(250.0, 14.0)),
        );
        let image = snapshot.render(view(&()), &theme);
        let editing = first_ink(&image, t.field).expect("the value's letters");
        assert_eq!(at_rest, editing);
    }
}
