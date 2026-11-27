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

pub mod audit;
pub mod contingents;
pub mod designer;
pub(crate) mod designer_view;
pub mod domains;
pub mod entry;
pub mod layer_indexes;
pub mod layer_rules;
pub mod library;
pub mod location;
pub mod map_pick;
#[cfg(test)]
mod related_actions_tests;
#[cfg(test)]
#[path = "../../tests/form_roles_tests.rs"]
mod form_roles_tests;
#[cfg(test)]
#[path = "../../tests/road_form_tests.rs"]
mod road_form_tests;
pub mod package;
pub mod relations;
pub mod row_target;
mod relation_checks;
pub mod subtypes;
pub mod table_styles;

use iced::Task;
use kentos_commands::{LayerField, ProjectForm};
use kentos_domain::Slot;
use kentos_interaction::Level;

use crate::app::{App, Message};

/// The commands this module runs.
pub const COMMANDS: [&str; 13] = [
    "data.forms",
    "data.formOpen",
    "data.formEdit",
    "data.relations",
    "data.joinViews",
    "data.formAudit",
    "data.tableStyles",
    "data.subtypes",
    "data.contingents",
    "data.layerRules",
    "data.layerIndexes",
    "data.domains",
    "data.duplicate",
];

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
            "data.formOpen" => return self.open_form_view(),
            "data.formEdit" => return self.open_form_entry(),
            "data.relations" => self.open_relations(),
            "data.joinViews" => self.open_joins(),
            "data.formAudit" => return self.open_form_audit(None),
            "data.tableStyles" => return self.open_table_styles(),
            "data.subtypes" => return self.open_subtypes(),
            "data.contingents" => return self.open_contingents(),
            "data.layerRules" => return self.open_layer_rules(),
            "data.layerIndexes" => return self.open_layer_indexes(),
            "data.domains" => return self.open_domains(),
            "data.duplicate" => return self.duplicate_selected(),
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

    /// Why the forms may not change in the open drawing (their design and publishing), or none.
    pub(crate) fn forms_locked(&self) -> Option<&'static str> {
        self.tree_locked().map(|_| FORMS_LOCKED)
    }

    /// Why the open drawing's forms may not be published from here (docs/adr/9174): the design's own lock first, then a
    /// cloud project's role without `forms.publish`, which the server refuses a release without too; none when it may.
    /// Designing and keeping the draft is another right (`forms.design`).
    pub(crate) fn forms_publish_locked(&self) -> Option<String> {
        if let Some(why) = self.forms_locked() {
            return Some(why.to_owned());
        }
        let source = self.document.as_ref()?.cloud_source()?;
        if source.info.access.permissions.contains(&kentos_commands::ProjectPermission::FormsPublish) {
            return None;
        }
        Some(format!(
            "Bu projedeki rolünüz ({}) formu yayınlamaz: taslağı kaydedebilirsiniz, yayını form yayımlama izni olan bir proje yöneticisi yapar.",
            crate::cloud::words::role(source.info.access.role)
        ))
    }

    /// Why the open drawing's records may not be written through a published form (docs/adr/9125), or none. Writing a
    /// record is not designing a form: a cloud project's Veri girişi writes records (`forms.use`, `feature.write`)
    /// though it may not change the project (`project.edit`). An archived project and a role that may only read say
    /// so, with the role's name; the form still opens for reading. A layer's lock is the entry's own check.
    pub(crate) fn records_locked(&self) -> Option<String> {
        let doc = self.document.as_ref()?;
        let source = doc.cloud_source()?;
        if source.archived() {
            return Some("Proje arşivde: kayıtlar yalnız görüntülenir. Arşivden çıkarılıp yeniden açılınca yazılabilir.".into());
        }
        let may = |p| source.info.access.permissions.contains(&p);
        if may(kentos_commands::ProjectPermission::FormsUse) || may(kentos_commands::ProjectPermission::FeatureWrite) {
            return None;
        }
        Some(format!(
            "Bu projedeki rolünüz ({}) kayıt yazmaz; form yalnız görüntülenir. Veri girmek için proje sahibinden Veri girişi rolünü isteyin.",
            crate::cloud::words::role(source.info.access.role)
        ))
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

    /// The layer of the one object chosen in the drawing.
    pub(crate) fn one_object_layer(&self) -> Option<String> {
        let [slot] = self.selection.ids()[..] else {
            return None;
        };
        Some(self.document.as_ref()?.model.get(slot)?.base().layer_id.clone())
    }

    /// The object a drawing tool has just written opens its form (docs/adr/0403 §4): when the
    /// preference is on, no window is open and the object's layer has a published form; the record
    /// opens as a new one, its defaults and calculations already written by the drawing's step.
    pub(crate) fn open_form_on_create(&mut self) -> Task<Message> {
        let Some(slot) = self.form_on_create.take() else {
            return Task::none();
        };
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
            .map(|f| f.id.clone())
            .filter(|_| self.settings.bool(ON_CREATE) && self.dialog.is_none());
        // A new row of a data layer being edited (the second round's T8, docs/adr/9346): its form opens at once, the
        // layer's own (its draft too), else the default form of its fields; the layer gets no form (T17).
        let new_row = self.settings.bool(ON_CREATE) && self.dialog.is_none() && self.edited_with_fields(&layer);
        let form = form.or_else(|| new_row.then(|| self.form_of_layer(&layer)).flatten());
        match form {
            Some(id) => {
                self.selection.set([slot]);
                self.open_entry(slot, &id)
            }
            None if new_row => {
                self.selection.set([slot]);
                self.open_entry_default(slot, &layer)
            }
            None => {
                // No form takes the new object: a required field it left empty is said now, not found later.
                if let Some(note) = self.required_left_empty(slot) {
                    self.warn(note);
                }
                Task::none()
            }
        }
    }

    /// Whether `layer` is a data layer being edited that has fields: its new rows' values are typed after drawing.
    fn edited_with_fields(&self, layer: &str) -> bool {
        self.document.as_ref().is_some_and(|d| {
            d.model.editing(layer) && d.model.layers().get(layer).is_some_and(|n| !n.fields.is_empty())
        })
    }

    /// The layer's required fields without a default that the object at `slot` has no value for, said with the way
    /// to fill them; none when every one is filled.
    pub(crate) fn required_left_empty(&self, slot: kentos_domain::Slot) -> Option<String> {
        let doc = self.document.as_ref()?;
        let e = doc.model.get(slot)?;
        let layers = doc.model.layers();
        let node = layers.get(&e.base().layer_id)?;
        let empty: Vec<String> = node
            .fields
            .iter()
            .filter(|f| f.required)
            .filter(|f| e.base().attrs.get(&f.name).is_none_or(|v| v.trim().is_empty()))
            .map(|f| f.alias.clone().unwrap_or_else(|| f.name.clone()))
            .collect();
        if empty.is_empty() {
            return None;
        }
        Some(format!(
            "Yeni nesnenin zorunlu {} boş: {}. Öznitelik tablosunda ya da Özellikler panelinde doldurun; boş zorunlu hücreler tabloda işaretlidir.",
            if empty.len() == 1 { "alanı" } else { "alanları" },
            empty.join(", ")
        ))
    }

    /// The layer's first published form, else its first form (a draft).
    pub(crate) fn form_of_layer(&self, layer: &str) -> Option<String> {
        let forms: Vec<&ProjectForm> = self
            .project_forms()
            .iter()
            .filter(|f| f.layer == layer)
            .collect();
        // A published form in use first, then any in use, then a withdrawn one (said when opened).
        forms
            .iter()
            .find(|f| !f.releases.is_empty() && !f.withdrawn)
            .or_else(|| forms.iter().find(|f| !f.withdrawn))
            .or(forms.first())
            .map(|f| f.id.clone())
    }

    /// A withdrawn form is not opened (docs/adr/0445 §1): said, and true.
    fn withdrawn_said(&mut self, id: &str) -> bool {
        let Some(name) = self
            .project_forms()
            .iter()
            .find(|f| f.id == id && f.withdrawn)
            .map(|f| f.name.clone())
        else {
            return false;
        };
        self.say(
            Level::Warn,
            format!("“{name}” formu yayından kaldırıldı; kayıt bu formla açılmaz. Formlar penceresinde Yeniden yayına al'ı kullanın."),
        );
        true
    }

    /// Formla düzenle: the chosen object's record through its layer's form.
    /// Öznitelik formunu aç (the CBS review's 2nd item): the chosen record's form for reading, nothing taken, written or
    /// put on it, Düzenle one press away. A data row chosen in the table or on the map is read from its source; else the
    /// chosen object's record.
    fn open_form_view(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        if self.selection.is_empty()
            && let Some((layer, key)) = self.data_form_target()
        {
            return self.open_form_on_row_as(layer, key, true);
        }
        let Some((slot, layer)) = self.form_target() else {
            if let Some(layer) = self.one_object_layer() {
                self.say(
                    Level::Warn,
                    format!(
                        "“{}” katmanının formu yok. Formla düzenle katmanın alanlarından bir form kurar; ya da Veri › Formlar'da bir form yapın.",
                        self.form_layer_name(&layer)
                    ),
                );
                return Task::none();
            }
            let rows = if self.selection.is_empty() { self.data_rows_chosen() } else { self.selection.len() };
            self.say(Level::Warn, crate::forms::row_target::choose_one(rows));
            return Task::none();
        };
        let Some(form) = self.form_of_layer(&layer) else {
            return Task::none();
        };
        if self.withdrawn_said(&form) {
            return Task::none();
        }
        let task = self.open_entry(slot, &form);
        if let Some(e) = self.gis.form_entry.as_mut() {
            e.read_only = true;
            e.read_why = Some(crate::forms::row_target::VIEWED.into());
            e.edit_slot = Some(slot);
        }
        task
    }

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
                Some(id) if self.withdrawn_said(&id) => Task::none(),
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
            // A row chosen in a data layer's table is a record too (docs/adr/9149).
            if self.selection.is_empty()
                && let Some((layer, key)) = self.data_form_target()
            {
                return self.open_form_on_row(layer, key);
            }
            // One object of a layer without a form: its default form, built from the layer's fields (docs/adr/9346); the
            // layer gets no form (T17).
            if let (Some(layer), Some(&slot)) = (self.one_object_layer(), self.selection.ids().first()) {
                return self.open_entry_default(slot, &layer);
            }
            let rows = if self.selection.is_empty() { self.data_rows_chosen() } else { self.selection.len() };
            self.say(Level::Warn, crate::forms::row_target::choose_one(rows));
            return Task::none();
        };
        match self.form_of_layer(&layer) {
            Some(id) if self.withdrawn_said(&id) => Task::none(),
            Some(id) => self.open_entry(slot, &id),
            None => Task::none(),
        }
    }
}

#[cfg(test)]
#[path = "source_rows_tests.rs"]
mod source_rows_tests;
