//! Tablo ve ağaç satırı (docs/adr/0805): zemini, vurgusu ve altındaki çizgi tek yerde.
//!
//! Üzerine gelinen satır, tıklanmasa da aydınlanır: göz satırı izler. Seçili satır yumuşak vurgu zeminindedir, birkaçı
//! seçiliyken birincil olanı ayrıca vurgu kenarındadır. Satır en az [`metrics::row`] yüksekliğindedir, hücreler dikeyde
//! ortalanır; uzun yazı satırı büyütür.

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme,
    Vector, mouse,
};

use crate::theme::{Tokens, metrics};

/// Çok satırlı içeriğin üstünde ve altında kalan pay.
const WRAPPED_PAD: f32 = 4.0;

#[derive(Default)]
struct State {
    hovered: bool,
    pressed: bool,
}

/// Satır: hücrelerin dizisi ve durumları.
pub(crate) struct RowBox<'a, Message> {
    line: Element<'a, Message>,
    height: Option<f32>,
    on_press: Option<Message>,
    selected: bool,
    current: bool,
    tint: Option<Color>,
    ruled: bool,
    grid: bool,
}

impl<'a, Message: Clone + 'a> RowBox<'a, Message> {
    pub(crate) fn new(line: impl Into<Element<'a, Message>>) -> Self {
        Self {
            line: line.into(),
            height: None,
            on_press: None,
            selected: false,
            current: false,
            tint: None,
            ruled: false,
            grid: false,
        }
    }

    /// Sabit yükseklik: sanal listenin satırı.
    pub(crate) fn height(mut self, height: Option<f32>) -> Self {
        self.height = height;
        self
    }

    pub(crate) fn on_press_maybe(mut self, on_press: Option<Message>) -> Self {
        self.on_press = on_press;
        self
    }

    pub(crate) fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub(crate) fn current(mut self, current: bool) -> Self {
        self.current = current;
        self
    }

    pub(crate) fn tint(mut self, tint: Option<Color>) -> Self {
        self.tint = tint;
        self
    }

    /// Altında ince çizgi: tablo satırı; ağacın satırında yok.
    pub(crate) fn ruled(mut self, ruled: bool) -> Self {
        self.ruled = ruled;
        self
    }

    /// Hücrelerin arasında da çizgi: [`Table::grid`](super::Table::grid).
    pub(crate) fn grid(mut self, grid: bool) -> Self {
        self.grid = grid;
        self
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for RowBox<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.line)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.line));
    }

    fn size(&self) -> Size<Length> {
        Size::new(
            Length::Fill,
            self.height.map_or(Length::Shrink, Length::Fixed),
        )
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let width = limits.max().width;
        let rule = if self.ruled { 1.0 } else { 0.0 };
        let room = self
            .height
            .map_or(limits.max().height, |height| height - rule);
        let line = self.line.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(width, room)),
        );
        let content = line.size().height;
        let height = self.height.unwrap_or_else(|| {
            let least = metrics::row();
            if content + rule <= least {
                least
            } else {
                content + 2.0 * WRAPPED_PAD + rule
            }
        });
        // The cells in the middle, above the line, on whole pixels.
        let y = ((height - rule - content) / 2.0).round();
        Node::with_children(
            Size::new(width, height),
            vec![line.move_to(Point::new(0.0, y))],
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(line) = layout.children().next() {
            self.line
                .as_widget_mut()
                .operate(&mut tree.children[0], line, renderer, operation);
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
        let Some(line) = layout.children().next() else {
            return;
        };
        self.line.as_widget_mut().update(
            &mut tree.children[0],
            event,
            line,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        let bounds = layout.bounds();
        let over = cursor.is_over(bounds);
        let state = tree.state.downcast_mut::<State>();
        if over != state.hovered {
            state.hovered = over;
            shell.request_redraw();
        }
        if shell.is_event_captured() {
            return;
        }
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if over && self.on_press.is_some() =>
            {
                state.pressed = true;
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.pressed => {
                state.pressed = false;
                if over && let Some(on_press) = &self.on_press {
                    shell.publish(on_press.clone());
                }
                shell.capture_event();
                shell.request_redraw();
            }
            _ => {}
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
            .map_or_else(mouse::Interaction::default, |line| {
                self.line.as_widget().mouse_interaction(
                    &tree.children[0],
                    line,
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
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some(line) = layout.children().next() else {
            return;
        };
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let rule = if self.ruled { 1.0 } else { 0.0 };
        let lit = Rectangle {
            height: bounds.height - rule,
            ..bounds
        };
        let hovered = cursor.is_over(bounds);
        let pressed = tree.state.downcast_ref::<State>().pressed;
        let fill = |renderer: &mut Renderer, color: Color, border: Border| {
            renderer.fill_quad(
                Quad {
                    bounds: lit,
                    border,
                    snap: true,
                    ..Quad::default()
                },
                Background::Color(color),
            );
        };
        if self.selected {
            let ground = if hovered {
                t.selection_hover()
            } else {
                t.selection()
            };
            fill(renderer, ground, Border::default());
        } else {
            if let Some(tint) = self.tint {
                fill(renderer, tint, Border::default());
            }
            if hovered {
                let layer = t.layer(if pressed { 0.08 } else { 0.04 });
                fill(renderer, layer, Border::default());
            }
        }
        if self.current {
            fill(
                renderer,
                Color::TRANSPARENT,
                Border {
                    color: t.accent,
                    width: 1.0,
                    radius: 0.0.into(),
                },
            );
        }
        // Under the cells: a cell's edge in the accent is not cut by the column's line.
        if self.grid {
            for cell in line.children() {
                let cell = cell.bounds();
                let right = cell.x + cell.width;
                if right < bounds.x + bounds.width - 1.0 {
                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle {
                                x: right.round() - 1.0,
                                width: 1.0,
                                ..lit
                            },
                            snap: true,
                            ..Quad::default()
                        },
                        Background::Color(t.rule()),
                    );
                }
            }
        }
        self.line.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: t.text,
            },
            line,
            cursor,
            viewport,
        );
        if self.ruled {
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle {
                        y: bounds.y + bounds.height - 1.0,
                        height: 1.0,
                        ..bounds
                    },
                    snap: true,
                    ..Quad::default()
                },
                Background::Color(t.rule()),
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
        let line = layout.children().next()?;
        self.line.as_widget_mut().overlay(
            &mut tree.children[0],
            line,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: Clone + 'a> From<RowBox<'a, Message>> for Element<'a, Message> {
    fn from(row: RowBox<'a, Message>) -> Self {
        Element::new(row)
    }
}
