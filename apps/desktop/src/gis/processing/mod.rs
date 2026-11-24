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
//! - Toplu çalıştır ([`batch`], docs/adr/0529): the open tool or model over a
//!   table of rows of parameter values, run as `cad.processing.batch`.
//! - Ayrı pencerede aç ([`apart`], docs/adr/8060): the window in an operating-system
//!   window of its own, beside the main one, the drawing free while it runs.

pub(crate) mod apart;
#[cfg(test)]
mod apart_tests;
pub(crate) mod background;
pub(crate) mod batch;
pub(crate) mod data_select;
pub(crate) mod designer;
pub mod dialog;
mod fields;
pub(crate) mod overwrite;
#[cfg(test)]
mod overwrite_tests;
pub mod memory;
pub mod panel;
mod plan;
pub(crate) use plan::ServerPlace;
mod server;
pub(crate) mod environment;
pub(crate) mod sink;

/// The İşlemler panel's event as the app's message (the ribbon's model buttons send it too).
pub(crate) fn panel_message(event: panel::Event) -> crate::app::Message {
    crate::app::Message::Gis(gis::Event::Processing(Event::Panel(event)))
}
#[cfg(test)]
mod batch_tests;
#[cfg(test)]
mod convert_tests;
#[cfg(test)]
mod distance_tests;
#[cfg(test)]
mod generalize_tests;
#[cfg(test)]
mod generate_tests;
#[cfg(test)]
mod geometry_tests;
#[cfg(test)]
mod input_layer_tests;
#[cfg(test)]
mod hydrology_tests;
#[cfg(test)]
mod interpolation_tests;
#[cfg(test)]
mod layer_tools_tests;
#[cfg(test)]
mod lrs_tests;
#[cfg(test)]
mod proximity_tests;
#[cfg(test)]
mod query_tests;
#[cfg(test)]
mod raster_ops_tests;
#[cfg(test)]
mod raster_vector_tests;
#[cfg(test)]
mod remote_tests;
#[cfg(test)]
mod sampling_tests;
#[cfg(test)]
mod server_tests;
#[cfg(test)]
mod environment_tests;
#[cfg(test)]
mod sink_tests;
#[cfg(test)]
mod stats_tests;
#[cfg(test)]
mod suitability_tests;
#[cfg(test)]
pub(crate) mod surface_tests;
#[cfg(test)]
mod table_points_tests;
#[cfg(test)]
pub(crate) mod tests;
#[cfg(test)]
mod transfer_tests;
pub mod window;

use crate::gis;
use iced::Task;
use iced::futures::channel::mpsc;
use kentos_domain::Slot;
use kentos_expression::Variable;
use kentos_interaction::pick::PickPoint;
use kentos_interaction::{Level, Selection, Vec2};
use kentos_processing::model_runner::{MODEL_PREFIX, end_model, replay_model, run_model};
use kentos_processing::{
    Bounds, Defaults, Host, LogLine, Outcome, Prepared, Registry, Runner, Scene, Store, Target,
    Values,
};
use serde_json::{Value, json};

use crate::app::{App, Message};
pub use dialog::{RunStatus, ToolDialog};

/// Said when a model's drawing changed while it ran on another thread.
const CHANGED_MEANWHILE: &str =
    "Çizim model çalışırken değişti; model çizimin şimdiki hâlinde yeniden çalıştırıldı.";

/// Whether a message is a background run's last word (perf::frame waits for it).
#[cfg(test)]
pub(crate) fn is_answer(message: &Message) -> bool {
    matches!(
        message,
        Message::Gis(gis::Event::Processing(Event::Background(
            _,
            background::Reply::Done | background::Reply::Failed(_)
        )))
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
    /// Toplu çalıştır, over the open window (batch.rs): its table, and the job while one runs.
    pub(crate) batch: Option<batch::BatchState>,
    /// Runs going on other threads (background.rs).
    pub(crate) running: Vec<background::Running>,
    /// The last background run's id.
    runs: u64,
    /// The request id of a server job asked and not answered yet (server.rs): asked again with it after a failure.
    pub(crate) server_request: Option<String>,
    /// The ask on its way.
    pub(crate) server_asked: Option<u64>,
    /// The raster tools' analysis environment for the session (environment.rs, docs/adr/0643 §5).
    pub(crate) environment: environment::LocalEnvironment,
    /// İfadeyle seç asking a data layer's source (data_select.rs, docs/adr/9133).
    pub(crate) data_select: data_select::DataSelect,
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
            batch: None,
            running: Vec::new(),
            runs: 0,
            server_request: None,
            server_asked: None,
            environment: environment::LocalEnvironment::default(),
            data_select: data_select::DataSelect::default(),
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
    /// Open the current selection query in the existing shareable model designer.
    SelectionRecipe,
    Close,
    /// Varsayılanlar: every field back to its default.
    Reset,
    Run,
    /// Durdur: the window's background run stops.
    Stop,
    /// İfadeyle seç's answer from a data layer's source (data_select.rs).
    DataSelect(data_select::SelectEvent),
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
    /// A chosen file read off the UI thread (docs/adr/0223 §1): its value, or why not.
    FileRead(String, Result<Value, String>),
    /// A workbook's sheet chosen for a file parameter.
    Sheet(String, String),
    /// Konum…: where a result is written asked for (docs/adr/0207 §7).
    ChooseSave(String),
    /// Panoya kopyala for the run's table (Özet istatistik).
    CopyTable,
    /// Grafik for the run's table (docs/adr/0227 §4): the bottom panel's Grafik tab shows it.
    ChartTable,
    /// Kaydet ▾ for the run's table (docs/adr/0223 §2): written in a format.
    SaveTableAs(kentos_commands::WorkbookFormat),
    /// Sonuçları seç, or Seçime yakınlaştır after a selecting tool.
    Results,
    /// Geri al after a run.
    Undo,
    /// The dock's İşlemler tab.
    Panel(panel::Event),
    /// Toplu çalıştır (batch.rs).
    Batch(batch::Event),
    /// On the server, the result goes to a GeoPackage (true) or to the project (server.rs, docs/adr/0559 §5).
    ServerSink(bool),
    /// Sunucu işleri on the line after a job was asked: the catalog on the project's jobs.
    ServerJobs,
    /// Sonuç and Bütün tablo (sink.rs, docs/adr/0592 §3).
    Sink(sink::SinkEvent),
    /// Analiz ortamı (environment.rs, docs/adr/0643 §5).
    Environment(environment::EnvironmentEvent),
}

/// The app as the window reads it: the live descriptions and previews.
pub(crate) struct Look<'a> {
    pub doc: &'a kentos_domain::Document,
    pub selection: &'a Selection,
    pub store: &'a Store,
    pub view: Option<Bounds>,
    /// The `@` values the expressions read (docs/adr/0214 §2.3).
    pub variables: Vec<Variable>,
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

    fn variables(&self) -> Vec<Variable> {
        self.variables.clone()
    }
}

/// The app as a run changes it.
struct Stage<'a> {
    doc: &'a mut kentos_domain::Document,
    selection: &'a mut Selection,
    store: &'a Store,
    view: Option<Bounds>,
    variables: Vec<Variable>,
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

    fn variables(&self) -> Vec<Variable> {
        self.variables.clone()
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
            self.gis.processing.panel.tab = tab;
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
    pub fn open_processing(&mut self, id: &str, values: Option<Values>) {
        let Some(tool) = self.gis.processing.registry.get(id) else {
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
        self.gis.processing.batch = None;
        // The last values come back, a kind narrowing not: the chips narrow what was in scope then, and brought
        // back unseen, Çizgi alone refused the polygon on Çizim the window called eligible (the owner, 19 November
        // 2026). Values given (a model's step, a batch row) stay as they are.
        let stored = values.or_else(|| {
            let last = self.gis.processing.memory.last_values(&tool.id).cloned();
            // What the run reads follows the drawing now, not the last run (docs/adr/9144): the selection when there
            // is one, else the active layer; the last run's result name does not come back.
            Some(self.inputs_now(&tool, last.map(without_kind_narrowing).unwrap_or_default()))
        });
        let mut window =
            ToolDialog::new(tool, stored.as_ref(), explicit, &Defaults::of(&doc.model));

        window.propose_layers(&doc.model, stored.as_ref());
        window.reread_files();
        window.environment = self.gis.processing.environment.clone();
        window.takes_environment = self.processing_takes_environment(&window.tool);
        self.gis.processing.dialog = Some(window);
        self.refresh_processing();
        self.show_processing();
    }

    /// The open window's new layers keep the tool's own names (a test replaying a shared case, docs/adr/9189).
    #[cfg(any(test, feature = "testing"))]
    pub fn keep_tool_names(&mut self) {
        if let Some(w) = self.gis.processing.dialog.as_mut() {
            w.keep_tool_names();
        }
        self.refresh_processing();
    }

    /// The window's live parts again: problems, what the inputs resolve to, previews.
    pub(crate) fn refresh_processing(&mut self) {
        let variables = self.expression_variables();
        let (Some(doc), Some(window)) = (&self.document, &mut self.gis.processing.dialog) else {
            return;
        };
        let look = Look {
            doc: &doc.model,
            selection: &self.selection,
            store: self.spatial.store(),
            view: Some(self.viewport.camera.visible_bounds()),
            variables,
        };
        // The new layer named after what the run reads, while no name was typed (docs/adr/9189).
        window.derive_names(&doc.model, &self.selection);
        window.refresh(&self.gis.processing.runner, &look);
        window
            .sink
            .refresh(&doc.model, &window.tool, &window.values);
        let folder = doc.path.as_deref().and_then(std::path::Path::parent);
        window.overwrites = overwrite::overwrites(&doc.model, folder, &window.tool, &window.values);
        // İfadeyle seç on a data layer: its input is the source's rows, its preview counts them (data_select.rs,
        // docs/adr/9133).
        let (input, line) = (self.data_select_input(), self.data_select_preview());
        if let Some(window) = &mut self.gis.processing.dialog {
            if let Some(input) = input {
                window.inputs.insert("input".to_owned(), input);
            }
            if let Some(line) = line {
                window.previews.insert("condition".to_owned(), line);
            }
        }
    }

    pub(crate) fn processing_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::SelectionRecipe => {
                self.selection_recipe_from_dialog();
                return Task::none();
            }
            Event::Close => {
                self.gis.processing.dialog = None;
                self.gis.processing.batch = None;
                self.processing_dialog_away();
                // İfade oluşturucu of one of its fields goes with it: apart, it was over the main window.
                if self
                    .gis
                    .builder
                    .as_ref()
                    .is_some_and(|b| matches!(b.target, crate::expression::Target::Processing(_)))
                {
                    self.gis.builder = None;
                }
                return Task::none();
            }
            Event::Batch(e) => return self.processing_batch_event(e),
            Event::ServerSink(sink) => {
                if let Some(window) = &mut self.gis.processing.dialog {
                    window.server_sink = sink;
                }
                return Task::none();
            }
            Event::ServerJobs => return self.processing_open_jobs(),
            Event::Sink(e) => return self.processing_sink_event(e),
            Event::Environment(e) => {
                self.gis.processing.environment.apply(e);
                if let Some(window) = &mut self.gis.processing.dialog {
                    window.environment = self.gis.processing.environment.clone();
                }
                return Task::none();
            }
            Event::Run => return self.run_processing(),
            Event::Stop => {
                self.processing_stop();
                return Task::none();
            }
            Event::DataSelect(e) => return self.data_select_event(e),
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
            Event::FileChosen(_, None) => return Task::none(),
            Event::FileChosen(name, Some((file, path, bytes))) => {
                self.processing_file_read(name, dialog::file_value(&file, &path, &bytes, None));
            }
            Event::FileRead(name, read) => self.processing_file_read(name, read),
            Event::Sheet(name, sheet) => {
                let value = self
                    .gis
                    .processing
                    .dialog
                    .as_ref()
                    .and_then(|w| w.values.get(&name))
                    .and_then(|v| dialog::with_sheet(v, &sheet));
                if let Some(value) = value {
                    self.processing_file_read(name, Ok(value));
                }
            }
            Event::CopyTable => {
                let Some(table) = self.processing_table() else {
                    return Task::none();
                };
                self.say(Level::Success, "Tablo panoya kopyalandı.".to_owned());
                return iced::clipboard::write(crate::layer_list::tsv(&table.lines()));
            }
            Event::ChartTable => {
                let Some(window) = &self.gis.processing.dialog else {
                    return Task::none();
                };
                let Some(table) = window.chart.clone().or_else(|| self.processing_table()) else {
                    return Task::none();
                };
                let chart = crate::charts::ChartTable {
                    title: window.tool.label.clone(),
                    tool: window.tool.id.clone(),
                    columns: table.columns,
                    rows: table.rows,
                };
                // The window out of the chart's way, as Sonuçları seç leaves it; in its own window it is in nobody's
                // way and stays (apart.rs).
                if !self.processing_is_apart() {
                    self.gis.processing.dialog = None;
                    self.processing_dialog_away();
                }
                self.open_table_chart(chart);
            }
            Event::SaveTableAs(format) => {
                let (Some(table), Some(window)) =
                    (self.processing_table(), &self.gis.processing.dialog)
                else {
                    return Task::none();
                };
                let label = window.tool.label.clone();
                // The columns read as a file's table reads them (docs/adr/9190): a column of numbers a number, any
                // other text (a code “007” stays “007”).
                let (kinds, _) = kentos_processing::table_kinds::read(&table.rows);
                let sheet = kentos_commands::WorkbookSheet {
                    name: label.clone(),
                    columns: table
                        .columns
                        .iter()
                        .enumerate()
                        .map(|(j, name)| kentos_commands::WorkbookColumn {
                            name: name.clone(),
                            kind: match kinds.get(j) {
                                Some(kentos_native_application::geopackage::CellKind::Text) => {
                                    kentos_commands::CellKind::Text
                                }
                                Some(_) => kentos_commands::CellKind::Number,
                                None => kentos_commands::CellKind::Auto,
                            },
                            decimals: None,
                        })
                        .collect(),
                    rows: table.rows,
                };
                return crate::table_write::save(vec![sheet], format, label);
            }
            Event::Undo => {
                self.undo();
                if let Some(window) = &mut self.gis.processing.dialog {
                    window.status = RunStatus::Idle;
                }
            }
            Event::Pick(name) => {
                self.processing_pick(name, None);
                return Task::none();
            }
            Event::PickChoice(name) => {
                let picks = self.gis.processing.dialog.as_ref().and_then(|w| {
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
                if let Some(window) = &self.gis.processing.dialog {
                    self.gis
                        .processing
                        .memory
                        .set_target(&window.tool.id, &choice);
                }
                return Task::none();
            }
            Event::Reset => {
                if let (Some(doc), Some(window)) = (&self.document, &mut self.gis.processing.dialog)
                {
                    window.reset(&Defaults::of(&doc.model));
                    window.propose_layers(&doc.model, None);
                }
            }
            e => {
                let active = self
                    .document
                    .as_ref()
                    .map(|d| d.model.layers().active().to_owned());
                if let Some(window) = &mut self.gis.processing.dialog {
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
        let variables = self.expression_variables();
        let server = self.processing_server_place();
        // Analiz ortamı (environment.rs): a number that does not read keeps the run from starting.
        let environment = match self.processing_environment_resolved() {
            Ok(e) => e,
            Err(why) => {
                if let Some(window) = &mut self.gis.processing.dialog {
                    window.status = RunStatus::Invalid(why);
                }
                return Task::none();
            }
        };
        let Some(window) = &mut self.gis.processing.dialog else {
            return Task::none();
        };
        if window.running() {
            return Task::none();
        }
        let attempted = window.attempt();
        // İfadeyle seç on a data layer asks its source, not the drawing (data_select.rs, docs/adr/9133).
        if attempted && let Some(task) = self.processing_data_select_run() {
            return task;
        }
        let Some(window) = &mut self.gis.processing.dialog else {
            return Task::none();
        };
        let (Some(doc), true) = (&mut self.document, attempted) else {
            self.refresh_processing();
            return Task::none();
        };
        let values = window.values.clone();
        let tool = window.tool.clone();
        let targets = window.targets(&self.gis.processing.registry, &server);
        let choice = plan::effective_choice(
            plan::Choice::read(self.gis.processing.memory.target(&tool.id)),
            &targets.available,
        );
        // On the server (server.rs): the tool does not run here; a job is asked for.
        if choice == Some(plan::Choice::At(Target::Server)) {
            return self.processing_server_run();
        }
        // To a file or a database (sink.rs): `cad.processing.run` on a copy, on another thread.
        if window.sink.place != sink::Place::Drawing && !tool.id.starts_with(MODEL_PREFIX) {
            return self.processing_sink_run();
        }
        // Otomatik's answer for the whole: a tool's is taken again from its
        // job below, its locked objects left out (the web's).
        // An expression that calls an extension's function (`ext.…`) runs in the background whatever the choice: its
        // answers come from the extension's process, and the window never waits for it (extensions/tools.rs).
        let asks_extension = tool.targets.contains(&Target::Worker)
            && tool.parameters.iter().any(|p| {
                matches!(p.kind, kentos_processing::ParamKind::Expression { .. })
                    && values
                        .get(&p.name)
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|s| s.contains("ext."))
            });
        let background = |auto: Option<Target>| {
            asks_extension
                || match choice {
                    Some(plan::Choice::At(Target::Worker)) => true,
                    Some(plan::Choice::Auto) => auto == Some(Target::Worker),
                    _ => false,
                }
        };
        // The last values keep a file's name and path, not its rows (docs/adr/0200 §7).
        self.gis.processing.memory.remember(
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
            variables,
        };
        let mut log: Vec<LogLine> = Vec::new();
        let registry = &self.gis.processing.registry;
        let runner = &mut self.gis.processing.runner;
        runner.environment = environment.clone();
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
                environment,
            };
            self.gis.processing.runs += 1;
            background::start_model(
                self.gis.processing.runs,
                run,
                stage.doc.reading_copy(),
                stage.selection.ids().to_vec(),
                stage.view,
                session,
                Some(files),
                stage.variables.clone(),
                |id, reply| Message::Gis(gis::Event::Processing(Event::Background(id, reply))),
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
            self.gis.processing.runs += 1;
            background::start_tool(
                self.gis.processing.runs,
                job,
                stage.doc.reading_copy(),
                session,
                Some(files),
                |id, reply| Message::Gis(gis::Event::Processing(Event::Background(id, reply))),
            )
        };
        let id = running.id;
        self.gis.processing.running.push(running);
        // What resolving the inputs left out, said as the run starts (the web's).
        for line in log {
            self.say_line(line);
        }
        if let Some(window) = &mut self.gis.processing.dialog {
            window.started(id);
        }
        task
    }

    /// After a run that gave a table, the form scrolled to its end so the table under it is seen (the web's).
    fn result_shown(&self) -> Task<Message> {
        match self.gis.processing.dialog.as_ref().map(|w| &w.status) {
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
            Outcome::Ok { record, added, .. } => {
                self.say(Level::Success, format!("{label}: {}", record.summary));
                // A result its layer's style draws nothing for (a categorized fill without Diğer değerler): said, so
                // a written result is not taken for a missing one.
                let uids: Vec<String> = self.document.as_ref().map_or_else(Vec::new, |d| {
                    added.iter().filter_map(|s| d.model.uid(*s)).map(|u| u.to_string()).collect()
                });
                if let Some(note) = self.unstyled_note(&uids) {
                    self.warn(format!("{label}: {note}"));
                }
            }
            Outcome::Stopped { message, .. } => self.warn(message.clone()),
            Outcome::Invalid { .. } => {}
        }
        let window = match waiting {
            Some(id) => self.window_waiting(id),
            None => self.gis.processing.dialog.as_mut(),
        };
        if let Some(window) = window {
            window.ran(outcome);
        }
        self.refresh_processing();
    }

    /// The window waiting for background run `id`: open, or put aside
    /// while one of its fields is picked on the drawing.
    fn window_waiting(&mut self, id: u64) -> Option<&mut ToolDialog> {
        let p = &mut self.gis.processing;
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
        let Some(at) = self.gis.processing.running.iter().position(|r| r.id == id) else {
            return;
        };
        match reply {
            background::Reply::Progress(share, step) => {
                if let Some(run) = self.gis.processing.running.get_mut(at) {
                    run.progress = Some((share, step.clone()));
                }
                if let Some(window) = self.window_waiting(id) {
                    window.progressed(share, step);
                }
            }
            background::Reply::Line(line) => self.say_line(line),
            background::Reply::Done => {
                let running = self.gis.processing.running.remove(at);
                let answer = running.answer().ok_or_else(|| "sonuç alınamadı".to_owned());
                self.processing_finish(running, answer);
            }
            background::Reply::Failed(why) => {
                let running = self.gis.processing.running.remove(at);
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
        let variables = self.expression_variables();
        let label = running.work.label();
        let background::Running {
            id, work, session, ..
        } = running;
        let mut log = Vec::new();
        let runner = &mut self.gis.processing.runner;
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
                variables,
            });
        let outcome = match (work, answer, &mut stage) {
            (Work::Tool(job), Ok(Answer::Tool(result)), Some(stage)) => {
                runner.finish(stage, *job, *result, false, &mut log)
            }
            (Work::Tool(job), Ok(_), None) => runner.stop(*job),
            (Work::Tool(job), Ok(Answer::Model { .. }), Some(_)) => {
                runner.failed(*job, "beklenmeyen bir sonuç geldi")
            }
            (Work::Tool(job), Err(why), _) => runner.failed(*job, &why),
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
        if self.processing_sink_stop() || self.processing_data_select_stop() {
            return;
        }
        let Some(id) = self.gis.processing.dialog.as_ref().and_then(|w| w.waiting) else {
            return;
        };
        self.processing_stop_run(id);
    }

    /// A background run stopped by its id: its window's Durdur, or Çalışan işler's (works/), its window open or not.
    pub(crate) fn processing_stop_run(&mut self, id: u64) {
        let Some(at) = self.gis.processing.running.iter().position(|r| r.id == id) else {
            return;
        };
        let running = self.gis.processing.running.remove(at);
        running.stop();
        let label = running.work.label();
        let runner = &mut self.gis.processing.runner;
        let outcome = match running.work {
            background::Work::Tool(job) => runner.stop(*job),
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
            .gis
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
            |read| read,
        )
        .then(move |read| match read {
            // Read on its own thread: a big workbook takes a while (docs/adr/0223 §1).
            Some((file, path, bytes)) => {
                let name = name.clone();
                iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
                    let read = dialog::file_value(&file, &path, &bytes, None);
                    let answer = Message::Gis(gis::Event::Processing(Event::FileRead(name, read)));
                    let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(
                        &mut out, answer,
                    ));
                })
            }
            None => Task::none(),
        })
    }

    /// Konum… (docs/adr/0207 §7): where a result goes, asked with the
    /// parameter's extensions; the path becomes the parameter's value.
    fn processing_choose_save(&mut self, name: String) -> Task<Message> {
        let accept: Vec<String> = self
            .gis
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
                Some(p) => {
                    Message::Gis(gis::Event::Processing(Event::Value(name.clone(), json!(p))))
                }
                // Given up: nothing changes.
                None => Message::Gis(gis::Event::Processing(Event::FileChosen(
                    name.clone(),
                    None,
                ))),
            },
        )
    }

    /// A chosen file's value (its sheet's rows) the parameter's; what was
    /// left out of it is said; a file that cannot be read is said and
    /// changes nothing.
    fn processing_file_read(&mut self, name: String, read: Result<Value, String>) {
        match read {
            Ok(value) => {
                for why in value["leftOut"].as_array().into_iter().flatten() {
                    if let Some(why) = why.as_str() {
                        self.warn(why.to_owned());
                    }
                }
                let active = self
                    .document
                    .as_ref()
                    .map(|d| d.model.layers().active().to_owned());
                if let Some(window) = &mut self.gis.processing.dialog {
                    window.edit(Event::Value(name, value), active.as_deref().unwrap_or(""));
                }
            }
            Err(why) => self.warn(why),
        }
    }

    /// The open window's last run's table.
    fn processing_table(&self) -> Option<dialog::ResultTable> {
        match &self.gis.processing.dialog.as_ref()?.status {
            RunStatus::Ok { table, .. } => table.clone(),
            _ => None,
        }
    }

    /// Sonuçları seç: what the run made or changed becomes the selection,
    /// the window over the drawing closes and the view goes to it (the web's).
    fn processing_results(&mut self) {
        let Some(window) = &self.gis.processing.dialog else {
            return;
        };
        let pick = match &window.status {
            RunStatus::Ok { pick, selected, .. } if !selected => Some(pick.clone()),
            _ => None,
        };
        // Over the drawing the window steps aside to show them; in its own window it is in nobody's way and stays
        // (apart.rs, docs/adr/8060).
        if !self.processing_is_apart() {
            self.gis.processing.dialog = None;
            self.processing_dialog_away();
        }
        if let Some(pick) = pick {
            let exists: Vec<Slot> = match &self.document {
                Some(doc) => pick
                    .into_iter()
                    .filter(|id| doc.model.get(*id).is_some())
                    .collect(),
                None => Vec::new(),
            };
            self.selection.set(exists);
        }
        // The drawing's selection framed, else a data layer's rows İfadeyle seç selected (docs/adr/9133).
        self.zoom_selection();
    }

    /// Sahneden seç: the window steps aside while a point is picked on the
    /// drawing (snaps apply), then opens again with it (the web's
    /// `pickPoint`); `then` chooses a choice's option with it.
    fn processing_pick(&mut self, name: String, then: Option<(String, String)>) {
        let Some(window) = self.gis.processing.dialog.take() else {
            return;
        };
        let label = window
            .tool
            .parameters
            .iter()
            .find(|p| p.name == name)
            .map_or_else(|| name.clone(), |p| p.label.clone());
        self.say(Level::Command, format!("{}: {label}", window.tool.label));
        self.gis.processing.picking = Some(Picking::Point { window, name, then });
        self.processing_dialog_away();
        self.field = None;
        self.snap = None;
        self.session.run(Box::new(PickPoint::new("", label)));
        self.with_tool(|s, cx| s.activate(cx));
    }

    /// Sahneden seç for input objects: the window steps aside, the objects
    /// are picked on the drawing (the field's kinds only), and Enter brings
    /// the window back with them as its selection; Esc as it was.
    fn processing_pick_objects(&mut self, name: String) {
        let Some(window) = self.gis.processing.dialog.take() else {
            return;
        };
        let Some(def) = window.tool.parameters.iter().find(|p| p.name == name) else {
            self.gis.processing.dialog = Some(window);
            return;
        };
        // The tool's own kinds, the window's Uygun nesneler: a pick chooses each object by hand, so a narrowing chosen
        // with the kind chips does not refuse what the window calls eligible. Remembered in islemler.json and hidden
        // while nothing was in scope, Çizgi alone refused the polygon (the owner, 19 November 2026).
        let kinds: Option<Vec<String>> = match &def.kind {
            kentos_processing::ParamKind::Features { kinds, .. } => kinds.clone(),
            _ => None,
        };
        let label = def.label.clone();
        self.say(Level::Command, format!("{}: {label}", window.tool.label));
        let before: Vec<Slot> = self.selection.ids().to_vec();
        // A fresh pick: what was selected comes back on Esc.
        self.selection.clear();
        self.gis.processing.picking = Some(Picking::Objects {
            window,
            name,
            before,
        });
        self.processing_dialog_away();
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
        let picking = self.gis.processing.picking.take();
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
            self.gis.processing.picking = picking;
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
        self.gis.processing.dialog = Some(window);
        self.show_processing();
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
        }) = self.gis.processing.picking.take()
        else {
            return false;
        };
        if keep && !self.selection.is_empty() {
            // The objects picked are the field's: the scope is the selection and a kind narrowing goes; the chips
            // narrow the picked objects again when they hold two kinds or more.
            let value = json!({ "scope": "selection" });
            window.values.insert(name.clone(), value);
            window.touch(&name);
            self.say(
                Level::Info,
                format!("{} nesne seçildi.", self.selection.len()),
            );
        } else {
            self.selection.set(before);
        }
        self.gis.processing.dialog = Some(window);
        self.show_processing();
        self.refresh_processing();
        true
    }
}

/// Values without the kind narrowing of their features fields (a scope's `kinds`), the rest as it was.
impl App {
    /// The last values with each features input following the drawing now (docs/adr/9144): the selection when there is
    /// one and the input takes it, else the tool's own default; a result layer's remembered name goes (the tool's own
    /// comes back).
    fn inputs_now(&self, tool: &kentos_processing::Tool, mut values: Values) -> Values {
        let selected = !self.selection.is_empty();
        // The selection is the tool's first input of objects; another (an overlay, a second layer) keeps its own
        // default (docs/adr/9144 §2, 9149).
        let mut first = true;
        for p in &tool.parameters {
            match &p.kind {
                kentos_processing::ParamKind::Features { scopes, .. } => {
                    let offers = |k: kentos_processing::ScopeKind| scopes.as_ref().is_none_or(|s| s.contains(&k));
                    let takes = first && selected && offers(kentos_processing::ScopeKind::Selection);
                    first = false;
                    if takes {
                        values.insert(p.name.clone(), json!({ "scope": "selection" }));
                    } else {
                        // The tool's own default (the active layer, or a layer holding what it takes): not the last
                        // run's layer.
                        values.remove(&p.name);
                    }
                }
                kentos_processing::ParamKind::Layer { .. }
                    if values.get(&p.name).and_then(|v| v.get("newName")).is_some() =>
                {
                    values.remove(&p.name);
                }
                _ => {}
            }
        }
        values
    }
}

fn without_kind_narrowing(mut values: Values) -> Values {
    for value in values.values_mut() {
        if let Some(o) = value.as_object_mut()
            && o.contains_key("scope")
        {
            o.remove("kinds");
        }
    }
    values
}

#[cfg(test)]
#[path = "result_name_tests.rs"]
mod result_name_tests;
#[cfg(test)]
#[path = "buffer_cases_tests.rs"]
mod buffer_cases_tests;
