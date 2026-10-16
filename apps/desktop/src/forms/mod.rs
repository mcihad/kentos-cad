//! Formlar (docs/adr/0401): a layer's data entry screen, kept with the project.
//!
//! - Formlar (`data.forms`, [`designer`]): the project's forms, a new one made of a layer's fields,
//!   the designer (tabs, groups, fields, notes; their labels, kinds of widget and rules), the
//!   preflight's issues, Kaydet, Yayınla and Kuralları uygula. Every write is `cad.forms.edit`.
//! - Formla düzenle (`data.formEdit`, [`entry`]): the chosen object's record through its layer's
//!   form: tabs and groups, the fields shown, required and locked as the rules say now, the issues
//!   under their fields; Kaydet writes the record through `cad.forms.submit` and closes the window.
//!
//! The rules are the forms core's (`kentos_forms`): the windows ask it as the user types, the
//! commands ask it again before they write.

pub mod designer;
mod designer_view;
pub mod entry;

use iced::Task;
use kentos_contracts::{LayerField, ProjectForm};
use kentos_domain::Slot;
use kentos_interaction::Level;

use crate::app::{App, Message};

/// The commands this module runs.
pub const COMMANDS: [&str; 2] = ["data.forms", "data.formEdit"];

/// Why the forms may not change here.
pub const FORMS_LOCKED: &str = "Bu projede formları değiştirme yetkiniz yok (project.edit); proje sahibinden ya da yöneticisinden isteyin.";

impl App {
    pub(crate) fn forms_command(&mut self, id: &str) -> Task<Message> {
        match id {
            "data.forms" => self.open_forms(None),
            "data.formEdit" => return self.open_form_entry(),
            _ => {}
        }
        Task::none()
    }

    /// The open drawing's forms.
    pub(crate) fn project_forms(&self) -> &[ProjectForm] {
        self.document
            .as_ref()
            .map_or(&[], |d| d.model.settings().forms.as_slice())
    }

    /// A layer's fields in the open drawing (none when it is gone).
    pub(crate) fn layer_fields_of(&self, layer: &str) -> Vec<LayerField> {
        self.document
            .as_ref()
            .and_then(|d| d.model.layers().get(layer))
            .map(|n| n.fields.clone())
            .unwrap_or_default()
    }

    /// A layer's name in the open drawing (its id when it is gone).
    pub(crate) fn form_layer_name(&self, layer: &str) -> String {
        self.document
            .as_ref()
            .and_then(|d| d.model.layers().get(layer))
            .map_or_else(|| layer.to_owned(), |n| n.name.clone())
    }

    /// Why the forms may not change in the open drawing, or none.
    pub(crate) fn forms_locked(&self) -> Option<&'static str> {
        self.tree_locked().map(|_| FORMS_LOCKED)
    }

    /// The one chosen object, when it is on a layer with a form: its slot and its layer.
    pub(crate) fn form_target(&self) -> Option<(Slot, String)> {
        if self.selection.len() != 1 {
            return None;
        }
        let slot = *self.selection.ids().first()?;
        let doc = self.document.as_ref()?;
        let layer = doc.model.get(slot)?.base().layer_id.clone();
        self.project_forms()
            .iter()
            .any(|f| f.layer == layer)
            .then_some((slot, layer))
    }

    /// Formla düzenle: the chosen object's record through its layer's form.
    fn open_form_entry(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        let Some((slot, layer)) = self.form_target() else {
            let chosen = self.selection.len();
            let why = if chosen != 1 {
                "Formla düzenlemek için tek bir nesne seçin."
            } else {
                "Seçili nesnenin katmanının formu yok. Formlar penceresinden katmana bir form oluşturun."
            };
            self.say(Level::Warn, why);
            return Task::none();
        };
        // The layer's first published form, else its first form (a draft).
        let forms: Vec<&ProjectForm> = self
            .project_forms()
            .iter()
            .filter(|f| f.layer == layer)
            .collect();
        let form = forms
            .iter()
            .find(|f| !f.releases.is_empty())
            .or(forms.first())
            .map(|f| f.id.clone());
        match form {
            Some(id) => self.open_entry(slot, &id),
            None => Task::none(),
        }
    }
}
