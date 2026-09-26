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
use kentos_domain::Slot;
use kentos_interaction::Selection;
use kentos_render_wgpu::camera::FIT_PADDING;
use kentos_render_wgpu::scene::{self, Highlight, lod};
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
// document in place, with no copy, and is rebuilt when its revision changes.

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

/// Changes with every change to what the drawing holds (kentos-domain, docs/adr/0020).
fn revision(doc: &Document) -> u64 {
    doc.model.revision()
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
/// A right press released sooner is a click (Enter); held longer it would
/// open the command menu (the web's `RIGHT_HOLD_MS`; no menu on the desktop yet).
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
    /// Bumped when a drawing is opened: its scene is built even if the revision matches.
    generation: u64,
    id: ViewId,
    scene: RefCell<Option<Cached>>,
    /// The highlight parts as last built, and what they were built from.
    highlight: RefCell<Highlighted>,
    status: Arc<Mutex<Status>>,
}

/// The highlight parts and what they were built from.
#[derive(Default)]
struct Highlighted {
    selected: Option<(HighlightKey, Arc<ScenePart>)>,
    hovered: Option<(HighlightKey, Arc<ScenePart>)>,
}

/// What a highlight part depends on. The selection's part does not depend
/// on the hovered object: a pointer moving over a large selection's objects
/// never builds it again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HighlightKey {
    generation: u64,
    revision: u64,
    selection: u64,
    /// The hovered object's changes; 0 in the selection's key.
    hover: u64,
    accent: Rgba8,
    /// The scene's layer count: the highlights draw after them.
    below: usize,
    /// The curves' tolerance (its bits): curves are highlighted as finely as drawn.
    tolerance: u64,
}

/// The selection's fill of closed areas, and the hovered object's alpha (the web's).
const SELECTION_FILL: f64 = 0.13;
const HOVER_ALPHA: f64 = 0.85;
/// Point marks of a highlight, logical pixels (the web's `pointStyle: { size: 15, shape: 'ring' }`).
const HIGHLIGHT_MARK: f32 = 15.0;

/// The scene as last built, and what it was built from.
struct Cached {
    generation: u64,
    revision: u64,
    mode: Mode,
    origin: Vec2,
    fixed: Arc<ScenePart>,
    curves: Arc<ScenePart>,
    /// The zoom band the curves were tessellated for.
    band: i32,
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
            highlight: RefCell::new(Highlighted::default()),
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
            Event::Pressed(at) | Event::RightClick(at) => self.cursor = Some(self.world(at)),
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
    /// selection and the hovered object highlighted in `accent` (docs/adr/0029).
    /// A change of either value reaches the next frame: new targets on the
    /// same device, nothing reopened (TODOS.md AA-02).
    pub fn view<'a>(
        &'a self,
        doc: &Document,
        mode: Mode,
        graphics: Graphics,
        selection: &Selection,
        accent: Rgba8,
    ) -> Element<'a, Message> {
        let palette = palette(mode);
        let settings = RenderSettings {
            samples: graphics.samples,
            hi_dpi: graphics.hi_dpi,
            ..RenderSettings::new(palette.background)
        };
        let (origin, fixed, curves) = self.scene(doc, mode, &palette, &settings);
        let (selected, hovered) = self.highlights(doc, selection, accent, &fixed, &curves);
        let area: Element<'a, Message> = shader(Program {
            id: self.id,
            parts: [fixed, curves, selected, hovered],
            origin,
            camera: self.camera,
            settings,
            status: self.status.clone(),
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
        mode: Mode,
        palette: &Palette,
        settings: &RenderSettings,
    ) -> (Vec2, Arc<ScenePart>, Arc<ScenePart>) {
        let needed = lod::band(self.camera.scale, settings.curve_tolerance_px);
        let band = lod::build_band(self.camera.scale, settings.curve_tolerance_px);
        let budget = settings.curve_segment_budget;
        let revision = revision(doc);
        let mut cache = self.scene.borrow_mut();
        let current = cache.as_ref().is_some_and(|c| {
            c.generation == self.generation && c.revision == revision && c.mode == mode
        });
        if !current {
            let origin = scene::scene_origin(doc);
            *cache = Some(Cached {
                generation: self.generation,
                revision,
                mode,
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
            });
        } else if let Some(cached) = cache.as_mut()
            && lod::stale(cached.band, needed)
        {
            cached.curves = Arc::new(scene::build_curves(
                doc,
                palette,
                cached.origin,
                lod::tolerance(band),
                budget,
            ));
            cached.band = band;
        }
        match cache.as_ref() {
            Some(c) => (c.origin, c.fixed.clone(), c.curves.clone()),
            None => (Vec2::default(), Arc::default(), Arc::default()),
        }
    }

    /// The highlight parts (docs/adr/0029): the selection in the accent with
    /// a 13 % fill of closed areas and 15 px rings on points, and the hovered
    /// object, unless selected, in the accent at 85 % without fill (the
    /// web's `uploadHighlight`). They are wgpu scene parts drawn after every
    /// layer, built only when the selection, the hovered object, the drawing,
    /// the accent or the curves' band change; a pan or a zoom within the band
    /// draws them from the GPU as it is (measured: ADR 0029).
    fn highlights(
        &self,
        doc: &Document,
        selection: &Selection,
        accent: Rgba8,
        fixed: &ScenePart,
        curves: &ScenePart,
    ) -> (Arc<ScenePart>, Arc<ScenePart>) {
        let key = HighlightKey {
            generation: self.generation,
            revision: revision(doc),
            selection: selection.version(),
            hover: 0,
            accent,
            below: fixed.layers.len(),
            tolerance: curves.tolerance.to_bits(),
        };
        let ring = kentos_render_wgpu::layout::marker_shape::RING;
        let build = |ids: &mut dyn Iterator<Item = Slot>, style: Highlight| {
            let objects = ids.filter_map(|slot| doc.model.get(slot));
            Arc::new(scene::build_highlight(
                objects,
                &style,
                fixed.origin,
                curves.tolerance,
                key.below,
            ))
        };
        let mut cache = self.highlight.borrow_mut();
        let selected = match &cache.selected {
            Some((known, part)) if *known == key => part.clone(),
            _ => {
                let part = build(
                    &mut selection.ids().iter().copied(),
                    Highlight {
                        color: accent,
                        fill: Some(accent.with_alpha(SELECTION_FILL)),
                        mark_size: HIGHLIGHT_MARK,
                        mark_shape: ring,
                    },
                );
                cache.selected = Some((key, part.clone()));
                part
            }
        };
        let hover_key = HighlightKey {
            hover: selection.hover_version(),
            ..key
        };
        let hovered = match &cache.hovered {
            Some((known, part)) if *known == hover_key => part.clone(),
            _ => {
                // A selected object is not hovered over its own highlight.
                let part = build(
                    &mut selection
                        .hover()
                        .filter(|h| !selection.contains(*h))
                        .into_iter(),
                    Highlight {
                        color: accent.with_alpha(HOVER_ALPHA),
                        fill: None,
                        mark_size: HIGHLIGHT_MARK,
                        mark_shape: ring,
                    },
                );
                cache.hovered = Some((hover_key, part.clone()));
                part
            }
        };
        (selected, hovered)
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

/// The drawing's colours for the theme: the web's canvas tokens (DESIGN.md
/// §3.1–3.2, `apps/web/src/styles/tokens.css`). One token source for web and
/// desktop is UI-08.
pub fn palette(mode: Mode) -> Palette {
    match mode {
        Mode::Light => Palette {
            background: Rgba8::rgb(0xf8, 0xf9, 0xfa),
            fg: Rgba8::rgb(0x1e, 0x28, 0x33),
            fg_dim: Rgba8::rgb(0x5e, 0x6b, 0x78),
            ink: Rgba8::rgb(0x00, 0x00, 0x00),
        },
        Mode::Dark | Mode::Night | Mode::HighContrast => Palette {
            background: Rgba8::rgb(0x14, 0x1a, 0x21),
            fg: Rgba8::rgb(0xe4, 0xea, 0xf0),
            fg_dim: Rgba8::rgb(0xa3, 0xaf, 0xbc),
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

pub fn mark_colors(mode: Mode) -> MarkColors {
    let rgb = |c: Rgba8| iced::Color::from_rgb8(c.0[0], c.0[1], c.0[2]);
    let window = iced::Color::from_rgb8(0x6d, 0xb3, 0xf2);
    match mode {
        Mode::Light => MarkColors {
            snap: iced::Color::from_rgb8(0x1a, 0x9a, 0x48),
            window,
            halo: rgb(palette(mode).background),
        },
        Mode::Dark | Mode::Night | Mode::HighContrast => MarkColors {
            snap: iced::Color::from_rgb8(0x6f, 0xd0, 0x8c),
            window,
            halo: rgb(palette(mode).background),
        },
    }
}

/// The shader widget's program: a frame's inputs, and the pointer's gestures.
struct Program {
    id: ViewId,
    parts: [Arc<ScenePart>; 4],
    origin: Vec2,
    camera: Camera,
    settings: RenderSettings,
    status: Arc<Mutex<Status>>,
}

/// What the widget remembers between events.
#[derive(Debug, Default)]
pub struct Gesture {
    bounds: Option<Rectangle>,
    /// The pointer's last position while the middle button drags.
    pan: Option<Point>,
    inside: bool,
    last_middle: Option<(Instant, Point)>,
    /// When the right button went down over the area.
    right: Option<Instant>,
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
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
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
            // Released anywhere: the press began over the area (the web captures the pointer).
            if !std::mem::take(&mut state.left) {
                return None;
            }
            let position = cursor.position()?;
            let at = Point::new(position.x - bounds.x, position.y - bounds.y);
            Some((Some(Event::Released(at)), true))
        }
        mouse::Event::ButtonPressed(mouse::Button::Right) => {
            cursor.position_in(bounds)?;
            state.right = Some(now);
            Some((None, true))
        }
        mouse::Event::ButtonReleased(mouse::Button::Right) => {
            // Released anywhere: the press began over the area (the web captures the pointer).
            let pressed = state.right.take()?;
            let position = cursor.position()?;
            let at = Point::new(position.x - bounds.x, position.y - bounds.y);
            let quick = now.saturating_duration_since(pressed) < RIGHT_HOLD;
            Some((quick.then_some(Event::RightClick(at)), true))
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
    parts: [Arc<ScenePart>; 4],
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

    /// The highlights are scene parts built when what they show changes; a
    /// pointer moving over objects builds the hovered one's part only, never
    /// the selection's again (docs/adr/0029).
    #[test]
    fn a_hover_builds_only_the_hover_highlight() {
        let doc = sample();
        let viewport = Viewport::new();
        let palette = palette(Mode::Dark);
        let settings = RenderSettings::new(palette.background);
        let (_, fixed, curves) = viewport.scene(&doc, Mode::Dark, &palette, &settings);
        let accent = Rgba8::rgb(0x4c, 0x9b, 0xe8);
        let mut selection = Selection::new();
        let ids: Vec<Slot> = doc.model.entities().map(|e| Slot(e.base().id)).collect();
        selection.set(ids.iter().copied().take(ids.len() - 1));
        let (sel, hover) = viewport.highlights(&doc, &selection, accent, &fixed, &curves);
        assert!(!sel.segments.is_empty() && hover.segments.is_empty());
        selection.set_hover(ids.last().copied());
        let (sel2, hover2) = viewport.highlights(&doc, &selection, accent, &fixed, &curves);
        assert_eq!(sel2.id, sel.id, "the selection's part is kept");
        assert_ne!(hover2.id, hover.id);
        let (sel3, hover3) = viewport.highlights(&doc, &selection, accent, &fixed, &curves);
        assert_eq!((sel3.id, hover3.id), (sel.id, hover2.id), "nothing changed");
        // A selected object is not hovered over its own highlight.
        selection.set_hover(ids.first().copied());
        let (_, hover4) = viewport.highlights(&doc, &selection, accent, &fixed, &curves);
        assert!(hover4.segments.is_empty() && hover4.fills.is_empty());
        // Its colour and layer: after every layer of the scene.
        assert!(sel.segments.iter().all(|s| s.color == accent.0));
        assert_eq!(sel.layers.len(), fixed.layers.len() + 1);
    }

    /// Where the selection's highlight is drawn, measured (docs/adr/0029): as
    /// a wgpu scene part, built once per selection and drawn from the GPU
    /// every frame, against Iced's canvas, whose paths (dashed, as the web
    /// draws the selection) are tessellated on the CPU in every frame the view
    /// moves. A parcel map of `side²` parcels and `side` roads, all selected,
    /// the whole map on a 1400 × 800 area. Run by hand, in release:
    /// `cargo test --release -p kentos-desktop highlight_costs -- --ignored --nocapture`.
    #[test]
    #[ignore = "a measurement: run by hand in release, with a GPU"]
    fn highlight_costs_on_a_large_drawing() {
        use iced::advanced::renderer::Headless;
        use iced::widget::canvas::{self, LineDash, Path, Stroke};
        use kentos_contracts::{EntityBase, LineEntity, PathEntity, Vec2 as Wire};
        use std::time::Instant;

        let Some(renderer) = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            kentos_ui::theme::typography::ui(),
            iced::Pixels(16.0),
            Some("wgpu"),
        )) else {
            println!("wgpu çizicisi kurulamadı: ölçüm yapılmadı");
            return;
        };
        let base = |id: u32, layer: &str| EntityBase {
            id,
            layer_id: layer.into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
        };
        for side in [32usize, 100, 316] {
            let sample = sample();
            let layer = sample
                .model
                .layers()
                .leaves()
                .first()
                .map(|l| l.id.clone())
                .expect("a layer");
            let mut snapshot = sample.model.to_snapshot();
            snapshot.entities.clear();
            let (e, n) = (487000.0, 4420000.0);
            let mut id = 1;
            for row in 0..side {
                for col in 0..side {
                    let (x, y) = (e + col as f64 * 20.0, n + row as f64 * 20.0);
                    let pts = [
                        (x, y),
                        (x + 18.0, y),
                        (x + 18.5, y + 9.0),
                        (x + 17.0, y + 18.0),
                        (x, y + 17.5),
                    ]
                    .map(|(x, y)| Wire { x, y })
                    .to_vec();
                    snapshot.entities.push(Entity::Polygon(PathEntity {
                        base: base(id, &layer),
                        pts,
                        bulges: None,
                        holes: None,
                    }));
                    id += 1;
                }
                let y = n + row as f64 * 20.0 + 19.0;
                snapshot.entities.push(Entity::Line(LineEntity {
                    base: base(id, &layer),
                    a: Wire { x: e, y },
                    b: Wire {
                        x: e + side as f64 * 20.0,
                        y,
                    },
                }));
                id += 1;
            }
            let doc = Document::new(snapshot, None).expect("opens");
            let objects = doc.entity_count();
            let mut viewport = Viewport::new();
            viewport.update(
                Event::Resized(Rectangle::new(
                    Point::ORIGIN,
                    iced::Size::new(1400.0, 800.0),
                )),
                Some(&doc),
            );
            viewport.opened(&doc);
            viewport.update(Event::Extents, Some(&doc));
            let palette = palette(Mode::Dark);
            let settings = RenderSettings::new(palette.background);
            let t = Instant::now();
            let (_, fixed, curves) = viewport.scene(&doc, Mode::Dark, &palette, &settings);
            let scene_time = t.elapsed();
            let mut selection = Selection::new();
            selection.set(doc.model.entities().map(|e| Slot(e.base().id)));
            let accent = Rgba8::rgb(0x4c, 0x9b, 0xe8);
            let t = Instant::now();
            let (part, _) = viewport.highlights(&doc, &selection, accent, &fixed, &curves);
            let part_time = t.elapsed();
            let t = Instant::now();
            let _ = viewport.highlights(&doc, &selection, accent, &fixed, &curves);
            let cached = t.elapsed();

            // The same highlight on Iced's canvas: every frame the view moves.
            let camera = viewport.camera;
            let dashed = Stroke {
                line_dash: LineDash {
                    segments: &[6.0, 3.0],
                    offset: 0,
                },
                ..Stroke::default()
                    .with_color(iced::Color::from_rgb8(0x4c, 0x9b, 0xe8))
                    .with_width(1.0)
            };
            let fill = iced::Color::from_rgba8(0x4c, 0x9b, 0xe8, 0.13);
            let screen = |p: &Wire| {
                let [x, y] = camera.world_to_screen(Vec2::new(p.x, p.y));
                Point::new(x as f32, y as f32)
            };
            let frames = 3;
            let t = Instant::now();
            for _ in 0..frames {
                let mut frame = canvas::Frame::new(&renderer, iced::Size::new(1400.0, 800.0));
                for entity in doc.model.entities() {
                    match entity {
                        Entity::Polygon(p) => {
                            let path = Path::new(|b| {
                                b.move_to(screen(&p.pts[0]));
                                for q in &p.pts[1..] {
                                    b.line_to(screen(q));
                                }
                                b.close();
                            });
                            frame.fill(&path, fill);
                            frame.stroke(&path, dashed);
                        }
                        Entity::Line(l) => {
                            frame.stroke(&Path::line(screen(&l.a), screen(&l.b)), dashed);
                        }
                        _ => {}
                    }
                }
                std::hint::black_box(frame.into_geometry());
            }
            let canvas_frame = t.elapsed() / frames;
            println!(
                "{objects} seçili nesne: sahne {:.1} ms; wgpu vurgu parçası bir kez {:.1} ms ({} KB, {} çizgi parçası, {} dolgu üçgeni), \
                 önbellekten {:.3} ms, kare başına CPU 0; Iced canvas'ı kare başına {:.1} ms",
                scene_time.as_secs_f64() * 1e3,
                part_time.as_secs_f64() * 1e3,
                part.byte_size() / 1024,
                part.segments.len(),
                part.fills.len() / 3,
                cached.as_secs_f64() * 1e3,
                canvas_frame.as_secs_f64() * 1e3,
            );
        }
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
            let (_, fixed, curves) = app.viewport.scene(doc, mode, palette, &settings);
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
