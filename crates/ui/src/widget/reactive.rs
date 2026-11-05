//! Görünen her değişiklik çizilir (docs/adr/0976).
//!
//! Iced 0.14 pencereyi yalnız bir bileşen kare istediğinde (`Shell::request_redraw`) ya da uygulamaya bir mesaj
//! gittiğinde çizer: tepkisel çizim. Yazılanı kendi durumunda tutan bileşen (sayı girişi, düzenlenen hücre, çip girişi)
//! tuş için mesaj göndermez. Bir tuşta yalnız yerleşimi geçersiz kılarsa yerleşim kurulur ama kare çizilmez; harf,
//! başka bir şeyin istediği ilk karede görünür: imlecin sonraki yanıp sönmesi, imlecin kıpırdaması, bir zamanlayıcı.
//! Sahibin ölçtüğü gecikme bir buçuk saniyeye çıkıyordu (Proje ayarları › Birim ve ölçek, Nesne denetçisi, Poz
//! metraj ızgarası).
//!
//! - [`changed`]: kendi durumu görüneni değiştiren bileşenin çağırdığı, yerleşimi kurar ve kareyi ister.
//! - [`presented`]: uygulamanın kökü bununla sarılır; bir olayda herhangi bir bileşenin geçersiz kıldığı yerleşim bir
//!   sonraki karede çizilir. Kareyi istemeyi unutan bileşen de böylece hemen görülür. Çizim olayının kendisinde kare
//!   istenmez: o olayda kurulan yerleşim zaten o karede çizilir, istenseydi kareler kendini besleyen bir döngüye
//!   dönerdi. Kökün kendi katmanları (açılan liste, menü, ipucu) iced'in üst katmanından geçer, bu sarmalayıcıdan
//!   değil: onlar kareyi kendileri ister.

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse, window};

/// Bileşenin kendi durumu (yazılan metin, başlayan ya da biten düzenleme, kaydırılan sekmeler) görüneni değiştirdi:
/// yerleşim yeniden kurulur ve pencere bir sonraki karede çizilir. Yalnız `invalidate_layout` kurulur ama çizilmez.
pub(crate) fn changed<Message>(shell: &mut Shell<'_, Message>) {
    shell.invalidate_layout();
    shell.request_redraw();
}

/// Uygulamanın kökü: bir olayda geçersiz kılınan yerleşim bir sonraki karede çizilir ([modül](self)).
pub struct Presented<'a, Message> {
    content: Element<'a, Message>,
}

/// `content`'i, bir olayda geçersiz kılınan yerleşimi hemen çizdirerek sarar ([modül](self)).
pub fn presented<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Presented<'a, Message> {
    Presented { content: content.into() }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Presented<'a, Message> {
    // Saydam: ağaç, durum ve yerleşim içtekinin; sarmak arayüzün ağacını değiştirmez.
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(tree, layout, renderer, operation);
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
        self.content
            .as_widget_mut()
            .update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
        if shell.is_layout_invalid() && !matches!(event, Event::Window(window::Event::RedrawRequested(_))) {
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
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
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
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message: 'a> From<Presented<'a, Message>> for Element<'a, Message> {
    fn from(presented: Presented<'a, Message>) -> Self {
        Element::new(presented)
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use iced::advanced::layout::{self, Layout, Node};
    use iced::advanced::widget::{Tree, Widget, tree};
    use iced::advanced::{Clipboard, Shell, renderer};
    use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, mouse, window};

    use super::presented;
    use crate::snapshot::{Input, Snapshot};

    /// A widget that keeps what is typed in its own state and, as the kit's fields did, only invalidates the layout:
    /// on a key, or on every event (`always`).
    struct Keeps {
        always: bool,
    }

    impl Widget<(), Theme, Renderer> for Keeps {
        fn tag(&self) -> tree::Tag {
            tree::Tag::of::<String>()
        }

        fn state(&self) -> tree::State {
            tree::State::new(String::new())
        }

        fn size(&self) -> Size<Length> {
            Size::new(Length::Fixed(100.0), Length::Fixed(20.0))
        }

        fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, _limits: &layout::Limits) -> Node {
            Node::new(Size::new(100.0, 20.0))
        }

        fn update(
            &mut self,
            tree: &mut Tree,
            event: &Event,
            _layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _renderer: &Renderer,
            _clipboard: &mut dyn Clipboard,
            shell: &mut Shell<'_, ()>,
            _viewport: &Rectangle,
        ) {
            if let Event::Keyboard(iced::keyboard::Event::KeyPressed { text: Some(t), .. }) = event {
                tree.state.downcast_mut::<String>().push_str(t);
                shell.invalidate_layout();
            } else if self.always {
                shell.invalidate_layout();
            }
        }

        fn draw(
            &self,
            _tree: &Tree,
            _renderer: &mut Renderer,
            _theme: &Theme,
            _style: &renderer::Style,
            _layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) {
        }
    }

    fn bare(_: &()) -> Element<'_, ()> {
        Element::new(Keeps { always: false })
    }

    fn wrapped(_: &()) -> Element<'_, ()> {
        presented(Element::new(Keeps { always: false })).into()
    }

    fn always(_: &()) -> Element<'_, ()> {
        presented(Element::new(Keeps { always: true })).into()
    }

    fn keys_drawn(view: fn(&()) -> Element<'_, ()>) -> Vec<bool> {
        let mut snapshot = Snapshot::new(Size::new(200.0, 40.0)).expect("a renderer");
        snapshot
            .drawn(&mut (), view, &mut |_, _| {}, Input::Type("12".into()))
            .into_iter()
            .filter(|(e, _)| matches!(e, Event::Keyboard(iced::keyboard::Event::KeyPressed { .. })))
            .map(|(_, drawn)| drawn)
            .collect()
    }

    /// Iced 0.14 draws a layout invalidated alone only at another frame: the typed key waits. Under `presented` it is
    /// drawn at once (docs/adr/0976).
    #[test]
    fn a_layout_invalidated_on_a_key_is_drawn_at_once_under_presented() {
        assert_eq!(keys_drawn(bare), [false, false], "without it the key is not drawn: the fault itself");
        assert_eq!(keys_drawn(wrapped), [true, true]);
    }

    /// A frame's own event asks for no other frame: a widget laying out again on every event would otherwise draw the
    /// window in a loop.
    #[test]
    fn a_frame_asks_for_no_other_frame() {
        let mut snapshot = Snapshot::new(Size::new(200.0, 40.0)).expect("a renderer");
        let frame = Event::Window(window::Event::RedrawRequested(iced::time::Instant::now()));
        let moved = Event::Mouse(mouse::Event::CursorMoved { position: iced::Point::new(5.0, 5.0) });
        let drawn: Vec<bool> = snapshot
            .drawn_events(&mut (), always, &mut |_, _| {}, vec![frame, moved])
            .into_iter()
            .map(|(_, drawn)| drawn)
            .collect();
        assert_eq!(drawn, [false, true]);
    }
}
