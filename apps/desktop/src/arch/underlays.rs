//! Altlık modeller on the drawing (docs/adr/8101; plan/ifc-uyum.md IFC-09, B18): each underlay's file read off the
//! window's thread — checked against the digest the drawing keeps, its bodies made by the reading engine and placed by
//! the same rule the command placed it with (`kentos_ifc::underlay::place`) — then its plan at the active level for
//! the drawing area (cut at the level's cut height, the edges under it down to its floor, faint, under the drawing),
//! its bodies for 3B görünüm, and the object a click finds. Nothing of it is written into the drawing: a file that
//! changed on disk is said, not shown, until Yeniden oku takes its new revision.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::Task;
use iced::futures::channel::oneshot;
use kentos_commands::encoding::sha256_hex;
use kentos_commands::{ModelUnderlay, UnderlayPlacement};
use kentos_ifc::read::{self, Model};
use kentos_ifc::underlay::{self, MapFrame, Mesh, PlanLine};

use crate::app::{App, Message};
use crate::arch;

/// One body placed in the drawing's metres: its object (an index into the file's objects), its corners, its triangles.
pub(crate) struct Body {
    pub owner: u32,
    pub positions: Arc<Vec<[f64; 3]>>,
    pub triangles: Vec<[u32; 3]>,
}

/// An underlay's file as read: its digest and schema, the model and its bytes (an object's details are read from
/// them when asked), the bodies placed, the objects with a shape that have no body and why, how long the read took,
/// and the bodies' box in the drawing (x, y, z from, x, y, z to).
pub(crate) struct Loaded {
    pub sha256: String,
    pub schema: String,
    pub model: Model,
    pub bytes: Vec<u8>,
    pub bodies: Vec<Body>,
    pub not_shown: Vec<(usize, String)>,
    pub took: Duration,
    pub bounds: Option<[f64; 6]>,
}

impl std::fmt::Debug for Loaded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Loaded")
            .field("schema", &self.schema)
            .field("objects", &self.model.objects.len())
            .field("bodies", &self.bodies.len())
            .field("not_shown", &self.not_shown.len())
            .field("took", &self.took)
            .finish_non_exhaustive()
    }
}

impl Loaded {
    /// The objects with a shape: those with a body and those without.
    pub(crate) fn with_shape(&self) -> usize {
        self.bodies.len() + self.not_shown.len()
    }

    /// What its bodies and the file take in memory, about, in bytes: the file, the corners and triangles.
    pub(crate) fn memory(&self) -> usize {
        self.bytes.len()
            + self.bodies.iter().map(|b| b.positions.len() * 24 + b.triangles.len() * 12).sum::<usize>()
    }

    /// The object `owner` as the list says it: its class and its name.
    pub(crate) fn object_words(&self, owner: usize) -> String {
        match self.model.objects.get(owner) {
            Some(o) => match o.name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
                Some(n) => format!("{} {n}", o.class),
                None => o.class.clone(),
            },
            None => format!("#{owner}"),
        }
    }
}

/// Where an underlay's read stands.
#[derive(Clone, Debug)]
pub(crate) enum State {
    Reading,
    Ready(Arc<Loaded>),
    Failed(String),
}

/// What an underlay was read for: its file, digest and placement and the project's system; a change of any reads it
/// again.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Key {
    pub file: String,
    pub sha256: String,
    pub placement: UnderlayPlacement,
    pub project: Result<MapFrame, String>,
}

/// The underlays' reads and the plan last given to the drawing area.
#[derive(Default)]
pub(crate) struct Underlays {
    pub states: HashMap<String, (Key, State)>,
    /// The plan's key (the underlays drawn, by id and digest, and the cut and floor heights' bits) and its lines.
    plan: Option<(Vec<(String, String, u64, u64)>, Arc<Vec<PlanLine>>)>,
    /// One more each time the plan changes.
    pub version: u64,
}

impl std::fmt::Debug for Underlays {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Underlays").field("states", &self.states.len()).field("version", &self.version).finish()
    }
}

/// The file `file` read, checked against `sha256` and placed `placement` on `project`; why not, in the interface's
/// words.
pub(crate) fn load(file: &str, sha256: &str, placement: UnderlayPlacement, project: Result<MapFrame, String>) -> Result<Loaded, String> {
    let started = Instant::now();
    let bytes = std::fs::read(file)
        .map_err(|_| format!("{file} okunamadı: dosya yok ya da açılamıyor. Yerini Yeniden oku ile verin."))?;
    let digest = sha256_hex(&bytes);
    if digest != sha256 {
        return Err("Dosya eklendiğinden beri değişmiş (SHA-256'sı başka): gösterilen revizyon o değil. Yeniden oku \
                    yeni revizyonu alır."
            .to_owned());
    }
    let model = read::read(&bytes, &read::Limits::default(), &|| false).map_err(|e| e.to_string())?;
    let unit = model.summary.units.length.as_ref().map_or(f64::NAN, |u| u.to_si);
    let frame = model.summary.location.as_ref().and_then(read::Location::frame);
    let core = match placement {
        UnderlayPlacement::Map => underlay::Placement::Map,
        UnderlayPlacement::Origin => underlay::Placement::Origin,
    };
    let t = underlay::place(frame.as_ref(), unit, project.as_ref().map_err(String::as_str), core)?;
    let made = model.bodies(&bytes, &|| false).map_err(|e| e.to_string())?;
    let mut bounds: Option<[f64; 6]> = None;
    let bodies: Vec<Body> = made
        .bodies
        .into_iter()
        .map(|b| {
            let positions: Vec<[f64; 3]> = b.positions.iter().map(|p| t.apply(*p)).collect();
            for p in &positions {
                let k = bounds.get_or_insert([p[0], p[1], p[2], p[0], p[1], p[2]]);
                for i in 0..3 {
                    k[i] = k[i].min(p[i]);
                    k[i + 3] = k[i + 3].max(p[i]);
                }
            }
            Body { owner: u32::try_from(b.object).unwrap_or(u32::MAX), positions: Arc::new(positions), triangles: b.triangles }
        })
        .collect();
    let not_shown = made.not_shown.into_iter().map(|n| (n.object, n.reason)).collect();
    Ok(Loaded { sha256: digest, schema: model.summary.schema.clone(), model, bytes, bodies, not_shown, took: started.elapsed(), bounds })
}

/// The plan of `loaded` at a level: cut at `cut`, the edges under it down to `bottom`.
pub(crate) fn plan_of(loaded: &Loaded, cut: f64, bottom: f64) -> Vec<PlanLine> {
    let meshes: Vec<Mesh<'_>> = loaded
        .bodies
        .iter()
        .map(|b| Mesh { owner: b.owner, positions: &b.positions, triangles: &b.triangles })
        .collect();
    underlay::plan(&meshes, cut, bottom)
}

impl App {
    /// The project's system as its IFC files say it, an underlay is placed on; why it has none.
    fn underlay_project(&self) -> Result<MapFrame, String> {
        let doc = self.document.as_ref().ok_or_else(|| "açık çizim yok".to_owned())?;
        kentos_native_application::arch_ifc::crs(&doc.model).map(|c| MapFrame::from(&c))
    }

    /// The level the plan shows: its floor and the plan's cut height over it (docs/adr/8012); the drawing's ±0.00 when
    /// it has no level.
    pub(crate) fn underlay_level(&self) -> (f64, f64) {
        let Some(doc) = self.document.as_ref() else { return (kentos_commands::PLAN_CUT_HEIGHT, 0.0) };
        let settings = doc.model.settings();
        let arch = settings.arch.as_ref();
        let cut = arch.map_or(kentos_commands::PLAN_CUT_HEIGHT, kentos_commands::ArchSettings::plan_cut);
        let floor = crate::active_level::effective(&self.arch.active_level, settings)
            .and_then(|id| arch.and_then(|a| a.levels.iter().find(|l| l.id == id)))
            .map_or(0.0, |l| l.elevation);
        (floor + cut, floor)
    }

    /// The drawing's underlays read and drawn as they are now: a new or changed one read off the window's thread, one
    /// taken away let go, the plan at the active level given to the drawing area when it changed.
    pub(crate) fn follow_underlays(&mut self) -> Task<Message> {
        let list: Vec<ModelUnderlay> =
            self.document.as_ref().map(|d| d.model.underlays().to_vec()).unwrap_or_default();
        if list.is_empty() && self.arch.underlays.states.is_empty() && self.arch.underlays.plan.is_none() {
            return Task::none();
        }
        let project = if list.is_empty() { Err(String::new()) } else { self.underlay_project() };
        let mut tasks = Vec::new();
        for u in &list {
            let key = Key { file: u.file.clone(), sha256: u.sha256.clone(), placement: u.placement, project: project.clone() };
            if self.arch.underlays.states.get(&u.id).is_some_and(|(k, _)| *k == key) {
                continue;
            }
            self.arch.underlays.states.insert(u.id.clone(), (key.clone(), State::Reading));
            let (done, wait) = oneshot::channel();
            let (file, sha, placement, frame) = (u.file.clone(), u.sha256.clone(), u.placement, project.clone());
            std::thread::spawn(move || {
                let _ = done.send(load(&file, &sha, placement, frame));
            });
            let id = u.id.clone();
            tasks.push(Task::perform(wait, move |read| {
                let read = read.unwrap_or_else(|_| Err("Okuma yarıda kaldı.".to_owned())).map(Arc::new);
                Message::Arch(arch::Event::UnderlayRead(id.clone(), Box::new(key.clone()), read))
            }));
        }
        self.arch.underlays.states.retain(|id, _| list.iter().any(|u| &u.id == id));
        self.underlay_plan_follow();
        Task::batch(tasks)
    }

    /// An underlay's read came: kept when the drawing still wants what it was read for.
    pub(crate) fn underlay_read(&mut self, id: String, key: Key, read: Result<Arc<Loaded>, String>) {
        if let Some((k, state)) = self.arch.underlays.states.get_mut(&id)
            && *k == key
        {
            *state = match read {
                Ok(l) => State::Ready(l),
                Err(why) => State::Failed(why),
            };
        }
        self.underlay_plan_follow();
        self.building3d_underlays();
    }

    /// The underlays read and shown, in the drawing's order, with their records.
    pub(crate) fn underlays_shown(&self) -> Vec<(ModelUnderlay, Arc<Loaded>)> {
        let Some(doc) = self.document.as_ref() else { return Vec::new() };
        doc.model
            .underlays()
            .iter()
            .filter(|u| u.shown)
            .filter_map(|u| match self.arch.underlays.states.get(&u.id) {
                Some((_, State::Ready(l))) => Some((u.clone(), l.clone())),
                _ => None,
            })
            .collect()
    }

    /// The plan at the active level, made again when the underlays drawn or the level changed, and given to the
    /// drawing area.
    pub(crate) fn underlay_plan_follow(&mut self) {
        let shown = self.underlays_shown();
        let (cut, floor) = self.underlay_level();
        let key: Vec<(String, String, u64, u64)> =
            shown.iter().map(|(u, l)| (u.id.clone(), l.sha256.clone(), cut.to_bits(), floor.to_bits())).collect();
        if self.arch.underlays.plan.as_ref().is_some_and(|(k, _)| *k == key) {
            return;
        }
        let mut lines = Vec::new();
        for (_, l) in &shown {
            lines.extend(plan_of(l, cut, floor));
        }
        let lines = Arc::new(lines);
        self.arch.underlays.version += 1;
        self.arch.underlays.plan = Some((key, lines.clone()));
        self.viewport.set_underlay((self.arch.underlays.version, lines));
    }

    /// The underlay object nearest `at` in the plan within `reach` drawing units: the underlay's id and the object's
    /// index.
    pub(crate) fn underlay_object_at(&self, at: [f64; 2], reach: f64) -> Option<(String, usize)> {
        let shown = self.underlays_shown();
        let (cut, floor) = self.underlay_level();
        let mut best: Option<(f64, String, usize)> = None;
        for (u, l) in &shown {
            for line in plan_of(l, cut, floor) {
                let d = distance(at, line.a, line.b);
                if d <= reach && best.as_ref().is_none_or(|b| d < b.0) {
                    best = Some((d, u.id.clone(), line.owner as usize));
                }
            }
        }
        best.map(|(_, id, owner)| (id, owner))
    }
}

/// How far `p` is from segment a–b.
fn distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = dx * dx + dy * dy;
    let t = if len > 0.0 { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len).clamp(0.0, 1.0) } else { 0.0 };
    let (x, y) = (a[0] + t * dx - p[0], a[1] + t * dy - p[1]);
    (x * x + y * y).sqrt()
}
