//! Dairesel menü: imlecin çevresinde açılan, yöne göre seçilen komutlar.
//!
//! ```text
//!                      ( ⬚ Seç )
//!        ( ▭ Dikdörtgen )      ( ╱ Çizgi )
//!      ( ○ Daire )        ◎        ( ⌇ Çoklu çizgi )
//!           ( • Nokta )        ( ⬠ Alan )
//!                      ( ⟷ Ölç )
//! ```
//!
//! Blender'daki pasta menüleri ve Maya'daki işaretleme menüleri gibi: menü
//! imlecin olduğu yerde açılır, komutlar çevrede hep aynı yönlerde durur ve
//! kas hafızası oluşur ("sağ üst: çizgi"). İmleci bir yöne kaydırmak o
//! yöndeki komutu seçer (ortadaki ölü bölgenin dışında), tıklamak
//! çalıştırır. Menüyü açan tuş ([`RadialMenu::hold`]) basılı tutulup imleç
//! bir yöne çekilerek bırakılırsa seçili komut hemen çalışır; tuşa kısa
//! basmak menüyü açık bırakır, yeniden basmak kapatır. Fareyle basılı tutup
//! çekip bırakmak da aynı işi görür. Esc, sağ tık ya da ortaya tıklamak
//! menüyü kapatır.
//!
//! Komutlar tepeden başlayıp saat yönünde eşit aralıklarla dizilir (en çok
//! sekiz). Menü, açıldığı anda imlecin içerik üstündeki yerinde açılır;
//! imleç içeriğin dışındaysa ortada. Kenara yakın açılan menü içeriğin
//! içine kaydırılır.
//!
//! ```ignore
//! RadialMenu::new(model_space, self.radial_open, Message::RadialClosed)
//!     .hold(keyboard::Key::Named(key::Named::Space))
//!     .item(Icon::Select, "Seç", Message::ToolSelected(Tool::Select))
//!     .item(Icon::Line, "Çizgi", Message::ToolSelected(Tool::Line))
//! ```
//!
//! Komut seçilince önce komutun mesajı, ardından `on_close` gönderilir;
//! uygulama menüyü `on_close` ile kapatır.

use std::f32::consts::TAU;
use std::time::Duration;

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard::{self, key};
use iced::time::Instant;
use iced::widget::text::Wrapping;
use iced::{
    Background, Border, Element, Event, Length, Point, Rectangle, Renderer, Shadow, Size, Theme,
    Vector, mouse, window,
};

use crate::icon::{Icon, icon};
use crate::label;
use crate::theme::{Tokens, motion, typography};

/// Komutların merkeze uzaklığı ve ortadaki ölü bölgenin yarıçapı (12
/// piksellik gövde metnine göre).
const RADIUS: f32 = 92.0;
const DEAD: f32 = 16.0;
/// Komut kutusunun iç boşlukları ve ikonla ad arası.
const PAD_X: f32 = 10.0;
const PAD_Y: f32 = 5.0;
const ICON_GAP: f32 = 6.0;
/// Menünün içeriğin kenarlarıyla en az aralığı.
const MARGIN: f32 = 8.0;
/// Tuş ya da fare bundan uzun basılı tutulup bırakılırsa seçili komut
/// çalışır; daha kısa basış menüyü açık bırakır.
const HOLD: Duration = Duration::from_millis(220);
/// Açılış: komutlar merkezden dışa doğru yerlerine kayar.
const OPEN: Duration = Duration::from_millis(130);
/// En çok komut sayısı.
pub const MAX_ITEMS: usize = 8;

/// Dairesel menü.
pub struct RadialMenu<'a, Message> {
    content: Element<'a, Message>,
    open: bool,
    on_close: Message,
    hold: Option<keyboard::Key>,
    items: Vec<(Icon, String, Option<Message>)>,
}

impl<'a, Message: Clone + 'a> RadialMenu<'a, Message> {
    /// `open` menünün açık olup olmadığıdır; kapatmak için `on_close`
    /// gönderilir.
    pub fn new(content: impl Into<Element<'a, Message>>, open: bool, on_close: Message) -> Self {
        Self {
            content: content.into(),
            open,
            on_close,
            hold: None,
            items: Vec::new(),
        }
    }

    /// Komut; tepeden başlayarak saat yönünde dizilir. `on_press` yoksa
    /// sönüktür ve seçilemez. Sekizden fazlası yok sayılır.
    pub fn item(
        mut self,
        glyph: Icon,
        label: impl Into<String>,
        on_press: impl Into<Option<Message>>,
    ) -> Self {
        if self.items.len() < MAX_ITEMS {
            self.items.push((glyph, label.into(), on_press.into()));
        }
        self
    }

    /// Menüyü açan tuş: basılı tutulup bırakılınca seçili komut çalışır.
    pub fn hold(mut self, key: keyboard::Key) -> Self {
        self.hold = Some(key);
        self
    }
}

struct Radial<'a, Message> {
    /// İçerik; ardından her komutun ikonu ve adı.
    elements: Vec<Element<'a, Message>>,
    open: bool,
    on_close: Message,
    hold: Option<keyboard::Key>,
    actions: Vec<Option<Message>>,
}

impl<'a, Message: Clone + 'a> From<RadialMenu<'a, Message>> for Element<'a, Message> {
    fn from(menu: RadialMenu<'a, Message>) -> Self {
        let mut elements = vec![menu.content];
        let mut actions = Vec::with_capacity(menu.items.len());

        for (glyph, name, on_press) in menu.items {
            elements.push(icon(glyph).into());
            elements.push(label::body(name).wrapping(Wrapping::None).into());
            actions.push(on_press);
        }

        Element::new(Radial {
            elements,
            open: menu.open,
            on_close: menu.on_close,
            hold: menu.hold,
            actions,
        })
    }
}

#[derive(Debug)]
struct State {
    /// Uygulamanın son istediği durum; kapalıdan açığa geçişte menü açılır.
    requested: bool,
    /// Menü açık mı; komut seçilince uygulamayı beklemeden kapanır.
    open: bool,
    /// Açıldığı yer ve kenarlara göre kaydırılmış merkez (içeriğe göre).
    anchor: Option<Point>,
    center: Point,
    /// İmlecin içerik üstündeki son yeri (içeriğe göre).
    cursor: Option<Point>,
    highlighted: Option<usize>,
    /// İmleç ölü bölgeden çıktı mı: fareyle çekip bırakmak için.
    moved: bool,
    opened: Instant,
    now: Instant,
}

/// Menüyü açan tuş ya da fare bırakıldığında yapılacak iş.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Release {
    /// Basılı tutulup bir yöne çekildi: seçili komut çalışır.
    Choose(usize),
    /// Basılı tutuldu ama yön verilmedi: menü kapanır.
    Close,
    /// Kısa basış: menü tıklamayı bekler.
    Stay,
}

fn released(held: Duration, highlighted: Option<usize>) -> Release {
    match highlighted {
        _ if held < HOLD => Release::Stay,
        Some(index) => Release::Choose(index),
        None => Release::Close,
    }
}

/// `index`. komutun yönü: tepeden başlayıp saat yönünde; x sağa, y aşağı.
fn direction(index: usize, count: usize) -> Vector {
    let angle = TAU * index as f32 / count.max(1) as f32;

    Vector::new(angle.sin(), -angle.cos())
}

/// İmlecin merkeze göre yönündeki komut; ölü bölgedeyse `None`.
fn sector(center: Point, point: Point, count: usize) -> Option<usize> {
    let offset = point - center;

    if count == 0 || offset.x.hypot(offset.y) < DEAD {
        return None;
    }

    let angle = offset.x.atan2(-offset.y).rem_euclid(TAU);
    let step = TAU / count as f32;

    Some((angle / step).round() as usize % count)
}

/// Komut kutusunun merkeze göre yeri: sağdakiler dışa doğru uzar,
/// soldakiler içe doğru biter, tepedeki ve alttaki ortalanır.
fn chip(index: usize, count: usize, size: Size, radius: f32) -> Rectangle {
    let direction = direction(index, count);
    let anchor = Point::new(direction.x * radius, direction.y * radius);
    let cap = size.height / 2.0;

    let x = if direction.x > 0.25 {
        anchor.x - cap
    } else if direction.x < -0.25 {
        anchor.x - size.width + cap
    } else {
        anchor.x - size.width / 2.0
    };

    Rectangle::new(Point::new(x, anchor.y - size.height / 2.0), size)
}

/// Menünün, bütün kutuları içeriğin içinde kalacak biçimde kaydırılmış
/// merkezi.
fn fit(center: Point, extent: Rectangle, bounds: Size) -> Point {
    let axis = |center: f32, low: f32, high: f32, size: f32| {
        let min = MARGIN - low;
        let max = size - MARGIN - high;

        if min > max {
            size / 2.0
        } else {
            center.clamp(min, max)
        }
    };

    Point::new(
        axis(center.x, extent.x, extent.x + extent.width, bounds.width),
        axis(center.y, extent.y, extent.y + extent.height, bounds.height),
    )
}

impl<'a, Message: Clone + 'a> Radial<'a, Message> {
    /// Komut seçilir: mesajı, ardından kapatma gönderilir.
    fn choose(&self, state: &mut State, index: usize, shell: &mut Shell<'_, Message>) {
        if let Some(Some(message)) = self.actions.get(index) {
            shell.publish(message.clone());
        }

        self.close(state, shell);
    }

    fn close(&self, state: &mut State, shell: &mut Shell<'_, Message>) {
        state.open = false;
        state.highlighted = None;
        shell.publish(self.on_close.clone());
        shell.request_redraw();
    }

    /// İmlecin gösterdiği, seçilebilir komut: önce üzerinde durulan kutu,
    /// yoksa yöndeki komut.
    fn target(&self, layout: Layout<'_>, state: &State, point: Point) -> Option<usize> {
        let origin = layout.bounds().position();
        let local = Point::new(point.x - origin.x, point.y - origin.y);

        let over = layout
            .children()
            .skip(1)
            .position(|chip| chip.bounds().contains(point));

        over.or_else(|| sector(state.center, local, self.actions.len()))
            .filter(|index| matches!(self.actions.get(*index), Some(Some(_))))
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Radial<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        let now = Instant::now();

        tree::State::new(State {
            requested: self.open,
            open: self.open,
            anchor: None,
            center: Point::ORIGIN,
            cursor: None,
            highlighted: None,
            moved: false,
            opened: now,
            now,
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.elements.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.elements);

        let state = tree.state.downcast_mut::<State>();

        if self.open && !state.requested {
            state.open = true;
            state.anchor = state.cursor;
            state.highlighted = None;
            state.moved = false;
            state.opened = Instant::now();
        }

        if !self.open {
            state.open = false;
        }

        state.requested = self.open;
    }

    fn size(&self) -> Size<Length> {
        self.elements[0].as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let content =
            self.elements[0]
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits);
        let bounds = content.size();
        let radius = typography::scaled(RADIUS);
        let count = self.actions.len();
        let mut sizes = Vec::with_capacity(count);
        let mut nodes = Vec::with_capacity(count);

        for index in 0..count {
            let (glyph, name) = (1 + index * 2, 2 + index * 2);
            let loose = layout::Limits::new(Size::ZERO, bounds);
            let glyph_node = self.elements[glyph].as_widget_mut().layout(
                &mut tree.children[glyph],
                renderer,
                &loose,
            );
            let name_node = self.elements[name].as_widget_mut().layout(
                &mut tree.children[name],
                renderer,
                &loose,
            );
            let (glyph_size, name_size) = (glyph_node.size(), name_node.size());
            let height = glyph_size.height.max(name_size.height) + PAD_Y * 2.0;
            let size = Size::new(
                (PAD_X * 2.0 + glyph_size.width + ICON_GAP + name_size.width).ceil(),
                height.ceil(),
            );

            sizes.push(size);
            nodes.push(vec![
                glyph_node.move_to(Point::new(
                    PAD_X,
                    ((size.height - glyph_size.height) / 2.0).round(),
                )),
                name_node.move_to(Point::new(
                    PAD_X + glyph_size.width + ICON_GAP,
                    ((size.height - name_size.height) / 2.0).round(),
                )),
            ]);
        }

        // Kutuların merkeze göre yerleri ve hepsini kapsayan alan.
        let chips: Vec<Rectangle> = sizes
            .iter()
            .enumerate()
            .map(|(index, size)| chip(index, count, *size, radius))
            .collect();
        let extent = chips.iter().fold(
            Rectangle::new(Point::new(-DEAD, -DEAD), Size::new(DEAD * 2.0, DEAD * 2.0)),
            |extent, chip| extent.union(chip),
        );

        let state = tree.state.downcast_mut::<State>();
        let anchor = state
            .anchor
            .unwrap_or(Point::new(bounds.width / 2.0, bounds.height / 2.0));
        let center = fit(anchor, extent, bounds);
        state.center = center;

        let mut children = vec![content];

        children.extend(chips.iter().zip(nodes).map(|(chip, parts)| {
            Node::with_children(chip.size(), parts)
                .move_to(Point::new(center.x + chip.x, center.y + chip.y))
        }));

        Node::with_children(bounds, children)
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
        let Some(content) = layout.children().next() else {
            return;
        };
        let state = tree.state.downcast_mut::<State>();

        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            state.now = *now;

            if state.open && motion::running(state.opened, *now, OPEN) {
                shell.request_redraw();
            }
        }

        // Menü kapalıyken imlecin yeri izlenir; menü orada açılır.
        if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
            state.cursor = cursor
                .position_over(bounds)
                .map(|point| Point::new(point.x - bounds.x, point.y - bounds.y));
        }

        if !state.open {
            self.elements[0].as_widget_mut().update(
                &mut tree.children[0],
                event,
                content,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
            return;
        }

        let point = cursor.position();
        let target = point.and_then(|point| self.target(layout, state, point));

        match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let local = Point::new(position.x - bounds.x, position.y - bounds.y);
                let offset = local - state.center;

                if offset.x.hypot(offset.y) >= DEAD {
                    state.moved = true;
                }

                if target != state.highlighted {
                    state.highlighted = target;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                match target {
                    Some(index) => self.choose(state, index, shell),
                    None => self.close(state, shell),
                }

                shell.capture_event();
                return;
            }
            Event::Mouse(mouse::Event::ButtonPressed(_)) => {
                self.close(state, shell);
                shell.capture_event();
                return;
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                // Fareyle basılı tutup çekip bırakmak seçer.
                if state.moved {
                    let held = Instant::now().saturating_duration_since(state.opened);

                    if let Release::Choose(index) = released(held, target) {
                        self.choose(state, index, shell);
                    }
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { .. }) => {
                shell.capture_event();
                return;
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Escape),
                ..
            }) => {
                self.close(state, shell);
                shell.capture_event();
                return;
            }
            // Basılı tutulan tuşun yinelenmesi yok sayılır; kısa basışla açık
            // kalan menü tuşa yeniden basınca kapanır.
            Event::Keyboard(keyboard::Event::KeyPressed { key, repeat, .. })
                if self.hold.as_ref() == Some(key) =>
            {
                if !repeat {
                    self.close(state, shell);
                }

                shell.capture_event();
                return;
            }
            Event::Keyboard(keyboard::Event::KeyReleased { key, .. })
                if self.hold.as_ref() == Some(key) =>
            {
                let held = Instant::now().saturating_duration_since(state.opened);

                match released(held, target) {
                    Release::Choose(index) => self.choose(state, index, shell),
                    Release::Close => self.close(state, shell),
                    Release::Stay => {}
                }

                shell.capture_event();
                return;
            }
            _ => {}
        }

        // İçerik olayları alır ama imleci görmez: vurgular sönsün, basılı
        // kalan düğmeler bırakılsın.
        self.elements[0].as_widget_mut().update(
            &mut tree.children[0],
            event,
            content,
            cursor.levitate(),
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

        if state.open {
            return if state.highlighted.is_some() {
                mouse::Interaction::Pointer
            } else {
                mouse::Interaction::Idle
            };
        }

        layout
            .children()
            .next()
            .map_or(mouse::Interaction::None, |content| {
                self.elements[0].as_widget().mouse_interaction(
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
        let state = tree.state.downcast_ref::<State>();
        let mut children = layout.children();
        let Some(content) = children.next() else {
            return;
        };

        self.elements[0].as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            content,
            if state.open {
                cursor.levitate()
            } else {
                cursor
            },
            viewport,
        );

        if !state.open {
            return;
        }

        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let center = Point::new(bounds.x + state.center.x, bounds.y + state.center.y);
        let progress = motion::progress(state.opened, state.now, OPEN);

        renderer.with_layer(bounds, |renderer| {
            // Ortadaki halka ve imlecin yönünü gösteren nokta.
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(
                        Point::new(center.x - DEAD, center.y - DEAD),
                        Size::new(DEAD * 2.0, DEAD * 2.0),
                    ),
                    border: Border {
                        color: t.border.scale_alpha(progress),
                        width: 2.0,
                        radius: DEAD.into(),
                    },
                    shadow: Shadow {
                        color: t.shadow().scale_alpha(progress),
                        offset: Vector::new(0.0, 1.0),
                        blur_radius: 6.0,
                    },
                    ..Quad::default()
                },
                Background::Color(t.popover.scale_alpha(0.9 * progress)),
            );

            if let Some(point) = cursor.position() {
                let offset = point - center;
                let distance = offset.x.hypot(offset.y);

                if distance >= DEAD {
                    let dot = Point::new(
                        center.x + offset.x / distance * (DEAD - 1.0),
                        center.y + offset.y / distance * (DEAD - 1.0),
                    );

                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle::new(
                                Point::new(dot.x - 4.0, dot.y - 4.0),
                                Size::new(8.0, 8.0),
                            ),
                            border: Border {
                                radius: 4.0.into(),
                                ..Border::default()
                            },
                            ..Quad::default()
                        },
                        Background::Color(t.accent),
                    );
                }
            }
        });

        for (index, chip) in children.enumerate() {
            let chip_bounds = chip.bounds();
            let highlighted = state.highlighted == Some(index);
            let enabled = matches!(self.actions.get(index), Some(Some(_)));
            // Açılırken merkezden dışa doğru kayar.
            let slide = (Point::new(chip_bounds.center_x(), chip_bounds.center_y()) - center)
                * (-(1.0 - progress) * 0.35);
            let (fill, border, text) = if highlighted {
                (t.accent, t.accent, t.on_accent)
            } else {
                (
                    t.popover,
                    t.border,
                    if enabled { t.text } else { t.disabled() },
                )
            };

            renderer.with_layer(bounds, |renderer| {
                renderer.with_translation(slide, |renderer| {
                    renderer.fill_quad(
                        Quad {
                            bounds: chip_bounds,
                            border: Border {
                                color: border.scale_alpha(progress),
                                width: 1.0,
                                radius: (chip_bounds.height / 2.0).into(),
                            },
                            shadow: Shadow {
                                color: t.shadow().scale_alpha(progress),
                                offset: Vector::new(0.0, 2.0),
                                blur_radius: 8.0,
                            },
                            ..Quad::default()
                        },
                        Background::Color(fill.scale_alpha(progress)),
                    );

                    let text_style = renderer::Style {
                        text_color: text.scale_alpha(progress),
                    };

                    for (element, part) in [1 + index * 2, 2 + index * 2]
                        .into_iter()
                        .zip(chip.children())
                    {
                        self.elements[element].as_widget().draw(
                            &tree.children[element],
                            renderer,
                            theme,
                            &text_style,
                            part,
                            mouse::Cursor::Unavailable,
                            viewport,
                        );
                    }
                });
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
            self.elements[0].as_widget_mut().operate(
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

        self.elements[0].as_widget_mut().overlay(
            &mut tree.children[0],
            content,
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_run_clockwise_from_the_top() {
        let center = Point::new(100.0, 100.0);

        assert_eq!(sector(center, Point::new(100.0, 40.0), 8), Some(0));
        assert_eq!(sector(center, Point::new(150.0, 50.0), 8), Some(1));
        assert_eq!(sector(center, Point::new(160.0, 104.0), 8), Some(2));
        assert_eq!(sector(center, Point::new(100.0, 170.0), 8), Some(4));
        assert_eq!(sector(center, Point::new(40.0, 100.0), 8), Some(6));
        assert_eq!(sector(center, Point::new(55.0, 55.0), 8), Some(7));
        // Dört komutta çaprazlar en yakın ana yöne düşer.
        assert_eq!(sector(center, Point::new(160.0, 90.0), 4), Some(1));
        assert_eq!(sector(center, Point::new(95.0, 160.0), 4), Some(2));
    }

    #[test]
    fn the_dead_zone_selects_nothing() {
        let center = Point::new(100.0, 100.0);

        assert_eq!(sector(center, Point::new(105.0, 95.0), 8), None);
        assert_eq!(sector(center, Point::new(160.0, 100.0), 0), None);
    }

    #[test]
    fn chips_grow_outward_from_their_direction() {
        let size = Size::new(80.0, 26.0);

        // Tepe: ortalanmış.
        let top = chip(0, 4, size, 90.0);
        assert!((top.center_x() - 0.0).abs() < 1e-3);
        assert!((top.center_y() + 90.0).abs() < 1e-3);

        // Sağ: sol ucu yönün hizasında, dışa uzar.
        let right = chip(1, 4, size, 90.0);
        assert!((right.x - (90.0 - 13.0)).abs() < 1e-3);

        // Sol: sağ ucu yönün hizasında, içe doğru biter.
        let left = chip(3, 4, size, 90.0);
        assert!((left.x + left.width - (-90.0 + 13.0)).abs() < 1e-3);
    }

    #[test]
    fn menus_near_the_edges_move_inside() {
        let extent = Rectangle::new(Point::new(-150.0, -110.0), Size::new(300.0, 220.0));
        let bounds = Size::new(800.0, 600.0);

        assert_eq!(
            fit(Point::new(400.0, 300.0), extent, bounds),
            Point::new(400.0, 300.0)
        );
        assert_eq!(
            fit(Point::new(20.0, 590.0), extent, bounds),
            Point::new(158.0, 482.0)
        );
        // Sığmıyorsa ortalanır.
        assert_eq!(
            fit(Point::new(20.0, 20.0), extent, Size::new(200.0, 600.0)).x,
            100.0
        );
    }

    #[test]
    fn holding_and_releasing_chooses_while_a_tap_keeps_the_menu_open() {
        assert_eq!(released(Duration::from_millis(60), Some(2)), Release::Stay);
        assert_eq!(released(HOLD * 2, Some(2)), Release::Choose(2));
        assert_eq!(released(HOLD * 2, None), Release::Close);
    }
}

/// Gerçek olaylarla: yönle seçme, tıklayarak çalıştırma, ortaya tıklayıp
/// kapatma, Esc.
#[cfg(all(test, feature = "snapshot"))]
mod interaction {
    use iced::keyboard::key::Named;
    use iced::widget::{mouse_area, space};
    use iced::{Element, Fill, Point, Size};

    use super::RadialMenu;
    use crate::icon::Icon;
    use crate::snapshot::{Input, Snapshot};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Opened,
        Chosen(usize),
        Closed,
    }

    #[derive(Default)]
    struct Demo {
        open: bool,
        events: Vec<Event>,
    }

    fn view(demo: &Demo) -> Element<'_, Event> {
        let stage =
            mouse_area(space::horizontal().width(Fill).height(Fill)).on_press(Event::Opened);

        RadialMenu::new(stage, demo.open, Event::Closed)
            .item(Icon::Select, "Seç", Event::Chosen(0))
            .item(Icon::Line, "Çizgi", Event::Chosen(1))
            .item(Icon::Measure, "Ölç", Event::Chosen(2))
            .item(Icon::Circle, "Daire", None)
            .into()
    }

    fn update(demo: &mut Demo, event: Event) {
        match event {
            Event::Opened => demo.open = true,
            Event::Closed => demo.open = false,
            Event::Chosen(_) => {}
        }

        demo.events.push(event);
    }

    #[test]
    fn items_are_chosen_by_direction_and_the_center_closes() {
        let mut snapshot = Snapshot::new(Size::new(600.0, 500.0)).expect("çizici kurulamadı");
        let mut demo = Demo::default();
        let mut update = update;
        let mut input = |demo: &mut Demo, input| snapshot.input(demo, view, &mut update, input);

        // Tıklanan yerde açılır; sağa gidip tıklamak Çizgi'yi seçer.
        input(&mut demo, Input::Click(Point::new(300.0, 250.0)));
        assert!(demo.open);
        input(&mut demo, Input::Move(Point::new(360.0, 255.0)));
        input(&mut demo, Input::Click(Point::new(360.0, 255.0)));
        assert_eq!(
            demo.events,
            [Event::Opened, Event::Chosen(1), Event::Closed]
        );
        assert!(!demo.open);

        // Sönük komut (solda Daire) seçilemez: tıklamak menüyü kapatır.
        demo.events.clear();
        input(&mut demo, Input::Click(Point::new(300.0, 250.0)));
        input(&mut demo, Input::Click(Point::new(230.0, 250.0)));
        assert_eq!(demo.events, [Event::Opened, Event::Closed]);

        // Ortaya tıklamak ve Esc kapatır.
        demo.events.clear();
        input(&mut demo, Input::Click(Point::new(300.0, 250.0)));
        input(&mut demo, Input::Click(Point::new(302.0, 251.0)));
        input(&mut demo, Input::Click(Point::new(300.0, 250.0)));
        input(&mut demo, Input::Key(Named::Escape));
        assert_eq!(
            demo.events,
            [Event::Opened, Event::Closed, Event::Opened, Event::Closed]
        );
    }
}
