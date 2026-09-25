//! Kayan araç pencereleri: harita üstünde sürüklenen, arkadaki işi
//! kilitlemeyen küçük pencereler (ölçüm, koordinata git, katman stili).
//!
//! ```text
//! ┌ Model alanı ──────────────────────────────────────────────┐
//! │ ┊┏━ Ölçüm ─────────── 12,4 km ˄ × ┓                         │
//! │ ┊┃ …                              ┃   pencerelerin dışında  │
//! │ ┊┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛   harita çalışmayı      │
//! │ ┊┌ Koordinata git ───────── ˄ × ┐     sürdürür              │
//! │ ┊│ …                            │◢                          │
//! │ ┊└──────────────────────────────┘                           │
//! └───────────────────────────────────────────────────────────┘
//!   ┊ yakalama kılavuzu
//! ```
//!
//! - Pencereler başlıklarından sürüklenir. Alanın kenarlarına ve birbirlerine
//!   yaklaşınca yakalanırlar; bu sırada model alanındaki nesne yakalamasının
//!   sarı kılavuz çizgileri görünür. Ctrl basılıyken yakalama olmaz.
//! - Tıklanan pencere öne gelir. Öndeki pencerenin başlığının üstünde, şeritte
//!   seçili sekmede olduğu gibi vurgu çizgisi bulunur.
//! - Başlığa çift tıklamak ya da ˄ düğmesi pencereyi başlığına daraltır.
//!   Başlıktaki kısa bilgi ([`ToolWindow::meta`], ör. toplam uzunluk)
//!   daraltılmışken de okunur.
//! - [`ToolWindow::resizable`] pencereler kenarlarından ve köşelerinden
//!   boyutlandırılır.
//! - Konum, pencerenin yakın olduğu kenarlara göre saklanır ([`Placement`]):
//!   sağ alta bırakılan pencere, alan daralıp genişlediğinde (ör. yan panel
//!   boyutlandırılınca ya da tablo açılınca) o köşeyle birlikte kayar.
//!
//! Pencerelerin durumu ([`Windows`]) uygulamanındır; bileşen değişiklikleri
//! [`Event`] olarak bildirir:
//!
//! ```ignore
//! Floating::new(map, &self.windows, Message::Window, |pane| match pane {
//!     Pane::Measure => ToolWindow::new("Ölçüm", self.measure_body())
//!         .icon(Icon::Measure)
//!         .meta(total),
//!     Pane::GoTo => ToolWindow::new("Koordinata git", self.go_to_body()),
//! })
//!
//! // update:
//! Message::Window(event) => self.windows.update(event),
//! ```

use std::iter;
use std::time::Duration;

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::time::Instant;
use iced::widget::text::{Fragment, IntoFragment, Wrapping};
use iced::widget::{container, row, scrollable};
use iced::{
    Background, Border, Center, Color, Element, Event as IcedEvent, Fill, Length, Point, Rectangle,
    Renderer, Shadow, Size, Theme, Vector, border, keyboard, mouse,
};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::style::button::RADIUS;
use crate::theme::{Tokens, typography};

/// Kenarların yakalandığı uzaklık (piksel).
const SNAP: f32 = 8.0;

/// Yakalanan pencerenin alanın kenarına ve yanındaki pencereye uzaklığı.
pub const GAP: f32 = 8.0;

/// Aynı yere açılan pencerelerin basamak aralığı.
const CASCADE: f32 = 24.0;

/// Boyutlandırma bandının pencerenin dışında ve içinde kalan kalınlığı.
const GRIP_OUTSIDE: f32 = 4.0;
const GRIP_INSIDE: f32 = 3.0;

/// Köşeden boyutlandırmanın kenar boyunca uzunluğu.
const CORNER: f32 = 14.0;

/// Başlığa çift tık sayılacak en uzun aralık.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// Başlık düğmelerinin kenarı ve ikonlarının boyu.
const CONTROL: f32 = 20.0;
const GLYPH: f32 = 12.0;

/// Başlık çubuğunun iç boşlukları.
const BAR_PADDING: f32 = 5.0;
const BAR_LEFT: f32 = 10.0;
const BAR_RIGHT: f32 = 4.0;

/// Varsayılan genişlik ve boyutlandırmada en küçük boyut, 12 piksellik
/// gövde metnine göre.
const WIDTH: f32 = 280.0;
const MIN_SIZE: Size = Size::new(200.0, 96.0);

/// Pencerenin bir eksendeki yeri: alanın başındaki (sol, üst) ya da
/// sonundaki (sağ, alt) kenara uzaklığı.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Anchor {
    Start(f32),
    End(f32),
}

impl Anchor {
    /// `length` uzunluğundaki pencerenin `room` uzunluğundaki alandaki
    /// başlangıcı; pencere alanın içinde kalır.
    fn resolve(self, room: f32, length: f32) -> f32 {
        let position = match self {
            Anchor::Start(distance) => distance,
            Anchor::End(distance) => room - length - distance,
        };

        position.min(room - length).max(0.0)
    }

    /// `start` konumundaki pencerenin yeri; ortası hangi yarıdaysa o
    /// kenara göre.
    fn of(start: f32, length: f32, room: f32) -> Self {
        if start + length / 2.0 <= room / 2.0 {
            Anchor::Start(start.round())
        } else {
            Anchor::End((room - start - length).round())
        }
    }

    fn offset(self, by: f32) -> Self {
        match self {
            Anchor::Start(distance) => Anchor::Start(distance + by),
            Anchor::End(distance) => Anchor::End(distance + by),
        }
    }
}

/// Pencerenin alandaki yeri: yatayda sol ya da sağ, dikeyde üst ya da alt
/// kenara uzaklık.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub x: Anchor,
    pub y: Anchor,
}

impl Placement {
    pub const fn top_left(left: f32, top: f32) -> Self {
        Self {
            x: Anchor::Start(left),
            y: Anchor::Start(top),
        }
    }

    pub const fn top_right(right: f32, top: f32) -> Self {
        Self {
            x: Anchor::End(right),
            y: Anchor::Start(top),
        }
    }

    pub const fn bottom_left(left: f32, bottom: f32) -> Self {
        Self {
            x: Anchor::Start(left),
            y: Anchor::End(bottom),
        }
    }

    pub const fn bottom_right(right: f32, bottom: f32) -> Self {
        Self {
            x: Anchor::End(right),
            y: Anchor::End(bottom),
        }
    }

    /// `size` boyutundaki pencerenin `area` içindeki sol üst köşesi;
    /// pencere alanın dışına taşmaz.
    pub fn resolve(self, area: Size, size: Size) -> Point {
        Point::new(
            self.x.resolve(area.width, size.width),
            self.y.resolve(area.height, size.height),
        )
    }

    /// Alanın sol üst köşesine göre verilen dikdörtgenin yeri; her eksende
    /// yakın olduğu kenara göre.
    pub fn of(bounds: Rectangle, area: Size) -> Self {
        Self {
            x: Anchor::of(bounds.x, bounds.width, area.width),
            y: Anchor::of(bounds.y, bounds.height, area.height),
        }
    }

    fn offset(self, by: f32) -> Self {
        Self {
            x: self.x.offset(by),
            y: self.y.offset(by),
        }
    }
}

/// Açık bir pencerenin durumu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window<K> {
    pub key: K,
    pub placement: Placement,
    /// Kullanıcının verdiği genişlik, 12 piksellik gövde metnine göre;
    /// `None` ise pencerenin varsayılanı.
    pub width: Option<f32>,
    /// Kullanıcının verdiği yükseklik; `None` ise içeriğin yüksekliği.
    pub height: Option<f32>,
    pub collapsed: bool,
}

/// Açık pencereler, arkadan öne sıralı.
#[derive(Debug, Clone, PartialEq)]
pub struct Windows<K> {
    open: Vec<Window<K>>,
}

impl<K> Default for Windows<K> {
    fn default() -> Self {
        Self { open: Vec::new() }
    }
}

impl<K: Copy + PartialEq> Windows<K> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pencereyi açar ve öne getirir; açıksa yalnızca öne getirir. Başka bir
    /// pencere aynı yerdeyse yeni pencere biraz kaydırılarak açılır.
    pub fn open(&mut self, key: K, placement: Placement) {
        if self.is_open(key) {
            self.raise(key);
            return;
        }

        let mut placement = placement;

        while self.open.iter().any(|window| window.placement == placement) {
            placement = placement.offset(CASCADE);
        }

        self.open.push(Window {
            key,
            placement,
            width: None,
            height: None,
            collapsed: false,
        });
    }

    /// Pencereyi kapatır; açık değilse `false`.
    pub fn close(&mut self, key: K) -> bool {
        let count = self.open.len();
        self.open.retain(|window| window.key != key);
        self.open.len() != count
    }

    /// Açıksa kapatır, kapalıysa açar. Pencere şimdi açıksa `true`.
    pub fn toggle(&mut self, key: K, placement: Placement) -> bool {
        if self.close(key) {
            false
        } else {
            self.open(key, placement);
            true
        }
    }

    pub fn is_open(&self, key: K) -> bool {
        self.get(key).is_some()
    }

    pub fn get(&self, key: K) -> Option<&Window<K>> {
        self.open.iter().find(|window| window.key == key)
    }

    /// Öndeki (en son tıklanan ya da açılan) pencere.
    pub fn front(&self) -> Option<K> {
        self.open.last().map(|window| window.key)
    }

    pub fn raise(&mut self, key: K) {
        if let Some(index) = self.open.iter().position(|window| window.key == key) {
            let window = self.open.remove(index);
            self.open.push(window);
        }
    }

    /// Açık pencereler, arkadan öne.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Window<K>> {
        self.open.iter()
    }

    pub fn len(&self) -> usize {
        self.open.len()
    }

    pub fn is_empty(&self) -> bool {
        self.open.is_empty()
    }

    /// Bileşenin bildirdiği değişikliği uygular.
    pub fn update(&mut self, event: Event<K>) {
        match event {
            Event::Moved { key, placement } => {
                if let Some(window) = self.get_mut(key) {
                    window.placement = placement;
                }
            }
            Event::Resized {
                key,
                width,
                height,
                placement,
            } => {
                if let Some(window) = self.get_mut(key) {
                    window.width = Some(width);
                    window.height = height;
                    window.placement = placement;
                }
            }
            Event::Raised(key) => self.raise(key),
            Event::Collapsed {
                key,
                collapsed,
                placement,
            } => {
                if let Some(window) = self.get_mut(key) {
                    window.collapsed = collapsed;
                    window.placement = placement;
                }
            }
            Event::Closed(key) => {
                self.close(key);
            }
        }
    }

    fn get_mut(&mut self, key: K) -> Option<&mut Window<K>> {
        self.open.iter_mut().find(|window| window.key == key)
    }
}

/// Pencerelerde olan değişiklik.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event<K> {
    /// Başlığından sürüklenip bırakıldı.
    Moved { key: K, placement: Placement },
    /// Kenarından ya da köşesinden boyutlandırıldı. Boyut 12 piksellik gövde
    /// metnine göredir; yükseklik yalnızca üst ya da alt kenar
    /// sürüklendiyse (ya da önceden verilmişse) vardır.
    Resized {
        key: K,
        width: f32,
        height: Option<f32>,
        placement: Placement,
    },
    /// Tıklandı ve öne geldi.
    Raised(K),
    /// Başlığına daraltıldı ya da açıldı; başlık yerinde kalır.
    Collapsed {
        key: K,
        collapsed: bool,
        placement: Placement,
    },
    /// Kapatma düğmesine basıldı.
    Closed(K),
}

impl<K: Copy> Event<K> {
    /// Değişikliğin olduğu pencere.
    pub fn key(&self) -> K {
        match *self {
            Event::Moved { key, .. }
            | Event::Resized { key, .. }
            | Event::Collapsed { key, .. }
            | Event::Raised(key)
            | Event::Closed(key) => key,
        }
    }
}

/// Kayan pencerenin içeriği: başlık, ikon, başlıktaki kısa bilgi ve gövde.
pub struct ToolWindow<'a, Message> {
    title: Fragment<'a>,
    icon: Option<Icon>,
    meta: Option<Fragment<'a>>,
    body: Element<'a, Message>,
    width: f32,
    min_size: Size,
    resizable: bool,
    scrollable: bool,
    closable: bool,
}

impl<'a, Message: 'a> ToolWindow<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>, body: impl Into<Element<'a, Message>>) -> Self {
        Self {
            title: title.into_fragment(),
            icon: None,
            meta: None,
            body: body.into(),
            width: WIDTH,
            min_size: MIN_SIZE,
            resizable: false,
            scrollable: false,
            closable: true,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Başlıktaki kısa bilgi (ör. "12,4 km"); pencere daraltılmışken de
    /// görünür.
    pub fn meta(mut self, meta: impl IntoFragment<'a>) -> Self {
        self.meta = Some(meta.into_fragment());
        self
    }

    /// Varsayılan genişlik, 12 piksellik gövde metnine göre; yazı boyutuyla
    /// büyür.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Kenarlarından ve köşelerinden boyutlandırılır.
    pub fn resizable(mut self) -> Self {
        self.resizable = true;
        self
    }

    /// Boyutlandırmada en küçük boyut, 12 piksellik gövde metnine göre.
    pub fn min_size(mut self, size: Size) -> Self {
        self.min_size = size;
        self
    }

    /// Gövde sığmadığında ince kaydırma çubuğuyla kaydırılır.
    pub fn scrollable(mut self) -> Self {
        self.scrollable = true;
        self
    }

    /// Kapatma düğmesi olmayan pencere (ör. açık kalması gereken araç).
    pub fn permanent(mut self) -> Self {
        self.closable = false;
        self
    }
}

/// Ana içeriğin (ör. model alanı) üstünde kayan pencereler.
///
/// Bulunduğu alanı doldurur; kaydırılan bir sayfada sabit yükseklik
/// verilmelidir ([`height`](Floating::height)).
pub struct Floating<'a, K, Message> {
    base: Element<'a, Message>,
    windows: Vec<Entry<'a, K, Message>>,
    on_event: Box<dyn Fn(Event<K>) -> Message + 'a>,
    width: Length,
    height: Length,
}

/// Çizilecek pencere: durumu ve parçaları.
struct Entry<'a, K, Message> {
    key: K,
    placement: Placement,
    /// Piksel olarak genişlik ve varsa yükseklik.
    width: f32,
    height: Option<f32>,
    min_size: Size,
    collapsed: bool,
    /// Öndeki pencere.
    active: bool,
    resizable: bool,
    closable: bool,
    /// Başlık satırı, daraltma ikonu, kapatma ikonu ve gövde.
    parts: [Element<'a, Message>; 4],
}

/// Pencere ağacındaki parçaların sırası.
const TITLE: usize = 0;
const TOGGLE: usize = 1;
const CLOSE: usize = 2;
const BODY: usize = 3;

impl<'a, K, Message> Floating<'a, K, Message>
where
    K: Copy + PartialEq + 'static,
    Message: 'a,
{
    /// `base` üstünde `windows`'daki açık pencereler; her pencerenin içeriği
    /// `view` ile kurulur, değişiklikler `on_event` ile bildirilir.
    pub fn new(
        base: impl Into<Element<'a, Message>>,
        windows: &Windows<K>,
        on_event: impl Fn(Event<K>) -> Message + 'a,
        view: impl Fn(K) -> ToolWindow<'a, Message>,
    ) -> Self {
        let front = windows.front();

        let windows = windows
            .iter()
            .map(|window| Entry::new(window, view(window.key), front == Some(window.key)))
            .collect();

        Self {
            base: base.into(),
            windows,
            on_event: Box::new(on_event),
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl<'a, K: Copy, Message: 'a> Entry<'a, K, Message> {
    fn new(window: &Window<K>, content: ToolWindow<'a, Message>, active: bool) -> Self {
        let mut heading = row![].spacing(6).align_y(Center);

        if let Some(glyph) = content.icon {
            heading = heading.push(icon(glyph).size(GLYPH).tone(if active {
                Tone::Accent
            } else {
                Tone::Muted
            }));
        }

        // Uzun başlık kırpılır; kısa bilgi her zaman okunur.
        let title = label::strong(content.title)
            .wrapping(Wrapping::None)
            .style(if active {
                style::text::default
            } else {
                style::text::muted
            });

        heading = heading.push(container(title).width(Fill).clip(true));

        if let Some(meta) = content.meta {
            heading = heading.push(label::mono_caption(meta).wrapping(Wrapping::None));
        }

        let body: Element<'a, Message> = if content.scrollable {
            scrollable(content.body)
                .direction(style::field::thin_scrollbar())
                .spacing(0)
                .width(Fill)
                .into()
        } else {
            container(content.body).width(Fill).into()
        };

        let toggle = icon(if window.collapsed {
            Icon::ChevronDown
        } else {
            Icon::ChevronUp
        })
        .size(GLYPH);

        Self {
            key: window.key,
            placement: window.placement,
            width: typography::scaled(window.width.unwrap_or(content.width)),
            height: window.height.map(typography::scaled),
            min_size: Size::new(
                typography::scaled(content.min_size.width),
                typography::scaled(content.min_size.height),
            ),
            collapsed: window.collapsed,
            active,
            resizable: content.resizable,
            closable: content.closable,
            parts: [
                heading.into(),
                toggle.into(),
                icon(Icon::Close).size(GLYPH).into(),
                body,
            ],
        }
    }

    fn tree(&self) -> Tree {
        Tree {
            tag: tree::Tag::stateless(),
            state: tree::State::None,
            children: self.parts.iter().map(Tree::new).collect(),
        }
    }

    /// Pencereyi `area` içinde yerleştirir. Sürüklenen ya da boyutlandırılan
    /// pencerede konum ve boyut hareketten gelir.
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        area: Size,
        gesture: Option<&Gesture<K>>,
    ) -> Node {
        let resized = gesture.and_then(|gesture| match gesture.kind {
            Kind::Resize(edges) => Some((gesture.current, edges)),
            Kind::Move => None,
        });

        let width = resized
            .map_or(self.width, |(rect, _)| rect.width)
            .min(area.width)
            .max(1.0);

        let height = match resized {
            Some((rect, edges)) if edges.top || edges.bottom || self.height.is_some() => {
                Some(rect.height)
            }
            _ => self.height,
        };

        // Başlık: düğmeler sağda, başlık satırı kalan yerde.
        let controls = if self.closable { 2.0 } else { 1.0 } * CONTROL;
        let room = (width - BAR_LEFT - BAR_RIGHT - controls - 6.0).max(0.0);

        let title = self.parts[TITLE].as_widget_mut().layout(
            &mut tree.children[TITLE],
            renderer,
            &layout::Limits::new(Size::new(room, 0.0), Size::new(room, f32::INFINITY)),
        );

        let bar = (title.size().height + 2.0 * BAR_PADDING)
            .max(CONTROL + 4.0)
            .round();
        let title_y = ((bar - title.size().height) / 2.0).round();
        let title = title.move_to(Point::new(BAR_LEFT, title_y));

        let inset = ((CONTROL - GLYPH) / 2.0).round();
        let control_y = ((bar - CONTROL) / 2.0).round() + inset;
        let close_x = width - BAR_RIGHT - CONTROL;
        let toggle_x = if self.closable {
            close_x - CONTROL
        } else {
            close_x
        };

        let glyph = layout::Limits::new(Size::ZERO, Size::new(GLYPH, GLYPH));

        let toggle = self.parts[TOGGLE]
            .as_widget_mut()
            .layout(&mut tree.children[TOGGLE], renderer, &glyph)
            .move_to(Point::new(toggle_x + inset, control_y));

        let close = self.parts[CLOSE]
            .as_widget_mut()
            .layout(&mut tree.children[CLOSE], renderer, &glyph)
            .move_to(Point::new(close_x + inset, control_y));

        // Gövde başlığın altındaki çizginin altından başlar. Daraltılmış
        // pencerede gövde yer kaplamaz.
        let (body, size) = if self.collapsed {
            (
                Node::new(Size::ZERO).move_to(Point::new(0.0, bar + 1.0)),
                Size::new(width, bar),
            )
        } else {
            let room = (area.height - bar - 1.0).max(0.0);

            let limits = match height {
                Some(height) => {
                    let height = (height - bar - 1.0).min(room).max(0.0);

                    layout::Limits::new(Size::new(width, height), Size::new(width, height))
                }
                None => layout::Limits::new(Size::new(width, 0.0), Size::new(width, room)),
            };

            let body = self.parts[BODY]
                .as_widget_mut()
                .layout(&mut tree.children[BODY], renderer, &limits)
                .move_to(Point::new(0.0, bar + 1.0));

            let size = Size::new(width, bar + 1.0 + body.size().height);

            (body, size)
        };

        let position = match gesture {
            Some(gesture) => {
                let current = gesture.current;

                Point::new(
                    current.x.min(area.width - size.width).max(0.0),
                    current.y.min(area.height - size.height).max(0.0),
                )
            }
            None => self.placement.resolve(area, size),
        };

        Node::with_children(size, vec![title, toggle, close, body]).move_to(position)
    }
}

/// Pencerenin bölgeleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Title,
    Toggle,
    Close,
    Body,
    Edge(Edges),
}

/// Boyutlandırılan kenarlar; köşede iki kenar birden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
            (true, false) => mouse::Interaction::ResizingHorizontally,
            (false, true) => mouse::Interaction::ResizingVertically,
            _ if (self.left && self.top) || (self.right && self.bottom) => {
                mouse::Interaction::ResizingDiagonallyDown
            }
            _ => mouse::Interaction::ResizingDiagonallyUp,
        }
    }
}

/// Süren sürükleme ya da boyutlandırma.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Gesture<K> {
    key: K,
    kind: Kind,
    /// Basıldığı andaki imleç konumu.
    origin: Point,
    /// Basıldığı andaki ve şimdiki yer, alanın sol üst köşesine göre.
    start: Rectangle,
    current: Rectangle,
    /// İmleç kıpırdadı mı; kıpırdamadıysa bırakınca bir şey bildirilmez.
    moved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Move,
    Resize(Edges),
}

/// Yakalama kılavuzu: `vertical` ise `at` konumunda dikey çizgi.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Guide {
    vertical: bool,
    at: f32,
    from: f32,
    to: f32,
}

struct State<K> {
    /// Pencere ağaçlarının anahtarları, ağaçlarla aynı sırada.
    keys: Vec<K>,
    gesture: Option<Gesture<K>>,
    guides: Vec<Guide>,
    hovered: Option<(K, Part)>,
    /// Basılı başlık düğmesi; bırakıldığında üzerindeyse çalışır.
    pressed: Option<(K, Part)>,
    /// Başlığa son basış; çift tıkı tanımak için.
    last_press: Option<(K, Instant)>,
    modifiers: keyboard::Modifiers,
}

impl<K> State<K> {
    fn new(keys: Vec<K>) -> Self {
        Self {
            keys,
            gesture: None,
            guides: Vec::new(),
            hovered: None,
            pressed: None,
            last_press: None,
            modifiers: keyboard::Modifiers::default(),
        }
    }
}

/// Yerleşimden okunan pencere bölgeleri, mutlak koordinatlarda.
#[derive(Debug, Clone, Copy)]
struct Geometry {
    bounds: Rectangle,
    bar: f32,
    toggle: Rectangle,
    close: Option<Rectangle>,
    body: Rectangle,
}

impl Geometry {
    fn of(layout: Layout<'_>, closable: bool) -> Self {
        let bounds = layout.bounds();
        let mut parts = layout.children().skip(TOGGLE);
        let mut control = || {
            parts
                .next()
                .map(|part| part.bounds().expand((CONTROL - GLYPH) / 2.0))
                .unwrap_or_default()
        };

        let toggle = control();
        let close = control();
        let body = layout
            .children()
            .nth(BODY)
            .map(|body| body.bounds())
            .unwrap_or_default();

        Self {
            bounds,
            bar: body.y - bounds.y - 1.0,
            toggle,
            close: closable.then_some(close),
            body,
        }
    }

    /// Noktanın düştüğü bölge; pencerenin dışındaysa `None`.
    fn part(&self, point: Point, resizable: bool, collapsed: bool) -> Option<Part> {
        let bounds = self.bounds;

        if resizable {
            if !bounds.expand(GRIP_OUTSIDE).contains(point) {
                return None;
            }

            let right = bounds.x + bounds.width;
            let bottom = bounds.y + bounds.height;

            let near_left = point.x < bounds.x + GRIP_INSIDE;
            let near_right = point.x > right - GRIP_INSIDE;
            let near_top = point.y < bounds.y + GRIP_INSIDE;
            let near_bottom = point.y > bottom - GRIP_INSIDE;

            let vertical = near_top || near_bottom;
            let horizontal = near_left || near_right;

            let edges = Edges {
                left: near_left || (vertical && point.x < bounds.x + CORNER),
                right: near_right || (vertical && point.x > right - CORNER),
                top: !collapsed && (near_top || (horizontal && point.y < bounds.y + CORNER)),
                bottom: !collapsed && (near_bottom || (horizontal && point.y > bottom - CORNER)),
            };

            if edges.any() {
                return Some(Part::Edge(edges));
            }
        } else if !bounds.contains(point) {
            return None;
        }

        Some(if self.toggle.contains(point) {
            Part::Toggle
        } else if self.close.is_some_and(|close| close.contains(point)) {
            Part::Close
        } else if point.y < bounds.y + self.bar {
            Part::Title
        } else {
            Part::Body
        })
    }
}

/// Noktanın üzerindeki en öndeki pencere ve bölgesi.
fn hit<K, Message>(
    entries: &[Entry<'_, K, Message>],
    geometries: &[Geometry],
    point: Point,
) -> Option<(usize, Part)> {
    entries
        .iter()
        .zip(geometries)
        .enumerate()
        .rev()
        .find_map(|(index, (entry, geometry))| {
            geometry
                .part(point, entry.resizable, entry.collapsed)
                .map(|part| (index, part))
        })
}

impl<'a, K, Message> Widget<Message, Theme, Renderer> for Floating<'a, K, Message>
where
    K: Copy + PartialEq + 'static,
    Message: 'a,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<K>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new(
            self.windows.iter().map(|entry| entry.key).collect(),
        ))
    }

    fn children(&self) -> Vec<Tree> {
        iter::once(Tree::new(&self.base))
            .chain(self.windows.iter().map(Entry::tree))
            .collect()
    }

    /// Pencere ağaçları anahtarlarıyla eşlenir: pencereler öne gelince ya da
    /// biri kapanınca diğerlerinin durumu (ör. odaktaki metin girişi)
    /// karışmaz.
    fn diff(&self, tree: &mut Tree) {
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State<K>>();

        match children.first_mut() {
            Some(base) => base.diff(&self.base),
            None => children.push(Tree::new(&self.base)),
        }

        let mut previous: Vec<(K, Tree)> = state.keys.drain(..).zip(children.drain(1..)).collect();

        for entry in &self.windows {
            let tree = match previous.iter().position(|(key, _)| *key == entry.key) {
                Some(index) => {
                    let (_, mut tree) = previous.swap_remove(index);
                    tree.diff_children(&entry.parts);
                    tree
                }
                None => entry.tree(),
            };

            children.push(tree);
            state.keys.push(entry.key);
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let area = limits.resolve(self.width, self.height, Size::ZERO);
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_ref::<State<K>>();

        let base = self.base.as_widget_mut().layout(
            &mut children[0],
            renderer,
            &layout::Limits::new(area, area),
        );

        let windows = self
            .windows
            .iter_mut()
            .zip(children.iter_mut().skip(1))
            .map(|(entry, tree)| {
                let gesture = state
                    .gesture
                    .as_ref()
                    .filter(|gesture| gesture.key == entry.key);

                entry.layout(tree, renderer, area, gesture)
            });

        Node::with_children(area, iter::once(base).chain(windows).collect())
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
        let Self {
            base,
            windows: entries,
            on_event,
            ..
        } = self;
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State<K>>();

        let bounds = layout.bounds();
        let area = bounds.size();
        let origin = Vector::new(bounds.x, bounds.y);
        let mut layouts = layout.children();
        let Some(base_layout) = layouts.next() else {
            return;
        };
        let windows: Vec<Layout<'_>> = layouts.collect();
        let geometries: Vec<Geometry> = windows
            .iter()
            .zip(entries.iter())
            .map(|(layout, entry)| Geometry::of(*layout, entry.closable))
            .collect();

        if let IcedEvent::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
            state.modifiers = *modifiers;
        }

        // Süren sürükleme ya da boyutlandırma fareyi tek başına kullanır.
        if let Some(mut gesture) = state.gesture
            && let IcedEvent::Mouse(mouse_event) = event
        {
            match mouse_event {
                mouse::Event::CursorMoved { position } => {
                    let others: Vec<Rectangle> = entries
                        .iter()
                        .zip(&geometries)
                        .filter(|(entry, _)| entry.key != gesture.key)
                        .map(|(_, geometry)| geometry.bounds - origin)
                        .collect();
                    let min_size = entries
                        .iter()
                        .find(|entry| entry.key == gesture.key)
                        .map_or(Size::ZERO, |entry| entry.min_size);

                    state.guides = gesture.follow(
                        *position,
                        area,
                        &others,
                        min_size,
                        !state.modifiers.command(),
                    );
                    state.gesture = Some(gesture);

                    shell.invalidate_layout();
                    shell.request_redraw();
                }
                mouse::Event::ButtonReleased(mouse::Button::Left) => {
                    state.gesture = None;
                    state.guides.clear();

                    if gesture.moved {
                        let placement = Placement::of(gesture.current, area);

                        shell.publish(on_event(match gesture.kind {
                            Kind::Move => Event::Moved {
                                key: gesture.key,
                                placement,
                            },
                            Kind::Resize(edges) => {
                                let explicit = entries.iter().any(|entry| {
                                    entry.key == gesture.key && entry.height.is_some()
                                });

                                Event::Resized {
                                    key: gesture.key,
                                    width: typography::unscaled(gesture.current.width),
                                    height: (edges.top || edges.bottom || explicit)
                                        .then(|| typography::unscaled(gesture.current.height)),
                                    placement,
                                }
                            }
                        }));
                    }

                    shell.invalidate_layout();
                    shell.request_redraw();
                }
                _ => {}
            }

            shell.capture_event();
            return;
        }

        let position = cursor.position();
        let hovered = position.and_then(|point| hit(entries, &geometries, point));

        if let IcedEvent::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft) = event
        {
            let now = hovered.map(|(index, part)| (entries[index].key, part));

            if now != state.hovered {
                state.hovered = now;
                shell.request_redraw();
            }
        }

        // Başlık, başlık düğmeleri ve kenarlar.
        match event {
            IcedEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let (Some((index, part)), Some(position)) = (hovered, position) {
                    let entry = &entries[index];
                    let geometry = &geometries[index];

                    if !entry.active {
                        shell.publish(on_event(Event::Raised(entry.key)));
                    }

                    let gesture = |kind| Gesture {
                        key: entry.key,
                        kind,
                        origin: position,
                        start: geometry.bounds - origin,
                        current: geometry.bounds - origin,
                        moved: false,
                    };

                    match part {
                        Part::Edge(edges) => {
                            state.gesture = Some(gesture(Kind::Resize(edges)));
                        }
                        Part::Title => {
                            let now = Instant::now();
                            let double = state.last_press.is_some_and(|(key, last)| {
                                key == entry.key && now.duration_since(last) <= DOUBLE_CLICK
                            });

                            if double {
                                state.last_press = None;
                                shell.publish(on_event(collapse(entry, geometry, origin, area)));
                            } else {
                                state.last_press = Some((entry.key, now));
                                state.gesture = Some(gesture(Kind::Move));
                            }
                        }
                        Part::Toggle | Part::Close => state.pressed = Some((entry.key, part)),
                        Part::Body => {}
                    }

                    if part != Part::Body {
                        shell.capture_event();
                        shell.request_redraw();
                        return;
                    }
                }
            }
            IcedEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some((key, part)) = state.pressed.take() {
                    let released = hovered.map(|(index, part)| (entries[index].key, part));

                    if released == Some((key, part))
                        && let Some(index) = entries.iter().position(|entry| entry.key == key)
                    {
                        shell.publish(on_event(match part {
                            Part::Close => Event::Closed(key),
                            _ => collapse(&entries[index], &geometries[index], origin, area),
                        }));
                    }

                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }
            }
            _ => {}
        }

        // Pencerelerin içeriği, önden arkaya. Gerçek imleci yalnızca
        // imlecin üzerinde olduğu gövde alır.
        for (index, ((entry, tree), layout)) in entries
            .iter_mut()
            .zip(children.iter_mut().skip(1))
            .zip(&windows)
            .enumerate()
            .rev()
        {
            if entry.collapsed {
                continue;
            }

            let Some(body_layout) = layout.children().nth(BODY) else {
                continue;
            };
            let Some(body_viewport) = body_layout.bounds().intersection(viewport) else {
                continue;
            };

            let body_cursor = if hovered == Some((index, Part::Body)) {
                cursor
            } else {
                cursor.levitate()
            };

            entry.parts[BODY].as_widget_mut().update(
                &mut tree.children[BODY],
                event,
                body_layout,
                body_cursor,
                renderer,
                clipboard,
                shell,
                &body_viewport,
            );

            if shell.is_event_captured() {
                return;
            }
        }

        // Pencerenin üzerindeki basış ve tekerlek arkadaki içeriğe geçmez.
        // Bırakma geçer: haritada başlayan sürükleme pencerenin üstünde de
        // biter.
        if hovered.is_some()
            && matches!(
                event,
                IcedEvent::Mouse(
                    mouse::Event::ButtonPressed(_) | mouse::Event::WheelScrolled { .. }
                )
            )
        {
            shell.capture_event();
            return;
        }

        let base_cursor = if hovered.is_some() {
            cursor.levitate()
        } else {
            cursor
        };

        base.as_widget_mut().update(
            &mut children[0],
            event,
            base_layout,
            base_cursor,
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
        let state = tree.state.downcast_ref::<State<K>>();

        if let Some(gesture) = &state.gesture {
            return match gesture.kind {
                Kind::Move => mouse::Interaction::Grabbing,
                Kind::Resize(edges) => edges.interaction(),
            };
        }

        let mut layouts = layout.children();
        let Some(base_layout) = layouts.next() else {
            return mouse::Interaction::None;
        };
        let windows: Vec<Layout<'_>> = layouts.collect();
        let geometries: Vec<Geometry> = windows
            .iter()
            .zip(&self.windows)
            .map(|(layout, entry)| Geometry::of(*layout, entry.closable))
            .collect();

        let hovered = cursor
            .position()
            .and_then(|point| hit(&self.windows, &geometries, point));

        match hovered {
            Some((_, Part::Edge(edges))) => edges.interaction(),
            Some((_, Part::Title)) => mouse::Interaction::Grab,
            Some((_, Part::Toggle | Part::Close)) => mouse::Interaction::Pointer,
            Some((index, Part::Body)) => {
                let interaction = windows[index]
                    .children()
                    .nth(BODY)
                    .map(|body| {
                        self.windows[index].parts[BODY]
                            .as_widget()
                            .mouse_interaction(
                                &tree.children[index + 1].children[BODY],
                                body,
                                cursor,
                                viewport,
                                renderer,
                            )
                    })
                    .unwrap_or_default();

                match interaction {
                    mouse::Interaction::None => mouse::Interaction::Idle,
                    interaction => interaction,
                }
            }
            None => self.base.as_widget().mouse_interaction(
                &tree.children[0],
                base_layout,
                cursor,
                viewport,
                renderer,
            ),
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
        let state = tree.state.downcast_ref::<State<K>>();
        let bounds = layout.bounds();
        let mut layouts = layout.children();
        let Some(base_layout) = layouts.next() else {
            return;
        };
        let windows: Vec<Layout<'_>> = layouts.collect();
        let geometries: Vec<Geometry> = windows
            .iter()
            .zip(&self.windows)
            .map(|(layout, entry)| Geometry::of(*layout, entry.closable))
            .collect();

        let hovered = cursor
            .position()
            .and_then(|point| hit(&self.windows, &geometries, point));

        // Pencerenin altında kalan içerik imleci görmez (ör. model alanı
        // artı imlecini pencerenin altına çizmez).
        let base_cursor = if hovered.is_some() || state.gesture.is_some() {
            cursor.levitate()
        } else {
            cursor
        };

        self.base.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            base_layout,
            base_cursor,
            viewport,
        );

        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };

        let t = Tokens::of(theme);

        for (index, ((entry, tree), layout)) in self
            .windows
            .iter()
            .zip(tree.children.iter().skip(1))
            .zip(&windows)
            .enumerate()
        {
            let geometry = &geometries[index];
            let lifted = state
                .gesture
                .is_some_and(|gesture| gesture.key == entry.key);
            let body_cursor = if hovered == Some((index, Part::Body)) && state.gesture.is_none() {
                cursor
            } else {
                cursor.levitate()
            };

            // Her pencere kendi katmanındadır: arkadaki pencerenin ve
            // haritanın yazıları öndeki pencerenin üstüne çıkmaz.
            renderer.with_layer(clip, |renderer| {
                draw_frame(renderer, &t, geometry, entry, lifted);

                let mut parts = layout.children();

                if let Some(title) = parts.next() {
                    entry.parts[TITLE].as_widget().draw(
                        &tree.children[TITLE],
                        renderer,
                        theme,
                        &renderer::Style { text_color: t.text },
                        title,
                        mouse::Cursor::Unavailable,
                        &clip,
                    );
                }

                for (part, square) in [(TOGGLE, Some(geometry.toggle)), (CLOSE, geometry.close)] {
                    let (Some(glyph), Some(square)) = (parts.next(), square) else {
                        continue;
                    };

                    let which = if part == TOGGLE {
                        Part::Toggle
                    } else {
                        Part::Close
                    };
                    let pressed = state.pressed == Some((entry.key, which));
                    let hot = pressed || state.hovered == Some((entry.key, which));

                    if hot {
                        renderer.fill_quad(
                            Quad {
                                bounds: square,
                                border: border::rounded(3.0),
                                ..Quad::default()
                            },
                            Background::Color(t.layer(if pressed { 0.12 } else { 0.07 })),
                        );
                    }

                    entry.parts[part].as_widget().draw(
                        &tree.children[part],
                        renderer,
                        theme,
                        &renderer::Style {
                            text_color: if hot { t.text } else { t.muted },
                        },
                        glyph,
                        mouse::Cursor::Unavailable,
                        &clip,
                    );
                }

                if entry.collapsed {
                    return;
                }

                if let (Some(body), Some(body_clip)) =
                    (parts.next(), geometry.body.intersection(&clip))
                {
                    renderer.with_layer(body_clip, |renderer| {
                        entry.parts[BODY].as_widget().draw(
                            &tree.children[BODY],
                            renderer,
                            theme,
                            &renderer::Style { text_color: t.text },
                            body,
                            body_cursor,
                            &body_clip,
                        );
                    });
                }

                if entry.resizable {
                    draw_grip(renderer, &t, geometry.bounds);
                }
            });
        }

        if !state.guides.is_empty() {
            let origin = Vector::new(bounds.x, bounds.y);

            renderer.with_layer(clip, |renderer| {
                for guide in &state.guides {
                    draw_guide(renderer, *guide, origin, t.snap());
                }
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
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            let mut layouts = layout.children();
            let mut trees = tree.children.iter_mut();

            if let (Some(base_tree), Some(base_layout)) = (trees.next(), layouts.next()) {
                self.base
                    .as_widget_mut()
                    .operate(base_tree, base_layout, renderer, operation);
            }

            for ((entry, tree), layout) in self.windows.iter_mut().zip(trees).zip(layouts) {
                if entry.collapsed {
                    continue;
                }

                if let Some(body) = layout.children().nth(BODY) {
                    entry.parts[BODY].as_widget_mut().operate(
                        &mut tree.children[BODY],
                        body,
                        renderer,
                        operation,
                    );
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
        let Self { base, windows, .. } = self;
        let mut layouts = layout.children();
        let mut trees = tree.children.iter_mut();
        let mut overlays = Vec::new();

        if let (Some(base_tree), Some(base_layout)) = (trees.next(), layouts.next())
            && let Some(overlay) = base.as_widget_mut().overlay(
                base_tree,
                base_layout,
                renderer,
                viewport,
                translation,
            )
        {
            overlays.push(overlay);
        }

        for ((entry, tree), layout) in windows.iter_mut().zip(trees).zip(layouts) {
            if entry.collapsed {
                continue;
            }

            let Some(body) = layout.children().nth(BODY) else {
                continue;
            };

            if let Some(overlay) = entry.parts[BODY].as_widget_mut().overlay(
                &mut tree.children[BODY],
                body,
                renderer,
                viewport,
                translation,
            ) {
                overlays.push(overlay);
            }
        }

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, K, Message> From<Floating<'a, K, Message>> for Element<'a, Message>
where
    K: Copy + PartialEq + 'static,
    Message: 'a,
{
    fn from(floating: Floating<'a, K, Message>) -> Self {
        Element::new(floating)
    }
}

/// Daraltma ya da açma: başlık yerinde kalır.
fn collapse<K: Copy, Message>(
    entry: &Entry<'_, K, Message>,
    geometry: &Geometry,
    origin: Vector,
    area: Size,
) -> Event<K> {
    let bounds = geometry.bounds - origin;

    Event::Collapsed {
        key: entry.key,
        collapsed: !entry.collapsed,
        placement: Placement {
            x: Anchor::of(bounds.x, bounds.width, area.width),
            y: Anchor::Start(bounds.y.round()),
        },
    }
}

impl<K> Gesture<K> {
    /// İmleç `cursor`'a gelince pencerenin yeni yeri; yakalama açıksa
    /// kılavuzlarıyla birlikte.
    fn follow(
        &mut self,
        cursor: Point,
        area: Size,
        others: &[Rectangle],
        min_size: Size,
        snapping: bool,
    ) -> Vec<Guide> {
        let delta = cursor - self.origin;

        if delta.x != 0.0 || delta.y != 0.0 {
            self.moved = true;
        }

        let (moving, rect) = match self.kind {
            Kind::Move => (
                Moving::BOTH,
                Rectangle {
                    x: self.start.x + delta.x,
                    y: self.start.y + delta.y,
                    ..self.start
                },
            ),
            Kind::Resize(edges) => (
                Moving::of(edges),
                resize(self.start, edges, delta, min_size, area),
            ),
        };

        let (rect, guides) = if snapping {
            snap(rect, moving, area, others, min_size)
        } else {
            (rect, Vec::new())
        };

        self.current = match self.kind {
            Kind::Move => Rectangle {
                x: rect.x.min(area.width - rect.width).max(0.0).round(),
                y: rect.y.min(area.height - rect.height).max(0.0).round(),
                ..rect
            },
            Kind::Resize(_) => Rectangle {
                x: rect.x.round(),
                y: rect.y.round(),
                width: rect.width.round(),
                height: rect.height.round(),
            },
        };

        guides
    }
}

/// Kenarları sürüklenen dikdörtgen: en küçük boyutun altına inmez, alanın
/// dışına taşmaz.
fn resize(start: Rectangle, edges: Edges, delta: Vector, min: Size, area: Size) -> Rectangle {
    let mut left = start.x;
    let mut top = start.y;
    let mut right = start.x + start.width;
    let mut bottom = start.y + start.height;

    if edges.left {
        left = (left + delta.x).min(right - min.width).max(0.0);
    }

    if edges.right {
        right = (right + delta.x).max(left + min.width).min(area.width);
    }

    if edges.top {
        top = (top + delta.y).min(bottom - min.height).max(0.0);
    }

    if edges.bottom {
        bottom = (bottom + delta.y).max(top + min.height).min(area.height);
    }

    Rectangle::new(Point::new(left, top), Size::new(right - left, bottom - top))
}

/// Bir eksende hangi kenarların kıpırdadığı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    None,
    Start,
    End,
    Both,
}

impl Side {
    fn includes(self, side: Side) -> bool {
        self == Side::Both || self == side
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Moving {
    x: Side,
    y: Side,
}

impl Moving {
    const BOTH: Self = Self {
        x: Side::Both,
        y: Side::Both,
    };

    fn of(edges: Edges) -> Self {
        let side = |start, end| match (start, end) {
            (true, false) => Side::Start,
            (false, true) => Side::End,
            (true, true) => Side::Both,
            (false, false) => Side::None,
        };

        Self {
            x: side(edges.left, edges.right),
            y: side(edges.top, edges.bottom),
        }
    }
}

/// Yakalama hedefi: pencerenin bir kenarının oturacağı konum.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Target {
    side: Side,
    at: f32,
    kind: TargetKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TargetKind {
    /// Alanın kenarından [`GAP`] içeride.
    Area,
    /// Başka bir pencerenin aynı kenarıyla hizalı.
    Align(Rectangle),
    /// Başka bir pencerenin yanında, arada [`GAP`] boşlukla.
    Beside(Rectangle),
}

/// Eksen: `false` yatay (x), `true` dikey (y).
fn span(rect: Rectangle, vertical: bool) -> (f32, f32) {
    if vertical {
        (rect.y, rect.y + rect.height)
    } else {
        (rect.x, rect.x + rect.width)
    }
}

fn targets(rect: Rectangle, vertical: bool, area: Size, others: &[Rectangle]) -> Vec<Target> {
    let room = if vertical { area.height } else { area.width };
    let (cross_start, cross_end) = span(rect, !vertical);

    let mut targets = vec![
        Target {
            side: Side::Start,
            at: GAP,
            kind: TargetKind::Area,
        },
        Target {
            side: Side::End,
            at: room - GAP,
            kind: TargetKind::Area,
        },
    ];

    for &other in others {
        let (start, end) = span(other, vertical);
        let (other_start, other_end) = span(other, !vertical);

        targets.push(Target {
            side: Side::Start,
            at: start,
            kind: TargetKind::Align(other),
        });
        targets.push(Target {
            side: Side::End,
            at: end,
            kind: TargetKind::Align(other),
        });

        // Yan yana dizilme yalnızca öbür eksende örtüşen pencerelerde.
        if cross_start <= other_end + SNAP && other_start <= cross_end + SNAP {
            targets.push(Target {
                side: Side::Start,
                at: end + GAP,
                kind: TargetKind::Beside(other),
            });
            targets.push(Target {
                side: Side::End,
                at: start - GAP,
                kind: TargetKind::Beside(other),
            });
        }
    }

    targets
}

/// Bir eksende en yakın yakalama: kenarın kayacağı uzaklık.
fn nearest(rect: Rectangle, vertical: bool, side: Side, targets: &[Target]) -> Option<f32> {
    let (start, end) = span(rect, vertical);

    targets
        .iter()
        .filter(|target| side.includes(target.side))
        .map(|target| {
            target.at
                - match target.side {
                    Side::End => end,
                    _ => start,
                }
        })
        .filter(|distance| distance.abs() <= SNAP)
        .min_by(|a, b| a.abs().total_cmp(&b.abs()))
}

/// Pencereyi en yakın kenarlara yakalar; oturduğu hedeflerin kılavuzlarını
/// da döndürür.
fn snap(
    rect: Rectangle,
    moving: Moving,
    area: Size,
    others: &[Rectangle],
    min_size: Size,
) -> (Rectangle, Vec<Guide>) {
    let mut snapped = rect;

    for (vertical, side) in [(false, moving.x), (true, moving.y)] {
        if side == Side::None {
            continue;
        }

        let targets = targets(rect, vertical, area, others);
        let Some(distance) = nearest(rect, vertical, side, &targets) else {
            continue;
        };

        let (start, end) = span(snapped, vertical);
        let min = if vertical {
            min_size.height
        } else {
            min_size.width
        };

        let (start, end) = match side {
            Side::Both => (start + distance, end + distance),
            Side::Start if end - (start + distance) >= min => (start + distance, end),
            Side::End if (end + distance) - start >= min => (start, end + distance),
            _ => continue,
        };

        if vertical {
            snapped.y = start;
            snapped.height = end - start;
        } else {
            snapped.x = start;
            snapped.width = end - start;
        }
    }

    let mut guides = Vec::new();

    for (vertical, side) in [(false, moving.x), (true, moving.y)] {
        if side == Side::None {
            continue;
        }

        let (start, end) = span(snapped, vertical);
        let (cross_start, cross_end) = span(snapped, !vertical);
        let cross_room = if vertical { area.width } else { area.height };

        for target in targets(snapped, vertical, area, others) {
            let edge = match target.side {
                Side::End => end,
                _ => start,
            };

            if !side.includes(target.side) || (edge - target.at).abs() > 0.5 {
                continue;
            }

            let (at, from, to) = match target.kind {
                TargetKind::Area => (target.at, 0.0, cross_room),
                TargetKind::Align(other) => {
                    let (other_start, other_end) = span(other, !vertical);

                    (
                        target.at,
                        cross_start.min(other_start),
                        cross_end.max(other_end),
                    )
                }
                TargetKind::Beside(other) => {
                    let (other_start, other_end) = span(other, !vertical);
                    let middle = match target.side {
                        Side::End => target.at + GAP / 2.0,
                        _ => target.at - GAP / 2.0,
                    };

                    (
                        middle,
                        cross_start.max(other_start),
                        cross_end.min(other_end),
                    )
                }
            };

            let at = at.round();

            // Aynı çizgideki kılavuzlar (ör. alanın kenarı ve komşu pencere)
            // tek çizgide birleşir.
            match guides
                .iter_mut()
                .find(|guide: &&mut Guide| guide.vertical != vertical && guide.at == at)
            {
                Some(guide) => {
                    guide.from = guide.from.min(from);
                    guide.to = guide.to.max(to);
                }
                None if to > from => guides.push(Guide {
                    vertical: !vertical,
                    at,
                    from,
                    to,
                }),
                None => {}
            }
        }
    }

    (snapped, guides)
}

/// Pencerenin gövdesi, kenarı, gölgesi, başlık çubuğu ve öndeyse vurgu
/// çizgisi.
fn draw_frame<K, Message>(
    renderer: &mut Renderer,
    t: &Tokens,
    geometry: &Geometry,
    entry: &Entry<'_, K, Message>,
    lifted: bool,
) {
    let bounds = geometry.bounds;

    // Sürüklenen pencere havaya kalkar: gölgesi derinleşir.
    let shadow = if lifted {
        Shadow {
            color: t.shadow(),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        }
    } else {
        Shadow {
            color: t.shadow(),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 12.0,
        }
    };

    renderer.fill_quad(
        Quad {
            bounds,
            border: Border {
                color: t.border,
                width: 1.0,
                radius: RADIUS.into(),
            },
            shadow,
            ..Quad::default()
        },
        Background::Color(t.popover),
    );

    let bar_height = if entry.collapsed {
        geometry.bar - 2.0
    } else {
        geometry.bar - 1.0
    };

    renderer.fill_quad(
        Quad {
            bounds: Rectangle {
                x: bounds.x + 1.0,
                y: bounds.y + 1.0,
                width: bounds.width - 2.0,
                height: bar_height.max(0.0),
            },
            border: border::rounded(border::top(RADIUS - 1.0)),
            ..Quad::default()
        },
        Background::Color(t.header),
    );

    if !entry.collapsed {
        fill(
            renderer,
            Rectangle {
                x: bounds.x + 1.0,
                y: bounds.y + geometry.bar,
                width: bounds.width - 2.0,
                height: 1.0,
            },
            t.border,
        );
    }

    if entry.active {
        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    height: 2.0,
                    ..bounds
                },
                border: border::rounded(border::top(RADIUS)),
                ..Quad::default()
            },
            Background::Color(t.accent),
        );
    }
}

/// Boyutlandırılabilen pencerenin sağ alt köşesindeki nokta üçgeni.
fn draw_grip(renderer: &mut Renderer, t: &Tokens, bounds: Rectangle) {
    let right = bounds.x + bounds.width;
    let bottom = bounds.y + bounds.height;
    let color = t.muted.scale_alpha(0.7);

    for (column, row) in [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (0, 2)] {
        fill(
            renderer,
            Rectangle {
                x: right - 5.0 - 4.0 * column as f32,
                y: bottom - 5.0 - 4.0 * row as f32,
                width: 2.0,
                height: 2.0,
            },
            color,
        );
    }
}

/// Noktalı kılavuz çizgisi: model alanındaki yakalama izleri gibi.
fn draw_guide(renderer: &mut Renderer, guide: Guide, origin: Vector, color: Color) {
    const DASH: f32 = 3.0;
    const SPACE: f32 = 3.0;

    let mut position = guide.from;

    while position < guide.to {
        let length = DASH.min(guide.to - position);

        let rectangle = if guide.vertical {
            Rectangle {
                x: origin.x + guide.at,
                y: origin.y + position,
                width: 1.0,
                height: length,
            }
        } else {
            Rectangle {
                x: origin.x + position,
                y: origin.y + guide.at,
                width: length,
                height: 1.0,
            }
        };

        fill(renderer, rectangle, color);
        position += DASH + SPACE;
    }
}

fn fill(renderer: &mut Renderer, bounds: Rectangle, color: Color) {
    renderer.fill_quad(
        Quad {
            bounds,
            ..Quad::default()
        },
        Background::Color(color),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Size = Size::new(1000.0, 600.0);

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rectangle {
        Rectangle::new(Point::new(x, y), Size::new(width, height))
    }

    #[test]
    fn placements_follow_their_nearest_edges() {
        let size = Size::new(200.0, 100.0);

        assert_eq!(
            Placement::top_right(16.0, 16.0).resolve(AREA, size),
            Point::new(784.0, 16.0)
        );
        assert_eq!(
            Placement::bottom_left(16.0, 24.0).resolve(AREA, size),
            Point::new(16.0, 476.0)
        );

        // Pencere alanın dışına taşmaz.
        assert_eq!(
            Placement::top_left(950.0, -40.0).resolve(AREA, size),
            Point::new(800.0, 0.0)
        );

        // Sağ alta bırakılan pencere sağ alt köşeye göre saklanır; alan
        // büyüyünce köşeyle birlikte kayar.
        let placement = Placement::of(rect(700.0, 400.0, 200.0, 100.0), AREA);
        assert_eq!(placement, Placement::bottom_right(100.0, 100.0));
        assert_eq!(
            placement.resolve(Size::new(1200.0, 700.0), size),
            Point::new(900.0, 500.0)
        );
    }

    #[test]
    fn windows_open_raise_and_close() {
        let mut windows = Windows::new();

        windows.open('a', Placement::top_left(16.0, 16.0));
        windows.open('b', Placement::top_left(16.0, 16.0));

        // Aynı yere açılan pencere basamaklanır.
        assert_eq!(
            windows.get('b').map(|window| window.placement),
            Some(Placement::top_left(40.0, 40.0))
        );
        assert_eq!(windows.front(), Some('b'));

        windows.update(Event::Raised('a'));
        assert_eq!(windows.front(), Some('a'));

        // Açık pencereyi yeniden açmak yalnızca öne getirir.
        windows.open('b', Placement::top_left(500.0, 500.0));
        assert_eq!(windows.front(), Some('b'));
        assert_eq!(windows.len(), 2);

        windows.update(Event::Collapsed {
            key: 'b',
            collapsed: true,
            placement: Placement::top_left(40.0, 40.0),
        });
        assert!(windows.get('b').is_some_and(|window| window.collapsed));

        windows.update(Event::Closed('b'));
        assert!(!windows.is_open('b'));
        assert!(!windows.toggle('a', Placement::top_left(0.0, 0.0)));
        assert!(windows.is_empty());
    }

    #[test]
    fn dragged_windows_snap_to_the_area_and_each_other() {
        // Alanın sol kenarına 5 piksel kala: kenardan 8 piksel içeride durur.
        let (snapped, guides) = snap(
            rect(13.0, 200.0, 200.0, 100.0),
            Moving::BOTH,
            AREA,
            &[],
            Size::ZERO,
        );
        assert_eq!(snapped.x, GAP);
        assert_eq!(
            guides,
            vec![Guide {
                vertical: true,
                at: GAP,
                from: 0.0,
                to: AREA.height
            }]
        );

        // Başka bir pencerenin altına, sol kenarları hizalı.
        let other = rect(300.0, 40.0, 240.0, 120.0);
        let (snapped, guides) = snap(
            rect(305.0, 172.0, 200.0, 100.0),
            Moving::BOTH,
            AREA,
            &[other],
            Size::ZERO,
        );
        assert_eq!(snapped.position(), Point::new(300.0, 168.0));
        assert!(
            guides
                .iter()
                .any(|guide| guide.vertical && guide.at == 300.0)
        );
        assert!(
            guides
                .iter()
                .any(|guide| !guide.vertical && guide.at == 164.0)
        );

        // Üst kenar hem alanın kenar boşluğuna hem komşunun üstüne oturur:
        // kılavuzlar tek çizgide birleşir.
        let neighbour = rect(600.0, GAP, 200.0, 140.0);
        let (_, guides) = snap(
            rect(300.0, 12.0, 200.0, 100.0),
            Moving::BOTH,
            AREA,
            &[neighbour],
            Size::ZERO,
        );
        assert_eq!(
            guides,
            vec![Guide {
                vertical: false,
                at: GAP,
                from: 0.0,
                to: AREA.width
            }]
        );

        // Uzaktaki kenarlar yakalanmaz.
        let (free, guides) = snap(
            rect(400.0, 300.0, 200.0, 100.0),
            Moving::BOTH,
            AREA,
            &[],
            Size::ZERO,
        );
        assert_eq!(free.position(), Point::new(400.0, 300.0));
        assert!(guides.is_empty());
    }

    #[test]
    fn resizing_keeps_the_minimum_size_and_snaps_the_moving_edge() {
        let start = rect(100.0, 100.0, 300.0, 200.0);
        let min = Size::new(200.0, 96.0);

        let left = Edges {
            left: true,
            ..Edges::default()
        };
        assert_eq!(
            resize(start, left, Vector::new(250.0, 0.0), min, AREA),
            rect(200.0, 100.0, 200.0, 200.0)
        );

        let corner = Edges {
            right: true,
            bottom: true,
            ..Edges::default()
        };
        assert_eq!(
            resize(start, corner, Vector::new(900.0, 900.0), min, AREA),
            rect(100.0, 100.0, 900.0, 500.0)
        );

        // Sağ kenar alanın kenarına yaklaşınca yakalanır; sol kenar yerinde.
        let (snapped, _) = snap(
            rect(100.0, 100.0, 887.0, 200.0),
            Moving::of(corner),
            AREA,
            &[],
            min,
        );
        assert_eq!(snapped.x, 100.0);
        assert_eq!(snapped.x + snapped.width, AREA.width - GAP);
    }

    #[test]
    fn gestures_follow_the_cursor_inside_the_area() {
        let mut gesture = Gesture {
            key: 'a',
            kind: Kind::Move,
            origin: Point::new(50.0, 10.0),
            start: rect(40.0, 0.0, 200.0, 100.0),
            current: rect(40.0, 0.0, 200.0, 100.0),
            moved: false,
        };

        // Yakalama kapalı: imleç ne kadar kıpırdadıysa o kadar.
        let guides = gesture.follow(Point::new(250.5, 130.0), AREA, &[], Size::ZERO, false);
        assert!(gesture.moved);
        assert!(guides.is_empty());
        assert_eq!(gesture.current.position(), Point::new(241.0, 120.0));

        // Alanın dışına sürüklenen pencere kenarda kalır.
        gesture.follow(Point::new(-400.0, 900.0), AREA, &[], Size::ZERO, true);
        assert_eq!(gesture.current.position(), Point::new(0.0, 500.0));
    }

    #[test]
    fn edges_pick_the_resize_cursor() {
        let edges = |left, right, top, bottom| Edges {
            left,
            right,
            top,
            bottom,
        };

        assert_eq!(
            edges(true, false, false, false).interaction(),
            mouse::Interaction::ResizingHorizontally
        );
        assert_eq!(
            edges(false, false, false, true).interaction(),
            mouse::Interaction::ResizingVertically
        );
        assert_eq!(
            edges(false, true, false, true).interaction(),
            mouse::Interaction::ResizingDiagonallyDown
        );
        assert_eq!(
            edges(false, true, true, false).interaction(),
            mouse::Interaction::ResizingDiagonallyUp
        );
    }
}
