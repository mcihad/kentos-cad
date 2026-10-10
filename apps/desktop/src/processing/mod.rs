//! İşlemler on the desktop (docs/PROCESSING.md, docs/adr/0084): the web's
//! processing tools and models, run by `kentos_processing` over the native
//! document, with the web's window and commands.
//!
//! - Every tool is a command (`processing.run.<id>`), and every model too
//!   (`processing.model.<id>`), with the web's aliases on the command line;
//!   Harita's Kenar ölçülerini yaz opens Kenar uzunluklarını yaz.
//! - The window ([`dialog`]) is made from the tool's definition, as the
//!   web's `ToolDialog`: Girdi, Ayarlar, Çıktı and Gelişmiş ayarlar on the
//!   left; what the tool does, its preview, where it runs and its aliases on
//!   the right. It stays open after a run, to adjust and run again.
//! - A run writes through the document's transaction, one undo step named
//!   after the tool (a model after itself); history is kept for the session.
//! - A tool or a model whose inputs are 2 000 objects or more runs on
//!   another thread ([`background`], the web's worker; Nerede çalışır
//!   chooses): the window shows how far it is, Durdur stops it.
//! - Each tool's last values outlive the program in `islemler.json`
//!   ([`memory`]).

mod background;
pub(crate) mod designer;
pub mod dialog;
mod fields;
pub mod memory;
pub mod panel;
mod plan;

/// The İşlemler panel's event as the app's message (the ribbon's model buttons send it too).
pub(crate) fn panel_message(event: panel::Event) -> crate::app::Message {
    crate::app::Message::Processing(Event::Panel(event))
}
#[cfg(test)]
mod distance_tests;
#[cfg(test)]
mod geometry_tests;
#[cfg(test)]
mod hydrology_tests;
#[cfg(test)]
mod interpolation_tests;
#[cfg(test)]
mod query_tests;
#[cfg(test)]
mod raster_ops_tests;
#[cfg(test)]
mod raster_vector_tests;
#[cfg(test)]
mod remote_tests;
#[cfg(test)]
mod stats_tests;
#[cfg(test)]
mod suitability_tests;
#[cfg(test)]
pub(crate) mod surface_tests;
#[cfg(test)]
mod tests;
mod window;

use iced::Task;
use kentos_domain::Slot;
use kentos_interaction::pick::PickPoint;
use kentos_interaction::{Level, Selection, Vec2};
use kentos_processing::model_runner::{MODEL_PREFIX, end_model, replay_model, run_model};
use kentos_processing::{
    Bounds, Defaults, Host, LogLine, Outcome, Prepared, Registry, Runner, Scene, Store, Target,
    Values,
};
use serde_json::{Value, json};

use crate::app::{App, Dialog, Message};
pub use dialog::{RunStatus, ToolDialog};

/// Said when a model's drawing changed while it ran on another thread.
const CHANGED_MEANWHILE: &str =
    "Çizim model çalışırken değişti; model çizimin şimdiki hâlinde yeniden çalıştırıldı.";

/// Whether a message is a background run's last word (perf::frame waits for it).
#[cfg(test)]
pub(crate) fn is_answer(message: &Message) -> bool {
    matches!(
        message,
        Message::Processing(Event::Background(
            _,
            background::Reply::Done | background::Reply::Failed(_)
        ))
    )
}

/// Whether İşlemler answers a command: a tool, a model, Harita's Kenar ölçülerini yaz, Eşyükselti
/// üret and Eğim analizi (docs/adr/0231 §10).
pub fn answers(id: &str) -> bool {
    id.starts_with("processing.run.")
        || id.starts_with("processing.model.")
        || matches!(
            id,
            "map.edgeLengths"
                | "map.contours"
                | "analysis.slope"
                | "processing.toolbox"
                | "processing.history"
                | "processing.newModel"
        )
}

/// İşlemler's state while the app runs.
pub struct Processing {
    pub registry: Registry,
    pub runner: Runner,
    pub memory: memory::Memory,
    /// The open window.
    pub dialog: Option<ToolDialog>,
    /// The window put away while its field is picked on the drawing (Sahneden seç).
    pub(crate) picking: Option<Picking>,
    /// Model tasarımcısı (designer/, docs/adr/0116).
    pub(crate) designer: Option<Box<designer::Designer>>,
    /// The dock's İşlemler tab.
    pub panel: panel::PanelState,
    /// Runs going on other threads (background.rs).
    pub(crate) running: Vec<background::Running>,
    /// The last background run's id.
    runs: u64,
}

impl Default for Processing {
    fn default() -> Self {
        Self {
            registry: Registry::builtin(),
            runner: Runner::new(),
            memory: memory::Memory::temporary(),
            dialog: None,
            picking: None,
            designer: None,
            panel: panel::PanelState::default(),
            running: Vec::new(),
            runs: 0,
        }
    }
}

/// A field picked on the drawing while its window steps aside (docs/adr/0088).
pub(crate) enum Picking {
    /// A point for a model designer's step, its draft kept aside.
    Designer {
        designer: Box<designer::Designer>,
        step: String,
        name: String,
        then: Option<(String, String)>,
    },
    /// A point parameter's point; `then`: the choice and option it also chooses.
    Point {
        window: ToolDialog,
        name: String,
        then: Option<(String, String)>,
    },
    /// A features parameter's objects; the selection before, back on Esc.
    Objects {
        window: ToolDialog,
        name: String,
        before: Vec<Slot>,
    },
}

/// What the window asks for.
#[derive(Clone, Debug)]
pub enum Event {
    Close,
    /// Varsayılanlar: every field back to its default.
    Reset,
    Run,
    /// Durdur: the window's background run stops.
    Stop,
    /// What background run `id` says.
    Background(u64, background::Reply),
    /// A parameter's value chosen: a choice, a switch, a scope, a layer, kinds.
    Value(String, Value),
    /// A number field's text as typed.
    Number(String, String),
    /// A text, field or expression field's text as typed.
    Text(String, String),
    /// A name typed before it is added (Rasterlere değer's Ekle in a model's step, docs/adr/0237 §9).
    Draft(String, String),
    /// A new layer's name as typed.
    LayerName(String, String),
    /// A features parameter's scope chosen: the parameter and the scope's id.
    Scope(String, String),
    /// Nerede çalışır chosen: "auto" or a place's id, kept per tool.
    Target(String),
    /// Gelişmiş ayarlar opened or closed.
    Advanced,
    /// A kind chip pressed: the parameter and the kind.
    Kind(String, String),
    /// Text put at the end of an expression (a field, a variable, a function).
    Insert(String, String),
    /// Sahneden seç for a point parameter.
    Pick(String),
    /// Sahneden seç beside a choice one of whose options is a picked point
    /// (the numbering's start vertex).
    PickChoice(String),
    /// Sahneden seç for a features parameter's objects.
    PickObjects(String),
    /// Dosya seç…: a file parameter's file asked for (docs/adr/0200 §7).
    ChooseFile(String),
    /// A command a field's button runs (Ağlar… beside a network field, docs/adr/0209 §10).
    Command(&'static str),
    /// The file chosen for a parameter: its name, where it is and its bytes; none when given up.
    FileChosen(String, Option<(String, std::path::PathBuf, Vec<u8>)>),
    /// Konum…: where a result is written asked for (docs/adr/0207 §7).
    ChooseSave(String),
    /// Panoya kopyala for the run's table (Özet istatistik).
    CopyTable,
    /// CSV olarak kaydet for the run's table.
    SaveTable,
    /// Where the run's table is saved; none when given up.
    TableSaved(Option<std::path::PathBuf>),
    /// Sonuçları seç, or Seçime yakınlaştır after a selecting tool.
    Results,
    /// Geri al after a run.
    Undo,
    /// The dock's İşlemler tab.
    Panel(panel::Event),
}

/// The app as the window reads it: the live descriptions and previews.
pub(crate) struct Look<'a> {
    pub doc: &'a kentos_domain::Document,
    pub selection: &'a Selection,
    pub store: &'a Store,
    pub view: Option<Bounds>,
}

impl Scene for Look<'_> {
    fn doc(&self) -> &kentos_domain::Document {
        self.doc
    }

    fn selected(&self) -> Vec<Slot> {
        self.selection.ids().to_vec()
    }

    fn visible_bounds(&self) -> Option<Bounds> {
        self.view
    }

    fn store(&self) -> Option<&Store> {
        Some(self.store)
    }
}

/// The app as a run changes it.
struct Stage<'a> {
    doc: &'a mut kentos_domain::Document,
    selection: &'a mut Selection,
    store: &'a Store,
    view: Option<Bounds>,
}

impl Scene for Stage<'_> {
    fn doc(&self) -> &kentos_domain::Document {
        self.doc
    }

    fn selected(&self) -> Vec<Slot> {
        self.selection.ids().to_vec()
    }

    fn visible_bounds(&self) -> Option<Bounds> {
        self.view
    }

    fn store(&self) -> Option<&Store> {
        Some(self.store)
    }
}

impl Host for Stage<'_> {
    fn doc_mut(&mut self) -> &mut kentos_domain::Document {
        self.doc
    }

    fn select(&mut self, ids: &[Slot]) {
        self.selection.set(ids.iter().copied());
    }
}

impl App {
    /// A tool's or a model's command: its window opens.
    pub(crate) fn processing_command(&mut self, id: &str) -> Task<Message> {
        if self.document.is_none() {
            self.warn("İşlem araçları için önce bir çizim açın.");
            return Task::none();
        }
        // The toolbox and the history are the dock's İşlemler tab (panel.rs).
        if let Some(tab) = match id {
            "processing.toolbox" => Some(panel::Tab::Tools),
            "processing.history" => Some(panel::Tab::History),
            _ => None,
        } {
            self.processing.panel.tab = tab;
            self.docks.show(
                crate::app::Panel::Processing,
                kentos_ui::widget::docking::Side::Right,
            );
            return Task::none();
        }
        if id == "processing.newModel" {
            self.open_model_designer(None);
            return Task::none();
        }
        let tool = if id == "map.edgeLengths" {
            "annotation.edgeLengths".to_owned()
        } else if id == "map.contours" {
            "surface.contours".to_owned()
        } else if id == "analysis.slope" {
            "surface.slope".to_owned()
        } else if let Some(tool) = id.strip_prefix("processing.run.") {
            tool.to_owned()
        } else if let Some(model) = id.strip_prefix("processing.model.") {
            format!("{MODEL_PREFIX}{model}")
        } else {
            return Task::none();
        };
        self.open_processing(&tool, None);
        Task::none()
    }

    /// Opens a tool's or a model's window with these values, else with its
    /// last ones (the web's `openToolDialog`).
    pub(crate) fn open_processing(&mut self, id: &str, values: Option<Values>) {
        let Some(tool) = self.processing.registry.get(id) else {
            // The web's words: a model gone is likely deleted (ToolDialog.ts).
            self.error(match id.strip_prefix(MODEL_PREFIX) {
                Some(model) => format!("Model bulunamadı: {model}. Silinmiş olabilir."),
                None => format!("İşlem aracı bulunamadı: {id}"),
            });
            return;
        };
        let Some(doc) = &self.document else {
            return;
        };
        let explicit = values.is_some();
        let stored = values.or_else(|| self.processing.memory.last_values(&tool.id).cloned());
        let mut window =
            ToolDialog::new(tool, stored.as_ref(), explicit, &Defaults::of(&doc.model));
        window.reread_files();
        self.processing.dialog = Some(window);
        self.refresh_processing();
        self.dialog = Some(Dialog::Processing);
    }

    /// The window's live parts again: problems, what the inputs resolve to, previews.
    pub(crate) fn refresh_processing(&mut self) {
        let (Some(doc), Some(window)) = (&self.document, &mut self.processing.dialog) else {
            return;
        };
        let look = Look {
            doc: &doc.model,
            selection: &self.selection,
            store: self.spatial.store(),
            view: Some(self.viewport.camera.visible_bounds()),
        };
        window.refresh(&self.processing.runner, &look);
    }

    pub(crate) fn processing_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Close => {
                self.processing.dialog = None;
                self.dialog = None;
                return Task::none();
            }
            Event::Run => return self.run_processing(),
            Event::Stop => {
                self.processing_stop();
                return Task::none();
            }
            Event::Background(id, reply) => {
                self.processing_background(id, reply);
                return self.result_shown();
            }
            Event::Results => {
                self.processing_results();
                return Task::none();
            }
            Event::ChooseFile(name) => return self.processing_choose_file(name),
            Event::Command(id) => return self.run(id),
            Event::ChooseSave(name) => return self.processing_choose_save(name),
            Event::FileChosen(_, None) | Event::TableSaved(None) => return Task::none(),
            Event::FileChosen(name, Some((file, path, bytes))) => {
                self.processing_file_read(name, &file, &path, &bytes);
            }
            Event::CopyTable => {
                let Some(table) = self.processing_table() else {
                    return Task::none();
                };
                self.say(Level::Success, "Tablo panoya kopyalandı.".to_owned());
                return iced::clipboard::write(crate::layer_list::tsv(&table.lines()));
            }
            Event::SaveTable => {
                let Some(window) = &self.processing.dialog else {
                    return Task::none();
                };
                let label = window.tool.label.clone();
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title(format!("{label}: tabloyu kaydet"))
                            .add_filter("CSV (.csv)", &["csv"])
                            .set_file_name(format!("{label}.csv"))
                            .save_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |path| Message::Processing(Event::TableSaved(path)),
                );
            }
            Event::TableSaved(Some(path)) => {
                self.processing_save_table(&path);
                return Task::none();
            }
            Event::Undo => {
                self.undo();
                if let Some(window) = &mut self.processing.dialog {
                    window.status = RunStatus::Idle;
                }
            }
            Event::Pick(name) => {
                self.processing_pick(name, None);
                return Task::none();
            }
            Event::PickChoice(name) => {
                let picks = self.processing.dialog.as_ref().and_then(|w| {
                    w.tool
                        .parameters
                        .iter()
                        .find(|p| p.name == name)
                        .and_then(|p| p.picks.clone())
                });
                if let Some((option, point)) = picks {
                    self.processing_pick(point, Some((name, option)));
                }
                return Task::none();
            }
            Event::PickObjects(name) => {
                self.processing_pick_objects(name);
                return Task::none();
            }
            Event::Panel(e) => {
                self.processing_panel_event(e);
                return Task::none();
            }
            Event::Target(choice) => {
                if let Some(window) = &self.processing.dialog {
                    self.processing.memory.set_target(&window.tool.id, &choice);
                }
                return Task::none();
            }
            Event::Reset => {
                if let (Some(doc), Some(window)) = (&self.document, &mut self.processing.dialog) {
                    window.reset(&Defaults::of(&doc.model));
                }
            }
            e => {
                let active = self
                    .document
                    .as_ref()
                    .map(|d| d.model.layers().active().to_owned());
                if let Some(window) = &mut self.processing.dialog {
                    window.edit(e, active.as_deref().unwrap_or(""));
                }
            }
        }
        self.refresh_processing();
        Task::none()
    }

    /// Çalıştır (the web's `ToolDialog.run`): every problem shows, or the
    /// tool runs; the window stays open with what it did. Where it runs is
    /// Nerede çalışır's: Otomatik sends inputs of 2 000 objects or more to
    /// another thread, the window showing how far it is (background.rs).
    fn run_processing(&mut self) -> Task<Message> {
        let Some(window) = &mut self.processing.dialog else {
            return Task::none();
        };
        if window.running() {
            return Task::none();
        }
        let (Some(doc), true) = (&mut self.document, window.attempt()) else {
            self.refresh_processing();
            return Task::none();
        };
        let values = window.values.clone();
        let tool = window.tool.clone();
        let targets = window.targets(&self.processing.registry);
        let choice = plan::effective_choice(
            plan::Choice::read(self.processing.memory.target(&tool.id)),
            &targets.available,
        );
        // Otomatik's answer for the whole: a tool's is taken again from its
        // job below, its locked objects left out (the web's).
        let background = |auto: Option<Target>| match choice {
            Some(plan::Choice::At(Target::Worker)) => true,
            Some(plan::Choice::Auto) => auto == Some(Target::Worker),
            _ => false,
        };
        // The last values keep a file's name and path, not its rows (docs/adr/0200 §7).
        self.processing.memory.remember(
            &tool.id,
            &kentos_processing::parameters::stored_values(&tool, &values),
        );
        let session = doc.session;
        // The files the point cloud tools read and write (pointclouds/files.rs, docs/adr/0207 §7).
        let files = crate::pointclouds::files::for_drawing(&doc.model, doc.path.as_deref());
        let mut stage = Stage {
            doc: &mut doc.model,
            selection: &mut self.selection,
            store: self.spatial.store(),
            view: Some(self.viewport.camera.visible_bounds()),
        };
        let mut log: Vec<LogLine> = Vec::new();
        let registry = &self.processing.registry;
        let runner = &mut self.processing.runner;
        let (running, task) = if let Some(model) = tool
            .id
            .strip_prefix(MODEL_PREFIX)
            .and_then(|id| registry.model(id))
        {
            // A model: its steps one after another, here or whole on another thread.
            if !background(targets.auto) {
                let lookup = |id: &str| registry.tool(id);
                let outcome = run_model(model, &values, runner, &mut stage, &lookup, &mut log);
                self.processing_ran(&tool.label, None, log, outcome);
                return self.result_shown();
            }
            let run = background::ModelRun {
                model: model.clone(),
                inputs: values,
                tools: model
                    .steps
                    .iter()
                    .filter_map(|s| registry.tool(&s.tool).map(|t| (s.tool.clone(), t)))
                    .collect(),
                generation: stage.doc.generation(),
            };
            self.processing.runs += 1;
            background::start_model(
                self.processing.runs,
                run,
                stage.doc.reading_copy(),
                stage.selection.ids().to_vec(),
                stage.view,
                session,
                Some(files),
                |id, reply| Message::Processing(Event::Background(id, reply)),
            )
        } else {
            let mut job = match runner.prepare(&stage, &tool, &values, false, &mut log) {
                Prepared::Ready(job) => job,
                Prepared::Done(outcome) => {
                    self.processing_ran(&tool.label, None, log, outcome);
                    return self.result_shown();
                }
            };
            if !background(plan::auto_target(&targets.available, job.size())) {
                let outcome = runner.complete(&mut stage, job, &mut log);
                self.processing_ran(&tool.label, None, log, outcome);
                return self.result_shown();
            }
            job.target = Target::Worker;
            self.processing.runs += 1;
            background::start_tool(
                self.processing.runs,
                job,
                stage.doc.reading_copy(),
                session,
                Some(files),
                |id, reply| Message::Processing(Event::Background(id, reply)),
            )
        };
        let id = running.id;
        self.processing.running.push(running);
        // What resolving the inputs left out, said as the run starts (the web's).
        for line in log {
            self.say_line(line);
        }
        if let Some(window) = &mut self.processing.dialog {
            window.started(id);
        }
        task
    }

    /// After a run that gave a table, the form scrolled to its end so the table under it is seen (the web's).
    fn result_shown(&self) -> Task<Message> {
        match self.processing.dialog.as_ref().map(|w| &w.status) {
            Some(RunStatus::Ok { table: Some(_), .. }) => {
                iced::widget::operation::snap_to_end(window::FORM)
            }
            _ => Task::none(),
        }
    }

    /// A message of a tool's, in the command history.
    fn say_line(&mut self, line: LogLine) {
        match line.level {
            kentos_processing::Level::Info => self.say(Level::Info, line.text),
            kentos_processing::Level::Warn => self.warn(line.text),
        }
    }

    /// A run ended: its messages and how it went go to the command
    /// history, and its window shows it (`waiting`: the window waiting for
    /// that background run, open or put aside; none: the open one).
    fn processing_ran(
        &mut self,
        label: &str,
        waiting: Option<u64>,
        log: Vec<LogLine>,
        outcome: Outcome,
    ) {
        for line in log {
            self.say_line(line);
        }
        match &outcome {
            Outcome::Ok { record, .. } => {
                self.say(Level::Success, format!("{label}: {}", record.summary));
            }
            Outcome::Stopped { message, .. } => self.warn(message.clone()),
            Outcome::Invalid { .. } => {}
        }
        let window = match waiting {
            Some(id) => self.window_waiting(id),
            None => self.processing.dialog.as_mut(),
        };
        if let Some(window) = window {
            window.ran(outcome);
        }
        self.refresh_processing();
    }

    /// The window waiting for background run `id`: open, or put aside
    /// while one of its fields is picked on the drawing.
    fn window_waiting(&mut self, id: u64) -> Option<&mut ToolDialog> {
        let p = &mut self.processing;
        if p.dialog.as_ref().is_some_and(|w| w.waiting == Some(id)) {
            return p.dialog.as_mut();
        }
        match &mut p.picking {
            Some(Picking::Point { window, .. } | Picking::Objects { window, .. })
                if window.waiting == Some(id) =>
            {
                Some(window)
            }
            _ => None,
        }
    }

    /// What a background run says; a stopped run's words are not heard.
    fn processing_background(&mut self, id: u64, reply: background::Reply) {
        let Some(at) = self.processing.running.iter().position(|r| r.id == id) else {
            return;
        };
        match reply {
            background::Reply::Progress(share, step) => {
                if let Some(window) = self.window_waiting(id) {
                    window.progressed(share, step);
                }
            }
            background::Reply::Line(line) => self.say_line(line),
            background::Reply::Done => {
                let running = self.processing.running.remove(at);
                let answer = running.answer().ok_or_else(|| "sonuç alınamadı".to_owned());
                self.processing_finish(running, answer);
            }
            background::Reply::Failed(why) => {
                let running = self.processing.running.remove(at);
                self.processing_finish(running, Err(why));
            }
        }
    }

    /// A background run's answer, applied on the drawing it was computed
    /// for in one undo step; nothing changes when that drawing is gone. A
    /// model's steps apply as they went on the copy while the drawing is
    /// unchanged; changed meanwhile (another editor's work), the model runs
    /// again here, on the drawing as it is.
    fn processing_finish(
        &mut self,
        running: background::Running,
        answer: Result<background::Answer, String>,
    ) {
        use background::{Answer, Work};
        let label = running.work.label();
        let background::Running {
            id, work, session, ..
        } = running;
        let mut log = Vec::new();
        let runner = &mut self.processing.runner;
        let view = Some(self.viewport.camera.visible_bounds());
        let mut stage = self
            .document
            .as_mut()
            .filter(|d| d.session == session)
            .map(|doc| Stage {
                doc: &mut doc.model,
                selection: &mut self.selection,
                store: self.spatial.store(),
                view,
            });
        let outcome = match (work, answer, &mut stage) {
            (Work::Tool(job), Ok(Answer::Tool(result)), Some(stage)) => {
                runner.finish(stage, job, result, false, &mut log)
            }
            (Work::Tool(job), Ok(_), None) => runner.stop(job),
            (Work::Tool(job), Ok(Answer::Model { .. }), Some(_)) => {
                runner.failed(job, "beklenmeyen bir sonuç geldi")
            }
            (Work::Tool(job), Err(why), _) => runner.failed(job, &why),
            (Work::Model(run), Ok(Answer::Model { steps, log: said }), Some(stage)) => {
                let lookup = |id: &str| run.tool(id);
                if stage.doc.generation() == run.generation {
                    log = said;
                    // The replay repeats the copy's messages.
                    let mut again = Vec::new();
                    replay_model(
                        &run.model,
                        &run.inputs,
                        runner,
                        stage,
                        &lookup,
                        steps,
                        &mut again,
                    )
                } else {
                    log.push(LogLine {
                        level: kentos_processing::Level::Info,
                        text: CHANGED_MEANWHILE.to_owned(),
                    });
                    run_model(&run.model, &run.inputs, runner, stage, &lookup, &mut log)
                }
            }
            (Work::Model(run), Ok(_), _) => {
                end_model(&run.model, &run.inputs, runner, &|id| run.tool(id), None)
            }
            (Work::Model(run), Err(why), _) => end_model(
                &run.model,
                &run.inputs,
                runner,
                &|id| run.tool(id),
                Some(&why),
            ),
        };
        self.processing_ran(&label, Some(id), log, outcome);
    }

    /// Durdur (the web's): the window's background run ends now and the
    /// drawing stays as it is; the thread is asked to stop, and what it
    /// still says is not heard.
    fn processing_stop(&mut self) {
        let Some(id) = self.processing.dialog.as_ref().and_then(|w| w.waiting) else {
            return;
        };
        let Some(at) = self.processing.running.iter().position(|r| r.id == id) else {
            return;
        };
        let running = self.processing.running.remove(at);
        running.stop();
        let label = running.work.label();
        let runner = &mut self.processing.runner;
        let outcome = match running.work {
            background::Work::Tool(job) => runner.stop(job),
            background::Work::Model(run) => {
                end_model(&run.model, &run.inputs, runner, &|id| run.tool(id), None)
            }
        };
        self.processing_ran(&label, Some(id), Vec::new(), outcome);
    }

    /// Dosya seç… (docs/adr/0200 §7): the file asked for with the
    /// parameter's extensions, read on its answer.
    fn processing_choose_file(&mut self, name: String) -> Task<Message> {
        let accept: Vec<String> = self
            .processing
            .dialog
            .as_ref()
            .and_then(|w| w.tool.parameters.iter().find(|p| p.name == name))
            .and_then(|p| match &p.kind {
                kentos_processing::ParamKind::File { accept } => Some(accept.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let extensions: Vec<String> = accept
            .iter()
            .map(|a| a.trim_start_matches('.').to_owned())
            .collect();
        let filter = format!("Tablo ({})", accept.join(", "));
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Tablo dosyası")
                    .add_filter(filter, &extensions)
                    .pick_file()
                    .await?;
                let bytes = file.read().await;
                Some((file.file_name(), file.path().to_path_buf(), bytes))
            },
            move |read| Message::Processing(Event::FileChosen(name.clone(), read)),
        )
    }

    /// Konum… (docs/adr/0207 §7): where a result goes, asked with the
    /// parameter's extensions; the path becomes the parameter's value.
    fn processing_choose_save(&mut self, name: String) -> Task<Message> {
        let accept: Vec<String> = self
            .processing
            .dialog
            .as_ref()
            .and_then(|w| w.tool.parameters.iter().find(|p| p.name == name))
            .and_then(|p| match &p.kind {
                kentos_processing::ParamKind::SaveFile { accept, .. } => Some(accept.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let extensions: Vec<String> = accept
            .iter()
            .map(|a| a.trim_start_matches('.').to_owned())
            .collect();
        let filter = format!("Çıktı ({})", accept.join(", "));
        Task::perform(
            async move {
                let mut d = rfd::AsyncFileDialog::new().set_title("Çıktı dosyası");
                if !extensions.is_empty() {
                    d = d.add_filter(filter, &extensions);
                }
                d.save_file()
                    .await
                    .map(|f| f.path().to_string_lossy().into_owned())
            },
            move |path| match path {
                Some(p) => Message::Processing(Event::Value(name.clone(), json!(p))),
                // Given up: nothing changes.
                None => Message::Processing(Event::FileChosen(name.clone(), None)),
            },
        )
    }

    /// A chosen file read as Tablo ekle reads one, its first sheet's rows
    /// the parameter's value; a file that cannot be read is said and
    /// changes nothing.
    fn processing_file_read(
        &mut self,
        name: String,
        file: &str,
        path: &std::path::Path,
        bytes: &[u8],
    ) {
        match dialog::file_value(file, path, bytes) {
            Ok(value) => {
                let active = self
                    .document
                    .as_ref()
                    .map(|d| d.model.layers().active().to_owned());
                if let Some(window) = &mut self.processing.dialog {
                    window.edit(Event::Value(name, value), active.as_deref().unwrap_or(""));
                }
            }
            Err(why) => self.warn(why),
        }
    }

    /// The open window's last run's table.
    fn processing_table(&self) -> Option<dialog::ResultTable> {
        match &self.processing.dialog.as_ref()?.status {
            RunStatus::Ok { table, .. } => table.clone(),
            _ => None,
        }
    }

    /// CSV olarak kaydet: the run's table written where the user said.
    fn processing_save_table(&mut self, path: &std::path::Path) {
        let (Some(table), Some(window)) = (self.processing_table(), &self.processing.dialog) else {
            return;
        };
        let label = window.tool.label.clone();
        match std::fs::write(path, crate::layer_list::csv(&table.lines())) {
            Ok(()) => {
                let file = path
                    .file_name()
                    .map_or_else(String::new, |f| f.to_string_lossy().into_owned());
                self.say(
                    Level::Success,
                    format!(
                        "{label}: tablo CSV olarak kaydedildi: {file} ({} satır).",
                        table.rows.len()
                    ),
                );
            }
            Err(e) => self.warn(format!(
                "{label}: tablo kaydedilemedi: {e}. Başka bir klasör seçin."
            )),
        }
    }

    /// Sonuçları seç: what the run made or changed becomes the selection,
    /// the window closes and the view goes to it (the web's).
    fn processing_results(&mut self) {
        let Some(window) = self.processing.dialog.take() else {
            return;
        };
        if let RunStatus::Ok { pick, selected, .. } = &window.status
            && !selected
        {
            let exists: Vec<Slot> = match &self.document {
                Some(doc) => pick
                    .iter()
                    .copied()
                    .filter(|id| doc.model.get(*id).is_some())
                    .collect(),
                None => Vec::new(),
            };
            self.selection.set(exists);
        }
        self.dialog = None;
        if !self.selection.is_empty() {
            self.zoom_selection();
        }
    }

    /// Sahneden seç: the window steps aside while a point is picked on the
    /// drawing (snaps apply), then opens again with it (the web's
    /// `pickPoint`); `then` chooses a choice's option with it.
    fn processing_pick(&mut self, name: String, then: Option<(String, String)>) {
        let Some(window) = self.processing.dialog.take() else {
            return;
        };
        let label = window
            .tool
            .parameters
            .iter()
            .find(|p| p.name == name)
            .map_or_else(|| name.clone(), |p| p.label.clone());
        self.say(Level::Command, format!("{}: {label}", window.tool.label));
        self.processing.picking = Some(Picking::Point { window, name, then });
        self.dialog = None;
        self.field = None;
        self.snap = None;
        self.session.run(Box::new(PickPoint::new("", label)));
        self.with_tool(|s, cx| s.activate(cx));
    }

    /// Sahneden seç for input objects: the window steps aside, the objects
    /// are picked on the drawing (the field's kinds only), and Enter brings
    /// the window back with them as its selection; Esc as it was.
    fn processing_pick_objects(&mut self, name: String) {
        let Some(window) = self.processing.dialog.take() else {
            return;
        };
        let Some(def) = window.tool.parameters.iter().find(|p| p.name == name) else {
            self.processing.dialog = Some(window);
            return;
        };
        // The kinds the field takes, narrowed to the kinds chosen with its chips.
        let taken: Option<Vec<String>> = match &def.kind {
            kentos_processing::ParamKind::Features { kinds, .. } => kinds.clone(),
            _ => None,
        };
        let chosen = kentos_processing::values::FeaturesValue::read(
            window.values.get(&name).unwrap_or(&Value::Null),
        )
        .and_then(|v| v.kinds);
        let kinds = chosen.or(taken);
        let label = def.label.clone();
        self.say(Level::Command, format!("{}: {label}", window.tool.label));
        let before: Vec<Slot> = self.selection.ids().to_vec();
        // A fresh pick: what was selected comes back on Esc.
        self.selection.clear();
        self.processing.picking = Some(Picking::Objects {
            window,
            name,
            before,
        });
        self.dialog = None;
        self.field = None;
        self.snap = None;
        self.session.run(Box::new(
            kentos_interaction::pick_objects::PickObjects::new(label, kinds),
        ));
        self.with_tool(|s, cx| s.activate(cx));
    }

    /// The point shown, or none (Esc): the window opens again as it was.
    /// Returns false when no processing window was waiting for it.
    pub(crate) fn processing_picked(&mut self, p: Option<Vec2>) -> bool {
        let picking = self.processing.picking.take();
        if let Some(Picking::Designer {
            designer,
            step,
            name,
            then,
        }) = picking
        {
            self.designer_picked(designer, step, name, then, p);
            return true;
        }
        let Some(Picking::Point {
            mut window,
            name,
            then,
        }) = picking
        else {
            self.processing.picking = picking;
            return false;
        };
        if let Some(p) = p {
            window
                .values
                .insert(name.clone(), json!({ "x": p.x, "y": p.y }));
            window.touch(&name);
            // The choice whose option the pick gives (the numbering's start vertex).
            if let Some((choice, option)) = then {
                window.values.insert(choice.clone(), json!(option));
                window.touch(&choice);
            }
        }
        self.processing.dialog = Some(window);
        self.dialog = Some(Dialog::Processing);
        self.refresh_processing();
        true
    }

    /// The objects picked (kept) or not (Esc): the window opens again, its
    /// field on the selection that holds them. Returns false when no
    /// processing window was waiting for them.
    pub(crate) fn processing_picked_objects(&mut self, keep: bool) -> bool {
        let Some(Picking::Objects {
            mut window,
            name,
            before,
        }) = self.processing.picking.take()
        else {
            return false;
        };
        if keep && !self.selection.is_empty() {
            // Its kinds stay as they were chosen; the scope is the selection.
            let mut value = json!({ "scope": "selection" });
            if let Some(kinds) = window
                .values
                .get(&name)
                .and_then(|v| v.get("kinds"))
                .cloned()
            {
                value["kinds"] = kinds;
            }
            window.values.insert(name.clone(), value);
            window.touch(&name);
            self.say(
                Level::Info,
                format!("{} nesne seçildi.", self.selection.len()),
            );
        } else {
            self.selection.set(before);
        }
        self.processing.dialog = Some(window);
        self.dialog = Some(Dialog::Processing);
        self.refresh_processing();
        true
    }
}
