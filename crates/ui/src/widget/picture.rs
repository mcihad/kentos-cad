//! Çizim görüntüleyici: yakınlaştırılan, kaydırılan ve resim olarak kaydedilen çizim alanı (docs/adr/0937 §1).
//!
//! ```text
//! ┌ çizim ───────────────────────────────────── ╭──────────────────────────╮ ┐
//! │                                             │ −  %250  + │ ⤢ │ ⤓      │ │
//! │                                             ╰──────────────────────────╯ ┃│
//! │   tekerlek: imlecin altındaki nokta yerinde kalarak yakınlaşır          ┃│
//! │   Shift+tekerlek: sağa sola kaydırır · sürükleme: her yöne kaydırır      │
//! │   çift tık: sığdırır                                                     │
//! │        ━━━━━━━━━━━━━━━━━━  (yakınken: nerede olduğunu gösteren çubuklar) │
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! **Yakınlaştırma yeniden üretmedir, büyütme değil.** İçerik (bir tuval) görünen alanın `z` katı büyüklükte bir alana
//! yerleşir ve kendini o boyda yeniden çizer; görünen alan onun üstünde kayar. Tuval eksenlerini, aralıklarını ve
//! yazılarını kendi boyuna göre kurduğu için yakınlaşınca çizgiler incecik, yazılar keskin kalır, araya yeni ölçü
//! çizgileri girer: CAD'in yakınlaştırması gibi. Bir resmi büyütmek pikselleri iriltir; bir dönüşümle büyütmek de
//! çizgileri ve yazıları kalınlaştırırdı. Her tuvale ayrı bir görünüm dönüşümü yazmak gerekmez: yol pencerelerinin
//! çizimleri değişmeden yakınlaşır. Kenar boşlukları sabit piksel olduğu için imlecin altındaki nokta birkaç piksel
//! kayabilir.
//!
//! - **Tekerlek** imlecin altındaki noktayı yerinde tutarak yakınlaştırır (satır başına 1,2 kat); düğmeler ortaya göre
//!   1,5 kat. Yakınlık 1 (sığdırılmış) ile 40 arasıdır.
//! - **Kaydırma:** sol ya da orta tuşla sürükleme her yöne; **Shift+tekerlek** ve dokunmatik yüzeyin yatay kaydırması
//!   sağa sola; yakınken altta ve sağda beliren **çubuklar** nerede olunduğunu gösterir, tutulup sürüklenir, yoluna
//!   basılınca oraya atlar. Olay önce çizime verilir: çizim sol basışı kendisi yakalıyorsa (km çizgisini sürüklemek
//!   gibi) kaydırma orta tuşla yapılır. Çift tık sığdırır.
//! - **Araç çubuğu** sağ üstte, birleşik bir düğme grubudur: uzaklaştır, yakınlık, yakınlaştır | sığdır | resim olarak
//!   kaydet. Düğmenin adı üzerine gelince altında yazar; yakınlığın ipucu fare ile ne yapılacağını söyler.
//! - **Resim olarak kaydet** görüneni (yakınlık ve kaydırmasıyla) iki kat piksel yoğunluğunda, aydınlık temada ve düz
//!   zeminde yeniden çizer (rapora konacak bir resim, ekranın kopyası değil); PNG baytları `on_save`'in mesajıyla
//!   uygulamaya gider, dosyayı uygulama sorar ve yazar. Araç çubuğu ve kaydırma çubukları resme girmez.
//!
//! Durum (yakınlık, kaydırma) bileşenin ağacındadır: pencere yeniden çizildikçe korunur, çizimin verisi değişse de
//! (sonraki kesite geçmek gibi) aynı ayrıntıya bakılmaya devam edilir. [`View`] bir `Operation`'ın `custom`'ıyla
//! okunur ve değiştirilir.

use std::cell::Cell;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use iced::advanced::layout::{Layout, Limits, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::text::{self as text_core, Renderer as _};
use iced::advanced::widget::{Id, Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::alignment::Vertical;
use iced::keyboard;
use iced::time::Instant;
use iced::widget::container;
use iced::{
    Background, Border, Element, Event, Length, Pixels, Point, Rectangle, Renderer, Shadow, Size, Theme, Vector,
    mouse,
};

use crate::icon::{Icon, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};

/// The most the picture zooms in.
pub const MAX_ZOOM: f32 = 40.0;
/// A button's step, a wheel line's and a touchpad pixel's zoom; a wheel line's pan (px).
const STEP: f32 = 1.5;
const WHEEL: f32 = 1.2;
const PIXEL: f32 = 0.004;
const LINE: f32 = 48.0;
/// The bar's button side, its zoom label's width, its padding, a separator's width, its margin from the picture's
/// corner and its tip's gap (at the 12 px body text).
const ITEM: f32 = 26.0;
const LABEL: f32 = 48.0;
const PAD: f32 = 3.0;
const SEPARATOR: f32 = 9.0;
const MARGIN: f32 = 8.0;
const TIP_GAP: f32 = 6.0;
/// A picture narrower than this has its bar without the zoom's label (the 260 px plan of Enkesit had it over 70 % of
/// its width).
const COMPACT: f32 = 360.0;
/// A scrollbar's thickness, its gap from the picture's edge and its thumb's least length.
const THICK: f32 = 6.0;
const EDGE: f32 = 3.0;
const MIN_THUMB: f32 = 24.0;
/// Two left presses this close in time and place are a double click.
const DOUBLE: Duration = Duration::from_millis(400);
const SLOP: f32 = 4.0;
/// The saved image's pixel density.
pub const EXPORT_SCALE: f32 = 2.0;

thread_local! {
    /// While a picture draws its content: the part of it shown, from the content's corner.
    static VISIBLE: Cell<Option<Rectangle>> = const { Cell::new(None) };
}

/// While a picture draws its content (a canvas's `draw`): the part of the content shown, from its corner. A chart
/// keeps its axis labels on it as it is zoomed in, as a map keeps its scale; none outside a picture (the content is all
/// shown).
pub fn visible() -> Option<Rectangle> {
    VISIBLE.with(Cell::get)
}

/// Draws with [`visible`] saying `shown`.
fn showing<T>(shown: Rectangle, draw: impl FnOnce() -> T) -> T {
    let previous = VISIBLE.with(|v| v.replace(Some(shown)));
    let out = draw();
    VISIBLE.with(|v| v.set(previous));
    out
}

/// What the picture shows: its zoom (1 fitted) and how far its view has moved over the zoomed content (logical px,
/// from the content's corner).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub zoom: f32,
    pub offset: Vector,
}

impl Default for View {
    fn default() -> Self {
        Self { zoom: 1.0, offset: Vector::ZERO }
    }
}

/// The view after zooming by `factor` about `at` (a point of the view from its corner, of `size`): the content's point
/// under it stays where it is, the zoom within 1 and [`MAX_ZOOM`], the view on the content.
pub fn zoomed(view: View, size: Size, factor: f32, at: Vector) -> View {
    let zoom = (view.zoom * factor).clamp(1.0, MAX_ZOOM);
    let k = zoom / view.zoom;
    let offset = Vector::new((view.offset.x + at.x) * k - at.x, (view.offset.y + at.y) * k - at.y);
    View { zoom, offset: clamped(offset, size, zoom) }
}

/// `offset` kept on the content: from 0 to the zoomed size less the view's.
pub fn clamped(offset: Vector, size: Size, zoom: f32) -> Vector {
    let most = |side: f32| (side * (zoom - 1.0)).max(0.0);
    Vector::new(offset.x.clamp(0.0, most(size.width)), offset.y.clamp(0.0, most(size.height)))
}

/// What a wheel's turn does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Turn {
    /// Zoom by this factor about the cursor: the wheel (1,2 a line), a touchpad's vertical scroll.
    Zoom(f32),
    /// Move the view by this much (px): Shift with the wheel, a tilted wheel, a touchpad's horizontal scroll.
    Pan(Vector),
}

/// A wheel's turn read as iced's scrollable reads it (Shift turns it sideways), but the plain vertical wheel zooms.
pub fn turn(delta: mouse::ScrollDelta, shift: bool) -> Turn {
    let (x, y, k) = match delta {
        mouse::ScrollDelta::Lines { x, y } => (x, y, LINE),
        mouse::ScrollDelta::Pixels { x, y } => (x, y, 1.0),
    };
    let (x, y) = if shift { (y, x) } else { (x, y) };
    if shift || x.abs() > y.abs() {
        return Turn::Pan(Vector::new(-x * k, -y * k));
    }
    Turn::Zoom(match delta {
        mouse::ScrollDelta::Lines { .. } => WHEEL.powf(y),
        mouse::ScrollDelta::Pixels { .. } => (PIXEL * y).exp(),
    })
}

/// A saved picture: its name (the window's, for the file's), its size in pixels and its PNG bytes.
#[derive(Clone)]
pub struct Saved {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub png: Arc<Vec<u8>>,
}

impl fmt::Debug for Saved {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Saved({:?}, {}×{}, {} bayt)", self.name, self.width, self.height, self.png.len())
    }
}

/// A button of the bar, for those that drive a picture (tests, pictures for the owner): see [`button_at`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Out,
    Zoom,
    In,
    Fit,
    Save,
}

impl Button {
    fn tip(self) -> &'static str {
        match self {
            Button::Out => "Uzaklaştır",
            Button::Zoom => "Tekerlek yakınlaştırır · Shift+tekerlek ve sürükleme kaydırır · çift tık sığdırır",
            Button::In => "Yakınlaştır",
            Button::Fit => "Sığdır",
            Button::Save => "Resim olarak kaydet (PNG)",
        }
    }

    fn glyph(self) -> Option<Icon> {
        match self {
            Button::Out => Some(Icon::Minus),
            Button::Zoom => None,
            Button::In => Some(Icon::Plus),
            Button::Fit => Some(Icon::ZoomExtents),
            Button::Save => Some(Icon::Save),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Button(Button),
    Separator,
}

fn items(save: bool) -> Vec<Item> {
    let mut items = vec![
        Item::Button(Button::Out),
        Item::Button(Button::Zoom),
        Item::Button(Button::In),
        Item::Separator,
        Item::Button(Button::Fit),
    ];
    if save {
        items.extend([Item::Separator, Item::Button(Button::Save)]);
    }
    items
}

/// The bar's cells from its corner: each item's place, and the bar's size; `compact`, the zoom's label left out.
fn cells(items: &[Item], compact: bool) -> (Vec<Rectangle>, Size) {
    let item = typography::scaled(ITEM).round();
    let label = typography::scaled(LABEL).round();
    let mut x = PAD;
    let cells = items
        .iter()
        .map(|it| {
            let width = match it {
                Item::Button(Button::Zoom) if compact => 0.0,
                Item::Button(Button::Zoom) => label,
                Item::Button(_) => item,
                Item::Separator => SEPARATOR,
            };
            let cell = Rectangle::new(Point::new(x, PAD), Size::new(width, item));
            x += width;
            cell
        })
        .collect();
    (cells, Size::new(x + PAD, item + PAD * 2.0))
}

/// The bar's corner in a picture of `size`: its top right, `MARGIN` in; at the left edge where it does not fit.
fn bar_at(size: Size, bar: Size) -> Point {
    Point::new((size.width - MARGIN - bar.width).max(MARGIN.min(size.width - bar.width).max(0.0)), MARGIN)
}

/// The centre of `button` on the bar of a picture at `bounds` that saves (a picture of a window).
pub fn button_at(bounds: Rectangle, button: Button) -> Point {
    let items = items(true);
    let (cells, bar) = cells(&items, bounds.width < COMPACT);
    let corner = bar_at(bounds.size(), bar);
    let index = items.iter().position(|it| *it == Item::Button(button)).unwrap_or(0);
    let cell = cells[index];
    Point::new(bounds.x + corner.x + cell.center_x(), bounds.y + corner.y + cell.center_y())
}

/// An axis of the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

/// A zoomed picture's scrollbar along `axis` in the picture at `bounds`, the vertical one under `below` (the bar's
/// bottom): its track and its thumb. None while fitted or where there is no room.
pub fn scrollbar(bounds: Rectangle, below: f32, view: View, axis: Axis) -> Option<(Rectangle, Rectangle)> {
    if view.zoom <= 1.0 + 1e-4 {
        return None;
    }
    let (track, extent, offset) = match axis {
        Axis::X => (
            Rectangle::new(
                Point::new(bounds.x + EDGE, bounds.y + bounds.height - EDGE - THICK),
                Size::new((bounds.width - 3.0 * EDGE - THICK).max(0.0), THICK),
            ),
            bounds.width,
            view.offset.x,
        ),
        Axis::Y => {
            let top = (bounds.y + EDGE).max(below + EDGE);
            (
                Rectangle::new(
                    Point::new(bounds.x + bounds.width - EDGE - THICK, top),
                    Size::new(THICK, (bounds.y + bounds.height - 2.0 * EDGE - THICK - top).max(0.0)),
                ),
                bounds.height,
                view.offset.y,
            )
        }
    };
    let length = along(track.size(), axis);
    if length < MIN_THUMB {
        return None;
    }
    let thumb = (length / view.zoom).max(MIN_THUMB).min(length);
    let most = extent * (view.zoom - 1.0);
    let at = if most > 0.0 { offset / most * (length - thumb) } else { 0.0 };
    let thumb = match axis {
        Axis::X => Rectangle::new(Point::new(track.x + at, track.y), Size::new(thumb, THICK)),
        Axis::Y => Rectangle::new(Point::new(track.x, track.y + at), Size::new(THICK, thumb)),
    };
    Some((track, thumb))
}

/// A size's length along `axis`.
fn along(size: Size, axis: Axis) -> f32 {
    match axis {
        Axis::X => size.width,
        Axis::Y => size.height,
    }
}

fn coordinate(point: Point, axis: Axis) -> f32 {
    match axis {
        Axis::X => point.x,
        Axis::Y => point.y,
    }
}

/// The offset along `axis` that puts the thumb's start `at` from the track's start.
fn offset_for(track: Rectangle, thumb: Rectangle, extent: f32, zoom: f32, at: f32, axis: Axis) -> f32 {
    let room = (along(track.size(), axis) - along(thumb.size(), axis)).max(1e-6);
    (at / room).clamp(0.0, 1.0) * extent * (zoom - 1.0)
}

/// A picture that zooms, pans and saves itself.
pub struct Picture<'a, Message> {
    /// The content; then each button's glyph (a space for the zoom's label) and tip.
    elements: Vec<Element<'a, Message>>,
    items: Vec<Item>,
    name: String,
    on_save: Option<Box<dyn Fn(Saved) -> Message + 'a>>,
    id: Option<Id>,
    width: Length,
    height: Length,
}

impl<'a, Message: Clone + 'a> Picture<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            elements: vec![content.into()],
            items: Vec::new(),
            name: "Çizim".into(),
            on_save: None,
            id: None,
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    /// The picture's name: the saved file's.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Resim olarak kaydet: the bar's last button; the saved picture goes with the message.
    pub fn on_save(mut self, on_save: impl Fn(Saved) -> Message + 'a) -> Self {
        self.on_save = Some(Box::new(on_save));
        self
    }

    /// The id a [`View`] is read and set by (`Operation::custom`).
    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

impl<'a, Message: Clone + 'a> From<Picture<'a, Message>> for Element<'a, Message> {
    fn from(mut picture: Picture<'a, Message>) -> Self {
        picture.items = items(picture.on_save.is_some() && cfg!(feature = "snapshot"));
        let buttons: Vec<Button> =
            picture.items.iter().filter_map(|it| if let Item::Button(b) = it { Some(*b) } else { None }).collect();
        for button in buttons {
            let glyph: Element<'a, Message> = match button.glyph() {
                Some(g) => icon(g).size(15.0).into(),
                None => iced::widget::space::Space::new().into(),
            };
            picture.elements.push(glyph);
            picture.elements.push(
                container(label::caption(button.tip()).style(style::text::default))
                    .padding([4, 8])
                    .style(style::container::popover)
                    .into(),
            );
        }
        Element::new(picture)
    }
}

#[derive(Debug)]
struct State {
    view: View,
    /// A drag moving the view, one moving a scrollbar's thumb.
    drag: Option<Drag>,
    thumb: Option<Thumb>,
    /// The bar's item under the cursor and the one pressed; the scrollbar under the cursor.
    hovered: Option<usize>,
    pressed: Option<usize>,
    over_scrollbar: Option<Axis>,
    /// The last left press on the picture: a second one soon and near is a double click.
    click: Option<(Instant, Point)>,
    modifiers: keyboard::Modifiers,
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    from: Point,
    offset: Vector,
}

/// A thumb held: its axis and where along it it was taken.
#[derive(Debug, Clone, Copy)]
struct Thumb {
    axis: Axis,
    grab: f32,
}

impl<'a, Message: Clone + 'a> Picture<'a, Message> {
    /// The button's index among the buttons (its glyph is `1 + 2k`, its tip `2 + 2k`).
    fn button_index(&self, item: usize) -> Option<usize> {
        matches!(self.items.get(item), Some(Item::Button(_)))
            .then(|| self.items[..item].iter().filter(|it| matches!(it, Item::Button(_))).count())
    }

    fn enabled(&self, button: Button, view: View) -> bool {
        match button {
            Button::Out | Button::Fit => view.zoom > 1.0 + 1e-4,
            Button::In => view.zoom < MAX_ZOOM - 1e-4,
            Button::Zoom => false,
            Button::Save => self.on_save.is_some(),
        }
    }

    /// The bar's item under the cursor.
    fn hit(&self, bar: Layout<'_>, cursor: mouse::Cursor) -> Option<usize> {
        let point = cursor.position_over(bar.bounds())?;
        bar.children()
            .position(|cell| cell.bounds().contains(point))
            .filter(|i| matches!(self.items.get(*i), Some(Item::Button(_))))
    }

    /// Draws the view again as an image: what is shown, at twice the pixels, in the light theme on its plain ground.
    #[cfg(feature = "snapshot")]
    fn export(&mut self, view: View, size: Size) -> Option<Saved> {
        let mut snapshot = crate::snapshot::Snapshot::software(size).ok()?.scale(EXPORT_SCALE);
        let theme = crate::theme::theme(crate::theme::Mode::Light, crate::theme::Accent::default());
        let shown = Element::new(Shown { content: &mut self.elements[0], view });
        let image = snapshot.render(shown, &theme);
        let png = image.png().ok()?;
        Some(Saved { name: self.name.clone(), width: image.width, height: image.height, png: Arc::new(png) })
    }

    #[cfg(not(feature = "snapshot"))]
    fn export(&mut self, _: View, _: Size) -> Option<Saved> {
        None
    }
}

/// The scrollbar under the cursor: its axis, track and thumb.
fn scrollbar_under(
    bounds: Rectangle,
    below: f32,
    view: View,
    cursor: mouse::Cursor,
) -> Option<(Axis, Rectangle, Rectangle)> {
    let point = cursor.position()?;
    [Axis::X, Axis::Y].into_iter().find_map(|axis| {
        let (track, thumb) = scrollbar(bounds, below, view, axis)?;
        // A little wider than drawn: a 6 px bar is hard to catch.
        let catch =
            Rectangle::new(Point::new(track.x - 2.0, track.y - 2.0), Size::new(track.width + 4.0, track.height + 4.0));
        catch.contains(point).then_some((axis, track, thumb))
    })
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Picture<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            view: View::default(),
            drag: None,
            thumb: None,
            hovered: None,
            pressed: None,
            over_scrollbar: None,
            click: None,
            modifiers: keyboard::Modifiers::default(),
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.elements.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.elements);
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let size = limits.resolve(self.width, self.height, Size::ZERO);
        // The zoom only: the offset is kept on the content where it is used, against the final size. A dialog lays its
        // body out twice (at its least first, then at its own height, docs/adr/0746): clamping here clamped the offset
        // to the least and a zoom about a point low in a full screen window jumped up (1600 → 815, docs/adr/0937 §1).
        let zoom = tree.state.downcast_ref::<State>().view.zoom;
        let zoomed = Size::new(size.width * zoom, size.height * zoom);
        let content =
            self.elements[0].as_widget_mut().layout(&mut tree.children[0], renderer, &Limits::new(zoomed, zoomed));
        let (cells, bar) = cells(&self.items, size.width < COMPACT);
        let mut nodes = vec![content];
        let mut bar_cells = Vec::with_capacity(cells.len());
        let mut tips = Vec::new();
        for (index, cell) in cells.iter().enumerate() {
            match self.button_index(index) {
                Some(k) => {
                    let glyph = self.elements[1 + 2 * k].as_widget_mut().layout(
                        &mut tree.children[1 + 2 * k],
                        renderer,
                        &Limits::new(Size::ZERO, cell.size()),
                    );
                    let g = glyph.size();
                    let at =
                        Point::new(((cell.width - g.width) / 2.0).round(), ((cell.height - g.height) / 2.0).round());
                    bar_cells.push(Node::with_children(cell.size(), vec![glyph.move_to(at)]).move_to(cell.position()));
                    tips.push(self.elements[2 + 2 * k].as_widget_mut().layout(
                        &mut tree.children[2 + 2 * k],
                        renderer,
                        &Limits::new(Size::ZERO, Size::new(f32::INFINITY, f32::INFINITY)),
                    ));
                }
                None => bar_cells.push(Node::new(cell.size()).move_to(cell.position())),
            }
        }
        nodes.push(Node::with_children(bar, bar_cells).move_to(bar_at(size, bar)));
        nodes.extend(tips);
        Node::with_children(size, nodes)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        let state = tree.state.downcast_mut::<State>();
        let before = state.view;
        operation.custom(self.id.as_ref(), layout.bounds(), &mut state.view);
        if state.view != before {
            state.view.zoom = state.view.zoom.clamp(1.0, MAX_ZOOM);
            state.view.offset = clamped(state.view.offset, layout.bounds().size(), state.view.zoom);
        }
        if let Some(content) = layout.children().next() {
            self.elements[0].as_widget_mut().operate(&mut tree.children[0], content, renderer, operation);
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
        let mut children = layout.children();
        let (Some(content), Some(bar)) = (children.next(), children.next()) else {
            return;
        };
        let below = bar.bounds().y + bar.bounds().height;
        // The view kept on the content as it is laid out now (a resized picture keeps its zoom).
        {
            let state = tree.state.downcast_mut::<State>();
            state.view.offset = clamped(state.view.offset, bounds.size(), state.view.zoom);
        }
        if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
            tree.state.downcast_mut::<State>().modifiers = *modifiers;
        }
        let over_bar = cursor.is_over(bar.bounds());
        let hit = if over_bar { self.hit(bar, cursor) } else { None };
        // The bar first: its buttons, and nothing under it reaches the content.
        {
            let state = tree.state.downcast_mut::<State>();
            match event {
                Event::Mouse(mouse::Event::CursorMoved { .. }) if hit != state.hovered => {
                    state.hovered = hit;
                    shell.request_redraw();
                }
                Event::Mouse(mouse::Event::CursorLeft)
                    if state.hovered.is_some() || state.over_scrollbar.is_some() =>
                {
                    state.hovered = None;
                    state.over_scrollbar = None;
                    shell.request_redraw();
                }
                Event::Mouse(mouse::Event::ButtonPressed(button)) if over_bar => {
                    state.pressed = hit.filter(|_| *button == mouse::Button::Left);
                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.pressed.is_some() => {
                    let pressed = state.pressed.take();
                    let view = state.view;
                    let size = bounds.size();
                    shell.capture_event();
                    shell.request_redraw();
                    if pressed != hit {
                        return;
                    }
                    let Some(Item::Button(button)) = pressed.and_then(|i| self.items.get(i)).copied() else {
                        return;
                    };
                    if !self.enabled(button, view) {
                        return;
                    }
                    let centre = Vector::new(size.width / 2.0, size.height / 2.0);
                    let next = match button {
                        Button::Out => zoomed(view, size, 1.0 / STEP, centre),
                        Button::In => zoomed(view, size, STEP, centre),
                        Button::Fit => View::default(),
                        Button::Zoom => view,
                        Button::Save => {
                            if let Some(saved) = self.export(view, size)
                                && let Some(on_save) = &self.on_save
                            {
                                shell.publish(on_save(saved));
                            }
                            view
                        }
                    };
                    let state = tree.state.downcast_mut::<State>();
                    if next != state.view {
                        state.view = next;
                        shell.invalidate_layout();
                    }
                    return;
                }
                Event::Mouse(mouse::Event::WheelScrolled { .. }) if over_bar => {
                    shell.capture_event();
                    return;
                }
                _ => {}
            }
        }
        // The scrollbars next: a thumb taken, moved and let go; a press on the track jumps there.
        {
            let state = tree.state.downcast_mut::<State>();
            let view = state.view;
            let size = bounds.size();
            match event {
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    if let Some(thumb) = state.thumb {
                        if let Some((track, held)) = scrollbar(bounds, below, view, thumb.axis) {
                            let at = coordinate(*position, thumb.axis)
                                - coordinate(track.position(), thumb.axis)
                                - thumb.grab;
                            let value = offset_for(track, held, along(size, thumb.axis), view.zoom, at, thumb.axis);
                            let offset = match thumb.axis {
                                Axis::X => Vector::new(value, view.offset.y),
                                Axis::Y => Vector::new(view.offset.x, value),
                            };
                            if offset != state.view.offset {
                                state.view.offset = offset;
                                shell.request_redraw();
                            }
                        }
                        shell.capture_event();
                        return;
                    }
                    let over = scrollbar_under(bounds, below, view, cursor).map(|s| s.0);
                    if over != state.over_scrollbar {
                        state.over_scrollbar = over;
                        shell.request_redraw();
                    }
                }
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if !over_bar => {
                    if let Some((axis, track, thumb)) = scrollbar_under(bounds, below, view, cursor)
                        && let Some(p) = cursor.position()
                    {
                        let along_thumb = coordinate(p, axis) - coordinate(thumb.position(), axis);
                        let length = along(thumb.size(), axis);
                        // On the thumb: held where it was taken; on the track: the thumb's middle jumps there.
                        let grab = if (0.0..=length).contains(&along_thumb) { along_thumb } else { length / 2.0 };
                        let at = coordinate(p, axis) - coordinate(track.position(), axis) - grab;
                        let value = offset_for(track, thumb, along(size, axis), view.zoom, at, axis);
                        state.view.offset = match axis {
                            Axis::X => Vector::new(value, view.offset.y),
                            Axis::Y => Vector::new(view.offset.x, value),
                        };
                        state.thumb = Some(Thumb { axis, grab });
                        state.click = None;
                        shell.capture_event();
                        shell.request_redraw();
                        return;
                    }
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.thumb.is_some() => {
                    state.thumb = None;
                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }
                _ => {}
            }
        }
        let offset = tree.state.downcast_ref::<State>().view.offset;
        let cursor = if over_bar { cursor.levitate() } else { cursor };
        // The content next, the cursor and the viewport moved with the view: it may take the event (a km line).
        let visible = Rectangle::new(bounds.position() + offset, bounds.size());
        let _ = viewport;
        self.elements[0].as_widget_mut().update(
            &mut tree.children[0],
            event,
            content,
            cursor + offset,
            renderer,
            clipboard,
            shell,
            &visible,
        );
        if shell.is_event_captured() {
            return;
        }
        let state = tree.state.downcast_mut::<State>();
        let at = |p: Point| Vector::new(p.x - bounds.x, p.y - bounds.y);
        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let Some(p) = cursor.position_over(bounds).filter(|_| !cursor.is_levitating()) else {
                    return;
                };
                let next = match turn(*delta, state.modifiers.shift()) {
                    Turn::Zoom(factor) => zoomed(state.view, bounds.size(), factor, at(p)),
                    Turn::Pan(by) => View {
                        zoom: state.view.zoom,
                        offset: clamped(state.view.offset + by, bounds.size(), state.view.zoom),
                    },
                };
                shell.capture_event();
                if next != state.view {
                    if next.zoom != state.view.zoom {
                        shell.invalidate_layout();
                    }
                    state.view = next;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(button @ (mouse::Button::Left | mouse::Button::Middle))) => {
                let Some(p) = cursor.position_over(bounds).filter(|_| !cursor.is_levitating()) else {
                    return;
                };
                shell.capture_event();
                if *button == mouse::Button::Left {
                    let now = crate::clock::now();
                    let double = state
                        .click
                        .is_some_and(|(then, q)| now.duration_since(then) <= DOUBLE && p.distance(q) <= SLOP);
                    if double {
                        state.click = None;
                        if state.view != View::default() {
                            state.view = View::default();
                            shell.invalidate_layout();
                            shell.request_redraw();
                        }
                        return;
                    }
                    state.click = Some((now, p));
                }
                if state.view.zoom > 1.0 {
                    state.drag = Some(Drag { from: p, offset: state.view.offset });
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(drag) = state.drag {
                    let moved = Vector::new(position.x - drag.from.x, position.y - drag.from.y);
                    let offset = clamped(drag.offset - moved, bounds.size(), state.view.zoom);
                    if offset != state.view.offset {
                        state.view.offset = offset;
                        shell.request_redraw();
                    }
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left | mouse::Button::Middle))
                if state.drag.is_some() =>
            {
                state.drag = None;
                shell.capture_event();
                shell.request_redraw();
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
        let mut children = layout.children();
        let (Some(content), Some(bar)) = (children.next(), children.next()) else {
            return mouse::Interaction::None;
        };
        if cursor.is_over(bar.bounds()) {
            return match self.hit(bar, cursor).and_then(|i| self.items.get(i)) {
                Some(Item::Button(button)) if self.enabled(*button, state.view) => mouse::Interaction::Pointer,
                _ => mouse::Interaction::Idle,
            };
        }
        if state.drag.is_some() {
            return mouse::Interaction::Grabbing;
        }
        let bounds = layout.bounds();
        let below = bar.bounds().y + bar.bounds().height;
        if state.thumb.is_some() || scrollbar_under(bounds, below, state.view, cursor).is_some() {
            return mouse::Interaction::Idle;
        }
        let offset = clamped(state.view.offset, layout.bounds().size(), state.view.zoom);
        let visible = Rectangle::new(bounds.position() + offset, bounds.size());
        let _ = viewport;
        let inner = self.elements[0].as_widget().mouse_interaction(
            &tree.children[0],
            content,
            cursor + offset,
            &visible,
            renderer,
        );
        match inner {
            mouse::Interaction::None | mouse::Interaction::Idle
                if state.view.zoom > 1.0 && cursor.is_over(bounds) =>
            {
                mouse::Interaction::Grab
            }
            other => other,
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
        let bounds = layout.bounds();
        let mut children = layout.children();
        let (Some(content), Some(bar)) = (children.next(), children.next()) else {
            return;
        };
        let tips: Vec<Layout<'_>> = children.collect();
        let over_bar = cursor.is_over(bar.bounds());
        let offset = clamped(state.view.offset, layout.bounds().size(), state.view.zoom);
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };
        let visible = Rectangle::new(bounds.position() + offset, bounds.size());
        let content_cursor = (if over_bar { cursor.levitate() } else { cursor }) + offset;
        let shown = Rectangle::new(Point::new(offset.x, offset.y), bounds.size());
        renderer.with_layer(clip, |renderer| {
            renderer.with_translation(Vector::new(-offset.x, -offset.y), |renderer| {
                showing(shown, || {
                    self.elements[0].as_widget().draw(
                        &tree.children[0],
                        renderer,
                        theme,
                        style,
                        content,
                        content_cursor,
                        &visible,
                    );
                });
            });
        });

        let t = Tokens::of(theme);
        let below = bar.bounds().y + bar.bounds().height;
        let hovered = if over_bar { state.hovered } else { None };
        // Out of the picture the bar is only the zoom's pill at its right end: the drawing is not covered, the pill says
        // it zooms and how far. The cursor in, the whole group opens (as a plotting library's mode bar).
        let open = cursor.is_over(bounds) || state.pressed.is_some() || state.drag.is_some() || state.thumb.is_some();
        let zoom_words = format!("%{}", (state.view.zoom * 100.0).round() as i64);
        let zoom_color = if state.view.zoom > 1.0 + 1e-4 { t.text } else { t.muted };
        renderer.with_layer(clip, |renderer| {
            // Where the view is on the zoomed content: a quiet bar along the bottom and the right edge.
            for axis in [Axis::X, Axis::Y] {
                let Some((track, thumb)) = scrollbar(bounds, below, state.view, axis) else {
                    continue;
                };
                let active = state.thumb.is_some_and(|h| h.axis == axis) || state.over_scrollbar == Some(axis);
                let round = Border { radius: (THICK / 2.0).into(), ..Border::default() };
                renderer.fill_quad(
                    Quad { bounds: track, border: round, ..Quad::default() },
                    Background::Color(t.text.scale_alpha(if active { 0.10 } else { 0.05 })),
                );
                renderer.fill_quad(
                    Quad { bounds: thumb, border: round, ..Quad::default() },
                    Background::Color(t.text.scale_alpha(if active { 0.55 } else { 0.32 })),
                );
            }
            if !open {
                let label = typography::scaled(LABEL).round();
                let b = bar.bounds();
                let pill = Rectangle::new(Point::new(b.x + b.width - label, b.y), Size::new(label, b.height));
                renderer.fill_quad(
                    Quad {
                        bounds: pill,
                        border: Border { color: t.border, width: 1.0, radius: crate::theme::shape::radius(6.0).into() },
                        shadow: Shadow {
                            color: t.shadow().scale_alpha(0.4),
                            offset: Vector::new(0.0, 1.0),
                            blur_radius: 4.0,
                        },
                        ..Quad::default()
                    },
                    Background::Color(t.popover.scale_alpha(0.92)),
                );
                renderer.fill_text(
                    text_core::Text {
                        content: zoom_words.clone(),
                        bounds: pill.size(),
                        size: Pixels(typography::caption()),
                        line_height: text_core::LineHeight::default(),
                        font: typography::mono(),
                        align_x: text_core::Alignment::Center,
                        align_y: Vertical::Center,
                        shaping: text_core::Shaping::Basic,
                        wrapping: text_core::Wrapping::None,
                    },
                    pill.center(),
                    zoom_color,
                    pill,
                );
                return;
            }
            renderer.fill_quad(
                Quad {
                    bounds: bar.bounds(),
                    border: Border { color: t.border, width: 1.0, radius: crate::theme::shape::radius(6.0).into() },
                    shadow: Shadow {
                        color: t.shadow().scale_alpha(0.6),
                        offset: Vector::new(0.0, 1.0),
                        blur_radius: 6.0,
                    },
                    ..Quad::default()
                },
                Background::Color(t.popover),
            );
            for (index, (cell, item)) in bar.children().zip(&self.items).enumerate() {
                let b = cell.bounds();
                let Item::Button(button) = item else {
                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle::new(
                                Point::new(b.center_x().floor(), b.y + 6.0),
                                Size::new(1.0, b.height - 12.0),
                            ),
                            ..Quad::default()
                        },
                        Background::Color(t.border),
                    );
                    continue;
                };
                let enabled = self.enabled(*button, state.view);
                if *button == Button::Zoom {
                    if b.width < 1.0 {
                        continue;
                    }
                    renderer.fill_text(
                        text_core::Text {
                            content: zoom_words.clone(),
                            bounds: b.size(),
                            size: Pixels(typography::caption()),
                            line_height: text_core::LineHeight::default(),
                            font: typography::mono(),
                            align_x: text_core::Alignment::Center,
                            align_y: Vertical::Center,
                            shaping: text_core::Shaping::Basic,
                            wrapping: text_core::Wrapping::None,
                        },
                        b.center(),
                        zoom_color,
                        b,
                    );
                    continue;
                }
                if enabled && hovered == Some(index) {
                    let pressed = state.pressed == Some(index);
                    renderer.fill_quad(
                        Quad {
                            bounds: b,
                            border: Border { radius: crate::theme::shape::radius(4.0).into(), ..Border::default() },
                            ..Quad::default()
                        },
                        Background::Color(if pressed { t.layer(0.14) } else { t.surface_hover }),
                    );
                }
                let color = if enabled { t.text } else { t.disabled() };
                if let (Some(k), Some(glyph)) = (self.button_index(index), cell.children().next()) {
                    self.elements[1 + 2 * k].as_widget().draw(
                        &tree.children[1 + 2 * k],
                        renderer,
                        theme,
                        &renderer::Style { text_color: color },
                        glyph,
                        cursor,
                        viewport,
                    );
                }
            }
        });

        // The hovered button's name, under the bar, kept inside the picture.
        let Some(index) = state.hovered.filter(|_| over_bar) else {
            return;
        };
        let (Some(k), Some(cell)) = (self.button_index(index), bar.children().nth(index)) else {
            return;
        };
        let Some(tip) = tips.get(k) else {
            return;
        };
        let size = tip.bounds().size();
        let cell = cell.bounds();
        let x = (cell.center_x() - size.width / 2.0)
            .min(bounds.x + bounds.width - size.width - 2.0)
            .max(bounds.x + 2.0);
        let y = bar.bounds().y + bar.bounds().height + TIP_GAP;
        let origin = tip.bounds().position();
        renderer.with_layer(*viewport, |renderer| {
            renderer.with_translation(Vector::new(x - origin.x, y - origin.y), |renderer| {
                self.elements[2 + 2 * k].as_widget().draw(
                    &tree.children[2 + 2 * k],
                    renderer,
                    theme,
                    style,
                    *tip,
                    mouse::Cursor::Unavailable,
                    viewport,
                );
            });
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
        let view = tree.state.downcast_ref::<State>().view;
        let offset = clamped(view.offset, layout.bounds().size(), view.zoom);
        let content = layout.children().next()?;
        self.elements[0].as_widget_mut().overlay(&mut tree.children[0], content, renderer, viewport, translation - offset)
    }
}

/// A picture's content shown as the picture shows it, for the saved image: laid out zoomed, moved by the offset.
#[cfg_attr(not(feature = "snapshot"), allow(dead_code))]
struct Shown<'b, 'a, Message> {
    content: &'b mut Element<'a, Message>,
    view: View,
}

impl<'a, Message> Widget<Message, Theme, Renderer> for Shown<'_, 'a, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(self.content.as_widget())]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.content.as_widget()]);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let size = limits.max();
        let zoomed = Size::new(size.width * self.view.zoom, size.height * self.view.zoom);
        let content =
            self.content.as_widget_mut().layout(&mut tree.children[0], renderer, &Limits::new(zoomed, zoomed));
        Node::with_children(size, vec![content])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _: mouse::Cursor,
        _: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let Some(content) = layout.children().next() else {
            return;
        };
        let offset = self.view.offset;
        let visible = Rectangle::new(bounds.position() + offset, bounds.size());
        let shown = Rectangle::new(Point::new(offset.x, offset.y), bounds.size());
        renderer.with_layer(bounds, |renderer| {
            renderer.with_translation(Vector::new(-offset.x, -offset.y), |renderer| {
                showing(shown, || {
                    self.content.as_widget().draw(
                        &tree.children[0],
                        renderer,
                        theme,
                        style,
                        content,
                        mouse::Cursor::Unavailable,
                        &visible,
                    );
                });
            });
        });
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use std::sync::{Arc, Mutex};

    use iced::advanced::widget::{Id, Operation};
    use iced::widget::canvas::{self, Path};
    use iced::{Color, Element, Event, Point, Rectangle, Renderer, Size, Theme, Vector, keyboard, mouse};

    use super::{
        Axis, Button, MAX_ZOOM, Picture, Saved, Turn, View, bar_at, button_at, cells, clamped, items, scrollbar, turn,
        zoomed,
    };
    use crate::snapshot::{Input, Snapshot};

    #[test]
    fn the_point_under_the_wheel_stays() {
        let size = Size::new(400.0, 200.0);
        let at = Vector::new(100.0, 50.0);
        let v = zoomed(View::default(), size, 2.0, at);
        assert_eq!(v.zoom, 2.0);
        // The content's point under the cursor was (100, 50); zoomed it is (200, 100), still under the cursor.
        assert_eq!(v.offset, Vector::new(100.0, 50.0));
        // Back out: fitted again.
        assert_eq!(zoomed(v, size, 0.5, at), View::default());
        // Never under 1, never over the most; the view stays on the content.
        assert_eq!(zoomed(View::default(), size, 0.5, at).zoom, 1.0);
        assert_eq!(zoomed(View { zoom: 30.0, offset: Vector::ZERO }, size, 2.0, Vector::ZERO).zoom, MAX_ZOOM);
        assert_eq!(clamped(Vector::new(-5.0, 900.0), size, 2.0), Vector::new(0.0, 200.0));
    }

    #[test]
    fn the_wheel_zooms_and_sideways_it_pans() {
        let lines = |x: f32, y: f32| mouse::ScrollDelta::Lines { x, y };
        assert_eq!(turn(lines(0.0, 1.0), false), Turn::Zoom(1.2));
        assert!(matches!(turn(lines(0.0, -1.0), false), Turn::Zoom(f) if f < 1.0));
        // Shift with the wheel, a tilted wheel, a touchpad's horizontal scroll: sideways, as a scrollable reads them.
        assert_eq!(turn(lines(0.0, -1.0), true), Turn::Pan(Vector::new(48.0, 0.0)));
        assert_eq!(turn(lines(-1.0, 0.0), false), Turn::Pan(Vector::new(48.0, 0.0)));
        assert_eq!(turn(mouse::ScrollDelta::Pixels { x: -30.0, y: 2.0 }, false), Turn::Pan(Vector::new(30.0, -2.0)));
    }

    #[test]
    fn the_bar_sits_top_right_and_holds_its_buttons() {
        let (cells, bar) = cells(&items(true), false);
        assert_eq!(cells.len(), 7);
        assert!(bar.width > 150.0 && bar.width < 220.0, "{bar:?}");
        assert_eq!(bar_at(Size::new(600.0, 300.0), bar), Point::new(600.0 - 8.0 - bar.width, 8.0));
        // Too narrow for the bar: at the left edge.
        assert_eq!(bar_at(Size::new(100.0, 300.0), bar).x, 0.0);
        // A narrow picture's bar leaves the zoom's label out (the pill shows it when the cursor is away).
        let (narrow, small) = super::cells(&items(true), true);
        assert_eq!(narrow[1].width, 0.0);
        assert!(small.width < bar.width - 40.0, "{small:?}");
    }

    #[test]
    fn the_scrollbars_say_where_the_view_is() {
        let bounds = Rectangle::new(Point::new(10.0, 20.0), Size::new(600.0, 300.0));
        // Fitted: none.
        assert!(scrollbar(bounds, 60.0, View::default(), Axis::X).is_none());
        // Zoomed 2 at the middle: the thumb is half the track, in its middle.
        let view = View { zoom: 2.0, offset: Vector::new(300.0, 150.0) };
        let (track, thumb) = scrollbar(bounds, 60.0, view, Axis::X).expect("a bar");
        assert_eq!(track.width, 600.0 - 15.0);
        assert!((thumb.width - track.width / 2.0).abs() < 1e-3);
        assert!((thumb.x - (track.x + track.width / 4.0)).abs() < 1e-3, "{thumb:?}");
        // The vertical one starts under the bar.
        let (track, _) = scrollbar(bounds, 60.0, view, Axis::Y).expect("a bar");
        assert_eq!(track.y, 63.0);
    }

    /// A content that marks its centre: a red square a tenth of its width, so it grows as the content is laid out
    /// larger (a symbol of a fixed size would stay as it is: the zoom lays out again, it does not magnify).
    struct Mark;

    impl canvas::Program<Saved> for Mark {
        type State = ();

        fn draw(
            &self,
            _: &(),
            renderer: &Renderer,
            _: &Theme,
            bounds: Rectangle,
            _: mouse::Cursor,
        ) -> Vec<canvas::Geometry> {
            let mut frame = canvas::Frame::new(renderer, bounds.size());
            let c = Point::new(bounds.width / 2.0, bounds.height / 2.0);
            let half = bounds.width / 20.0;
            frame.fill(
                &Path::rectangle(Point::new(c.x - half, c.y - half), Size::new(2.0 * half, 2.0 * half)),
                Color::from_rgb(1.0, 0.0, 0.0),
            );
            vec![frame.into_geometry()]
        }
    }

    /// Reads the picture's view and its bounds.
    #[derive(Clone)]
    struct Read(Arc<Mutex<Option<(View, Rectangle)>>>);

    impl Operation for Read {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
            operate(self);
        }

        fn custom(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn std::any::Any) {
            if id == Some(&Id::new("p"))
                && let Some(view) = state.downcast_ref::<View>()
            {
                *self.0.lock().unwrap() = Some((*view, bounds));
            }
        }
    }

    struct App {
        saved: Vec<Saved>,
    }

    fn view(_: &App) -> Element<'_, Saved> {
        Picture::new(canvas::Canvas::new(Mark).width(iced::Fill).height(iced::Fill))
            .name("Deneme")
            .id(Id::new("p"))
            .on_save(|s| s)
            .into()
    }

    fn read(snapshot: &mut Snapshot, app: &App) -> (View, Rectangle) {
        let found = Read(Arc::new(Mutex::new(None)));
        snapshot.operate(view(app), Box::new(found.clone()));
        found.0.lock().unwrap().expect("the picture")
    }

    #[test]
    fn the_wheel_zooms_a_drag_pans_a_double_click_fits_and_the_bar_saves() {
        let mut snapshot = Snapshot::new(Size::new(600.0, 300.0)).expect("çizici kurulamadı");
        let mut app = App { saved: Vec::new() };
        let mut update = |app: &mut App, saved: Saved| app.saved.push(saved);
        snapshot.settle(&mut app, view, &mut update);
        let (v, bounds) = read(&mut snapshot, &app);
        assert_eq!((v, bounds.size()), (View::default(), Size::new(600.0, 300.0)));
        // Two wheel lines at (150, 100): ×1,44, the point under the cursor stays.
        snapshot.input(&mut app, view, &mut update, Input::Scroll(Point::new(150.0, 100.0), 2.0));
        let (v, _) = read(&mut snapshot, &app);
        assert!((v.zoom - 1.44).abs() < 1e-4, "{v:?}");
        assert!((v.offset.x - 150.0 * 0.44).abs() < 1e-3 && (v.offset.y - 100.0 * 0.44).abs() < 1e-3, "{v:?}");
        // A drag of (−40, −20) moves the view by (40, 20) over the content.
        let before = v.offset;
        snapshot.input(&mut app, view, &mut update, Input::Drag(Point::new(300.0, 200.0), Point::new(260.0, 180.0)));
        let (v, _) = read(&mut snapshot, &app);
        assert!(
            (v.offset.x - (before.x + 40.0)).abs() < 1e-3 && (v.offset.y - (before.y + 20.0)).abs() < 1e-3,
            "{v:?}"
        );
        // A double click fits.
        snapshot.input(&mut app, view, &mut update, Input::Click(Point::new(300.0, 250.0)));
        snapshot.input(&mut app, view, &mut update, Input::Click(Point::new(300.0, 250.0)));
        assert_eq!(read(&mut snapshot, &app).0, View::default());
        // The bar's last button saves the picture: twice the pixels, a PNG.
        snapshot.input(&mut app, view, &mut update, Input::Click(button_at(bounds, Button::Save)));
        assert_eq!(app.saved.len(), 1, "saved once");
        let saved = &app.saved[0];
        assert_eq!((saved.name.as_str(), saved.width, saved.height), ("Deneme", 1200, 600));
        assert_eq!(&saved.png[1..4], b"PNG");
        // Its plus zooms in by 1,5 about the centre.
        snapshot.input(&mut app, view, &mut update, Input::Click(button_at(bounds, Button::In)));
        let (v, _) = read(&mut snapshot, &app);
        assert!(
            (v.zoom - 1.5).abs() < 1e-5 && (v.offset.x - 150.0).abs() < 1e-3 && (v.offset.y - 75.0).abs() < 1e-3,
            "{v:?}"
        );
    }

    #[test]
    fn sideways_it_pans_by_shift_the_touchpad_and_the_scrollbar() {
        let mut snapshot = Snapshot::new(Size::new(600.0, 300.0)).expect("çizici kurulamadı");
        let mut app = App { saved: Vec::new() };
        let mut update = |app: &mut App, saved: Saved| app.saved.push(saved);
        snapshot.settle(&mut app, view, &mut update);
        let (_, bounds) = read(&mut snapshot, &app);
        snapshot.input(&mut app, view, &mut update, Input::Click(button_at(bounds, Button::In)));
        let (v, _) = read(&mut snapshot, &app);
        assert_eq!(v.offset, Vector::new(150.0, 75.0));
        // Shift and the wheel down: 48 px to the right, the zoom as it was.
        let shift = Event::Keyboard(keyboard::Event::ModifiersChanged(keyboard::Modifiers::SHIFT));
        let _ = snapshot.deliver(view(&app), &shift);
        snapshot.input(&mut app, view, &mut update, Input::Scroll(Point::new(300.0, 150.0), -1.0));
        let (v, _) = read(&mut snapshot, &app);
        assert_eq!((v.zoom, v.offset), (1.5, Vector::new(198.0, 75.0)));
        let plain = Event::Keyboard(keyboard::Event::ModifiersChanged(keyboard::Modifiers::default()));
        let _ = snapshot.deliver(view(&app), &plain);
        // A touchpad's horizontal scroll: 30 px further.
        let swipe =
            Event::Mouse(mouse::Event::WheelScrolled { delta: mouse::ScrollDelta::Pixels { x: -30.0, y: 0.0 } });
        let _ = snapshot.deliver(view(&app), &swipe);
        let (v, _) = read(&mut snapshot, &app);
        assert_eq!((v.zoom, v.offset), (1.5, Vector::new(228.0, 75.0)));
        // The horizontal thumb dragged to the track's start: the view at the content's left edge.
        let (track, thumb) = scrollbar(bounds, 40.0, v, Axis::X).expect("a bar");
        let held = Point::new(thumb.center_x(), thumb.center_y());
        snapshot.input(&mut app, view, &mut update, Input::Drag(held, Point::new(held.x - 1000.0, held.y)));
        let (v, _) = read(&mut snapshot, &app);
        assert_eq!((v.zoom, v.offset), (1.5, Vector::new(0.0, 75.0)));
        // A press on the track's far end: the view at the content's right edge.
        snapshot.input(
            &mut app,
            view,
            &mut update,
            Input::Click(Point::new(track.x + track.width - 1.0, track.center_y())),
        );
        let (v, _) = read(&mut snapshot, &app);
        assert!((v.offset.x - 300.0).abs() < 1e-3, "{v:?}");
    }

    /// The saved image is the view: zoomed in on the content's centre, the mark fills more of it.
    #[test]
    fn the_saved_image_is_what_is_shown() {
        let mut snapshot = Snapshot::new(Size::new(200.0, 100.0)).expect("çizici kurulamadı");
        let mut app = App { saved: Vec::new() };
        let mut update = |app: &mut App, saved: Saved| app.saved.push(saved);
        snapshot.settle(&mut app, view, &mut update);
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(200.0, 100.0));
        snapshot.input(&mut app, view, &mut update, Input::Click(button_at(bounds, Button::Save)));
        snapshot.input(&mut app, view, &mut update, Input::Click(button_at(bounds, Button::In)));
        snapshot.input(&mut app, view, &mut update, Input::Click(button_at(bounds, Button::In)));
        snapshot.input(&mut app, view, &mut update, Input::Click(button_at(bounds, Button::Save)));
        assert_eq!(app.saved.len(), 2);
        let red = |s: &Saved| {
            let decoder = png::Decoder::new(std::io::Cursor::new(s.png.as_slice()));
            let mut reader = decoder.read_info().expect("a png");
            let mut buf = vec![0; reader.output_buffer_size().expect("its size")];
            let info = reader.next_frame(&mut buf).expect("a frame");
            buf[..info.buffer_size()].chunks(4).filter(|p| p[0] > 200 && p[1] < 60 && p[2] < 60).count()
        };
        // 20×20 at twice the pixels: 1600; laid out 2,25 times larger: about 5,06 times more.
        let (a, b) = (red(&app.saved[0]), red(&app.saved[1]));
        assert!((1500..1700).contains(&a), "{a}");
        assert!((b as f32 / a as f32 - 5.06).abs() < 0.3, "{a} {b}");
    }
}
