//! Cetveller ve kılavuz çizgileri: içeriğin üstünde ve solunda birimli
//! cetveller, cetvelden sürüklenen kılavuzlar.
//!
//! ```text
//! ┌──┬──────┬─────────┬─────────┬─────────┬──┐
//! │mm│0     │50       │100      │150      │  │   üst cetvel
//! ├──┼──────┴─────────┴──╎──────┴─────────┴──┤
//! │ 0│                   ╎                   │
//! │  │                   ╎  dikey kılavuz    │
//! │ 5│───────────────────╎───────────────────│   yatay kılavuz
//! │ 0│                   ╎                   │
//! └──┴───────────────────────────────────────┘
//! ```
//!
//! Tasarım ve pafta programlarındaki gibi: cetveller içeriğin birimini
//! (ör. kâğıtta milimetre) gösterir; yakınlaştıkça aralıklar 1, 2, 5 × 10ⁿ
//! adımlarla sıklaşır. İmlecin yeri iki cetvelde de ince çizgiyle
//! işaretlenir. Üst cetvelden aşağı sürüklemek yatay, sol cetvelden sağa
//! sürüklemek dikey kılavuz çıkarır. Kılavuz sürüklenerek taşınır, cetvele
//! geri bırakılınca silinir; Shift basılıyken cetvelin küçük çizgilerine
//! oturur, Esc sürüklemeyi bırakır. Sürüklerken değer imlecin yanında
//! yazar.
//!
//! Birimle içerik arasındaki dönüşüm uygulamanındır ([`Transform`]): sıfır
//! noktasının içerikteki yeri ve birim başına piksel. Kılavuzlar birim
//! cinsinden saklanır ([`Guides`]); görünüm değişince yerlerinde kalırlar.
//!
//! ```ignore
//! Rulers::new(sheet, Transform::new(paper_origin, pixels_per_mm))
//!     .unit("mm")
//!     .guides(&self.guides, Message::Guide)
//!
//! // update
//! Message::Guide(event) => self.guides.update(event),
//! ```

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::text::{self, Renderer as _, Text};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::alignment::Vertical;
use iced::keyboard::{self, key};
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::{
    Background, Border, Color, Element, Length, Pixels, Point, Rectangle, Renderer, Shadow, Size,
    Theme, Vector, mouse,
};

use crate::attribute::number;
use crate::theme::{Tokens, typography};

/// Cetvelin kalınlığı, 12 piksellik gövde metnine göre.
const THICKNESS: f32 = 18.0;
/// Etiketli çizgiler arasında en az bu kadar piksel olur.
const LABEL_SPACING: f32 = 56.0;
/// Küçük çizgiler arasında en az bu kadar piksel olur.
const TICK_SPACING: f32 = 5.0;
/// Kılavuzu tutmak için imlecin çizgiye en fazla uzaklığı.
const GRAB: f32 = 4.0;

/// Cetvelin kalınlığı, geçerli yazı boyutunda.
pub fn thickness() -> f32 {
    typography::scaled(THICKNESS).round()
}

/// Kılavuzun yönü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// Yatay çizgi (sabit y); üst cetvelden sürüklenir.
    Horizontal,
    /// Dikey çizgi (sabit x); sol cetvelden sürüklenir.
    Vertical,
}

/// Kılavuz çizgisi: yönü ve birim cinsinden yeri.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Guide {
    pub orientation: Orientation,
    pub value: f32,
}

impl Guide {
    /// `y` yüksekliğinde yatay kılavuz.
    pub fn horizontal(y: f32) -> Self {
        Self {
            orientation: Orientation::Horizontal,
            value: y,
        }
    }

    /// `x` yerinde dikey kılavuz.
    pub fn vertical(x: f32) -> Self {
        Self {
            orientation: Orientation::Vertical,
            value: x,
        }
    }
}

/// Kılavuzlarda yapılan değişiklik.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// Cetvelden sürüklenen yeni kılavuz içeriğe bırakıldı.
    Added(Guide),
    /// Kılavuz (sırası) yeni yerine taşındı.
    Moved(usize, f32),
    /// Kılavuz cetvele geri bırakıldı.
    Removed(usize),
}

/// Uygulamanın kılavuzları.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Guides(Vec<Guide>);

impl Guides {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, guide: Guide) {
        self.0.push(guide);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Guide> {
        self.0.iter()
    }

    pub fn as_slice(&self) -> &[Guide] {
        &self.0
    }

    /// Cetvelin bildirdiği değişikliği uygular.
    pub fn update(&mut self, event: Event) {
        match event {
            Event::Added(guide) => self.0.push(guide),
            Event::Moved(index, value) => {
                if let Some(guide) = self.0.get_mut(index) {
                    guide.value = value;
                }
            }
            Event::Removed(index) => {
                if index < self.0.len() {
                    self.0.remove(index);
                }
            }
        }
    }
}

/// Birimlerle içerik arasındaki dönüşüm: sıfır noktasının içeriğin sol üst
/// köşesine göre yeri ve birim başına piksel. `y_up` ise y yukarı doğru
/// artar (CAD'de olduğu gibi); yoksa aşağı doğru (kâğıtta olduğu gibi).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub origin: Point,
    pub scale: f32,
    pub y_up: bool,
}

impl Transform {
    /// y aşağı doğru artar.
    pub fn new(origin: Point, scale: f32) -> Self {
        Self {
            origin,
            scale: scale.max(f32::EPSILON),
            y_up: false,
        }
    }

    /// y yukarı doğru artar.
    pub fn y_up(mut self) -> Self {
        self.y_up = true;
        self
    }

    /// İçerikteki yerin birim karşılığı.
    pub fn to_units(&self, point: Point) -> Point {
        Point::new(
            (point.x - self.origin.x) / self.scale,
            if self.y_up {
                (self.origin.y - point.y) / self.scale
            } else {
                (point.y - self.origin.y) / self.scale
            },
        )
    }

    /// Birim cinsinden yerin içerikteki karşılığı.
    pub fn to_content(&self, point: Point) -> Point {
        Point::new(
            self.origin.x + point.x * self.scale,
            if self.y_up {
                self.origin.y - point.y * self.scale
            } else {
                self.origin.y + point.y * self.scale
            },
        )
    }

    /// Kılavuzun içerikteki yeri: yataysa y, dikeyse x.
    fn guide_position(&self, guide: Guide) -> f32 {
        match guide.orientation {
            Orientation::Horizontal => self.to_content(Point::new(0.0, guide.value)).y,
            Orientation::Vertical => self.to_content(Point::new(guide.value, 0.0)).x,
        }
    }

    /// İçerikteki yerin, kılavuzun yönünde birim karşılığı.
    fn guide_value(&self, orientation: Orientation, point: Point) -> f32 {
        let units = self.to_units(point);

        match orientation {
            Orientation::Horizontal => units.y,
            Orientation::Vertical => units.x,
        }
    }
}

/// Etiketli çizgilerin aralığı (birim) ve iki etiket arasındaki bölüm
/// sayısı. Etiketler en az `LABEL_SPACING`, küçük çizgiler en az
/// `TICK_SPACING` piksel aralıklıdır; aralık 1, 2 ya da 5 × 10ⁿ birimdir.
fn ticks(scale: f32) -> (f32, u32) {
    let raw = LABEL_SPACING / scale.max(f32::EPSILON);
    let base = 10f32.powf(raw.log10().floor());
    let (nice, parts): (f32, &[u32]) = match raw / base {
        mantissa if mantissa <= 1.0 => (1.0, &[10, 5, 2]),
        mantissa if mantissa <= 2.0 => (2.0, &[4, 2]),
        mantissa if mantissa <= 5.0 => (5.0, &[5]),
        _ => (10.0, &[10, 5, 2]),
    };
    let major = nice * base;
    let parts = parts
        .iter()
        .copied()
        .find(|parts| major * scale / *parts as f32 >= TICK_SPACING)
        .unwrap_or(1);

    (major, parts)
}

/// Etiketteki ondalık basamak sayısı: aralık 1'den küçükse gereken kadar.
fn decimals(major: f32) -> usize {
    if major >= 1.0 {
        0
    } else {
        (-major.log10() - 1e-3).ceil().max(0.0) as usize
    }
}

/// Shift basılıyken kılavuz küçük çizgilere oturur.
fn snap(value: f32, scale: f32) -> f32 {
    let (major, parts) = ticks(scale);
    let step = major / parts as f32;

    (value / step).round() * step
}

/// Cetveller.
pub struct Rulers<'a, Message> {
    content: Element<'a, Message>,
    transform: Transform,
    unit: String,
    guides: &'a [Guide],
    on_event: Option<Box<dyn Fn(Event) -> Message + 'a>>,
}

impl<'a, Message: 'a> Rulers<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>, transform: Transform) -> Self {
        Self {
            content: content.into(),
            transform,
            unit: String::new(),
            guides: &[],
            on_event: None,
        }
    }

    /// Köşede yazan birim (ör. "mm").
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    /// Kılavuzlar; eklenip taşınıp silinince `on_event` gönderilir.
    pub fn guides(mut self, guides: &'a Guides, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        self.guides = guides.as_slice();
        self.on_event = Some(Box::new(on_event));
        self
    }
}

/// Sürüklenen kılavuz: var olanın sırası ya da yeni, yönü ve değeri.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Drag {
    index: Option<usize>,
    orientation: Orientation,
    value: f32,
    /// İmleç içeriğin üstünde mi; değilse bırakmak siler ya da vazgeçer.
    inside: bool,
}

#[derive(Debug, Default)]
struct State {
    drag: Option<Drag>,
    hovered: Option<usize>,
    shift: bool,
    /// İmlecin içerikteki yeri; cetvellerde işaretlenir.
    cursor: Option<Point>,
}

/// Cetvellerin ve içeriğin yerleri.
struct Areas {
    corner: Rectangle,
    top: Rectangle,
    left: Rectangle,
    content: Rectangle,
}

fn areas(bounds: Rectangle) -> Areas {
    let size = thickness();

    Areas {
        corner: Rectangle::new(bounds.position(), Size::new(size, size)),
        top: Rectangle::new(
            Point::new(bounds.x + size, bounds.y),
            Size::new((bounds.width - size).max(0.0), size),
        ),
        left: Rectangle::new(
            Point::new(bounds.x, bounds.y + size),
            Size::new(size, (bounds.height - size).max(0.0)),
        ),
        content: Rectangle::new(
            Point::new(bounds.x + size, bounds.y + size),
            Size::new(
                (bounds.width - size).max(0.0),
                (bounds.height - size).max(0.0),
            ),
        ),
    }
}

impl<'a, Message: 'a> Rulers<'a, Message> {
    /// İmlecin altındaki kılavuz.
    fn guide_at(&self, content: Rectangle, point: Point) -> Option<usize> {
        if !content.contains(point) {
            return None;
        }

        let local = Point::new(point.x - content.x, point.y - content.y);

        self.guides
            .iter()
            .enumerate()
            .map(|(index, guide)| {
                let position = self.transform.guide_position(*guide);
                let distance = match guide.orientation {
                    Orientation::Horizontal => (local.y - position).abs(),
                    Orientation::Vertical => (local.x - position).abs(),
                };

                (index, distance)
            })
            .filter(|(_, distance)| *distance <= GRAB)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }

    fn drag_value(
        &self,
        state: &State,
        orientation: Orientation,
        content: Rectangle,
        point: Point,
    ) -> f32 {
        let local = Point::new(point.x - content.x, point.y - content.y);
        let value = self.transform.guide_value(orientation, local);

        if state.shift {
            snap(value, self.transform.scale)
        } else {
            value
        }
    }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Rulers<'a, Message> {
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
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let size = limits.max();
        let ruler = thickness();
        let inner = Size::new(
            (size.width - ruler).max(0.0),
            (size.height - ruler).max(0.0),
        );
        let content = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, inner),
        );

        Node::with_children(size, vec![content.move_to(Point::new(ruler, ruler))])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Some(content_layout) = layout.children().next() else {
            return;
        };
        let areas = areas(layout.bounds());
        let content = content_layout.bounds();
        let state = tree.state.downcast_mut::<State>();

        if let iced::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
            state.shift = modifiers.shift();
        }

        if let iced::Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
            let inside = cursor
                .position_over(content)
                .map(|point| Point::new(point.x - content.x, point.y - content.y));

            if inside != state.cursor {
                state.cursor = inside;
                shell.request_redraw();
            }
        }

        if let Some(on_event) = &self.on_event {
            if let Some(mut drag) = state.drag {
                match event {
                    iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                        drag.value = self.drag_value(state, drag.orientation, content, *position);
                        drag.inside = content.contains(*position);
                        state.drag = Some(drag);
                        shell.request_redraw();
                        shell.capture_event();
                        return;
                    }
                    iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                        state.drag = None;

                        let message = match (drag.index, drag.inside) {
                            (None, true) => Some(Event::Added(Guide {
                                orientation: drag.orientation,
                                value: drag.value,
                            })),
                            (Some(index), true) => Some(Event::Moved(index, drag.value)),
                            (Some(index), false) => Some(Event::Removed(index)),
                            (None, false) => None,
                        };

                        if let Some(message) = message {
                            shell.publish(on_event(message));
                        }

                        shell.request_redraw();
                        shell.capture_event();
                        return;
                    }
                    iced::Event::Keyboard(keyboard::Event::KeyPressed {
                        key: keyboard::Key::Named(key::Named::Escape),
                        ..
                    }) => {
                        state.drag = None;
                        shell.request_redraw();
                        shell.capture_event();
                        return;
                    }
                    _ => {}
                }
            } else {
                match event {
                    iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                        let hovered = self.guide_at(content, *position);

                        if hovered != state.hovered {
                            state.hovered = hovered;
                            shell.request_redraw();
                        }
                    }
                    iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                        let start = cursor.position().and_then(|point| {
                            let start = if areas.top.contains(point) {
                                Some((None, Orientation::Horizontal))
                            } else if areas.left.contains(point) {
                                Some((None, Orientation::Vertical))
                            } else {
                                self.guide_at(content, point)
                                    .map(|index| (Some(index), self.guides[index].orientation))
                            };

                            start.map(|(index, orientation)| (index, orientation, point))
                        });

                        if let Some((index, orientation, point)) = start {
                            state.drag = Some(Drag {
                                index,
                                orientation,
                                value: self.drag_value(state, orientation, content, point),
                                inside: content.contains(point),
                            });
                            shell.request_redraw();
                            shell.capture_event();
                            return;
                        }
                    }
                    _ => {}
                }
            }
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            content_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
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
        let resize = |orientation| match orientation {
            Orientation::Horizontal => mouse::Interaction::ResizingVertically,
            Orientation::Vertical => mouse::Interaction::ResizingHorizontally,
        };

        if let Some(drag) = state.drag {
            return resize(drag.orientation);
        }

        let areas = areas(layout.bounds());

        if self.on_event.is_some()
            && let Some(point) = cursor.position()
        {
            if areas.top.contains(point) {
                return resize(Orientation::Horizontal);
            }

            if areas.left.contains(point) {
                return resize(Orientation::Vertical);
            }

            if let Some(index) = self.guide_at(areas.content, point) {
                return resize(self.guides[index].orientation);
            }
        }

        layout
            .children()
            .next()
            .map_or(mouse::Interaction::None, |content| {
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
        let Some(content_layout) = layout.children().next() else {
            return;
        };
        let state = tree.state.downcast_ref::<State>();
        let t = Tokens::of(theme);
        let areas = areas(layout.bounds());
        let content = content_layout.bounds();

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            content_layout,
            if state.drag.is_some() {
                cursor.levitate()
            } else {
                cursor
            },
            viewport,
        );

        let transform = self.transform;
        let (major, parts) = ticks(transform.scale);
        let decimals = decimals(major);
        let step = major / parts as f32;
        let font = typography::mono();
        let size = Pixels(typography::scaled(9.0).round());
        let ruler = thickness();

        // Kılavuzlar içeriğin üstünde; sürüklenen, eski yerinin yerine.
        renderer.with_layer(content, |renderer| {
            let line =
                |renderer: &mut Renderer, orientation, position: f32, color: Color, width: f32| {
                    let bounds = match orientation {
                        Orientation::Horizontal => Rectangle::new(
                            Point::new(content.x, content.y + position - width / 2.0),
                            Size::new(content.width, width),
                        ),
                        Orientation::Vertical => Rectangle::new(
                            Point::new(content.x + position - width / 2.0, content.y),
                            Size::new(width, content.height),
                        ),
                    };

                    renderer.fill_quad(
                        Quad {
                            bounds,
                            ..Quad::default()
                        },
                        Background::Color(color),
                    );
                };

            for (index, guide) in self.guides.iter().enumerate() {
                if state.drag.is_some_and(|drag| drag.index == Some(index)) {
                    continue;
                }

                let hovered = state.hovered == Some(index) && state.drag.is_none();

                line(
                    renderer,
                    guide.orientation,
                    transform.guide_position(*guide),
                    t.accent.scale_alpha(if hovered { 1.0 } else { 0.75 }),
                    if hovered { 2.0 } else { 1.0 },
                );
            }

            if let Some(drag) = state.drag {
                let position = transform.guide_position(Guide {
                    orientation: drag.orientation,
                    value: drag.value,
                });
                // İçeriğin dışındayken bırakmak siler: çizgi soluklaşır.
                let color = if drag.inside {
                    t.accent
                } else {
                    t.accent.scale_alpha(0.35)
                };

                line(renderer, drag.orientation, position, color, 1.0);
            }
        });

        renderer.with_layer(layout.bounds(), |renderer| {
            let background = t.header;

            for area in [areas.top, areas.left, areas.corner] {
                renderer.fill_quad(
                    Quad {
                        bounds: area,
                        ..Quad::default()
                    },
                    Background::Color(background),
                );
            }

            // Cetvellerin içeriğe bakan kenarları.
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(
                        Point::new(areas.corner.x, areas.top.y + ruler - 1.0),
                        Size::new(areas.corner.width + areas.top.width, 1.0),
                    ),
                    ..Quad::default()
                },
                Background::Color(t.border),
            );
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(
                        Point::new(areas.left.x + ruler - 1.0, areas.corner.y),
                        Size::new(1.0, areas.corner.height + areas.left.height),
                    ),
                    ..Quad::default()
                },
                Background::Color(t.border),
            );

            let tick = |renderer: &mut Renderer, bounds: Rectangle| {
                renderer.fill_quad(
                    Quad {
                        bounds,
                        ..Quad::default()
                    },
                    Background::Color(t.muted.scale_alpha(0.8)),
                );
            };
            // Etiketin sığacağı alan: iki etiket arası; sonsuz alan metni
            // harf harf kırar.
            let room = Size::new((major * transform.scale).max(ruler), ruler);
            let label =
                |renderer: &mut Renderer, content: String, position: Point, clip: Rectangle| {
                    renderer.fill_text(
                        Text {
                            content,
                            bounds: room,
                            size,
                            line_height: LineHeight::Absolute(size),
                            font,
                            align_x: text::Alignment::Left,
                            align_y: Vertical::Top,
                            shaping: Shaping::Basic,
                            wrapping: Wrapping::None,
                        },
                        position,
                        t.muted,
                        clip,
                    );
                };
            // Bölümün boyu: etiketli çizgi tam, yarısı orta, öbürleri kısa.
            let length = |part: i64| {
                if part.rem_euclid(parts as i64) == 0 {
                    ruler - 1.0
                } else if parts % 2 == 0 && part.rem_euclid(parts as i64) == parts as i64 / 2 {
                    (ruler * 0.45).round()
                } else {
                    (ruler * 0.25).round()
                }
            };

            // Üst cetvel: soldan sağa birim.
            renderer.with_layer(areas.top, |renderer| {
                let from = transform.to_units(Point::new(0.0, 0.0)).x;
                let to = transform.to_units(Point::new(content.width, 0.0)).x;
                let (low, high) = (from.min(to), from.max(to));
                let first = (low / step).floor() as i64;
                let last = (high / step).ceil() as i64;

                for part in first..=last {
                    let value = part as f32 * step;
                    let x = content.x + transform.to_content(Point::new(value, 0.0)).x;
                    let height = length(part);

                    tick(
                        renderer,
                        Rectangle::new(
                            Point::new(x.round(), areas.top.y + ruler - 1.0 - height),
                            Size::new(1.0, height),
                        ),
                    );

                    if part.rem_euclid(parts as i64) == 0 {
                        label(
                            renderer,
                            number::real(f64::from(value), decimals),
                            Point::new(x.round() + 3.0, areas.top.y + 2.0),
                            areas.top,
                        );
                    }
                }
            });

            // Sol cetvel: rakamlar alt alta.
            renderer.with_layer(areas.left, |renderer| {
                let from = transform.to_units(Point::new(0.0, 0.0)).y;
                let to = transform.to_units(Point::new(0.0, content.height)).y;
                let (low, high) = (from.min(to), from.max(to));
                let first = (low / step).floor() as i64;
                let last = (high / step).ceil() as i64;
                let line_height = size.0;

                for part in first..=last {
                    let value = part as f32 * step;
                    let y = content.y + transform.to_content(Point::new(0.0, value)).y;
                    let width = length(part);

                    tick(
                        renderer,
                        Rectangle::new(
                            Point::new(areas.left.x + ruler - 1.0 - width, y.round()),
                            Size::new(width, 1.0),
                        ),
                    );

                    if part.rem_euclid(parts as i64) == 0 {
                        let text = number::real(f64::from(value), decimals);

                        for (row, character) in text.chars().enumerate() {
                            label(
                                renderer,
                                character.to_string(),
                                Point::new(
                                    areas.left.x + 2.0,
                                    y.round() + 2.0 + row as f32 * line_height,
                                ),
                                areas.left,
                            );
                        }
                    }
                }
            });

            // Kılavuzların cetveldeki yerleri ve imlecin yeri.
            let mark = |renderer: &mut Renderer, orientation, position: f32, color: Color| {
                let bounds = match orientation {
                    Orientation::Vertical => Rectangle::new(
                        Point::new(content.x + position.round(), areas.top.y),
                        Size::new(1.0, ruler - 1.0),
                    ),
                    Orientation::Horizontal => Rectangle::new(
                        Point::new(areas.left.x, content.y + position.round()),
                        Size::new(ruler - 1.0, 1.0),
                    ),
                };

                let clip = match orientation {
                    Orientation::Vertical => areas.top,
                    Orientation::Horizontal => areas.left,
                };

                renderer.with_layer(clip, |renderer| {
                    renderer.fill_quad(
                        Quad {
                            bounds,
                            ..Quad::default()
                        },
                        Background::Color(color),
                    );
                });
            };

            for guide in self.guides {
                mark(
                    renderer,
                    guide.orientation,
                    transform.guide_position(*guide),
                    t.accent.scale_alpha(0.6),
                );
            }

            if let Some(point) = state.cursor {
                mark(renderer, Orientation::Vertical, point.x, t.accent);
                mark(renderer, Orientation::Horizontal, point.y, t.accent);
            }

            // Köşede birim.
            if !self.unit.is_empty() {
                renderer.fill_text(
                    Text {
                        content: self.unit.clone(),
                        bounds: areas.corner.size(),
                        size,
                        line_height: LineHeight::Absolute(size),
                        font,
                        align_x: text::Alignment::Center,
                        align_y: Vertical::Center,
                        shaping: Shaping::Basic,
                        wrapping: Wrapping::None,
                    },
                    Point::new(areas.corner.center_x(), areas.corner.center_y()),
                    t.muted,
                    areas.corner,
                );
            }
        });

        // Sürüklenen kılavuzun değeri, imlecin yanında.
        if let (Some(drag), Some(point)) = (state.drag, cursor.position()) {
            let text = format!(
                "{} {}",
                number::real(f64::from(drag.value), decimals + 1),
                self.unit
            );
            let text = text.trim().to_owned();
            let width = text.chars().count() as f32 * size.0 * 0.62 + 12.0;
            let height = size.0 + 10.0;
            let bounds = layout.bounds();
            let x = (point.x + 14.0).min(bounds.x + bounds.width - width - 2.0);
            let y = (point.y + 16.0).min(bounds.y + bounds.height - height - 2.0);
            let tip = Rectangle::new(Point::new(x, y), Size::new(width, height));

            renderer.with_layer(bounds, |renderer| {
                renderer.fill_quad(
                    Quad {
                        bounds: tip,
                        border: Border {
                            color: t.border,
                            width: 1.0,
                            radius: 3.0.into(),
                        },
                        shadow: Shadow {
                            color: t.shadow(),
                            offset: Vector::new(0.0, 2.0),
                            blur_radius: 6.0,
                        },
                        ..Quad::default()
                    },
                    Background::Color(t.popover),
                );
                renderer.fill_text(
                    Text {
                        content: text,
                        bounds: tip.size(),
                        size,
                        line_height: LineHeight::Absolute(size),
                        font,
                        align_x: text::Alignment::Center,
                        align_y: Vertical::Center,
                        shaping: Shaping::Basic,
                        wrapping: Wrapping::None,
                    },
                    Point::new(tip.center_x(), tip.center_y()),
                    if drag.inside { t.text } else { t.muted },
                    tip,
                );
            });
        }
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

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let content = layout.children().next()?;

        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            content,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<Rulers<'a, Message>> for Element<'a, Message> {
    fn from(rulers: Rulers<'a, Message>) -> Self {
        Element::new(rulers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_step_by_one_two_and_five() {
        // Milimetre başına 2 piksel: 56 piksel 28 mm eder, aralık 50 mm.
        assert_eq!(ticks(2.0), (50.0, 5));
        // Milimetre başına 6 piksel: 9,3 mm → 10 mm, onda birer.
        assert_eq!(ticks(6.0), (10.0, 10));
        // Milimetre başına 30 piksel: 1,87 mm → 2 mm, dörtte birer.
        assert_eq!(ticks(30.0), (2.0, 4));
        // Çok uzakta: 5.600 birim → 10.000; küçük çizgiler sığdığı kadar.
        let (major, parts) = ticks(0.01);
        assert_eq!(major, 10_000.0);
        assert!(major * 0.01 / parts as f32 >= TICK_SPACING);
    }

    #[test]
    fn labels_show_as_many_decimals_as_the_step_needs() {
        assert_eq!(decimals(50.0), 0);
        assert_eq!(decimals(1.0), 0);
        assert_eq!(decimals(0.5), 1);
        assert_eq!(decimals(0.2), 1);
        assert_eq!(decimals(0.05), 2);
    }

    #[test]
    fn transforms_round_trip_in_both_directions() {
        let paper = Transform::new(Point::new(40.0, 30.0), 2.0);
        let units = paper.to_units(Point::new(140.0, 230.0));

        assert_eq!(units, Point::new(50.0, 100.0));
        assert_eq!(paper.to_content(units), Point::new(140.0, 230.0));

        let cad = Transform::new(Point::new(40.0, 230.0), 2.0).y_up();
        assert_eq!(cad.to_units(Point::new(40.0, 30.0)), Point::new(0.0, 100.0));
        assert_eq!(
            cad.to_content(Point::new(0.0, 100.0)),
            Point::new(40.0, 30.0)
        );
    }

    #[test]
    fn guides_are_added_moved_and_removed() {
        let mut guides = Guides::new();

        guides.update(Event::Added(Guide::horizontal(20.0)));
        guides.update(Event::Added(Guide::vertical(35.0)));
        guides.update(Event::Moved(0, 25.0));
        assert_eq!(
            guides.as_slice(),
            [Guide::horizontal(25.0), Guide::vertical(35.0)]
        );

        guides.update(Event::Removed(0));
        guides.update(Event::Removed(5));
        assert_eq!(guides.as_slice(), [Guide::vertical(35.0)]);
    }

    #[test]
    fn shift_snaps_to_the_small_ticks() {
        // 6 piksel/mm: 10 mm'lik aralık onda birer; 1 mm'ye oturur.
        assert_eq!(snap(12.4, 6.0), 12.0);
        assert_eq!(snap(12.6, 6.0), 13.0);
    }
}

/// Gerçek olaylarla: cetvelden kılavuz çıkarma, taşıma ve cetvele geri
/// bırakıp silme.
#[cfg(all(test, feature = "snapshot"))]
mod interaction {
    use iced::widget::space;
    use iced::{Element, Fill, Point, Size};

    use super::{Event, Guide, Guides, Rulers, Transform, thickness};
    use crate::snapshot::{Input, Snapshot};

    fn view(guides: &Guides) -> Element<'_, Event> {
        Rulers::new(
            space::horizontal().width(Fill).height(Fill),
            Transform::new(Point::ORIGIN, 2.0),
        )
        .unit("mm")
        .guides(guides, |event| event)
        .into()
    }

    #[test]
    fn guides_are_dragged_out_moved_and_dropped_back() {
        let mut snapshot = Snapshot::new(Size::new(400.0, 300.0)).expect("çizici kurulamadı");
        let mut guides = Guides::new();
        let mut update = |guides: &mut Guides, event| guides.update(event);
        let mut input =
            |guides: &mut Guides, input| snapshot.input(guides, view, &mut update, input);
        let ruler = thickness();

        // Üst cetvelden aşağı: içerikte y = 100 piksel = 50 mm.
        input(
            &mut guides,
            Input::Drag(
                Point::new(200.0, ruler / 2.0),
                Point::new(200.0, ruler + 100.0),
            ),
        );
        assert_eq!(guides.as_slice(), [Guide::horizontal(50.0)]);

        // Sol cetvelden sağa: x = 60 piksel = 30 mm.
        input(
            &mut guides,
            Input::Drag(
                Point::new(ruler / 2.0, 150.0),
                Point::new(ruler + 60.0, 150.0),
            ),
        );
        assert_eq!(guides.as_slice()[1], Guide::vertical(30.0));

        // Yatay kılavuzu 40 piksel aşağı taşı.
        input(
            &mut guides,
            Input::Drag(
                Point::new(300.0, ruler + 100.0),
                Point::new(300.0, ruler + 140.0),
            ),
        );
        assert_eq!(guides.as_slice()[0], Guide::horizontal(70.0));

        // Dikey kılavuzu sol cetvele geri bırak: silinir.
        input(
            &mut guides,
            Input::Drag(
                Point::new(ruler + 60.0, 200.0),
                Point::new(ruler / 2.0, 200.0),
            ),
        );
        assert_eq!(guides.as_slice(), [Guide::horizontal(70.0)]);
    }
}
