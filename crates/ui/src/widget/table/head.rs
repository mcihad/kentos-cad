//! Tablonun başlığı (docs/adr/0805): satırla aynı yükseklikte kendi bandı, altında güçlü bir çizgi.
//!
//! ```text
//! ▒ Ad ▒▒▒▒▒▒▒▒▒▒▒▒▒│▒ Tür ▒▒▒▒▒│▒▒▒▒▒ ⌃ Alan ▒▒
//! ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//!   Kapalı alan       Konut         1 204.50
//! ───────────────────────────────────────────────
//! ```
//!
//! Sıralanabilir sütunun bütün başlık hücresi basılır; ok sıralı sütunda vurgu renginde durur, öbürlerinde yalnız imleç
//! üstündeyken soluk görünür. Ok, yazının sütunun dışına bakan yanındadır: sağa hizalı başlığın yazısı sayıların hizasından
//! ayrılmaz. Boyutlanan sütunun sağ kenarı sürüklenir, çift tık `on_fit` gönderir. Sığmayan başlık “…” ile biter ve tamamı
//! ipucunda okunur.

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::mouse::{Click, click};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::alignment::Horizontal;
use iced::widget::container;
use iced::widget::tooltip::Position;
use iced::{
    Background, Element, Event, Length, Padding, Point, Rectangle, Renderer, Size, Theme, Vector,
    mouse,
};

use super::{Column, SortOrder, cell_pads, line};
use crate::icon::{Icon, icon};
use crate::theme::{Tokens, metrics, typography};
use crate::widget::elided::Elided;
use crate::widget::{Tip, tip};

/// Okun boyu ve yazıyla arası: sıralanabilir sütunda yazı bu payı oka bırakır.
const ARROW: f32 = 12.0;
const ARROW_GAP: f32 = 3.0;
/// Sütun kenarının tutulduğu pay, çizginin iki yanında.
const GRAB: f32 = 4.0;
/// Sürüklenen sütunun en dar yazı alanı.
const LEAST: f32 = 24.0;

/// Sütunun başlıktaki davranışı.
struct Part<'a, Message> {
    align: Horizontal,
    titled: bool,
    sorted: bool,
    sort: Option<Message>,
    resize: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    fit: Option<Message>,
}

/// Başlıkta imlecin üstünde durduğu şey.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hot {
    /// Sıralanabilir sütunun başlığı.
    Column(usize),
    /// Boyutlanan sütunun sağ kenarı.
    Edge(usize),
}

/// Sürüklenen sütun: imlecin başladığı yer ve o andaki yazı alanı.
#[derive(Debug, Clone, Copy)]
struct Drag {
    column: usize,
    from: f32,
    width: f32,
    last: f32,
}

#[derive(Default)]
struct State {
    hot: Option<Hot>,
    pressed: Option<usize>,
    drag: Option<Drag>,
    last_click: Option<Click>,
}

pub(super) struct Head<'a, Message> {
    line: Element<'a, Message>,
    /// Sıralanabilir sütunların okları, sütunlarıyla.
    arrows: Vec<(usize, Element<'a, Message>)>,
    parts: Vec<Part<'a, Message>>,
}

/// Sütunların başlığı.
pub(super) fn head<'a, Message: Clone + 'a>(
    columns: Vec<Column<'a, Message>>,
    layout: &[(Length, Horizontal)],
) -> Head<'a, Message> {
    let mut parts = Vec::with_capacity(columns.len());
    let mut arrows = Vec::new();
    let mut titles = Vec::with_capacity(columns.len());
    for (i, column) in columns.into_iter().enumerate() {
        let (order, sort) = match column.sort {
            Some((order, message)) => (order, Some(message)),
            None => (None, None),
        };
        let titled = !column.title.is_empty();
        let whole = column.title.to_string();
        let title: Element<'a, Message> = Elided::new(column.title)
            .size(typography::caption())
            .font(typography::ui_strong())
            .into();
        let title = if titled {
            tip(title, Tip::new(whole).when_cut(), Position::Top)
        } else {
            title
        };
        // The arrow's room, on the side away from the column's edge the title stands at.
        let slot = if sort.is_some() {
            ARROW + ARROW_GAP
        } else {
            0.0
        };
        let room = if column.align == Horizontal::Right {
            Padding::ZERO.left(slot)
        } else {
            Padding::ZERO.right(slot)
        };
        titles.push(container(title).padding(room).into());
        if sort.is_some() {
            arrows.push((
                i,
                icon(match order {
                    Some(SortOrder::Descending) => Icon::ChevronDown,
                    _ => Icon::ChevronUp,
                })
                .size(ARROW)
                .into(),
            ));
        }
        parts.push(Part {
            align: column.align,
            titled,
            sorted: order.is_some(),
            sort,
            resize: column.resize,
            fit: column.fit,
        });
    }
    Head {
        line: line(layout, titles).into(),
        arrows,
        parts,
    }
}

/// Sütunların yerleri.
fn cells(layout: Layout<'_>) -> impl Iterator<Item = Rectangle> + '_ {
    layout
        .children()
        .next()
        .into_iter()
        .flat_map(|line| line.children())
        .map(|cell| cell.bounds())
}

impl<'a, Message: Clone + 'a> Head<'a, Message> {
    /// `point`'te ne var: önce boyutlanan sütunun kenarı, sonra sıralanabilir sütun.
    fn hit(&self, layout: Layout<'_>, point: Option<Point>) -> Option<Hot> {
        let point = point?;
        if !layout.bounds().contains(point) {
            return None;
        }
        let mut under = None;
        for (i, cell) in cells(layout).enumerate() {
            let part = self.parts.get(i)?;
            if part.resize.is_some() && (point.x - (cell.x + cell.width)).abs() <= GRAB {
                return Some(Hot::Edge(i));
            }
            if point.x >= cell.x && point.x < cell.x + cell.width && part.sort.is_some() {
                under = Some(Hot::Column(i));
            }
        }
        under
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Head<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        std::iter::once(Tree::new(&self.line))
            .chain(self.arrows.iter().map(|(_, arrow)| Tree::new(arrow)))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let all: Vec<&Element<'a, Message>> = std::iter::once(&self.line)
            .chain(self.arrows.iter().map(|(_, arrow)| arrow))
            .collect();
        tree.diff_children(&all);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(metrics::row() + 1.0))
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let band = metrics::row();
        let whole = band + 1.0;
        let size = limits.resolve(Length::Fill, whole, Size::new(0.0, whole));
        let inner = layout::Limits::new(Size::ZERO, Size::new(size.width, band));
        let line = self
            .line
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &inner);
        let y = ((band - line.size().height) / 2.0).round();
        let line = line.move_to(Point::new(0.0, y));
        let mut nodes = Vec::with_capacity(self.arrows.len() + 1);
        for (at, (column, arrow)) in self.arrows.iter_mut().enumerate() {
            let node = arrow.as_widget_mut().layout(
                &mut tree.children[at + 1],
                renderer,
                &layout::Limits::new(Size::ZERO, Size::new(ARROW, ARROW)),
            );
            let cell = &line.children()[*column];
            let holder = &cell.children()[0];
            let title = holder.children()[0].bounds();
            let title_x = cell.bounds().x + holder.bounds().x + title.x;
            let x = if self.parts[*column].align == Horizontal::Right {
                title_x - ARROW_GAP - ARROW
            } else {
                title_x + title.width + ARROW_GAP
            };
            nodes.push(node.move_to(Point::new(x.round(), ((band - ARROW) / 2.0).round())));
        }
        nodes.insert(0, line);
        Node::with_children(size, nodes)
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
        let state = tree.state.downcast_mut::<State>();
        // A column dragged goes on past the band, past the scrolled part too.
        if let Some(drag) = &mut state.drag {
            match event {
                Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                    if let (Some(point), Some(resize)) = (
                        cursor.land().position(),
                        self.parts
                            .get(drag.column)
                            .and_then(|part| part.resize.as_ref()),
                    ) {
                        let width = (drag.width + point.x - drag.from).round().max(LEAST);
                        if width != drag.last {
                            drag.last = width;
                            // A column dragged is no half of a double click.
                            state.last_click = None;
                            shell.publish(resize(typography::unscaled(width)));
                        }
                    }
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    state.drag = None;
                    shell.request_redraw();
                }
                _ => {}
            }
            shell.capture_event();
            return;
        }
        let hot = self.hit(layout, cursor.position());
        if hot != state.hot {
            state.hot = hot;
            shell.request_redraw();
        }
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(point) = cursor.position() else {
                    return;
                };
                match hot {
                    Some(Hot::Edge(column)) => {
                        let click = Click::new(point, mouse::Button::Left, state.last_click);
                        state.last_click = Some(click);
                        let part = &self.parts[column];
                        if click.kind() == click::Kind::Double && part.fit.is_some() {
                            if let Some(fit) = &part.fit {
                                shell.publish(fit.clone());
                            }
                        } else if let Some(cell) = cells(layout).nth(column) {
                            let (left, right) = cell_pads(column, self.parts.len());
                            let width = cell.width - left - right;
                            state.drag = Some(Drag {
                                column,
                                from: point.x,
                                width,
                                last: width,
                            });
                        }
                        shell.capture_event();
                        shell.request_redraw();
                    }
                    Some(Hot::Column(column)) => {
                        state.pressed = Some(column);
                        shell.capture_event();
                        shell.request_redraw();
                    }
                    None => {}
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(column) = state.pressed.take() {
                    if hot == Some(Hot::Column(column))
                        && let Some(sort) = &self.parts[column].sort
                    {
                        shell.publish(sort.clone());
                    }
                    shell.capture_event();
                    shell.request_redraw();
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
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if tree.state.downcast_ref::<State>().drag.is_some() {
            return mouse::Interaction::ResizingHorizontally;
        }
        match self.hit(layout, cursor.position()) {
            Some(Hot::Edge(_)) => mouse::Interaction::ResizingHorizontally,
            Some(Hot::Column(_)) => mouse::Interaction::Pointer,
            None => layout
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
                }),
        }
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
        let mut children = layout.children();
        let Some(line) = children.next() else {
            return;
        };
        let t = Tokens::of(theme);
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let band = Rectangle {
            height: metrics::row(),
            ..bounds
        };
        let quad = |renderer: &mut Renderer, at: Rectangle, color: iced::Color| {
            renderer.fill_quad(
                Quad {
                    bounds: at,
                    snap: true,
                    ..Quad::default()
                },
                Background::Color(color),
            );
        };
        quad(renderer, band, t.table_head());
        let columns: Vec<Rectangle> = cells(layout).collect();
        let hot = match state.drag {
            Some(drag) => Some(Hot::Edge(drag.column)),
            None => self.hit(layout, cursor.position()),
        };
        // The head of the column under the pointer, from line to line.
        if let Some(Hot::Column(column)) = hot
            && let Some(cell) = columns.get(column)
        {
            let pressed = state.pressed == Some(column);
            quad(
                renderer,
                Rectangle {
                    x: cell.x,
                    width: cell.width,
                    ..band
                },
                t.layer(if pressed { 0.1 } else { 0.05 }),
            );
        }
        let text = renderer::Style {
            text_color: t.text,
        };
        self.line.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &text,
            line,
            cursor,
            viewport,
        );
        for (at, ((column, arrow), place)) in self.arrows.iter().zip(children).enumerate() {
            let sorted = self.parts.get(*column).is_some_and(|part| part.sorted);
            if sorted || hot == Some(Hot::Column(*column)) {
                arrow.as_widget().draw(
                    &tree.children[at + 1],
                    renderer,
                    theme,
                    &renderer::Style {
                        text_color: if sorted { t.accent } else { t.soft() },
                    },
                    place,
                    cursor,
                    viewport,
                );
            }
        }
        // A line between two titled columns; after the last one where the band goes on past it. A column whose edge
        // is held has its line in the accent.
        let last = columns.len().saturating_sub(1);
        for (i, cell) in columns.iter().enumerate() {
            let right = cell.x + cell.width;
            let titled = self.parts.get(i).is_some_and(|part| part.titled);
            let held = hot == Some(Hot::Edge(i));
            let shows = titled
                && if i == last {
                    right < bounds.x + bounds.width - 1.0
                } else {
                    self.parts.get(i + 1).is_some_and(|part| part.titled)
                };
            if shows || held {
                quad(
                    renderer,
                    Rectangle {
                        x: right.round() - 1.0,
                        width: 1.0,
                        ..band
                    },
                    if held { t.accent } else { t.border_strong() },
                );
            }
        }
        quad(
            renderer,
            Rectangle {
                y: bounds.y + band.height,
                height: 1.0,
                ..bounds
            },
            t.border_strong(),
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

impl<'a, Message: Clone + 'a> From<Head<'a, Message>> for Element<'a, Message> {
    fn from(head: Head<'a, Message>) -> Self {
        Element::new(head)
    }
}
