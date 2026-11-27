//! Every other processing tool over data layers' rows (the second round's T28, T29; docs/adr/9353). A data layer's
//! rows are its source's, not the drawing's objects (docs/adr/9101): Tampon, Anahtarla birleştir and the rest found no
//! object to take with only data layers on the map, and a point drawn in the edit session was taken for the whole
//! layer.
//!
//! The tool's inputs are read on the copy of the drawing that holds the rows (data_location.rs `Located`: the source's
//! rows less those the edit session holds, then the drawing's own objects on the layer, the new and the changed
//! ones): a row added in the session is taken, a changed one once, a deleted one not. The run writes its result into
//! the drawing, as any run does: a new layer and its objects, one undo step. A tool that changes its input in place
//! does not take a data layer's rows (they are changed in the layer's edit session): said before it runs, with what to
//! do instead.
//!
//! The window counts the inputs on the copy; their lines name the data layers they take with their rows and what waits
//! in the session.

use kentos_domain::Slot;
use kentos_expression::Variable;
use kentos_interaction::Selection;
use kentos_processing::{Bounds, FeaturesValue, Host, LogLine, ParamKind, Prepared, Scene, Scope, Store};

use super::dialog::RunStatus;
use crate::app::{App, Message};

/// The words.
pub(crate) mod words {
    pub const WRITES: &str = "Bu araç girdisini yerinde değiştirir; veri katmanının satırları burada değiştirilmez, \
                              kaynağın satırlarıdır. Katmanı Düzenle sekmesinde düzenlemeye açıp değişikliği orada \
                              yapın ya da girdi olarak bir çizim katmanı seçin.";
}

/// A run over the rows: the inputs read on the copy, the result written into the drawing.
struct RowsStage<'a> {
    read: &'a kentos_domain::Document,
    /// The rows chosen in the table, as objects of the copy.
    chosen: &'a [Slot],
    doc: &'a mut kentos_domain::Document,
    selection: &'a mut Selection,
    view: Option<Bounds>,
    variables: Vec<Variable>,
}

impl Scene for RowsStage<'_> {
    fn doc(&self) -> &kentos_domain::Document {
        self.read
    }

    fn selected(&self) -> Vec<Slot> {
        let mut out = self.selection.ids().to_vec();
        out.extend(self.chosen.iter().copied());
        out
    }

    fn visible_bounds(&self) -> Option<Bounds> {
        self.view
    }

    fn store(&self) -> Option<&Store> {
        None
    }

    fn variables(&self) -> Vec<Variable> {
        self.variables.clone()
    }
}

impl Host for RowsStage<'_> {
    fn doc_mut(&mut self) -> &mut kentos_domain::Document {
        self.doc
    }

    fn select(&mut self, ids: &[Slot]) {
        // What the run made, in the drawing; a row of the copy is no object of it.
        let doc = &*self.doc;
        self.selection.set(ids.iter().copied().filter(|s| doc.get(*s).is_some()));
    }
}

impl App {
    /// Whether the open tool takes a data layer's rows into an input it changes in place.
    fn rows_into_writes(&self) -> bool {
        let (Some(window), Some(doc)) = (self.gis.processing.dialog.as_ref(), self.document.as_ref()) else {
            return false;
        };
        let tree = doc.model.layers();
        let data: Vec<&kentos_commands::LayerNode> = tree.leaves().into_iter().filter(|l| l.data.is_some()).collect();
        let chosen = self.gis.features.data.selected_layer().is_some() && !self.gis.features.data.selected.is_empty();
        window.tool.parameters.iter().any(|p| {
            let ParamKind::Features { writes: true, .. } = p.kind else {
                return false;
            };
            let Some(v) = window.values.get(&p.name).and_then(FeaturesValue::read) else {
                return false;
            };
            match v.scope {
                Scope::All => !data.is_empty(),
                Scope::Visible => data.iter().any(|l| tree.is_visible(&l.id)),
                Scope::Layer(id) => data.iter().any(|l| l.id == id),
                Scope::Selection => chosen,
                _ => false,
            }
        })
    }

    /// Çalıştır of another tool with a data layer among its inputs (see the module); none when it takes none, for the
    /// selecting tools (data_location.rs) and for a model.
    pub(super) fn processing_rows_run(&mut self) -> Option<iced::Task<Message>> {
        let window = self.gis.processing.dialog.as_ref()?;
        let id = window.tool.id.as_str();
        if matches!(id, super::data_location::TOOL | super::data_location::EXPRESSION)
            || id.starts_with(kentos_processing::model_runner::MODEL_PREFIX)
        {
            return None;
        }
        let located = self.data_location()?;
        if !located.reading.is_empty() {
            let why = format!("Kaynaktan okunuyor: {}. Satırlar okununca çalıştırın.", located.reading.join(", "));
            if let Some(window) = &mut self.gis.processing.dialog {
                window.status = RunStatus::Invalid(why);
            }
            return Some(iced::Task::none());
        }
        if self.rows_into_writes() {
            if let Some(window) = &mut self.gis.processing.dialog {
                window.status = RunStatus::Invalid(words::WRITES.to_owned());
            }
            return Some(iced::Task::none());
        }
        let environment = match self.processing_environment_resolved() {
            Ok(e) => e,
            Err(why) => {
                if let Some(window) = &mut self.gis.processing.dialog {
                    window.status = RunStatus::Invalid(why);
                }
                return Some(iced::Task::none());
            }
        };
        let variables = self.expression_variables();
        let view = Some(self.viewport.camera.visible_bounds());
        let window = self.gis.processing.dialog.as_ref()?;
        let (tool, values) = (window.tool.clone(), window.values.clone());
        self.gis.processing.memory.remember(
            &tool.id,
            &kentos_processing::parameters::stored_values(&tool, &values),
        );
        let doc = self.document.as_mut()?;
        let mut stage = RowsStage {
            read: &located.doc,
            chosen: located.chosen(),
            doc: &mut doc.model,
            selection: &mut self.selection,
            view,
            variables,
        };
        let mut log: Vec<LogLine> = Vec::new();
        let runner = &mut self.gis.processing.runner;
        runner.environment = environment;
        let outcome = match runner.prepare(&stage, &tool, &values, false, &mut log) {
            Prepared::Done(outcome) => outcome,
            Prepared::Ready(job) => runner.complete(&mut stage, job, &mut log),
        };
        self.processing_ran(&tool.label, None, log, outcome);
        Some(self.result_shown())
    }
}

#[cfg(test)]
#[path = "data_inputs_tests.rs"]
mod data_inputs_tests;
