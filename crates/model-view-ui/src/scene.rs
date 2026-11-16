//! The 3D scene on screen (docs/adr/0910 §4–§6, docs/adr/8073 §2): source meshes cut into tiles with their own f64
//! origins (`kentos_scene3d::chunk`), selected each frame by the screen-space error and drawn by the shared renderer
//! (`kentos_render_scene3d`) through Iced's shader widget, a clicked point ringed over it. Every 3D view draws with it:
//! the building's and the road's (the application's 3B görünüm) and the IFC viewer's. It knows no host: a drag, a right
//! drag (or Shift), the wheel and a click come back as [`Gesture`]s through the host's function.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use iced::widget::{canvas, shader, stack};
use iced::{Color, Element, Fill, Rectangle, Theme, mouse, wgpu};
use kentos_render_scene3d::{DEPTH_CLEAR, DepthTarget, Frame as Scene3dFrame, Scene3dRenderer, Style};
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
        Some(Picked { point: hit.point, what: s.name.clone(), owner: s.owner.clone() })
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
}

/// One view of a scene: the scene, the camera, the exaggeration, the parts' styles, towards the light, the half-spaces
/// a section keeps (in the shown space; none: nothing cut), the point ringed, and the frame's shared notes (what it
/// drew; whether contents are still waiting for the GPU).
pub struct SceneView {
    pub scene: Arc<Scene>,
    pub camera: Camera,
    pub exaggeration: Exaggeration,
    pub styles: Arc<Vec<Style>>,
    pub light: [f64; 3],
    pub planes: Vec<[f64; 4]>,
    pub marked: Option<[f64; 3]>,
    pub seen: Arc<Mutex<Seen>>,
    pub pending: Arc<AtomicBool>,
}

/// The scene drawn and its gestures sent through `on`.
pub fn view<'a, M: Clone + 'a>(v: SceneView, on: impl Fn(Gesture) -> M + 'a) -> Element<'a, M> {
    let marker = canvas(Marker { camera: v.camera, point: v.marked }).width(Fill).height(Fill);
    let program = Program {
        scene: v.scene,
        camera: v.camera,
        exaggeration: v.exaggeration,
        styles: v.styles,
        light: v.light,
        planes: Arc::new(v.planes),
        seen: v.seen,
        pending: v.pending,
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
    planes: Arc<Vec<[f64; 4]>>,
    seen: Arc<Mutex<Seen>>,
    pending: Arc<AtomicBool>,
    on: Box<dyn Fn(Gesture) -> M + 'a>,
}

/// A drag in progress (where, whether it pans) and the view's key in the pipeline.
pub struct State {
    view: u64,
    drag: Option<Press>,
    shift: bool,
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
        State { view: NEXT_VIEW.fetch_add(1, Ordering::Relaxed), drag: None, shift: false }
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
            // Contents still waiting for the GPU: another frame.
            iced::Event::Window(iced::window::Event::RedrawRequested(_)) => {
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
    planes: Arc<Vec<[f64; 4]>>,
    seen: Arc<Mutex<Seen>>,
    pending: Arc<AtomicBool>,
}

impl fmt::Debug for Frame3d {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame3d").field("view", &self.view).finish_non_exhaustive()
    }
}

/// Where a view draws: its viewport and scissor in the target's pixels.
struct Placed {
    viewport: [f32; 4],
    scissor: [u32; 4],
}

/// The views' renderers (one each: a renderer keeps one frame's draws) and the depth buffer they share.
pub struct Pipeline {
    format: wgpu::TextureFormat,
    renderers: HashMap<u64, Scene3dRenderer>,
    depth: Option<DepthTarget>,
    placed: HashMap<u64, Placed>,
}

impl shader::Pipeline for Pipeline {
    fn new(_device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Pipeline { format, renderers: HashMap::new(), depth: None, placed: HashMap::new() }
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
        let _ = DepthTarget::ensure(&mut pipeline.depth, device, size.width, size.height);
        let format = pipeline.format;
        let r = pipeline.renderers.entry(self.view).or_insert_with(|| Scene3dRenderer::new(device, format));
        let s = viewport.scale_factor();
        let (bx, by, bw, bh) = (bounds.x * s, bounds.y * s, bounds.width * s, bounds.height * s);
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
        // The viewport is the whole box (the projection is the box's); wgpu keeps a viewport inside the target, so a
        // box partly off it is drawn into the part on it.
        let whole = bx >= 0.0 && by >= 0.0 && bx + bw <= size.width as f32 && by + bh <= size.height as f32;
        let viewport = if whole { [bx, by, bw, bh] } else { [vx, vy, x1 - vx, y1 - vy] };
        pipeline.placed.insert(self.view, Placed { viewport, scissor: [vx as u32, vy as u32, x1 as u32, y1 as u32] });
    }

    fn draw(&self, _pipeline: &Pipeline, _render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        false
    }

    fn render(&self, pipeline: &Pipeline, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, clip: &Rectangle<u32>) {
        let (Some(p), Some(r)) = (pipeline.placed.get(&self.view), pipeline.renderers.get(&self.view)) else { return };
        let Some(depth) = pipeline.depth.as_ref().map(DepthTarget::view) else { return };
        let x0 = p.scissor[0].max(clip.x);
        let y0 = p.scissor[1].max(clip.y);
        let x1 = p.scissor[2].min(clip.x + clip.width);
        let y1 = p.scissor[3].min(clip.y + clip.height);
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("kentos.scene3d"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(DEPTH_CLEAR), store: wgpu::StoreOp::Discard }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let [vx, vy, vw, vh] = p.viewport;
        pass.set_viewport(vx, vy, vw, vh, 0.0, 1.0);
        pass.set_scissor_rect(x0, y0, x1 - x0, y1 - y0);
        r.render(&mut pass);
    }
}

/// The point picked, ringed where the camera shows it.
struct Marker {
    camera: Camera,
    point: Option<[f64; 3]>,
}

impl<M> canvas::Program<M> for Marker {
    type State = ();

    fn draw(&self, _: &(), renderer: &iced::Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        use iced::widget::canvas::{Frame, Path, Stroke};
        let Some(p) = self.point else { return Vec::new() };
        let vp = Viewport { width: f64::from(bounds.width), height: f64::from(bounds.height) };
        let Some((x, y, _)) = self.camera.project(p, &vp) else { return Vec::new() };
        let mut frame = Frame::new(renderer, bounds.size());
        let at = iced::Point::new(x as f32, y as f32);
        let accent = Tokens::of(theme).accent;
        frame.stroke(&Path::circle(at, 7.0), Stroke::default().with_color(Color::WHITE).with_width(4.0));
        frame.stroke(&Path::circle(at, 7.0), Stroke::default().with_color(accent).with_width(2.0));
        frame.fill(&Path::circle(at, 2.0), accent);
        vec![frame.into_geometry()]
    }
}
