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

/// The drawing tools whose one new object opens its layer's form (docs/adr/0403 §4); a copy, an
/// array, a paste or an import does not.
pub const DRAWING_TOOLS: [&str; 19] = [
    "polygon",
    "polyline",
    "area",
    "parcel",
    "point",
    "spot",
    "circle",
    "arc",
    "ellipse",
    "rectangle",
    "rectangle3",
    "regularPolygon",
    "spline",
    "nurbs",
    "line",
    "sector",
    "revcloud",
    "adjoin",
    "donut",
];

/// The preference that opens a drawn object's form (docs/adr/0403 §4).
pub const ON_CREATE: &str = "drafting.formOnCreate";

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

    /// The object a drawing tool has just written opens its form (docs/adr/0403 §4): when the
    /// preference is on, no window is open and the object's layer has a published form; the record
    /// opens as a new one, its defaults and calculations already written by the drawing's step.
    pub(crate) fn open_form_on_create(&mut self) -> Task<Message> {
        let Some(slot) = self.form_on_create.take() else {
            return Task::none();
        };
        if !self.settings.bool(ON_CREATE) || self.dialog.is_some() {
            return Task::none();
        }
        let Some(layer) = self
            .document
            .as_ref()
            .and_then(|d| d.model.get(slot))
            .map(|e| e.base().layer_id.clone())
        else {
            return Task::none();
        };
        let form = self
            .project_forms()
            .iter()
            .find(|f| f.layer == layer && !f.releases.is_empty())
            .map(|f| f.id.clone());
        match form {
            Some(id) => self.open_entry(slot, &id),
            None => Task::none(),
        }
    }

    /// The layer's first published form, else its first form (a draft).
    fn form_of_layer(&self, layer: &str) -> Option<String> {
        let forms: Vec<&ProjectForm> = self
            .project_forms()
            .iter()
            .filter(|f| f.layer == layer)
            .collect();
        forms
            .iter()
            .find(|f| !f.releases.is_empty())
            .or(forms.first())
            .map(|f| f.id.clone())
    }

    /// Formla düzenle: the chosen object's record through its layer's form.
    fn open_form_entry(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        // Many objects of one layer with a form: the many-record mode (docs/adr/0403 §6).
        if self.selection.len() > 1 {
            let slots: Vec<Slot> = self.selection.ids().to_vec();
            let layers: std::collections::BTreeSet<String> = self
                .document
                .as_ref()
                .map(|d| {
                    slots
                        .iter()
                        .filter_map(|s| d.model.get(*s).map(|e| e.base().layer_id.clone()))
                        .collect()
                })
                .unwrap_or_default();
            let form = match &layers.iter().collect::<Vec<_>>()[..] {
                [layer] => self.form_of_layer(layer),
                _ => None,
            };
            return match form {
                Some(id) => self.open_entry_many(&slots, &id),
                None => {
                    self.say(
                        Level::Warn,
                        "Birden çok nesneyi formla düzenlemek için aynı, formu olan katmandan nesneler seçin.",
                    );
                    Task::none()
                }
            };
        }
        let Some((slot, layer)) = self.form_target() else {
            let chosen = self.selection.len();
            let why = if chosen != 1 {
                "Formla düzenlemek için bir nesne seçin."
            } else {
                "Seçili nesnenin katmanının formu yok. Formlar penceresinden katmana bir form oluşturun."
            };
            self.say(Level::Warn, why);
            return Task::none();
        };
        match self.form_of_layer(&layer) {
            Some(id) => self.open_entry(slot, &id),
            None => Task::none(),
        }
    }
}
