//! Görünüm alanları: aynı modeli ya da haritayı birden çok görünümde
//! gösteren bölmeler.
//!
//! ```text
//! ┌[Üst ⌄][Tel kafes ⌄]───── ⤢ ┬[Ön ⌄][Gölgeli ⌄]──────── ⤢ ┐
//! │                            │                           │
//! ├[Sağ ⌄]────────────────── ⤢ ┼[Perspektif ⌄]─────────── ⤢ ┤
//! │                            │                           │
//! └────────────────────────────┴───────────────────────────┘
//! ```
//!
//! - **Düzen.** Tek, iki (yan yana ya da alt alta), üç (solda büyük) ya da
//!   dört görünüm ([`Arrangement`]).
//! - **Etkin görünüm.** Görünüme tıklamak onu etkin yapar; etkin görünüm
//!   vurgu renginde çerçevelenir. Uygulama komutları etkin görünüme uygular.
//! - **Başlık.** Sol üstteki menüler uygulamanındır (ör. bakış yönü ve görsel
//!   stil, [`View::menu`]); sağdaki düğme görünümü büyütür. Başlığa çift
//!   tıklamak da büyütür; yeniden basınca düzene dönülür.
//! - **Bölme.** Görünümler arasındaki çizgiler sürüklenerek oranlar değişir.
//!
//! Yerleşim uygulamanın durumudur ([`Views`]); bileşen değişiklikleri
//! [`Event`] olarak bildirir.
//!
//! ```ignore
//! Viewports::new(&self.views, Message::Views, |index| {
//!     View::new(self.scene(index))
//!         .menu(self.cameras[index].label(), move || self.camera_menu(index))
//!         .menu(self.styles[index].label(), move || self.style_menu(index))
//! })
//! ```

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::time::Instant;
use iced::widget::{Row, button, container, space, text};
use iced::{
    Background, Border, Center, Color, Element, Event as IcedEvent, Length, Point, Rectangle,
    Renderer, Size, Theme, Vector, mouse,
};

use crate::icon::{Icon, icon};
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::context_menu::{Menu, MenuButton};

/// Başlığın görünümün kenarından uzaklığı.
const INSET: f32 = 4.0;
/// Bölme oranlarının sınırları.
const MIN_SPLIT: f32 = 0.15;
/// Tutamağın çizginin iki yanındaki tutma payı.
const GRAB: f32 = 3.0;
/// Çift tık sayılan en uzun aralık.
const DOUBLE_CLICK: std::time::Duration = std::time::Duration::from_millis(400);

/// Görünümlerin düzeni.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Arrangement {
    #[default]
    Single,
    /// İki görünüm yan yana.
    Columns,
    /// İki görünüm alt alta.
    Rows,
    /// Solda büyük, sağda alt alta iki görünüm.
    Three,
    /// İki satır, iki sütun.
    Quad,
}

impl Arrangement {
    pub const ALL: [Arrangement; 5] = [
        Arrangement::Single,
        Arrangement::Columns,
        Arrangement::Rows,
        Arrangement::Three,
        Arrangement::Quad,
    ];

    /// Görünüm sayısı.
    pub fn count(self) -> usize {
        match self {
            Arrangement::Single => 1,
            Arrangement::Columns | Arrangement::Rows => 2,
            Arrangement::Three => 3,
            Arrangement::Quad => 4,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Arrangement::Single => "Tek görünüm",
            Arrangement::Columns => "İki görünüm, yan yana",
            Arrangement::Rows => "İki görünüm, alt alta",
            Arrangement::Three => "Üç görünüm",
            Arrangement::Quad => "Dört görünüm",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Arrangement::Single => Icon::ViewSingle,
            Arrangement::Columns => Icon::ViewColumns,
            Arrangement::Rows => Icon::ViewRows,
            Arrangement::Three => Icon::ViewThree,
            Arrangement::Quad => Icon::ViewQuad,
        }
    }
}

/// Görünüm alanlarının yerleşimi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Views {
    pub arrangement: Arrangement,
    /// Dikey ve yatay bölme çizgilerinin oranı (0–1).
    pub split: [f32; 2],
    /// Etkin görünüm.
    pub active: usize,
    /// Etkin görünüm bütün alanı kaplıyor.
    pub maximized: bool,
}

impl Default for Views {
    fn default() -> Self {
        Self::new(Arrangement::default())
    }
}

impl Views {
    pub fn new(arrangement: Arrangement) -> Self {
        Self {
            arrangement,
            split: [0.5, 0.5],
            active: 0,
            maximized: false,
        }
    }

    /// Bileşenin bildirdiği değişikliği uygular.
    pub fn update(&mut self, event: Event) {
        match event {
            Event::Activated(index) => {
                if index < self.arrangement.count() {
                    self.active = index;
                }
            }
            Event::Maximized(maximized) => self.maximized = maximized,
            Event::Split(split) => {
                self.split = split.map(|ratio| ratio.clamp(MIN_SPLIT, 1.0 - MIN_SPLIT));
            }
            Event::Arranged(arrangement) => {
                self.arrangement = arrangement;
                self.active = self.active.min(arrangement.count() - 1);
                self.maximized = false;
            }
        }
    }
}

/// Görünüm alanlarının bildirdiği değişiklik.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Activated(usize),
    Maximized(bool),
    Split([f32; 2]),
    Arranged(Arrangement),
}

/// Bir görünüm: içeriği ve başlıktaki menüleri.
pub struct View<'a, Message> {
    content: Element<'a, Message>,
    menus: Vec<Element<'a, Message>>,
}

impl<'a, Message: Clone + 'a> View<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            menus: Vec::new(),
        }
    }

    /// Başlığa menülü bir etiket ekler (ör. "Üst", "Tel kafes"); menü her
    /// açılışta kurulur.
    pub fn menu(mut self, label: impl Into<String>, menu: impl Fn() -> Menu<Message> + 'a) -> Self {
        let label: String = label.into();

        self.menus.push(
            MenuButton::new(
                container(
                    Row::new()
                        .push(
                            text(label)
                                .font(typography::ui())
                                .size(typography::caption()),
                        )
                        .push(icon(Icon::ChevronDown).size(9.0))
                        .spacing(4)
                        .align_y(Center),
                )
                .padding([2, 7])
                .style(chip),
                menu,
            )
            .into(),
        );
        self
    }
}

/// Görünümün üstündeki etiketin zemini: içeriğin üstünde okunur kalır.
fn chip(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);

    container::Style {
        background: Some(Background::Color(t.popover.scale_alpha(0.86))),
        border: Border {
            color: t.border.scale_alpha(0.6),
            width: 1.0,
            radius: 3.0.into(),
        },
        text_color: Some(t.text),
        ..container::Style::default()
    }
}

/// Görünüm alanları.
pub struct Viewports<'a, Message> {
    views: Views,
    /// Her görünümün içeriği ve başlığı.
    cells: Vec<[Element<'a, Message>; 2]>,
    on_event: Box<dyn Fn(Event) -> Message + 'a>,
}

impl<'a, Message: Clone + 'a> Viewports<'a, Message> {
    /// `view` her görünümün içeriğini ve menülerini sırasıyla kurar.
    pub fn new(
        views: &Views,
        on_event: impl Fn(Event) -> Message + 'a,
        view: impl Fn(usize) -> View<'a, Message>,
    ) -> Self {
        let views = *views;
        let count = views.arrangement.count();

        let cells = (0..count)
            .map(|index| {
                let View { content, menus } = view(index);
                let maximized = views.maximized && index == views.active;

                let toggle = button(
                    icon(if maximized {
                        Icon::Restore
                    } else {
                        Icon::Maximize
                    })
                    .size(12.0),
                )
                .on_press(on_event(Event::Maximized(!maximized)))
                .padding(4)
                .style(style::button::flat);

                let mut header = Row::new().spacing(4).align_y(Center);

                for menu in menus {
                    header = header.push(menu);
                }

                // Tek görünümde büyütülecek bir şey yoktur.
                let header = if count > 1 {
                    header
                        .push(space::horizontal())
                        .push(container(toggle).style(chip))
                } else {
                    header
                };

                [content, header.into()]
            })
            .collect();

        Self {
            views,
            cells,
            on_event: Box::new(on_event),
        }
    }
}

/// Yerleşimin tutamağı.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Line {
    bounds: Rectangle,
    /// Dikey çizgi (x oranını değiştirir).
    vertical: bool,
}

impl Line {
    fn grab(&self) -> Rectangle {
        if self.vertical {
            Rectangle {
                x: self.bounds.x - GRAB,
                width: self.bounds.width + 2.0 * GRAB,
                ..self.bounds
            }
        } else {
            Rectangle {
                y: self.bounds.y - GRAB,
                height: self.bounds.height + 2.0 * GRAB,
                ..self.bounds
            }
        }
    }
}

/// Görünümlerin yeri ve aralarındaki çizgiler; `size` içinde, çizgiler bir
/// piksel.
fn arrange(views: &Views, size: Size) -> (Vec<Rectangle>, Vec<Line>) {
    let count = views.arrangement.count();

    if views.maximized || count == 1 {
        let mut cells = vec![Rectangle::default(); count];
        cells[views.active.min(count - 1)] = Rectangle::with_size(size);

        return (cells, Vec::new());
    }

    let [sx, sy] = views
        .split
        .map(|ratio| ratio.clamp(MIN_SPLIT, 1.0 - MIN_SPLIT));
    let x = (size.width * sx).round();
    let y = (size.height * sy).round();
    let rect = |x0: f32, y0: f32, x1: f32, y1: f32| {
        Rectangle::new(
            Point::new(x0, y0),
            Size::new((x1 - x0).max(0.0), (y1 - y0).max(0.0)),
        )
    };
    let (w, h) = (size.width, size.height);
    let vertical = |from: f32, to: f32| Line {
        bounds: rect(x, from, x + 1.0, to),
        vertical: true,
    };
    let horizontal = |from: f32, to: f32| Line {
        bounds: rect(from, y, to, y + 1.0),
        vertical: false,
    };

    match views.arrangement {
        Arrangement::Single => (vec![Rectangle::with_size(size)], Vec::new()),
        Arrangement::Columns => (
            vec![rect(0.0, 0.0, x, h), rect(x + 1.0, 0.0, w, h)],
            vec![vertical(0.0, h)],
        ),
        Arrangement::Rows => (
            vec![rect(0.0, 0.0, w, y), rect(0.0, y + 1.0, w, h)],
            vec![horizontal(0.0, w)],
        ),
        Arrangement::Three => (
            vec![
                rect(0.0, 0.0, x, h),
                rect(x + 1.0, 0.0, w, y),
                rect(x + 1.0, y + 1.0, w, h),
            ],
            vec![vertical(0.0, h), horizontal(x + 1.0, w)],
        ),
        Arrangement::Quad => (
            vec![
                rect(0.0, 0.0, x, y),
                rect(x + 1.0, 0.0, w, y),
                rect(0.0, y + 1.0, x, h),
                rect(x + 1.0, y + 1.0, w, h),
            ],
            vec![vertical(0.0, h), horizontal(0.0, w)],
        ),
    }
}

#[derive(Debug, Default)]
struct State {
    cells: Vec<Rectangle>,
    lines: Vec<Line>,
    /// Sürüklenen çizgi.
    dragging: Option<usize>,
    hovered: Option<usize>,
    last_press: Option<(usize, Instant)>,
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Viewports<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.cells
            .iter()
            .map(|parts| Tree {
                tag: tree::Tag::stateless(),
                state: tree::State::None,
                children: parts.iter().map(Tree::new).collect(),
            })
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.children.resize_with(self.cells.len(), Tree::empty);

        for (cell, parts) in tree.children.iter_mut().zip(&self.cells) {
            cell.diff_children(parts);
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        let (cells, lines) = arrange(&self.views, size);
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();

        let nodes = self
            .cells
            .iter_mut()
            .zip(children.iter_mut())
            .zip(&cells)
            .map(|((parts, tree), bounds)| {
                if bounds.width <= 0.0 || bounds.height <= 0.0 {
                    return Node::with_children(Size::ZERO, vec![Node::default(), Node::default()]);
                }

                let content = parts[0].as_widget_mut().layout(
                    &mut tree.children[0],
                    renderer,
                    &layout::Limits::new(Size::ZERO, bounds.size()),
                );
                let header = parts[1]
                    .as_widget_mut()
                    .layout(
                        &mut tree.children[1],
                        renderer,
                        &layout::Limits::new(
                            Size::ZERO,
                            Size::new(
                                (bounds.width - 2.0 * INSET).max(0.0),
                                typography::scaled(24.0),
                            ),
                        ),
                    )
                    .move_to(Point::new(INSET, INSET));

                Node::with_children(bounds.size(), vec![content, header]).move_to(bounds.position())
            })
            .collect();

        state.cells = cells;
        state.lines = lines;

        Node::with_children(size, nodes)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &IcedEvent,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let origin = Vector::new(bounds.x, bounds.y);
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();

        // Bölme çizgisi sürükleniyor.
        if let Some(line) = state.dragging {
            match event {
                IcedEvent::Mouse(mouse::Event::CursorMoved { position }) => {
                    let vertical = state.lines.get(line).is_some_and(|line| line.vertical);
                    let mut split = self.views.split;

                    if vertical {
                        split[0] = (position.x - bounds.x) / bounds.width.max(1.0);
                    } else {
                        split[1] = (position.y - bounds.y) / bounds.height.max(1.0);
                    }

                    let split = split.map(|ratio| ratio.clamp(MIN_SPLIT, 1.0 - MIN_SPLIT));

                    if split != self.views.split {
                        shell.publish((self.on_event)(Event::Split(split)));
                    }
                }
                IcedEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    state.dragging = None;
                    shell.request_redraw();
                }
                _ => {}
            }

            shell.capture_event();
            return;
        }

        let point = cursor.position_over(bounds).map(|point| point - origin);
        let line = point.and_then(|point| {
            state
                .lines
                .iter()
                .position(|line| line.grab().contains(point))
        });

        if let IcedEvent::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft) = event
            && line != state.hovered
        {
            state.hovered = line;
            shell.request_redraw();
        }

        if let IcedEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            if let Some(line) = line {
                state.dragging = Some(line);
                shell.capture_event();
                shell.request_redraw();
                return;
            }

            // Görünüme basmak onu etkin yapar; basış içeriğe de gider.
            if let Some(index) = point.and_then(|point| {
                state
                    .cells
                    .iter()
                    .position(|cell| cell.width > 0.0 && cell.contains(point))
            }) && index != self.views.active
            {
                shell.publish((self.on_event)(Event::Activated(index)));
            }
        }

        let count = self.cells.len();

        for ((parts, tree), cell_layout) in self
            .cells
            .iter_mut()
            .zip(children.iter_mut())
            .zip(layout.children())
        {
            if cell_layout.bounds().width <= 0.0 {
                continue;
            }

            let mut layouts = cell_layout.children();
            let (Some(content_layout), Some(header_layout)) = (layouts.next(), layouts.next())
            else {
                continue;
            };

            parts[1].as_widget_mut().update(
                &mut tree.children[1],
                event,
                header_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );

            if shell.is_event_captured() {
                return;
            }

            // Başlığın boş yerine çift tık görünümü büyütür ya da geri alır.
            if let IcedEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
                && count > 1
                && cursor.is_over(header_layout.bounds())
            {
                let index = state
                    .cells
                    .iter()
                    .position(|cell| cell.position() + origin == cell_layout.bounds().position())
                    .unwrap_or(0);
                let now = Instant::now();
                let double = state.last_press.is_some_and(|(last, time)| {
                    last == index && now.duration_since(time) <= DOUBLE_CLICK
                });

                if double {
                    state.last_press = None;
                    shell.publish((self.on_event)(Event::Maximized(!self.views.maximized)));
                    shell.capture_event();
                    return;
                }

                state.last_press = Some((index, now));
            }

            parts[0].as_widget_mut().update(
                &mut tree.children[0],
                event,
                content_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
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
        let line = state.dragging.or_else(|| {
            cursor.position_over(bounds).and_then(|point| {
                let point = point - Vector::new(bounds.x, bounds.y);

                state
                    .lines
                    .iter()
                    .position(|line| line.grab().contains(point))
            })
        });

        if let Some(line) = line.and_then(|line| state.lines.get(line)) {
            return if line.vertical {
                mouse::Interaction::ResizingHorizontally
            } else {
                mouse::Interaction::ResizingVertically
            };
        }

        self.cells
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .filter(|(_, cell)| cell.bounds().width > 0.0 && cursor.is_over(cell.bounds()))
            .map(|((parts, tree), cell)| {
                let mut layouts = cell.children();
                let content = layouts.next();
                let header = layouts.next();

                let over_header = header.map(|header| {
                    parts[1].as_widget().mouse_interaction(
                        &tree.children[1],
                        header,
                        cursor,
                        viewport,
                        renderer,
                    )
                });

                match over_header {
                    Some(mouse::Interaction::None) | None => {
                        content.map_or(mouse::Interaction::None, |content| {
                            parts[0].as_widget().mouse_interaction(
                                &tree.children[0],
                                content,
                                cursor,
                                viewport,
                                renderer,
                            )
                        })
                    }
                    Some(interaction) => interaction,
                }
            })
            .next()
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
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let origin = Vector::new(bounds.x, bounds.y);
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };

        for (index, ((parts, tree), cell)) in self
            .cells
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .enumerate()
        {
            let cell_bounds = cell.bounds();

            if cell_bounds.width <= 0.0 {
                continue;
            }

            let Some(cell_clip) = cell_bounds.intersection(&clip) else {
                continue;
            };
            let mut layouts = cell.children();
            let (Some(content), Some(header)) = (layouts.next(), layouts.next()) else {
                continue;
            };

            renderer.with_layer(cell_clip, |renderer| {
                parts[0].as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    content,
                    cursor,
                    &cell_clip,
                );
            });

            renderer.with_layer(cell_clip, |renderer| {
                parts[1].as_widget().draw(
                    &tree.children[1],
                    renderer,
                    theme,
                    &renderer::Style { text_color: t.text },
                    header,
                    cursor,
                    &cell_clip,
                );

                // Etkin görünümün çerçevesi.
                if index == self.views.active && self.cells.len() > 1 && !self.views.maximized {
                    renderer.fill_quad(
                        Quad {
                            bounds: cell_bounds,
                            border: Border {
                                color: t.accent,
                                width: 1.0,
                                radius: 0.0.into(),
                            },
                            ..Quad::default()
                        },
                        Background::Color(Color::TRANSPARENT),
                    );
                }
            });
        }

        // Görünümler arasındaki çizgiler; sürüklenen ya da üzerine gelinen
        // vurgulanır.
        renderer.with_layer(clip, |renderer| {
            for (index, line) in state.lines.iter().enumerate() {
                let hot = state.dragging == Some(index)
                    || (state.dragging.is_none() && state.hovered == Some(index));
                let bounds = line.bounds + origin;
                let thick = match (hot, line.vertical) {
                    (true, true) => Rectangle {
                        x: bounds.x - 1.0,
                        width: 3.0,
                        ..bounds
                    },
                    (true, false) => Rectangle {
                        y: bounds.y - 1.0,
                        height: 3.0,
                        ..bounds
                    },
                    (false, _) => bounds,
                };

                renderer.fill_quad(
                    Quad {
                        bounds: thick,
                        ..Quad::default()
                    },
                    Background::Color(if hot { t.accent } else { t.border }),
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
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((parts, tree), cell) in self
                .cells
                .iter_mut()
                .zip(tree.children.iter_mut())
                .zip(layout.children())
            {
                if cell.bounds().width <= 0.0 {
                    continue;
                }

                for ((part, tree), layout) in parts
                    .iter_mut()
                    .zip(tree.children.iter_mut())
                    .zip(cell.children())
                {
                    part.as_widget_mut()
                        .operate(tree, layout, renderer, operation);
                }
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
        let mut overlays = Vec::new();

        for ((parts, tree), cell) in self
            .cells
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
        {
            if cell.bounds().width <= 0.0 {
                continue;
            }

            for ((part, tree), layout) in parts
                .iter_mut()
                .zip(tree.children.iter_mut())
                .zip(cell.children())
            {
                if let Some(overlay) =
                    part.as_widget_mut()
                        .overlay(tree, layout, renderer, viewport, translation)
                {
                    overlays.push(overlay);
                }
            }
        }

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, Message: Clone + 'a> From<Viewports<'a, Message>> for Element<'a, Message> {
    fn from(viewports: Viewports<'a, Message>) -> Self {
        Element::new(viewports)
    }
}

/// Düzen seçici: düzenlerin ikonlarından oluşan küçük düğme dizisi.
pub fn arrangements<'a, Message: Clone + 'a>(
    current: Arrangement,
    on_select: impl Fn(Arrangement) -> Message + 'a,
) -> Element<'a, Message> {
    Arrangement::ALL
        .into_iter()
        .fold(Row::new().spacing(2), |row, arrangement| {
            row.push(crate::widget::tip(
                button(icon(arrangement.icon()).size(16.0))
                    .on_press(on_select(arrangement))
                    .padding(4)
                    .style(style::button::tool(arrangement == current)),
                crate::widget::Tip::new(arrangement.label()),
                iced::widget::tooltip::Position::Bottom,
            ))
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Size = Size::new(1001.0, 601.0);

    #[test]
    fn arrangements_split_the_area() {
        let mut views = Views::new(Arrangement::Quad);
        let (cells, lines) = arrange(&views, SIZE);

        assert_eq!(cells.len(), 4);
        assert_eq!(
            cells[0],
            Rectangle::new(Point::ORIGIN, Size::new(501.0, 301.0))
        );
        assert_eq!(cells[3].position(), Point::new(502.0, 302.0));
        assert_eq!(lines.len(), 2);

        // Üçlü düzende yatay çizgi yalnızca sağ sütundadır.
        views.update(Event::Arranged(Arrangement::Three));
        views.update(Event::Split([0.6, 0.3]));
        let (cells, lines) = arrange(&views, SIZE);
        assert_eq!(cells[0].height, SIZE.height);
        assert_eq!(cells[1].x, cells[2].x);
        assert_eq!(lines[1].bounds.x, cells[1].x);

        // Büyütülen görünüm bütün alanı kaplar, öbürleri gizlenir.
        views.update(Event::Activated(2));
        views.update(Event::Maximized(true));
        let (cells, lines) = arrange(&views, SIZE);
        assert_eq!(cells[2], Rectangle::with_size(SIZE));
        assert!(cells[0].width == 0.0 && lines.is_empty());

        // Düzen değişince büyütme biter, etkin görünüm sığar.
        views.update(Event::Arranged(Arrangement::Columns));
        assert!(!views.maximized);
        assert_eq!(views.active, 1);

        // Oranlar sınırda tutulur.
        views.update(Event::Split([0.01, 2.0]));
        assert_eq!(views.split, [MIN_SPLIT, 1.0 - MIN_SPLIT]);
    }
}
