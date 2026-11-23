//! Formla düzenle on a row chosen in a data layer's table (docs/adr/9149): the row is the form's record as a chosen
//! object of the drawing is. A row taken already opens its object's record, no second copy of it; a row of a layer
//! being edited is read from its source off the interface's thread, taken into the edit session and opened, so the form
//! and the session share one pending change; a layer not being edited says so with Düzenlemeyi başlat one press away.
//! A read-only connection's row is read the same way and shown in its form for reading, not taken (docs/adr/9159).
//! A related item's row its source holds opens the same way over the record's form, which waits under it
//! (docs/adr/9176).

use iced::Task;

use crate::app::{App, Message};
use crate::gis;

/// A row read for its form: the ask it answers, the layer, the key, the form, the rows or why not, and whether it is
/// only shown (a read-only connection's) rather than taken.
#[derive(Debug, Clone)]
pub struct RowTaken {
    pub asked: u64,
    pub layer: String,
    pub key: i64,
    pub form: String,
    pub rows: Result<Vec<(i64, kentos_commands::Entity)>, String>,
    pub shown: bool,
    /// The record's form a related row opens over ([`crate::forms::entry::Entry::identity`]): that form waits under
    /// it and comes back when it closes; an answer for a form closed or changed meanwhile is dropped. None for a
    /// table's row.
    pub over: Option<String>,
    /// Why the row's form only reads it ([`crate::forms::entry::Entry::read_why`]).
    pub read_why: Option<String>,
}

impl App {
    /// The one row chosen in a data layer's table, of a layer with a form: its layer and key.
    pub(crate) fn data_form_target(&self) -> Option<(String, i64)> {
        let layer = self.gis.features.data.selected_layer()?.to_owned();
        let [key] = self.gis.features.data.selected_of(&layer)[..] else {
            return None;
        };
        self.project_forms().iter().any(|f| f.layer == layer).then_some((layer, key))
    }

    /// Formla düzenle on a table's row (see the module).
    pub(crate) fn open_form_on_row(&mut self, layer: String, key: i64) -> Task<Message> {
        let Some(form) = self.form_of_layer(&layer) else {
            return Task::none();
        };
        if self.withdrawn_said(&form) {
            return Task::none();
        }
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        // Taken already: its object's record, the same pending change.
        if let Some(slot) = doc.model.taken_uid(&layer, key).and_then(|u| doc.model.slot_of(u)) {
            return self.open_entry(slot, &form);
        }
        let Some(node) = doc.model.layers().get(&layer).cloned() else {
            return Task::none();
        };
        // A read-only connection's row: read and shown, not taken; no edit session needed.
        let shown = node.data.as_ref().is_some_and(|d| d.read_only);
        if !shown && !doc.model.editing(&layer) {
            let why = format!(
                "“{}” düzenlemeye açık değil; {key} anahtarlı kaydı formla düzenlemek için önce katmanın düzenleme oturumunu başlatın.",
                node.name
            );
            self.warn(why.clone());
            self.toast(
                kentos_ui::widget::Toast::info("Katman düzenlemeye kapalı")
                    .body(why)
                    .action("Düzenlemeyi başlat", crate::data_edits::event(crate::data_edits::Event::Start(layer))),
            );
            return Task::none();
        }
        self.gis.form_row_asked += 1;
        let asked = self.gis.form_row_asked;
        let settings = doc.model.settings().clone();
        crate::db_connections::off_thread(
            move || kentos_native_application::data_write::rows_to_take(&node, &settings, &[key], &|| false),
            move |rows| {
                Message::Gis(gis::Event::FormRow(Box::new(RowTaken {
                    asked,
                    layer,
                    key,
                    form,
                    rows,
                    shown,
                    over: None,
                    read_why: None,
                })))
            },
        )
    }

    /// The row read: taken into the edit session and its record opened.
    pub(crate) fn form_row_taken(&mut self, t: RowTaken) -> Task<Message> {
        if t.asked != self.gis.form_row_asked {
            return Task::none();
        }
        // A related row's form opens over the record's: that one closed or another opened meanwhile, stale.
        if let Some(over) = &t.over
            && self.gis.form_entry.as_ref().map(|e| e.identity()).as_ref() != Some(over)
        {
            return Task::none();
        }
        let rows = match t.rows {
            Ok(rows) if !rows.is_empty() => rows,
            Ok(_) => {
                self.warn(format!("{} anahtarlı kayıt tabloda yok.", t.key));
                return Task::none();
            }
            Err(why) => {
                self.warn(why);
                return Task::none();
            }
        };
        if t.shown {
            let Some((_, e)) = rows.iter().find(|(k, _)| *k == t.key) else {
                return Task::none();
            };
            let nested = self.nest_over(t.over.is_some());
            let task = self.open_entry_row(&t.layer, t.key, e, &t.form);
            self.read_why_of_opened(nested, t.read_why);
            return task;
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let uid = match doc.model.take_data_rows(&t.layer, rows) {
            Ok(uids) => uids.into_iter().flatten().next(),
            Err(refusal) => {
                self.warn(refusal.0);
                return Task::none();
            }
        };
        match uid.and_then(|u| doc.model.slot_of(u)) {
            Some(slot) => {
                if self.nest_over(t.over.is_some()) {
                    self.selection.set([slot]);
                }
                self.open_entry(slot, &t.form)
            }
            None => Task::none(),
        }
    }

    /// The record's form put under the one about to open, when `over`: it waits there and comes back when that one
    /// closes (docs/adr/0440 §7). Whether it did.
    pub(crate) fn nest_over(&mut self, over: bool) -> bool {
        if !over {
            return false;
        }
        match self.gis.form_entry.take() {
            Some(parent) => {
                self.gis.form_stack.push(parent);
                true
            }
            None => false,
        }
    }

    /// The form just opened only reads its record, and why, when it opened over another.
    fn read_why_of_opened(&mut self, nested: bool, why: Option<String>) {
        if let Some(e) = self.gis.form_entry.as_mut().filter(|_| nested) {
            e.read_only = true;
            e.read_why = why;
        }
    }
}

#[cfg(test)]
#[path = "row_target_tests.rs"]
mod row_target_tests;
