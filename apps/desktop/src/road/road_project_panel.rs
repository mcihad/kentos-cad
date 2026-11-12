//! Yol projesi (plan/road.md Y13.3, docs/adr/0911 §4; ROAD-MOD-04): the road project's tree in the dock, as
//! `road.project.query` reads it (Civil 3D's Prospector, OpenRoads' Explorer › OpenRoads Model), drawn by KentOS UI's
//! status tree: a row a source or a drawing under its parent, its state a pill (up to date is quiet), its reasons the
//! row's tip. A source's row opens its editor; a drawing's row selects its objects and shows them, its ↻ draws it again
//! (`road.project.rebuild`). Above: Kurarak denetle (the deep reading: surfaces and corridors made, approved volumes
//! computed) and Kendiliğinden yeniden üret (`road.project.edit`), İş grafiği… (the chain to the sheets,
//! `road.project.graph`, docs/adr/1940); the tree's head: the counts that filter, Hepsini yeniden üret. The tree is
//! read again only when the drawing's revision or the depth changes. A rebuild runs as a job (docs/adr/2440): the panel
//! shows its progress a drawing at a time and Durdur, heard between the drawings; a stopped one writes nothing.

use std::cell::RefCell;
use std::collections::HashSet;

use iced::widget::{column, container};
use iced::{Element, Task};
use iced::futures::{SinkExt, Stream, channel::mpsc};
use kentos_commands::{
    CallEnvelope, CallOp, CommandResult, JobPhase, RoadNode, RoadNodeKind as N, RoadNodeState as S, RoadProjectEdit,
    RoadProjectRebuild, RoadProjectRebuilt, RoadProjectTree,
};
use kentos_domain::{Slot, Uuid};
use kentos_native_application::jobs::Job;
use kentos_native_application::{ExecutionContext, road_project_edit, road_project_query};
use kentos_ui::widget::progress::{self, Tint};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::widget::tree_view::Check;
use kentos_ui::widget::{Status, StatusItem, StatusTree};

use kentos_interaction::Level;

use crate::app::{App, Message};

#[derive(Clone, Debug)]
pub enum Event {
    /// A row, by its place in the tree.
    Open(usize),
    /// Draw one drawing again (by id; `edited` also when changed by hand), or every stale and missing one.
    Rebuild(Option<String>, bool),
    Auto(bool),
    /// Yalnız değişeni yaz (docs/adr/4320): ticked, a drawing drawn again keeps what comes out as it is.
    Regional(bool),
    /// Read the tree deep (surfaces and corridors made), or back to the quick reading.
    Deep(bool),
    /// A branch folded or opened against its start, by its key.
    Fold(String),
    /// Only the rows in a state (and their parents), or all.
    Filter(Option<Status>),
    /// The running rebuild looked at (the clock's tick), and asked to stop (docs/adr/2440).
    Tick,
    Stop,
}

/// A rebuild running as a job: the drawing it was started on, how far it has come, whether it was asked to stop.
pub struct Running {
    job: Job,
    session: u64,
    share: Option<f64>,
    stopping: bool,
}

/// How often a running rebuild is looked at.
const TICK: std::time::Duration = std::time::Duration::from_millis(120);

/// The clock a running rebuild listens to.
pub fn ticks() -> impl Stream<Item = Message> {
    let (mut out, ticks) = mpsc::channel(1);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(TICK);
            if iced::futures::executor::block_on(out.send(msg(Event::Tick))).is_err() {
                break;
            }
        }
    });
    ticks
}

fn msg(e: Event) -> Message {
    Message::RoadProject(e)
}

/// The last tree read: the revision and the depth it was read at, and what it answered.
type Read = (String, bool, Result<RoadProjectTree, String>);

/// The panel's state: deep or quick, the user's folds, the filter, and the last tree.
#[derive(Default)]
pub struct PanelState {
    pub deep: bool,
    /// The branches the user folded or opened against their start (a branch starts open when something in it wants
    /// attention, folded when all of it is quiet): by kind and id.
    toggled: HashSet<String>,
    filter: Option<Status>,
    read: RefCell<Option<Read>>,
    /// The rebuild running as a job, if one is.
    pub running: Option<Running>,
}

impl PanelState {
    /// Whether a rebuild is running (the app listens to the clock then).
    pub fn running(&self) -> bool {
        self.running.is_some()
    }
}

impl std::fmt::Debug for PanelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PanelState").field("deep", &self.deep).finish()
    }
}

/// Durdur: a running rebuild asked to stop between its drawings.
pub const STOP: &str = "Durdur";

/// A state in words.
pub(crate) fn state_name(s: S) -> &'static str {
    match s {
        S::Current => "Güncel",
        S::New => "Kabul edilmedi",
        S::Stale => "Güncel değil",
        S::Unbuildable => "Kurulamıyor",
        S::Unsolved => "Çözülmeyen kesit",
        S::Approved => "Onaylı",
        S::Changed => "Onaydan sonra değişti",
        S::Missing => "Nesnesi eksik",
        S::Edited => "Elle değişti",
        S::Gone => "Kaynağı yok",
    }
}

/// A state's tone: up to date quiet, stale amber, broken red (Civil 3D's yellow badge on an out of date item).
fn tone(s: S) -> Status {
    match s {
        S::Current => Status::Ok,
        S::New => Status::Neutral,
        S::Approved => Status::Info,
        S::Stale | S::Unsolved | S::Missing | S::Edited => Status::Warning,
        S::Unbuildable | S::Changed | S::Gone => Status::Danger,
    }
}

/// What a row of `kind` is called.
fn kind_words(kind: N) -> &'static str {
    match kind {
        N::Surface => "Yüzey",
        N::Alignment => "Aks",
        N::Profile => "Boykesit",
        N::Sections => "Kesit takımı",
        N::Superelevation => "Dever",
        N::Junction => "Kavşak",
        N::Template => "Tip kesit",
        N::Corridor => "Koridor",
        N::Volume => "Kübaj",
        N::Haul => "Brükner",
        N::Guardrail => "Otokorkuluk",
        N::Culvert => "Menfez",
        N::Wall => "İstinat duvarı",
        N::Platform => "Platform",
        N::SheetSet => "Pafta takımı",
        N::Pay => "Poz metrajı",
        N::Network => "Ağ",
        N::Drawing => "Çizim",
    }
}

/// The command that opens a row's editor and its icon (the editor's own).
fn editor_of(kind: N) -> Option<(&'static str, &'static str)> {
    Some(match kind {
        N::Surface => ("terrain.surfaces", "terrainSurface"),
        N::Alignment => ("alignment.editor", "alignmentEditor"),
        N::Profile => ("profile.editor", "profileEditor"),
        N::Sections | N::Template => ("section.editor", "sectionEditor"),
        N::Superelevation => ("superelevation.editor", "superelevationEditor"),
        N::Junction => ("junction.editor", "junctionEditor"),
        N::Corridor => ("corridor.editor", "corridorEditor"),
        N::Volume => ("volume.editor", "volumeEditor"),
        N::Haul => ("haul.editor", "haulEditor"),
        N::Guardrail => ("guardrail.editor", "guardrailEditor"),
        N::Culvert => ("culvert.editor", "culvertEditor"),
        N::Wall => ("retainingWall.editor", "retainingWallEditor"),
        N::Platform => ("platform.editor", "platformEditor"),
        N::SheetSet => ("road.sheets", "roadSheets"),
        N::Pay => ("payItems.editor", "payItemsEditor"),
        // Docs/adr/3560: a network a pipe run profile was drawn from opens Boru hattı denetimi, where the profile is drawn.
        N::Network => ("pipes.check", "pipesCheck"),
        N::Drawing => return None,
    })
}

/// A row's key in the folds: its kind and id.
fn key_of(n: &RoadNode) -> String {
    format!("{:?}:{}", n.kind, n.id)
}

impl App {
    /// A dock pane's title: the road project's tree is “Proje ağacı” in KentOS Harita (its surfaces and what was drawn
    /// from them, docs/adr/1720), “Yol projesi” elsewhere.
    pub(crate) fn panel_title(&self, panel: crate::app::Panel) -> &'static str {
        if panel == crate::app::Panel::RoadProject && self.program == Some(kentos_launch::Module::Map) {
            "Proje ağacı"
        } else {
            panel.title()
        }
    }

    /// The tree as the drawing is now, read again when its revision or the depth changed.
    pub(crate) fn road_project_read(&self) -> Option<Result<RoadProjectTree, String>> {
        let doc = self.document.as_ref()?;
        let revision = doc.model.revision().to_string();
        let deep = self.road_project.deep;
        let mut kept = self.road_project.read.borrow_mut();
        if let Some((r, d, answer)) = kept.as_ref()
            && *r == revision
            && *d == deep
        {
            return Some(answer.clone());
        }
        let answer = road_project_query::preview(&doc.model, deep);
        *kept = Some((revision, deep, answer.clone()));
        Some(answer)
    }

    pub(crate) fn road_project_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open(k) => {
                let Some(Ok(tree)) = self.road_project_read() else { return Task::none() };
                let Some(n) = tree.nodes.get(k).cloned() else { return Task::none() };
                if n.kind == N::Drawing {
                    self.select_drawing(&n.id);
                } else if let Some((command, _)) = editor_of(n.kind) {
                    return self.update(Message::Run(command));
                }
            }
            Event::Rebuild(id, edited) => {
                if self.road_project.running.is_some() {
                    return Task::none();
                }
                let Some(doc) = self.document.as_ref() else { return Task::none() };
                // Docs/adr/4740: Yalnız değişeni yaz is the project's (drawing again by itself follows it too).
                let regional = !doc.model.road_rebuild_whole();
                let input = RoadProjectRebuild { drawings: id.map(|i| vec![i]), edited, regional, expected_revision: None };
                let envelope = CallEnvelope {
                    command: kentos_commands::ROAD_PROJECT_REBUILD.into(),
                    version: None,
                    op: CallOp::Execute,
                    input: serde_json::to_value(&input).unwrap_or_default(),
                    request_id: None,
                    expected: None,
                    relax_constraints: false,
                };
                self.road_project.running = Some(Running {
                    job: Job::start(format!("yeniden-uret-{}", doc.model.revision()), &doc.model, envelope),
                    session: doc.session,
                    share: Some(0.0),
                    stopping: false,
                });
            }
            Event::Tick => self.road_project_tick(),
            Event::Stop => {
                if let Some(r) = self.road_project.running.as_mut() {
                    r.job.cancel();
                    r.stopping = true;
                }
            }
            Event::Auto(on) => {
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                let input = RoadProjectEdit { auto_rebuild: Some(on), regional: None, expected_revision: None };
                let result = road_project_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
                if self.answered(result).is_some() {
                    let words = if on {
                        "Kendiliğinden yeniden üret açık: eskiyen çizimler değişikliğin adımında yeniden üretilir."
                    } else {
                        "Kendiliğinden yeniden üret kapalı."
                    };
                    self.say(Level::Info, words.to_owned());
                }
            }
            // Docs/adr/4740: the project's choice, one undo step “Yalnız değişeni yaz”.
            Event::Regional(on) => {
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                let input = RoadProjectEdit { auto_rebuild: None, regional: Some(on), expected_revision: None };
                let result = road_project_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
                if self.answered(result).is_some() {
                    let words = if on {
                        "Yalnız değişeni yaz açık: yeniden üretim, elle de kendiliğinden de, aynı çıkan nesneleri yerinde bırakır."
                    } else {
                        "Yalnız değişeni yaz kapalı: yeniden üretim her çizimi bütün çizer."
                    };
                    self.say(Level::Info, words.to_owned());
                }
            }
            Event::Deep(on) => self.road_project.deep = on,
            Event::Fold(key) => {
                if !self.road_project.toggled.remove(&key) {
                    self.road_project.toggled.insert(key);
                }
            }
            Event::Filter(f) => self.road_project.filter = f,
        }
        Task::none()
    }

    /// The running rebuild looked at: its progress, and when it is over its answer (written to the drawing by the job).
    pub(crate) fn road_project_tick(&mut self) {
        let (Some(doc), Some(r)) = (self.document.as_mut(), self.road_project.running.as_mut()) else {
            return;
        };
        if doc.session != r.session {
            // The drawing it was started on is gone: nothing to write to.
            r.job.cancel();
            self.road_project.running = None;
            return;
        }
        let status = r.job.status(&mut doc.model);
        if !status.phase.finished() {
            r.share = status.progress;
            return;
        }
        self.road_project.running = None;
        if status.phase == JobPhase::Cancelled {
            self.say(Level::Info, "Yeniden üretim durduruldu; hiçbir şey yazılmadı.".to_owned());
            return;
        }
        let result = match status.answer.map(|a| a.result) {
            Some(CommandResult::Completed { output, warnings }) => match serde_json::from_value::<RoadProjectRebuilt>(output) {
                Ok(output) => CommandResult::Completed { output, warnings },
                Err(e) => return self.warn(format!("Yeniden üretimin cevabı okunamadı: {e}")),
            },
            Some(CommandResult::Failed { error }) => CommandResult::Failed { error },
            Some(CommandResult::Conflict { error }) => CommandResult::Conflict { error },
            _ => return self.say(Level::Info, "Yeniden üretim durduruldu; hiçbir şey yazılmadı.".to_owned()),
        };
        if let Some(out) = self.answered(result) {
            let words = self.rebuilt_words(&out);
            self.say(Level::Success, words);
        }
    }

    /// What a rebuild did, in a sentence: the drawings and their objects; asked to write only what changed
    /// (docs/adr/4320), the objects that stayed, those written and removed, and the box where the lines changed.
    fn rebuilt_words(&self, out: &RoadProjectRebuilt) -> String {
        let objects: usize = out.rebuilt.iter().map(|r| r.uids.len()).sum();
        let n = out.rebuilt.len();
        if n == 0 {
            return "Güncel olmayan çizim yok.".to_owned();
        }
        if out.rebuilt.iter().all(|r| r.kept.is_none()) {
            return format!("{n} çizim yeniden üretildi: {objects} nesne.");
        }
        let kept: usize = out.rebuilt.iter().filter_map(|r| r.kept).map(|k| k as usize).sum();
        let removed: usize = out.rebuilt.iter().map(|r| r.removed as usize).sum();
        let written = objects.saturating_sub(kept);
        if written == 0 && removed == 0 {
            return format!("{n} çizim yeniden üretildi: hiçbir nesne değişmedi, {kept} nesne yerinde; kayıt güncel.");
        }
        let region = out.rebuilt.iter().filter_map(|r| r.region).reduce(|[a, b, c, d], [e, f, g, h]| {
            [a.min(e), b.min(f), c.max(g), d.max(h)]
        });
        let place = region.map_or_else(String::new, |[x0, y0, x1, y1]| {
            let f = self.format();
            format!("; değişen bölge {} × {}", f.length(x1 - x0), f.length(y1 - y0))
        });
        format!("{n} çizim yeniden üretildi: {kept} nesne yerinde kaldı, {written} yazıldı, {removed} silindi{place}.")
    }

    /// A drawing's objects selected and shown.
    fn select_drawing(&mut self, id: &str) {
        let Some(doc) = self.document.as_ref() else { return };
        let Some(d) = doc.model.road_drawings().iter().find(|d| d.id == id) else { return };
        let slots: Vec<Slot> =
            d.uids.iter().filter_map(|u| Uuid::parse_str(u).ok()).filter_map(|u| doc.model.slot_of(u)).collect();
        let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
        self.selection.set(slots);
        if !ids.is_empty() {
            self.navigating(|app| app.zoom_to_objects(&ids));
        }
    }

    pub(crate) fn road_project_view(&self) -> Element<'_, Message> {
        let Some(read) = self.road_project_read() else {
            return container(label::caption("Açık çizim yok.")).padding([8, 12]).into();
        };
        let tree = match read {
            Ok(t) => t,
            Err(why) => return container(label::caption(why)).padding([8, 12]).into(),
        };
        // The tree's items from its flat rows, children before their parents taken.
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); tree.nodes.len()];
        let mut roots = Vec::new();
        for (k, n) in tree.nodes.iter().enumerate() {
            match n.parent {
                Some(p) if (p as usize) < k => children[p as usize].push(k),
                _ => roots.push(k),
            }
        }
        let mut made: Vec<Option<StatusItem<'_, Message>>> = (0..tree.nodes.len()).map(|_| None).collect();
        for k in (0..tree.nodes.len()).rev() {
            let mut item = road_item(k, &tree.nodes[k]);
            for &c in &children[k] {
                if let Some(child) = made[c].take() {
                    item = item.push(child);
                }
            }
            made[k] = Some(item);
        }
        let items: Vec<_> = roots.into_iter().filter_map(|k| made[k].take()).collect();
        let on = |b: bool| if b { Check::Checked } else { Check::Unchecked };
        let options = column![
            crate::style::fields::check(
                on(self.road_project.deep),
                "Kurarak denetle".to_owned(),
                Some(msg(Event::Deep(!self.road_project.deep))),
            ),
            crate::style::fields::check(
                on(tree.auto_rebuild),
                "Kendiliğinden yeniden üret".to_owned(),
                Some(msg(Event::Auto(!tree.auto_rebuild))),
            ),
            // Docs/adr/4320, 4740: a drawing drawn again, by hand or by itself, keeps the objects that come out as they
            // are; the project's choice.
            crate::style::fields::check(
                on(tree.regional),
                "Yalnız değişeni yaz".to_owned(),
                Some(msg(Event::Regional(!tree.regional))),
            ),
            // İş grafiği (docs/adr/1940): the chain from the sources to the sheets, the drawings' recipes.
            crate::exchange::words::secondary(
                crate::project_graph_window::OPEN,
                Some(Message::ProjectGraph(crate::project_graph_window::Event::Open)),
            ),
        ]
        .spacing(4)
        .padding([2, 12]);
        let running = self.road_project.running.as_ref();
        let rebuild = (tree.rebuildable > 0 && running.is_none()).then(|| msg(Event::Rebuild(None, false)));
        let body = StatusTree::new(items)
            .toggled(&self.road_project.toggled, |key| msg(Event::Fold(key)))
            .filter(self.road_project.filter, |f| msg(Event::Filter(f)))
            .bulk(icon(Icon::Redo).size(13.0), "Hepsini yeniden üret", tree.rebuildable as usize, rebuild)
            .counted("Güncel değil", "Sorun")
            .empty(if self.program == Some(kentos_launch::Module::Map) {
                "Projede yüzey yok: Arazi sekmesinden yüzey oluşturun; ondan çizilen eş yükselti, platform ve sıfır hattı burada görünür."
            } else {
                "Projede yol kaynağı yok: Arazi sekmesinden yüzey, Güzergâh'tan aks oluşturun."
            });
        // A running rebuild (docs/adr/2440): its progress and Durdur.
        let progress_row: Element<'_, Message> = match running {
            Some(r) => iced::widget::row![
                progress::bar(r.share.map(|s| s as f32)).tint(Tint::Muted),
                label::caption(if r.stopping { "Durduruluyor…" } else { "Yeniden üretiliyor…" }),
                crate::exchange::words::secondary(STOP, (!r.stopping).then(|| msg(Event::Stop)))
            ]
            .spacing(8)
            .align_y(iced::Center)
            .padding([2, 12])
            .into(),
            None => iced::widget::space().into(),
        };
        column![options, progress_row, kentos_ui::widget::horizontal_divider(), body.view()].spacing(4).padding([6, 0]).into()
    }
}

/// A row of the tree: its kind's icon, its name, its count or kind faint, its state's pill and why, and for a drawing
/// the button that draws it again.
fn road_item<'a>(k: usize, n: &RoadNode) -> StatusItem<'a, Message> {
    let meta = match (n.kind, n.objects) {
        (N::Drawing, Some(c)) => format!("{c} nesne"),
        (kind, _) => kind_words(kind).to_owned(),
    };
    let glyph = match editor_of(n.kind) {
        Some((_, name)) => crate::icons::from_web(Some(name)),
        None => Icon::Layers,
    };
    let mut item = StatusItem::new(key_of(n), n.name.clone())
        .icon(icon(glyph).size(14.0))
        .meta(meta)
        .status(tone(n.state), state_name(n.state))
        .detail(n.why.join(" "))
        .on_press(msg(Event::Open(k)));
    match (n.kind, n.state) {
        (N::Drawing, S::Stale | S::Missing | S::Current) => {
            item = item.action(icon(Icon::Redo).size(13.0), "Yeniden üret", msg(Event::Rebuild(Some(n.id.clone()), false)));
        }
        (N::Drawing, S::Edited) => {
            item = item.action(
                icon(Icon::Redo).size(13.0),
                "Üstüne yaz: elle değişenler gider",
                msg(Event::Rebuild(Some(n.id.clone()), true)),
            );
        }
        _ => {}
    }
    item
}
