//! Öznitelikler ızgarasının bir satırı: adı ve değeri birlikte (docs/adr/0747).
//!
//! ```text
//!   Yazı yüksekliği   2.500                 m     ← düz satır
//!   Ötelenme          0.800                 m     ← üzerine gelince bütün satır hafifçe aydınlanır
//! ┃ Yazı             [Ada 5|               ]      ← düzenlenirken: solda vurgu çubuğu, satır biraz daha aydınlık
//! ```
//!
//! Satır göz için bir birimdir: üzerine gelinen satırın adı ile değeri birlikte aydınlanır; değeri düzenlenen satır
//! solundaki 2 piksellik vurgu çubuğuyla (katman ağacındaki etkin katmanın ve komut satırının odağının işareti)
//! gösterilir. Düzenleme, satırın içindeki odaklı metin girişinden anlaşılır: hücre ayrıca bir şey bildirmez.

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Id, Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::{Background, Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::theme::Tokens;

/// Vurgu çubuğunun genişliği.
const BAR: f32 = 2.0;

/// Satırın içinde odaklı bir metin girişi var mı.
struct Focused(bool);

impl Operation for Focused {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        self.0 |= state.is_focused();
    }
}

#[derive(Default)]
struct State {
    hovered: bool,
    editing: bool,
}

/// Adı ve değeri olan satır: üzerine gelince aydınlanır, değeri düzenlenirken vurgu çubuğunu taşır.
pub struct SheetRow<'a, Message> {
    content: Element<'a, Message>,
    /// Satırın yalnız bu üst kısmı aydınlanır (altındaki ince çizgi değil).
    height: f32,
}

impl<'a, Message: 'a> SheetRow<'a, Message> {
    /// `content`: satırın kendisi; `height`: aydınlanan kısmın yüksekliği (altındaki çizgi hariç).
    pub fn new(content: impl Into<Element<'a, Message>>, height: f32) -> Self {
        Self {
            content: content.into(),
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
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let content = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        Node::with_children(content.size(), vec![content])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(content) = layout.children().next() {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                content,
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
        let Some(content) = layout.children().next() else {
            return;
        };
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            content,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        let hovered = cursor.is_over(lit(layout.bounds(), self.height));
        // The focus is asked only when the pointer or the keyboard did something: a press starts or ends an edit,
        // a key (Tab, Enter, Esc) moves it.
        let asks = matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(_) | mouse::Event::ButtonReleased(_))
                | Event::Keyboard(_)
        );
        let mut editing = tree.state.downcast_ref::<State>().editing;
        if asks {
            let mut focused = Focused(false);
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                content,
                renderer,
                &mut focused,
            );
            editing = focused.0;
        }
        let state = tree.state.downcast_mut::<State>();
        if hovered != state.hovered || editing != state.editing {
            state.hovered = hovered;
            state.editing = editing;
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
        layout
            .children()
            .next()
            .map_or_else(mouse::Interaction::default, |content| {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    content,
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
        let Some(content) = layout.children().next() else {
            return;
        };
        let state = tree.state.downcast_ref::<State>();
        let t = Tokens::of(theme);
        let row = lit(layout.bounds(), self.height);
        if state.editing || state.hovered {
            renderer.fill_quad(
                Quad {
                    bounds: row,
                    ..Quad::default()
                },
                Background::Color(if state.editing {
                    t.surface_hover
                } else {
                    t.layer(0.035)
                }),
            );
        }
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            content,
            cursor,
            viewport,
        );
        if state.editing {
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle { width: BAR, ..row },
                    ..Quad::default()
                },
                Background::Color(t.accent),
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
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next()?,
            renderer,
            viewport,
            translation,
        )
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
    fn the_row_edited_carries_the_accent_bar_and_a_row_under_the_pointer_is_lit() {
        let mut snapshot = Snapshot::new(Size::new(320.0, 120.0)).expect("çizici kurulamadı");
        let mut update = |_: &mut (), _: ()| {};
        let theme = Theme::Dark;
        let accent = Tokens::of(&theme).accent;
        let image = snapshot.render(view(&()), &theme);
        assert!(
            !near(pixel(&image, 0, 14), accent),
            "no bar before the edit"
        );
        // A press on the value starts the edit: the bar at the row's left.
        snapshot.input(
            &mut (),
            view,
            &mut update,
            Input::Click(Point::new(250.0, 14.0)),
        );
        let image = snapshot.render(view(&()), &theme);
        assert!(near(pixel(&image, 0, 14), accent), "the bar while editing");
        assert!(near(pixel(&image, 1, 14), accent));
        assert!(
            !near(pixel(&image, 0, 43), accent),
            "the other row has none"
        );
        // Esc ends it: no bar.
        snapshot.input(&mut (), view, &mut update, Input::Key(Named::Escape));
        snapshot.input(
            &mut (),
            view,
            &mut update,
            Input::Move(Point::new(100.0, 43.0)),
        );
        let image = snapshot.render(view(&()), &theme);
        assert!(!near(pixel(&image, 0, 14), accent), "no bar after Esc");
        // The second row is lit under the pointer, the first is not.
        assert_ne!(pixel(&image, 60, 43), pixel(&image, 60, 14));
    }
}
