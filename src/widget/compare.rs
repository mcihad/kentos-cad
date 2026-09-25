//! Karşılaştırma perdesi: iki içeriği üst üste koyup sürüklenen ayraçla
//! birinin solunu (üstünü), öbürünün sağını (altını) gösterir.
//!
//! ```text
//! ┌───────────────────┬─────────────────────┐
//! │ Önce              ┃               Sonra │
//! │   (sol içerik)    ◀▶   (sağ içerik)     │
//! │                   ┃                     │
//! └───────────────────┴─────────────────────┘
//! ```
//!
//! CBS'deki kaydırma (swipe) aracı ve görüntü karşılaştırmaları gibi: iki
//! zemin, iki tarih ya da iki çizim stili aynı yerde karşılaştırılır.
//! Ayraç tutamağından ya da çizgisinden sürüklenir; çift tıklamak ortaya
//! alır. Fare olayları imlecin olduğu yana gider; bir yanda başlayan
//! sürükleme ayracı geçse de o yanda sürer. İki içerik aynı görünümü
//! paylaşıyorsa (ör. aynı harita görünümü) biri kaydırılınca öbürü de
//! kayar.
//!
//! ```ignore
//! Compare::new(before, after, self.split, Message::SplitMoved)
//!     .labels("2019", "2024")
//! ```

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::mouse::Click;
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::text::{self, Renderer as _, Text};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::alignment::Vertical;
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::{
    Background, Border, Element, Event, Length, Pixels, Point, Rectangle, Renderer, Shadow, Size,
    Theme, Vector, mouse,
};

use crate::theme::{Tokens, typography};

/// Ayracı tutmak için imlecin çizgiye en fazla uzaklığı ve tutamağın çapı.
const GRAB: f32 = 8.0;
const HANDLE: f32 = 28.0;

/// Ayracın yönü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Dikey ayraç: solda birinci, sağda ikinci içerik.
    #[default]
    Horizontal,
    /// Yatay ayraç: üstte birinci, altta ikinci içerik.
    Vertical,
}

/// Karşılaştırma perdesi.
pub struct Compare<'a, Message> {
    first: Element<'a, Message>,
    second: Element<'a, Message>,
    split: f32,
    on_change: Box<dyn Fn(f32) -> Message + 'a>,
    labels: Option<(String, String)>,
    direction: Direction,
    width: Length,
    height: Length,
}

impl<'a, Message: 'a> Compare<'a, Message> {
    /// `split` ayracın yeridir: 0 baş, 1 son.
    pub fn new(
        first: impl Into<Element<'a, Message>>,
        second: impl Into<Element<'a, Message>>,
        split: f32,
        on_change: impl Fn(f32) -> Message + 'a,
    ) -> Self {
        Self {
            first: first.into(),
            second: second.into(),
            split: split.clamp(0.0, 1.0),
            on_change: Box::new(on_change),
            labels: None,
            direction: Direction::Horizontal,
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    /// İki yanın adı; köşelerde yazar.
    pub fn labels(mut self, first: impl Into<String>, second: impl Into<String>) -> Self {
        self.labels = Some((first.into(), second.into()));
        self
    }

    /// Yatay ayraç: üst ve alt.
    pub fn vertical(mut self) -> Self {
        self.direction = Direction::Vertical;
        self
    }

    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Ayracın ekrandaki yeri: dikeyse x, yataysa y.
    fn divider(&self, bounds: Rectangle) -> f32 {
        match self.direction {
            Direction::Horizontal => (bounds.x + bounds.width * self.split).round(),
            Direction::Vertical => (bounds.y + bounds.height * self.split).round(),
        }
    }

    /// İki yanın alanları.
    fn halves(&self, bounds: Rectangle) -> (Rectangle, Rectangle) {
        let divider = self.divider(bounds);

        match self.direction {
            Direction::Horizontal => (
                Rectangle::new(
                    bounds.position(),
                    Size::new(divider - bounds.x, bounds.height),
                ),
                Rectangle::new(
                    Point::new(divider, bounds.y),
                    Size::new(bounds.x + bounds.width - divider, bounds.height),
                ),
            ),
            Direction::Vertical => (
                Rectangle::new(
                    bounds.position(),
                    Size::new(bounds.width, divider - bounds.y),
                ),
                Rectangle::new(
                    Point::new(bounds.x, divider),
                    Size::new(bounds.width, bounds.y + bounds.height - divider),
                ),
            ),
        }
    }

    /// Noktanın ayraca uzaklığı.
    fn distance(&self, bounds: Rectangle, point: Point) -> f32 {
        let divider = self.divider(bounds);

        match self.direction {
            Direction::Horizontal => (point.x - divider).abs(),
            Direction::Vertical => (point.y - divider).abs(),
        }
    }

    /// Noktaya göre ayracın yeri, 0 ile 1 arası.
    fn split_at(&self, bounds: Rectangle, point: Point) -> f32 {
        match self.direction {
            Direction::Horizontal => (point.x - bounds.x) / bounds.width.max(1.0),
            Direction::Vertical => (point.y - bounds.y) / bounds.height.max(1.0),
        }
        .clamp(0.0, 1.0)
    }
}

#[derive(Debug, Default)]
struct State {
    dragging: bool,
    hovered: bool,
    last: Option<Click>,
    /// İmlecin son bulunduğu yan (0 birinci, 1 ikinci).
    side: Option<usize>,
    /// Basılı tuşun başladığı yan: bırakılana kadar fare olayları ona gider
    /// (ör. haritayı sürüklerken ayracın öbür yanına geçmek).
    pressed: Option<usize>,
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Compare<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.first), Tree::new(&self.second)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.first, &self.second]);
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let limits = limits.width(self.width).height(self.height);
        let size = limits.resolve(self.width, self.height, Size::ZERO);
        let inner = layout::Limits::new(Size::ZERO, size);

        let first = self
            .first
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &inner);
        let second = self
            .second
            .as_widget_mut()
            .layout(&mut tree.children[1], renderer, &inner);

        Node::with_children(size, vec![first, second])
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
        let state = tree.state.downcast_mut::<State>();

        match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if state.dragging {
                    shell.publish((self.on_change)(self.split_at(bounds, *position)));
                    shell.capture_event();
                    return;
                }

                let hovered = cursor.is_over(bounds) && self.distance(bounds, *position) <= GRAB;

                if hovered != state.hovered {
                    state.hovered = hovered;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(point) = cursor.position_over(bounds)
                    && self.distance(bounds, point) <= GRAB
                {
                    // Ayraca çift tıklamak ortaya alır.
                    let click = Click::new(point, mouse::Button::Left, state.last);

                    if click.kind() == iced::advanced::mouse::click::Kind::Double {
                        shell.publish((self.on_change)(0.5));
                    } else {
                        state.dragging = true;
                    }

                    state.last = Some(click);
                    shell.capture_event();
                    return;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.dragging => {
                state.dragging = false;
                shell.request_redraw();
                shell.capture_event();
                return;
            }
            _ => {}
        }

        // Fare olayları imlecin olduğu yana (tuş basılıysa basıldığı yana)
        // gider; imleç yan değiştirince eski yan bir kez imlecin çıktığını
        // görür. Öbür olaylar (klavye, pencere) iki yana da gider.
        let (first_half, _) = self.halves(bounds);
        let over = cursor
            .position_over(bounds)
            .map(|point| usize::from(!first_half.contains(point)));
        let dragging = state.dragging;
        let mut children = layout.children();
        let (Some(first), Some(second)) = (children.next(), children.next()) else {
            return;
        };
        let layouts = [first, second];
        let mut forward = |index: usize, cursor: mouse::Cursor, tree: &mut Tree| {
            let element = if index == 0 {
                &mut self.first
            } else {
                &mut self.second
            };

            element.as_widget_mut().update(
                &mut tree.children[index],
                event,
                layouts[index],
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        };

        if dragging {
            return;
        }

        match event {
            Event::Mouse(mouse_event) => {
                let state = tree.state.downcast_mut::<State>();
                let (pressed, side) = (state.pressed, state.side);
                let target = pressed.or(over);
                let moved = matches!(mouse_event, mouse::Event::CursorMoved { .. });

                if moved {
                    if pressed.is_none() {
                        state.side = over;
                    }
                } else if let mouse::Event::ButtonPressed(_) = mouse_event {
                    state.pressed = over;
                } else if let mouse::Event::ButtonReleased(_) = mouse_event {
                    state.pressed = None;
                }

                if moved
                    && pressed.is_none()
                    && let Some(left) = side.filter(|side| Some(*side) != over)
                {
                    forward(left, cursor.levitate(), tree);
                }

                if let Some(target) = target {
                    forward(target, cursor, tree);
                }
            }
            _ => {
                for index in 0..2 {
                    let cursor = if over == Some(index) {
                        cursor
                    } else {
                        cursor.levitate()
                    };

                    forward(index, cursor, tree);
                }
            }
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
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let resize = match self.direction {
            Direction::Horizontal => mouse::Interaction::ResizingHorizontally,
            Direction::Vertical => mouse::Interaction::ResizingVertically,
        };

        if state.dragging {
            return resize;
        }

        let Some(point) = cursor.position_over(bounds) else {
            return mouse::Interaction::None;
        };

        if self.distance(bounds, point) <= GRAB {
            return resize;
        }

        let (first_half, _) = self.halves(bounds);
        let mut children = layout.children();
        let (Some(first), Some(second)) = (children.next(), children.next()) else {
            return mouse::Interaction::None;
        };

        if first_half.contains(point) {
            self.first.as_widget().mouse_interaction(
                &tree.children[0],
                first,
                cursor,
                viewport,
                renderer,
            )
        } else {
            self.second.as_widget().mouse_interaction(
                &tree.children[1],
                second,
                cursor,
                viewport,
                renderer,
            )
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
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let (first_half, second_half) = self.halves(bounds);
        let mut children = layout.children();
        let (Some(first), Some(second)) = (children.next(), children.next()) else {
            return;
        };
        let over = cursor
            .position_over(bounds)
            .map(|point| usize::from(!first_half.contains(point)));
        let active_side = state.pressed.or(over);

        for (index, (element, child, half)) in [
            (&self.first, first, first_half),
            (&self.second, second, second_half),
        ]
        .into_iter()
        .enumerate()
        {
            if half.width < 1.0 || half.height < 1.0 {
                continue;
            }

            let cursor = if !state.dragging && active_side == Some(index) {
                cursor
            } else {
                cursor.levitate()
            };

            renderer.with_layer(half, |renderer| {
                element.as_widget().draw(
                    &tree.children[index],
                    renderer,
                    theme,
                    style,
                    child,
                    cursor,
                    viewport,
                );
            });
        }

        let divider = self.divider(bounds);
        let active = state.dragging || state.hovered;

        renderer.with_layer(bounds, |renderer| {
            // Ayraç çizgisi ve ortasındaki tutamak.
            let (line, center) = match self.direction {
                Direction::Horizontal => (
                    Rectangle::new(
                        Point::new(divider - 1.0, bounds.y),
                        Size::new(2.0, bounds.height),
                    ),
                    Point::new(divider, bounds.center_y()),
                ),
                Direction::Vertical => (
                    Rectangle::new(
                        Point::new(bounds.x, divider - 1.0),
                        Size::new(bounds.width, 2.0),
                    ),
                    Point::new(bounds.center_x(), divider),
                ),
            };

            renderer.fill_quad(
                Quad {
                    bounds: line,
                    shadow: Shadow {
                        color: t.shadow(),
                        offset: Vector::ZERO,
                        blur_radius: 4.0,
                    },
                    ..Quad::default()
                },
                Background::Color(if active { t.accent } else { t.on_accent }),
            );

            let handle = Rectangle::new(
                Point::new(center.x - HANDLE / 2.0, center.y - HANDLE / 2.0),
                Size::new(HANDLE, HANDLE),
            );

            renderer.fill_quad(
                Quad {
                    bounds: handle,
                    border: Border {
                        color: if active { t.accent } else { t.border },
                        width: 1.5,
                        radius: (HANDLE / 2.0).into(),
                    },
                    shadow: Shadow {
                        color: t.shadow(),
                        offset: Vector::new(0.0, 1.0),
                        blur_radius: 6.0,
                    },
                    ..Quad::default()
                },
                Background::Color(t.popover),
            );

            let size = Pixels(typography::scaled(11.0).round());
            let arrows = match self.direction {
                Direction::Horizontal => "◀ ▶",
                Direction::Vertical => "▲▼",
            };

            renderer.fill_text(
                Text {
                    content: arrows.to_owned(),
                    bounds: handle.size(),
                    size,
                    line_height: LineHeight::Absolute(size),
                    font: typography::ui(),
                    align_x: text::Alignment::Center,
                    align_y: Vertical::Center,
                    shaping: Shaping::Advanced,
                    wrapping: Wrapping::None,
                },
                center,
                if active { t.accent } else { t.muted },
                handle,
            );

            // Yanların adları köşelerde.
            if let Some((first_label, second_label)) = &self.labels {
                let size = Pixels(typography::scaled(11.0).round());
                let tag = |renderer: &mut Renderer, text: &str, half: Rectangle, end: bool| {
                    let width = text.chars().count() as f32 * size.0 * 0.6 + 14.0;
                    let height = size.0 + 8.0;

                    if half.width < width + 16.0 || half.height < height + 16.0 {
                        return;
                    }

                    let x = if end {
                        half.x + half.width - width - 8.0
                    } else {
                        half.x + 8.0
                    };
                    let y = if end && self.direction == Direction::Vertical {
                        half.y + half.height - height - 8.0
                    } else {
                        half.y + 8.0
                    };
                    let tag = Rectangle::new(Point::new(x, y), Size::new(width, height));

                    renderer.fill_quad(
                        Quad {
                            bounds: tag,
                            border: Border {
                                radius: 3.0.into(),
                                ..Border::default()
                            },
                            ..Quad::default()
                        },
                        Background::Color(t.popover.scale_alpha(0.85)),
                    );
                    renderer.fill_text(
                        Text {
                            content: text.to_owned(),
                            bounds: tag.size(),
                            size,
                            line_height: LineHeight::Absolute(size),
                            font: typography::ui_strong(),
                            align_x: text::Alignment::Center,
                            align_y: Vertical::Center,
                            shaping: Shaping::Advanced,
                            wrapping: Wrapping::None,
                        },
                        Point::new(tag.center_x(), tag.center_y()),
                        t.text,
                        tag,
                    );
                };

                tag(renderer, first_label, first_half, false);
                tag(renderer, second_label, second_half, true);
            }
        });
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let mut children = layout.children();

        if let (Some(first), Some(second)) = (children.next(), children.next()) {
            self.first
                .as_widget_mut()
                .operate(&mut tree.children[0], first, renderer, operation);
            self.second
                .as_widget_mut()
                .operate(&mut tree.children[1], second, renderer, operation);
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
        let mut children = layout.children();
        let (first_layout, second_layout) = (children.next()?, children.next()?);
        let (first_tree, second_tree) = tree.children.split_at_mut(1);

        let overlays: Vec<_> = [
            self.first.as_widget_mut().overlay(
                &mut first_tree[0],
                first_layout,
                renderer,
                viewport,
                translation,
            ),
            self.second.as_widget_mut().overlay(
                &mut second_tree[0],
                second_layout,
                renderer,
                viewport,
                translation,
            ),
        ]
        .into_iter()
        .flatten()
        .collect();

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, Message: 'a> From<Compare<'a, Message>> for Element<'a, Message> {
    fn from(compare: Compare<'a, Message>) -> Self {
        Element::new(compare)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::space;

    fn compare(direction: Direction, split: f32) -> Compare<'static, f32> {
        Compare::new(space::horizontal(), space::horizontal(), split, |split| {
            split
        })
        .direction(direction)
    }

    #[test]
    fn the_divider_splits_the_area_at_its_fraction() {
        let bounds = Rectangle::new(Point::new(100.0, 50.0), Size::new(400.0, 200.0));
        let (first, second) = compare(Direction::Horizontal, 0.25).halves(bounds);

        assert_eq!(
            first,
            Rectangle::new(Point::new(100.0, 50.0), Size::new(100.0, 200.0))
        );
        assert_eq!(
            second,
            Rectangle::new(Point::new(200.0, 50.0), Size::new(300.0, 200.0))
        );

        let (top, bottom) = compare(Direction::Vertical, 0.5).halves(bounds);
        assert_eq!(top.height, 100.0);
        assert_eq!(bottom.y, 150.0);
    }

    #[test]
    fn dragging_maps_the_cursor_to_a_fraction() {
        let bounds = Rectangle::new(Point::new(100.0, 50.0), Size::new(400.0, 200.0));
        let horizontal = compare(Direction::Horizontal, 0.5);

        assert_eq!(horizontal.split_at(bounds, Point::new(200.0, 60.0)), 0.25);
        assert_eq!(horizontal.split_at(bounds, Point::new(900.0, 60.0)), 1.0);
        assert_eq!(horizontal.distance(bounds, Point::new(305.0, 60.0)), 5.0);

        let vertical = compare(Direction::Vertical, 0.5);
        assert_eq!(vertical.split_at(bounds, Point::new(0.0, 100.0)), 0.25);
    }
}

/// Gerçek olaylarla: ayracı sürükleme, çift tıklama ve olayların yanlara
/// dağılması.
#[cfg(all(test, feature = "snapshot"))]
mod interaction {
    use iced::widget::{button, space};
    use iced::{Element, Fill, Point, Size};

    use super::Compare;
    use crate::snapshot::{Input, Snapshot};

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Event {
        Split(f32),
        First,
        Second,
    }

    struct Demo {
        split: f32,
        events: Vec<Event>,
    }

    fn view(demo: &Demo) -> Element<'_, Event> {
        let side = |event| {
            button(space::horizontal().width(Fill).height(Fill))
                .on_press(event)
                .width(Fill)
                .height(Fill)
        };

        Compare::new(
            side(Event::First),
            side(Event::Second),
            demo.split,
            Event::Split,
        )
        .into()
    }

    #[test]
    fn the_divider_moves_and_each_side_gets_its_clicks() {
        let mut snapshot = Snapshot::new(Size::new(400.0, 200.0)).expect("çizici kurulamadı");
        let mut demo = Demo {
            split: 0.5,
            events: Vec::new(),
        };
        let mut update = |demo: &mut Demo, event| {
            if let Event::Split(split) = event {
                demo.split = split;
            }

            demo.events.push(event);
        };
        let mut input = |demo: &mut Demo, input| snapshot.input(demo, view, &mut update, input);

        input(
            &mut demo,
            Input::Drag(Point::new(200.0, 100.0), Point::new(100.0, 100.0)),
        );
        assert_eq!(demo.split, 0.25);

        // Ayracın solu birinci, sağı ikinci içeriktir.
        demo.events.clear();
        input(&mut demo, Input::Click(Point::new(50.0, 100.0)));
        input(&mut demo, Input::Click(Point::new(300.0, 100.0)));
        assert_eq!(demo.events, [Event::First, Event::Second]);
    }
}
