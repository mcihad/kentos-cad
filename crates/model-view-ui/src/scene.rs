//! The 3D scene on screen (docs/adr/0910 §4–§6, docs/adr/8073 §2): source meshes cut into tiles with their own f64
//! origins (`kentos_scene3d::chunk`), selected each frame by the screen-space error and drawn by the shared renderer
//! (`kentos_render_scene3d`) through Iced's shader widget, a clicked point ringed over it. Every 3D view draws with it:
//! the building's and the road's (the application's 3B görünüm) and the IFC viewer's. It knows no host: a drag, a right
//! drag (or Shift), the wheel, a click and a section's handle dragged (handles.rs, docs/adr/8132) come back as
//! [`Gesture`]s through the host's function. Each view draws at the
//! quality the host asks (docs/adr/8120): into its own targets, four samples a pixel, its bodies' edges and, at the
//! highest, the ambient light's occlusion, then laid over the window.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use iced::widget::{canvas, shader, stack};
use iced::{Color, Element, Fill, Rectangle, Theme, mouse, wgpu};
use kentos_model_view::section::Handle;
pub use kentos_render_scene3d::Quality;
use kentos_render_scene3d::{Frame as Scene3dFrame, Scene3dRenderer, Style};
use kentos_scene3d::{
    Aabb, Camera, Chunked, Content, Exaggeration, Limits, Settings, SourceMesh, Viewport, chunk,
    select,
};
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
    /// Each content's place in `chunked.contents` by its id.
    index: HashMap<u64, usize>,
    /// Each content's digest of what the GPU draws of it (its origin, positions, parts, normals, indices and ranges).
    digests: Vec<u64>,
}

/// Each scene's new contents take their own ids (a block of 2³² from here): a renderer keeps what it uploaded by id, so
/// a scene made again (a toggle, a selection) never draws a tile of the one before that differs from its own. A
/// content the scene before had as it is keeps its id (docs/adr/8171 §2.4): it is not uploaded again.
static NEXT_CONTENT: AtomicU64 = AtomicU64::new(1);

impl fmt::Debug for Scene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scene")
            .field("tiles", &self.chunked.hierarchy.tiles.len())
            .finish_non_exhaustive()
    }
}

impl Scene {
    /// The scene of `sources`, which show `count` things.
    pub fn of(sources: Vec<Source>, count: usize) -> Scene {
        Scene::after(sources, count, None)
    }

    /// The scene of `sources` made after `before` (the view's scene it takes the place of): each content `before` had
    /// as it is keeps its id, so a selection or a body hidden uploads only the tiles it changed (docs/adr/8171 §2.4).
    pub fn after(sources: Vec<Source>, count: usize, before: Option<&Scene>) -> Scene {
        let first = NEXT_CONTENT.fetch_add(1 << 32, Ordering::Relaxed);
        let mut chunked = {
            let meshes: Vec<SourceMesh> = sources
                .iter()
                .map(|s| SourceMesh {
                    part: s.part,
                    vertices: &s.vertices,
                    triangles: &s.triangles,
                    smooth: s.smooth,
                })
                .collect();
            chunk(&meshes, Limits::default(), first)
        };
        let digests: Vec<u64> = chunked.contents.iter().map(digest).collect();
        if let Some(b) = before {
            let mut kept: HashMap<u64, u64> = HashMap::new();
            let mut used: std::collections::HashSet<u64> = std::collections::HashSet::new();
            let mut by_digest: HashMap<u64, Vec<usize>> = HashMap::new();
            for (k, d) in b.digests.iter().enumerate() {
                by_digest.entry(*d).or_default().push(k);
            }
            for (k, c) in chunked.contents.iter_mut().enumerate() {
                let same = by_digest.get(&digests[k]).and_then(|list| {
                    list.iter()
                        .map(|&j| &b.chunked.contents[j])
                        .find(|o| !used.contains(&o.id) && drawn_alike(o, c))
                });
                if let Some(o) = same {
                    used.insert(o.id);
                    kept.insert(c.id, o.id);
                    c.id = o.id;
                }
            }
            for t in &mut chunked.hierarchy.tiles {
                if let Some(c) = &mut t.content
                    && let Some(&id) = kept.get(&c.id)
                {
                    c.id = id;
                }
            }
        }
        let index = chunked
            .contents
            .iter()
            .enumerate()
            .map(|(k, c)| (c.id, k))
            .collect();
        let bounds = chunked
            .hierarchy
            .roots
            .first()
            .map_or(Aabb::EMPTY, |&r| chunked.hierarchy.tiles[r as usize].bounds);
        let triangles = sources.iter().map(|s| s.triangles.len()).sum();
        Scene {
            chunked,
            first,
            bounds,
            sources: count,
            triangles,
            meshes: sources,
            index,
            digests,
        }
    }

    /// Its contents' ids.
    pub fn content_ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.chunked.contents.iter().map(|c| c.id)
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
        let mut out: Vec<u32> = self
            .chunked
            .contents
            .iter()
            .flat_map(|c| c.ranges.iter().map(|r| r.0))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The point a ray (in the shown space) meets first among the tiles `select` draws for `camera`: the source's
    /// coordinates, its height real, what it is on and its object. A click goes through a source made to be seen
    /// through to what is behind it; it meets that source only when nothing else is there.
    pub fn pick(
        &self,
        camera: &Camera,
        vp: &Viewport,
        ex: &Exaggeration,
        x: f64,
        y: f64,
    ) -> Option<Picked> {
        self.pick_cut(camera, vp, ex, x, y, &[])
    }

    /// As [`Scene::pick`] in a scene a section cuts (`planes`, in the shown space): only the part of the ray the
    /// planes keep meets anything, so a click never meets what is cut away.
    pub fn pick_cut(
        &self,
        camera: &Camera,
        vp: &Viewport,
        ex: &Exaggeration,
        x: f64,
        y: f64,
        planes: &[[f64; 4]],
    ) -> Option<Picked> {
        let drawn = select(
            &self.chunked.hierarchy,
            camera,
            vp,
            ex,
            Settings {
                max_screen_error: MAX_SCREEN_ERROR,
            },
            |_| true,
        )
        .draw;
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
                    let s = self
                        .meshes
                        .get(m as usize)
                        .filter(|s| through || !s.through)?;
                    let tri = s.triangles.get(t as usize)?;
                    Some([
                        s.vertices.get(tri[0] as usize).copied()?,
                        s.vertices.get(tri[1] as usize).copied()?,
                        s.vertices.get(tri[2] as usize).copied()?,
                    ])
                },
                o,
                d,
                ex,
            )
        };
        let within = |h: &kentos_scene3d::Hit| t0 + h.distance <= t1;
        let hit = meet(false)
            .filter(within)
            .or_else(|| meet(true).filter(within))?;
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
        self.chunked.contents.get(*self.index.get(&id)?)
    }
}

/// A digest of what the GPU draws of a content: its origin, positions, parts, normals, indices and ranges.
fn digest(c: &Content) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    c.origin.map(f64::to_bits).hash(&mut h);
    for p in &c.positions {
        p.map(f32::to_bits).hash(&mut h);
    }
    c.parts.hash(&mut h);
    c.normals.hash(&mut h);
    c.indices.hash(&mut h);
    for (part, r) in &c.ranges {
        (part, r.start, r.end).hash(&mut h);
    }
    h.finish()
}

/// Whether two contents draw alike on the GPU (what a renderer uploads of them is the same).
fn drawn_alike(a: &Content, b: &Content) -> bool {
    a.origin == b.origin
        && a.positions == b.positions
        && a.parts == b.parts
        && a.normals == b.normals
        && a.indices == b.indices
        && a.ranges == b.ranges
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
    /// A section's handle (its place among the view's handles) dragged this far along its arrow, metres
    /// (docs/adr/8132).
    Handle(usize, f64),
    /// The handle let go.
    HandleDone(usize),
}

/// What the scene draws over itself besides the clicked point (docs/adr/8121): a measurement's lines, points and
/// values, in the source's coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum Mark {
    /// A measured line; faint: a component along an axis, an outline, the line to the pointer.
    Line {
        a: [f64; 3],
        b: [f64; 3],
        faint: bool,
    },
    /// A point measured; with a snap, the pointer's: its symbol (a square at a corner, a triangle at an edge's middle,
    /// a diamond on an edge, a ring on a face) and its name beside it.
    Point {
        at: [f64; 3],
        snap: Option<kentos_model_view::measure::Snap>,
    },
    /// A value on a plate at a point.
    Value { at: [f64; 3], text: String },
}

/// One view of a scene: the scene, the camera, the exaggeration, the parts' styles, towards the light, the sky's and
/// the fill's shares of the light (0: the light alone; docs/adr/8120), the quality it is drawn at, the half-spaces a
/// section keeps (in the shown space; none: nothing cut), the point ringed, and the frame's shared notes (what it drew;
/// whether contents are still waiting for the GPU). With `sun` the light is the sun and the faces cast its shadows
/// within the scene's box (docs/adr/8177 §4).
pub struct SceneView {
    pub scene: Arc<Scene>,
    /// A layer drawn with the scene: a section's caps (docs/adr/8131, 8171 §2.2), made again as the section moves while
    /// the scene stays as it is; none: nothing over it.
    pub caps: Option<Arc<Scene>>,
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
    /// A section's handles (docs/adr/8132), and the one being dragged.
    pub handles: Vec<Handle>,
    pub active: Option<usize>,
    pub seen: Arc<Mutex<Seen>>,
    pub pending: Arc<AtomicBool>,
    pub sun: bool,
}

/// The scene drawn and its gestures sent through `on`.
pub fn view<'a, M: Clone + 'a>(v: SceneView, on: impl Fn(Gesture) -> M + 'a) -> Element<'a, M> {
    let handles = Arc::new(v.handles);
    let marker = canvas(Marker {
        camera: v.camera,
        point: v.marked,
        marks: v.marks,
        handles: handles.clone(),
        active: v.active,
    })
    .width(Fill)
    .height(Fill);
    let program = Program {
        scene: v.scene,
        caps: v.caps,
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
        sun: v.sun,
        hover: v.hover,
        handles,
        on: Box::new(on),
    };
    stack![shader(program).width(Fill).height(Fill), marker].into()
}

// ── The widget: Iced's shader over the shared renderer ────────────────────────────────────────────────────────────

struct Program<'a, M> {
    scene: Arc<Scene>,
    caps: Option<Arc<Scene>>,
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
    sun: bool,
    hover: bool,
    handles: Arc<Vec<Handle>>,
    on: Box<dyn Fn(Gesture) -> M + 'a>,
}

/// A drag in progress (where, whether it pans), the view's key in the pipeline, the size last told, the pointer's
/// place last told (none: outside) and the handle under it.
pub struct State {
    view: u64,
    drag: Option<Press>,
    shift: bool,
    size: (f32, f32),
    over: Option<(f32, f32)>,
    hot: Option<usize>,
}

#[derive(Clone, Copy)]
struct Press {
    last: iced::Point,
    pan: bool,
    down: iced::Point,
    moved: bool,
    /// The section's handle it took (docs/adr/8132): its drag moves the plane, not the camera.
    handle: Option<usize>,
}

/// A press that moves less than this (logical pixels) is a click: it picks.
const CLICK: f32 = 3.0;

static NEXT_VIEW: AtomicU64 = AtomicU64::new(1);

/// The views whose widgets are gone (their window closed, the scene left the screen): their renderers and the GPU
/// buffers they hold are let go at the next frame a view draws. A window of its own takes its pipeline with it.
static RETIRED: Mutex<Vec<u64>> = Mutex::new(Vec::new());

impl Default for State {
    fn default() -> Self {
        State {
            view: NEXT_VIEW.fetch_add(1, Ordering::Relaxed),
            drag: None,
            shift: false,
            size: (0.0, 0.0),
            over: None,
            hot: None,
        }
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

    fn update(
        &self,
        state: &mut State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<shader::Action<M>> {
        use iced::keyboard;
        match event {
            // The view's size told when it changed (the message draws the next frame); else contents still waiting for
            // the GPU: another frame.
            iced::Event::Window(iced::window::Event::RedrawRequested(_)) => {
                let size = (bounds.width, bounds.height);
                if size != state.size && size.0 >= 1.0 && size.1 >= 1.0 {
                    state.size = size;
                    return Some(shader::Action::publish((self.on)(Gesture::Size(
                        size.0, size.1,
                    ))));
                }
                self.pending
                    .load(Ordering::Relaxed)
                    .then(shader::Action::request_redraw)
            }
            iced::Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => {
                state.shift = m.shift();
                None
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(
                b @ (mouse::Button::Left | mouse::Button::Right),
            )) => {
                let at = cursor.position_in(bounds)?;
                // A left press on a section's handle takes it (docs/adr/8132).
                let handle = (*b == mouse::Button::Left)
                    .then(|| self.handle_at(bounds, at))
                    .flatten();
                state.drag = Some(Press {
                    last: at,
                    pan: *b == mouse::Button::Right || state.shift,
                    down: at,
                    moved: false,
                    handle,
                });
                Some(shader::Action::capture())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(_)) => {
                let p = state.drag.take()?;
                if let Some(i) = p.handle {
                    return Some(
                        shader::Action::publish((self.on)(Gesture::HandleDone(i))).and_capture(),
                    );
                }
                if p.moved || p.pan {
                    return Some(shader::Action::capture());
                }
                Some(
                    shader::Action::publish((self.on)(Gesture::Pick(
                        p.down.x,
                        p.down.y,
                        bounds.width,
                        bounds.height,
                    )))
                    .and_capture(),
                )
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) if state.drag.is_none() => {
                // Over a section's handle or off it: drawn again, lit or not (the marks' layer reads the pointer).
                let hot = cursor
                    .position_in(bounds)
                    .and_then(|p| self.handle_at(bounds, p));
                let relit = hot != state.hot;
                state.hot = hot;
                // While the host measures: the pointer's place when it moved a pixel, and once when it left.
                if !self.hover {
                    return relit.then(shader::Action::request_redraw);
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
                Some(shader::Action::publish((self.on)(Gesture::Hover(
                    at,
                    bounds.width,
                    bounds.height,
                ))))
            }
            iced::Event::Mouse(mouse::Event::CursorLeft) if self.hover && state.over.is_some() => {
                state.over = None;
                Some(shader::Action::publish((self.on)(Gesture::Hover(
                    None,
                    bounds.width,
                    bounds.height,
                ))))
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
                let g = match p.handle {
                    // The handle's plane follows the pointer along its arrow.
                    Some(i) => {
                        let vp = Viewport {
                            width: f64::from(bounds.width),
                            height: f64::from(bounds.height),
                        };
                        let Some(h) = self.handles.get(i) else {
                            return Some(shader::Action::capture());
                        };
                        Gesture::Handle(i, crate::handles::along(h, &self.camera, &vp, dx, dy))
                    }
                    None if p.pan => Gesture::Pan(dx, dy),
                    None => Gesture::Orbit(dx, dy),
                };
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
            caps: self.caps.clone(),
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
            // The box the sun's shadows are cast within: the scene's and its caps'.
            shadows: self
                .sun
                .then(|| {
                    let mut b = self.scene.bounds();
                    if let Some(c) = &self.caps {
                        let o = c.bounds();
                        for i in 0..3 {
                            b.min[i] = b.min[i].min(o.min[i]);
                            b.max[i] = b.max[i].max(o.max[i]);
                        }
                    }
                    [b.min, b.max]
                })
                .filter(|[lo, hi]| {
                    (0..3).all(|i| lo[i].is_finite() && hi[i].is_finite() && lo[i] <= hi[i])
                }),
        }
    }

    fn mouse_interaction(
        &self,
        state: &State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        match state.drag {
            Some(_) => mouse::Interaction::Grabbing,
            None if cursor
                .position_in(bounds)
                .and_then(|p| self.handle_at(bounds, p))
                .is_some() =>
            {
                mouse::Interaction::Pointer
            }
            None if cursor.is_over(bounds) => mouse::Interaction::Grab,
            None => mouse::Interaction::default(),
        }
    }
}

impl<M> Program<'_, M> {
    /// The section's handle at a place of the view (docs/adr/8132).
    fn handle_at(&self, bounds: Rectangle, at: iced::Point) -> Option<usize> {
        if self.handles.is_empty() {
            return None;
        }
        let vp = Viewport {
            width: f64::from(bounds.width),
            height: f64::from(bounds.height),
        };
        crate::handles::at(&self.handles, &self.camera, &vp, at)
    }
}

/// One frame of a view, handed to the pipeline.
pub struct Frame3d {
    view: u64,
    scene: Arc<Scene>,
    caps: Option<Arc<Scene>>,
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
    shadows: Option<[[f64; 3]; 2]>,
}

impl fmt::Debug for Frame3d {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame3d")
            .field("view", &self.view)
            .finish_non_exhaustive()
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
    /// The scenes each view drew last (their first ids) and their contents' ids: a scene made again lets go of the
    /// contents of the one before it does not keep (docs/adr/8171 §2.4) at once, not when the budget is full.
    drawn: HashMap<u64, (Vec<u64>, std::collections::HashSet<u64>)>,
    /// Each view's ground while the sun lights it.
    grounds: HashMap<u64, Ground>,
}

impl shader::Pipeline for Pipeline {
    fn new(_device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Pipeline {
            format,
            renderers: HashMap::new(),
            placed: HashMap::new(),
            drawn: HashMap::new(),
            grounds: HashMap::new(),
        }
    }
}

/// The ground the sun's shadows fall on (docs/adr/8177 §4), while the sun lights a scene: a square three times the
/// scene's width about its middle, 2 cm under ±0.00 when the scene's heights reach across it (a ground floor's slab
/// on it shows over it), else 2 cm under the scene. Without it a building stood in the sky and its shadow fell on
/// nothing.
struct Ground {
    /// The square's middle, half side and height, and its part (the style after the scene's): made again when one
    /// changes.
    key: [u64; 5],
    content: Content,
}

/// The content id the ground takes (a scene's take blocks of 2³² from the bottom).
const GROUND_ID: u64 = u64::MAX - 1;
/// The ground's colour: a light, warm grey.
const GROUND_STYLE: Style = Style::of([0.80, 0.78, 0.74, 1.0]);

impl Ground {
    /// Its height for the scene's box `lo`–`hi`.
    fn height(lo: [f64; 3], hi: [f64; 3]) -> f64 {
        if lo[2] <= 0.0 && hi[2] >= 0.0 {
            -0.02
        } else {
            lo[2] - 0.02
        }
    }

    fn of(lo: [f64; 3], hi: [f64; 3], part: u32) -> Option<Ground> {
        let (cx, cy) = ((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0);
        let half = (1.5 * (hi[0] - lo[0]).max(hi[1] - lo[1])).max(20.0);
        let z = Ground::height(lo, hi);
        let key = [
            cx.to_bits(),
            cy.to_bits(),
            half.to_bits(),
            z.to_bits(),
            u64::from(part),
        ];
        let vertices = [
            [cx - half, cy - half, z],
            [cx + half, cy - half, z],
            [cx + half, cy + half, z],
            [cx - half, cy + half, z],
        ];
        let mesh = kentos_scene3d::SourceMesh {
            part,
            vertices: &vertices,
            triangles: &[[0, 1, 2], [0, 2, 3]],
            smooth: false,
        };
        let content = kentos_scene3d::chunk(
            &[mesh],
            kentos_scene3d::Limits {
                max_triangles: 8,
                max_extent: f64::MAX,
            },
            GROUND_ID,
        )
        .contents
        .into_iter()
        .next()?;
        Some(Ground { key, content })
    }
}

impl shader::Primitive for Frame3d {
    type Pipeline = Pipeline;

    fn prepare(
        &self,
        pipeline: &mut Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        if let Ok(mut retired) = RETIRED.lock() {
            for view in retired.drain(..) {
                pipeline.renderers.remove(&view);
                pipeline.placed.remove(&view);
                pipeline.drawn.remove(&view);
            }
        }
        let size = viewport.physical_size();
        let format = pipeline.format;
        // A quality changed in the settings: the view's renderer made again at it (its contents uploaded again).
        if pipeline
            .renderers
            .get(&self.view)
            .is_some_and(|r| r.quality() != self.quality)
        {
            pipeline.renderers.remove(&self.view);
        }
        let r = pipeline
            .renderers
            .entry(self.view)
            .or_insert_with(|| Scene3dRenderer::with_quality(device, format, self.quality));
        let s = viewport.scale_factor();
        let (bx, by, bw, bh) = (
            bounds.x * s,
            bounds.y * s,
            bounds.width * s,
            bounds.height * s,
        );
        r.ensure_targets(
            device,
            bw.round().max(1.0) as u32,
            bh.round().max(1.0) as u32,
        );
        let vp = Viewport {
            width: f64::from(bw.max(1.0)),
            height: f64::from(bh.max(1.0)),
        };
        let scenes: Vec<&Scene> = std::iter::once(&*self.scene)
            .chain(self.caps.as_deref())
            .collect();
        // The contents of the scenes this view drew before and draws no more: let go.
        let firsts: Vec<u64> = scenes.iter().map(|s| s.first).collect();
        if pipeline
            .drawn
            .get(&self.view)
            .is_none_or(|(f, _)| *f != firsts)
        {
            let ids: std::collections::HashSet<u64> =
                scenes.iter().flat_map(|s| s.content_ids()).collect();
            if let Some((_, before)) = pipeline.drawn.insert(self.view, (firsts, ids.clone())) {
                for id in before.difference(&ids) {
                    r.forget(*id);
                }
            }
        }
        // The ground while the sun lights the scene, made again when the scene's box or parts change.
        let ground = self
            .shadows
            .and_then(|[lo, hi]| Ground::of(lo, hi, self.styles.len() as u32));
        if pipeline.grounds.get(&self.view).map(|g| g.key) != ground.as_ref().map(|g| g.key) {
            r.forget(GROUND_ID);
            match ground {
                Some(g) => pipeline.grounds.insert(self.view, g),
                None => pipeline.grounds.remove(&self.view),
            };
        }
        let ground = pipeline.grounds.get(&self.view).map(|g| &g.content);
        let settings = Settings {
            max_screen_error: MAX_SCREEN_ERROR,
        };
        let content = |id: u64| scenes.iter().find_map(|s| s.content(id));
        // Select with what is resident, upload what it draws and asks for within the budgets, select again with what
        // is resident now, and draw that.
        let first: Vec<_> = scenes
            .iter()
            .map(|s| {
                select(
                    &s.chunked.hierarchy,
                    &self.camera,
                    &vp,
                    &self.exaggeration,
                    settings,
                    |c| r.is_resident(c.id),
                )
            })
            .collect();
        let mut asked: Vec<&Content> = scenes
            .iter()
            .zip(&first)
            .flat_map(|(s, sel)| {
                let h = &s.chunked.hierarchy;
                sel.draw
                    .iter()
                    .chain(&sel.wanted)
                    .filter_map(move |&i| h.tiles[i as usize].content)
                    .filter_map(|c| content(c.id))
            })
            .collect();
        asked.extend(ground);
        r.load(device, &asked);
        let now: Vec<_> = scenes
            .iter()
            .map(|s| {
                select(
                    &s.chunked.hierarchy,
                    &self.camera,
                    &vp,
                    &self.exaggeration,
                    settings,
                    |c| r.is_resident(c.id),
                )
            })
            .collect();
        let mut draws: Vec<&Content> = scenes
            .iter()
            .zip(&now)
            .flat_map(|(s, sel)| {
                let h = &s.chunked.hierarchy;
                sel.draw
                    .iter()
                    .filter_map(move |&i| h.tiles[i as usize].content)
                    .filter_map(|c| content(c.id))
            })
            .collect();
        draws.extend(ground.filter(|g| r.is_resident(g.id)));
        let wanted: usize = now.iter().map(|sel| sel.wanted.len()).sum();
        // The ground's style after the scene's; the shadows' box down to the ground.
        let styles: Vec<Style> = self
            .styles
            .iter()
            .copied()
            .chain(ground.map(|_| GROUND_STYLE))
            .collect();
        let shadows = self
            .shadows
            .map(|[lo, hi]| [[lo[0], lo[1], lo[2].min(Ground::height(lo, hi))], hi]);
        let stats = r.prepare(
            device,
            queue,
            &Scene3dFrame {
                camera: &self.camera,
                exaggeration: self.exaggeration,
                viewport: vp,
                draws: &draws,
                styles: &styles,
                light: self.light,
                ambient: 0.38,
                sky: self.sky,
                fill: self.fill,
                clip: &self.planes,
                shadows,
            },
        );
        self.pending.store(wanted > 0, Ordering::Relaxed);
        if let Ok(mut seen) = self.seen.lock() {
            *seen = Seen {
                tiles: stats.tiles,
                triangles: stats.triangles,
                waiting: wanted,
                resident: stats.resident_bytes,
            };
        }
        let (vx, vy) = (bx.max(0.0), by.max(0.0));
        let (x1, y1) = (
            (bx + bw).min(size.width as f32),
            (by + bh).min(size.height as f32),
        );
        if x1 <= vx || y1 <= vy {
            pipeline.placed.remove(&self.view);
            return;
        }
        // The view's own targets are the whole box (the projection is the box's); wgpu keeps a viewport inside the
        // window, so a box partly off it lays the part of its targets on it.
        let (bw, bh) = (bw.max(1.0), bh.max(1.0));
        let uv = [
            (vx - bx) / bw,
            (vy - by) / bh,
            (x1 - bx) / bw,
            (y1 - by) / bh,
        ];
        r.place(queue, uv);
        pipeline.placed.insert(
            self.view,
            Placed {
                viewport: [vx, vy, x1 - vx, y1 - vy],
                scissor: [vx as u32, vy as u32, x1 as u32, y1 as u32],
            },
        );
    }

    fn draw(&self, _pipeline: &Pipeline, _render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        false
    }

    fn render(
        &self,
        pipeline: &Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip: &Rectangle<u32>,
    ) {
        let (Some(p), Some(r)) = (
            pipeline.placed.get(&self.view),
            pipeline.renderers.get(&self.view),
        ) else {
            return;
        };
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

/// The point picked, ringed where the camera shows it, the measurements' marks (docs/adr/8121): their lines in the
/// accent colour (faint ones thinner and dashed), their points white-ringed dots, the pointer's snap its symbol and its
/// name, their values on plates of the field's colour; and under them a section's handles (docs/adr/8132), the one
/// being dragged or under the pointer lit.
struct Marker {
    camera: Camera,
    point: Option<[f64; 3]>,
    marks: Vec<Mark>,
    handles: Arc<Vec<Handle>>,
    active: Option<usize>,
}

impl<M> canvas::Program<M> for Marker {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        use iced::widget::canvas::{Frame, LineDash, Path, Stroke, Text};
        use kentos_model_view::measure::Snap;
        if self.point.is_none() && self.marks.is_empty() && self.handles.is_empty() {
            return Vec::new();
        }
        let vp = Viewport {
            width: f64::from(bounds.width),
            height: f64::from(bounds.height),
        };
        let on_screen = |p: [f64; 3]| {
            self.camera
                .project(p, &vp)
                .map(|(x, y, _)| iced::Point::new(x as f32, y as f32))
        };
        let mut frame = Frame::new(renderer, bounds.size());
        let t = Tokens::of(theme);
        let accent = t.accent;
        // A section's handles under the marks; the one dragged, else the one under the pointer, lit.
        let hot = self.active.or_else(|| {
            cursor
                .position_in(bounds)
                .and_then(|p| crate::handles::at(&self.handles, &self.camera, &vp, p))
        });
        crate::handles::draw(&mut frame, &self.handles, &self.camera, &vp, hot, &t);
        // Lines first, then points, then the values over them.
        for m in &self.marks {
            if let Mark::Line { a, b, faint } = m
                && let (Some(a), Some(b)) = (on_screen(*a), on_screen(*b))
            {
                let path = Path::line(a, b);
                if *faint {
                    let dash = Stroke {
                        line_dash: LineDash {
                            segments: &[5.0, 4.0],
                            offset: 0,
                        },
                        ..Stroke::default()
                    };
                    frame.stroke(
                        &path,
                        dash.with_color(accent.scale_alpha(0.75)).with_width(1.2),
                    );
                } else {
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_color(Color::WHITE.scale_alpha(0.85))
                            .with_width(4.0),
                    );
                    frame.stroke(&path, Stroke::default().with_color(accent).with_width(2.0));
                }
            }
        }
        for m in &self.marks {
            let Mark::Point { at, snap } = m else {
                continue;
            };
            let Some(c) = on_screen(*at) else { continue };
            match snap {
                None => {
                    frame.fill(&Path::circle(c, 4.5), Color::WHITE);
                    frame.fill(&Path::circle(c, 3.0), accent);
                }
                Some(s) => {
                    let symbol = match s {
                        Snap::Corner => Path::rectangle(
                            iced::Point::new(c.x - 6.0, c.y - 6.0),
                            iced::Size::new(12.0, 12.0),
                        ),
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
                    frame.stroke(
                        &symbol,
                        Stroke::default()
                            .with_color(Color::WHITE.scale_alpha(0.9))
                            .with_width(4.0),
                    );
                    frame.stroke(
                        &symbol,
                        Stroke::default().with_color(accent).with_width(1.8),
                    );
                    frame.fill(&Path::circle(c, 1.8), accent);
                    plate(
                        &mut frame,
                        s.name(),
                        iced::Point::new(c.x + 12.0, c.y - 14.0),
                        &t,
                        false,
                    );
                }
            }
        }
        for m in &self.marks {
            if let Mark::Value { at, text } = m
                && let Some(c) = on_screen(*at)
            {
                plate(
                    &mut frame,
                    text,
                    iced::Point::new(c.x, c.y - 14.0),
                    &t,
                    true,
                );
            }
        }
        if let Some(c) = self.point.and_then(on_screen) {
            frame.stroke(
                &Path::circle(c, 7.0),
                Stroke::default().with_color(Color::WHITE).with_width(4.0),
            );
            frame.stroke(
                &Path::circle(c, 7.0),
                Stroke::default().with_color(accent).with_width(2.0),
            );
            frame.fill(&Path::circle(c, 2.0), accent);
        }
        let _ = Text::default;
        vec![frame.into_geometry()]
    }
}

/// Words on a plate of the field's colour, centred on `at` (a value) or starting at it (a snap's name); kept within the
/// scene, so a value at its edge is read whole.
fn plate(
    frame: &mut iced::widget::canvas::Frame,
    words: &str,
    at: iced::Point,
    t: &Tokens,
    centred: bool,
) {
    use iced::widget::canvas::{Path, Stroke, Text};
    let size = 12.0;
    // The UI font's digits and letters are about 0.56 of its size wide.
    let width = words.chars().count() as f32 * size * 0.56 + 12.0;
    let left = if centred { at.x - width / 2.0 } else { at.x };
    let left = left.clamp(4.0, (frame.width() - width - 4.0).max(4.0));
    let at = iced::Point::new(at.x, at.y.clamp(14.0, (frame.height() - 14.0).max(14.0)));
    let body = Path::rounded_rectangle(
        iced::Point::new(left, at.y - 10.0),
        iced::Size::new(width, 20.0),
        iced::border::Radius::from(4.0),
    );
    frame.fill(&body, t.field.scale_alpha(0.94));
    frame.stroke(
        &body,
        Stroke::default()
            .with_color(if centred { t.accent } else { t.border })
            .with_width(1.0),
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A box of side 1 at `x` as a body's source of `part`.
    fn cube(x: f64, part: u32) -> Source {
        let c = |i: usize| {
            [
                x + (i & 1) as f64,
                ((i >> 1) & 1) as f64,
                ((i >> 2) & 1) as f64,
            ]
        };
        let faces = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ];
        let (mut vertices, mut triangles) = (Vec::new(), Vec::new());
        for f in faces {
            let at = vertices.len() as u32;
            vertices.extend(f.map(c));
            triangles.extend([[at, at + 1, at + 2], [at, at + 2, at + 3]]);
        }
        Source {
            part,
            name: "kutu".into(),
            owner: None,
            vertices: Arc::new(vertices),
            triangles,
            smooth: false,
            through: false,
        }
    }

    /// Two boxes 2 km apart, a tile each (a tile spans at most 1 km): made again as it was, every content keeps its id;
    /// the second box painted another part (chosen), only its tile takes a new one (docs/adr/8171 §2.4).
    #[test]
    fn a_scene_made_again_keeps_the_ids_of_the_tiles_it_did_not_change() {
        let a = Scene::of(vec![cube(0.0, 0), cube(2000.0, 0)], 2);
        let ids = |s: &Scene| -> Vec<u64> { s.content_ids().collect() };
        assert_eq!(a.contents(), 2);
        let same = Scene::after(vec![cube(0.0, 0), cube(2000.0, 0)], 2, Some(&a));
        assert_eq!(
            ids(&same),
            ids(&a),
            "nothing changed: nothing uploaded again"
        );
        let chosen = Scene::after(vec![cube(0.0, 0), cube(2000.0, 7)], 2, Some(&same));
        let kept: Vec<u64> = ids(&chosen)
            .into_iter()
            .filter(|id| ids(&same).contains(id))
            .collect();
        assert_eq!(
            kept.len(),
            1,
            "the first box's tile kept, the chosen one's new: {:?} → {:?}",
            ids(&same),
            ids(&chosen)
        );
        // Every tile finds its content by its id, kept or new.
        for t in &chosen.chunked.hierarchy.tiles {
            if let Some(c) = &t.content {
                assert!(chosen.content(c.id).is_some(), "tile content {}", c.id);
            }
        }
    }
}
