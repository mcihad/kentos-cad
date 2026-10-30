//! Odağın bir parçadan çıkışı: içindeki herhangi bir odaklanabilir öğe (metin kutusu, seçim kutusu, anahtar)
//! odağı aldığında `true`, parçanın dışına bıraktığında `false` ile haber verir. Form satırı “alandan
//! çıkınca” denetimini bununla sorar (docs/adr/0760 §2).
//!
//! ```text
//!  ┌ Kat sayısı ─────────┐        Tab        ┌ Kat sayısı ─────────┐
//!  │ [ 25▏           ]   │  ──────────────▶  │ [ 25            ]   │  on_focus(false): çıkıldı
//!  └─────────────────────┘                   └─────────────────────┘
//! ```
//!
//! iced'in metin girişi odağı kendi içinde tutar ve bırakışını bildirmez; burada her fare basışından ve
//! tuştan sonra parçanın odaklanabilirleri sayılır (yalnız parçanın kendi ağacı), değişince mesaj gider.
//! Tab'ın odağı taşıması bir işlemdir, olay değildir: çıkış tuşun bırakılışında görülür.
//!
//! ```ignore
//! focus_watch(row_of_the_field).on_focus(move |on| Event::Focus(id.clone(), on))
//! ```

use std::rc::Rc;

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::widget::operation::{Focusable, Operation};
use iced::advanced::widget::{Id, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

/// İçindeki öğelerin odağını izleyen sarmalayıcı.
pub struct FocusWatch<'a, Message> {
    content: Element<'a, Message>,
    on_focus: Option<Rc<dyn Fn(bool) -> Message + 'a>>,
}

/// `content`'in odağını izler; mesajı [`FocusWatch::on_focus`] verir.
pub fn focus_watch<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> FocusWatch<'a, Message> {
    FocusWatch {
        content: content.into(),
        on_focus: None,
    }
}

impl<'a, Message: 'a> FocusWatch<'a, Message> {
    /// Parçanın içine odak girince `true`, dışarı çıkınca `false` ile gönderilen mesaj.
    pub fn on_focus(mut self, on_focus: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_focus = Some(Rc::new(on_focus));
        self
    }
}

/// Son görülen odak: içindeki bir öğe odakta mıydı.
#[derive(Default)]
struct State {
    focused: bool,
}

/// Bir odaklanabilir öğe odakta mı: parçanın ağacında sorulur.
struct Focused(bool);

impl Operation for Focused {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        self.0 |= state.is_focused();
    }
}

/// Odağın değişebildiği olay: fare basışı ya da bırakışı, tuş, dokunma.
fn may_move_focus(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonPressed(_) | mouse::Event::ButtonReleased(_))
            | Event::Keyboard(_)
            | Event::Touch(_)
    )
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for FocusWatch<'a, Message> {
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
        let node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        Node::with_children(node.size(), vec![node])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(child) = layout.children().next() {
            self.content
                .as_widget_mut()
                .operate(&mut tree.children[0], child, renderer, operation);
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
        let Some(child) = layout.children().next() else {
            return;
        };
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            child,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        let Some(on_focus) = &self.on_focus else {
            return;
        };
        if !may_move_focus(event) {
            return;
        }
        let mut focused = Focused(false);
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], child, renderer, &mut focused);
        let state = tree.state.downcast_mut::<State>();
        if state.focused != focused.0 {
            state.focused = focused.0;
            shell.publish(on_focus(focused.0));
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
            .map_or(mouse::Interaction::default(), |child| {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
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
        if let Some(child) = layout.children().next() {
            self.content.as_widget().draw(
                &tree.children[0],
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
        let child = layout.children().next()?;
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            child,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<FocusWatch<'a, Message>> for Element<'a, Message> {
    fn from(watch: FocusWatch<'a, Message>) -> Self {
        Element::new(watch)
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use super::*;

    #[test]
    fn only_presses_and_keys_may_move_the_focus() {
        use iced::keyboard;
        assert!(may_move_focus(&Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left
        ))));
        assert!(may_move_focus(&Event::Keyboard(
            keyboard::Event::ModifiersChanged(keyboard::Modifiers::empty())
        )));
        assert!(!may_move_focus(&Event::Mouse(mouse::Event::CursorMoved {
            position: iced::Point::ORIGIN
        })));
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Focus(&'static str, bool),
        Typed,
    }

    fn two_fields<'a>() -> Element<'a, Msg> {
        let field = |name: &'static str| {
            focus_watch(
                iced::widget::text_input("", "")
                    .on_input(|_| Msg::Typed)
                    .width(Length::Fixed(100.0)),
            )
            .on_focus(move |on| Msg::Focus(name, on))
        };
        iced::widget::row![field("a"), field("b")].into()
    }

    fn click(snapshot: &mut crate::snapshot::Snapshot, x: f32) -> Vec<Msg> {
        let mut out = Vec::new();
        for event in [
            Event::Mouse(mouse::Event::CursorMoved {
                position: iced::Point::new(x, 10.0),
            }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ] {
            out.extend(snapshot.deliver(two_fields(), &event).0);
        }
        out
    }

    #[test]
    fn it_says_when_the_focus_enters_and_leaves_its_part() {
        let mut snapshot =
            crate::snapshot::Snapshot::new(Size::new(300.0, 60.0)).expect("a renderer");
        assert_eq!(click(&mut snapshot, 50.0), [Msg::Focus("a", true)]);
        assert_eq!(click(&mut snapshot, 50.0), [], "a press inside keeps it");
        assert_eq!(
            click(&mut snapshot, 150.0),
            [Msg::Focus("a", false), Msg::Focus("b", true)]
        );
        assert_eq!(click(&mut snapshot, 280.0), [Msg::Focus("b", false)]);
    }

    #[test]
    fn a_tab_s_move_is_seen_on_the_next_key() {
        let mut snapshot =
            crate::snapshot::Snapshot::new(Size::new(300.0, 60.0)).expect("a renderer");
        assert_eq!(click(&mut snapshot, 50.0), [Msg::Focus("a", true)]);
        snapshot.operate(
            two_fields(),
            Box::new(crate::focus::step_operation(false, false)),
        );
        let release = Event::Keyboard(iced::keyboard::Event::KeyReleased {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab),
            modified_key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab),
            physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::Tab),
            location: iced::keyboard::Location::Standard,
            modifiers: iced::keyboard::Modifiers::empty(),
        });
        assert_eq!(
            snapshot.deliver(two_fields(), &release).0,
            [Msg::Focus("a", false), Msg::Focus("b", true)]
        );
    }
}
