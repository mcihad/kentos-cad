//! 3B görünüm (plan/road.md Y12.3d, docs/adr/0910 §4–§6, `scene3d.view`): the road project's terrain surfaces and
//! corridors in 3D, through the shared scene core and renderer: the meshes as they build now (`road_scene::scene`), cut
//! into tiles with their own f64 origins (`kentos_scene3d::chunk`), selected each frame by the screen-space error
//! (`select`) and drawn with a reversed depth buffer (`kentos_render_wgpu::scene3d`). This file only adapts: the window,
//! its camera and Iced's shader widget.
//!
//! The terrain is shaded smooth (each vertex the area-weighted normal of its triangles; Civil 3D's realistic styles and
//! ArcGIS smooth a TIN's shading the same way, the surface itself unchanged), Düz yüzler shows its faces. A corridor's
//! triangles are flat and coloured by what they are in the template: lanes, shoulders, cut and fill slopes
//! (`corridor::Role`); the terrain under them. The camera is the window's (an orbit about a point kept in the source's
//! heights): a drag publishes an orbit, a right drag (or Shift) a pan, the wheel a dolly, so the view buttons and a
//! picture are the same every time. The vertical exaggeration is the view's alone; the sources keep their heights.
//!
//! ```text
//! ┌ 3B görünüm ──────────────────────────────────────────────────────────────────────────────────┐
//! │ [Kuzeydoğu] [Güneybatı] [Kuzeybatı] [Üst] [Sığdır]       Düşey abartma [3]   [ ] Arazi saydam │
//! │ ┌ the scene: drag to orbit, right drag (or Shift) to pan, the wheel to dolly ┐                │
//! │ └────────────────────────────────────────────────────────────────────────────┘                │
//! │ ■ Arazi ■ Şerit ■ Banket ■ Yarma şevi ■ Dolgu şevi                                            │
//! │ 3 yüzey ve koridor · 928 üçgen · 3 karo çizildi …                     [glTF ver]  [Kapat]     │
//! └───────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use iced::widget::{Column, Row, container, row, shader, space};
use iced::{Background, Border, Color, Element, Fill, Rectangle, Task, Theme, mouse, wgpu};
use kentos_commands::SceneMeshKind;
use kentos_native_application::road_scene::{self, SurfaceRole};
use kentos_render_wgpu::scene3d::{DEPTH_CLEAR, DepthTarget, Frame as Scene3dFrame, Scene3dRenderer, Style};
use kentos_scene3d::{Aabb, Camera, Chunked, Content, Exaggeration, Limits, Projection, Settings, SourceMesh, Viewport, chunk, select};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::tree_view::Check;
use kentos_ui::label;

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "3B görünüm";
const CLOSE: &str = "Kapat";
const EXPORT: &str = "glTF ver";
const FIT: &str = "Sığdır";
const EXAGGERATION: &str = "Düşey abartma";
const SEE_THROUGH: &str = "Arazi saydam";
const CUT: &str = "Araziyi kes";
const FLAT: &str = "Düz yüzler";
/// The view presets: the camera's azimuth (degrees from east, counterclockwise) and elevation.
pub(crate) const VIEWS: [(&str, f64, f64); 4] =
    [("Kuzeydoğu", 45.0, 30.0), ("Güneybatı", 225.0, 30.0), ("Kuzeybatı", 135.0, 30.0), ("Üst", -90.0, 89.0)];
/// The parts the tiles carry and their colours (linear RGB): the terrain, then a corridor's roles.
pub(crate) const PARTS: [(&str, [f32; 3]); 5] = [
    ("Arazi", [0.36, 0.47, 0.25]),
    ("Şerit", [0.17, 0.17, 0.19]),
    ("Banket", [0.62, 0.60, 0.55]),
    ("Yarma şevi", [0.66, 0.45, 0.27]),
    ("Dolgu şevi", [0.85, 0.70, 0.42]),
];
const FOV: f64 = 40.0;
/// Refined above 16 pixels of error: CesiumJS's default `maximumScreenSpaceError`.
const MAX_SCREEN_ERROR: f64 = 16.0;

fn part_of(role: SurfaceRole) -> u32 {
    match role {
        SurfaceRole::Lane => 1,
        SurfaceRole::Shoulder => 2,
        SurfaceRole::Cut => 3,
        SurfaceRole::Fill => 4,
    }
}

/// A source mesh to cut: its part, vertices, triangles and whether it is smooth.
type Owned<'a> = (u32, &'a [[f64; 3]], Vec<[u32; 3]>, bool);

/// The scene cut into tiles, its bounds and what it is made of.
pub struct Scene {
    chunked: Chunked,
    first: u64,
    bounds: Aabb,
    sources: usize,
    triangles: usize,
    /// Each source mesh the tiles were cut from (their lineage's mesh index): what it is, its source's name, its
    /// vertices (f64, the source's) and triangles; a pick meets these.
    meshes: Vec<Picked3d>,
}

/// A source mesh a pick can meet: its part, its source's name, vertices and triangles.
struct Picked3d {
    part: u32,
    name: String,
    vertices: Arc<Vec<[f64; 3]>>,
    triangles: Vec<[u32; 3]>,
}

impl fmt::Debug for Scene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scene").field("tiles", &self.chunked.hierarchy.tiles.len()).finish_non_exhaustive()
    }
}

impl Scene {
    /// The scene of `parts`, the terrain shaded smooth or flat; a corridor is always flat (its components' edges
    /// show).
    fn of(parts: &[road_scene::ScenePart], smooth: bool) -> Scene {
        // A corridor is cut by its triangles' roles: one source mesh a role, the vertices shared.
        let mut owned: Vec<Owned<'_>> = Vec::new();
        let mut names: Vec<(String, Arc<Vec<[f64; 3]>>)> = Vec::new();
        for p in parts {
            let shared = Arc::new(p.vertices.clone());
            if p.kind == SceneMeshKind::Terrain || p.roles.len() != p.triangles.len() {
                owned.push((0, &p.vertices, p.triangles.clone(), smooth && p.kind == SceneMeshKind::Terrain));
                names.push((p.name.clone(), shared));
                continue;
            }
            let mut by: HashMap<u32, Vec<[u32; 3]>> = HashMap::new();
            for (t, r) in p.triangles.iter().zip(&p.roles) {
                by.entry(part_of(*r)).or_default().push(*t);
            }
            let mut keys: Vec<u32> = by.keys().copied().collect();
            keys.sort_unstable();
            for k in keys {
                owned.push((k, &p.vertices, by.remove(&k).unwrap_or_default(), false));
                names.push((p.name.clone(), shared.clone()));
            }
        }
        let meshes: Vec<SourceMesh> =
            owned.iter().map(|(part, v, t, smooth)| SourceMesh { part: *part, vertices: v, triangles: t, smooth: *smooth }).collect();
        let first = 1;
        let chunked = chunk(&meshes, Limits::default(), first);
        let bounds = chunked.hierarchy.roots.first().map_or(Aabb::EMPTY, |&r| chunked.hierarchy.tiles[r as usize].bounds);
        let meshes = owned
            .iter()
            .zip(names)
            .map(|((part, _, t, _), (name, vertices))| Picked3d { part: *part, name, vertices, triangles: t.clone() })
            .collect();
        Scene { chunked, first, bounds, sources: parts.len(), triangles: parts.iter().map(|p| p.triangles.len()).sum(), meshes }
    }

    /// The point a ray (in the shown space) meets first among the tiles `select` draws for `camera`: the source's
    /// coordinates, its height real, and what it is on.
    fn pick(&self, camera: &Camera, vp: &Viewport, ex: &Exaggeration, x: f64, y: f64) -> Option<Picked> {
        let drawn = select(&self.chunked.hierarchy, camera, vp, ex, Settings { max_screen_error: MAX_SCREEN_ERROR }, |_| true).draw;
        let (o, d) = camera.ray(x, y, vp);
        let hit = kentos_scene3d::pick(
            &self.chunked.hierarchy,
            &drawn,
            |id| self.content(id),
            |m, t| {
                let s = self.meshes.get(m as usize)?;
                let tri = s.triangles.get(t as usize)?;
                Some([s.vertices.get(tri[0] as usize).copied()?, s.vertices.get(tri[1] as usize).copied()?, s.vertices.get(tri[2] as usize).copied()?])
            },
            o,
            d,
            ex,
        )?;
        let s = &self.meshes[hit.mesh as usize];
        Some(Picked { point: hit.point, what: format!("{} · {}", s.name, PARTS[s.part as usize].0) })
    }

    fn content(&self, id: u64) -> Option<&Content> {
        self.chunked.contents.get(id.checked_sub(self.first)? as usize)
    }
}

/// A point picked in the scene: the source's coordinates (height real) and what it is on.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Picked {
    pub(crate) point: [f64; 3],
    pub(crate) what: String,
}

/// What the last frame drew: tiles, triangles, contents waiting, bytes on the GPU (read by the tests; the window's line
/// gives the scene, not a frame that may be a frame behind).
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct Seen {
    pub(crate) tiles: usize,
    pub(crate) triangles: u64,
    pub(crate) waiting: usize,
    pub(crate) resident: u64,
}

#[derive(Debug)]
pub struct Window {
    /// The meshes as they built when the window opened, the terrain whole.
    parts: Arc<Vec<road_scene::ScenePart>>,
    pub(crate) scene: Arc<Scene>,
    /// Whether the terrain is cut by the corridors (the ground they remove not drawn over them).
    pub(crate) cut: bool,
    /// Whether the terrain shows its faces (flat shading) rather than smooth.
    pub(crate) flat: bool,
    /// The point the camera turns about, in the source's heights.
    pub(crate) target: [f64; 3],
    pub(crate) azimuth: f64,
    pub(crate) elevation: f64,
    pub(crate) distance: f64,
    pub(crate) exaggeration: String,
    pub(crate) see_through: bool,
    /// The point last picked, none before a click.
    pub(crate) picked: Option<Picked>,
    pub(crate) seen: Arc<Mutex<Seen>>,
    pending: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Orbit(f32, f32),
    Pan(f32, f32),
    Dolly(f32),
    /// A click at a place of the view (logical pixels from its top left) of a size.
    Pick(f32, f32, f32, f32),
    View(usize),
    Fit,
    Exaggeration(String),
    SeeThrough,
    Cut,
    Flat,
    Export,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Scene3d(event)
}

#[cfg(test)]
impl Window {
    /// The scene's triangles, as its sources have them.
    pub(crate) fn scene_triangles(&self) -> usize {
        self.scene.triangles
    }

    /// The middle of the `n`th triangle of a source mesh of `part` (its real height): a point the pictures click.
    pub(crate) fn point_on(&self, part: u32, n: usize) -> Option<[f64; 3]> {
        let m = self.scene.meshes.iter().find(|m| m.part == part)?;
        let t = m.triangles.get(n.min(m.triangles.len().checked_sub(1)?))?;
        let [a, b, c] = t.map(|k| m.vertices[k as usize]);
        Some([(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0])
    }

    /// The parts its tiles carry.
    pub(crate) fn scene_parts(&self) -> Vec<u32> {
        let mut out: Vec<u32> = self.scene.chunked.contents.iter().flat_map(|c| c.ranges.iter().map(|r| r.0)).collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

impl Window {
    /// The vertical exaggeration as typed, about the scene's lowest point; 1 when it does not read or is not positive.
    pub(crate) fn exaggeration(&self) -> Exaggeration {
        let factor =
            self.exaggeration.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite() && *v > 0.0).unwrap_or(1.0);
        Exaggeration { factor, datum: self.scene.bounds.min[2] }
    }

    /// The camera now, in the shown space.
    pub(crate) fn camera(&self) -> Camera {
        let near = (self.distance * 1e-3).clamp(0.05, 5.0);
        let target = self.exaggeration().shown(self.target);
        Camera::orbit(target, self.azimuth, self.elevation, self.distance, Projection::Perspective { fov_y: FOV.to_radians(), near })
    }

    /// The camera at a preset's angles, looking at the scene's middle from where it all shows.
    fn fit(&mut self, azimuth: f64, elevation: f64) {
        let b = self.scene.bounds;
        let ex = self.exaggeration();
        let shown = ex.shown_box(&b);
        self.target = b.center();
        self.azimuth = azimuth;
        self.elevation = elevation;
        self.distance = (shown.diagonal() / 2.0).max(1.0) / (FOV.to_radians() / 2.0).sin() * 0.62;
    }
}

impl App {
    /// 3B görünüm: the road project's scene as it builds now, seen from the north-east.
    pub(crate) fn open_scene3d(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("3B görünüm yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let (parts, _) = road_scene::scene(&doc.model);
        if parts.is_empty() {
            self.warn("3B görünümde gösterilecek bir şey yok: önce bir arazi yüzeyi ya da koridor oluşturun.".to_owned());
            return;
        }
        let mut w = Window {
            scene: Arc::new(Scene::of(&road_scene::with_terrain_cut(&parts), true)),
            parts: Arc::new(parts),
            cut: true,
            flat: false,
            target: [0.0; 3],
            azimuth: 0.0,
            elevation: 0.0,
            distance: 1.0,
            exaggeration: "3".into(),
            see_through: false,
            picked: None,
            seen: Arc::default(),
            pending: Arc::default(),
        };
        let (_, a, e) = VIEWS[0];
        w.fit(a, e);
        self.scene3d_window = Some(Box::new(w));
        self.dialog = Some(Dialog::Scene3d);
    }

    pub(crate) fn scene3d_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.scene3d_window.as_mut() else { return Task::none() };
        match event {
            Event::Orbit(dx, dy) => {
                w.azimuth -= f64::from(dx) * 0.4;
                w.elevation = (w.elevation + f64::from(dy) * 0.3).clamp(2.0, 89.0);
            }
            Event::Pan(dx, dy) => {
                // Along the ground, the target's distance a window's width across.
                let k = w.distance * (FOV.to_radians() / 2.0).tan() * 2.0 / 600.0;
                let a = w.azimuth.to_radians();
                let (right, ahead) = ([-a.sin(), a.cos()], [-a.cos(), -a.sin()]);
                w.target[0] -= (right[0] * f64::from(dx) - ahead[0] * f64::from(dy)) * k;
                w.target[1] -= (right[1] * f64::from(dx) - ahead[1] * f64::from(dy)) * k;
            }
            Event::Dolly(steps) => w.distance = (w.distance * 0.85f64.powf(f64::from(steps))).clamp(1.0, 1.0e6),
            Event::Pick(x, y, width, height) => {
                let vp = Viewport { width: f64::from(width), height: f64::from(height) };
                w.picked = w.scene.pick(&w.camera(), &vp, &w.exaggeration(), f64::from(x), f64::from(y));
            }
            Event::View(i) => {
                let (_, a, e) = VIEWS[i.min(VIEWS.len() - 1)];
                w.fit(a, e);
            }
            Event::Fit => {
                let (a, e) = (w.azimuth, w.elevation);
                w.fit(a, e);
            }
            Event::Exaggeration(t) => w.exaggeration = t,
            Event::SeeThrough => w.see_through = !w.see_through,
            Event::Cut | Event::Flat => {
                if matches!(event, Event::Cut) {
                    w.cut = !w.cut;
                } else {
                    w.flat = !w.flat;
                }
                let parts = if w.cut { road_scene::with_terrain_cut(&w.parts) } else { w.parts.to_vec() };
                w.scene = Arc::new(Scene::of(&parts, !w.flat));
            }
            Event::Export => return self.export_gltf(),
            Event::Close => {
                self.scene3d_window = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn scene3d_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.scene3d_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let mut presets = row![].spacing(6);
        for (i, (name, _, _)) in VIEWS.iter().enumerate() {
            presets = presets.push(words::secondary(*name, Some(msg(Event::View(i)))));
        }
        presets = presets.push(words::secondary(FIT, Some(msg(Event::Fit))));
        let top = row![
            presets,
            space::horizontal().width(Fill),
            label::caption(EXAGGERATION).style(muted),
            kentos_ui::widget::text_field("1", &w.exaggeration).on_input(|t| msg(Event::Exaggeration(t))).width(60),
            crate::style::fields::check(
                if w.cut { Check::Checked } else { Check::Unchecked },
                CUT.to_owned(),
                Some(msg(Event::Cut)),
            ),
            crate::style::fields::check(
                if w.flat { Check::Checked } else { Check::Unchecked },
                FLAT.to_owned(),
                Some(msg(Event::Flat)),
            ),
            crate::style::fields::check(
                if w.see_through { Check::Checked } else { Check::Unchecked },
                SEE_THROUGH.to_owned(),
                Some(msg(Event::SeeThrough)),
            ),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Center);
        let program = Program {
            scene: w.scene.clone(),
            camera: w.camera(),
            exaggeration: w.exaggeration(),
            see_through: w.see_through,
            seen: w.seen.clone(),
            pending: w.pending.clone(),
        };
        // The scene takes the dialog's room: when the window was given a size or made full screen (docs/adr/0746) all
        // of it, else what the application window leaves under the rows, between 240 and 520 (scaled) pixels; the
        // legend under it, the line and the buttons always at the dialog's foot.
        let sized = kentos_ui::widget::sizable::sizes::get(TITLE).is_some();
        let room = (self.window_size.height - typography::scaled(330.0)).clamp(typography::scaled(240.0), typography::scaled(520.0));
        let marker = iced::widget::canvas(Marker {
            camera: w.camera(),
            point: w.picked.as_ref().map(|p| w.exaggeration().shown(p.point)),
        })
        .width(Fill)
        .height(Fill);
        let view = container(iced::widget::stack![shader(program).width(Fill).height(Fill), marker])
            .padding(1)
            .width(Fill)
            .height(Fill)
            .style(|t: &Theme| {
                let k = Tokens::of(t);
                let dark = k.surface.r + k.surface.g + k.surface.b < 1.5;
                container::Style {
                    // The sky: light above a light theme's window, deep above a dark one's.
                    background: Some(Background::Color(if dark { Color::from_rgb(0.10, 0.13, 0.18) } else { Color::from_rgb(0.86, 0.91, 0.96) })),
                    border: Border { color: k.border, width: 1.0, radius: 4.0.into() },
                    ..container::Style::default()
                }
            });
        let mut legend = Row::new().spacing(14).align_y(iced::alignment::Vertical::Center);
        for (name, c) in PARTS {
            let swatch = container(space::horizontal().width(14)).height(14).style(move |_: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgb(c[0].powf(1.0 / 2.2), c[1].powf(1.0 / 2.2), c[2].powf(1.0 / 2.2)))),
                border: Border { radius: 2.0.into(), ..Border::default() },
                ..container::Style::default()
            });
            legend = legend.push(row![swatch, label::caption(name)].spacing(5).align_y(iced::alignment::Vertical::Center));
        }
        legend = legend.push(space::horizontal().width(Fill));
        legend = legend.push(match &w.picked {
            Some(p) => label::caption(format!("Seçilen nokta: Y {:.3}  X {:.3}  Z {:.3} · {}", p.point[0], p.point[1], p.point[2], p.what)),
            None => label::caption("Bir noktaya tıklayın: koordinatı ve neyin üstünde olduğu burada yazar.").style(muted),
        });
        let area = container(Column::new().spacing(8).push(view).push(legend).height(Fill))
            .height(Fill)
            .max_height(if sized { f32::INFINITY } else { room + typography::scaled(26.0) });
        let ex = w.exaggeration();
        let info = format!(
            "{} yüzey ve koridor · {} üçgen, {} karo · düşey abartma {}× · bakış {:.0}°, eğim {:.0}° · sürükle: döndür, sağ ya da Shift: kaydır, tekerlek: yaklaş",
            w.scene.sources,
            w.scene.triangles,
            w.scene.chunked.contents.len(),
            ex.factor,
            w.azimuth.rem_euclid(360.0),
            w.elevation,
        );
        Frame::new(TITLE)
            .push(top)
            .fill(area)
            .footer(
                row![
                    label::caption(info).style(muted).width(Fill),
                    words::primary(EXPORT, Some(msg(Event::Export))),
                    space::horizontal().width(16),
                    words::secondary(CLOSE, Some(msg(Event::Close))),
                ]
                .spacing(8)
                .align_y(iced::alignment::Vertical::Center),
            )
            .width(1180.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn scene3d_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.scene3d_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(EXPORT) => Some(msg(Event::Export)),
            Control::Press(FIT) => Some(msg(Event::Fit)),
            Control::Press(SEE_THROUGH) => Some(msg(Event::SeeThrough)),
            Control::Press(CUT) => Some(msg(Event::Cut)),
            Control::Press(FLAT) => Some(msg(Event::Flat)),
            Control::Press(name) if VIEWS.iter().any(|v| v.0 == name) => {
                Some(msg(Event::View(VIEWS.iter().position(|v| v.0 == name).unwrap_or(0))))
            }
            Control::Fill(EXAGGERATION, t) => Some(msg(Event::Exaggeration(t.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

// ── The widget: Iced's shader over the shared renderer ────────────────────────────────────────────────────────────

struct Program {
    scene: Arc<Scene>,
    camera: Camera,
    exaggeration: Exaggeration,
    see_through: bool,
    seen: Arc<Mutex<Seen>>,
    pending: Arc<AtomicBool>,
}

/// A drag in progress (where, whether it pans) and the view's key in the pipeline.
pub struct State {
    view: u64,
    /// A press in progress: where the pointer was last, whether it pans, where it went down and whether it moved
    /// past a click.
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

impl Default for State {
    fn default() -> Self {
        State { view: NEXT_VIEW.fetch_add(1, Ordering::Relaxed), drag: None, shift: false }
    }
}

impl shader::Program<Message> for Program {
    type State = State;
    type Primitive = Frame3d;

    fn update(&self, state: &mut State, event: &iced::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<shader::Action<Message>> {
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
                Some(shader::Action::publish(msg(Event::Pick(p.down.x, p.down.y, bounds.width, bounds.height))).and_capture())
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
                let e = if p.pan { Event::Pan(dx, dy) } else { Event::Orbit(dx, dy) };
                Some(shader::Action::publish(msg(e)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                cursor.position_in(bounds)?;
                let steps = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };
                Some(shader::Action::publish(msg(Event::Dolly(steps))).and_capture())
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
            see_through: self.see_through,
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
    see_through: bool,
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
        let styles: Vec<Style> = PARTS
            .iter()
            .enumerate()
            .map(|(k, (_, c))| Style { color: [c[0], c[1], c[2], if k == 0 && self.see_through { 0.45 } else { 1.0 }] })
            .collect();
        let stats = r.prepare(
            device,
            queue,
            &Scene3dFrame {
                camera: &self.camera,
                exaggeration: self.exaggeration,
                viewport: vp,
                draws: &draws,
                styles: &styles,
                light: [-0.45, -0.35, 0.82],
                ambient: 0.38,
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

impl iced::widget::canvas::Program<Message> for Marker {
    type State = ();

    fn draw(&self, _: &(), renderer: &iced::Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<iced::widget::canvas::Geometry> {
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

