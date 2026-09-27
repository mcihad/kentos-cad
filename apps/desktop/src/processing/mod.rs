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
//! - Each tool's last values outlive the program in `islemler.json`
//!   ([`memory`]).

pub(crate) mod designer;
pub mod dialog;
mod fields;
pub mod memory;
pub mod panel;
#[cfg(test)]
mod tests;
mod window;

use iced::Task;
use kentos_domain::Slot;
use kentos_interaction::pick::PickPoint;
use kentos_interaction::{Level, Selection, Vec2};
use kentos_processing::model_runner::{MODEL_PREFIX, run_model};
use kentos_processing::{
    Bounds, Defaults, Host, LogLine, Outcome, Registry, Runner, Scene, Store, Values,
};
use serde_json::{Value, json};

use crate::app::{App, Dialog, Message};
pub use dialog::{RunStatus, ToolDialog};

/// Whether İşlemler answers a command: a tool, a model, or Harita's Kenar ölçülerini yaz.
pub fn answers(id: &str) -> bool {
    id.starts_with("processing.run.")
        || id.starts_with("processing.model.")
        || matches!(
            id,
            "map.edgeLengths" | "processing.toolbox" | "processing.history" | "processing.newModel"
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
    /// A parameter's value chosen: a choice, a switch, a scope, a layer, kinds.
    Value(String, Value),
    /// A number field's text as typed.
    Number(String, String),
    /// A text, field or expression field's text as typed.
    Text(String, String),
    /// A new layer's name as typed.
    LayerName(String, String),
    /// A features parameter's scope chosen: the parameter and the scope's id.
    Scope(String, String),
    /// Nerede çalışır chosen (the desktop runs every tool here: kept for the web's parity).
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
            self.error(format!("İşlem aracı bulunamadı: {id}"));
            return;
        };
        let Some(doc) = &self.document else {
            return;
        };
        let explicit = values.is_some();
        let stored = values.or_else(|| self.processing.memory.last_values(&tool.id).cloned());
        let window = ToolDialog::new(tool, stored.as_ref(), explicit, &Defaults::of(&doc.model));
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
            Event::Run => {
                self.run_processing();
                return Task::none();
            }
            Event::Results => {
                self.processing_results();
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
    /// tool runs; the window stays open with what it did.
    fn run_processing(&mut self) {
        let Some(window) = &mut self.processing.dialog else {
            return;
        };
        let (Some(doc), true) = (&mut self.document, window.attempt()) else {
            self.refresh_processing();
            return;
        };
        let values = window.values.clone();
        let tool = window.tool.clone();
        self.processing.memory.remember(&tool.id, &values);
        let mut stage = Stage {
            doc: &mut doc.model,
            selection: &mut self.selection,
            store: self.spatial.store(),
            view: Some(self.viewport.camera.visible_bounds()),
        };
        let mut log: Vec<LogLine> = Vec::new();
        let registry = &self.processing.registry;
        let outcome = match tool
            .id
            .strip_prefix(MODEL_PREFIX)
            .and_then(|id| registry.model(id))
        {
            Some(model) => {
                let lookup = |id: &str| registry.tool(id);
                run_model(
                    model,
                    &values,
                    &mut self.processing.runner,
                    &mut stage,
                    &lookup,
                    &mut log,
                )
            }
            None => self
                .processing
                .runner
                .run(&mut stage, &tool, &values, false, &mut log),
        };
        for line in log {
            match line.level {
                kentos_processing::Level::Info => self.say(Level::Info, line.text),
                kentos_processing::Level::Warn => self.warn(line.text),
            }
        }
        match &outcome {
            Outcome::Ok { record, .. } => {
                self.say(
                    Level::Success,
                    format!("{}: {}", tool.label, record.summary),
                );
            }
            Outcome::Stopped { message, .. } => self.warn(message.clone()),
            Outcome::Invalid { .. } => {}
        }
        if let Some(window) = &mut self.processing.dialog {
            window.ran(outcome);
        }
        self.refresh_processing();
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
