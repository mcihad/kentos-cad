//! Sanal liste: yalnızca görünen satırları kuran, eşit yükseklikli
//! satırlardan oluşan kaydırılabilir liste.
//!
//! On binlerce satırlı listeler (ör. bir katmanın bütün öğeleri) için:
//! satırlar `view` ile yalnızca görünür olduklarında kurulur, çizilir ve
//! olay alır. Kaydırma tekerlekle ve sağdaki ince çubukla yapılır;
//! [`VirtualList::reveal`] verilen satırı görünür yapar (ör. haritada
//! seçilen öğenin satırı).
//!
//! ```ignore
//! VirtualList::new(self.rows.len(), row_height(), |index| self.row(index))
//!     .reveal(self.selected)
//! ```

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::{
    Background, Border, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
    mouse,
};

use crate::theme::Tokens;

/// Kaydırma çubuğunun genişliği (üzerine gelince kalınlaşır) ve en kısa
/// tutamağı.
const BAR: f32 = 4.0;
const BAR_HOT: f32 = 7.0;
const MIN_THUMB: f32 = 24.0;
/// Tekerleğin bir satırı kaç satır kaydırır.
const WHEEL_ROWS: f32 = 3.0;

/// Sanal liste.
pub struct VirtualList<'a, Message> {
    count: usize,
    row_height: f32,
    view: Box<dyn Fn(usize) -> Element<'a, Message> + 'a>,
    /// Görünen satırlar, yerleşimde kurulur.
    rows: Vec<(usize, Element<'a, Message>)>,
    width: Length,
    height: Length,
    reveal: Option<usize>,
}

impl<'a, Message: 'a> VirtualList<'a, Message> {
    /// `count` satır; her biri `row_height` piksel yüksekliğinde ve `view`
    /// ile kurulur.
    pub fn new(
        count: usize,
        row_height: f32,
        view: impl Fn(usize) -> Element<'a, Message> + 'a,
    ) -> Self {
        Self {
            count,
            row_height: row_height.max(1.0),
            view: Box::new(view),
            rows: Vec::new(),
            width: Length::Fill,
            height: Length::Fill,
            reveal: None,
        }
    }

    /// Satırı görünür yapar; aynı satır için bir kez, kullanıcının
    /// kaydırmasını geri almaz.
    pub fn reveal(mut self, index: Option<usize>) -> Self {
        self.reveal = index;
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

    fn total(&self) -> f32 {
        self.count as f32 * self.row_height
    }
}

/// Görünür aralık: kaydırmaya ve yüksekliğe göre ilk ve son (hariç) satır.
fn visible(offset: f32, height: f32, row_height: f32, count: usize) -> std::ops::Range<usize> {
    let first = (offset / row_height).floor().max(0.0) as usize;
    let last = ((offset + height) / row_height).ceil().max(0.0) as usize;

    first.min(count)..last.min(count)
}

/// Satırı görünür yapacak kaydırma; zaten görünüyorsa aynısı.
fn reveal(offset: f32, height: f32, row_height: f32, index: usize) -> f32 {
    let top = index as f32 * row_height;
    let bottom = top + row_height;

    if top < offset {
        top
    } else if bottom > offset + height {
        bottom - height
    } else {
        offset
    }
}

#[derive(Default)]
struct State {
    offset: f32,
    /// Görünen satırların ağaçları, satırlarıyla.
    trees: Vec<(usize, Tree)>,
    /// Kaydırma çubuğu sürükleniyor: imlecin başladığı yer ve kaydırma.
    dragging: Option<(f32, f32)>,
    bar_hovered: bool,
    revealed: Option<usize>,
}

impl<'a, Message: 'a> VirtualList<'a, Message> {
    /// Kaydırma çubuğunun rayı ve tutamağı; kaydırılacak bir şey yoksa yok.
    fn bar(&self, bounds: Rectangle, offset: f32, hot: bool) -> Option<(Rectangle, Rectangle)> {
        let total = self.total();

        if total <= bounds.height {
            return None;
        }

        let width = if hot { BAR_HOT } else { BAR };
        let rail = Rectangle::new(
            Point::new(bounds.x + bounds.width - width - 1.0, bounds.y),
            Size::new(width, bounds.height),
        );
        let length = (bounds.height * bounds.height / total).max(MIN_THUMB);
        let travel = bounds.height - length;
        let top = travel * offset / (total - bounds.height);

        Some((
            rail,
            Rectangle::new(Point::new(rail.x, rail.y + top), Size::new(width, length)),
        ))
    }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for VirtualList<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let size = limits.resolve(self.width, self.height, Size::ZERO);
        let state = tree.state.downcast_mut::<State>();
        let total = self.total();

        if let Some(index) = self.reveal.filter(|index| state.revealed != Some(*index)) {
            state.revealed = Some(index);
            state.offset = reveal(state.offset, size.height, self.row_height, index);
        }

        state.offset = state.offset.clamp(0.0, (total - size.height).max(0.0));

        let range = visible(state.offset, size.height, self.row_height, self.count);
        self.rows = range.map(|index| (index, (self.view)(index))).collect();

        // Ağaçlar satırlarıyla eşlenir; görünmez olan satırın ağacı atılır.
        let mut previous = std::mem::take(&mut state.trees);
        let limits = layout::Limits::new(Size::ZERO, Size::new(size.width, self.row_height));
        let mut nodes = Vec::with_capacity(self.rows.len());

        for (index, element) in &mut self.rows {
            let mut row_tree = match previous.iter().position(|(stored, _)| stored == index) {
                Some(position) => {
                    let (_, mut row_tree) = previous.swap_remove(position);
                    row_tree.diff(&*element);
                    row_tree
                }
                None => Tree::new(&*element),
            };

            let node = element
                .as_widget_mut()
                .layout(&mut row_tree, renderer, &limits)
                .move_to(Point::new(
                    0.0,
                    *index as f32 * self.row_height - state.offset,
                ));

            nodes.push(node);
            state.trees.push((*index, row_tree));
        }

        Node::with_children(size, nodes)
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
        let total = self.total();
        let max_offset = (total - bounds.height).max(0.0);

        // Kaydırma çubuğu sürükleniyor.
        if let Some((origin, start)) = state.dragging {
            match event {
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    if let Some((_, thumb)) = self.bar(bounds, state.offset, true) {
                        let travel = (bounds.height - thumb.height).max(1.0);
                        state.offset = (start + (position.y - origin) * max_offset / travel)
                            .clamp(0.0, max_offset);
                        shell.invalidate_layout();
                        shell.request_redraw();
                    }
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    state.dragging = None;
                    shell.request_redraw();
                }
                _ => {}
            }

            shell.capture_event();
            return;
        }

        let over = cursor.position_over(bounds);

        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if over.is_some() => {
                let dy = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * self.row_height * WHEEL_ROWS,
                    mouse::ScrollDelta::Pixels { y, .. } => *y,
                };
                let offset = (state.offset - dy).clamp(0.0, max_offset);

                if offset != state.offset {
                    state.offset = offset;
                    shell.invalidate_layout();
                    shell.request_redraw();
                    shell.capture_event();
                    return;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let hot = over.is_some_and(|point| {
                    self.bar(bounds, state.offset, true)
                        .is_some_and(|(rail, _)| rail.expand(2.0).contains(point))
                });

                if hot != state.bar_hovered {
                    state.bar_hovered = hot;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let (Some(point), Some((rail, thumb))) =
                    (over, self.bar(bounds, state.offset, true))
                    && rail.expand(2.0).contains(point)
                {
                    // Tutamağın dışına basmak bir sayfa kaydırır.
                    if thumb.contains(point) {
                        state.dragging = Some((point.y, state.offset));
                    } else {
                        let page = if point.y < thumb.y {
                            -bounds.height
                        } else {
                            bounds.height
                        };
                        state.offset = (state.offset + page).clamp(0.0, max_offset);
                        shell.invalidate_layout();
                    }

                    shell.request_redraw();
                    shell.capture_event();
                    return;
                }
            }
            _ => {}
        }

        // Liste dışındaki (kırpılan) satır parçaları imleci görmez.
        let row_cursor = if over.is_some() {
            cursor
        } else {
            cursor.levitate()
        };
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };

        for (((_, element), (_, row_tree)), row_layout) in self
            .rows
            .iter_mut()
            .zip(state.trees.iter_mut())
            .zip(layout.children())
        {
            element.as_widget_mut().update(
                row_tree, event, row_layout, row_cursor, renderer, clipboard, shell, &clip,
            );

            if shell.is_event_captured() {
                return;
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

        if state.dragging.is_some() {
            return mouse::Interaction::Grabbing;
        }

        if state.bar_hovered {
            return mouse::Interaction::Idle;
        }

        if !cursor.is_over(bounds) {
            return mouse::Interaction::None;
        }

        self.rows
            .iter()
            .zip(&state.trees)
            .zip(layout.children())
            .map(|(((_, element), (_, row_tree)), row_layout)| {
                element
                    .as_widget()
                    .mouse_interaction(row_tree, row_layout, cursor, viewport, renderer)
            })
            .find(|interaction| *interaction != mouse::Interaction::None)
            .unwrap_or_default()
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
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };
        let row_cursor = if cursor.is_over(bounds) {
            cursor
        } else {
            cursor.levitate()
        };

        renderer.with_layer(clip, |renderer| {
            for (((_, element), (_, row_tree)), row_layout) in
                self.rows.iter().zip(&state.trees).zip(layout.children())
            {
                if row_layout.bounds().intersects(&clip) {
                    element.as_widget().draw(
                        row_tree, renderer, theme, style, row_layout, row_cursor, &clip,
                    );
                }
            }

            let hot = state.bar_hovered || state.dragging.is_some();

            if let Some((_, thumb)) = self.bar(bounds, state.offset, hot) {
                let t = Tokens::of(theme);

                renderer.fill_quad(
                    Quad {
                        bounds: thumb,
                        border: Border {
                            radius: (thumb.width / 2.0).into(),
                            ..Border::default()
                        },
                        ..Quad::default()
                    },
                    Background::Color(t.muted.scale_alpha(if hot { 0.7 } else { 0.45 })),
                );
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
        let state = tree.state.downcast_mut::<State>();

        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for (((_, element), (_, row_tree)), row_layout) in self
                .rows
                .iter_mut()
                .zip(state.trees.iter_mut())
                .zip(layout.children())
            {
                element
                    .as_widget_mut()
                    .operate(row_tree, row_layout, renderer, operation);
            }
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<State>();
        let overlays: Vec<_> = self
            .rows
            .iter_mut()
            .zip(state.trees.iter_mut())
            .zip(layout.children())
            .filter_map(|(((_, element), (_, row_tree)), row_layout)| {
                element.as_widget_mut().overlay(
                    row_tree,
                    row_layout,
                    renderer,
                    viewport,
                    translation,
                )
            })
            .collect();

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, Message: 'a> From<VirtualList<'a, Message>> for Element<'a, Message> {
    fn from(list: VirtualList<'a, Message>) -> Self {
        Element::new(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_rows_in_view_are_built() {
        // 24 piksellik satırlar, 100 piksellik alan.
        assert_eq!(visible(0.0, 100.0, 24.0, 1000), 0..5);
        assert_eq!(visible(30.0, 100.0, 24.0, 1000), 1..6);
        assert_eq!(visible(23_950.0, 100.0, 24.0, 1000), 997..1000);
        assert_eq!(visible(0.0, 100.0, 24.0, 2), 0..2);
    }

    #[test]
    fn revealed_rows_scroll_into_view() {
        // Görünür satır için kaydırma değişmez; üstteki ve alttaki satırlar
        // kenara gelir.
        assert_eq!(reveal(48.0, 100.0, 24.0, 3), 48.0);
        assert_eq!(reveal(48.0, 100.0, 24.0, 1), 24.0);
        assert_eq!(reveal(0.0, 100.0, 24.0, 10), 164.0);
    }
}
