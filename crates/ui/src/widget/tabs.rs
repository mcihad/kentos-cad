//! Belge sekmeleri: açık çizimler ya da AutoCAD'in Model / Düzen sekmeleri
//! gibi aynı alanı paylaşan görünümler.
//!
//! - Etkin sekme bağlandığı içerikle aynı zemindedir ve vurgu çizgisi
//!   taşır; şeridin kenar çizgisi etkin sekmenin altında kesilir.
//! - Kapatma düğmesi etkin sekmede ve üzerine gelinen sekmede görünür.
//!   Kaydedilmemiş sekmede yerinde bir nokta durur. Orta tık da kapatır.
//! - Sekmeler sürüklenerek sıralanır ([`Tabs::on_reorder`]).
//! - Sekmeler daralmaz: her biri başlığı kadar geniştir (docs/adr/0806). Sığmayan şerit kayar: sağda `‹ ›` okları ve
//!   bütün sekmelerin listesi çıkar, tekerlek de kaydırır; kesilen kenar solar, seçilen sekme görünür yapılır.
//!   Yalnız en geniş sınırı aşan başlık kısılır, sonu solar.
//! - [`Tabs::bottom`] ile şerit içeriğin altına asılır.
//!
//! ```ignore
//! column![
//!     Tabs::new(
//!         self.sheets.iter().map(|sheet| Tab::new(&sheet.name).dirty(sheet.dirty)),
//!         self.current,
//!         Message::SheetSelected,
//!     )
//!     .on_close(Message::SheetClosed)
//!     .on_reorder(Message::SheetMoved)
//!     .on_new(Message::SheetAdded),
//!     sheet,
//! ]
//! ```

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::gradient::Linear;
use iced::widget::text::{Fragment, IntoFragment, Wrapping};
use iced::widget::{container, row};
use iced::{
    Background, Center, Color, Degrees, Element, Event, Gradient, Length, Point, Rectangle,
    Renderer, Size, Theme, Vector, border, mouse,
};

use crate::icon::{Icon, icon};
use crate::label;
use crate::style;
use crate::theme::accent::mix;
use crate::theme::{Tokens, typography};
use crate::widget::context_menu::{Menu, MenuButton};

/// Sekme genişliğinin sınırları ve şeridin okları, 12 piksellik gövde metnine göre.
pub(crate) const MIN_WIDTH: f32 = 64.0;
pub(crate) const MAX_WIDTH: f32 = 200.0;
const ARROW: f32 = 20.0;
/// Tekerleğin bir çentiğinin kaydırdığı.
const WHEEL: f32 = 48.0;
/// Şeridin kesilen kenarının solan genişliği.
const EDGE: f32 = 18.0;

/// Başlığın solundaki boşluk; kapatma düğmesi olmayan sekmede sağında da.
pub(crate) const PAD: f32 = 12.0;
/// Kapatma düğmesinin tıklama alanı, glifi ve sekmenin kenarına uzaklığı.
pub(crate) const CLOSE: f32 = 18.0;
pub(crate) const GLYPH: f32 = 10.0;
const CLOSE_MARGIN: f32 = 5.0;
/// Başlıkla kapatma düğmesi arası.
const GAP: f32 = 4.0;
/// Sığmayan başlığın solan ucu.
const FADE: f32 = 20.0;

/// Sürüklemenin başladığı uzaklık.
pub(crate) const DRAG: f32 = 5.0;

/// Şeridin yüksekliği, geçerli yazı boyutunda: satırların modülü (docs/adr/0804).
pub fn height() -> f32 {
    crate::theme::metrics::row()
}

/// Şeridin bir okunun (ve listesinin) genişliği, geçerli yazı boyutunda.
pub(crate) fn arrow_width() -> f32 {
    typography::scaled(ARROW)
}

/// Bir sekme.
pub struct Tab<'a> {
    title: Fragment<'a>,
    icon: Option<Icon>,
    dirty: bool,
    closable: bool,
    badge: usize,
    note: bool,
}

impl<'a> Tab<'a> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            icon: None,
            dirty: false,
            closable: true,
            badge: 0,
            note: false,
        }
    }

    /// Başlığın yanında bilgi mavisi küçük nokta: sekmenin söyleyecek bir
    /// şeyi var (ör. paftanın şablonunun yeni sürümü; web'in
    /// `.sheet-tab__newer`'ı, 6 piksel).
    pub fn note(mut self, note: bool) -> Self {
        self.note = note;
        self
    }

    /// Başlığın yanında sayı rozeti (ör. görülmemiş uyarılar): amber zeminde
    /// kalın, küçük rakamlar (web'in `.badge`'i); 0 ise rozet yoktur.
    pub fn badge(mut self, count: usize) -> Self {
        self.badge = count;
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Kaydedilmemiş değişiklik var: kapatma düğmesinin yerinde nokta.
    pub fn dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }

    /// Kapatılabilir mi (varsayılan evet); ör. Model sekmesi kapanmaz.
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }
}

/// Sekme şeridi.
pub struct Tabs<'a, Message> {
    tabs: Vec<Entry<'a, Message>>,
    active: usize,
    bottom: bool,
    on_select: Box<dyn Fn(usize) -> Message + 'a>,
    on_close: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_reorder: Option<Box<dyn Fn(usize, usize) -> Message + 'a>>,
    on_new: Option<Message>,
    /// Yeni sekme ikonu.
    plus: Element<'a, Message>,
    /// Bütün sekmelerin listesi: şerit sığmayınca.
    list: Element<'a, Message>,
    /// Şeridin okları: sola, sağa.
    prev: Element<'a, Message>,
    next: Element<'a, Message>,
}

struct Entry<'a, Message> {
    closable: bool,
    dirty: bool,
    /// İkon ve başlık; kapatma ikonu.
    parts: [Element<'a, Message>; 2],
}

impl<Message> Entry<'_, Message> {
    fn trail(&self) -> f32 {
        trail(self.closable || self.dirty)
    }

    fn room(&self, tab: Rectangle) -> Rectangle {
        room(tab, self.trail())
    }
}

/// Başlığın sağındaki boşluk: kapatma düğmesi ya da nokta için yer
/// ayrılır.
pub(crate) fn trail(button: bool) -> f32 {
    if button {
        GAP + CLOSE + CLOSE_MARGIN
    } else {
        PAD
    }
}

/// Başlığın sığması gereken alan.
pub(crate) fn room(tab: Rectangle, trail: f32) -> Rectangle {
    Rectangle {
        x: tab.x + PAD,
        width: (tab.width - PAD - trail).max(0.0),
        ..tab
    }
}

/// Sekmenin bilgi noktası: 6 piksel, bilgi mavisi.
fn note_dot<'a, Message: 'a>() -> Element<'a, Message> {
    container(iced::widget::space().width(6).height(6))
        .style(|theme: &iced::Theme| iced::widget::container::Style {
            background: Some(Tokens::of(theme).info.into()),
            border: iced::Border {
                radius: 3.0.into(),
                ..iced::Border::default()
            },
            ..iced::widget::container::Style::default()
        })
        .into()
}

/// Sekmenin sayı rozeti: 16 piksel yüksek, en az 16 piksel geniş hap.
fn badge<'a, Message: 'a>(count: usize) -> Element<'a, Message> {
    let digits = count.to_string();
    let wide = digits.len() > 1;
    let pill = container(
        iced::widget::text(digits)
            .font(typography::ui_strong())
            .size(typography::caption() - 1.0)
            .line_height(1.0),
    )
    .height(16)
    .align_y(Center)
    .style(style::container::warning_count);

    if wide {
        pill.padding([0, 4]).into()
    } else {
        pill.center_x(16).into()
    }
}

impl<'a, Message: Clone + 'a> Tabs<'a, Message> {
    pub fn new(
        tabs: impl IntoIterator<Item = Tab<'a>>,
        active: usize,
        on_select: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        let tabs: Vec<Tab<'a>> = tabs.into_iter().collect();
        let choices: Vec<(String, Message)> = tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| (tab.title.to_string(), on_select(index)))
            .collect();

        let list = MenuButton::new(
            container(icon(Icon::ChevronDown).size(12.0))
                .center_x(arrow_width())
                .center_y(height()),
            move || {
                choices
                    .iter()
                    .enumerate()
                    .fold(Menu::new(), |menu, (index, (title, message))| {
                        menu.check(title.clone(), index == active, message.clone())
                    })
            },
        );

        let tabs = tabs
            .into_iter()
            .map(|tab| {
                let mut heading = row![].spacing(6).align_y(Center);

                if let Some(glyph) = tab.icon {
                    heading = heading.push(icon(glyph).size(14.0));
                }

                heading = heading.push(label::text(tab.title).wrapping(Wrapping::None));

                if tab.badge > 0 {
                    heading = heading.push(badge(tab.badge));
                }

                if tab.note {
                    heading = heading.push(note_dot());
                }

                Entry {
                    closable: tab.closable,
                    dirty: tab.dirty,
                    parts: [heading.into(), icon(Icon::Close).size(GLYPH).into()],
                }
            })
            .collect();

        Self {
            tabs,
            active,
            bottom: false,
            on_select: Box::new(on_select),
            on_close: None,
            on_reorder: None,
            on_new: None,
            plus: icon(Icon::Plus).size(12.0).into(),
            list: list.into(),
            prev: icon(Icon::ChevronLeft).size(12.0).into(),
            next: icon(Icon::ChevronRight).size(12.0).into(),
        }
    }

    /// Kapatma düğmesi ve orta tık.
    pub fn on_close(mut self, on_close: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_close = Some(Box::new(on_close));
        self
    }

    /// Sekmeler sürüklenerek sıralanır: `(nereden, nereye)`; `nereye`,
    /// sekme listeden çıkarıldıktan sonra ekleneceği sıradır
    /// (`tabs.insert(to, tabs.remove(from))`).
    pub fn on_reorder(mut self, on_reorder: impl Fn(usize, usize) -> Message + 'a) -> Self {
        self.on_reorder = Some(Box::new(on_reorder));
        self
    }

    /// Son sekmenin yanındaki + düğmesi.
    pub fn on_new(mut self, message: Message) -> Self {
        self.on_new = Some(message);
        self
    }

    /// Şerit içeriğin altında durur; vurgu çizgisi etkin sekmenin altına
    /// geçer.
    pub fn bottom(mut self) -> Self {
        self.bottom = true;
        self
    }
}

/// Şeridin kayması: sekmeler daralmaz, sığmayan şerit kayar (docs/adr/0806). Etkin sekme değişince ya da şeridin yeri
/// değişince etkin sekme görünür yapılır; okuyan kişinin kaydırması başka zaman geri alınmaz.
#[derive(Debug, Clone, Default)]
pub(crate) struct Scroll {
    offset: f32,
    active: Option<usize>,
    room: f32,
    widths: Vec<f32>,
}

impl Scroll {
    /// Bu karenin sekmeleri ve yeri: sekmelerin başlangıcının ne kadar sola kaydığı.
    pub(crate) fn settle(&mut self, widths: &[f32], active: usize, room: f32) -> f32 {
        let changed = self.active != Some(active) || (self.room - room).abs() > 0.5;
        self.widths.clear();
        self.widths.extend_from_slice(widths);
        self.room = room;
        if changed {
            self.active = Some(active);
            self.offset = self.revealing(active);
        }
        self.offset = self.offset.clamp(0.0, self.most());
        self.offset
    }

    fn total(&self) -> f32 {
        self.widths.iter().sum()
    }

    fn most(&self) -> f32 {
        (self.total() - self.room).max(0.0)
    }

    /// Sola kayacak yer var mı.
    pub(crate) fn can_back(&self) -> bool {
        self.offset > 0.5
    }

    /// Sağa kayacak yer var mı.
    pub(crate) fn can_forth(&self) -> bool {
        self.offset < self.most() - 0.5
    }

    /// Sekmeyi bütünüyle gösteren kayma; zaten görünüyorsa aynısı.
    fn revealing(&self, index: usize) -> f32 {
        let Some(width) = self.widths.get(index) else {
            return self.offset;
        };
        let left: f32 = self.widths[..index].iter().sum();
        let right = left + width;
        if left < self.offset || *width > self.room {
            left
        } else if right > self.offset + self.room {
            right - self.room
        } else {
            self.offset
        }
    }

    /// Bu sekmeyi bir sonraki yerleşimde görünür yapar (kısmen görünene basıldı).
    pub(crate) fn reveal(&mut self, index: usize) -> bool {
        let offset = self.revealing(index);
        let moved = (offset - self.offset).abs() > 0.5;
        self.offset = offset;
        moved
    }

    /// Bir sekme ileri ya da geri: her basış tam bir sekme kaydırır, sekmenin başı şeridin soluna oturur. Kaydıysa doğru.
    pub(crate) fn step(&mut self, forth: bool) -> bool {
        let before = self.offset;
        let mut left = 0.0;
        let mut target = None;
        for width in &self.widths {
            if forth {
                if left > self.offset + 0.5 {
                    target = Some(left);
                    break;
                }
            } else if left < self.offset - 0.5 {
                target = Some(left);
            }
            left += width;
        }
        if let Some(target) = target {
            self.offset = target.clamp(0.0, self.most());
        }
        (self.offset - before).abs() > 0.5
    }

    /// Tekerlek: yukarı ya da sola çevirmek başa doğru götürür. Kaydıysa doğru.
    pub(crate) fn wheel(&mut self, delta: mouse::ScrollDelta) -> bool {
        let (x, y) = match delta {
            mouse::ScrollDelta::Lines { x, y } => (x * WHEEL, y * WHEEL),
            mouse::ScrollDelta::Pixels { x, y } => (x, y),
        };
        let amount = if x.abs() > y.abs() { x } else { y };
        let before = self.offset;
        self.offset = (self.offset - amount).clamp(0.0, self.most());
        (self.offset - before).abs() > 0.5
    }
}

/// Sekmenin bölgesi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Tab(usize),
    Close(usize),
    New,
    /// Şeridin okları.
    Prev,
    Next,
}

impl Target {
    fn tab(self) -> Option<usize> {
        match self {
            Target::Tab(index) | Target::Close(index) => Some(index),
            Target::New | Target::Prev | Target::Next => None,
        }
    }
}

#[derive(Debug, Default)]
struct State {
    hovered: Option<Target>,
    pressed: Option<Target>,
    /// Basılan sekme ve basıldığı yer; imleç eşiği aşınca sürüklenir.
    origin: Option<(usize, Point)>,
    dragging: Option<usize>,
    /// Sürüklenen sekmenin bırakılacağı sıra.
    drop: Option<usize>,
    /// Şeridin kayması.
    scroll: Scroll,
    /// Sekmelerin göründüğü genişlik.
    room: f32,
    /// Sekmeler sığmıyor: oklar ve liste gösterilir.
    overflow: bool,
    /// Başlığı sığmayan sekmeler; başlığın sonu solar.
    clipped: Vec<bool>,
}

/// Yerleşimden okunan bölgeler.
struct Geometry {
    /// Sekmelerin göründüğü şerit; dışında kalan sekme parçası basılmaz.
    strip: Rectangle,
    /// Görünen sekmeler: sırası ve sınırı.
    tabs: Vec<(usize, Rectangle)>,
    closes: Vec<(usize, Rectangle)>,
    new: Option<Rectangle>,
    /// Şeridin okları: şerit sığmıyorsa.
    arrows: Option<(Rectangle, Rectangle)>,
}

impl Geometry {
    fn of<Message>(tabs: &Tabs<'_, Message>, layout: Layout<'_>, state: &State) -> Self {
        let bounds = layout.bounds();
        let height = bounds.height;
        let mut children = layout.children();
        let plus = children.next();
        let _list = children.next();
        let prev = children.next();
        let next = children.next();
        let slot = |glyph: Option<Layout<'_>>| {
            let center = glyph.map_or(Point::ORIGIN, |glyph| glyph.bounds().center());
            let width = arrow_width();
            Rectangle::new(
                Point::new(center.x - width / 2.0, bounds.y),
                Size::new(width, height),
            )
        };
        let arrows = state.overflow.then(|| (slot(prev), slot(next)));

        let mut visible = Vec::new();
        let mut closes = Vec::new();

        for (index, (entry, tab)) in tabs.tabs.iter().zip(children).enumerate() {
            let bounds = tab.bounds();

            if bounds.width <= 0.0 {
                continue;
            }

            visible.push((index, bounds));

            if entry.closable {
                closes.push((index, close_area(bounds)));
            }
        }

        Self {
            strip: Rectangle {
                width: state.room,
                ..bounds
            },
            tabs: visible,
            closes,
            new: plus
                .filter(|_| tabs.on_new.is_some())
                .map(|plus| square(plus.bounds().center(), height)),
            arrows,
        }
    }

    fn target(&self, point: Point) -> Option<Target> {
        if let Some((prev, next)) = self.arrows {
            if prev.contains(point) {
                return Some(Target::Prev);
            }
            if next.contains(point) {
                return Some(Target::Next);
            }
        }

        if self.strip.contains(point) {
            if let Some((index, _)) = self.closes.iter().find(|(_, area)| area.contains(point)) {
                return Some(Target::Close(*index));
            }

            if let Some((index, _)) = self.tabs.iter().find(|(_, tab)| tab.contains(point)) {
                return Some(Target::Tab(*index));
            }
        }

        self.new
            .filter(|new| new.contains(point))
            .map(|_| Target::New)
    }

    /// Sürüklenen sekmenin bırakılacağı sıra: imlecin ortasını geçtiği
    /// sekmelere göre.
    fn drop_index(&self, from: usize, x: f32) -> usize {
        let first = self.tabs.first().map_or(0, |(index, _)| *index);
        let slot = first
            + self
                .tabs
                .iter()
                .filter(|(_, tab)| x > tab.center_x())
                .count();

        if slot > from { slot - 1 } else { slot }
    }

    /// Bırakma sırasının şeritteki yatay konumu; sekme yerinden
    /// oynamıyorsa yok.
    fn marker(&self, from: usize, to: usize) -> Option<f32> {
        if from == to {
            return None;
        }

        let first = self.tabs.first().map_or(0, |(index, _)| *index);
        let rest: Vec<Rectangle> = self
            .tabs
            .iter()
            .filter(|(index, _)| *index != from)
            .map(|(_, tab)| *tab)
            .collect();

        match rest.get(to.checked_sub(first)?) {
            Some(tab) => Some(tab.x),
            None => rest.last().map(|tab| tab.x + tab.width),
        }
    }
}

/// Sekmenin kapatma düğmesinin tıklama alanı.
pub(crate) fn close_area(tab: Rectangle) -> Rectangle {
    square(
        Point::new(
            tab.x + tab.width - CLOSE_MARGIN - CLOSE / 2.0,
            tab.center_y(),
        ),
        CLOSE,
    )
}

pub(crate) fn square(center: Point, side: f32) -> Rectangle {
    Rectangle::new(
        Point::new(center.x - side / 2.0, center.y - side / 2.0),
        Size::new(side, side),
    )
}

/// Sekmelerden önceki parçalar: yeni sekme, liste ve iki ok.
const EXTRAS: usize = 4;

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Tabs<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        [
            Tree::new(&self.plus),
            Tree::new(&self.list),
            Tree::new(&self.prev),
            Tree::new(&self.next),
        ]
        .into_iter()
        .chain(self.tabs.iter().map(|entry| Tree {
            tag: tree::Tag::stateless(),
            state: tree::State::None,
            children: entry.parts.iter().map(Tree::new).collect(),
        }))
        .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.children
            .resize_with(EXTRAS + self.tabs.len(), Tree::empty);
        tree.children[0].diff(&self.plus);
        tree.children[1].diff(&self.list);
        tree.children[2].diff(&self.prev);
        tree.children[3].diff(&self.next);

        for (tab, entry) in tree.children[EXTRAS..].iter_mut().zip(&self.tabs) {
            tab.diff_children(&entry.parts);
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(height()))
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let height = height();
        let width = limits.resolve(Length::Fill, height, Size::ZERO).width;
        let (min, max) = (typography::scaled(MIN_WIDTH), typography::scaled(MAX_WIDTH));
        let (extras, trees) = tree.children.split_at_mut(EXTRAS);

        // Başlıkların doğal boyutları ve sekmelerin genişlikleri.
        let labels: Vec<Node> = self
            .tabs
            .iter_mut()
            .zip(trees.iter_mut())
            .map(|(entry, tree)| {
                entry.parts[0].as_widget_mut().layout(
                    &mut tree.children[0],
                    renderer,
                    &layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, height)),
                )
            })
            .collect();
        let natural: Vec<f32> = self
            .tabs
            .iter()
            .zip(&labels)
            .map(|(entry, label)| (PAD + label.size().width + entry.trail()).clamp(min, max))
            .collect();

        // Sekmeler daralmaz: sığmayan şerit kayar, oklar ve liste sağda durur.
        let new = if self.on_new.is_some() { height } else { 0.0 };
        let arrow = arrow_width();
        let overflow = natural.iter().sum::<f32>() + new > width;
        let controls = if overflow { 3.0 * arrow } else { 0.0 };
        let room = (width - new - controls).max(0.0);
        let widths: Vec<f32> = natural.iter().map(|tab| tab.min(room)).collect();

        let state = tree.state.downcast_mut::<State>();
        let offset = state.scroll.settle(&widths, self.active, room);
        state.room = room;
        state.overflow = overflow;
        state.clipped.clear();

        let mut x = -offset;
        let mut tabs = Vec::with_capacity(self.tabs.len());

        for ((entry, tree), (label, width)) in self
            .tabs
            .iter_mut()
            .zip(trees.iter_mut())
            .zip(labels.into_iter().zip(widths))
        {
            // Genişlik başlıktan hesaplandığı için yuvarlama payı bırakılır.
            let tab = Rectangle::with_size(Size::new(width, height));
            state
                .clipped
                .push(label.size().width > entry.room(tab).width + 0.5);

            // Şeridin dışında kalan sekme çizilmez.
            if x + width <= 0.0 || x >= room {
                tabs.push(Node::with_children(
                    Size::ZERO,
                    vec![Node::default(), Node::default()],
                ));
                x += width;
                continue;
            }

            let label_y = ((height - label.size().height) / 2.0).round();
            let glyph = entry.parts[1]
                .as_widget_mut()
                .layout(
                    &mut tree.children[1],
                    renderer,
                    &layout::Limits::new(Size::ZERO, Size::new(GLYPH, GLYPH)),
                )
                .move_to(square(close_area(tab).center(), GLYPH).position());

            tabs.push(
                Node::with_children(
                    tab.size(),
                    vec![label.move_to(Point::new(PAD, label_y)), glyph],
                )
                .move_to(Point::new(x, 0.0)),
            );
            x += width;
        }

        let plus = self.plus.as_widget_mut().layout(
            &mut extras[0],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(height, height)),
        );
        let side = plus.size().width;
        // Şerit kayarken + okların yanında durur.
        let plus_x = if overflow { room } else { x };
        let plus =
            plus.move_to(square(Point::new(plus_x + new / 2.0, height / 2.0), side).position());

        let glyph = |element: &mut Element<'a, Message>, tree: &mut Tree, center: f32| {
            let node = element.as_widget_mut().layout(
                tree,
                renderer,
                &layout::Limits::new(Size::ZERO, Size::new(arrow, height)),
            );
            let size = node.size();
            node.move_to(Point::new(
                center - size.width / 2.0,
                (height - size.height) / 2.0,
            ))
        };
        let (list, prev, next) = if overflow {
            let (list_tree, rest) = extras[1..].split_at_mut(1);
            let (prev_tree, next_tree) = rest.split_at_mut(1);
            let end = width - 3.0 * arrow;
            (
                glyph(&mut self.list, &mut list_tree[0], end + 2.5 * arrow),
                glyph(&mut self.prev, &mut prev_tree[0], end + 0.5 * arrow),
                glyph(&mut self.next, &mut next_tree[0], end + 1.5 * arrow),
            )
        } else {
            (Node::default(), Node::default(), Node::default())
        };

        Node::with_children(
            Size::new(width, height),
            [plus, list, prev, next].into_iter().chain(tabs).collect(),
        )
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
        // Sığmayan sekmelerin listesi kendi olaylarını işler.
        if tree.state.downcast_ref::<State>().overflow
            && let Some(list) = layout.children().nth(1)
        {
            self.list.as_widget_mut().update(
                &mut tree.children[1],
                event,
                list,
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

        let state = tree.state.downcast_mut::<State>();
        let geometry = Geometry::of(self, layout, state);
        let target = cursor
            .position_over(layout.bounds())
            .and_then(|point| geometry.target(point));

        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if state.overflow && cursor.is_over(layout.bounds()) && state.scroll.wheel(*delta) {
                    shell.invalidate_layout();
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some((index, origin)) = state.origin
                    && state.dragging.is_none()
                    && self.on_reorder.is_some()
                    && position.distance(origin) > DRAG
                {
                    state.dragging = Some(index);
                }

                if let Some(from) = state.dragging {
                    state.drop = Some(geometry.drop_index(from, position.x));
                    state.hovered = None;
                    shell.request_redraw();
                    shell.capture_event();
                } else if state.hovered != target {
                    state.hovered = target;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorLeft) => {
                if state.hovered.take().is_some() {
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(target) = target else {
                    return;
                };

                // Oklar şeridi birer sekme kaydırır.
                if matches!(target, Target::Prev | Target::Next) {
                    if state.scroll.step(target == Target::Next) {
                        shell.invalidate_layout();
                        shell.request_redraw();
                    }
                    shell.capture_event();
                    return;
                }

                // Sekme basılınca seçilir; sürüklenirse sıralanır. Kısmen görünen sekme görünür olur.
                if let Target::Tab(index) = target {
                    if index != self.active {
                        shell.publish((self.on_select)(index));
                    } else if state.scroll.reveal(index) {
                        shell.invalidate_layout();
                    }

                    state.origin = cursor.position().map(|point| (index, point));
                }

                state.pressed = Some(target);
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                if let Some(index) = target.and_then(Target::tab)
                    && let Some(on_close) = &self.on_close
                    && self.tabs[index].closable
                {
                    shell.publish(on_close(index));
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let pressed = state.pressed.take();
                let dragging = state.dragging.take();
                let drop = state.drop.take();
                state.origin = None;

                if let Some(from) = dragging {
                    if let (Some(to), Some(on_reorder)) = (drop, &self.on_reorder)
                        && to != from
                    {
                        shell.publish(on_reorder(from, to));
                    }
                } else if pressed == target {
                    match target {
                        Some(Target::Close(index)) => {
                            if let Some(on_close) = &self.on_close {
                                shell.publish(on_close(index));
                            }
                        }
                        Some(Target::New) => {
                            if let Some(message) = &self.on_new {
                                shell.publish(message.clone());
                            }
                        }
                        Some(Target::Tab(_) | Target::Prev | Target::Next) | None => {}
                    }
                }

                if pressed.is_some() {
                    state.hovered = target;
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
        let state = tree.state.downcast_ref::<State>();

        if state.dragging.is_some() {
            return mouse::Interaction::Grabbing;
        }

        if state.overflow
            && let Some(list) = layout.children().nth(1)
            && cursor.is_over(list.bounds())
        {
            return self.list.as_widget().mouse_interaction(
                &tree.children[1],
                list,
                cursor,
                viewport,
                renderer,
            );
        }

        let target = cursor
            .position_over(layout.bounds())
            .and_then(|point| Geometry::of(self, layout, state).target(point));

        match target {
            Some(Target::Close(_) | Target::New) => mouse::Interaction::Pointer,
            Some(Target::Prev) if state.scroll.can_back() => mouse::Interaction::Pointer,
            Some(Target::Next) if state.scroll.can_forth() => mouse::Interaction::Pointer,
            Some(Target::Tab(_) | Target::Prev | Target::Next) | None => mouse::Interaction::None,
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
        let state = tree.state.downcast_ref::<State>();
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        strip(renderer, &t, bounds, self.bottom);

        let mut children = layout.children();
        let plus = children.next();
        let list = children.next();
        let prev = children.next();
        let next = children.next();
        let strip_rect = Rectangle {
            width: state.room,
            ..bounds
        };

        // Sekmeler şeridin içinde: kesilen kenarlar soluk.
        renderer.with_layer(strip_rect, |renderer| {
            for (index, ((entry, tree), tab)) in self
                .tabs
                .iter()
                .zip(&tree.children[EXTRAS..])
                .zip(children)
                .enumerate()
            {
                let bounds = tab.bounds();

                if bounds.width <= 0.0 {
                    continue;
                }

                let active = index == self.active;
                let hovered = state.hovered.and_then(Target::tab) == Some(index);
                let under = self::tab(
                    renderer,
                    &t,
                    bounds,
                    Look {
                        active,
                        hovered,
                        bottom: self.bottom,
                        accent: true,
                    },
                );

                let mut parts = tab.children();
                let (Some(label), Some(glyph)) = (parts.next(), parts.next()) else {
                    continue;
                };
                let text_color = if active || hovered { t.text } else { t.muted };
                let draw_label = |renderer: &mut Renderer| {
                    entry.parts[0].as_widget().draw(
                        &tree.children[0],
                        renderer,
                        theme,
                        &renderer::Style { text_color },
                        label,
                        mouse::Cursor::Unavailable,
                        viewport,
                    );
                };

                // Sığmayan başlık kısılır, sonu sekmenin zeminine doğru solar.
                if state.clipped.get(index).copied().unwrap_or(false) {
                    let room = entry.room(bounds);

                    renderer.with_layer(room, draw_label);
                    renderer.with_layer(room, |renderer| fade(renderer, room, under));
                } else {
                    draw_label(renderer);
                }

                // Üzerine gelinen sekmede kapatma düğmesi; yoksa kaydedilmemiş
                // sekmenin noktası; o da yoksa etkin sekmenin kapatma düğmesi.
                let close_hovered = state.hovered == Some(Target::Close(index));

                if entry.closable && (hovered || (active && !entry.dirty)) {
                    if close_hovered {
                        renderer.fill_quad(
                            Quad {
                                bounds: close_area(bounds),
                                border: border::rounded(crate::theme::shape::radius(3.0)),
                                ..Quad::default()
                            },
                            Background::Color(t.layer(0.12)),
                        );
                    }

                    entry.parts[1].as_widget().draw(
                        &tree.children[1],
                        renderer,
                        theme,
                        &renderer::Style {
                            text_color: if close_hovered { t.text } else { t.muted },
                        },
                        glyph,
                        mouse::Cursor::Unavailable,
                        viewport,
                    );
                } else if entry.dirty {
                    renderer.fill_quad(
                        Quad {
                            bounds: square(close_area(bounds).center(), 7.0),
                            border: border::rounded(crate::theme::shape::radius(3.5)),
                            ..Quad::default()
                        },
                        Background::Color(if active { t.text } else { t.muted }),
                    );
                }
            }

            fade_edges(renderer, strip_rect, &state.scroll, t.window);
        });

        if let Some(plus) = plus
            && self.on_new.is_some()
        {
            let hot = state.hovered == Some(Target::New);

            if hot {
                renderer.fill_quad(
                    Quad {
                        bounds: square(plus.bounds().center(), bounds.height - 8.0),
                        border: border::rounded(crate::theme::shape::radius(4.0)),
                        ..Quad::default()
                    },
                    Background::Color(t.layer(0.08)),
                );
            }

            self.plus.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                &renderer::Style {
                    text_color: if hot { t.text } else { t.muted },
                },
                plus,
                mouse::Cursor::Unavailable,
                viewport,
            );
        }

        if state.overflow
            && let Some(list) = list
        {
            self.list.as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                &renderer::Style {
                    text_color: t.muted,
                },
                list,
                cursor,
                viewport,
            );
            for (glyph, element, child, target, can) in [
                (
                    prev,
                    &self.prev,
                    &tree.children[2],
                    Target::Prev,
                    state.scroll.can_back(),
                ),
                (
                    next,
                    &self.next,
                    &tree.children[3],
                    Target::Next,
                    state.scroll.can_forth(),
                ),
            ] {
                let Some(glyph) = glyph else {
                    continue;
                };
                draw_arrow(
                    renderer,
                    &t,
                    theme,
                    element,
                    child,
                    glyph,
                    can,
                    state.hovered == Some(target),
                    viewport,
                );
            }
        }

        // Sürüklenen sekmenin bırakılacağı yer.
        if let (Some(from), Some(to)) = (state.dragging, state.drop)
            && let Some(x) = Geometry::of(self, layout, state).marker(from, to)
        {
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(
                        Point::new(x - 1.0, bounds.y + 4.0),
                        Size::new(2.0, bounds.height - 8.0),
                    ),
                    border: border::rounded(1.0),
                    ..Quad::default()
                },
                Background::Color(t.accent),
            );
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if tree.state.downcast_ref::<State>().overflow
            && let Some(list) = layout.children().nth(1)
        {
            self.list
                .as_widget_mut()
                .operate(&mut tree.children[1], list, renderer, operation);
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
        if !tree.state.downcast_ref::<State>().overflow {
            return None;
        }

        let list = layout.children().nth(1)?;

        self.list.as_widget_mut().overlay(
            &mut tree.children[1],
            list,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: Clone + 'a> From<Tabs<'a, Message>> for Element<'a, Message> {
    fn from(tabs: Tabs<'a, Message>) -> Self {
        Element::new(tabs)
    }
}

/// Şeridin bir oku: kayacak yer yoksa sönük; üzerine gelince zemini aydınlanır.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_arrow<'a, Message: 'a>(
    renderer: &mut Renderer,
    t: &Tokens,
    theme: &Theme,
    element: &Element<'a, Message>,
    tree: &Tree,
    glyph: Layout<'_>,
    can: bool,
    hot: bool,
    viewport: &Rectangle,
) {
    if can && hot {
        renderer.fill_quad(
            Quad {
                bounds: square(glyph.bounds().center(), arrow_width() - 2.0),
                border: border::rounded(crate::theme::shape::radius(3.0)),
                ..Quad::default()
            },
            Background::Color(t.layer(0.08)),
        );
    }
    element.as_widget().draw(
        tree,
        renderer,
        theme,
        &renderer::Style {
            text_color: if !can {
                t.disabled()
            } else if hot {
                t.text
            } else {
                t.muted
            },
        },
        glyph,
        mouse::Cursor::Unavailable,
        viewport,
    );
}

/// Şeridin kesilen kenarları: o yönde daha sekme var, zemine doğru solar.
pub(crate) fn fade_edges(renderer: &mut Renderer, strip: Rectangle, scroll: &Scroll, under: Color) {
    let width = EDGE.min(strip.width / 2.0);
    let shade = |renderer: &mut Renderer, x: f32, angle: f32| {
        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x,
                    y: strip.y,
                    width,
                    height: strip.height - 2.0,
                },
                ..Quad::default()
            },
            Background::Gradient(Gradient::Linear(
                Linear::new(Degrees(angle))
                    .add_stop(0.0, Color { a: 0.0, ..under })
                    .add_stop(1.0, under),
            )),
        );
    };
    if scroll.can_back() {
        shade(renderer, strip.x, 270.0);
    }
    if scroll.can_forth() {
        shade(renderer, strip.x + strip.width - width, 90.0);
    }
}

/// Sekmenin görünüşü.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Look {
    pub active: bool,
    pub hovered: bool,
    /// Şerit içeriğin altında.
    pub bottom: bool,
    /// Etkin sekmenin çizgisi tam vurgu renginde (ör. odaktaki yığın);
    /// değilse daha soluk.
    pub accent: bool,
}

/// Şeridin zemini ve içeriğe bakan kenarındaki çizgi.
pub(crate) fn strip(renderer: &mut Renderer, t: &Tokens, bounds: Rectangle, bottom: bool) {
    fill(renderer, bounds, t.window);

    let y = if bottom {
        bounds.y
    } else {
        bounds.y + bounds.height - 1.0
    };

    fill(
        renderer,
        Rectangle::new(Point::new(bounds.x, y), Size::new(bounds.width, 1.0)),
        t.border,
    );
}

/// Sekmenin zemini ve çizgisi (web'in sekmeleri, DESIGN.md §7.5; şeridin
/// sekmeleri gibi): kutu yoktur. Etkin sekmenin içeriğe bakan kenarında 2
/// piksellik vurgu çizgisi durur, üzerine gelinen sekme hafif bir katman
/// alır. Başlığın altındaki (solan ucun karıştığı) rengi döndürür.
pub(crate) fn tab(renderer: &mut Renderer, t: &Tokens, bounds: Rectangle, look: Look) -> Color {
    let mut under = t.window;

    if look.hovered && !look.active {
        let layer = t.layer(0.05);
        fill(renderer, bounds, layer);
        under = mix(t.window, Color { a: 1.0, ..layer }, layer.a);
    }

    if look.active {
        let y = if look.bottom {
            bounds.y
        } else {
            bounds.y + bounds.height - 2.0
        };

        fill(
            renderer,
            Rectangle::new(Point::new(bounds.x, y), Size::new(bounds.width, 2.0)),
            if look.accent {
                t.accent
            } else {
                t.accent.scale_alpha(0.55)
            },
        );
    }

    under
}

/// Kısılan başlığın sonu: zemine doğru solan şerit. Sekmenin kenar ve vurgu
/// çizgilerine değmez.
pub(crate) fn fade(renderer: &mut Renderer, room: Rectangle, under: Color) {
    let width = FADE.min(room.width);

    renderer.fill_quad(
        Quad {
            bounds: Rectangle {
                x: room.x + room.width - width,
                y: room.y + 3.0,
                width,
                height: room.height - 6.0,
            },
            ..Quad::default()
        },
        Background::Gradient(Gradient::Linear(
            Linear::new(Degrees(90.0))
                .add_stop(0.0, Color { a: 0.0, ..under })
                .add_stop(1.0, under),
        )),
    );
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

    fn scrolled(widths: &[f32], active: usize, room: f32) -> Scroll {
        let mut scroll = Scroll::default();
        scroll.settle(widths, active, room);
        scroll
    }

    #[test]
    fn tabs_keep_their_widths_and_the_strip_scrolls_to_the_active_one() {
        let widths = [100.0; 10];
        // All fit: nothing moves.
        assert_eq!(scrolled(&widths, 3, 1000.0).offset, 0.0);
        // The active one at the start stays put, the one past the edge comes in at the edge.
        assert_eq!(scrolled(&widths, 2, 450.0).offset, 0.0);
        assert_eq!(scrolled(&widths, 7, 450.0).offset, 350.0);
        // Back to one on the left: it comes in at the left edge.
        let mut scroll = scrolled(&widths, 7, 450.0);
        assert_eq!(scroll.settle(&widths, 1, 450.0), 100.0);
        // A tab wider than the strip shows its start.
        assert_eq!(scrolled(&[100.0, 600.0, 100.0], 1, 300.0).offset, 100.0);
        // Nothing to show.
        assert_eq!(scrolled(&[], 0, 100.0).offset, 0.0);
    }

    #[test]
    fn the_readers_scrolling_is_kept_until_the_active_tab_changes() {
        let widths = [100.0; 10];
        let mut scroll = scrolled(&widths, 0, 450.0);
        assert!(scroll.step(true));
        assert_eq!(scroll.offset, 100.0, "a whole tab");
        assert!(scroll.step(true));
        assert_eq!(scroll.offset, 200.0);
        // The next frame keeps it, though the active tab is out of sight now.
        assert_eq!(scroll.settle(&widths, 0, 450.0), 200.0);
        // The active tab changes: it is shown.
        assert_eq!(scroll.settle(&widths, 9, 450.0), 550.0);
        assert!(!scroll.can_forth());
        assert!(scroll.can_back());
        assert!(!scroll.step(true), "at the end");
        assert!(scroll.step(false));
        assert_eq!(
            scroll.offset, 500.0,
            "the cut one comes in at the left edge"
        );
        // A narrower strip shows the active tab again; a wider one takes the room back.
        assert_eq!(scroll.settle(&widths, 9, 300.0), 700.0);
        assert_eq!(scroll.settle(&widths, 9, 2000.0), 0.0);
        assert!(!scroll.can_back() && !scroll.can_forth());
    }

    #[test]
    fn the_wheel_scrolls_the_strip_within_its_ends() {
        let widths = [100.0; 4];
        let mut scroll = scrolled(&widths, 0, 250.0);
        assert!(
            !scroll.wheel(mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 }),
            "at the start"
        );
        assert!(scroll.wheel(mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 }));
        assert_eq!(scroll.offset, WHEEL);
        assert!(scroll.wheel(mouse::ScrollDelta::Pixels { x: -1000.0, y: 0.0 }));
        assert_eq!(scroll.offset, 150.0, "the end");
    }

    fn strip_of(count: usize, first: usize) -> Geometry {
        Geometry {
            strip: Rectangle::new(Point::ORIGIN, Size::new(10_000.0, 28.0)),
            arrows: None,
            tabs: (0..count)
                .map(|slot| {
                    (
                        first + slot,
                        Rectangle::new(
                            Point::new(slot as f32 * 100.0, 0.0),
                            Size::new(100.0, 28.0),
                        ),
                    )
                })
                .collect(),
            closes: Vec::new(),
            new: None,
        }
    }

    #[test]
    fn dragged_tabs_drop_past_the_middle_of_their_neighbours() {
        let geometry = strip_of(5, 0);

        // İkinci sekme (1) sürükleniyor.
        assert_eq!(geometry.drop_index(1, 10.0), 0);
        assert_eq!(geometry.drop_index(1, 140.0), 1);
        assert_eq!(geometry.drop_index(1, 240.0), 1);
        assert_eq!(geometry.drop_index(1, 260.0), 2);
        assert_eq!(geometry.drop_index(1, 999.0), 4);

        // İşaret, sekmenin gireceği aralıkta.
        assert_eq!(geometry.marker(1, 0), Some(0.0));
        assert_eq!(geometry.marker(1, 1), None);
        assert_eq!(geometry.marker(1, 2), Some(300.0));
        assert_eq!(geometry.marker(1, 4), Some(500.0));

        // Şerit kaymışken sıralar gerçek sıradır.
        let shifted = strip_of(3, 4);
        assert_eq!(shifted.drop_index(5, 260.0), 6);
        assert_eq!(shifted.marker(5, 6), Some(300.0));
        assert_eq!(shifted.drop_index(5, 10.0), 4);
        assert_eq!(shifted.marker(5, 4), Some(0.0));
    }
}

/// Gerçek olaylarla: tıklama, kapatma, sürükleyerek sıralama, yeni sekme.
#[cfg(all(test, feature = "snapshot"))]
mod interaction {
    use iced::{Element, Point, Size};

    use super::{MIN_WIDTH, Tab, Tabs, height};
    use crate::snapshot::{Input, Snapshot};
    use crate::theme::typography;

    #[derive(Debug, Clone)]
    enum Message {
        Selected(usize),
        Closed(usize),
        Moved(usize, usize),
        Added,
    }

    struct Strip {
        tabs: Vec<&'static str>,
        active: usize,
    }

    impl Strip {
        fn view(&self) -> Element<'_, Message> {
            Tabs::new(
                self.tabs.iter().map(|title| Tab::new(*title)),
                self.active,
                Message::Selected,
            )
            .on_close(Message::Closed)
            .on_reorder(Message::Moved)
            .on_new(Message::Added)
            .into()
        }

        fn update(&mut self, message: Message) {
            match message {
                Message::Selected(index) => self.active = index,
                Message::Closed(index) => {
                    self.tabs.remove(index);

                    if index <= self.active {
                        self.active = self.active.saturating_sub(1);
                    }
                }
                Message::Moved(from, to) => {
                    let tab = self.tabs.remove(from);
                    self.tabs.insert(to, tab);

                    if self.active == from {
                        self.active = to;
                    }
                }
                Message::Added => {
                    self.tabs.push("E");
                    self.active = self.tabs.len() - 1;
                }
            }
        }
    }

    #[test]
    fn tabs_are_selected_closed_moved_and_added() {
        // GPU yoksa yazılım çizici kullanılır.
        let mut snapshot = Snapshot::new(Size::new(600.0, 100.0)).expect("çizici kurulamadı");

        let mut strip = Strip {
            tabs: vec!["A", "B", "C", "D"],
            active: 0,
        };
        let mut update = |strip: &mut Strip, message| strip.update(message);

        // Kısa başlıklı sekmeler en dar genişliktedir.
        let width = typography::scaled(MIN_WIDTH);
        let y = height() / 2.0;
        let close = |tab: f32| Point::new((tab + 1.0) * width - 14.0, y);

        snapshot.settle(&mut strip, Strip::view, &mut update);

        snapshot.input(
            &mut strip,
            Strip::view,
            &mut update,
            Input::Click(Point::new(width + 16.0, y)),
        );
        assert_eq!(strip.active, 1);

        // Etkin sekmenin kapatma düğmesi; solundaki sekme açılır.
        snapshot.input(
            &mut strip,
            Strip::view,
            &mut update,
            Input::Click(close(1.0)),
        );
        assert_eq!(strip.tabs, ["A", "C", "D"]);
        assert_eq!(strip.active, 0);

        // İlk sekme son sekmenin ortasını geçecek kadar sürüklenir.
        snapshot.input(
            &mut strip,
            Strip::view,
            &mut update,
            Input::Drag(Point::new(16.0, y), Point::new(3.0 * width - 4.0, y)),
        );
        assert_eq!(strip.tabs, ["C", "D", "A"]);
        assert_eq!(strip.active, 2);

        // Kısa sürükleme sıralamaz.
        snapshot.input(
            &mut strip,
            Strip::view,
            &mut update,
            Input::Drag(Point::new(16.0, y), Point::new(19.0, y)),
        );
        assert_eq!(strip.tabs, ["C", "D", "A"]);
        assert_eq!(strip.active, 0);

        snapshot.input(
            &mut strip,
            Strip::view,
            &mut update,
            Input::Click(Point::new(3.0 * width + height() / 2.0, y)),
        );
        assert_eq!(strip.tabs, ["C", "D", "A", "E"]);
        assert_eq!(strip.active, 3);
    }

    #[test]
    fn a_strip_that_does_not_fit_scrolls_by_its_arrows_and_wheel_and_keeps_its_tabs_whole() {
        let mut snapshot = Snapshot::new(Size::new(300.0, 60.0)).expect("çizici kurulamadı");
        let mut strip = Strip {
            tabs: vec!["A", "B", "C", "D", "E", "F", "G", "H"],
            active: 0,
        };
        let mut update = |strip: &mut Strip, message| strip.update(message);
        let width = typography::scaled(MIN_WIDTH);
        let y = height() / 2.0;
        let arrow = super::arrow_width();
        // The + stands beside the arrows, the list at the far end.
        let next = Point::new(300.0 - 1.5 * arrow, y);
        let prev = Point::new(300.0 - 2.5 * arrow, y);
        let room = 300.0 - 3.0 * arrow - height();

        snapshot.settle(&mut strip, Strip::view, &mut update);
        // Each press brings in one more tab.
        snapshot.input(&mut strip, Strip::view, &mut update, Input::Click(next));
        snapshot.input(&mut strip, Strip::view, &mut update, Input::Click(next));
        assert_eq!(strip.active, 0, "scrolling selects nothing");
        // A press at the strip's left end is now on the third tab.
        let first_seen = 2;
        let offset = first_seen as f32 * width;
        snapshot.input(
            &mut strip,
            Strip::view,
            &mut update,
            Input::Click(Point::new(width * first_seen as f32 - offset + 14.0, y)),
        );
        assert_eq!(strip.active, first_seen);
        // Back.
        for _ in 0..4 {
            snapshot.input(&mut strip, Strip::view, &mut update, Input::Click(prev));
        }
        snapshot.input(
            &mut strip,
            Strip::view,
            &mut update,
            Input::Click(Point::new(20.0, y)),
        );
        assert_eq!(strip.active, 0);
        // The wheel over the strip goes as far as the last tab.
        for _ in 0..40 {
            snapshot.input(
                &mut strip,
                Strip::view,
                &mut update,
                Input::Scroll(Point::new(60.0, y), -1.0),
            );
        }
        snapshot.input(
            &mut strip,
            Strip::view,
            &mut update,
            Input::Click(Point::new(room - width + 14.0, y)),
        );
        assert_eq!(strip.active, 7, "the last tab ends at the strip's end");
    }
}
