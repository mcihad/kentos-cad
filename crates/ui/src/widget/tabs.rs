//! Belge sekmeleri: açık çizimler ya da AutoCAD'in Model / Düzen sekmeleri
//! gibi aynı alanı paylaşan görünümler.
//!
//! - Etkin sekme bağlandığı içerikle aynı zemindedir ve vurgu çizgisi
//!   taşır; şeridin kenar çizgisi etkin sekmenin altında kesilir.
//! - Kapatma düğmesi etkin sekmede ve üzerine gelinen sekmede görünür.
//!   Kaydedilmemiş sekmede yerinde bir nokta durur. Orta tık da kapatır.
//! - Sekmeler sürüklenerek sıralanır ([`Tabs::on_reorder`]).
//! - Sekmeler şeritten taşmaz: sığmayanlar sağdaki listeden seçilir, etkin
//!   sekme her zaman görünür. Uzun başlıklar kısılır, sonu solar.
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

use std::ops::Range;

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
use crate::theme::accent::mix;
use crate::theme::{Tokens, typography};
use crate::widget::context_menu::{Menu, MenuButton};

/// Şeridin yüksekliği ve sekme genişliğinin sınırları, 12 piksellik gövde
/// metnine göre.
const HEIGHT: f32 = 28.0;
pub(crate) const MIN_WIDTH: f32 = 64.0;
pub(crate) const MAX_WIDTH: f32 = 200.0;
/// Sığmayan sekmelerin daralabildiği en dar genişlik; altında listeye
/// taşarlar.
pub(crate) const SHRINK: f32 = 110.0;

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

/// Şeridin yüksekliği, geçerli yazı boyutunda.
pub fn height() -> f32 {
    typography::scaled(HEIGHT)
}

/// Bir sekme.
pub struct Tab<'a> {
    title: Fragment<'a>,
    icon: Option<Icon>,
    dirty: bool,
    closable: bool,
}

impl<'a> Tab<'a> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            icon: None,
            dirty: false,
            closable: true,
        }
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
    content: Option<Paint<'a>>,
    /// Yeni sekme ikonu.
    plus: Element<'a, Message>,
    /// Sığmayan sekmelerin listesi.
    list: Element<'a, Message>,
}

/// Temadan okunan renk.
type Paint<'a> = Box<dyn Fn(&Theme) -> Color + 'a>;

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
                .center_x(height())
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

                Entry {
                    closable: tab.closable,
                    dirty: tab.dirty,
                    parts: [
                        heading
                            .push(label::text(tab.title).wrapping(Wrapping::None))
                            .into(),
                        icon(Icon::Close).size(GLYPH).into(),
                    ],
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
            content: None,
            plus: icon(Icon::Plus).size(12.0).into(),
            list: list.into(),
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

    /// Etkin sekmenin zemini: sekmenin bağlandığı içeriğin rengi
    /// (varsayılan yüzey rengi).
    pub fn content(mut self, color: impl Fn(&Theme) -> Color + 'a) -> Self {
        self.content = Some(Box::new(color));
        self
    }
}

/// Sekmelerin genişlikleri ve taşma: hepsi sığıyorsa doğal genişlikleri.
/// Sığmıyorsa etkin sekme doğal genişliğinde kalır, diğerleri aynı üst
/// sınıra kadar daralır; her sekme kendi tabanında (`floors`) durur. Öbür
/// sekmeler tabanlarındayken de sığmıyorsa etkin sekme en fazla
/// `active_floor`'a kadar daralır; o da yetmezse sığmayanlar taşar.
pub(crate) fn fit(
    natural: &[f32],
    active: usize,
    room: f32,
    floors: &[f32],
    active_floor: f32,
) -> (Vec<f32>, bool) {
    if natural.iter().sum::<f32>() <= room {
        return (natural.to_vec(), false);
    }

    let kept = natural.get(active).copied().unwrap_or(0.0);
    let lows: Vec<f32> = natural
        .iter()
        .zip(floors.iter().chain(std::iter::repeat(&0.0)))
        .map(|(tab, floor)| tab.min(*floor))
        .collect();
    let others: f32 = (0..natural.len())
        .filter(|index| *index != active)
        .map(|index| lows[index])
        .sum();
    let at_floor = |kept: f32| {
        (0..natural.len())
            .map(|index| if index == active { kept } else { lows[index] })
            .collect::<Vec<f32>>()
    };

    if kept + others > room {
        let smallest = active_floor.min(kept);

        return if smallest + others <= room {
            (at_floor((room - others).floor()), false)
        } else {
            (at_floor(kept), natural.len() > 1)
        };
    }

    // Su doldurma: tabanına inen sekme tabanda kalır, kalan yer öbür
    // sekmelere eşit bölünür.
    let mut pinned = vec![false; natural.len()];

    loop {
        let free: Vec<usize> = (0..natural.len())
            .filter(|index| *index != active && !pinned[*index])
            .collect();
        let budget = room
            - kept
            - (0..natural.len())
                .filter(|index| pinned[*index])
                .map(|index| lows[index])
                .sum::<f32>();
        let cap = level(
            &free.iter().map(|index| natural[*index]).collect::<Vec<_>>(),
            budget,
        );
        let sunk: Vec<usize> = free
            .iter()
            .copied()
            .filter(|index| natural[*index].min(cap) < lows[*index])
            .collect();

        if sunk.is_empty() {
            let widths = (0..natural.len())
                .map(|index| {
                    if index == active {
                        natural[index]
                    } else if pinned[index] {
                        lows[index]
                    } else {
                        natural[index].min(cap)
                    }
                })
                .collect();

            return (widths, false);
        }

        for index in sunk {
            pinned[index] = true;
        }
    }
}

/// Genişliklerin toplamı `budget`'ı geçmesin diye hepsine uygulanan en
/// geniş üst sınır; en darlardan başlayarak kalan yer eşit bölünür.
fn level(widths: &[f32], budget: f32) -> f32 {
    let mut sorted = widths.to_vec();
    sorted.sort_by(f32::total_cmp);

    let mut used = 0.0;

    for (index, width) in sorted.iter().enumerate() {
        let share = (budget - used) / (sorted.len() - index) as f32;

        if share <= *width {
            return share.floor();
        }

        used += width;
    }

    f32::INFINITY
}

/// Görünen sekmeler. `first` önceki karedeki ilk görünen sekmedir: sekmeler
/// yerinde durur, yalnızca etkin sekme dışarıda kalınca şerit kayar. Sonda
/// boşluk kalırsa soldaki sekmeler de gösterilir.
pub(crate) fn window(widths: &[f32], active: usize, first: usize, room: f32) -> Range<usize> {
    let Some(last) = widths.len().checked_sub(1) else {
        return 0..0;
    };
    let active = active.min(last);
    let fits = |range: Range<usize>| widths[range].iter().sum::<f32>() <= room;

    let mut start = first.min(active);

    while start < active && !fits(start..active + 1) {
        start += 1;
    }

    let mut end = active + 1;

    while end <= last && fits(start..end + 1) {
        end += 1;
    }

    while start > 0 && fits(start - 1..end) {
        start -= 1;
    }

    start..end
}

/// Sekmenin bölgesi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Tab(usize),
    Close(usize),
    New,
}

impl Target {
    fn tab(self) -> Option<usize> {
        match self {
            Target::Tab(index) | Target::Close(index) => Some(index),
            Target::New => None,
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
    /// İlk görünen sekme.
    first: usize,
    /// Sığmayan sekme var; liste düğmesi gösterilir.
    overflow: bool,
    /// Başlığı sığmayan sekmeler; başlığın sonu solar.
    clipped: Vec<bool>,
}

/// Yerleşimden okunan bölgeler.
struct Geometry {
    /// Görünen sekmeler: sırası ve sınırı.
    tabs: Vec<(usize, Rectangle)>,
    closes: Vec<(usize, Rectangle)>,
    new: Option<Rectangle>,
}

impl Geometry {
    fn of<Message>(tabs: &Tabs<'_, Message>, layout: Layout<'_>) -> Self {
        let height = layout.bounds().height;
        let mut children = layout.children();
        let plus = children.next();
        let _list = children.next();

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
            tabs: visible,
            closes,
            new: plus
                .filter(|_| tabs.on_new.is_some())
                .map(|plus| square(plus.bounds().center(), height)),
        }
    }

    fn target(&self, point: Point) -> Option<Target> {
        if let Some((index, _)) = self.closes.iter().find(|(_, area)| area.contains(point)) {
            return Some(Target::Close(*index));
        }

        if let Some((index, _)) = self.tabs.iter().find(|(_, tab)| tab.contains(point)) {
            return Some(Target::Tab(*index));
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

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Tabs<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        [Tree::new(&self.plus), Tree::new(&self.list)]
            .into_iter()
            .chain(self.tabs.iter().map(|entry| Tree {
                tag: tree::Tag::stateless(),
                state: tree::State::None,
                children: entry.parts.iter().map(Tree::new).collect(),
            }))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.children.resize_with(2 + self.tabs.len(), Tree::empty);
        tree.children[0].diff(&self.plus);
        tree.children[1].diff(&self.list);

        for (tab, entry) in tree.children[2..].iter_mut().zip(&self.tabs) {
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
        let (extras, trees) = tree.children.split_at_mut(2);

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

        // Sığmayan sekmeler önce eşit ölçüde daralır (etkin sekme okunur
        // kalır); okunur genişliğin altına inmeleri gerekirse sığmayanlar
        // listeye taşar.
        let new = if self.on_new.is_some() { height } else { 0.0 };
        let (widths, overflow) = fit(
            &natural,
            self.active,
            width - new,
            &vec![typography::scaled(SHRINK); natural.len()],
            f32::INFINITY,
        );
        let room = width - new - if overflow { height } else { 0.0 };
        let widths: Vec<f32> = widths.into_iter().map(|tab| tab.min(room)).collect();

        let state = tree.state.downcast_mut::<State>();
        let visible = window(&widths, self.active, state.first, room);
        state.first = visible.start;
        state.overflow = overflow;
        state.clipped.clear();

        let mut x = 0.0;
        let mut tabs = Vec::with_capacity(self.tabs.len());

        for (index, ((entry, tree), (label, width))) in self
            .tabs
            .iter_mut()
            .zip(trees.iter_mut())
            .zip(labels.into_iter().zip(widths))
            .enumerate()
        {
            // Genişlik başlıktan hesaplandığı için yuvarlama payı bırakılır.
            let tab = Rectangle::with_size(Size::new(width, height));
            state
                .clipped
                .push(label.size().width > entry.room(tab).width + 0.5);

            if !visible.contains(&index) {
                tabs.push(Node::with_children(
                    Size::ZERO,
                    vec![Node::default(), Node::default()],
                ));
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
        let plus = plus.move_to(square(Point::new(x + new / 2.0, height / 2.0), side).position());

        let list = if overflow {
            self.list
                .as_widget_mut()
                .layout(
                    &mut extras[1],
                    renderer,
                    &layout::Limits::new(Size::ZERO, Size::new(height, height)),
                )
                .move_to(Point::new(width - height, 0.0))
        } else {
            Node::default()
        };

        Node::with_children(
            Size::new(width, height),
            [plus, list].into_iter().chain(tabs).collect(),
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

        let geometry = Geometry::of(self, layout);
        let state = tree.state.downcast_mut::<State>();
        let target = cursor
            .position_over(layout.bounds())
            .and_then(|point| geometry.target(point));

        match event {
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

                // Sekme basılınca seçilir; sürüklenirse sıralanır.
                if let Target::Tab(index) = target {
                    if index != self.active {
                        shell.publish((self.on_select)(index));
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
                        Some(Target::Tab(_)) | None => {}
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
            .and_then(|point| Geometry::of(self, layout).target(point));

        match target {
            Some(Target::Close(_) | Target::New) => mouse::Interaction::Pointer,
            Some(Target::Tab(_)) | None => mouse::Interaction::None,
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
        let content = self
            .content
            .as_ref()
            .map_or(t.surface, |color| color(theme));

        strip(renderer, &t, bounds, self.bottom);

        let mut children = layout.children();
        let plus = children.next();
        let list = children.next();
        let mut previous = None;

        for (index, ((entry, tree), tab)) in self
            .tabs
            .iter()
            .zip(&tree.children[2..])
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
                    separator: previous == Some(false) && !active,
                    content,
                    accent: true,
                },
            );
            previous = Some(active);

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
                            border: border::rounded(3.0),
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
                        border: border::rounded(3.5),
                        ..Quad::default()
                    },
                    Background::Color(if active { t.text } else { t.muted }),
                );
            }
        }

        if let Some(plus) = plus
            && self.on_new.is_some()
        {
            let hot = state.hovered == Some(Target::New);

            if hot {
                renderer.fill_quad(
                    Quad {
                        bounds: square(plus.bounds().center(), bounds.height - 8.0),
                        border: border::rounded(4.0),
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
        }

        // Sürüklenen sekmenin bırakılacağı yer.
        if let (Some(from), Some(to)) = (state.dragging, state.drop)
            && let Some(x) = Geometry::of(self, layout).marker(from, to)
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

/// Sekmenin görünüşü.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Look {
    pub active: bool,
    pub hovered: bool,
    /// Şerit içeriğin altında.
    pub bottom: bool,
    /// Solundaki sekme de etkin değil: aralarına ince ayraç çizilir.
    pub separator: bool,
    /// Etkin sekmenin bağlandığı içeriğin rengi.
    pub content: Color,
    /// Etkin sekmenin kenarında vurgu çizgisi (ör. odaktaki yığın).
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

/// Sekmenin zemini; başlığın altındaki (solan ucun karıştığı) rengi
/// döndürür.
pub(crate) fn tab(renderer: &mut Renderer, t: &Tokens, bounds: Rectangle, look: Look) -> Color {
    if look.active {
        // Etkin sekme şeridin kenar çizgisini örter, içeriğe bağlanır.
        fill(renderer, bounds, look.content);

        for x in [bounds.x, bounds.x + bounds.width - 1.0] {
            fill(
                renderer,
                Rectangle::new(Point::new(x, bounds.y), Size::new(1.0, bounds.height)),
                t.border,
            );
        }

        if look.accent {
            let y = if look.bottom {
                bounds.y + bounds.height - 2.0
            } else {
                bounds.y
            };

            fill(
                renderer,
                Rectangle::new(Point::new(bounds.x, y), Size::new(bounds.width, 2.0)),
                t.accent,
            );
        }

        return look.content;
    }

    if look.separator {
        let inset = (bounds.height * 0.28).round();

        fill(
            renderer,
            Rectangle::new(
                Point::new(bounds.x, bounds.y + inset),
                Size::new(1.0, bounds.height - inset * 2.0),
            ),
            t.border,
        );
    }

    if look.hovered {
        let layer = t.layer(0.05);
        fill(renderer, bounds, layer);

        return mix(t.window, Color { a: 1.0, ..layer }, layer.a);
    }

    t.window
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

    #[test]
    fn the_active_tab_stays_visible() {
        let widths = [100.0; 10];

        // Hepsi sığar.
        assert_eq!(window(&widths, 3, 0, 1000.0), 0..10);

        // Etkin sekme görünen aralıktaysa şerit kaymaz.
        assert_eq!(window(&widths, 2, 0, 450.0), 0..4);
        assert_eq!(window(&widths, 3, 0, 450.0), 0..4);

        // Dışarıdaysa en sağa gelecek kadar kayar; geri dönerken en sola
        // gelecek kadar.
        assert_eq!(window(&widths, 7, 0, 450.0), 4..8);
        assert_eq!(window(&widths, 5, 4, 450.0), 4..8);
        assert_eq!(window(&widths, 1, 4, 450.0), 1..5);

        // Sonda boşluk kalırsa soldakiler de görünür (ör. sekme kapanınca).
        assert_eq!(window(&widths[..6], 5, 4, 450.0), 2..6);

        // Tek sekme bile sığmıyorsa yalnızca etkin sekme.
        assert_eq!(window(&widths, 6, 0, 50.0), 6..7);
        assert_eq!(window(&[], 0, 0, 100.0), 0..0);
    }

    #[test]
    fn crowded_tabs_shrink_before_they_overflow() {
        let natural = [80.0, 200.0, 150.0, 200.0];

        // Sığıyor.
        assert_eq!(
            fit(&natural, 0, 700.0, &[110.0; 4], f32::INFINITY),
            (natural.to_vec(), false)
        );

        // Dar sekme yerinde kalır, geniş sekmeler eşit daralır.
        assert_eq!(
            fit(&natural, 0, 560.0, &[110.0; 4], f32::INFINITY),
            (vec![80.0, 165.0, 150.0, 165.0], false)
        );

        // Etkin sekme doğal genişliğinde kalır.
        assert_eq!(
            fit(&natural, 1, 500.0, &[110.0; 4], f32::INFINITY),
            (vec![80.0, 200.0, 110.0, 110.0], false)
        );
        assert_eq!(
            fit(&natural, 0, 500.0, &[110.0; 4], f32::INFINITY),
            (vec![80.0, 140.0, 140.0, 140.0], false)
        );

        // Okunur genişliğin altına inilmez; sığmayanlar listeye taşar.
        assert_eq!(
            fit(&natural, 3, 400.0, &[110.0; 4], f32::INFINITY),
            (vec![80.0, 110.0, 110.0, 200.0], true)
        );
        assert_eq!(
            fit(&[300.0], 0, 100.0, &[110.0], f32::INFINITY),
            (vec![300.0], false)
        );

        // Her sekme kendi tabanında durur (ör. yalnızca ikon kalan sekme).
        assert_eq!(
            fit(
                &natural,
                1,
                400.0,
                &[40.0, 110.0, 40.0, 110.0],
                f32::INFINITY
            ),
            (vec![45.0, 200.0, 45.0, 110.0], false)
        );

        // Öbürleri tabandayken etkin sekme de daralabilir.
        assert_eq!(
            fit(&natural, 1, 300.0, &[40.0, 110.0, 40.0, 110.0], 100.0),
            (vec![40.0, 110.0, 40.0, 110.0], false)
        );
        assert_eq!(
            fit(&natural, 1, 280.0, &[40.0, 110.0, 40.0, 110.0], 100.0),
            (vec![40.0, 200.0, 40.0, 110.0], true)
        );
    }

    fn strip_of(count: usize, first: usize) -> Geometry {
        Geometry {
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
}
