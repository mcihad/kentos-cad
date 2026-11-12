//! The drawing area (docs/adr/0019; TODOS.md REN-01..07): the open drawing
//! drawn by KentOS's own wgpu pipeline (`kentos-render-wgpu`) inside Iced's
//! shader widget, which lends it Iced's device, queue and render pass.
//!
//! The camera is the app's state, not the widget's: opening a drawing fits
//! it, the status bar reads the world point under the pointer, and it
//! survives the widget tree being rebuilt. The scene is a cache of the
//! document: built again only when the document, the theme or the curves'
//! zoom band changes, never per frame or per pointer event. Frames are drawn
//! on demand (Iced redraws after an event), with no loop of their own.
//!
//! Gestures, as on the web: the middle button drags the view, the wheel zooms
//! at the pointer, a middle double click shows everything. A left press gives
//! the running tool a point; a quick right click (under 300 ms) is Enter
//! (docs/adr/0018). The widget reports what happened; the app decides.
//!
//! Multisampling and the pixel ratio come from the typed settings
//! (`graphics.msaa`, `graphics.hiDpi`; docs/adr/0023). Single-sampled at full
//! resolution the area draws in Iced's pass; otherwise it draws into its own
//! targets and composes the picture into Iced's frame (`Primitive::render`).
//! A change reaches the next frame; the status says which sample counts the
//! device takes and which one it could not make.
//!
//! The selection and the hovered object are two more scene parts, drawn after
//! every layer (`highlight.rs`, docs/adr/0029).

pub(crate) mod highlight;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use iced::time::Instant;
use iced::widget::{container, shader, stack};
use iced::{Element, Fill, Point, Rectangle, Vector, mouse, wgpu};

use kentos_commands::{DrawingFont, Entity, LayerNode};
use kentos_interaction::{Cursor, Selection, ViewChange, Viewpoint};
use kentos_render_wgpu::camera::{FIT_PADDING, MAX_SCALE, MIN_SCALE};
use kentos_render_wgpu::scene::{self, lod};
use kentos_render_wgpu::styled::StyledScene;
use kentos_render_wgpu::{
    Bounds, Camera, Drawing, FrameInput, FrameStats, Palette, RenderError, RenderSettings,
    Renderer, Rgba8, SampleFailure, ScenePart, Vec2, ViewId,
};
use kentos_ui::theme::Mode;
use kentos_ui::widget::EmptyState;

use crate::app::Message;
use crate::document::Document;

// ── How the drawing area reads the open drawing ─────────────────────────────
// The one place that knows the document's fields. The scene reads the live
// document in place, with no copy, and is rebuilt when what it holds changes
// (its generation: edits and changes from outside alike).

impl Drawing for Document {
    fn layer_tree(&self) -> &[LayerNode] {
        self.layers()
    }

    fn objects(&self) -> impl Iterator<Item = &Entity> {
        self.model.entities()
    }

    fn anchor(&self) -> kentos_commands::Vec2 {
        self.model.origin()
    }

    fn drawing_font(&self) -> Option<DrawingFont> {
        self.settings().drawing_font
    }
}

/// Changes with every change to what the drawing holds: this user's edits,
/// undo and redo, layer changes, and other editors' changes taken in from
/// the cloud, which leave the revision alone (kentos-domain, docs/adr/0040).
fn changes(doc: &Document) -> u64 {
    doc.model.generation()
}

/// The drawing's start view, when it has one.
fn start_view(doc: &Document) -> Option<Bounds> {
    doc.model.home_view().map(|h| Bounds {
        min_x: h.min_x,
        min_y: h.min_y,
        max_x: h.max_x,
        max_y: h.max_y,
    })
}

/// Wheel zoom per line (a notch) and per pixel of a touchpad: the web's steps.
const WHEEL_LINE: f64 = 0.15;
const WHEEL_PIXEL: f64 = 0.0015;
/// Two middle presses this close in time and place are a double click.
const DOUBLE_CLICK: Duration = Duration::from_millis(500);
const DOUBLE_CLICK_DISTANCE: f32 = 4.0;
/// A right press released sooner is a click (Enter, or the idle menu); held
/// longer it opens the command menu (the web's `RIGHT_HOLD_MS`, drawing_menus.rs).
pub const RIGHT_HOLD: Duration = Duration::from_millis(300);

static NEXT_VIEW: AtomicU64 = AtomicU64::new(1);

/// What the drawing area tells the app.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The area's place in the window and its size, logical pixels.
    Resized(Rectangle),
    /// The pointer moved over the area (logical pixels from its top-left).
    Moved(Point),
    /// The pointer left the area.
    Left,
    /// A middle-button drag moved the pointer by `by`, now at `at`.
    Panned { by: Vector, at: Point },
    /// The wheel turned: zoom by `factor` about `at`.
    Zoomed { factor: f64, at: Point },
    /// Show everything (a middle double click).
    Extents,
    /// Put this world point in the middle of the area.
    CenterOn(Vec2),
    /// The left button went down here (logical pixels from the area's top-left).
    Pressed(Point),
    /// The left button came up here, after going down over the area (the
    /// web's pointer capture: wherever it is released).
    Released(Point),
    /// The right button went down here (Shift and it: the snap menu, drawing_menus.rs).
    RightPressed(Point),
    /// The right button pressed here has been held for [`RIGHT_HOLD`]: a menu opens.
    RightHeld(Point),
    /// The right button went down and up here within [`RIGHT_HOLD`].
    RightClick(Point),
    /// The wheel stopped: symbols of a fixed screen size are built again at
    /// the view's scale (docs/adr/0090).
    Settled,
}

/// What the last frame drew, or why it could not.
#[derive(Debug, Clone, Default)]
pub struct Status {
    pub stats: FrameStats,
    pub error: Option<String>,
    /// The sample counts the device takes (TODOS.md AA-01); empty before the first frame.
    pub supported: Vec<u32>,
    /// A count the area could not draw with; it draws with the last that worked (AA-02).
    pub failure: Option<SampleFailure>,
    /// The styled atlas left images for another frame (docs/adr/0090): the area asks for one.
    pub images_pending: bool,
    /// The window's device pixels per logical pixel, as the last frame was drawn.
    pub scale_factor: f32,
}

thread_local! {
    /// Whether a zoom while the wheel turns may scale the last sharp picture (docs/adr/1068) when the setting asks for
    /// it (`Graphics::zoom_scaled`); off only in a test's control.
    static ZOOM_SCALED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// What the drawing area draws the styled layers with (docs/adr/0090): the
/// geometry store they are built next to, the style library and its
/// pictures, and the view's symbol settings.
#[derive(Clone, Copy)]
pub struct Styling<'a> {
    pub spatial: &'a kentos_interaction::Spatial,
    pub styles: &'a crate::style::Styles,
    /// Semboller → Ekranda sabit (`graphics.symbolSize`).
    pub screen_symbols: bool,
    /// Kalınlık (`graphics.lineWeights`): off draws every line one pixel wide.
    pub line_weights: bool,
    /// Görünüm kipleri (docs/adr/0195): what the layers leave out, and their colours.
    pub view_build: kentos_native_style::View,
    pub view_colors: kentos_native_style::color::ViewColors,
    /// The labels placed this frame: the masks cutting with a layer's labels take
    /// their boxes (docs/adr/0226 §3).
    pub labels: Option<&'a kentos_interaction::spatial::LabelParts>,
}

/// The scene with its masks' label boxes (docs/adr/0226 §3): each source layer's
/// placed labels (a curved one's letters), as grown on screen, from the layers'
/// origin; hidden and unplaced labels are no shapes.
fn with_mask_labels(
    mut scene: StyledScene,
    asks: &[(usize, usize, String)],
    spots: Option<&kentos_interaction::spatial::LabelParts>,
    doc: &kentos_domain::Document,
    origin: kentos_render_wgpu::Vec2,
) -> StyledScene {
    use kentos_geometry_core::labels::engine::{CURVED, HIDDEN, UNPLACED};
    use kentos_interaction::LabelSpot;
    use kentos_render_wgpu::compose::LabelBox;
    let Some(spots) = spots.filter(|_| !asks.is_empty()) else {
        return scene;
    };
    let wanted: std::collections::HashSet<&str> =
        asks.iter().map(|(_, _, id)| id.as_str()).collect();
    let mut boxes: std::collections::HashMap<String, Vec<LabelBox>> =
        std::collections::HashMap::new();
    // A curved label is its letters; a hidden or unplaced one, and its letters, is none.
    let mut skip = false;
    for (spot, grow) in spots.iter() {
        let (slot, at, turn, size) = match spot {
            LabelSpot::Placed {
                slot,
                at,
                angle,
                width,
                height,
                state,
                ..
            } => {
                skip = state & (HIDDEN | UNPLACED) != 0;
                if skip || state & CURVED != 0 {
                    continue;
                }
                (*slot, *at, *angle, [*width, *height])
            }
            LabelSpot::PlacedLetter {
                slot,
                at,
                angle,
                size,
                advance,
                ..
            } if !skip => (*slot, *at, *angle, [*advance, *size]),
            _ => continue,
        };
        let Some(e) = doc.get(slot) else {
            continue;
        };
        let layer = e.base().layer_id.as_str();
        if !wanted.contains(layer) {
            continue;
        }
        let k = grow.k;
        let c = kentos_render_wgpu::Vec2::new(
            grow.anchor.x + (at.x - grow.anchor.x) * k,
            grow.anchor.y + (at.y - grow.anchor.y) * k,
        );
        boxes.entry(layer.to_owned()).or_default().push(LabelBox {
            centre: [c.x - origin.x, c.y - origin.y],
            size: [(size[0] * k) as f32, (size[1] * k) as f32],
            turn: turn as f32,
        });
    }
    for (c, m, id) in asks {
        let (Some(found), Some((_, draw))) = (boxes.get(id), scene.compose.get_mut(*c)) else {
            continue;
        };
        let mut d = (**draw).clone();
        if let Some(mask) = d.masks.get_mut(*m) {
            mask.labels = Arc::new(found.clone());
        }
        *draw = Arc::new(d);
    }
    scene
}

/// How the area draws: the settings' effective `graphics.msaa` and `graphics.hiDpi` (docs/adr/0023), whether a zoom
/// while the wheel turns scales the last sharp picture and draws whole once it stops (`graphics.wheelZoomScaled`,
/// docs/adr/1068; off: every wheel step drawn whole), and the highlights' width in logical pixels
/// (`graphics.highlightWidth`, docs/adr/0195).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Graphics {
    pub samples: u32,
    pub hi_dpi: bool,
    pub zoom_scaled: bool,
    pub highlight_width: u32,
}

impl Default for Graphics {
    fn default() -> Self {
        Self {
            samples: 1,
            hi_dpi: true,
            zoom_scaled: false,
            highlight_width: 1,
        }
    }
}

/// The named highlight colours (`graphics.highlightColor`, docs/adr/0195 §1; the web's `HIGHLIGHT_COLORS`);
/// `accent` and anything else is the theme's accent.
pub fn highlight_color(key: &str, accent: Rgba8) -> Rgba8 {
    match key {
        "orange" => Rgba8::rgb(0xF5, 0xA6, 0x23),
        "red" => Rgba8::rgb(0xE5, 0x48, 0x4D),
        "green" => Rgba8::rgb(0x30, 0xA4, 0x6C),
        "cyan" => Rgba8::rgb(0x05, 0xA2, 0xC2),
        "magenta" => Rgba8::rgb(0xD6, 0x40, 0x9F),
        _ => accent,
    }
}

/// The project's line types as last made, by the drawing's session and generation (docs/adr/0313 §3).
type KeptPhasing = (
    (u64, u64, u64),
    std::sync::Arc<kentos_native_style::phases::Phasing>,
);
type KeptLineTypes = (
    (u64, u64),
    std::sync::Arc<kentos_native_style::line_types::LineTypes>,
);

/// The drawing area's state in the app.
pub struct Viewport {
    pub camera: Camera,
    pub sheet_saved_camera: Option<Camera>,
    /// The world point under the pointer; `None` off the area.
    pub cursor: Option<Vec2>,
    /// Where the area is in the window, logical pixels; empty until it reports.
    pub bounds: Rectangle,
    /// The area has reported its size (a fit can happen at once).
    sized: bool,
    /// A drawing was opened and waits for the area's size to be fitted.
    fit_pending: bool,
    /// The opened drawing's box as its store has it (a block's insert by its
    /// pieces), for the fit an opening waits to make.
    opened_extent: Option<Bounds>,
    /// Bumped when a drawing is opened: its scene is built even if its changes count matches.
    generation: u64,
    id: ViewId,
    scene: RefCell<Option<Cached>>,
    /// The highlight parts as last built, and what they were built from.
    highlight: RefCell<highlight::Highlighted>,
    /// Whether the grid is drawn (Izgara, F7; docs/adr/0080), and the
    /// grid as last built.
    pub grid_shown: bool,
    grid: RefCell<Option<Grid>>,
    status: Arc<Mutex<Status>>,
    /// The drawing's layers through the style engine, as last built (docs/adr/0090).
    styled: RefCell<crate::style::scene::StyledCache>,
    /// The project's line types as the style engine draws them, by the drawing's session and generation: made again
    /// only when the drawing changed, not every frame (docs/adr/0313 §3).
    line_types: RefCell<Option<KeptLineTypes>>,
    /// The phase view's looks (docs/adr/0346 §2) with Kat görünümü's (docs/adr/0715) by the drawing's session and
    /// generation and the level view's version.
    phasings: RefCell<Option<KeptPhasing>>,
    /// The wheel's last zoom: screen-sized symbols wait for it to settle.
    zoomed_at: Option<Instant>,
    /// The view's last move, a pan or the wheel: the labels keep their placing until it settles (docs/adr/0597).
    moved_at: Option<Instant>,
    /// A view of a split area that is not the active one, its place among them (split_view.rs, docs/adr/0285 §3,
    /// 0637 §1): its events come as `Message::OtherViewport`.
    pub(crate) other: Option<usize>,
    /// Kaydırmalı karşılaştır's view (docs/adr/0523 §3): drawn, but the pointer goes through it to the view under.
    pub(crate) passive: bool,
    /// The view's theme (docs/adr/0523 §1): a layer state of the project it shows the drawing with, the drawing
    /// untouched; none, as the drawing is.
    pub(crate) theme: Option<kentos_commands::LayerState>,
    /// The tree as the theme shows it, by the drawing's session and generation and the theme it was made with.
    themed: RefCell<Option<KeptTheme>>,
    /// The time the view shows (docs/adr/0576 §1): the slider's, or a moment of its own.
    pub(crate) time: crate::split_view::ViewTime,
    /// The walls' view a view of the split area that is not the active one draws: its own camera's detail
    /// (docs/adr/0696's 29 October addendum); none, the store's (the active view picks and snaps at it). The active
    /// view holds one only from the press that made it active to the next message, which takes it as its detail.
    pub(crate) wall_view: Option<kentos_geometry_core::geom::wall::View>,
}

type KeptTheme = (
    (u64, u64),
    kentos_commands::LayerState,
    std::sync::Arc<kentos_domain::LayerTree>,
);

/// How long after the wheel's last turn screen-sized symbols are built again (the web's).
const SYMBOLS_SETTLE: Duration = Duration::from_millis(150);

/// The grid lines on the GPU and what they were built for: the world box
/// they span (three views wide around the view then), their spacing, the
/// origin and the drawing area's ground (the web's `GridExtent`).
struct Grid {
    area: Bounds,
    spacing: (f64, f64),
    origin: Vec2,
    /// The user coordinate system its lines follow: origin and angle (docs/adr/0261 §4).
    frame: (Vec2, f64),
    canvas: Canvas,
    part: Arc<ScenePart>,
}

/// How far apart minor grid lines stay on screen, at least (the web's `minPx`).
const GRID_MIN_PX: f64 = 14.0;

/// The grid's minor and major lines on a ground (the web's
/// `--canvas-grid-minor` and `--canvas-grid-major`): a trace of the ink.
fn grid_colors(canvas: Canvas) -> [Rgba8; 2] {
    let ink = match canvas {
        Canvas::Paper => Rgba8::rgb(0, 0, 0),
        _ => Rgba8::rgb(0xff, 0xff, 0xff),
    };
    [
        Rgba8([ink.0[0], ink.0[1], ink.0[2], 0x0a]),
        Rgba8([ink.0[0], ink.0[1], ink.0[2], 0x17]),
    ]
}

/// The scene as last built, and what it was built from.
struct Cached {
    generation: u64,
    changes: u64,
    canvas: Canvas,
    /// Built as the plain scene (no style library given): the drawing's own parts.
    plain: bool,
    origin: Vec2,
    fixed: Arc<ScenePart>,
    curves: Arc<ScenePart>,
    /// The zoom band the curves were tessellated for.
    band: i32,
    /// Infinite lines and rays, clipped to `clip`: a box around the view
    /// when they were built.
    construction: Arc<ScenePart>,
    clip: Bounds,
}

/// The box construction lines are clipped to: the view and three times its
/// size around it, so a pan builds them again only once it leaves the box.
fn construction_clip(view: &Bounds) -> Bounds {
    let (w, h) = (view.max_x - view.min_x, view.max_y - view.min_y);
    Bounds {
        min_x: view.min_x - 3.0 * w,
        min_y: view.min_y - 3.0 * h,
        max_x: view.max_x + 3.0 * w,
        max_y: view.max_y + 3.0 * h,
    }
}

/// Whether the construction lines must be clipped again: the view left the
/// box, or zoomed in so far that the box holds needlessly long lines.
fn clip_stale(view: &Bounds, clip: &Bounds) -> bool {
    let inside = view.min_x >= clip.min_x
        && view.min_y >= clip.min_y
        && view.max_x <= clip.max_x
        && view.max_y <= clip.max_y;
    !inside || (clip.max_x - clip.min_x) > 20.0 * (view.max_x - view.min_x)
}

impl Default for Viewport {
    fn default() -> Self {
        Self::new()
    }
}

/// What the drawing area is drawn on: the theme's own background, or the
/// one chosen in Görünüm → Çizim zemini (appearance.rs), whatever the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Canvas {
    /// The graphite model space (the dark theme's).
    Slate,
    /// Near-white paper (the light theme's).
    Paper,
    /// Very dark and dimmed, for a dark room (the night theme's).
    Night,
    /// Pure black, bright lines (classic AutoCAD; the high-contrast theme's).
    Black,
}

impl From<Mode> for Canvas {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Dark => Canvas::Slate,
            Mode::Light => Canvas::Paper,
            Mode::Night => Canvas::Night,
            Mode::HighContrast => Canvas::Black,
        }
    }
}

/// The drawing's colours on a canvas: the web's canvas tokens for slate and
/// paper (DESIGN.md §3.1–3.2, `apps/web/src/styles/tokens.css`); night dims
/// them, black brightens them. One token source for web and desktop is UI-08.
pub fn palette(canvas: impl Into<Canvas>) -> Palette {
    match canvas.into() {
        Canvas::Paper => Palette {
            background: Rgba8::rgb(0xf8, 0xf9, 0xfa),
            fg: Rgba8::rgb(0x1e, 0x28, 0x33),
            fg_dim: Rgba8::rgb(0x5e, 0x6b, 0x78),
            ink: Rgba8::rgb(0x00, 0x00, 0x00),
        },
        Canvas::Slate => Palette {
            background: Rgba8::rgb(0x14, 0x1a, 0x21),
            fg: Rgba8::rgb(0xe4, 0xea, 0xf0),
            fg_dim: Rgba8::rgb(0xa3, 0xaf, 0xbc),
            ink: Rgba8::rgb(0xff, 0xff, 0xff),
        },
        Canvas::Night => Palette {
            background: Rgba8::rgb(0x0b, 0x0e, 0x13),
            fg: Rgba8::rgb(0xbc, 0xc5, 0xcf),
            fg_dim: Rgba8::rgb(0x80, 0x8b, 0x97),
            ink: Rgba8::rgb(0xdc, 0xe2, 0xe8),
        },
        Canvas::Black => Palette {
            background: Rgba8::rgb(0x00, 0x00, 0x00),
            fg: Rgba8::rgb(0xff, 0xff, 0xff),
            fg_dim: Rgba8::rgb(0xcf, 0xd6, 0xdd),
            ink: Rgba8::rgb(0xff, 0xff, 0xff),
        },
    }
}

/// The colours of the marks drawn over the drawing on Iced's canvas
/// (marks.rs, docs/adr/0029).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkColors {
    /// The snap marker and the crossing box: `--canvas-snap` (DESIGN.md §3.1–3.2, §8).
    pub snap: iced::Color,
    /// The window box: the web's blue (`drawSelectionBox`, the dark theme's
    /// `--c-info`), in both themes as on the web.
    pub window: iced::Color,
    /// The halo around the snap marker's name, and the grips' edge: the area's colour.
    pub halo: iced::Color,
    /// The grip being moved: the drawing's ink (the web's `pal.fg`).
    pub fg: iced::Color,
}

pub fn mark_colors(canvas: impl Into<Canvas>) -> MarkColors {
    let canvas = canvas.into();
    let rgb = |c: Rgba8| iced::Color::from_rgb8(c.0[0], c.0[1], c.0[2]);
    let window = iced::Color::from_rgb8(0x6d, 0xb3, 0xf2);
    match canvas {
        Canvas::Paper => MarkColors {
            snap: iced::Color::from_rgb8(0x1a, 0x9a, 0x48),
            window,
            halo: rgb(palette(canvas).background),
            fg: rgb(palette(canvas).fg),
        },
        Canvas::Slate | Canvas::Night | Canvas::Black => MarkColors {
            snap: iced::Color::from_rgb8(0x6f, 0xd0, 0x8c),
            window,
            halo: rgb(palette(canvas).background),
            fg: rgb(palette(canvas).fg),
        },
    }
}

/// The shader widget's program: a frame's inputs, and the pointer's gestures.
struct Program {
    id: ViewId,
    document: u64,
    /// The grid, the drawing's parts and the highlights, bottom first.
    parts: [Arc<ScenePart>; 6],
    origin: Vec2,
    camera: Camera,
    settings: RenderSettings,
    status: Arc<Mutex<Status>>,
    /// The pointer's look the running tool asks for over the area.
    cursor: Cursor,
    /// The drawing's styled layers and where their images come from (docs/adr/0090).
    styled: StyledScene,
    images: Option<Arc<crate::style::images::Images>>,
    /// When the wheel's zoom settles and screen-sized symbols are to be built again.
    settle_at: Option<Instant>,
    /// The view is being panned or zoomed (docs/adr/1067): the kept picture may be moved by a pan's whole pixels.
    moving: bool,
    /// What the picture shows apart from the camera (`FrameInput::drawing`, docs/adr/1068).
    drawing: Option<u64>,
    /// The inactive view of a split area (docs/adr/0285 §3).
    other: Option<usize>,
    /// Kaydırmalı karşılaştır's view (docs/adr/0523 §3): its size told, the pointer let through.
    passive: bool,
}

impl Program {
    /// The area's event as the app takes it: the active view's, or another view's of a split area.
    fn message(&self, event: Event) -> Message {
        match self.other {
            Some(i) => Message::OtherViewport(i, event),
            None => Message::Viewport(event),
        }
    }
}

/// What the widget remembers between events.
#[derive(Debug, Default)]
pub struct Gesture {
    bounds: Option<Rectangle>,
    document: Option<u64>,
    /// Event position and Iced's cursor for that delivery. Native batches may
    /// share their final cursor; buttons must follow the preceding move.
    pointer: Option<(Point, mouse::Cursor)>,
    /// The pointer's last position while the middle button drags.
    pan: Option<Point>,
    inside: bool,
    last_middle: Option<(Instant, Point)>,
    /// The right button went down over the area: when, where, and whether
    /// holding it has been told (a menu opened).
    right: Option<(Instant, Point, bool)>,
    /// The left button went down over the area and is still down.
    left: bool,
}

impl Gesture {
    fn release_capture(&mut self) {
        self.pointer = None;
        self.pan = None;
        self.inside = false;
        self.last_middle = None;
        self.right = None;
        self.left = false;
    }
}

impl shader::Program<Message> for Program {
    type State = Gesture;
    type Primitive = Frame;

    fn update(
        &self,
        state: &mut Gesture,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<shader::Action<Message>> {
        if state.document != Some(self.document) {
            state.release_capture();
            state.document = Some(self.document);
        }
        // Do this before a held-button redraw can publish a late right menu.
        if matches!(event, iced::Event::Window(iced::window::Event::Unfocused)) {
            state.release_capture();
            return None;
        }
        // A new frame ends the preceding native event batch. A later press
        // without a move must use Iced's fresh cursor, not an old event.
        if matches!(
            event,
            iced::Event::Window(iced::window::Event::RedrawRequested(_))
        ) {
            state.pointer = None;
        }
        // A new size first: a view waiting to settle after the wheel would else keep its old size (and draw its
        // scene stretched to the new one) until it settled (docs/adr/0637 §1).
        if state.bounds != Some(bounds) {
            state.bounds = Some(bounds);
            return Some(shader::Action::publish(self.message(Event::Resized(bounds))));
        }
        // A held right button: told once its time is up, else a frame then.
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event
            && let Some(hold) = right_hold(state, *now)
        {
            return Some(match hold {
                Hold::Held(at) => shader::Action::publish(self.message(Event::RightHeld(at))),
                Hold::Wait(until) => shader::Action::request_redraw_at(until),
            });
        }
        // Screen-sized symbols wait for the wheel to stop, then are built again (docs/adr/0090).
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event
            && let Some(at) = self.settle_at
        {
            return Some(if *now >= at {
                shader::Action::publish(self.message(Event::Settled))
            } else {
                shader::Action::request_redraw_at(at)
            });
        }
        // The styled atlas left images for another frame (docs/adr/0090).
        if let iced::Event::Window(iced::window::Event::RedrawRequested(_)) = event
            && self
                .status
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .images_pending
        {
            return Some(shader::Action::request_redraw());
        }
        let (event, capture) = gesture(state, event, bounds, cursor, Instant::now())?;
        // Kaydırmalı karşılaştır's view tells its size and nothing else: the view under takes the pointer.
        if self.passive {
            return match event {
                Some(e @ Event::Resized(_)) => Some(shader::Action::publish(self.message(e))),
                _ => None,
            };
        }
        let action = match event {
            Some(event) => shader::Action::publish(self.message(event)),
            None => shader::Action::request_redraw(),
        };
        Some(if capture {
            action.and_capture()
        } else {
            action
        })
    }

    fn draw(&self, _state: &Gesture, _cursor: mouse::Cursor, _bounds: Rectangle) -> Frame {
        Frame {
            id: self.id,
            parts: self.parts.clone(),
            origin: self.origin,
            camera: self.camera,
            settings: self.settings,
            status: self.status.clone(),
            styled: self.styled.clone(),
            images: self.images.clone(),
            lens: false,
            moving: self.moving,
            drawing: self.drawing,
        }
    }

    fn mouse_interaction(
        &self,
        state: &Gesture,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.pan.is_some() {
            mouse::Interaction::Grabbing
        } else if self.passive {
            mouse::Interaction::default()
        } else if self.other.is_some() && cursor.is_over(bounds) {
            // The other view of a split area: a click makes it the active one.
            mouse::Interaction::Pointer
        } else if cursor.is_over(bounds) {
            match self.cursor {
                // The crosshair is drawn over the drawing (marks.rs), as the
                // web hides its pointer on the canvas (`drawCrosshair`).
                Cursor::Cross | Cursor::Pick => mouse::Interaction::Hidden,
                // Kaydır: the open hand, as the web's `grab` (docs/adr/0056).
                Cursor::Grab => mouse::Interaction::Grab,
            }
        } else {
            mouse::Interaction::default()
        }
    }
}

/// A right press still down at a frame: its time is up, or when it will be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hold {
    Held(Point),
    Wait(Instant),
}

/// At a frame (`now`), what a right press still down means: the hold's time
/// is up (told once) or when to look again; nothing without one.
pub fn right_hold(state: &mut Gesture, now: Instant) -> Option<Hold> {
    let (pressed, at, told) = state.right.as_mut()?;
    if *told {
        return None;
    }
    let until = *pressed + RIGHT_HOLD;
    if now >= until {
        *told = true;
        Some(Hold::Held(*at))
    } else {
        Some(Hold::Wait(until))
    }
}

/// What a pointer event means for the view: an event for the app (if any)
/// and whether the event is used up. `now` dates middle and right presses.
pub fn gesture(
    state: &mut Gesture,
    event: &iced::Event,
    bounds: Rectangle,
    cursor: mouse::Cursor,
    now: Instant,
) -> Option<(Option<Event>, bool)> {
    if matches!(event, iced::Event::Window(iced::window::Event::Unfocused)) {
        state.release_capture();
        return None;
    }
    if state.bounds != Some(bounds) {
        state.bounds = Some(bounds);
        state.pointer = None;
        return Some((Some(Event::Resized(bounds)), false));
    }
    let iced::Event::Mouse(event) = event else {
        return None;
    };
    if let mouse::Event::CursorMoved { position } = event {
        state.pointer = Some((*position, cursor));
    }
    // Keep Iced's availability/overlay gate. Only substitute the position
    // while its delivery cursor is unchanged; a fresh cursor supersedes it.
    let cursor = match state.pointer {
        Some((position, observed)) if observed == cursor => match cursor {
            mouse::Cursor::Available(_) => mouse::Cursor::Available(position),
            mouse::Cursor::Levitating(_) => mouse::Cursor::Levitating(position),
            mouse::Cursor::Unavailable => mouse::Cursor::Unavailable,
        },
        _ => cursor,
    };
    match event {
        mouse::Event::CursorMoved { position } => {
            let at = Point::new(position.x - bounds.x, position.y - bounds.y);
            if let Some(last) = state.pan {
                // A drag goes on past the area's edge.
                state.pan = Some(at);
                return Some((Some(Event::Panned { by: at - last, at }), true));
            }
            // A left press that began over the area follows the pointer over the
            // command strip above it too (command_bar.rs; the web captures the pointer).
            let cursor = if state.left { cursor.land() } else { cursor };
            if cursor.position_in(bounds).is_some() {
                state.inside = true;
                Some((Some(Event::Moved(at)), false))
            } else if state.inside {
                state.inside = false;
                Some((Some(Event::Left), false))
            } else {
                None
            }
        }
        mouse::Event::CursorLeft => {
            state.pointer = None;
            let was = std::mem::take(&mut state.inside);
            was.then_some((Some(Event::Left), false))
        }
        mouse::Event::ButtonPressed(mouse::Button::Middle) => {
            let at = cursor.position_in(bounds)?;
            let double = state.last_middle.is_some_and(|(then, there)| {
                now.duration_since(then) <= DOUBLE_CLICK
                    && there.distance(at) <= DOUBLE_CLICK_DISTANCE
            });
            if double {
                state.last_middle = None;
                state.pan = None;
                return Some((Some(Event::Extents), true));
            }
            state.last_middle = Some((now, at));
            state.pan = Some(at);
            Some((None, true))
        }
        mouse::Event::ButtonReleased(mouse::Button::Middle) => {
            state.pan.take().map(|_| (None, true))
        }
        mouse::Event::ButtonPressed(mouse::Button::Left) => {
            let at = cursor.position_in(bounds)?;
            state.left = true;
            Some((Some(Event::Pressed(at)), true))
        }
        mouse::Event::ButtonReleased(mouse::Button::Left) => {
            // Released anywhere, the command strip above the area included: the press
            // began over the area (the web captures the pointer).
            if !std::mem::take(&mut state.left) {
                return None;
            }
            let position = cursor.land().position()?;
            let at = Point::new(position.x - bounds.x, position.y - bounds.y);
            Some((Some(Event::Released(at)), true))
        }
        mouse::Event::ButtonPressed(mouse::Button::Right) => {
            let at = cursor.position_in(bounds)?;
            state.right = Some((now, at, false));
            Some((Some(Event::RightPressed(at)), true))
        }
        mouse::Event::ButtonReleased(mouse::Button::Right) => {
            // Released anywhere, the command strip above the area included: the press
            // began over the area (the web captures the pointer).
            let (pressed, pressed_at, told) = state.right.take()?;
            let position = cursor.land().position()?;
            let at = Point::new(position.x - bounds.x, position.y - bounds.y);
            let quick = now.saturating_duration_since(pressed) < RIGHT_HOLD;
            let event = match (told, quick) {
                (true, _) => None,
                (false, true) => Some(Event::RightClick(at)),
                // Held long enough without a frame to tell it: its menu opens now.
                (false, false) => Some(Event::RightHeld(pressed_at)),
            };
            Some((event, true))
        }
        mouse::Event::WheelScrolled { delta } => {
            let at = cursor.position_in(bounds)?;
            let steps = wheel_steps(delta);
            let factor = steps.exp();
            (steps != 0.0 && factor.is_finite())
                .then_some((Some(Event::Zoomed { factor, at }), true))
        }
        _ => None,
    }
}

/// A wheel's turn as zoom steps: the zoom is their exponential (Genel bakış zooms by them too).
pub(crate) fn wheel_steps(delta: &mouse::ScrollDelta) -> f64 {
    match delta {
        mouse::ScrollDelta::Lines { y, .. } => f64::from(*y) * WHEEL_LINE,
        mouse::ScrollDelta::Pixels { y, .. } => f64::from(*y) * WHEEL_PIXEL,
    }
}

/// One frame of the drawing area, handed to the renderer.
pub struct Frame {
    id: ViewId,
    parts: [Arc<ScenePart>; 6],
    origin: Vec2,
    camera: Camera,
    settings: RenderSettings,
    status: Arc<Mutex<Status>>,
    styled: StyledScene,
    images: Option<Arc<crate::style::images::Images>>,
    /// Büyüteç's window (docs/adr/0181 §5): the area `id`'s scene through
    /// `camera`. One primitive type for both, since Iced keeps a renderer
    /// per primitive type and the window draws with the area's.
    lens: bool,
    /// The view is being panned or zoomed (`FrameInput::moving`, docs/adr/1067).
    moving: bool,
    /// What the picture shows apart from the camera (`FrameInput::drawing`, docs/adr/1068); none in Büyüteç's window.
    drawing: Option<u64>,
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame")
            .field("id", &self.id)
            .field("parts", &self.parts.each_ref().map(|p| p.id))
            .field("camera", &self.camera)
            .finish_non_exhaustive()
    }
}

impl shader::Primitive for Frame {
    type Pipeline = Pipeline;

    fn prepare(
        &self,
        pipeline: &mut Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        let scale = viewport.scale_factor();
        // Büyüteç's window: the area's scene through its camera, after the area's own frame.
        if self.lens {
            if let Ok(renderer) = &mut pipeline.0 {
                renderer.prepare_lens(
                    device,
                    queue,
                    self.id,
                    &kentos_render_wgpu::LensInput {
                        camera: &self.camera,
                        origin: self.origin,
                        size_px: [bounds.width * scale, bounds.height * scale],
                        scale_factor: f64::from(scale),
                        settings: &self.settings,
                    },
                );
            }
            return;
        }
        let status = match &mut pipeline.0 {
            Ok(renderer) => {
                let parts = self.parts.each_ref().map(|p| &**p);
                let prepared = renderer.prepare(
                    device,
                    queue,
                    self.id,
                    &parts,
                    &FrameInput {
                        camera: &self.camera,
                        origin: self.origin,
                        size_px: [bounds.width * scale, bounds.height * scale],
                        origin_px: [bounds.x * scale, bounds.y * scale],
                        scale_factor: f64::from(scale),
                        settings: &self.settings,
                        // Iced draws the window again for any event; the drawing is
                        // drawn again only when it changed. The hovered object (the
                        // last part) goes over it (kentos-render-wgpu, renderer.rs).
                        keep_picture: true,
                        overlays: 1,
                        moving: self.moving,
                        drawing: self.drawing,
                    },
                );
                // The styled layers after the plain parts of the same frame (docs/adr/0090).
                let images_pending = match &self.images {
                    Some(images) => {
                        renderer.prepare_styled(device, queue, self.id, &self.styled, &**images)
                    }
                    None => renderer.prepare_styled(
                        device,
                        queue,
                        self.id,
                        &self.styled,
                        &kentos_render_wgpu::styled::NoImages,
                    ),
                };
                Status {
                    stats: renderer.stats(self.id).unwrap_or_default(),
                    error: prepared
                        .err()
                        .or_else(|| renderer.error(self.id).cloned())
                        .map(|e| e.to_string()),
                    supported: renderer.sample_counts().to_vec(),
                    failure: renderer.sample_failure(self.id).cloned(),
                    images_pending,
                    scale_factor: scale,
                }
            }
            Err(error) => Status {
                error: Some(error.to_string()),
                ..Status::default()
            },
        };
        *self.status.lock().unwrap_or_else(PoisonError::into_inner) = status;
    }

    /// Single-sampled at full resolution the area draws in Iced's pass; with
    /// its own targets (MSAA, HiDPI off) it asks for `render` instead.
    fn draw(&self, pipeline: &Pipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        match &pipeline.0 {
            // The magnifier's window is always drawn in Iced's pass: its viewport is the window.
            Ok(renderer) if self.lens => {
                renderer.draw_lens(render_pass, self.id);
                true
            }
            Ok(renderer) if renderer.owns_targets(self.id) => false,
            Ok(renderer) => {
                renderer.draw(render_pass, self.id);
                true
            }
            Err(_) => true,
        }
    }

    /// The area's own pass, then its picture composed into Iced's frame inside the area.
    fn render(
        &self,
        pipeline: &Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        if let Ok(renderer) = &pipeline.0 {
            renderer.render(
                encoder,
                target,
                [
                    clip_bounds.x,
                    clip_bounds.y,
                    clip_bounds.width,
                    clip_bounds.height,
                ],
                self.id,
            );
        }
    }
}

/// Büyüteç's window (docs/adr/0181 §5): the drawing area `main`'s scene
/// through a second camera.
struct Lens {
    main: ViewId,
    origin: Vec2,
    camera: Camera,
    settings: RenderSettings,
}

impl shader::Program<Message> for Lens {
    type State = ();
    type Primitive = Frame;

    fn draw(&self, _state: &(), _cursor: mouse::Cursor, _bounds: Rectangle) -> Frame {
        Frame {
            id: self.main,
            parts: std::array::from_fn(|_| Arc::default()),
            origin: self.origin,
            camera: self.camera,
            settings: self.settings,
            status: Arc::default(),
            styled: StyledScene::default(),
            images: None,
            lens: true,
            moving: false,
            drawing: None,
        }
    }
}

/// The renderer in Iced's primitive storage: one per window's device, made
/// with the first frame. If its pipelines cannot be built, the reason is kept
/// and shown instead of the drawing, and written to standard error for the
/// service log (`make desktop` → .run/desktop.log).
pub struct Pipeline(Result<Renderer, RenderError>);

impl Pipeline {
    /// The renderer, when its pipelines could be built: the symbol pictures
    /// (style/thumbs.rs) draw with the drawing's pipelines and atlas.
    pub(crate) fn renderer(&self) -> Option<&Renderer> {
        self.0.as_ref().ok()
    }

    pub(crate) fn renderer_mut(&mut self) -> Option<&mut Renderer> {
        self.0.as_mut().ok()
    }
}

impl shader::Pipeline for Pipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let renderer = Renderer::new(device, format);
        if let Err(error) = &renderer {
            eprintln!("kentos-cad: çizim alanının wgpu hattı kurulamadı ({format:?}): {error}");
        }
        Self(renderer)
    }

    fn trim(&mut self) {
        if let Ok(renderer) = &mut self.0 {
            renderer.trim();
        }
    }
}

#[cfg(test)]
mod tests;

mod state;
