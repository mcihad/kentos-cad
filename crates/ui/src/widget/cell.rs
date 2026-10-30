//! Izgara hücresi: nesne inceleyicinin, Öznitelikler'in ve her iki sütunlu
//! ızgaranın değer hücresi (DESIGN.md §7.5, docs/adr/0804).
//!
//! ```text
//!  Tür          │ Kapalı alan              ← salt okunur: zemin yok, panelin kendisi
//! ──────────────┼─────────────────────
//!  Yükseklik    │▒2.500▒▒▒▒▒▒▒▒▒▒▒▒▒▒ m    ← yazılabilir: hücre çizgiden çizgiye gömük zeminde
//! ──────────────┼─────────────────────
//!  Genişlik     │░0.800░░░░░░░░░░░░░░ m    ← imleç üstünde: zemin aydınlanır
//! ──────────────╔═════════════════════╗
//!  Yazı         ║ Ada 5|              ║    ← düzenlenirken: alan zemini ve tam kenarlarda vurgu çizgisi
//! ──────────────╚═════════════════════╝
//! ```
//!
//! Hücre durumları tek yerde çizer: içindeki düzenleyici (metin girişi, açılır
//! liste, tarih, parçalı seçim) çerçevesiz ve zeminsizdir, hücreyi doldurur ve
//! yazısı [`metrics::cell_pad`] içeriden başlar. Yazı dört durumda da kımıldamaz.
//! Köşeler düzdür: hücre ızgaranın bir parçasıdır, ızgarada duran bir kutu değil.
//!
//! Düzenlenme, içerideki odaklı metin girişinden ya da açık menüden anlaşılır;
//! içerik bunun için bir şey bildirmez, [`Opened`] dışında.

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Id, Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::widget::container;
use iced::{
    Background, Border, Color, Element, Event, Fill, Length, Point, Rectangle, Renderer, Size,
    Theme, Vector, mouse, window,
};

use crate::theme::{Tokens, metrics};

/// Bir menünün ya da açılır panelin [`Operation::custom`] ile bildirdiği:
/// açık mı. Hücre açık menüsünü düzenleme gibi gösterir.
pub(crate) struct Opened(pub bool);

/// İçeride düzenlenen bir şey var mı: odaklı giriş ya da açık panel.
struct Probe(bool);

impl Operation for Probe {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        self.0 |= state.is_focused();
    }

    fn custom(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn std::any::Any) {
        if let Some(opened) = state.downcast_ref::<Opened>() {
            self.0 |= opened.0;
        }
    }
}

#[derive(Default)]
struct State {
    hovered: bool,
    active: bool,
}

/// Değer hücresi.
pub struct Cell<'a, Message> {
    content: Element<'a, Message>,
    writable: bool,
    invalid: bool,
    /// Dışarıdan bilinen düzenleme (ör. hücrenin altında açık uzun metin düzenleyicisi).
    forced: bool,
    /// Hücrenin kendi zemini: koşullu biçim, kurala uymayan değer.
    tint: Option<Tint<'a>>,
}

/// Hücrenin zemini, temaya göre.
type Tint<'a> = Box<dyn Fn(&Theme) -> Color + 'a>;

impl<'a, Message: 'a> Cell<'a, Message> {
    /// Yazılabilir hücre: gömük zemin, imleçte aydınlanır, düzenlenirken kenarı vurgulu.
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            writable: true,
            invalid: false,
            forced: false,
            tint: None,
        }
    }

    /// Salt okunur hücre: zemini ve durumu yok, panelin üstünde düz yazı.
    pub fn flat(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            writable: false,
            ..Self::new(content)
        }
    }

    /// Kendisi hücreyi doldurmayan yazılabilir içerik (anahtar, renk örneği):
    /// yazıların hizasında, hücrenin içinde durur.
    pub fn padded(content: impl Into<Element<'a, Message>>) -> Self {
        Self::new(
            container(content)
                .padding(iced::Padding {
                    left: metrics::cell_pad(),
                    right: metrics::cell_end(),
                    ..iced::Padding::ZERO
                })
                .width(Fill),
        )
    }

    /// Geçersiz değer: kenar kırmızıdır.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Hücrenin kendi zemini, çizgiden çizgiye (tablonun koşullu biçimi, kurala uymayan değeri).
    pub fn tint(mut self, tint: impl Fn(&Theme) -> Color + 'a) -> Self {
        self.tint = Some(Box::new(tint));
        self
    }

    /// Düzenleniyor sayılır, içerisi ne derse desin.
    pub fn active(mut self, active: bool) -> Self {
        self.forced = active;
        self
    }
}

/// Bir hücrenin yan yana duran parçaları arasındaki ince çizgi (ör. iki koordinat).
pub fn divider<'a, Message: 'a>() -> Element<'a, Message> {
    container(iced::widget::space::horizontal())
        .width(1)
        .height(Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(Tokens::of(theme).rule().into()),
            snap: true,
            ..container::Style::default()
        })
        .into()
}

/// `element` zaten bir hücre mi: öyleyse ızgaranın satırı onu sütununa yayar, değilse içine pay bırakır.
pub(crate) fn is_cell<Message>(element: &Element<'_, Message>) -> bool {
    element.as_widget().tag() == tree::Tag::of::<State>()
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Cell<'a, Message> {
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
        Size::new(Length::Fill, Length::Fixed(metrics::row()))
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let height = metrics::row();
        let size = limits.resolve(Length::Fill, height, Size::new(0.0, height));
        let inner = layout::Limits::new(Size::ZERO, size);
        let content = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &inner);
        // The content in the middle, on whole pixels.
        let y = ((size.height - content.size().height) / 2.0).round();
        Node::with_children(size, vec![content.move_to(Point::new(0.0, y))])
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
        let hovered = cursor.is_over(layout.bounds());
        // Asked when the pointer pressed, the keyboard did something or a new frame comes (the
        // focus moved by a task, a menu closed by its own item): not for each pointer move.
        let asks = matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(_) | mouse::Event::ButtonReleased(_))
                | Event::Keyboard(_)
                | Event::Window(window::Event::RedrawRequested(_))
        );
        let mut active = tree.state.downcast_ref::<State>().active;
        if asks {
            let mut probe = Probe(false);
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                content,
                renderer,
                &mut probe,
            );
            active = probe.0;
        }
        let state = tree.state.downcast_mut::<State>();
        if hovered != state.hovered || active != state.active {
            state.hovered = hovered;
            state.active = active;
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
        let bounds = layout.bounds();
        let active = state.active || self.forced;
        if self.writable {
            let ground = if active {
                t.field
            } else if state.hovered {
                t.cell_hover()
            } else {
                t.cell()
            };
            renderer.fill_quad(
                Quad {
                    bounds,
                    snap: true,
                    ..Quad::default()
                },
                Background::Color(ground),
            );
        }
        if let Some(tint) = &self.tint {
            renderer.fill_quad(
                Quad {
                    bounds,
                    snap: true,
                    ..Quad::default()
                },
                Background::Color(tint(theme)),
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
        let edge = if self.invalid {
            Some(t.danger)
        } else {
            active.then_some(t.accent)
        };
        if let Some(color) = edge {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        color,
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    snap: true,
                    ..Quad::default()
                },
                Background::Color(Color::TRANSPARENT),
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

impl<'a, Message: 'a> From<Cell<'a, Message>> for Element<'a, Message> {
    fn from(cell: Cell<'a, Message>) -> Self {
        Element::new(cell)
    }
}
