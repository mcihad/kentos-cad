//! Odak halkası: içindeki bir metin girişi odaktayken kutunun çevresinde
//! yumuşak bir vurgu halesi (web'in odak halkası gibi). Kutunun kendi kenarı
//! odakta vurgu çizgisini alır (`style::field::input`); hale onu dışarıdan,
//! köşeleri izleyerek sarar ve imlecin nerede olduğunu uzaktan belli eder.
//!
//! ```text
//!  ╭─────────────────────╮   ← hale (vurgu, hafif saydam)
//!  │┌───────────────────┐│
//!  ││ ⌕ kad|            ││   ← kutunun kenarı (vurgu çizgisi)
//!  │└───────────────────┘│
//!  ╰─────────────────────╯
//! ```
//!
//! ```ignore
//! focus_ring(text_input("Katman ara", &search).style(style::field::input))
//! ```
//!
//! Sarmalanan öğe bir metin girişi ya da onu içeren bir kutu olabilir (ör.
//! arama simgesiyle birlikte). Hale, sarmalanan öğenin sınırına çizilir.

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::text::Renderer as TextRenderer;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::widget::text_input;
use iced::{Border, Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::theme::{Tokens, shape};

type Paragraph = <Renderer as TextRenderer>::Paragraph;

/// Halenin kalınlığı.
const WIDTH: f32 = 3.0;

/// Odak halkası.
pub struct FocusRing<'a, Message> {
    content: Element<'a, Message>,
}

/// İçindeki metin girişi odaktayken çevresine hale çizen kap.
pub fn focus_ring<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> FocusRing<'a, Message> {
    FocusRing {
        content: content.into(),
    }
}

/// Whether a text input anywhere under `tree` has the keyboard.
fn focused(tree: &Tree) -> bool {
    let own = tree.tag == tree::Tag::of::<text_input::State<Paragraph>>()
        && tree
            .state
            .downcast_ref::<text_input::State<Paragraph>>()
            .is_focused();
    own || tree.children.iter().any(focused)
}

impl<Message> Widget<Message, Theme, Renderer> for FocusRing<'_, Message> {
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
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
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
        if let Some(child) = layout.children().next() {
            self.content
                .as_widget_mut()
                .operate(&mut tree.children[0], child, renderer, operation);
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
        if focused(&tree.children[0]) {
            let t = Tokens::of(theme);
            let bounds = child.bounds().expand(WIDTH);
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        color: t.accent.scale_alpha(if t.is_dark { 0.32 } else { 0.24 }),
                        width: WIDTH,
                        radius: (shape::sm() + WIDTH).into(),
                    },
                    ..Quad::default()
                },
                iced::Color::TRANSPARENT,
            );
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

impl<'a, Message: 'a> From<FocusRing<'a, Message>> for Element<'a, Message> {
    fn from(ring: FocusRing<'a, Message>) -> Self {
        Element::new(ring)
    }
}
