//! A form's related rows from data layers' sources (docs/adr/9165): asked when a record's form opens, read off the
//! interface's thread, put into the form's world when they come, so a related item over a data layer lists the rows
//! its source holds with the record's code. An answer to an earlier form is stale; a source that cannot be read is
//! said under the item, the drawing's objects shown still.

use super::*;
use kentos_native_application::forms_source::{self, SourceRows};

/// The rows read for a form: the ask they answer and each layer's rows or why not.
pub type RowsRead = (
    u64,
    Vec<(String, Result<Vec<(i64, kentos_forms::Attrs)>, String>)>,
);

impl App {
    /// The open form's related rows asked of data layers' sources, when it has an item over one tied by a parent key.
    pub(crate) fn ask_entry_rows(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(e) = self.gis.form_entry.as_ref() else {
            return Task::none();
        };
        if e.preview || !e.members.is_empty() {
            return Task::none();
        }
        let asks = forms_source::source_asks(&doc.model, &e.body, &e.screen.layer, &e.screen.attrs);
        let nodes: Vec<(forms_source::SourceAsk, kentos_commands::LayerNode)> = asks
            .into_iter()
            .filter_map(|a| Some((a.clone(), doc.model.layers().get(&a.layer)?.clone())))
            .collect();
        if nodes.is_empty() {
            return Task::none();
        }
        let settings = doc.model.settings().clone();
        self.gis.form_rows_asked += 1;
        let asked = self.gis.form_rows_asked;
        if let Some(e) = self.gis.form_entry.as_mut() {
            e.source_rows_asked = asked;
        }
        crate::db_connections::off_thread(
            move || {
                nodes
                    .iter()
                    .map(|(a, n)| {
                        (
                            a.layer.clone(),
                            forms_source::read_related(n, &settings, &a.field, &a.value),
                        )
                    })
                    .collect::<Vec<_>>()
            },
            move |answers| Message::Gis(gis::Event::FormSourceRows(Box::new((asked, answers)))),
        )
    }

    /// The rows read: kept by the form (a layer's rows of several asks once each) and its screen prepared again over
    /// them; why a source could not be read kept to be said.
    pub(crate) fn entry_rows_read(&mut self, read: RowsRead) -> Task<Message> {
        let (asked, answers) = read;
        let Some(e) = self.gis.form_entry.as_mut() else {
            return Task::none();
        };
        if e.source_rows_asked != asked {
            return Task::none();
        }
        let mut rows = SourceRows::new();
        let mut said = Vec::new();
        for (layer, answer) in answers {
            match answer {
                Ok(list) => {
                    let into = rows.entry(layer).or_default();
                    for (key, attrs) in list {
                        if !into.iter().any(|(k, _)| *k == key) {
                            into.push((key, attrs));
                        }
                    }
                }
                Err(why) => said.push((layer, why)),
            }
        }
        e.source_rows = rows;
        e.source_rows_said = said;
        self.entry_reprepare();
        Task::none()
    }

    /// The open form's screen prepared again over its source rows (its related items list them).
    pub(super) fn entry_reprepare(&mut self) {
        let (Some(doc), Some(e)) = (&self.document, self.gis.form_entry.as_mut()) else {
            return;
        };
        match kentos_native_application::forms::prepared_with_rows(
            &doc.model,
            &e.form,
            e.draft,
            e.moment.as_deref(),
            e.user.as_deref(),
            &e.source_rows,
        ) {
            Ok(prepared) => {
                e.screen.prepared = prepared;
                e.recompute(&doc.model);
            }
            Err(error) => {
                let why = error.message;
                self.say(Level::Warn, why);
            }
        }
    }
}
