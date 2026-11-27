//! Nokta bulutu alan istatistiği, Nokta bulutu profili and Mesh sor (docs/adr/0552, plan/gis.md CBS-32): three windows
//! over the read-only commands `cad.pointclouds.region`, `cad.pointclouds.profile` and `cad.meshes.query`; Mesh sor
//! asks at a place (Yer), in an area (Alan: `cad.meshes.region`) or along a path (Hat: `cad.meshes.profile`;
//! docs/adr/0645 §7). The shape
//! is the selection's (an area, a line or a polyline, a point; Seçimden al reads it again), the other values are typed;
//! Çalıştır reads the files off the window's thread at the source's full resolution and shows what the command answers:
//! counts, heights and classes of the area, the stations of the corridor, the face and the values at the place. Panoya
//! kopyala takes the answer as text. A script and the window do the same.

use std::fmt::Write as _;

use iced::widget::{Column, row};
use iced::{Element, Fill, Task};
use kentos_commands::{
    CommandResult, CommandWarning, Entity, MeshZonal, MeshesProfile, MeshesProfiled, MeshesQueried,
    MeshesQuery, MeshesRegion, MeshesRegioned, PointCloudsProfile, PointCloudsProfiled,
    PointCloudsRegion, PointCloudsRegioned, Vec2,
};
use kentos_interaction::Format;
use kentos_native_application::{ExecutionContext, mesh_areas, meshes, pointcloud_areas};
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, overlay};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message};

/// The rows of a table shown at most; the copy holds them all.
const SHOWN: usize = 60;

/// What a window asked the command, and what it answered.
#[derive(Debug, Clone)]
pub enum Outcome {
    Region(Result<(PointCloudsRegioned, Vec<CommandWarning>), String>),
    Profile(Result<(PointCloudsProfiled, Vec<CommandWarning>), String>),
    Mesh(Result<(MeshesQueried, Vec<CommandWarning>), String>),
    MeshRegion(Result<(MeshesRegioned, Vec<CommandWarning>), String>),
    MeshProfile(Result<(MeshesProfiled, Vec<CommandWarning>), String>),
}

/// What Mesh sor asks: at a place, in an area or along a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshMode {
    Place,
    Area,
    Path,
}

impl std::fmt::Display for MeshMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MeshMode::Place => "Yer",
            MeshMode::Area => "Alan",
            MeshMode::Path => "Hat",
        })
    }
}

/// An area taken from the selection: its ring and holes in plan, what it is said to be.
#[derive(Debug, Clone)]
pub struct Shape {
    pub ring: Vec<Vec2>,
    pub holes: Vec<Vec<Vec2>>,
    pub said: String,
}

/// The drawing's point clouds a window may read (GIS-036): each one's id and its name as the window says it; with
/// one, it is named and taken; with more, Tüm bulutlar or one of them.
#[derive(Debug, Clone, Default)]
pub struct Clouds {
    pub all: Vec<(String, String)>,
    /// With more than one: 0 Tüm bulutlar, `i` the `i`-th cloud.
    pub chosen: usize,
}

impl Clouds {
    /// The ids the command reads: none for all.
    pub(super) fn uids(&self) -> Vec<String> {
        match self.chosen {
            0 => Vec::new(),
            i => self.all.get(i - 1).map(|c| vec![c.0.clone()]).unwrap_or_default(),
        }
    }
}

/// How an answer's numbers are written (GIS-036, GIS-041): in the project's units; a geographic project's plan
/// lengths and areas in degrees, as its drawing is (the command measures in the drawing's plane), heights in metres.
#[derive(Clone, Copy, Debug)]
pub struct Units {
    pub f: Format,
    pub degrees: bool,
}

impl Units {
    pub(super) fn area(&self, v: f64) -> String {
        if self.degrees { format!("{} derece²", degrees(v)) } else { self.f.area(v) }
    }

    pub(super) fn plan(&self, v: f64) -> String {
        if self.degrees { format!("{}°", degrees(v)) } else { self.f.length(v) }
    }

    fn plan_bare(&self, v: f64) -> String {
        if self.degrees { degrees(v) } else { self.f.length_bare(v) }
    }

    fn per_area(&self) -> &'static str {
        if self.degrees { "nokta/derece²" } else { "nokta/m²" }
    }
}

/// Degrees with up to seven decimals (about a centimetre), trailing zeros dropped.
fn degrees(v: f64) -> String {
    let t = format!("{v:.7}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    if t.is_empty() || t == "-" { "0".to_owned() } else { t.to_owned() }
}

#[derive(Debug)]
pub struct RegionState {
    pub clouds: Clouds,
    /// The project's units, as the answer is written.
    pub units: Units,
    pub shape: Result<Shape, String>,
    pub result: Option<Outcome>,
    pub running: bool,
}

#[derive(Debug)]
pub struct ProfileState {
    pub clouds: Clouds,
    pub units: Units,
    /// The path taken from the selection and what it is said to be.
    pub path: Result<(Vec<Vec2>, String), String>,
    pub width: String,
    pub step: String,
    pub points: bool,
    pub result: Option<Outcome>,
    pub running: bool,
}

#[derive(Debug)]
pub struct MeshState {
    pub units: Units,
    /// The drawing's meshes the window reads (docs/adr/9335): with more than one, all or one of them.
    pub meshes: Clouds,
    pub mode: MeshMode,
    pub x: String,
    pub y: String,
    /// The area (Alan) and the path (Hat) taken from the selection.
    pub shape: Result<Shape, String>,
    pub path: Result<(Vec<Vec2>, String), String>,
    pub step: String,
    /// Alan: list the faces too.
    pub faces: bool,
    /// A moment as the windows write it (`2024-05-01 06:30`); empty: the steps the raster shows.
    pub time: String,
    pub series: bool,
    pub result: Option<Outcome>,
    pub running: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// Seçimden al: the shape, the path or the place read from the selection again.
    Use,
    /// Sahneden seç: the window steps aside while the area or the line is picked on the drawing, and comes back with
    /// the same values (GIS-036).
    Pick,
    /// The cloud read: 0 Tüm bulutlar, `i` the `i`-th.
    Cloud(usize),
    Width(String),
    Step(String),
    X(String),
    Y(String),
    Time(String),
    /// Noktaları da listele, Her adımı göster, Yüzleri listele.
    Toggle,
    /// Mesh sor's Yer, Alan or Hat.
    Mode(MeshMode),
    Run,
    Done(Box<Outcome>),
    Copy,
}

fn event(e: Event) -> Message {
    message(Exchange::Area(e))
}

fn fixed(v: f64) -> String {
    let t = format!("{v:.3}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    if t.is_empty() || t == "-" {
        "0".to_owned()
    } else {
        t.to_owned()
    }
}

/// The area the selection holds: its first area (or closed line), its holes; why not.
fn shape_of(app: &App) -> Result<Shape, String> {
    let Some(doc) = &app.document else {
        return Err("Açık çizim yok.".into());
    };
    for slot in app.selection.ids() {
        let Some(e) = doc.model.get(*slot) else {
            continue;
        };
        let path = match e {
            Entity::Polygon(p) => Some(p),
            Entity::Polyline(p) if p.closed => Some(p),
            _ => None,
        };
        let Some(p) = path else { continue };
        if p.pts.len() < 3 || ring_area(&p.pts).abs() <= 0.0 {
            return Err("Seçilen alan geçersiz: en az üç ayrı köşesi ve sıfırdan büyük alanı olmalı. Başka bir alan seçin.".into());
        }
        let curved = p
            .bulges
            .as_ref()
            .is_some_and(|b| b.iter().any(|v| *v != 0.0));
        let mut said = format!("{} köşeli alan", p.pts.len());
        if let Some(h) = &p.holes
            && !h.is_empty()
        {
            let _ = write!(said, ", {} delik", h.len());
        }
        if p.parts.as_ref().is_some_and(|x| !x.is_empty()) {
            said.push_str("; yalnız ilk parçası");
        }
        if curved {
            said.push_str("; yaylı kenarlar köşelerini birleştiren doğru sayılır");
        }
        return Ok(Shape {
            ring: p.pts.clone(),
            holes: p.holes.iter().flatten().map(|h| h.pts.clone()).collect(),
            said,
        });
    }
    Err("Seçimde alan yok: Sahneden seç ile çizimde alanı (ya da kapalı çoklu çizgiyi) tıklayıp Enter'a basın; pencere aynı ayarlarla döner.".into())
}

/// The ring's signed area (the shoelace), only to tell a flat ring.
fn ring_area(pts: &[Vec2]) -> f64 {
    let n = pts.len();
    (0..n).map(|i| {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        (a.x - pts[0].x) * (b.y - pts[0].y) - (b.x - pts[0].x) * (a.y - pts[0].y)
    }).sum::<f64>() / 2.0
}

/// The line or polyline the selection holds; why not.
fn path_of(app: &App) -> Result<(Vec<Vec2>, String), String> {
    let Some(doc) = &app.document else {
        return Err("Açık çizim yok.".into());
    };
    for slot in app.selection.ids() {
        match doc.model.get(*slot) {
            Some(Entity::Line(l)) => return Ok((vec![l.a, l.b], "2 köşeli çizgi".into())),
            Some(Entity::Polyline(p)) if !p.closed => {
                return Ok((p.pts.clone(), format!("{} köşeli çoklu çizgi", p.pts.len())));
            }
            _ => {}
        }
    }
    // The way to it said (UR-038, GIS-036): Sahneden seç steps the window aside; the cloud needs no selecting.
    Err("Seçimde çizgi yok: Sahneden seç ile çizimde profil yolunu (çizgi ya da çoklu çizgi) tıklayıp Enter'a basın; pencere aynı ayarlarla döner.".into())
}

/// The drawing's point clouds, each by its label, else its first file's name, with its points.
fn clouds_of(app: &App) -> Clouds {
    let Some(doc) = &app.document else {
        return Clouds::default();
    };
    let all = doc
        .model
        .entities_with_uids()
        .filter_map(|(u, e)| match e {
            Entity::PointCloud(c) => {
                let label = c.base.label.as_deref().map(str::trim).filter(|l| !l.is_empty());
                let file = c.cloud.sources.iter().find_map(|s| {
                    s.file.as_deref().or(s.url.as_deref()).map(|f| {
                        std::path::Path::new(f)
                            .file_name()
                            .map_or_else(|| f.to_owned(), |n| n.to_string_lossy().into_owned())
                    })
                });
                let name = label
                    .map(str::to_owned)
                    .or(file)
                    .unwrap_or_else(|| doc.model.layers().get(&c.base.layer_id).map_or_else(|| "Nokta bulutu".to_owned(), |l| l.name.clone()));
                Some((u.to_string(), format!("“{name}” · {} nokta", grouped(c.cloud.count))))
            }
            _ => None,
        })
        .collect();
    Clouds { all, chosen: 0 }
}

/// The drawing's meshes (docs/adr/9335), each by its label, else its file's name, else its layer's, with the dataset
/// and the mesh it shows.
fn meshes_of(app: &App) -> Clouds {
    let Some(doc) = &app.document else {
        return Clouds::default();
    };
    let all = doc
        .model
        .entities_with_uids()
        .filter_map(|(u, e)| match e {
            Entity::Raster(r) => {
                let set = r.raster.dataset.as_ref()?;
                let mesh = set.mesh.as_deref()?;
                let label = r.base.label.as_deref().map(str::trim).filter(|l| !l.is_empty());
                let file = r.raster.file.as_deref().or(r.raster.url.as_deref()).map(|f| {
                    std::path::Path::new(f)
                        .file_name()
                        .map_or_else(|| f.to_owned(), |n| n.to_string_lossy().into_owned())
                });
                let name = label.map(str::to_owned).or(file).unwrap_or_else(|| {
                    doc.model.layers().get(&r.base.layer_id).map_or_else(|| "Mesh".to_owned(), |l| l.name.clone())
                });
                Some((u.to_string(), format!("“{name}” · {} · ağ {mesh}", set.variable)))
            }
            _ => None,
        })
        .collect();
    Clouds { all, chosen: 0 }
}

/// Points per square metre with three significant digits below one (0.00237), else three decimals.
fn density(d: f64) -> String {
    if d >= 1.0 || d <= 0.0 || !d.is_finite() {
        return fixed(d);
    }
    let decimals = (2 - d.log10().floor() as i32).clamp(3, 12) as usize;
    let t = format!("{d:.decimals$}");
    t.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A count with its thousands apart by dots (2.822).
fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// The first selected point, its place as typed text.
fn place_of(app: &App) -> Option<(String, String)> {
    let doc = app.document.as_ref()?;
    app.selection
        .ids()
        .iter()
        .find_map(|s| match doc.model.get(*s) {
            Some(Entity::Point(p)) => Some((fixed(p.p.x), fixed(p.p.y))),
            _ => None,
        })
}

fn answered<T>(r: CommandResult<T>, nothing: &str) -> Result<(T, Vec<CommandWarning>), String> {
    match r {
        CommandResult::Completed { output, warnings } => Ok((output, warnings)),
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Err(nothing.to_owned()),
    }
}

impl App {
    pub(super) fn region_open(&mut self) -> Task<Message> {
        let shape = shape_of(self);
        self.open_window(Window::CloudRegion(RegionState {
            clouds: clouds_of(self),
            units: self.area_units(),
            shape,
            result: None,
            running: false,
        }));
        Task::none()
    }

    pub(super) fn profile_open(&mut self) -> Task<Message> {
        let path = path_of(self);
        self.open_window(Window::CloudProfile(ProfileState {
            clouds: clouds_of(self),
            units: self.area_units(),
            path,
            width: "10".into(),
            step: "25".into(),
            points: false,
            result: None,
            running: false,
        }));
        Task::none()
    }

    pub(super) fn mesh_query_open(&mut self) -> Task<Message> {
        let (x, y) = place_of(self).unwrap_or_default();
        let (shape, path) = (shape_of(self), path_of(self));
        // The selection says what is asked: an area, a line, else a place.
        let mode = if shape.is_ok() {
            MeshMode::Area
        } else if path.is_ok() {
            MeshMode::Path
        } else {
            MeshMode::Place
        };
        self.open_window(Window::MeshQuery(MeshState {
            units: self.area_units(),
            meshes: meshes_of(self),
            mode,
            x,
            y,
            shape,
            path,
            step: "10".into(),
            faces: false,
            time: String::new(),
            series: false,
            result: None,
            running: false,
        }));
        Task::none()
    }

    /// The project's units, as the answers are written; a geographic project's plan in degrees (GIS-041).
    fn area_units(&self) -> Units {
        let f = self.document.as_ref().map_or_else(Format::default, |d| Format::of(d.settings()));
        let degrees = self
            .document
            .as_ref()
            .and_then(|d| kentos_project::systems::own(d.settings()))
            .is_some_and(|n| n.geographic());
        Units { f, degrees }
    }

    /// Back from Sahneden seç (window_pick.rs): the area or the line read from the selection again.
    pub(crate) fn area_after_pick(&mut self) {
        if matches!(self.exchange, Some(Window::CloudRegion(_) | Window::CloudProfile(_) | Window::MeshQuery(_))) {
            let _ = self.area_event(Event::Use);
        }
    }

    pub(super) fn area_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Pick => {
                let what = match &self.exchange {
                    Some(Window::CloudProfile(_)) => "Profil yolu",
                    Some(Window::MeshQuery(s)) if s.mode == MeshMode::Path => "Hat",
                    Some(Window::MeshQuery(s)) if s.mode == MeshMode::Place => "Yer (nokta)",
                    _ => "Alan",
                };
                self.pick_for_window(crate::app::Dialog::Exchange, what);
                Task::none()
            }
            Event::Cloud(i) => {
                match &mut self.exchange {
                    Some(Window::CloudRegion(s)) => {
                        s.clouds.chosen = i;
                        s.result = None;
                    }
                    Some(Window::CloudProfile(s)) => {
                        s.clouds.chosen = i;
                        s.result = None;
                    }
                    // Mesh sor's mesh (docs/adr/9335): all, or one of them.
                    Some(Window::MeshQuery(s)) => {
                        s.meshes.chosen = i;
                        s.result = None;
                    }
                    _ => {}
                }
                Task::none()
            }
            Event::Use => {
                match &mut self.exchange {
                    Some(Window::CloudRegion(_)) => {
                        let shape = shape_of(self);
                        if let Some(Window::CloudRegion(s)) = &mut self.exchange {
                            s.shape = shape;
                            s.result = None;
                        }
                    }
                    Some(Window::CloudProfile(_)) => {
                        let path = path_of(self);
                        if let Some(Window::CloudProfile(s)) = &mut self.exchange {
                            s.path = path;
                            s.result = None;
                        }
                    }
                    Some(Window::MeshQuery(_)) => {
                        let (at, shape, path) = (place_of(self), shape_of(self), path_of(self));
                        if let Some(Window::MeshQuery(s)) = &mut self.exchange {
                            match s.mode {
                                MeshMode::Place => {
                                    if let Some((x, y)) = at {
                                        s.x = x;
                                        s.y = y;
                                    }
                                }
                                MeshMode::Area => s.shape = shape,
                                MeshMode::Path => s.path = path,
                            }
                            s.result = None;
                        }
                    }
                    _ => {}
                }
                Task::none()
            }
            Event::Width(t) => {
                if let Some(Window::CloudProfile(s)) = &mut self.exchange {
                    s.width = t;
                }
                Task::none()
            }
            Event::Step(t) => {
                match &mut self.exchange {
                    Some(Window::CloudProfile(s)) => s.step = t,
                    Some(Window::MeshQuery(s)) => s.step = t,
                    _ => {}
                }
                Task::none()
            }
            Event::Mode(m) => {
                if let Some(Window::MeshQuery(s)) = &mut self.exchange
                    && s.mode != m
                {
                    s.mode = m;
                    s.result = None;
                }
                Task::none()
            }
            Event::X(t) => {
                if let Some(Window::MeshQuery(s)) = &mut self.exchange {
                    s.x = t;
                }
                Task::none()
            }
            Event::Y(t) => {
                if let Some(Window::MeshQuery(s)) = &mut self.exchange {
                    s.y = t;
                }
                Task::none()
            }
            Event::Time(t) => {
                if let Some(Window::MeshQuery(s)) = &mut self.exchange {
                    s.time = t;
                }
                Task::none()
            }
            Event::Toggle => {
                match &mut self.exchange {
                    Some(Window::CloudProfile(s)) => s.points = !s.points,
                    Some(Window::MeshQuery(s)) if s.mode == MeshMode::Area => s.faces = !s.faces,
                    Some(Window::MeshQuery(s)) => s.series = !s.series,
                    _ => {}
                }
                Task::none()
            }
            Event::Run => self.area_run(),
            Event::Done(outcome) => {
                match (&mut self.exchange, *outcome) {
                    (Some(Window::CloudRegion(s)), o @ Outcome::Region(_)) => {
                        s.result = Some(o);
                        s.running = false;
                    }
                    (Some(Window::CloudProfile(s)), o @ Outcome::Profile(_)) => {
                        s.result = Some(o);
                        s.running = false;
                    }
                    (
                        Some(Window::MeshQuery(s)),
                        o @ (Outcome::Mesh(_) | Outcome::MeshRegion(_) | Outcome::MeshProfile(_)),
                    ) => {
                        s.result = Some(o);
                        s.running = false;
                    }
                    _ => {}
                }
                Task::none()
            }
            Event::Copy => {
                let text = match &self.exchange {
                    Some(Window::CloudRegion(s)) => s.result.as_ref().map(|r| copy_text(r, &s.units)),
                    Some(Window::CloudProfile(s)) => s.result.as_ref().map(|r| copy_text(r, &s.units)),
                    Some(Window::MeshQuery(s)) => s.result.as_ref().map(|r| copy_text(r, &s.units)),
                    _ => None,
                };
                text.map_or_else(Task::none, iced::clipboard::write)
            }
        }
    }

    fn area_run(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let model = doc.model.clone();
        let drawing = doc
            .path
            .as_deref()
            .and_then(|p| p.to_str())
            .map(str::to_owned);
        let done = |o: Outcome| Exchange::Area(Event::Done(Box::new(o)));
        match &mut self.exchange {
            Some(Window::CloudRegion(s)) => {
                let Ok(shape) = &s.shape else {
                    return Task::none();
                };
                if s.clouds.all.is_empty() {
                    return Task::none();
                }
                let input = PointCloudsRegion {
                    ring: shape.ring.clone(),
                    holes: shape.holes.clone(),
                    uids: s.clouds.uids(),
                    drawing,
                };
                s.running = true;
                off_thread(
                    move || {
                        let mut model = model;
                        Outcome::Region(answered(
                            pointcloud_areas::region::plan(
                                &ExecutionContext::new(&mut model),
                                &input,
                            ),
                            "Alan okunamadı.",
                        ))
                    },
                    done,
                )
            }
            Some(Window::CloudProfile(s)) => {
                let Ok((path, _)) = &s.path else {
                    return Task::none();
                };
                if s.clouds.all.is_empty() {
                    return Task::none();
                }
                let (width, step) = (parse(&s.width), parse(&s.step));
                let (Some(width), Some(step)) = (width, step) else {
                    s.result = Some(Outcome::Profile(Err(
                        "Genişlik ve adım sayı olmalı (örneğin 10 ve 25).".into(),
                    )));
                    return Task::none();
                };
                let input = PointCloudsProfile {
                    path: path.clone(),
                    width,
                    step,
                    points: s.points,
                    uids: s.clouds.uids(),
                    drawing,
                };
                s.running = true;
                off_thread(
                    move || {
                        let mut model = model;
                        Outcome::Profile(answered(
                            pointcloud_areas::profile::plan(
                                &ExecutionContext::new(&mut model),
                                &input,
                            ),
                            "Profil okunamadı.",
                        ))
                    },
                    done,
                )
            }
            Some(Window::MeshQuery(s)) => {
                let time = if s.time.trim().is_empty() {
                    None
                } else {
                    match kentos_geometry_core::time::read(s.time.trim()).moment() {
                        Some(t) => Some(t),
                        None => {
                            s.result = Some(Outcome::Mesh(Err(
                                "An okunamadı; 2024-05-01 06:30 gibi yazın.".into(),
                            )));
                            return Task::none();
                        }
                    }
                };
                match s.mode {
                    MeshMode::Area => {
                        let Ok(shape) = &s.shape else {
                            return Task::none();
                        };
                        let input = MeshesRegion {
                            ring: shape.ring.clone(),
                            holes: shape.holes.clone(),
                            uids: s.meshes.uids(),
                            time,
                            variables: Vec::new(),
                            faces: s.faces,
                            drawing,
                        };
                        s.running = true;
                        return off_thread(
                            move || {
                                let mut model = model;
                                Outcome::MeshRegion(answered(
                                    mesh_areas::region::plan(
                                        &ExecutionContext::new(&mut model),
                                        &input,
                                    ),
                                    "Mesh okunamadı.",
                                ))
                            },
                            done,
                        );
                    }
                    MeshMode::Path => {
                        let Ok((path, _)) = &s.path else {
                            return Task::none();
                        };
                        let Some(step) = parse(&s.step) else {
                            s.result = Some(Outcome::MeshProfile(Err(
                                "Adım sayı olmalı (örneğin 10).".into(),
                            )));
                            return Task::none();
                        };
                        let input = MeshesProfile {
                            path: path.clone(),
                            step,
                            uids: s.meshes.uids(),
                            time,
                            variables: Vec::new(),
                            drawing,
                        };
                        s.running = true;
                        return off_thread(
                            move || {
                                let mut model = model;
                                Outcome::MeshProfile(answered(
                                    mesh_areas::profile::plan(
                                        &ExecutionContext::new(&mut model),
                                        &input,
                                    ),
                                    "Mesh okunamadı.",
                                ))
                            },
                            done,
                        );
                    }
                    MeshMode::Place => {}
                }
                let (x, y) = (parse(&s.x), parse(&s.y));
                let (Some(x), Some(y)) = (x, y) else {
                    s.result = Some(Outcome::Mesh(Err("Yerin Y'si (doğu) ve X'i (kuzey) sayı olmalı.".into())));
                    return Task::none();
                };
                let input = MeshesQuery {
                    at: Vec2 { x, y },
                    uids: s.meshes.uids(),
                    time,
                    variables: Vec::new(),
                    series: s.series,
                    drawing,
                };
                s.running = true;
                off_thread(
                    move || {
                        let mut model = model;
                        Outcome::Mesh(answered(
                            meshes::query::plan(&ExecutionContext::new(&mut model), &input),
                            "Mesh okunamadı.",
                        ))
                    },
                    done,
                )
            }
            _ => Task::none(),
        }
    }

    pub(super) fn region_view<'a>(&'a self, s: &'a RegionState) -> Element<'a, Message> {
        let mut body = Column::new().spacing(12);
        let (ready, said) = match &s.shape {
            Ok(shape) => (true, words::text_line(Line::Info, shape.said.clone())),
            Err(why) => (false, words::text_line(Line::Warn, why.clone())),
        };
        let (has_cloud, cloud) = cloud_field(&s.clouds);
        body = body.push(cloud);
        body = body.push(words::field(
            "Alan",
            said,
            Some("Alanın içindeki nokta bulutu noktaları kaynak dosyalarının tam çözünürlüğünde okunur; sınırdaki nokta içeridedir.".into()),
        ));
        body = body.push(pick_row());
        body = body.push(result_view(s.result.as_ref(), s.running, &s.units));
        window(
            "Nokta bulutu alan istatistiği",
            body,
            ready && has_cloud && !s.running,
            s.result.is_some(),
        )
    }

    pub(super) fn profile_view<'a>(&'a self, s: &'a ProfileState) -> Element<'a, Message> {
        let mut body = Column::new().spacing(12);
        let (ready, said) = match &s.path {
            Ok((_, said)) => (true, words::text_line(Line::Info, said.clone())),
            Err(why) => (false, words::text_line(Line::Warn, why.clone())),
        };
        let (has_cloud, cloud) = cloud_field(&s.clouds);
        body = body.push(cloud);
        body = body.push(words::field("Yol", said, None));
        body = body.push(pick_row());
        body = body.push(words::field(
            "Koridorun genişliği, m (iki yanda toplam)",
            input("10", &s.width, Event::Width),
            None,
        ));
        body = body.push(words::field(
            "İstasyon adımı, m (yol boyunca)",
            input("25", &s.step, Event::Step),
            None,
        ));
        body = body.push(words::check(
            s.points,
            "Noktaları da listele (istasyon, ofset, yer, sınıf)",
            Some(event(Event::Toggle)),
        ));
        body = body.push(result_view(s.result.as_ref(), s.running, &s.units));
        window(
            "Nokta bulutu profili",
            body,
            ready && has_cloud && !s.running,
            s.result.is_some(),
        )
    }

    pub(super) fn mesh_view<'a>(&'a self, s: &'a MeshState) -> Element<'a, Message> {
        let mut body = Column::new().spacing(12);
        // The mesh read first: without one, why and the way to add one (docs/adr/9335).
        let (has_mesh, mesh) = mesh_field(&s.meshes);
        body = body.push(mesh);
        body = body.push(words::field(
            "Sorulan",
            Segmented::new(
                [MeshMode::Place, MeshMode::Area, MeshMode::Path],
                s.mode,
                |m| event(Event::Mode(m)),
            ),
            Some(match s.mode {
                MeshMode::Place => "Bir yerin yüzü ve değerleri.".to_owned(),
                MeshMode::Area => "Merkezi alanın içindeki yüzler ve veri setlerinin istatistiği (yüz alanlarıyla ağırlıklı ortalama, değer × alan toplamı).".to_owned(),
                MeshMode::Path => "Yol boyunca adım adım istasyonların yüzü ve değerleri.".to_owned(),
            }),
        ));
        let ready = match s.mode {
            MeshMode::Place => {
                // Y (doğu) before X (kuzey), as Ölçme writes a place (docs/adr/9335).
                body = body.push(words::field(
                    "Y (doğu)",
                    input("500012", &s.x, Event::X),
                    None,
                ));
                body = body.push(words::field(
                    "X (kuzey)",
                    input("4420010", &s.y, Event::Y),
                    None,
                ));
                body = body.push(words::ghost("Seçili noktadan al", Some(event(Event::Use))));
                parse(&s.x).is_some() && parse(&s.y).is_some()
            }
            MeshMode::Area => {
                let (ok, said) = match &s.shape {
                    Ok(shape) => (true, words::text_line(Line::Info, shape.said.clone())),
                    Err(why) => (false, words::text_line(Line::Warn, why.clone())),
                };
                body = body.push(words::field("Alan", said, None));
                body = body.push(words::ghost("Seçimden al", Some(event(Event::Use))));
                ok
            }
            MeshMode::Path => {
                let (ok, said) = match &s.path {
                    Ok((_, said)) => (true, words::text_line(Line::Info, said.clone())),
                    Err(why) => (false, words::text_line(Line::Warn, why.clone())),
                };
                body = body.push(words::field("Yol", said, None));
                body = body.push(words::ghost("Seçimden al", Some(event(Event::Use))));
                body = body.push(words::field(
                    "İstasyon adımı, m (yol boyunca)",
                    input("10", &s.step, Event::Step),
                    None,
                ));
                ok
            }
        };
        body = body.push(words::field(
            "An (isteğe bağlı)",
            input("2024-05-01 06:30", &s.time, Event::Time),
            Some("Boşsa rasterin gösterdiği adımlar; verilirse her veri setinin o andan önceki son adımı.".into()),
        ));
        match s.mode {
            MeshMode::Place => {
                body = body.push(words::check(
                    s.series,
                    "Zaman boyutlu veri setlerinin her adımını göster",
                    Some(event(Event::Toggle)),
                ));
            }
            MeshMode::Area => {
                body = body.push(words::check(
                    s.faces,
                    "Yüzlerin sırasını da listele",
                    Some(event(Event::Toggle)),
                ));
            }
            MeshMode::Path => {}
        }
        body = body.push(result_view(s.result.as_ref(), s.running, &s.units));
        window("Mesh sor", body, ready && has_mesh && !s.running, s.result.is_some())
    }
}

fn parse(t: &str) -> Option<f64> {
    t.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

fn input<'a>(hint: &str, value: &str, on: fn(String) -> Event) -> Element<'a, Message> {
    kentos_ui::widget::text_field(hint, value)
        .on_input(move |t| event(on(t)))
        .width(Fill)
        .into()
}

fn window<'a>(
    title: &'a str,
    body: Column<'a, Message>,
    ready: bool,
    copyable: bool,
) -> Element<'a, Message> {
    overlay::blocking(
        Dialog::new(title)
            .scroll(body)
            .action(words::secondary("Kapat", Some(message(Exchange::Close))))
            .action(words::secondary(
                "Panoya kopyala",
                copyable.then(|| event(Event::Copy)),
            ))
            .action(words::primary("Çalıştır", ready.then(|| event(Event::Run))))
            .width(700.0)
            .max_height(760.0),
    )
}

/// Sahneden seç and Seçimden al side by side.
fn pick_row<'a>() -> Element<'a, Message> {
    row![
        words::secondary("Sahneden seç", Some(event(Event::Pick))),
        words::ghost("Seçimden al", Some(event(Event::Use))),
    ]
    .spacing(8)
    .into()
}

/// Bulut: none said with the way to one, one named, more chosen from (Tüm bulutlar first); whether one is there.
fn cloud_field(c: &Clouds) -> (bool, Element<'_, Message>) {
    let body: Element<'_, Message> = match c.all.len() {
        0 => words::text_line(Line::Warn, "Çizimde nokta bulutu yok: önce Ekle › Nokta bulutu ile bir bulut ekleyin."),
        1 => words::text_line(Line::Info, c.all[0].1.clone()),
        n => Select::new(
            std::iter::once(Choice::new(format!("Tüm bulutlar ({n})")))
                .chain(c.all.iter().map(|(_, name)| Choice::new(name.clone()))),
            Some(c.chosen),
            |i| event(Event::Cloud(i)),
        )
        .into(),
    };
    (!c.all.is_empty(), words::field("Nokta bulutu", body, None))
}

/// The meshes Mesh sor reads (docs/adr/9335): with none, why Çalıştır waits and Mesh ekle…; with one, its name; with
/// more, Tüm meshler or one of them.
fn mesh_field(m: &Clouds) -> (bool, Element<'_, Message>) {
    let body: Element<'_, Message> = match m.all.len() {
        0 => Column::new()
            .spacing(6)
            .push(words::text_line(
                Line::Warn,
                "Projede mesh yok: sorulacak bir mesh olmadan Çalıştır bekler. UGRID NetCDF ya da 2DM dosyasını Mesh \
                 ekle ile ekleyin.",
            ))
            .push(words::secondary("Mesh ekle…", Some(Message::Run("mesh.add"))))
            .into(),
        1 => words::text_line(Line::Info, m.all[0].1.clone()),
        n => Select::new(
            std::iter::once(Choice::new(format!("Tüm meshler ({n})")))
                .chain(m.all.iter().map(|(_, name)| Choice::new(name.clone()))),
            Some(m.chosen),
            |i| event(Event::Cloud(i)),
        )
        .into(),
    };
    (!m.all.is_empty(), words::field("Mesh", body, None))
}

fn result_view<'a>(r: Option<&'a Outcome>, running: bool, f: &Units) -> Element<'a, Message> {
    if running {
        return words::summary(vec![words::text_line(Line::Info, "Dosyalar okunuyor…")]);
    }
    let Some(r) = r else {
        return words::summary(Vec::new());
    };
    let mut lines = Vec::new();
    let warn = |warnings: &[CommandWarning], lines: &mut Vec<Element<'_, Message>>| {
        for w in warnings {
            lines.push(words::text_line(Line::Warn, w.message.clone()));
        }
    };
    match r {
        Outcome::Region(Err(why))
        | Outcome::Profile(Err(why))
        | Outcome::Mesh(Err(why))
        | Outcome::MeshRegion(Err(why))
        | Outcome::MeshProfile(Err(why)) => {
            lines.push(words::text_line(Line::Error, why.clone()));
        }
        Outcome::MeshRegion(Ok((o, warnings))) => {
            lines.extend(texts_lines(mesh_region_texts(o, f)));
            warn(warnings, &mut lines);
        }
        Outcome::MeshProfile(Ok((o, warnings))) => {
            lines.extend(texts_lines(mesh_profile_texts(o, f)));
            warn(warnings, &mut lines);
        }
        Outcome::Region(Ok((o, warnings))) => {
            lines.extend(region_lines(o, f));
            warn(warnings, &mut lines);
        }
        Outcome::Profile(Ok((o, warnings))) => {
            lines.extend(profile_lines(o, f));
            warn(warnings, &mut lines);
        }
        Outcome::Mesh(Ok((o, warnings))) => {
            lines.extend(mesh_lines(o, f));
            warn(warnings, &mut lines);
        }
    }
    words::summary(lines)
}

fn stats_text(s: &kentos_commands::CloudStats) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(z) = &s.z {
        out.push(format!(
            "Yükseklik: en az {}, en çok {}, ortalama {}",
            fixed(z.min),
            fixed(z.max),
            fixed(z.mean)
        ));
    }
    if let Some(i) = &s.intensity {
        out.push(format!(
            "Yoğunluk: en az {}, en çok {}, ortalama {}",
            fixed(i.min),
            fixed(i.max),
            fixed(i.mean)
        ));
    }
    if !s.classes.is_empty() {
        let mut classes: Vec<(&String, &u64)> = s.classes.iter().collect();
        classes.sort_by_key(|(c, _)| c.parse::<u32>().unwrap_or(u32::MAX));
        let said: Vec<String> = classes.iter().map(|(c, n)| format!("{c}: {n}")).collect();
        out.push(format!("Sınıflar: {}", said.join(", ")));
    }
    out
}

fn region_texts(o: &PointCloudsRegioned, f: &Units) -> Vec<String> {
    // The area in the project's unit, the density per its square (GIS-036, GIS-041).
    let mut out = vec![format!(
        "Alan {}; {} nokta{}; {} nokta okundu.",
        f.area(o.area),
        grouped(o.total.count),
        o.density
            .map_or_else(String::new, |d| format!(" ({} {})", density(d), f.per_area())),
        grouped(o.read)
    )];
    out.extend(stats_text(&o.total));
    if o.clouds.len() > 1 {
        for (i, c) in o.clouds.iter().enumerate() {
            out.push(format!(
                "{}. bulut: {} nokta, {} okundu",
                i + 1,
                c.stats.count,
                c.read
            ));
        }
    }
    out
}

fn region_lines(o: &PointCloudsRegioned, f: &Units) -> Vec<Element<'static, Message>> {
    let texts = region_texts(o, f);
    let kind = if o.total.count == 0 {
        Line::Warn
    } else {
        Line::Ok
    };
    texts
        .into_iter()
        .enumerate()
        .map(|(i, t)| words::text_line(if i == 0 { kind } else { Line::Info }, t))
        .collect()
}

fn profile_texts(o: &PointCloudsProfiled, f: &Units) -> Vec<String> {
    let mut out = vec![format!(
        "Yol {}; koridorda {} nokta; {} nokta okundu; {} istasyon.",
        f.plan(o.length),
        grouped(o.count),
        grouped(o.read),
        o.bins.len()
    )];
    for b in &o.bins {
        out.push(match &b.z {
            Some(z) => format!(
                "{} – {}: {} nokta, yükseklik {} … {} (ort. {})",
                f.plan_bare(b.from),
                f.plan(b.to),
                grouped(b.count),
                f.f.length_bare(z.min),
                f.f.length(z.max),
                f.f.length(z.mean)
            ),
            None => format!("{} – {}: nokta yok", f.plan_bare(b.from), f.plan(b.to)),
        });
    }
    out
}

fn profile_lines<'a>(o: &'a PointCloudsProfiled, f: &Units) -> Vec<Element<'a, Message>> {
    let texts = profile_texts(o, f);
    let more = texts.len().saturating_sub(SHOWN + 1);
    let mut lines: Vec<Element<'_, Message>> = texts
        .into_iter()
        .take(SHOWN + 1)
        .enumerate()
        .map(|(i, t)| words::text_line(if i == 0 { Line::Ok } else { Line::Info }, t))
        .collect();
    if more > 0 {
        lines.push(words::text_line(
            Line::Info,
            format!("… {more} istasyon daha (Panoya kopyala hepsini alır)."),
        ));
    }
    if !o.points.is_empty() {
        lines.push(words::text_line(
            Line::Info,
            format!(
                "{} nokta listelendi (Panoya kopyala satır satır alır).",
                o.points.len()
            ),
        ));
    }
    lines
}

fn mesh_texts(o: &MeshesQueried, u: &Units) -> Vec<String> {
    let Some(f) = &o.face else {
        return vec!["Yer hiçbir meshin yüzünde değil.".to_owned()];
    };
    let nodes: Vec<String> = f.nodes.iter().map(|n| (n + 1).to_string()).collect();
    let mut out = vec![format!(
        "Yüz {} (“{}” ağı, düğümler {}); alanı {}; {} değer okundu.",
        f.face + 1,
        f.topology,
        nodes.join("-"),
        u.area(f.area),
        o.read
    )];
    if !f.neighbours.is_empty() {
        let n: Vec<String> = f.neighbours.iter().map(|k| (k + 1).to_string()).collect();
        out.push(format!("Komşu yüzler: {}", n.join(", ")));
    }
    for v in &o.values {
        let name = match &v.vector {
            Some(y) => format!("{} / {y}", v.variable),
            None => v.variable.clone(),
        };
        let unit = v
            .units
            .as_deref()
            .map_or_else(String::new, |u| format!(" {u}"));
        let at = v
            .time
            .map(|t| {
                format!(
                    " ({})",
                    kentos_formats::multidim::cube::dim_labels(&[t], true, None)
                        .pop()
                        .unwrap_or_default()
                )
            })
            .unwrap_or_default();
        out.push(match v.value {
            Some(x) => format!("{name} [{}]: {}{unit}{at}", v.location, fixed(x)),
            None => format!("{name} [{}]: değer yok{at}", v.location),
        });
        for s in &v.series {
            out.push(match s.value {
                Some(x) => format!("    {}: {}{unit}", s.label, fixed(x)),
                None => format!("    {}: değer yok", s.label),
            });
        }
    }
    out
}

fn named(z: &MeshZonal) -> String {
    match &z.vector {
        Some(y) => format!("{} / {y}", z.variable),
        None => z.variable.clone(),
    }
}

fn moment_of(z: &MeshZonal) -> String {
    z.time
        .map(|t| {
            format!(
                " ({})",
                kentos_formats::multidim::cube::dim_labels(&[t], true, None)
                    .pop()
                    .unwrap_or_default()
            )
        })
        .unwrap_or_default()
}

fn mesh_region_texts(o: &MeshesRegioned, u: &Units) -> Vec<String> {
    let mut out = vec![format!("Alan {}; {} değer okundu.", u.area(o.area), o.read)];
    if o.meshes.is_empty() {
        out.push("Alanda mesh yok.".to_owned());
    }
    for m in &o.meshes {
        out.push(format!(
            "“{}” ağı: {} yüz, alanları toplamı {}",
            m.topology,
            m.count,
            u.area(m.area)
        ));
        for z in &m.datasets {
            let unit = z
                .units
                .as_deref()
                .map_or_else(String::new, |u| format!(" {u}"));
            out.push(match (z.min, z.max, z.mean) {
                (Some(lo), Some(hi), Some(mean)) => format!(
                    "    {} [{}]{}: {} yüzde en az {}, en çok {}, ortalama {}{unit}; toplam (değer × alan) {}",
                    named(z),
                    z.location,
                    moment_of(z),
                    z.count,
                    fixed(lo),
                    fixed(hi),
                    fixed(mean),
                    z.integral.map_or_else(String::new, fixed)
                ),
                _ => format!("    {} [{}]{}: değer yok", named(z), z.location, moment_of(z)),
            });
        }
        if !m.faces.is_empty() {
            let ids: Vec<String> = m.faces.iter().map(|f| (f + 1).to_string()).collect();
            out.push(format!("    Yüzler: {}", ids.join(", ")));
        }
    }
    out
}

fn mesh_profile_texts(o: &MeshesProfiled, u: &Units) -> Vec<String> {
    let mut out = vec![format!("Yol {}; {} değer okundu.", u.plan(o.length), o.read)];
    for m in &o.meshes {
        let heads: Vec<String> = m
            .datasets
            .iter()
            .map(|z| format!("{}{}", named(z), moment_of(z)))
            .collect();
        out.push(format!(
            "“{}” ağı: {} istasyon; {}",
            m.topology,
            m.stations.len(),
            heads.join(", ")
        ));
        for st in &m.stations {
            let values: Vec<String> = st
                .values
                .iter()
                .map(|v| v.map_or_else(|| "—".to_owned(), fixed))
                .collect();
            out.push(match st.face {
                Some(f) => format!(
                    "    {}: yüz {}; {}",
                    u.plan_bare(st.station),
                    f + 1,
                    values.join(" · ")
                ),
                None => format!("    {}: meshin dışında", u.plan_bare(st.station)),
            });
        }
    }
    out
}

fn texts_lines<'a>(texts: Vec<String>) -> Vec<Element<'a, Message>> {
    let more = texts.len().saturating_sub(SHOWN + 1);
    let mut lines: Vec<Element<'a, Message>> = texts
        .into_iter()
        .take(SHOWN + 1)
        .enumerate()
        .map(|(i, t)| words::text_line(if i == 0 { Line::Ok } else { Line::Info }, t))
        .collect();
    if more > 0 {
        lines.push(words::text_line(
            Line::Info,
            format!("… {more} satır daha (Panoya kopyala hepsini alır)."),
        ));
    }
    lines
}

fn mesh_lines<'a>(o: &'a MeshesQueried, u: &Units) -> Vec<Element<'a, Message>> {
    mesh_texts(o, u)
        .into_iter()
        .take(SHOWN + 1)
        .enumerate()
        .map(|(i, t)| {
            words::text_line(
                if i == 0 {
                    if o.face.is_some() {
                        Line::Ok
                    } else {
                        Line::Warn
                    }
                } else {
                    Line::Info
                },
                t,
            )
        })
        .collect()
}

/// The answer as text for the clipboard: the lines, a profile's points as tab separated rows.
pub(super) fn copy_text(r: &Outcome, f: &Units) -> String {
    match r {
        Outcome::Region(Ok((o, _))) => region_texts(o, f).join("\n"),
        Outcome::Profile(Ok((o, _))) => {
            let mut t = profile_texts(o, f).join("\n");
            if !o.points.is_empty() {
                t.push_str("\nİstasyon\tOfset\tY\tX\tZ\tSınıf\n");
                for p in &o.points {
                    let _ = writeln!(
                        t,
                        "{}\t{}\t{}\t{}\t{}\t{}",
                        fixed(p.station),
                        fixed(p.offset),
                        fixed(p.x),
                        fixed(p.y),
                        fixed(p.z),
                        p.class
                    );
                }
            }
            t
        }
        Outcome::Mesh(Ok((o, _))) => mesh_texts(o, f).join("\n"),
        Outcome::MeshRegion(Ok((o, _))) => mesh_region_texts(o, f).join("\n"),
        Outcome::MeshProfile(Ok((o, _))) => mesh_profile_texts(o, f).join("\n"),
        Outcome::Region(Err(why))
        | Outcome::Profile(Err(why))
        | Outcome::Mesh(Err(why))
        | Outcome::MeshRegion(Err(why))
        | Outcome::MeshProfile(Err(why)) => why.clone(),
    }
}
