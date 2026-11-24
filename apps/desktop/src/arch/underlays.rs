//! Altlık modeller on the drawing (docs/adr/8101; plan/ifc-uyum.md IFC-09, B18): each underlay's file read off the
//! window's thread — checked against the digest the drawing keeps, its bodies made by the reading engine and placed by
//! the same rule the command placed it with (`kentos_ifc::underlay::place`) — then its plan at the active level for
//! the drawing area (cut at the level's cut height, the edges under it down to its floor, faint, under the drawing),
//! its bodies for 3B görünüm, and the object a click finds. Nothing of it is written into the drawing: a file that
//! changed on disk is said, not shown, until Yeniden oku takes its new revision.

use std::collections::HashMap;
use std::sync::Arc;
use crate::arch::ArchState;

use iced::Task;
use iced::futures::channel::oneshot;
use kentos_commands::{ModelUnderlay, UnderlayPlacement};
use kentos_ifc::underlay::{MapFrame, PlanLine};

pub use crate::underlay_model::{Loaded, load, plan_of};

use crate::app::{App, Message};
use crate::arch;

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
pub struct Key {
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
    #[allow(clippy::type_complexity)]
    plan: Option<(Vec<(String, String, u64, u64)>, Arc<Vec<PlanLine>>)>,
    /// One more each time the plan changes.
    pub version: u64,
}

impl std::fmt::Debug for Underlays {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Underlays").field("states", &self.states.len()).field("version", &self.version).finish()
    }
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
        let floor = crate::active_level::effective(&self.active_level, settings)
            .and_then(|id| arch.and_then(|a| a.levels.iter().find(|l| l.id == id)))
            .map_or(0.0, |l| l.elevation);
        (floor + cut, floor)
    }

    /// The drawing's underlays read and drawn as they are now: a new or changed one read off the window's thread, one
    /// taken away let go, the plan at the active level given to the drawing area when it changed.
    pub(crate) fn follow_underlays(&mut self) -> Task<Message> {
        let list: Vec<ModelUnderlay> =
            self.document.as_ref().map(|d| d.model.underlays().to_vec()).unwrap_or_default();
        if list.is_empty() && self.modules.arch().underlays.states.is_empty() && self.modules.arch().underlays.plan.is_none() {
            return Task::none();
        }
        let project = if list.is_empty() { Err(String::new()) } else { self.underlay_project() };
        let mut tasks = Vec::new();
        for u in &list {
            let key = Key { file: u.file.clone(), sha256: u.sha256.clone(), placement: u.placement, project: project.clone() };
            if self.modules.arch().underlays.states.get(&u.id).is_some_and(|(k, _)| *k == key) {
                continue;
            }
            self.modules.arch_mut().underlays.states.insert(u.id.clone(), (key.clone(), State::Reading));
            let (done, wait) = oneshot::channel();
            let (file, sha, placement, frame) = (u.file.clone(), u.sha256.clone(), u.placement, project.clone());
            std::thread::spawn(move || {
                let _ = done.send(load(&file, &sha, placement, frame));
            });
            let id = u.id.clone();
            tasks.push(Task::perform(wait, move |read| {
                let read = read.unwrap_or_else(|_| Err("Okuma yarıda kaldı.".to_owned())).map(Arc::new);
                Message::from(arch::Event::UnderlayRead(id.clone(), Box::new(key.clone()), read))
            }));
        }
        self.modules.arch_mut().underlays.states.retain(|id, _| list.iter().any(|u| &u.id == id));
        self.underlay_plan_follow();
        Task::batch(tasks)
    }

    /// An underlay's read came: kept when the drawing still wants what it was read for.
    pub(crate) fn underlay_read(&mut self, id: String, key: Key, read: Result<Arc<Loaded>, String>) {
        if let Some((k, state)) = self.modules.arch_mut().underlays.states.get_mut(&id)
            && *k == key
        {
            *state = match read {
                Ok(l) => State::Ready(l),
                Err(why) => State::Failed(why),
            };
        }
        // 3B görünüm takes the underlays shown when it follows the drawing (scene3d_building.rs).
        self.underlay_plan_follow();
    }

    /// The underlays read and shown, in the drawing's order, with their records.
    pub(crate) fn underlays_shown(&self) -> Vec<(ModelUnderlay, Arc<Loaded>)> {
        let Some(doc) = self.document.as_ref() else { return Vec::new() };
        doc.model
            .underlays()
            .iter()
            .filter(|u| u.shown)
            .filter_map(|u| match self.modules.arch().underlays.states.get(&u.id) {
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
        if self.modules.arch().underlays.plan.as_ref().is_some_and(|(k, _)| *k == key) {
            return;
        }
        let mut lines = Vec::new();
        for (_, l) in &shown {
            lines.extend(plan_of(l, cut, floor));
        }
        let lines = Arc::new(lines);
        self.modules.arch_mut().underlays.version += 1;
        self.modules.arch_mut().underlays.plan = Some((key, lines.clone()));
        self.viewport.set_underlay((self.modules.arch().underlays.version, lines));
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
