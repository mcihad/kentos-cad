//! Sürükleyerek değer değiştirme: içeriği (ör. özellik adı) yana sürüklemek
//! sayıyı adım adım değiştirir; sürüklerken altında ince bir verniyer
//! ölçeği kayar (teodolitin okuma ölçeği gibi).
//!
//! ```text
//!   Yükseklik ◂──▸           sürükle: değer adım adım değişir
//!   ╵╵╵╵│╵╵╵╵│╵╵╵╵│           ölçek imleçle birlikte kayar
//! ```
//!
//! Shift ince (onda bir), Ctrl kaba (on kat) ayardır. Sürükleme 3 pikseli
//! geçince başlar; daha kısa bir basış içeriğe tıklama gibi geçer.
//! Uygulama başlangıçta `on_start`'ı, sürüklerken başlangıçtan bu yana
//! gidilen yolu (piksel, Shift ve Ctrl'le çarpılmış) `on_scrub`'la,
//! bırakınca `on_end`'i alır.
//!
//! ```ignore
//! Scrub::new(label::body("Yükseklik"), move |pixels| Message::Height(start + pixels * 0.1))
//!     .on_start(Message::HeightStarted)
//!     .on_end(Message::HeightSettled)
//! ```

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard::{self, Modifiers};
use iced::{Background, Color, Element, Event, Length, Rectangle, Renderer, Size, Theme, mouse};

use crate::theme::Tokens;

/// Sürüklemenin başlaması için gereken yol.
const THRESHOLD: f32 = 3.0;
/// Verniyerin çizgi aralığı ve uzun çizgilerin sıklığı.
const TICK: f32 = 4.0;
const MAJOR: i32 = 5;

/// Sürükleyerek değer değiştirme tutamağı.
pub struct Scrub<'a, Message> {
    content: Element<'a, Message>,
    on_scrub: Box<dyn Fn(f32) -> Message + 'a>,
    on_start: Option<Message>,
    on_end: Option<Message>,
}

impl<'a, Message: Clone + 'a> Scrub<'a, Message> {
    /// `on_scrub` başlangıçtan bu yana gidilen yolu alır (piksel; sağa
    /// artı, Shift ve Ctrl'le çarpılmış).
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        on_scrub: impl Fn(f32) -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            on_scrub: Box::new(on_scrub),
            on_start: None,
            on_end: None,
        }
    }

    /// Sürükleme başlarken (ör. başlangıç değerini saklamak için).
    pub fn on_start(mut self, message: Message) -> Self {
        self.on_start = Some(message);
        self
    }

    /// Sürükleme bitince (ör. geri alma adımını kapatmak için).
    pub fn on_end(mut self, message: Message) -> Self {
        self.on_end = Some(message);
        self
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct State {
    /// Basılı fare: başladığı yer, sürükleme başladı mı ve şimdiye dek
    /// gidilen yol (çarpılmış).
    press: Option<Press>,
    modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy)]
struct Press {
    origin: f32,
    last: f32,
    travelled: f32,
    moved: bool,
}

impl State {
    fn scale(&self) -> f32 {
        if self.modifiers.shift() {
            0.1
        } else if self.modifiers.command() {
            10.0
        } else {
            1.0
        }
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Scrub<'a, Message> {
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

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);

        layout::Node::with_children(child.size(), vec![child])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();

        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(position) = cursor.position_over(layout.bounds()) {
                    state.press = Some(Press {
                        origin: position.x,
                        last: position.x,
                        travelled: 0.0,
                        moved: false,
                    });
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let scale = state.scale();

                if let Some(press) = &mut state.press {
                    if !press.moved && (position.x - press.origin).abs() > THRESHOLD {
                        press.moved = true;
                        press.last = position.x;

                        if let Some(message) = &self.on_start {
                            shell.publish(message.clone());
                        }
                    }

                    if press.moved {
                        press.travelled += (position.x - press.last) * scale;
                        press.last = position.x;
                        shell.publish((self.on_scrub)(press.travelled));
                        shell.request_redraw();
                    }

                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(press) = state.press.take() {
                    if press.moved
                        && let Some(message) = &self.on_end
                    {
                        shell.publish(message.clone());
                    }

                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();

        if state.press.is_some_and(|press| press.moved) || cursor.is_over(layout.bounds()) {
            mouse::Interaction::ResizingHorizontally
        } else {
            mouse::Interaction::None
        }
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
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();

        if let Some(press) = state.press.filter(|press| press.moved) {
            let t = Tokens::of(theme);
            vernier(renderer, bounds, press.last - press.origin, t.accent);
        }

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
}

impl<'a, Message: Clone + 'a> From<Scrub<'a, Message>> for Element<'a, Message> {
    fn from(scrub: Scrub<'a, Message>) -> Self {
        Element::new(scrub)
    }
}

/// Verniyer: `bounds`'ın alt kenarında, `offset` kadar kaymış ince ölçek.
/// Kısa çizgiler 4 pikselde bir, her beşincisi uzun; uçlara doğru söner.
pub(crate) fn vernier(renderer: &mut Renderer, bounds: Rectangle, offset: f32, color: Color) {
    let shift = offset.rem_euclid(TICK * MAJOR as f32);
    let first = -(MAJOR as f32 * TICK) + shift;
    let bottom = bounds.y + bounds.height - 1.0;
    let mut x = first;
    let mut index = 0;

    while x < bounds.width {
        if x >= 1.0 {
            let major = index % MAJOR == 0;
            let height = if major { 5.0 } else { 2.5 };
            // Ortada tam, kenarlara doğru sönük.
            let middle = 1.0 - ((x / bounds.width) - 0.5).abs() * 2.0;
            let alpha = (0.25 + 0.75 * middle) * if major { 0.95 } else { 0.6 };

            renderer.fill_quad(
                Quad {
                    bounds: Rectangle {
                        x: (bounds.x + x).round(),
                        y: bottom - height,
                        width: 1.0,
                        height,
                    },
                    ..Quad::default()
                },
                Background::Color(color.scale_alpha(alpha)),
            );
        }

        x += TICK;
        index += 1;
    }
}
