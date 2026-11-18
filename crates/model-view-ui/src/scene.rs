//! The 3D scene on screen (docs/adr/0910 §4–§6, docs/adr/8073 §2): source meshes cut into tiles with their own f64
//! origins (`kentos_scene3d::chunk`), selected each frame by the screen-space error and drawn by the shared renderer
//! (`kentos_render_scene3d`) through Iced's shader widget, a clicked point ringed over it. Every 3D view draws with it:
//! the building's and the road's (the application's 3B görünüm) and the IFC viewer's. It knows no host: a drag, a right
//! drag (or Shift), the wheel and a click come back as [`Gesture`]s through the host's function. Each view draws at the
//! quality the host asks (docs/adr/8120): into its own targets, four samples a pixel, its bodies' edges and, at the
//! highest, the ambient light's occlusion, then laid over the window.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use iced::widget::{canvas, shader, stack};
use iced::{Color, Element, Fill, Rectangle, Theme, mouse, wgpu};
pub use kentos_render_scene3d::Quality;
use kentos_render_scene3d::{Frame as Scene3dFrame, Scene3dRenderer, Style};
use kentos_scene3d::{Aabb, Camera, Chunked, Content, Exaggeration, Limits, Settings, SourceMesh, Viewport, chunk, select};
use kentos_ui::theme::Tokens;

/// Refined above 16 pixels of error: the usual default of tiled 3D streaming.
pub const MAX_SCREEN_ERROR: f64 = 16.0;

/// A source mesh the scene is cut from: its part (its style's index), what it is (said when a click meets it), the
/// object it draws (its key: a click chooses it), its vertices (f64, the source's) and triangles, whether it is shaded
/// smooth, and whether a click goes through it (a space's pale volume: met only when nothing behind it is).
#[derive(Debug, Clone)]
pub struct Source {
    pub part: u32,
    pub name: String,
    pub owner: Option<String>,
    pub vertices: Arc<Vec<[f64; 3]>>,
    pub triangles: Vec<[u32; 3]>,
    pub smooth: bool,
    pub through: bool,
}

/// The scene cut into tiles, its bounds and what it is made of.
pub struct Scene {
    chunked: Chunked,
    first: u64,
    bounds: Aabb,
    sources: usize,
    triangles: usize,
    meshes: Vec<Source>,
}

/// Each scene's contents take their own ids (a block of 2³² from here): a renderer keeps what it uploaded by id, so a
/// scene made again (a toggle, a selection) never draws the one before's tiles.
static NEXT_CONTENT: AtomicU64 = AtomicU64::new(1);

impl fmt::Debug for Scene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scene").field("tiles", &self.chunked.hierarchy.tiles.len()).finish_non_exhaustive()
    }
}

impl Scene {
    /// The scene of `sources`, which show `count` things.
    pub fn of(sources: Vec<Source>, count: usize) -> Scene {
        let first = NEXT_CONTENT.fetch_add(1 << 32, Ordering::Relaxed);
        let chunked = {
            let meshes: Vec<SourceMesh> = sources
                .iter()
                .map(|s| SourceMesh { part: s.part, vertices: &s.vertices, triangles: &s.triangles, smooth: s.smooth })
                .collect();
            chunk(&meshes, Limits::default(), first)
        };
        let bounds = chunked.hierarchy.roots.first().map_or(Aabb::EMPTY, |&r| chunked.hierarchy.tiles[r as usize].bounds);
        let triangles = sources.iter().map(|s| s.triangles.len()).sum();
        Scene { chunked, first, bounds, sources: count, triangles, meshes: sources }
    }

    /// Its box (the root tile's).
    pub fn bounds(&self) -> Aabb {
        self.bounds
    }

    /// How many things it shows.
    pub fn sources(&self) -> usize {
        self.sources
    }

    /// Its triangles, as its sources have them.
    pub fn triangles(&self) -> usize {
        self.triangles
    }

    /// How many tile contents it was cut into.
    pub fn contents(&self) -> usize {
        self.chunked.contents.len()
    }

    /// The source meshes it was cut from.
    pub fn meshes(&self) -> &[Source] {
        &self.meshes
    }

    /// The parts its tiles carry.
    pub fn parts(&self) -> Vec<u32> {
        let mut out: Vec<u32> = self.chunked.contents.iter().flat_map(|c| c.ranges.iter().map(|r| r.0)).collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The point a ray (in the shown space) meets first among the tiles `select` draws for `camera`: the source's
    /// coordinates, its height real, what it is on and its object. A click goes through a source made to be seen
    /// through to what is behind it; it meets that source only when nothing else is there.
    pub fn pick(&self, camera: &Camera, vp: &Viewport, ex: &Exaggeration, x: f64, y: f64) -> Option<Picked> {
        self.pick_cut(camera, vp, ex, x, y, &[])
    }

    /// As [`Scene::pick`] in a scene a section cuts (`planes`, in the shown space): only the part of the ray the
    /// planes keep meets anything, so a click never meets what is cut away.
    pub fn pick_cut(&self, camera: &Camera, vp: &Viewport, ex: &Exaggeration, x: f64, y: f64, planes: &[[f64; 4]]) -> Option<Picked> {
        let drawn = select(&self.chunked.hierarchy, camera, vp, ex, Settings { max_screen_error: MAX_SCREEN_ERROR }, |_| true).draw;
        let (o, d) = camera.ray(x, y, vp);
        let (t0, t1) = kentos_model_view::section::kept(planes, o, d)?;
        // From where the kept part begins; a hit past its end is cut away.
        let o = [o[0] + d[0] * t0, o[1] + d[1] * t0, o[2] + d[2] * t0];
        let meet = |through: bool| {
            kentos_scene3d::pick(
                &self.chunked.hierarchy,
                &drawn,
                |id| self.content(id),
                |m, t| {
                    let s = self.meshes.get(m as usize).filter(|s| through || !s.through)?;
                    let tri = s.triangles.get(t as usize)?;
                    Some([s.vertices.get(tri[0] as usize).copied()?, s.vertices.get(tri[1] as usize).copied()?, s.vertices.get(tri[2] as usize).copied()?])
                },
                o,
                d,
                ex,
            )
        };
        let within = |h: &kentos_scene3d::Hit| t0 + h.distance <= t1;
        let hit = meet(false).filter(within).or_else(|| meet(true).filter(within))?;
        let s = &self.meshes[hit.mesh as usize];
        Some(Picked {
            point: hit.point,
            what: s.name.clone(),
            owner: s.owner.clone(),
            mesh: hit.mesh as usize,
            triangle: hit.triangle as usize,
        })
    }

    fn content(&self, id: u64) -> Option<&Content> {
        self.chunked.contents.get(id.checked_sub(self.first)? as usize)
    }
}

/// A point picked in the scene: the source's coordinates (height real), what it is on and its object's key.
#[derive(Debug, Clone, PartialEq)]
pub struct Picked {
    pub point: [f64; 3],
    pub what: String,
    pub owner: Option<String>,
    /// The source mesh met (its index in the scene's) and its triangle: what a measurement snaps on (docs/adr/8121).
    pub mesh: usize,
    pub triangle: usize,
}

/// What the last frame drew: tiles, triangles, contents waiting, bytes on the GPU.
#[derive(Debug, Clone, Copy, Default)]
pub struct Seen {
    pub tiles: usize,
    pub triangles: u64,
    pub waiting: usize,
    pub resident: u64,
}

/// What the pointer did over the scene: a drag (orbit), a right drag or a drag with Shift (pan), the wheel (dolly), and
/// a click at a place of the view (logical pixels from its top left) of a size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gesture {
    Orbit(f32, f32),
    Pan(f32, f32),
    Dolly(f32),
    Pick(f32, f32, f32, f32),
    /// The view's size (logical pixels) at its first frame and whenever it changes: a host fits the model by its shape.
    Size(f32, f32),
    /// Where the pointer is over the view (none: it left it) of a size, while the host asks (measuring, docs/adr/8121).
    Hover(Option<(f32, f32)>, f32, f32),
}

/// What the scene draws over itself besides the clicked point (docs/adr/8121): a measurement's lines, points and
/// values, in the source's coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum Mark {
    /// A measured line; faint: a component along an axis, an outline, the line to the pointer.
    Line { a: [f64; 3], b: [f64; 3], faint: bool },
    /// A point measured; with a snap, the pointer's: its symbol (a square at a corner, a triangle at an edge's middle,
    /// a diamond on an edge, a ring on a face) and its name beside it.
    Point { at: [f64; 3], snap: Option<kentos_model_view::measure::Snap> },
    /// A value on a plate at a point.
    Value { at: [f64; 3], text: String },
}

/// One view of a scene: the scene, the camera, the exaggeration, the parts' styles, towards the light, the sky's and
/// the fill's shares of the light (0: the light alone; docs/adr/8120), the quality it is drawn at, the half-spaces a
/// section keeps (in the shown space; none: nothing cut), the point ringed, and the frame's shared notes (what it drew;
/// whether contents are still waiting for the GPU).
pub struct SceneView {
    pub scene: Arc<Scene>,
    pub camera: Camera,
    pub exaggeration: Exaggeration,
    pub styles: Arc<Vec<Style>>,
    pub light: [f64; 3],
    pub sky: f32,
    pub fill: f32,
    pub quality: Quality,
    pub planes: Vec<[f64; 4]>,
    pub marked: Option<[f64; 3]>,
    /// The measurements drawn over the scene, and whether the pointer's moves come back as [`Gesture::Hover`].
    pub marks: Vec<Mark>,
    pub hover: bool,
    pub seen: Arc<Mutex<Seen>>,
    pub pending: Arc<AtomicBool>,
}

/// The scene drawn and its gestures sent through `on`.
pub fn view<'a, M: Clone + 'a>(v: SceneView, on: impl Fn(Gesture) -> M + 'a) -> Element<'a, M> {
    let marker = canvas(Marker { camera: v.camera, point: v.marked, marks: v.marks }).width(Fill).height(Fill);
    let program = Program {
        scene: v.scene,
        camera: v.camera,
        exaggeration: v.exaggeration,
        styles: v.styles,
        light: v.light,
        sky: v.sky,
        fill: v.fill,
        quality: v.quality,
        planes: Arc::new(v.planes),
        seen: v.seen,
        pending: v.pending,
        hover: v.hover,
        on: Box::new(on),
    };
    stack![shader(program).width(Fill).height(Fill), marker].into()
}

// ── The widget: Iced's shader over the shared renderer ────────────────────────────────────────────────────────────

struct Program<'a, M> {
    scene: Arc<Scene>,
    camera: Camera,
    exaggeration: Exaggeration,
    styles: Arc<Vec<Style>>,
    light: [f64; 3],
    sky: f32,
    fill: f32,
    quality: Quality,
    planes: Arc<Vec<[f64; 4]>>,
    seen: Arc<Mutex<Seen>>,
    pending: Arc<AtomicBool>,
    hover: bool,
    on: Box<dyn Fn(Gesture) -> M + 'a>,
}

/// A drag in progress (where, whether it pans), the view's key in the pipeline, the size last told and the pointer's
/// place last told (none: outside).
pub struct State {
    view: u64,
    drag: Option<Press>,
    shift: bool,
    size: (f32, f32),
    over: Option<(f32, f32)>,
}

#[derive(Clone, Copy)]
struct Press {
    last: iced::Point,
    pan: bool,
    down: iced::Point,
    moved: bool,
}

/// A press that moves less than this (logical pixels) is a click: it picks.
const CLICK: f32 = 3.0;

static NEXT_VIEW: AtomicU64 = AtomicU64::new(1);

/// The views whose widgets are gone (their window closed, the scene left the screen): their renderers and the GPU
/// buffers they hold are let go at the next frame a view draws. A window of its own takes its pipeline with it.
static RETIRED: Mutex<Vec<u64>> = Mutex::new(Vec::new());

impl Default for State {
    fn default() -> Self {
        State { view: NEXT_VIEW.fetch_add(1, Ordering::Relaxed), drag: None, shift: false, size: (0.0, 0.0), over: None }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        if let Ok(mut retired) = RETIRED.lock() {
            retired.push(self.view);
        }
    }
}

impl<M> shader::Program<M> for Program<'_, M> {
    type State = State;
    type Primitive = Frame3d;

    fn update(&self, state: &mut State, event: &iced::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<shader::Action<M>> {
        use iced::keyboard;
        match event {
            // The view's size told when it changed (the message draws the next frame); else contents still waiting for
            // the GPU: another frame.
            iced::Event::Window(iced::window::Event::RedrawRequested(_)) => {
                let size = (bounds.width, bounds.height);
                if size != state.size && size.0 >= 1.0 && size.1 >= 1.0 {
                    state.size = size;
                    return Some(shader::Action::publish((self.on)(Gesture::Size(size.0, size.1))));
                }
                self.pending.load(Ordering::Relaxed).then(shader::Action::request_redraw)
            }
            iced::Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => {
                state.shift = m.shift();
                None
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(b @ (mouse::Button::Left | mouse::Button::Right))) => {
                let at = cursor.position_in(bounds)?;
                state.drag = Some(Press { last: at, pan: *b == mouse::Button::Right || state.shift, down: at, moved: false });
                Some(shader::Action::capture())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(_)) => {
                let p = state.drag.take()?;
                if p.moved || p.pan {
                    return Some(shader::Action::capture());
                }
                Some(shader::Action::publish((self.on)(Gesture::Pick(p.down.x, p.down.y, bounds.width, bounds.height))).and_capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) if state.drag.is_none() => {
                // While the host measures: the pointer's place when it moved a pixel, and once when it left.
                if !self.hover {
                    return None;
                }
                let at = cursor.position_in(bounds).map(|p| (p.x, p.y));
                let moved = match (at, state.over) {
                    (Some(a), Some(b)) => (a.0 - b.0).abs() >= 1.0 || (a.1 - b.1).abs() >= 1.0,
                    (None, None) => false,
                    _ => true,
                };
                let _ = position;
                if !moved {
                    return None;
                }
                state.over = at;
                Some(shader::Action::publish((self.on)(Gesture::Hover(at, bounds.width, bounds.height))))
            }
            iced::Event::Mouse(mouse::Event::CursorLeft) if self.hover && state.over.is_some() => {
                state.over = None;
                Some(shader::Action::publish((self.on)(Gesture::Hover(None, bounds.width, bounds.height))))
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let mut p = state.drag?;
                let at = iced::Point::new(position.x - bounds.x, position.y - bounds.y);
                if !p.moved && (at.x - p.down.x).hypot(at.y - p.down.y) < CLICK {
                    return Some(shader::Action::capture());
                }
                p.moved = true;
                let (dx, dy) = (at.x - p.last.x, at.y - p.last.y);
                p.last = at;
                state.drag = Some(p);
                let g = if p.pan { Gesture::Pan(dx, dy) } else { Gesture::Orbit(dx, dy) };
                Some(shader::Action::publish((self.on)(g)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                cursor.position_in(bounds)?;
                let steps = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };
                Some(shader::Action::publish((self.on)(Gesture::Dolly(steps))).and_capture())
            }
            _ => None,
        }
    }

    fn draw(&self, state: &State, _cursor: mouse::Cursor, _bounds: Rectangle) -> Frame3d {
        Frame3d {
            view: state.view,
            scene: self.scene.clone(),
            camera: self.camera,
            exaggeration: self.exaggeration,
            styles: self.styles.clone(),
            light: self.light,
            sky: self.sky,
            fill: self.fill,
            quality: self.quality,
            planes: self.planes.clone(),
            seen: self.seen.clone(),
            pending: self.pending.clone(),
        }
    }

    fn mouse_interaction(&self, state: &State, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        match state.drag {
            Some(_) => mouse::Interaction::Grabbing,
            None if cursor.is_over(bounds) => mouse::Interaction::Grab,
            None => mouse::Interaction::default(),
        }
    }
}

/// One frame of a view, handed to the pipeline.
pub struct Frame3d {
    view: u64,
    scene: Arc<Scene>,
    camera: Camera,
    exaggeration: Exaggeration,
    styles: Arc<Vec<Style>>,
    light: [f64; 3],
    sky: f32,
    fill: f32,
    quality: Quality,
    planes: Arc<Vec<[f64; 4]>>,
    seen: Arc<Mutex<Seen>>,
    pending: Arc<AtomicBool>,
}

impl fmt::Debug for Frame3d {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame3d").field("view", &self.view).finish_non_exhaustive()
    }
}

/// Where a view is laid over the window: its viewport and scissor in the window's pixels.
struct Placed {
    viewport: [f32; 4],
    scissor: [u32; 4],
}

/// The views' renderers (one each, at its quality: a renderer keeps one frame's draws and the view's own targets).
pub struct Pipeline {
    format: wgpu::TextureFormat,
    renderers: HashMap<u64, Scene3dRenderer>,
    placed: HashMap<u64, Placed>,
}

impl shader::Pipeline for Pipeline {
    fn new(_device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Pipeline { format, renderers: HashMap::new(), placed: HashMap::new() }
    }
}

impl shader::Primitive for Frame3d {
    type Pipeline = Pipeline;

    fn prepare(&self, pipeline: &mut Pipeline, device: &wgpu::Device, queue: &wgpu::Queue, bounds: &Rectangle, viewport: &shader::Viewport) {
        if let Ok(mut retired) = RETIRED.lock() {
            for view in retired.drain(..) {
                pipeline.renderers.remove(&view);
                pipeline.placed.remove(&view);
            }
        }
        let size = viewport.physical_size();
        let format = pipeline.format;
        // A quality changed in the settings: the view's renderer made again at it (its contents uploaded again).
        if pipeline.renderers.get(&self.view).is_some_and(|r| r.quality() != self.quality) {
            pipeline.renderers.remove(&self.view);
        }
        let r = pipeline.renderers.entry(self.view).or_insert_with(|| Scene3dRenderer::with_quality(device, format, self.quality));
        let s = viewport.scale_factor();
        let (bx, by, bw, bh) = (bounds.x * s, bounds.y * s, bounds.width * s, bounds.height * s);
        r.ensure_targets(device, bw.round().max(1.0) as u32, bh.round().max(1.0) as u32);
        let vp = Viewport { width: f64::from(bw.max(1.0)), height: f64::from(bh.max(1.0)) };
        let h = &self.scene.chunked.hierarchy;
        let settings = Settings { max_screen_error: MAX_SCREEN_ERROR };
        // Select with what is resident, upload what it draws and asks for within the budgets, select again with what
        // is resident now, and draw that.
        let first = select(h, &self.camera, &vp, &self.exaggeration, settings, |c| r.is_resident(c.id));
        let asked: Vec<&Content> = first
            .draw
            .iter()
            .chain(&first.wanted)
            .filter_map(|&i| h.tiles[i as usize].content.and_then(|c| self.scene.content(c.id)))
            .collect();
        r.load(device, &asked);
        let now = select(h, &self.camera, &vp, &self.exaggeration, settings, |c| r.is_resident(c.id));
        let draws: Vec<&Content> =
            now.draw.iter().filter_map(|&i| h.tiles[i as usize].content.and_then(|c| self.scene.content(c.id))).collect();
        let stats = r.prepare(
            device,
            queue,
            &Scene3dFrame {
                camera: &self.camera,
                exaggeration: self.exaggeration,
                viewport: vp,
                draws: &draws,
                styles: &self.styles,
                light: self.light,
                ambient: 0.38,
                sky: self.sky,
                fill: self.fill,
                clip: &self.planes,
            },
        );
        self.pending.store(!now.wanted.is_empty(), Ordering::Relaxed);
        if let Ok(mut seen) = self.seen.lock() {
            *seen = Seen { tiles: stats.tiles, triangles: stats.triangles, waiting: now.wanted.len(), resident: stats.resident_bytes };
        }
        let (vx, vy) = (bx.max(0.0), by.max(0.0));
        let (x1, y1) = ((bx + bw).min(size.width as f32), (by + bh).min(size.height as f32));
        if x1 <= vx || y1 <= vy {
            pipeline.placed.remove(&self.view);
            return;
        }
        // The view's own targets are the whole box (the projection is the box's); wgpu keeps a viewport inside the
        // window, so a box partly off it lays the part of its targets on it.
        let (bw, bh) = (bw.max(1.0), bh.max(1.0));
        let uv = [(vx - bx) / bw, (vy - by) / bh, (x1 - bx) / bw, (y1 - by) / bh];
        r.place(queue, uv);
        pipeline.placed.insert(self.view, Placed { viewport: [vx, vy, x1 - vx, y1 - vy], scissor: [vx as u32, vy as u32, x1 as u32, y1 as u32] });
    }

    fn draw(&self, _pipeline: &Pipeline, _render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        false
    }

    fn render(&self, pipeline: &Pipeline, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, clip: &Rectangle<u32>) {
        let (Some(p), Some(r)) = (pipeline.placed.get(&self.view), pipeline.renderers.get(&self.view)) else { return };
        let x0 = p.scissor[0].max(clip.x);
        let y0 = p.scissor[1].max(clip.y);
        let x1 = p.scissor[2].min(clip.x + clip.width);
        let y1 = p.scissor[3].min(clip.y + clip.height);
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        r.render_offscreen(encoder, target, p.viewport, [x0, y0, x1, y1]);
    }
}

/// The point picked, ringed where the camera shows it, and the measurements' marks (docs/adr/8121): their lines in the
/// accent colour (faint ones thinner and dashed), their points white-ringed dots, the pointer's snap its symbol and its
/// name, their values on plates of the field's colour.
struct Marker {
    camera: Camera,
    point: Option<[f64; 3]>,
    marks: Vec<Mark>,
}

impl<M> canvas::Program<M> for Marker {
    type State = ();

    fn draw(&self, _: &(), renderer: &iced::Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        use iced::widget::canvas::{Frame, LineDash, Path, Stroke, Text};
        use kentos_model_view::measure::Snap;
        if self.point.is_none() && self.marks.is_empty() {
            return Vec::new();
        }
        let vp = Viewport { width: f64::from(bounds.width), height: f64::from(bounds.height) };
        let on_screen = |p: [f64; 3]| self.camera.project(p, &vp).map(|(x, y, _)| iced::Point::new(x as f32, y as f32));
        let mut frame = Frame::new(renderer, bounds.size());
        let t = Tokens::of(theme);
        let accent = t.accent;
        // Lines first, then points, then the values over them.
        for m in &self.marks {
            if let Mark::Line { a, b, faint } = m
                && let (Some(a), Some(b)) = (on_screen(*a), on_screen(*b))
            {
                let path = Path::line(a, b);
                if *faint {
                    let dash = Stroke { line_dash: LineDash { segments: &[5.0, 4.0], offset: 0 }, ..Stroke::default() };
                    frame.stroke(&path, dash.with_color(accent.scale_alpha(0.75)).with_width(1.2));
                } else {
                    frame.stroke(&path, Stroke::default().with_color(Color::WHITE.scale_alpha(0.85)).with_width(4.0));
                    frame.stroke(&path, Stroke::default().with_color(accent).with_width(2.0));
                }
            }
        }
        for m in &self.marks {
            let Mark::Point { at, snap } = m else { continue };
            let Some(c) = on_screen(*at) else { continue };
            match snap {
                None => {
                    frame.fill(&Path::circle(c, 4.5), Color::WHITE);
                    frame.fill(&Path::circle(c, 3.0), accent);
                }
                Some(s) => {
                    let symbol = match s {
                        Snap::Corner => Path::rectangle(iced::Point::new(c.x - 6.0, c.y - 6.0), iced::Size::new(12.0, 12.0)),
                        Snap::Middle => Path::new(|p| {
                            p.move_to(iced::Point::new(c.x, c.y - 7.0));
                            p.line_to(iced::Point::new(c.x + 7.0, c.y + 5.0));
                            p.line_to(iced::Point::new(c.x - 7.0, c.y + 5.0));
                            p.close();
                        }),
                        Snap::Edge => Path::new(|p| {
                            p.move_to(iced::Point::new(c.x, c.y - 7.0));
                            p.line_to(iced::Point::new(c.x + 7.0, c.y));
                            p.line_to(iced::Point::new(c.x, c.y + 7.0));
                            p.line_to(iced::Point::new(c.x - 7.0, c.y));
                            p.close();
                        }),
                        Snap::Face => Path::circle(c, 6.0),
                    };
                    frame.stroke(&symbol, Stroke::default().with_color(Color::WHITE.scale_alpha(0.9)).with_width(4.0));
                    frame.stroke(&symbol, Stroke::default().with_color(accent).with_width(1.8));
                    frame.fill(&Path::circle(c, 1.8), accent);
                    plate(&mut frame, s.name(), iced::Point::new(c.x + 12.0, c.y - 14.0), &t, false);
                }
            }
        }
        for m in &self.marks {
            if let Mark::Value { at, text } = m
                && let Some(c) = on_screen(*at)
            {
                plate(&mut frame, text, iced::Point::new(c.x, c.y - 14.0), &t, true);
            }
        }
        if let Some(c) = self.point.and_then(on_screen) {
            frame.stroke(&Path::circle(c, 7.0), Stroke::default().with_color(Color::WHITE).with_width(4.0));
            frame.stroke(&Path::circle(c, 7.0), Stroke::default().with_color(accent).with_width(2.0));
            frame.fill(&Path::circle(c, 2.0), accent);
        }
        let _ = Text::default;
        vec![frame.into_geometry()]
    }
}

/// Words on a plate of the field's colour, centred on `at` (a value) or starting at it (a snap's name); kept within the
/// scene, so a value at its edge is read whole.
fn plate(frame: &mut iced::widget::canvas::Frame, words: &str, at: iced::Point, t: &Tokens, centred: bool) {
    use iced::widget::canvas::{Path, Stroke, Text};
    let size = 12.0;
    // The UI font's digits and letters are about 0.56 of its size wide.
    let width = words.chars().count() as f32 * size * 0.56 + 12.0;
    let left = if centred { at.x - width / 2.0 } else { at.x };
    let left = left.clamp(4.0, (frame.width() - width - 4.0).max(4.0));
    let at = iced::Point::new(at.x, at.y.clamp(14.0, (frame.height() - 14.0).max(14.0)));
    let body = Path::rounded_rectangle(iced::Point::new(left, at.y - 10.0), iced::Size::new(width, 20.0), iced::border::Radius::from(4.0));
    frame.fill(&body, t.field.scale_alpha(0.94));
    frame.stroke(&body, Stroke::default().with_color(if centred { t.accent } else { t.border }).with_width(1.0));
    frame.fill_text(Text {
        content: words.to_owned(),
        position: iced::Point::new(left + width / 2.0, at.y),
        color: t.text,
        size: iced::Pixels(size),
        font: kentos_ui::theme::typography::ui(),
        align_x: iced::advanced::text::Alignment::Center,
        align_y: iced::alignment::Vertical::Center,
        ..Text::default()
    });
}
