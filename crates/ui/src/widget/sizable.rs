//! Boyutlandırılan, tam ekran yapılan ve boyunu hatırlayan iletişim kutusu (docs/adr/0746).
//!
//! ```text
//! ┌ Formla düzenle ─────────────── [⤢] [×] ┐  kenarlardan ve köşelerden çekilir;
//! │ …                                       │  [⤢] ya da başlığa çift tık: tam ekran
//! │                        [Vazgeç] [Kaydet]│  (arkadaki pencereyi kenarlarda
//! └──────────────────────────────────────╯    biraz boşluk bırakarak doldurur)
//! ```
//!
//! Çekilebildiğini kutunun kendi çerçevesi söyler (docs/adr/0804): ayrı bir kulakçık yok, sağ alt köşenin kavisi biraz
//! kalındır; imleç bir kenara gelince o kenarın çerçevesi vurgu rengiyle aydınlanır ([`frame_mark`]).
//!
//! Kutu ortadadır: kenarı çekilen kutu iki yana birlikte büyür, kenar imleci izler. Her kutunun boyu ve tam ekran
//! olup olmadığı anahtarıyla (öntanımlı olarak başlığı) [`sizes`]'ta tutulur: kutu bir sonraki açılışta aynı boyda
//! açılır. Uygulama kayıtları açılışta yükler ([`sizes::load`]) ve değişince yazar ([`sizes::take_changed`]).
//! Boylar yazı boyutuna bölünmüş tutulur: yazı büyüyünce kutu da büyür.

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::{
    Background, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector, border,
    mouse,
};

use crate::icon::{Icon, icon};
use crate::theme::{Tokens, typography};

/// Tam ekran kutunun arkadaki pencerenin kenarlarına uzaklığı.
pub const MARGIN: f32 = 16.0;

/// Kutunun iç boşluğu (dialog'un `padding`'i): düğme onun köşesinde durur.
pub const PAD: f32 = 18.0;

/// Başlıkta tam ekran düğmesine ayrılan kare, varsayılan yazı boyutunda; kapatma düğmesi de bu boydadır.
pub const TOGGLE: f32 = crate::theme::metrics::INLINE;

/// Başlıktaki düğmelerin arası.
pub const TOGGLE_GAP: f32 = 2.0;

/// Aydınlanan çerçevenin kalınlığı.
const MARK: f32 = 1.5;

/// Dinlenen köşe kavisinin kenar boyunca uzantısı.
const MARK_TAIL: f32 = 3.0;

/// Üzerine gelinen köşede kavisin kenar boyunca uzantısı.
const MARK_TAIL_HOT: f32 = 10.0;

/// Kenarın içinde çekilen bandın kalınlığı.
const GRIP: f32 = 6.0;

/// Köşeden çekmenin kenar boyunca uzunluğu.
const CORNER: f32 = 16.0;

/// En küçük boy (yazı boyutuyla büyür).
const MIN: Size = Size::new(280.0, 160.0);

/// Kutuların hatırlanan boyları.
pub mod sizes {
    use std::collections::BTreeMap;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Bir kutunun hatırlanan boyu, yazı boyutuna bölünmüş; `maximized`: tam ekran.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Kept {
        pub width: f32,
        pub height: f32,
        pub maximized: bool,
    }

    static SIZES: Mutex<BTreeMap<String, Kept>> = Mutex::new(BTreeMap::new());
    static CHANGED: AtomicBool = AtomicBool::new(false);

    fn with<T>(f: impl FnOnce(&mut BTreeMap<String, Kept>) -> T) -> T {
        let mut map = SIZES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&mut map)
    }

    /// `key`'in hatırlanan boyu.
    pub fn get(key: &str) -> Option<Kept> {
        with(|m| m.get(key).copied())
    }

    /// `key`'in boyunu tutar; değişmişse kayıtlar yazılmayı bekler.
    pub fn set(key: &str, kept: Kept) {
        let changed = with(|m| m.insert(key.to_owned(), kept) != Some(kept));
        if changed {
            CHANGED.store(true, Ordering::Relaxed);
        }
    }

    /// `key`'i unutur: kutu yine içeriği kadar açılır.
    pub fn forget(key: &str) {
        if with(|m| m.remove(key).is_some()) {
            CHANGED.store(true, Ordering::Relaxed);
        }
    }

    /// Kayıtların hepsi (yazmak için).
    pub fn all() -> BTreeMap<String, Kept> {
        with(|m| m.clone())
    }

    /// Kayıtları değiştirir (açılışta okunanlar); yazılmayı beklemez.
    pub fn load(kept: BTreeMap<String, Kept>) {
        with(|m| *m = kept);
        CHANGED.store(false, Ordering::Relaxed);
    }

    /// Son çağrıdan beri bir kayıt değiştiyse `true`.
    pub fn take_changed() -> bool {
        CHANGED.swap(false, Ordering::Relaxed)
    }

    static SHOWN: Mutex<Vec<(String, [f32; 4])>> = Mutex::new(Vec::new());

    /// Çizilen kutunun anahtarı ve ekrandaki yeri (x, y, en, boy): son çizilenler, en yenisi sonda.
    pub(super) fn note_shown(key: &str, bounds: [f32; 4]) {
        let mut shown = SHOWN
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        shown.retain(|(k, _)| k != key);
        shown.push((key.to_owned(), bounds));
        let over = shown.len().saturating_sub(16);
        shown.drain(..over);
    }

    /// Son çizilen kutular, en yenisi sonda (testler ve ekran resimleri kutunun kenarını buradan bulur).
    pub fn shown() -> Vec<(String, [f32; 4])> {
        SHOWN
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

/// Hangi kenarlar çekiliyor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Edges {
    left: bool,
    right: bool,
    top: bool,
    bottom: bool,
}

impl Edges {
    fn any(self) -> bool {
        self.left || self.right || self.top || self.bottom
    }

    fn interaction(self) -> mouse::Interaction {
        match (self.left || self.right, self.top || self.bottom) {
            (true, true) if (self.left && self.top) || (self.right && self.bottom) => {
                mouse::Interaction::ResizingDiagonallyDown
            }
            (true, true) => mouse::Interaction::ResizingDiagonallyUp,
            (true, false) => mouse::Interaction::ResizingHorizontally,
            (false, true) => mouse::Interaction::ResizingVertically,
            (false, false) => mouse::Interaction::None,
        }
    }
}

/// Kenarların `point`'e yakın olanları.
fn edges_at(bounds: Rectangle, point: Point) -> Edges {
    if !bounds.contains(point) {
        return Edges::default();
    }
    let near = |a: f32, b: f32, band: f32| (a - b).abs() <= band;
    let mut e = Edges {
        left: near(point.x, bounds.x, GRIP),
        right: near(point.x, bounds.x + bounds.width, GRIP),
        top: near(point.y, bounds.y, GRIP),
        bottom: near(point.y, bounds.y + bounds.height, GRIP),
    };
    // Köşeye yakın kenar, köşenin öbür kenarını da alır.
    if e.left || e.right {
        e.top |= near(point.y, bounds.y, CORNER);
        e.bottom |= near(point.y, bounds.y + bounds.height, CORNER);
    }
    if e.top || e.bottom {
        e.left |= near(point.x, bounds.x, CORNER);
        e.right |= near(point.x, bounds.x + bounds.width, CORNER);
    }
    e
}

/// Çerçevenin çekilebilen yerini söyler: dinlenirken yalnız sağ alt köşenin kavisi (`rest` tonunda), imleç bir kenara
/// ya da köşeye gelince o kenar ya da köşe vurgu renginde. Kutunun kendi kenarının üstüne çizilir; kutu `radius`
/// köşeli. Yüzen pencereler de aynısını kullanır.
pub(crate) fn frame_mark(
    renderer: &mut Renderer,
    t: &Tokens,
    bounds: Rectangle,
    radius: f32,
    (left, right, top, bottom): (bool, bool, bool, bool),
) {
    let any = left || right || top || bottom;
    // Which part of the frame shows: one edge wholly, a corner with short tails, the arc alone at rest.
    let corner = (left || right) && (top || bottom);
    let reach = radius + if corner { MARK_TAIL_HOT } else { MARK_TAIL };
    let along = |low: bool, high: bool, start: f32, length: f32| -> (f32, f32) {
        match (low, high) {
            (true, _) => (start, reach.min(length)),
            (_, true) => (start + (length - reach).max(0.0), reach.min(length)),
            _ => (start, length),
        }
    };
    let (x, width) = if any {
        along(left, right, bounds.x, bounds.width)
    } else {
        along(false, true, bounds.x, bounds.width)
    };
    let (y, height) = if any {
        along(top, bottom, bounds.y, bounds.height)
    } else {
        along(false, true, bounds.y, bounds.height)
    };
    let clip = Rectangle {
        x,
        y,
        width,
        height,
    };
    renderer.with_layer(clip, |renderer| {
        renderer.fill_quad(
            Quad {
                bounds,
                border: iced::Border {
                    color: if any { t.accent } else { t.border_grip() },
                    width: MARK,
                    radius: radius.into(),
                },
                ..Quad::default()
            },
            Background::Color(iced::Color::TRANSPARENT),
        );
    });
}

#[derive(Default)]
struct State {
    /// Çekilen kenarlar, basılan yer ve o andaki boy.
    drag: Option<(Edges, Point, Size)>,
    /// İmlecin üstünde durduğu kenarlar.
    hover: Edges,
    toggle_hovered: bool,
    last_click: Option<iced::advanced::mouse::Click>,
}

/// Boyutlandırılan kutu: içeriği ve başlıktaki tam ekran düğmesi.
pub struct Sizable<'a, Message> {
    key: String,
    content: Element<'a, Message>,
    toggle: Element<'a, Message>,
    maximized: bool,
    /// Başlıkta tam ekran düğmesinin sağında bir düğme daha var: kapatma.
    closable: bool,
}

impl<'a, Message: 'a> Sizable<'a, Message> {
    /// `content`: sabit boyda yerleşebilen kutu (hatırlanan boyu varsa `Fill`).
    pub fn new(key: impl Into<String>, content: impl Into<Element<'a, Message>>) -> Self {
        let key = key.into();
        let maximized = sizes::get(&key).is_some_and(|k| k.maximized);
        Self {
            toggle: icon(if maximized {
                Icon::Restore
            } else {
                Icon::Maximize
            })
            .size(14.0)
            .into(),
            key,
            content: content.into(),
            maximized,
            closable: false,
        }
    }

    /// Başlıkta tam ekran düğmesinin sağında kapatma düğmesi durur: tam ekran düğmesi onun yanına çekilir.
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Tam ekran düğmesinin karesi.
    fn toggle_bounds(&self, bounds: Rectangle) -> Rectangle {
        let side = typography::from_default(TOGGLE);
        let beside = if self.closable {
            side + TOGGLE_GAP
        } else {
            0.0
        };
        Rectangle::new(
            Point::new(
                bounds.x + bounds.width - PAD - side - beside,
                bounds.y + PAD,
            ),
            Size::new(side, side),
        )
    }
}

/// Yazı boyutunun çarpanı (12 piksellik gövde metnine göre).
fn scale() -> f32 {
    typography::scaled(100.0) / 100.0
}

/// Kutunun alabileceği en büyük boy: arkadaki pencere, kenarlarda `MARGIN` kalarak.
fn room(max: Size) -> Size {
    Size::new(
        (max.width - 2.0 * MARGIN).max(1.0),
        (max.height - 2.0 * MARGIN).max(1.0),
    )
}

fn clamp(size: Size, room: Size) -> Size {
    let min = Size::new(MIN.width * scale(), MIN.height * scale());
    Size::new(
        size.width.max(min.width.min(room.width)).min(room.width),
        size.height
            .max(min.height.min(room.height))
            .min(room.height),
    )
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Sizable<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content), Tree::new(&self.toggle)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.content, &self.toggle]);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let room = room(limits.max());
        let kept = sizes::get(&self.key).map(|k| {
            if k.maximized {
                room
            } else {
                clamp(Size::new(k.width * scale(), k.height * scale()), room)
            }
        });
        let child_limits = match kept {
            Some(size) => layout::Limits::new(size, size),
            None => layout::Limits::new(Size::ZERO, room),
        };
        let content =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &child_limits);
        let size = content.size();
        let side = typography::from_default(TOGGLE);
        let glyph = self.toggle.as_widget_mut().layout(
            &mut tree.children[1],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(side, side)),
        );
        let g = glyph.size();
        let square = self.toggle_bounds(Rectangle::with_size(size));
        let glyph = glyph.move_to(Point::new(
            square.x + (side - g.width) / 2.0,
            square.y + (side - g.height) / 2.0,
        ));
        Node::with_children(size, vec![content, glyph])
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
        let bounds = layout.bounds();
        let toggle = self.toggle_bounds(bounds);
        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let (Some((edges, from, start)), Some(at)) = (state.drag, cursor.position()) {
                    let (dx, dy) = (at.x - from.x, at.y - from.y);
                    let mut size = start;
                    // Kutu ortada: kenar imleci izlesin diye iki yana birlikte büyür.
                    if edges.right {
                        size.width += 2.0 * dx;
                    } else if edges.left {
                        size.width -= 2.0 * dx;
                    }
                    if edges.bottom {
                        size.height += 2.0 * dy;
                    } else if edges.top {
                        size.height -= 2.0 * dy;
                    }
                    let size = clamp(size, room(viewport.size()));
                    sizes::set(
                        &self.key,
                        sizes::Kept {
                            width: size.width / scale(),
                            height: size.height / scale(),
                            maximized: false,
                        },
                    );
                    shell.invalidate_widgets();
                    shell.invalidate_layout();
                    shell.request_redraw();
                    shell.capture_event();
                    return;
                }
                let hover = if self.maximized {
                    Edges::default()
                } else {
                    cursor
                        .position()
                        .map_or_else(Edges::default, |p| edges_at(bounds, p))
                };
                let on_toggle = cursor.is_over(toggle);
                if hover != state.hover || on_toggle != state.toggle_hovered {
                    state.hover = hover;
                    state.toggle_hovered = on_toggle;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(at) = cursor.position() else {
                    return;
                };
                if toggle.contains(at) {
                    self.flip(bounds.size());
                    shell.invalidate_widgets();
                    shell.invalidate_layout();
                    shell.capture_event();
                    return;
                }
                if !self.maximized {
                    let edges = edges_at(bounds, at);
                    if edges.any() {
                        state.drag = Some((edges, at, bounds.size()));
                        shell.capture_event();
                        return;
                    }
                }
                // Başlığa çift tık: tam ekran ya da önceki boy.
                let header = Rectangle::new(
                    Point::new(bounds.x, bounds.y),
                    Size::new(bounds.width, PAD + typography::from_default(TOGGLE)),
                );
                if header.contains(at) {
                    let click = iced::advanced::mouse::Click::new(
                        at,
                        mouse::Button::Left,
                        state.last_click,
                    );
                    state.last_click = Some(click);
                    if click.kind() == iced::advanced::mouse::click::Kind::Double {
                        self.flip(bounds.size());
                        shell.invalidate_widgets();
                        shell.invalidate_layout();
                        shell.capture_event();
                        return;
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.drag.is_some() =>
            {
                state.drag = None;
                shell.capture_event();
                return;
            }
            _ => {}
        }
        if let Some(content) = layout.children().next() {
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
        if let Some((edges, ..)) = state.drag {
            return edges.interaction();
        }
        let bounds = layout.bounds();
        if !self.maximized
            && let Some(at) = cursor.position()
        {
            let edges = edges_at(bounds, at);
            if edges.any() {
                return edges.interaction();
            }
        }
        if cursor.is_over(self.toggle_bounds(bounds)) {
            return mouse::Interaction::Pointer;
        }
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
        let mut parts = layout.children();
        let (Some(content), Some(glyph)) = (parts.next(), parts.next()) else {
            return;
        };
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            content,
            cursor,
            viewport,
        );
        let b = layout.bounds();
        sizes::note_shown(&self.key, [b.x, b.y, b.width, b.height]);
        let t = Tokens::of(theme);
        let state = tree.state.downcast_ref::<State>();
        let square = self.toggle_bounds(layout.bounds());
        if state.toggle_hovered {
            renderer.fill_quad(
                Quad {
                    bounds: square,
                    border: border::rounded(crate::theme::shape::radius(3.0)),
                    ..Quad::default()
                },
                Background::Color(t.layer(0.07)),
            );
        }
        self.toggle.as_widget().draw(
            &tree.children[1],
            renderer,
            theme,
            &renderer::Style {
                text_color: if state.toggle_hovered {
                    t.text
                } else {
                    t.muted
                },
            },
            glyph,
            mouse::Cursor::Unavailable,
            viewport,
        );
        // Çekilebildiğini çerçevenin kendisi söyler (docs/adr/0804).
        if !self.maximized {
            let e = state.drag.map_or(state.hover, |(e, ..)| e);
            frame_mark(
                renderer,
                &t,
                layout.bounds(),
                crate::theme::shape::lg(),
                (e.left, e.right, e.top, e.bottom),
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

impl<'a, Message: 'a> Sizable<'a, Message> {
    /// Tam ekran ile önceki boy arasında geçer; boyu olmayan kutunun önceki boyu şimdiki boyudur.
    fn flip(&mut self, now: Size) {
        let kept = sizes::get(&self.key).unwrap_or(sizes::Kept {
            width: now.width / scale(),
            height: now.height / scale(),
            maximized: false,
        });
        sizes::set(
            &self.key,
            sizes::Kept {
                maximized: !kept.maximized,
                ..kept
            },
        );
    }
}

impl<'a, Message: 'a> From<Sizable<'a, Message>> for Element<'a, Message> {
    fn from(s: Sizable<'a, Message>) -> Self {
        Element::new(s)
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use iced::widget::text;
    use iced::{Element, Event, Point, Size, Theme, mouse};

    use super::sizes;
    use crate::snapshot::Snapshot;
    use crate::widget::{Dialog, overlay};

    fn view(_: &()) -> Element<'_, ()> {
        overlay::modal(
            Dialog::new("Boyutlanan kutu")
                .push(text("Gövde"))
                .width(500.0),
            (),
        )
    }

    fn at(x: f32, y: f32) -> Event {
        Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(x, y),
        })
    }

    #[test]
    fn a_dialog_is_resized_from_its_edge_and_made_full_screen_from_its_button() {
        sizes::load(Default::default());
        let mut snapshot = Snapshot::new(Size::new(1000.0, 800.0)).expect("çizici kurulamadı");
        let mut update = |_: &mut (), _: ()| {};
        let _ = snapshot.render(view(&()), &Theme::Dark);
        let (key, [x, y, w, h]) = sizes::shown().pop().expect("the box was drawn");
        assert_eq!(key, "Boyutlanan kutu");
        // The right edge dragged 100 to the right: the box, in the middle, grows 200.
        let edge = x + w - 2.0;
        snapshot.step(
            &mut (),
            view,
            &mut update,
            &[
                at(edge, y + h / 2.0),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                at(edge + 100.0, y + h / 2.0 + 60.0),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
        );
        let kept = sizes::get(&key).expect("kept");
        let scale = super::scale();
        assert!(
            (kept.width * scale - (w + 200.0)).abs() < 0.5,
            "{kept:?} {w}"
        );
        assert!(!kept.maximized);
        assert!(sizes::take_changed());
        let _ = snapshot.render(view(&()), &Theme::Dark);
        let (_, [_, _, w2, h2]) = sizes::shown().pop().expect("drawn");
        assert!((w2 - (w + 200.0)).abs() < 0.5, "{w2}");
        assert!(h2 >= h, "{h2} {h}");
        // The full screen button: the whole window but a margin; again: the kept size.
        let [x2, y2, ..] = sizes::shown().pop().expect("drawn").1;
        let button = Point::new(x2 + w2 - super::PAD - 8.0, y2 + super::PAD + 8.0);
        snapshot.step(
            &mut (),
            view,
            &mut update,
            &[
                at(button.x, button.y),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
        );
        assert!(sizes::get(&key).is_some_and(|k| k.maximized));
        let _ = snapshot.render(view(&()), &Theme::Dark);
        let (_, [x3, y3, w3, h3]) = sizes::shown().pop().expect("drawn");
        assert_eq!(
            [x3, y3, w3, h3],
            [
                super::MARGIN,
                super::MARGIN,
                1000.0 - 2.0 * super::MARGIN,
                800.0 - 2.0 * super::MARGIN
            ]
        );
        snapshot.step(
            &mut (),
            view,
            &mut update,
            &[
                at(x3 + w3 - super::PAD - 8.0, y3 + super::PAD + 8.0),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
        );
        let back = sizes::get(&key).expect("kept");
        assert!(!back.maximized && (back.width - kept.width).abs() < 0.01);
        sizes::load(Default::default());
    }
}
