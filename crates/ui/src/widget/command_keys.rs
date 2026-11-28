//! ⌘ ile basılan tuşun metni (gis-usage-report GIS-025, docs/adr/1086): macOS'ta ⌘S, ⌘K gibi bir kısayolun harfi,
//! odaktaki metin kutusuna yazılmaz. iced'in metin girişi Komut tuşuna bakmadan olayın metnini ekler; bu sarmalayıcı
//! ⌘ basılıyken gelen tuş olayını içindekilere metinsiz verir. Kısayollar uygulamanın aboneliğinde olayın aslını
//! görür; Option ile yazılan harfler (Türkçe Q'da @) olduğu gibi kalır.
//!
//! ```ignore
//! command_keys(app_view)
//! ```

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::widget::operation::Operation;
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, keyboard, mouse};

/// İçindekilere ⌘'li tuşun metnini vermeyen sarmalayıcı.
pub struct CommandKeys<'a, Message> {
    content: Element<'a, Message>,
}

/// `content`'i sarar: ⌘'li tuş metni içine yazılmaz.
pub fn command_keys<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> CommandKeys<'a, Message> {
    CommandKeys {
        content: content.into(),
    }
}

/// Komut tuşu: macOS'ta ⌘ (logo); başka yerde Ctrl'li harf zaten denetim karakteridir, metin kutusu yazmaz.
fn command_held(modifiers: keyboard::Modifiers) -> bool {
    cfg!(target_os = "macos") && modifiers.logo()
}

/// ⌘ basılıyken gelen tuşun metinsiz kopyası; başka olay için hiçbiri.
fn without_command_text(event: &Event) -> Option<Event> {
    let Event::Keyboard(keyboard::Event::KeyPressed {
        modifiers,
        text: Some(_),
        ..
    }) = event
    else {
        return None;
    };
    if !command_held(*modifiers) {
        return None;
    }
    let mut out = event.clone();
    if let Event::Keyboard(keyboard::Event::KeyPressed { text, .. }) = &mut out {
        *text = None;
    }
    Some(out)
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for CommandKeys<'a, Message> {
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
        let stripped = without_command_text(event);
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            stripped.as_ref().unwrap_or(event),
            child,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
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

impl<'a, Message: 'a> From<CommandKeys<'a, Message>> for Element<'a, Message> {
    fn from(keys: CommandKeys<'a, Message>) -> Self {
        Element::new(keys)
    }
}

#[cfg(all(test, feature = "snapshot", target_os = "macos"))]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Typed(String),
    }

    fn field<'a>() -> Element<'a, Msg> {
        command_keys(
            iced::widget::text_input("", "18")
                .on_input(Msg::Typed)
                .width(Length::Fixed(200.0)),
        )
        .into()
    }

    fn key(c: &str, modifiers: keyboard::Modifiers) -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character(c.into()),
            modified_key: keyboard::Key::Character(c.into()),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyS),
            location: keyboard::Location::Standard,
            modifiers,
            text: Some(c.into()),
            repeat: false,
        })
    }

    #[test]
    fn a_command_key_types_nothing_and_a_plain_s_types() {
        let mut snapshot =
            crate::snapshot::Snapshot::new(Size::new(300.0, 40.0)).expect("a renderer");
        for event in [
            Event::Mouse(mouse::Event::CursorMoved {
                position: iced::Point::new(50.0, 10.0),
            }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ] {
            snapshot.deliver(field(), &event);
        }
        let (typed, _) = snapshot.deliver(field(), &key("s", keyboard::Modifiers::LOGO));
        assert_eq!(typed, [], "⌘S types no s");
        let (typed, _) = snapshot.deliver(field(), &key("s", keyboard::Modifiers::empty()));
        assert_eq!(typed, [Msg::Typed("18s".into())], "a plain s types");
    }
}
