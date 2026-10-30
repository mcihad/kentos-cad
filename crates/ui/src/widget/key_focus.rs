//! Klavye odağı: düğme tabanlı form öğelerini (onay kutusu, parçalı seçim, radyo) odaklanabilir yapan
//! sarmalayıcı ve odak alan öğelerin ortak parçaları (odak durumu, odak halesi, tuşun okunması).
//!
//! ```text
//!  ╭──────────╮
//!  │ ☑ Görünür│   ← odakta: vurgu rengi hale
//!  ╰──────────╯
//! ```
//!
//! Tab ile odak alır ([`crate::focus::next`]); odaktayken Boşluk ya da Enter `on_activate`'i, oklar
//! `on_step`'i (sol ve yukarı −1, sağ ve aşağı +1) gönderir. Fareyle basınca da odak alır, başka yere
//! basınca bırakır; böylece fare ile klavye aynı öğede buluşur.
//!
//! ```ignore
//! key_focus(check_box(state, Some(toggle.clone()))).on_activate(toggle)
//! ```

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::keyboard::{self, Modifiers, key::Named};
use iced::{Border, Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::theme::{Tokens, shape};

/// Odak halesinin kalınlığı .
pub(crate) const HALO: f32 = 3.0;

/// Iced text_input caches ModifiersChanged independently of KeyPressed.
/// A newly built/focused field may have missed that notification (or its
/// release); synchronize from this key's actual modifiers before forwarding it.
pub(crate) fn text_modifiers(event: &Event) -> Option<Event> {
    match event {
        Event::Keyboard(keyboard::Event::KeyPressed { modifiers, .. }) => Some(Event::Keyboard(
            keyboard::Event::ModifiersChanged(*modifiers),
        )),
        _ => None,
    }
}

/// Bir öğenin klavye odağı: iced'in odak işlemleri onu bu bayrakla açar ve kapatır.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Flag(pub bool);

impl Focusable for Flag {
    fn is_focused(&self) -> bool {
        self.0
    }

    fn focus(&mut self) {
        self.0 = true;
    }

    fn unfocus(&mut self) {
        self.0 = false;
    }
}

/// Odaktaki öğenin çevresindeki hale: vurgu rengi, hafif saydam, köşeleri izleyerek.
pub(crate) fn halo(renderer: &mut Renderer, theme: &Theme, bounds: Rectangle, radius: f32) {
    let t = Tokens::of(theme);
    renderer.fill_quad(
        Quad {
            bounds: bounds.expand(HALO),
            border: Border {
                color: t.accent.scale_alpha(if t.is_dark { 0.32 } else { 0.24 }),
                width: HALO,
                radius: (radius + HALO).into(),
            },
            ..Quad::default()
        },
        iced::Color::TRANSPARENT,
    );
}

/// Basılan adlı tuş ve değiştiricileri; tekrar eden basış da sayılır.
pub(crate) fn pressed(event: &Event) -> Option<(Named, Modifiers)> {
    match event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(named),
            modifiers,
            ..
        }) => Some((*named, *modifiers)),
        _ => None,
    }
}

/// A character shortcut belongs to the host, except field editing and AltGr text.
pub(crate) fn host_character_chord(event: &Event) -> bool {
    let Event::Keyboard(keyboard::Event::KeyPressed {
        key: key @ keyboard::Key::Character(_),
        physical_key,
        modifiers,
        text,
        ..
    }) = event
    else {
        return false;
    };
    let altgr = modifiers.control()
        && modifiers.alt()
        && !modifiers.logo()
        && text.as_deref().is_some_and(|s| {
            let mut chars = s.chars();
            chars
                .next()
                .is_some_and(|c| !c.is_alphanumeric() && !c.is_control())
                && chars.next().is_none()
        });
    let editing = modifiers.command()
        && match key.to_latin(*physical_key) {
            Some('a' | 'c' | 'x') => true,
            Some('v') => !modifiers.alt(),
            _ => false,
        };
    (modifiers.control() || modifiers.logo()) && !altgr && !editing
}

/// Ok tuşunun adımı: sol ve yukarı −1, sağ ve aşağı +1.
pub(crate) fn arrow_step(named: Named) -> Option<i32> {
    match named {
        Named::ArrowLeft | Named::ArrowUp => Some(-1),
        Named::ArrowRight | Named::ArrowDown => Some(1),
        _ => None,
    }
}

/// Fareyle basış odağı verir ya da alır: öğenin üstüne basılınca odak, başka yere basılınca bırakılır.
/// Değiştiyse yeniden çizim ister.
pub(crate) fn follow_press(
    flag: &mut Flag,
    event: &Event,
    cursor: mouse::Cursor,
    bounds: Rectangle,
    shell: &mut Shell<'_, impl Sized>,
) {
    if let Event::Mouse(mouse::Event::ButtonPressed(_)) = event {
        let over = cursor.is_over(bounds);
        if flag.0 != over {
            flag.0 = over;
            shell.request_redraw();
        }
    }
}

/// Düğme tabanlı bir öğeyi odaklanabilir yapan sarmalayıcı.
pub struct KeyFocus<'a, Message> {
    content: Element<'a, Message>,
    on_activate: Option<Message>,
    on_step: Option<Box<dyn Fn(i32) -> Option<Message> + 'a>>,
    radius: f32,
}

/// `content`'i odaklanabilir yapar; tuşlarının mesajları [`KeyFocus::on_activate`] ve
/// [`KeyFocus::on_step`] ile verilir. Mesajı olmayan (devre dışı) öğe odak almaz.
pub fn key_focus<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> KeyFocus<'a, Message> {
    KeyFocus {
        content: content.into(),
        on_activate: None,
        on_step: None,
        radius: shape::sm(),
    }
}

impl<'a, Message: Clone + 'a> KeyFocus<'a, Message> {
    /// Boşluk ve Enter: öğeye basılmış gibi.
    pub fn on_activate(mut self, message: Message) -> Self {
        self.on_activate = Some(message);
        self
    }

    /// Boşluk ve Enter, mesaj varsa.
    pub fn on_activate_maybe(mut self, message: Option<Message>) -> Self {
        self.on_activate = message;
        self
    }

    /// Oklar: −1 önceki, +1 sonraki; `None` o yönde gidecek yer yok demek.
    pub fn on_step(mut self, step: impl Fn(i32) -> Option<Message> + 'a) -> Self {
        self.on_step = Some(Box::new(step));
        self
    }

    /// Halenin köşe yarıçapı (yuvarlak öğeler için).
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    fn enabled(&self) -> bool {
        self.on_activate.is_some() || self.on_step.is_some()
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for KeyFocus<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Flag>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Flag::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
        // A field that went disabled lets the keyboard go.
        if !self.enabled() {
            tree.state.downcast_mut::<Flag>().0 = false;
        }
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
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
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if self.enabled() {
            follow_press(
                tree.state.downcast_mut::<Flag>(),
                event,
                cursor,
                bounds,
                shell,
            );
        }
        if let Some(child) = layout.children().next() {
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
        }
        if shell.is_event_captured() || !tree.state.downcast_ref::<Flag>().0 {
            return;
        }
        let Some((named, modifiers)) = pressed(event) else {
            return;
        };
        if modifiers.control() || modifiers.alt() || modifiers.logo() {
            return;
        }
        let message = match named {
            Named::Space | Named::Enter => self.on_activate.clone(),
            other => arrow_step(other)
                .and_then(|step| self.on_step.as_ref().and_then(|on_step| on_step(step))),
        };
        if let Some(message) = message {
            shell.publish(message);
            shell.capture_event();
        } else if matches!(named, Named::Space)
            || arrow_step(named).is_some() && self.on_step.is_some()
        {
            // A step past the end, or Space on a step-only field: still this field's key.
            shell.capture_event();
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
            .map_or_else(mouse::Interaction::default, |child| {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    child,
                    cursor,
                    viewport,
                    renderer,
                )
            })
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if self.enabled() {
            operation.focusable(None, layout.bounds(), tree.state.downcast_mut::<Flag>());
        }
        if let Some(child) = layout.children().next() {
            operation.traverse(&mut |operation| {
                self.content.as_widget_mut().operate(
                    &mut tree.children[0],
                    child,
                    renderer,
                    operation,
                );
            });
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
        let Some(child) = layout.children().next() else {
            return;
        };
        if tree.state.downcast_ref::<Flag>().0 {
            halo(renderer, theme, child.bounds(), self.radius);
        }
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

impl<'a, Message: Clone + 'a> From<KeyFocus<'a, Message>> for Element<'a, Message> {
    fn from(focus: KeyFocus<'a, Message>) -> Self {
        Element::new(focus)
    }
}
