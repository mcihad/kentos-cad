//! 3B görünüm (plan/road.md Y12.3d, docs/adr/0910 §4–§6, `scene3d.view`): the road project's terrain surfaces and
//! corridors in 3D through the shared scene core and renderer: the meshes as they build now ([`Source`]), cut into
//! tiles with their own f64 origins (`kentos_scene3d::chunk`), selected each frame by the screen-space error (`select`)
//! and drawn with a reversed depth buffer (`kentos_render_scene3d`). The scene, its widget, its look and the orbit
//! camera are the model viewer's (crates/model-view-ui, kentos-model-view; docs/adr/8073); this file only adapts: the
//! window, what it shows and its events. Any other drawing's building is the model viewer itself, its tree, details
//! and section with it (scene3d_building.rs, docs/adr/8119); the window only hosts it.
//!
//! The terrain is shaded smooth (each vertex the area-weighted normal of its triangles, the surface itself unchanged),
//! Düz yüzler shows its faces. A corridor's triangles are flat and coloured by what they are in the template: lanes,
//! shoulders, cut and fill slopes (`corridor::Role`); the terrain under them. The camera is the window's (an orbit
//! about a point kept in the source's heights): a drag publishes an orbit, a right drag (or Shift) a pan, the wheel a
//! dolly, so the view buttons and a picture are the same every time. The vertical exaggeration is the view's alone;
//! the sources keep their heights.
//!
//! The window opens beside the plan in an operating-system window of its own (shell/windows/, docs/adr/8060 §9), or
//! over the drawing once the user brings it into the main window; its look (the tool strip, the scene with its north
//! arrow and legend, the status bar) is scene3d_view.rs. Beside the plan the main window is free, so the scene follows
//! the drawing ([`App::follow_scene3d`]).
//!
//! ```text
//! ┌ 3B görünüm — KentOS CAD ─────────────────────────────────────────────────────────────────────┐
//! │ ◈ Kuzeydoğu ▾  ⤢ Sığdır │ Düşey abartma [3] │ Araziyi kes  Düz yüzler  Arazi saydam  ⇩ glTF ver ⧉ │
//! ├───────────────────────────────────────────────────────────────────────────────────────────────┤
//! │        the scene: drag to orbit, right drag (or Shift) to pan, the wheel to dolly        (K)  │
//! │ ┌ Lejant ┐                                                                                    │
//! │ │■ Arazi │                                                                                    │
//! ├───────────────────────────────────────────────────────────────────────────────────────────────┤
//! │ ⌖ Şerit, Koridor 1   Y …  X …  Z …                                  3 yüzey ve koridor │ ? Fare │
//! └───────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use iced::Task;
use kentos_commands::SceneMeshKind;
pub(crate) use kentos_model_view::VIEWS;
use kentos_model_view::OrbitCamera;
use kentos_model_view_ui::look::{LEGEND, NORTH};
use kentos_model_view_ui::scene::{Picked, Seen};
pub(crate) use kentos_model_view_ui::scene::{Scene, Source};
use kentos_native_application::road_scene::{self, SurfaceRole};
use kentos_render_wgpu::scene3d::Style;
use kentos_scene3d::{Camera, Exaggeration, Viewport};

use crate::app::{App, Message};
use crate::scene3d_building::{Building3d, Made};
use crate::traces::Control;

#[path = "scene3d_view.rs"]
mod view;

pub(crate) use view::{View3d, sky};

/// The window's title, which a trace names it by.
pub const TITLE: &str = "3B görünüm";
const CLOSE: &str = "Kapat";
const EXPORT: &str = "glTF ver";
const FIT: &str = "Sığdır";
const EXAGGERATION: &str = "Düşey abartma";
const SEE_THROUGH: &str = "Arazi saydam";
const CUT: &str = "Araziyi kes";
const FLAT: &str = "Düz yüzler";
/// The parts the tiles carry and their colours (linear RGB): the terrain, then a corridor's roles.
pub(crate) const PARTS: [(&str, [f32; 3]); 5] = [
    ("Arazi", [0.36, 0.47, 0.25]),
    ("Şerit", [0.17, 0.17, 0.19]),
    ("Banket", [0.62, 0.60, 0.55]),
    ("Yarma şevi", [0.66, 0.45, 0.27]),
    ("Dolgu şevi", [0.85, 0.70, 0.42]),
];

fn part_of(role: SurfaceRole) -> u32 {
    match role {
        SurfaceRole::Lane => 1,
        SurfaceRole::Shoulder => 2,
        SurfaceRole::Cut => 3,
        SurfaceRole::Fill => 4,
    }
}

/// The road project's scene of `parts`, the terrain shaded smooth or flat; a corridor is always flat (its
/// components' edges show) and cut by its triangles' roles: one source mesh a role, the vertices shared.
fn road(parts: &[road_scene::ScenePart], smooth: bool) -> Scene {
    let mut sources: Vec<Source> = Vec::new();
    for p in parts {
        let shared = Arc::new(p.vertices.clone());
        let source = |part: u32, triangles: Vec<[u32; 3]>, smooth: bool| Source {
            part,
            name: format!("{}, {}", PARTS[part as usize].0, p.name),
            owner: None,
            vertices: shared.clone(),
            triangles,
            smooth,
            through: false,
        };
        if p.kind == SceneMeshKind::Terrain || p.roles.len() != p.triangles.len() {
            sources.push(source(0, p.triangles.clone(), smooth && p.kind == SceneMeshKind::Terrain));
            continue;
        }
        let mut by: HashMap<u32, Vec<[u32; 3]>> = HashMap::new();
        for (t, r) in p.triangles.iter().zip(&p.roles) {
            by.entry(part_of(*r)).or_default().push(*t);
        }
        let mut keys: Vec<u32> = by.keys().copied().collect();
        keys.sort_unstable();
        for k in keys {
            sources.push(source(k, by.remove(&k).unwrap_or_default(), false));
        }
    }
    Scene::of(sources, parts.len())
}

#[derive(Debug)]
pub struct Window {
    /// The road project's meshes as they built when the window opened, the terrain whole; none for a building.
    parts: Arc<Vec<road_scene::ScenePart>>,
    /// The building shown in the model viewer (scene3d_building.rs); none: the road project's scene, which the fields
    /// below draw.
    pub building: Option<Building3d>,
    pub(crate) scene: Arc<Scene>,
    /// Whether the terrain is cut by the corridors (the ground they remove not drawn over them).
    pub(crate) cut: bool,
    /// Whether the terrain shows its faces (flat shading) rather than smooth.
    pub(crate) flat: bool,
    /// The camera: about a point in the source's heights.
    pub(crate) orbit: OrbitCamera,
    pub(crate) exaggeration: String,
    pub(crate) see_through: bool,
    /// The point last picked, none before a click.
    pub(crate) picked: Option<Picked>,
    /// Whether the legend over the scene is open (its heading folds it).
    pub(crate) legend: bool,
    /// The drawing's generation and revision the road's scene was made from (a building keeps its own).
    made: (u64, u64),
    pub seen: Arc<Mutex<Seen>>,
    pending: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Orbit(f32, f32),
    Pan(f32, f32),
    Dolly(f32),
    /// A click at a place of the view (logical pixels from its top left) of a size.
    Pick(f32, f32, f32, f32),
    /// The scene's size at its first frame and when it changes: the road's camera stays as the user put it.
    Sized(f32, f32),
    View(usize),
    Fit,
    Exaggeration(String),
    SeeThrough,
    Cut,
    Flat,
    /// A building's storey shown or hidden, by its place.
    Storey(usize),
    /// Every storey of a building shown again.
    AllStoreys,
    /// A building's viewer's own message (its tree, details, section, switches and scene).
    Viewer(kentos_model_view_ui::Message),
    /// A building's model made (by its making's number), or why not; empty: stopped.
    Made(u64, Result<Arc<Made>, String>),
    /// The camera turned to look north, its height and distance kept (the north arrow).
    North,
    /// The legend over the scene folded or opened.
    Legend,
    Export,
    /// Paftaya koy: the building's view drawn and placed on the sheet in front (docs/adr/8175 §3.2).
    ToSheet,
    /// Its picture made, or why not.
    Pictured(Result<Arc<crate::scene3d_sheet::Picture>, String>),
    Close,
}

fn msg(event: Event) -> Message {
    Message::Scene3d(event)
}

#[cfg(any(test, feature = "testing"))]
impl Window {
    /// The scene drawn: a building's viewer's, else the road's.
    fn drawn(&self) -> &Arc<Scene> {
        match &self.building {
            Some(b) => b.viewer.scene(),
            None => &self.scene,
        }
    }

    /// The camera the scene is drawn with and its exaggeration: a building's viewer's (seen as it is), else the road's.
    pub fn drawn_camera(&self) -> (Camera, Exaggeration) {
        match &self.building {
            Some(b) => (b.viewer.camera(), Exaggeration::NONE),
            None => (self.camera(), self.exaggeration()),
        }
    }

    /// An object's key in the scene as a test names it: a building's drawing objects by their persistent ids (their
    /// GlobalIds in the viewer), anything else as it is.
    fn named(&self, key: &str) -> String {
        match (&self.building, crate::scene3d_building::uid_of(key)) {
            (Some(_), Some(u)) => u.to_string(),
            _ => key.to_owned(),
        }
    }

    /// The scene's key of an object a test names (the inverse of [`Window::named`]).
    fn key_of(&self, owner: &str) -> String {
        match (&self.building, kentos_domain::Uuid::parse_str(owner)) {
            (Some(_), Ok(u)) => kentos_ifc::step::guid(u.as_u128()),
            _ => owner.to_owned(),
        }
    }

    /// The point last picked: a building's viewer's, else the road's.
    pub(crate) fn drawn_picked(&self) -> Option<&Picked> {
        match &self.building {
            Some(b) => b.viewer.picked.as_ref(),
            None => self.picked.as_ref(),
        }
    }

    /// The object of the point last picked, as a test names it.
    pub fn picked_owner(&self) -> Option<String> {
        self.drawn_picked().and_then(|p| p.owner.as_deref()).map(|k| self.named(k))
    }

    /// The scene's triangles, as its sources have them.
    pub fn scene_triangles(&self) -> usize {
        self.drawn().triangles()
    }

    /// The middle of the `n`th triangle of a source mesh of `part` (its real height): a point the pictures click.
    pub fn point_on(&self, part: u32, n: usize) -> Option<[f64; 3]> {
        let m = self.drawn().meshes().iter().find(|m| m.part == part)?;
        let t = m.triangles.get(n.min(m.triangles.len().checked_sub(1)?))?;
        let [a, b, c] = t.map(|k| m.vertices[k as usize]);
        Some([(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0])
    }

    /// How many things the scene shows.
    pub fn scene_sources(&self) -> usize {
        self.drawn().sources()
    }

    /// The middle of the `n`th triangle of the `k`th source mesh of `part` and that mesh's object: a place a click
    /// meets it, unless something stands in front.
    pub fn owner_on(&self, part: u32, k: usize, n: usize) -> Option<([f64; 3], String)> {
        let m = self.drawn().meshes().iter().filter(|m| m.part == part).nth(k)?;
        let t = m.triangles.get(n)?;
        let [a, b, c] = t.map(|i| m.vertices[i as usize]);
        let owner = self.named(m.owner.as_deref()?);
        Some(([(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0], owner))
    }

    /// The parts of the source meshes of an object.
    pub fn parts_of(&self, owner: &str) -> Vec<u32> {
        let key = self.key_of(owner);
        self.drawn().meshes().iter().filter(|m| m.owner.as_deref() == Some(key.as_str())).map(|m| m.part).collect()
    }

    /// The parts its tiles carry, a section's caps' layer with them (docs/adr/8171 §2.2).
    pub fn scene_parts(&self) -> Vec<u32> {
        let mut parts = self.drawn().parts();
        if let Some(caps) = self.building.as_ref().and_then(|b| b.viewer.caps()) {
            parts.extend(caps.parts());
            parts.sort_unstable();
            parts.dedup();
        }
        parts
    }
}

impl Window {
    /// The road's parts' styles, the terrain see-through when asked.
    fn styles(&self) -> Arc<Vec<Style>> {
        Arc::new(
            PARTS
                .iter()
                .enumerate()
                .map(|(k, (_, c))| Style::of([c[0], c[1], c[2], if k == 0 && self.see_through { 0.45 } else { 1.0 }]))
                .collect(),
        )
    }

    /// Towards the light: the road's sun from the south-west.
    fn light(&self) -> [f64; 3] {
        [-0.45, -0.35, 0.82]
    }

    /// The vertical exaggeration as typed, about the scene's lowest point; 1 when it does not read or is not positive.
    pub fn exaggeration(&self) -> Exaggeration {
        let factor =
            self.exaggeration.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite() && *v > 0.0).unwrap_or(1.0);
        Exaggeration { factor, datum: self.scene.bounds().min[2] }
    }

    /// The camera now, in the shown space.
    pub(crate) fn camera(&self) -> Camera {
        self.orbit.camera(&self.exaggeration())
    }

    /// A window on `scene`, seen from the north-east: the road's (its `parts`, made from the drawing's `made`
    /// generation and revision), or a building's viewer (its own scene and camera; `scene` empty).
    pub(crate) fn new(scene: Scene, parts: Vec<road_scene::ScenePart>, building: Option<Building3d>, made: (u64, u64)) -> Window {
        let mut w = Window {
            scene: Arc::new(scene),
            parts: Arc::new(parts),
            // A building is seen as it is, a road's ground three times as steep.
            exaggeration: if building.is_some() { "1" } else { "3" }.into(),
            building,
            cut: true,
            flat: false,
            orbit: OrbitCamera { target: [0.0; 3], azimuth: 0.0, elevation: 0.0, distance: 1.0 },
            see_through: false,
            picked: None,
            legend: true,
            made,
            seen: Arc::default(),
            pending: Arc::default(),
        };
        w.look(0);
        w
    }

    /// The room a fit leaves: a road's ground is flat and wide, so closer.
    fn room(&self) -> f64 {
        0.62
    }

    /// The camera at the `i`th view's angles, looking at the scene's middle from where it all shows.
    fn look(&mut self, i: usize) {
        let (b, ex, room) = (self.scene.bounds(), self.exaggeration(), self.room());
        self.orbit.look(i, &b, &ex, room);
    }

    /// The whole scene in view at the camera's angles.
    fn fit(&mut self) {
        let (b, ex, room) = (self.scene.bounds(), self.exaggeration(), self.room());
        self.orbit.fit(&b, &ex, room);
    }
}

impl App {
    /// 3B görünüm: the road project's scene as it builds now, seen from the north-east; or any other drawing's
    /// building in the model viewer, its model made off this thread (scene3d_building.rs).
    pub(crate) fn open_scene3d(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            return self.open_building3d();
        }
        let (parts, _) = road_scene::scene(&doc.model);
        if parts.is_empty() {
            self.warn("3B görünümde gösterilecek bir şey yok: önce bir arazi yüzeyi ya da koridor oluşturun.".to_owned());
            return Task::none();
        }
        let made = (doc.model.generation(), doc.model.revision());
        let scene = road(&road_scene::with_terrain_cut(&parts), true);
        self.scene3d_window = Some(Box::new(Window::new(scene, parts, None, made)));
        self.show_surface(crate::windows::Surface::Scene3d);
        Task::none()
    }

    /// The window closes: its scene goes, and the main window's dialog when the window was over the drawing.
    pub(crate) fn scene3d_gone(&mut self) {
        self.scene3d_window = None;
        self.surface_away(crate::windows::Surface::Scene3d);
    }

    /// 3B görünüm follows the drawing while it is open (shell/app/update.rs, after every message): beside the plan
    /// the main window is free, so an edit, an undo or another drawing opened shows in 3D at once, and a building
    /// takes the plan's selection and the findings chosen in Denetim, its model made again off this thread
    /// (scene3d_building.rs). The camera stays where the user put it. The drawing closed, or nothing left to show,
    /// closes the window and says why.
    pub(crate) fn follow_scene3d(&mut self) -> Task<Message> {
        let Some(building) = self.scene3d_window.as_ref().map(|w| w.building.is_some()) else { return Task::none() };
        let Some(doc) = self.document.as_ref() else {
            self.scene3d_gone();
            return Task::none();
        };
        if building {
            return self.building3d_follow();
        }
        let now = (doc.model.generation(), doc.model.revision());
        if self.scene3d_window.as_ref().is_none_or(|w| w.made == now) {
            return Task::none();
        }
        let (parts, _) = road_scene::scene(&doc.model);
        if parts.is_empty() {
            self.scene3d_gone();
            self.warn("3B görünümde gösterilecek arazi yüzeyi ya da koridor kalmadı; 3B görünüm kapandı.".to_owned());
            return Task::none();
        }
        let Some(w) = self.scene3d_window.as_mut() else { return Task::none() };
        let shown = if w.cut { road_scene::with_terrain_cut(&parts) } else { parts.clone() };
        w.scene = Arc::new(road(&shown, !w.flat));
        w.parts = Arc::new(parts);
        w.made = now;
        Task::none()
    }

    pub(crate) fn scene3d_event(&mut self, event: Event) -> Task<Message> {
        if self.scene3d_window.as_ref().is_some_and(|w| w.building.is_some()) {
            return self.building3d_event(event);
        }
        let Some(w) = self.scene3d_window.as_mut() else { return Task::none() };
        match event {
            Event::Orbit(dx, dy) => w.orbit.orbit(f64::from(dx), f64::from(dy)),
            Event::Pan(dx, dy) => w.orbit.pan(f64::from(dx), f64::from(dy)),
            Event::Dolly(steps) => w.orbit.dolly(f64::from(steps)),
            Event::Pick(x, y, width, height) => {
                let vp = Viewport { width: f64::from(width), height: f64::from(height) };
                w.picked = w.scene.pick(&w.camera(), &vp, &w.exaggeration(), f64::from(x), f64::from(y));
            }
            // From the south, looking north: north is up on the screen.
            Event::North => w.orbit.north(),
            Event::Legend => w.legend = !w.legend,
            Event::View(i) => w.look(i),
            Event::Fit => w.fit(),
            Event::Exaggeration(t) => w.exaggeration = t,
            Event::SeeThrough => w.see_through = !w.see_through,
            Event::Cut | Event::Flat => {
                if matches!(event, Event::Cut) {
                    w.cut = !w.cut;
                } else {
                    w.flat = !w.flat;
                }
                let parts = if w.cut { road_scene::with_terrain_cut(&w.parts) } else { w.parts.to_vec() };
                w.scene = Arc::new(road(&parts, !w.flat));
            }
            // glTF ver: the module's command (docs/adr/9199 §3.3).
            Event::Export => return self.run("gltf.export"),
            Event::Close => self.scene3d_gone(),
            Event::Sized(..) => {}
            // A building's alone.
            Event::Storey(_) | Event::AllStoreys | Event::Viewer(_) | Event::Made(..) | Event::ToSheet | Event::Pictured(_) => {}
        }
        Task::none()
    }

    /// An event of 3B görünüm while it shows a building: the window's own buttons as the viewer's messages, Katlar's
    /// ticks as the storeys' eyes in its tree (scene3d_building.rs).
    fn building3d_event(&mut self, event: Event) -> Task<Message> {
        use kentos_model_view_ui::Message as V;
        use kentos_model_view_ui::scene::Gesture;
        let viewer = match event {
            Event::Made(id, result) => return self.building3d_made(id, result),
            Event::ToSheet => return self.building3d_to_sheet(),
            Event::Pictured(made) => return self.building3d_pictured(made),
            Event::Viewer(m) => m,
            Event::Orbit(dx, dy) => V::Gesture(Gesture::Orbit(dx, dy)),
            Event::Pan(dx, dy) => V::Gesture(Gesture::Pan(dx, dy)),
            Event::Dolly(steps) => V::Gesture(Gesture::Dolly(steps)),
            Event::Pick(x, y, width, height) => V::Gesture(Gesture::Pick(x, y, width, height)),
            Event::Sized(width, height) => V::Gesture(Gesture::Size(width, height)),
            Event::View(i) => V::View(i),
            Event::Fit => V::Fit,
            Event::North => V::North,
            Event::Legend => V::Legend,
            Event::SeeThrough => V::SeeThrough,
            Event::Close => V::Close,
            Event::Storey(i) => {
                if let Some(b) = self.scene3d_window.as_mut().and_then(|w| w.building.as_mut()) {
                    b.toggle_storey(i);
                }
                return Task::none();
            }
            Event::AllStoreys => {
                if let Some(b) = self.scene3d_window.as_mut().and_then(|w| w.building.as_mut()) {
                    b.show_all_storeys();
                }
                return Task::none();
            }
            // The road's alone.
            Event::Exaggeration(_) | Event::Cut | Event::Flat | Event::Export => return Task::none(),
        };
        self.building3d_viewer(viewer);
        Task::none()
    }

    pub(crate) fn scene3d_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = self.scene3d_window.as_ref() else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        if let Some(b) = &w.building {
            if let Some(m) = b.control(&control) {
                return Ok(Some(m));
            }
            return crate::ifc_viewer::viewer_control(&b.viewer, control, TITLE).map(|m| Some(msg(Event::Viewer(m))));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(EXPORT) => Some(msg(Event::Export)),
            Control::Press(FIT) => Some(msg(Event::Fit)),
            Control::Press(SEE_THROUGH) => Some(msg(Event::SeeThrough)),
            Control::Press(CUT) => Some(msg(Event::Cut)),
            Control::Press(FLAT) => Some(msg(Event::Flat)),
            Control::Press(LEGEND) => Some(msg(Event::Legend)),
            Control::Press(NORTH) => Some(msg(Event::North)),
            Control::Press(name) if VIEWS.iter().any(|v| v.0 == name) => {
                Some(msg(Event::View(VIEWS.iter().position(|v| v.0 == name).unwrap_or(0))))
            }
            Control::Fill(EXAGGERATION, t) => Some(msg(Event::Exaggeration(t.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
