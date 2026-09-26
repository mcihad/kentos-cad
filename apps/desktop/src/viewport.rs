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

mod highlight;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use iced::time::Instant;
use iced::widget::{container, shader, stack};
use iced::{Element, Fill, Point, Rectangle, Vector, mouse, wgpu};

use kentos_contracts::{DrawingFont, Entity, LayerNode};
use kentos_interaction::{Cursor, Selection, ViewChange};
use kentos_render_wgpu::camera::FIT_PADDING;
use kentos_render_wgpu::scene::{self, lod};
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

    fn anchor(&self) -> kentos_contracts::Vec2 {
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
}

/// How the area draws: the settings' effective `graphics.msaa` and `graphics.hiDpi` (docs/adr/0023).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Graphics {
    pub samples: u32,
    pub hi_dpi: bool,
}

impl Default for Graphics {
    fn default() -> Self {
        Self {
            samples: 1,
            hi_dpi: true,
        }
    }
}

/// The drawing area's state in the app.
pub struct Viewport {
    pub camera: Camera,
    /// The world point under the pointer; `None` off the area.
    pub cursor: Option<Vec2>,
    /// Where the area is in the window, logical pixels; empty until it reports.
    pub bounds: Rectangle,
    /// The area has reported its size (a fit can happen at once).
    sized: bool,
    /// A drawing was opened and waits for the area's size to be fitted.
    fit_pending: bool,
    /// Bumped when a drawing is opened: its scene is built even if its changes count matches.
    generation: u64,
    id: ViewId,
    scene: RefCell<Option<Cached>>,
    /// The highlight parts as last built, and what they were built from.
    highlight: RefCell<highlight::Highlighted>,
    status: Arc<Mutex<Status>>,
}

/// The scene as last built, and what it was built from.
struct Cached {
    generation: u64,
    changes: u64,
    canvas: Canvas,
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

impl Viewport {
    pub fn new() -> Self {
        Self {
            camera: Camera::default(),
            cursor: None,
            bounds: Rectangle::default(),
            sized: false,
            fit_pending: false,
            generation: 0,
            id: NEXT_VIEW.fetch_add(1, Ordering::Relaxed),
            scene: RefCell::new(None),
            highlight: RefCell::new(highlight::Highlighted::default()),
            status: Arc::new(Mutex::new(Status::default())),
        }
    }

    /// A drawing was opened: its scene is built afresh and the view goes to
    /// its start view, else to its extents (the web's `replaceDrawing`).
    pub fn opened(&mut self, doc: &Document) {
        self.generation += 1;
        self.cursor = None;
        self.fit_pending = true;
        if self.sized {
            self.fit(doc);
        }
    }

    pub fn update(&mut self, event: Event, doc: Option<&Document>) {
        match event {
            Event::Resized(bounds) => {
                self.bounds = bounds;
                self.camera
                    .set_size(f64::from(bounds.width), f64::from(bounds.height));
                self.sized = true;
                if self.fit_pending
                    && let Some(doc) = doc
                {
                    self.fit(doc);
                }
            }
            Event::Moved(at) => self.cursor = Some(self.world(at)),
            Event::Left => self.cursor = None,
            Event::Panned { by, at } => {
                self.camera.pan_by(f64::from(by.x), f64::from(by.y));
                self.cursor = Some(self.world(at));
            }
            Event::Zoomed { factor, at } => {
                self.camera
                    .zoom_at(factor, f64::from(at.x), f64::from(at.y));
                self.cursor = Some(self.world(at));
            }
            Event::Extents => {
                if let Some(extents) = doc.and_then(scene::extents) {
                    self.camera.fit(&extents, FIT_PADDING);
                }
            }
            Event::CenterOn(p) => self.camera.center_on(p),
            // The app gives these to the tool session (app.rs); the pointer is there too.
            Event::Pressed(at)
            | Event::RightPressed(at)
            | Event::RightHeld(at)
            | Event::RightClick(at) => {
                self.cursor = Some(self.world(at));
            }
            // It may come from off the area; the last move placed the pointer.
            Event::Released(_) => {}
        }
    }

    /// What the last frame drew, or why it could not.
    pub fn status(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Tests: what a frame would report about the device (no GPU in unit tests).
    #[cfg(test)]
    pub fn set_status_for_tests(&self, supported: Vec<u32>, failure: Option<SampleFailure>) {
        let mut status = self.status.lock().unwrap_or_else(PoisonError::into_inner);
        status.supported = supported;
        status.failure = failure;
    }

    /// Objects on shown layers the area does not draw yet, by kind (text,
    /// dimensions, construction lines): said, not hidden.
    pub fn not_drawn(&self) -> BTreeMap<&'static str, usize> {
        self.scene
            .try_borrow()
            .ok()
            .and_then(|cache| cache.as_ref().map(|c| c.fixed.not_drawn.clone()))
            .unwrap_or_default()
    }

    /// The drawing area showing `doc`, drawn as `graphics` says, with the
    /// selection and the hovered object highlighted in `accent` (docs/adr/0029)
    /// and the pointer looking as the running tool asks (docs/adr/0056).
    /// A change of either value reaches the next frame: new targets on the
    /// same device, nothing reopened (TODOS.md AA-02).
    pub fn view<'a>(
        &'a self,
        doc: &Document,
        canvas: impl Into<Canvas>,
        graphics: Graphics,
        selection: &Selection,
        accent: Rgba8,
        cursor: Cursor,
    ) -> Element<'a, Message> {
        let canvas = canvas.into();
        let palette = palette(canvas);
        let settings = RenderSettings {
            samples: graphics.samples,
            hi_dpi: graphics.hi_dpi,
            ..RenderSettings::new(palette.background)
        };
        let (origin, fixed, curves, construction, clip) =
            self.scene(doc, canvas, &palette, &settings);
        let (selected, hovered) = self.highlights(doc, selection, accent, &fixed, &curves, &clip);
        let area: Element<'a, Message> = shader(Program {
            id: self.id,
            parts: [fixed, curves, construction, selected, hovered],
            origin,
            camera: self.camera,
            settings,
            status: self.status.clone(),
            cursor,
        })
        .width(Fill)
        .height(Fill)
        .into();
        match self.status().error {
            None => area,
            Some(error) => stack![
                area,
                container(EmptyState::error("Çizim alanı çizilemedi").description(format!(
                    "KentOS'un wgpu çizim hattı: {error}. Çizim, katmanlar ve kayıt etkilenmedi. \
                     Ekran kartı sürücüsünü güncelleyin ya da WGPU_BACKEND=gl ile başka bir \
                     sürücü yolu deneyin."
                )))
                .center(Fill)
            ]
            .into(),
        }
    }

    /// The scene of `doc`, from the cache when nothing it depends on changed.
    fn scene(
        &self,
        doc: &Document,
        canvas: impl Into<Canvas>,
        palette: &Palette,
        settings: &RenderSettings,
    ) -> (Vec2, Arc<ScenePart>, Arc<ScenePart>, Arc<ScenePart>, Bounds) {
        let canvas = canvas.into();
        let view = self.camera.visible_bounds();
        let needed = lod::band(self.camera.scale, settings.curve_tolerance_px);
        let band = lod::build_band(self.camera.scale, settings.curve_tolerance_px);
        let budget = settings.curve_segment_budget;
        let changes = changes(doc);
        let mut cache = self.scene.borrow_mut();
        let current = cache.as_ref().is_some_and(|c| {
            c.generation == self.generation && c.changes == changes && c.canvas == canvas
        });
        if !current {
            let origin = scene::scene_origin(doc);
            let clip = construction_clip(&view);
            *cache = Some(Cached {
                generation: self.generation,
                changes,
                canvas,
                origin,
                fixed: Arc::new(scene::build_fixed(doc, palette, origin)),
                curves: Arc::new(scene::build_curves(
                    doc,
                    palette,
                    origin,
                    lod::tolerance(band),
                    budget,
                )),
                band,
                construction: Arc::new(scene::build_construction(doc, palette, origin, &clip)),
                clip,
            });
        } else if let Some(cached) = cache.as_mut() {
            if lod::stale(cached.band, needed) {
                cached.curves = Arc::new(scene::build_curves(
                    doc,
                    palette,
                    cached.origin,
                    lod::tolerance(band),
                    budget,
                ));
                cached.band = band;
            }
            if clip_stale(&view, &cached.clip) {
                cached.clip = construction_clip(&view);
                cached.construction = Arc::new(scene::build_construction(
                    doc,
                    palette,
                    cached.origin,
                    &cached.clip,
                ));
            }
        }
        match cache.as_ref() {
            Some(c) => (
                c.origin,
                c.fixed.clone(),
                c.curves.clone(),
                c.construction.clone(),
                c.clip,
            ),
            None => (
                Vec2::default(),
                Arc::default(),
                Arc::default(),
                Arc::default(),
                construction_clip(&view),
            ),
        }
    }

    /// Shows `b` as large as it fits, with the margin a fit keeps.
    pub fn show(&mut self, b: &Bounds) {
        self.camera.fit(b, FIT_PADDING);
    }

    /// A view change a tool asked for (Kaydır, Pencere yakınlaştır;
    /// docs/adr/0056). The pointer stays where it is on the area, so the
    /// world point under it follows the view.
    pub fn change(&mut self, change: ViewChange) {
        let at = self.cursor.map(|c| self.camera.world_to_screen(c));
        match change {
            ViewChange::Pan { dx, dy } => self.camera.pan_by(dx, dy),
            ViewChange::Fit { bounds, padding } => self.camera.fit(&bounds, padding),
            // The app opens the text field (text_field.rs); the camera stays.
            ViewChange::Text(_) => {}
        }
        self.cursor = at.map(|[x, y]| self.camera.screen_to_world(x, y));
    }

    /// The document's start view, else its extents; its origin when it has neither.
    fn fit(&mut self, doc: &Document) {
        self.fit_pending = false;
        match start_view(doc).or_else(|| scene::extents(doc)) {
            Some(b) => self.camera.fit(&b, FIT_PADDING),
            None => self.camera.center_on(scene::scene_origin(doc)),
        }
    }

    /// The world point at a position of the area, through the float64 camera.
    pub fn world(&self, at: Point) -> Vec2 {
        self.camera
            .screen_to_world(f64::from(at.x), f64::from(at.y))
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
    /// The halo around the snap marker's name: the area's colour.
    pub halo: iced::Color,
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
        },
        Canvas::Slate | Canvas::Night | Canvas::Black => MarkColors {
            snap: iced::Color::from_rgb8(0x6f, 0xd0, 0x8c),
            window,
            halo: rgb(palette(canvas).background),
        },
    }
}

/// The shader widget's program: a frame's inputs, and the pointer's gestures.
struct Program {
    id: ViewId,
    parts: [Arc<ScenePart>; 5],
    origin: Vec2,
    camera: Camera,
    settings: RenderSettings,
    status: Arc<Mutex<Status>>,
    /// The pointer's look the running tool asks for over the area.
    cursor: Cursor,
}

/// What the widget remembers between events.
#[derive(Debug, Default)]
pub struct Gesture {
    bounds: Option<Rectangle>,
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
        // A held right button: told once its time is up, else a frame then.
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event
            && let Some(hold) = right_hold(state, *now)
        {
            return Some(match hold {
                Hold::Held(at) => {
                    shader::Action::publish(Message::Viewport(Event::RightHeld(at)))
                }
                Hold::Wait(until) => shader::Action::request_redraw_at(until),
            });
        }
        let (event, capture) = gesture(state, event, bounds, cursor, Instant::now())?;
        let action = match event {
            Some(event) => shader::Action::publish(Message::Viewport(event)),
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
        } else if cursor.is_over(bounds) {
            match self.cursor {
                Cursor::Cross => mouse::Interaction::Crosshair,
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
    if state.bounds != Some(bounds) {
        state.bounds = Some(bounds);
        return Some((Some(Event::Resized(bounds)), false));
    }
    let iced::Event::Mouse(event) = event else {
        return None;
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
            let steps = match delta {
                mouse::ScrollDelta::Lines { y, .. } => f64::from(*y) * WHEEL_LINE,
                mouse::ScrollDelta::Pixels { y, .. } => f64::from(*y) * WHEEL_PIXEL,
            };
            let factor = steps.exp();
            (steps != 0.0 && factor.is_finite())
                .then_some((Some(Event::Zoomed { factor, at }), true))
        }
        _ => None,
    }
}

/// One frame of the drawing area, handed to the renderer.
pub struct Frame {
    id: ViewId,
    parts: [Arc<ScenePart>; 5],
    origin: Vec2,
    camera: Camera,
    settings: RenderSettings,
    status: Arc<Mutex<Status>>,
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
                    },
                );
                Status {
                    stats: renderer.stats(self.id).unwrap_or_default(),
                    error: prepared
                        .err()
                        .or_else(|| renderer.error(self.id).cloned())
                        .map(|e| e.to_string()),
                    supported: renderer.sample_counts().to_vec(),
                    failure: renderer.sample_failure(self.id).cloned(),
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

/// The renderer in Iced's primitive storage: one per window's device, made
/// with the first frame. If its pipelines cannot be built, the reason is kept
/// and shown instead of the drawing, and written to standard error for the
/// service log (`make desktop` → .run/desktop.log).
pub struct Pipeline(Result<Renderer, RenderError>);

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
mod tests {
    use super::*;
    use iced::Size;

    const AREA: Rectangle = Rectangle {
        x: 100.0,
        y: 50.0,
        width: 800.0,
        height: 600.0,
    };

    fn over(x: f32, y: f32) -> mouse::Cursor {
        mouse::Cursor::Available(Point::new(x, y))
    }

    fn sized() -> Gesture {
        let mut state = Gesture::default();
        let first = gesture(
            &mut state,
            &iced::Event::Mouse(mouse::Event::CursorLeft),
            AREA,
            mouse::Cursor::Unavailable,
            Instant::now(),
        );
        assert_eq!(first, Some((Some(Event::Resized(AREA)), false)));
        state
    }

    fn mouse_event(
        state: &mut Gesture,
        event: mouse::Event,
        cursor: mouse::Cursor,
    ) -> Option<(Option<Event>, bool)> {
        gesture(
            state,
            &iced::Event::Mouse(event),
            AREA,
            cursor,
            Instant::now(),
        )
    }

    #[test]
    fn the_area_reports_its_size_first_and_the_pointer_in_its_own_pixels() {
        let mut state = sized();
        let moved = mouse_event(
            &mut state,
            mouse::Event::CursorMoved {
                position: Point::new(150.0, 70.0),
            },
            over(150.0, 70.0),
        );
        assert_eq!(
            moved,
            Some((Some(Event::Moved(Point::new(50.0, 20.0))), false))
        );
        let left = mouse_event(
            &mut state,
            mouse::Event::CursorMoved {
                position: Point::new(10.0, 10.0),
            },
            over(10.0, 10.0),
        );
        assert_eq!(left, Some((Some(Event::Left), false)));
    }

    /// The left button's release reaches the app wherever it happens, once
    /// it went down over the area (a selection box ends off the area too);
    /// a release that began elsewhere is not the area's.
    #[test]
    fn the_left_release_follows_a_press_over_the_area() {
        let mut state = sized();
        let release = mouse::Event::ButtonReleased(mouse::Button::Left);
        assert_eq!(mouse_event(&mut state, release, over(150.0, 70.0)), None);
        let press = mouse_event(
            &mut state,
            mouse::Event::ButtonPressed(mouse::Button::Left),
            over(150.0, 70.0),
        );
        assert_eq!(
            press,
            Some((Some(Event::Pressed(Point::new(50.0, 20.0))), true))
        );
        assert_eq!(
            mouse_event(&mut state, release, over(1000.0, 20.0)),
            Some((Some(Event::Released(Point::new(900.0, -30.0))), true))
        );
        assert_eq!(mouse_event(&mut state, release, over(150.0, 70.0)), None);
    }

    #[test]
    fn the_middle_button_drags_the_view_even_past_the_edge() {
        let mut state = sized();
        let press = mouse_event(
            &mut state,
            mouse::Event::ButtonPressed(mouse::Button::Middle),
            over(200.0, 200.0),
        );
        assert_eq!(press, Some((None, true)));
        let drag = mouse_event(
            &mut state,
            mouse::Event::CursorMoved {
                position: Point::new(1000.0, 180.0),
            },
            over(1000.0, 180.0),
        );
        assert_eq!(
            drag,
            Some((
                Some(Event::Panned {
                    by: Vector::new(800.0, -20.0),
                    at: Point::new(900.0, 130.0)
                }),
                true
            ))
        );
        let release = mouse_event(
            &mut state,
            mouse::Event::ButtonReleased(mouse::Button::Middle),
            over(1000.0, 180.0),
        );
        assert_eq!(release, Some((None, true)));
        assert!(state.pan.is_none());
    }

    /// The command strip over the area (command_bar.rs) levitates the
    /// pointer: a press that began over the area still moves and ends there,
    /// as the web's captured pointer does; with no press, the pointer has
    /// left the area.
    #[test]
    fn a_press_goes_on_and_ends_over_the_strip_above_the_area() {
        let mut state = sized();
        let press = mouse_event(
            &mut state,
            mouse::Event::ButtonPressed(mouse::Button::Left),
            over(500.0, 300.0),
        );
        assert_eq!(
            press,
            Some((Some(Event::Pressed(Point::new(400.0, 250.0))), true))
        );
        let strip = mouse::Cursor::Levitating(Point::new(500.0, 70.0));
        let moved = mouse_event(
            &mut state,
            mouse::Event::CursorMoved {
                position: Point::new(500.0, 70.0),
            },
            strip,
        );
        assert_eq!(
            moved,
            Some((Some(Event::Moved(Point::new(400.0, 20.0))), false))
        );
        let released = mouse_event(
            &mut state,
            mouse::Event::ButtonReleased(mouse::Button::Left),
            strip,
        );
        assert_eq!(
            released,
            Some((Some(Event::Released(Point::new(400.0, 20.0))), true))
        );
        // A quick right press in the area let go over the strip is still Enter.
        let _ = mouse_event(
            &mut state,
            mouse::Event::ButtonPressed(mouse::Button::Right),
            over(500.0, 300.0),
        );
        let enter = mouse_event(
            &mut state,
            mouse::Event::ButtonReleased(mouse::Button::Right),
            strip,
        );
        assert_eq!(
            enter,
            Some((Some(Event::RightClick(Point::new(400.0, 20.0))), true))
        );
        // With no button down, over the strip is off the area.
        let left = mouse_event(
            &mut state,
            mouse::Event::CursorMoved {
                position: Point::new(500.0, 72.0),
            },
            strip,
        );
        assert_eq!(left, Some((Some(Event::Left), false)));
    }

    #[test]
    fn a_held_right_button_is_told_once_and_its_release_is_no_click() {
        let mut state = sized();
        let now = Instant::now();
        let at = Point::new(300.0, 200.0);
        state.right = Some((now, at, false));
        // Before its time: a frame then.
        assert_eq!(right_hold(&mut state, now), Some(Hold::Wait(now + RIGHT_HOLD)));
        // Its time up: told once.
        assert_eq!(right_hold(&mut state, now + RIGHT_HOLD), Some(Hold::Held(at)));
        assert_eq!(right_hold(&mut state, now + RIGHT_HOLD * 2), None);
        let released = gesture(
            &mut state,
            &iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
            AREA,
            mouse::Cursor::Available(Point::new(300.0, 200.0)),
            now + RIGHT_HOLD * 2,
        );
        assert_eq!(released, Some((None, true)), "no Enter after a menu");
        // Held long enough with no frame to tell it: the release opens the menu.
        state.right = Some((now, at, false));
        let released = gesture(
            &mut state,
            &iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
            AREA,
            mouse::Cursor::Available(Point::new(310.0, 205.0)),
            now + RIGHT_HOLD * 2,
        );
        assert_eq!(released, Some((Some(Event::RightHeld(at)), true)));
    }

    #[test]
    fn a_middle_double_click_shows_everything() {
        let mut state = sized();
        let now = Instant::now();
        let press = |state: &mut Gesture, at: Instant| {
            gesture(
                state,
                &iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)),
                AREA,
                over(300.0, 300.0),
                at,
            )
        };
        assert_eq!(press(&mut state, now), Some((None, true)));
        let _ = gesture(
            &mut state,
            &iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)),
            AREA,
            over(300.0, 300.0),
            now,
        );
        assert_eq!(
            press(&mut state, now + Duration::from_millis(200)),
            Some((Some(Event::Extents), true))
        );
        // Slow presses are two drags.
        assert_eq!(
            press(&mut state, now + Duration::from_secs(3)),
            Some((None, true))
        );
    }

    #[test]
    fn the_wheel_zooms_at_the_pointer_and_only_over_the_area() {
        let mut state = sized();
        let notch = mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
        };
        let Some((Some(Event::Zoomed { factor, at }), true)) =
            mouse_event(&mut state, notch, over(500.0, 350.0))
        else {
            panic!("a notch zooms");
        };
        assert!((factor - 0.15f64.exp()).abs() < 1e-12);
        assert_eq!(at, Point::new(400.0, 300.0));
        assert_eq!(mouse_event(&mut state, notch, over(20.0, 20.0)), None);
        let back = mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -100.0 },
        };
        let Some((Some(Event::Zoomed { factor, .. }), true)) =
            mouse_event(&mut state, back, over(500.0, 350.0))
        else {
            panic!("a touchpad zooms");
        };
        assert!((factor - (-0.15f64).exp()).abs() < 1e-12);
    }

    fn sample() -> Document {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/document/v1/sample.json");
        Document::read(&path).expect("the sample reads")
    }

    #[test]
    fn opening_fits_the_start_view_once_the_area_has_a_size() {
        let doc = sample();
        let mut viewport = Viewport::new();
        viewport.opened(&doc);
        assert_eq!(viewport.camera, Camera::default(), "no size yet: no fit");
        viewport.update(
            Event::Resized(Rectangle::new(Point::ORIGIN, Size::new(1000.0, 800.0))),
            Some(&doc),
        );
        // The sample's start view: 70 × 80 m around the parcel.
        let home = start_view(&doc).expect("the sample has a start view");
        let visible = viewport.camera.visible_bounds();
        assert!(visible.min_x <= home.min_x && visible.max_x >= home.max_x);
        assert!(visible.min_y <= home.min_y && visible.max_y >= home.max_y);
        assert!((viewport.camera.center.x - (home.min_x + home.max_x) / 2.0).abs() < 1e-9);

        viewport.update(Event::Extents, Some(&doc));
        let extents = scene::extents(&doc).expect("extents");
        assert!((viewport.camera.center.y - (extents.min_y + extents.max_y) / 2.0).abs() < 1e-9);

        // The pointer's world point follows pan and zoom.
        viewport.update(Event::Moved(Point::new(500.0, 400.0)), Some(&doc));
        let middle = viewport.cursor.expect("over the area");
        assert!((middle.x - viewport.camera.center.x).abs() < 1e-9);
        viewport.update(
            Event::Zoomed {
                factor: 2.0,
                at: Point::new(500.0, 400.0),
            },
            Some(&doc),
        );
        let after = viewport.cursor.expect("still over the area");
        assert!((after.x - middle.x).abs() < 1e-9 && (after.y - middle.y).abs() < 1e-9);
        viewport.update(Event::Left, Some(&doc));
        assert_eq!(viewport.cursor, None);
    }

    #[test]
    fn the_scene_is_built_again_only_when_its_inputs_change() {
        use crate::app::App;
        // Through the app's own messages, as the window drives it.
        let (mut app, _) = App::boot(None);
        let _ = app.update(Message::Opened(Some(Ok(Box::new(sample())))));
        let _ = app.update(Message::Viewport(Event::Resized(Rectangle::new(
            Point::ORIGIN,
            Size::new(1000.0, 800.0),
        ))));
        let palette = palette(Mode::Dark);
        let settings = RenderSettings::new(palette.background);
        let scene = |app: &App, mode: Mode, palette: &Palette| {
            let doc = app.document.as_ref().expect("the sample is open");
            let (_, fixed, curves, _, _) = app.viewport.scene(doc, mode, palette, &settings);
            (fixed, curves)
        };
        let (fixed, curves) = scene(&app, Mode::Dark, &palette);
        // A pan leaves the scene as it is.
        let _ = app.update(Message::Viewport(Event::Panned {
            by: Vector::new(30.0, 10.0),
            at: Point::new(1.0, 1.0),
        }));
        let (same_fixed, same_curves) = scene(&app, Mode::Dark, &palette);
        assert_eq!((fixed.id, curves.id), (same_fixed.id, same_curves.id));
        // Zooming in 8× leaves the curves' band: only the curves are built again.
        let _ = app.update(Message::Viewport(Event::Zoomed {
            factor: 8.0,
            at: Point::new(500.0, 400.0),
        }));
        let (zoomed_fixed, zoomed_curves) = scene(&app, Mode::Dark, &palette);
        assert_eq!(zoomed_fixed.id, fixed.id);
        assert_ne!(zoomed_curves.id, curves.id);
        assert!(zoomed_curves.segments.len() > curves.segments.len());
        // A change to the drawing (the Kadastro group hidden) builds both.
        let _ = app.update(Message::LayerVisible("layer-g".into()));
        let (changed, _) = scene(&app, Mode::Dark, &palette);
        assert_ne!(changed.id, fixed.id);
        assert!(
            changed.segments.is_empty(),
            "every shown layer is in the hidden group"
        );
        // So does the theme: token colours resolve anew.
        let light = super::palette(Mode::Light);
        let (relit, _) = scene(&app, Mode::Light, &light);
        assert_ne!(relit.id, changed.id);
    }
}
