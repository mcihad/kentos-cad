//! Kapı ve pencere tipleri (docs/adr/0772 §1, §4; `arch.openingTypes`): the project's door and window types, each
//! with its openings counted, those at an older version and those keeping values of their own. The chosen type's
//! values are typed in the project's unit; before Uygula the plan of `cad.openingtypes.edit` says what the change does
//! (how many openings follow, how many keep their own value, how many are left on a locked layer, which refuse it).
//! Yeni tip copies the chosen one, Sil leaves its openings with their values; Kitaplıktan al and Kitaplığa yaz carry
//! the office's types as a `kentos.opening-types` file. Every write is one undo step. Aynı tiptekileri seç
//! (`arch.openingSameType`) and the selection's Tipten ayır, Tipe uygula and Tipe döndür live here too.

use std::path::PathBuf;

use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::Entity;
use kentos_commands::{
    CommandResult, OpeningForm, OpeningOperation, OpeningSizeBy, OpeningType, OpeningTypeChange,
    OpeningTypeConflict, OpeningTypeField, OpeningTypeRefused, OpeningTypeReport, OpeningTypeUse,
    OpeningTypesEdit, OpeningTypesEdited, OpeningTypesOperation, OpeningsEdit, OpeningsOperation,
    OpeningsQuery,
};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::{Format, Level};
use kentos_ui::icon::icon;
use kentos_ui::widget::{Choice, Dialog as Frame, Select};
use kentos_ui::{label, style};
use serde_json::{Map, Value};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Kapı ve pencere tipleri";
const CLOSE: &str = "Kapat";
const APPLY: &str = "Uygula";
const NEW: &str = "Yeni tip";
const REMOVE: &str = "Sil";
const IMPORT: &str = "Kitaplıktan al…";
const EXPORT: &str = "Kitaplığa yaz…";

/// The values typed as lengths in the project's unit, by form.
const LENGTHS: [OpeningTypeField; 14] = [
    OpeningTypeField::Frame,
    OpeningTypeField::Fit,
    OpeningTypeField::Head,
    OpeningTypeField::Sill,
    OpeningTypeField::LeafThickness,
    OpeningTypeField::FrameDepth,
    OpeningTypeField::Threshold,
    OpeningTypeField::Casing,
    OpeningTypeField::SillBoard,
    OpeningTypeField::UseZone,
    OpeningTypeField::Depth,
    // A revolving door's drum, a sliding door's overlap, a pocket door's lip (docs/adr/0777).
    OpeningTypeField::Drum,
    OpeningTypeField::Overlap,
    OpeningTypeField::Lip,
];
/// The switches typed Evet (on) or left empty (off): a sliding door's track, a folding door's both sides.
const SWITCHES: [OpeningTypeField; 2] = [OpeningTypeField::Track, OpeningTypeField::BothSides];
/// The values typed as whole numbers or plain values.
const COUNTS: [OpeningTypeField; 5] = [
    OpeningTypeField::Leaves,
    OpeningTypeField::Split,
    OpeningTypeField::Mullions,
    OpeningTypeField::Transoms,
    // A three- or four-leaf sliding door's tracks (docs/adr/0792).
    OpeningTypeField::Tracks,
];

/// Whether a type of `form` moving as `op` takes `field` in this window (a special door's only its own, docs/adr/0777).
fn shown(form: OpeningForm, op: OpeningOperation, field: OpeningTypeField) -> bool {
    use OpeningTypeField::*;
    let door = form == OpeningForm::Door;
    match field {
        Drum => door && op == OpeningOperation::Revolving,
        Lip => door && op == OpeningOperation::Pocket,
        BothSides => {
            (door && op == OpeningOperation::Folding)
                || (matches!(form, OpeningForm::Door | OpeningForm::Window) && op == OpeningOperation::Slide)
        }
        Track | Overlap | Tracks => matches!(form, OpeningForm::Door | OpeningForm::Window) && op == OpeningOperation::Slide,
        Sill | SillBoard | Mullions | Transoms => form == OpeningForm::Window,
        Threshold | LeafThickness | UseZone => form == OpeningForm::Door,
        Depth => false,
        Split => form != OpeningForm::Opening,
        Frame | Fit | Casing | FrameDepth | Leaves => form != OpeningForm::Opening,
        _ => true,
    }
}

#[derive(Clone, Debug, Default)]
pub struct Window {
    pub uses: Vec<OpeningTypeUse>,
    pub chosen: usize,
    pub name: String,
    /// The chosen type's nominal size (by its size reference) as typed.
    pub nominal: String,
    pub size_by: OpeningSizeBy,
    pub operation: OpeningOperation,
    pub typed: Vec<(OpeningTypeField, String)>,
    /// What the change typed would do; its refusal's words when refused.
    pub preview: Option<Result<OpeningTypesEdited, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Choose(usize),
    Name(String),
    Nominal(String),
    SizeBy(OpeningSizeBy),
    Operation(OpeningOperation),
    Field(OpeningTypeField, String),
    New,
    Remove,
    Apply,
    Import,
    Imported(Option<PathBuf>),
    Export,
    Exported(Option<PathBuf>),
    Close,
}

fn msg(e: Event) -> Message {
    Message::OpeningTypes(e)
}

/// A length as the window shows it, in the project's unit; empty for none.
fn length_text(f: &Format, v: &Value) -> String {
    v.as_f64().map_or_else(String::new, |m| f.length_bare(m))
}

/// What the window's report line says about `r`.
pub(crate) fn report_words(r: &OpeningTypeReport) -> String {
    let mut parts = vec![format!("{} açıklık izliyor", r.applied.len())];
    if !r.kept.is_empty() {
        parts.push(format!("{} istisna korunur", r.kept.len()));
    }
    if !r.skipped.is_empty() {
        parts.push(format!(
            "{} kilitli katmanda eski sürümde kalır",
            r.skipped.len()
        ));
    }
    if !r.refused.is_empty() {
        parts.push(format!("{} sığmıyor", r.refused.len()));
    }
    parts.join(" · ")
}

impl App {
    fn opening_types_format(&self) -> Option<Format> {
        self.document
            .as_ref()
            .map(|d| Format::of(d.model.settings()))
    }

    /// The project's types with their openings counted.
    fn opening_type_uses(&self) -> Vec<OpeningTypeUse> {
        let Some(doc) = self.document.as_ref() else {
            return Vec::new();
        };
        match kentos_native_application::openings_query::query(
            &doc.model,
            &OpeningsQuery::default(),
        ) {
            CommandResult::Completed { output, .. } => output.types,
            _ => Vec::new(),
        }
    }

    /// The window read again, the type `keep` (an id) chosen if it is still there.
    fn opening_types_read(&mut self, keep: Option<String>) {
        let uses = self.opening_type_uses();
        let chosen = keep
            .and_then(|id| uses.iter().position(|u| u.kind.id == id))
            .unwrap_or(0);
        let mut w = Window {
            uses,
            chosen,
            ..Window::default()
        };
        if let (Some(f), Some(t)) = (
            self.opening_types_format(),
            w.uses.get(chosen).map(|u| u.kind.clone()),
        ) {
            fill(&mut w, &t, &f);
        }
        self.opening_types = Some(w);
    }

    /// Kapı ve pencere tipleri (`arch.openingTypes`): the window.
    pub(crate) fn open_opening_types(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        // The selected opening's type chosen when it follows one.
        let keep = self
            .selected_openings()
            .into_iter()
            .find_map(|(_, style)| style);
        self.opening_types_read(keep);
        self.dialog = Some(Dialog::OpeningTypes);
    }

    /// The selected openings: their ids and the type each follows or was made with.
    fn selected_openings(&self) -> Vec<(String, Option<String>)> {
        let Some(doc) = self.document.as_ref() else {
            return Vec::new();
        };
        self.selection
            .ids()
            .iter()
            .filter_map(|&s| match doc.model.get(s) {
                Some(Entity::Opening(o)) => {
                    Some((doc.model.uid(s)?.to_string(), o.opening.style.clone()))
                }
                _ => None,
            })
            .collect()
    }

    /// Aynı tiptekileri seç (`arch.openingSameType`): every opening of the selected one's type, on the active level
    /// when one is set (independent units are docs/adr/0772's boundary), selected.
    pub(crate) fn select_same_opening_type(&mut self) {
        let Some(kind) = self.selected_openings().into_iter().find_map(|(_, s)| s) else {
            self.warn("Önce bir tipi izleyen açıklık seçin: aynı tiptekiler onun tipiyle seçilir.");
            return;
        };
        let level = self.active_level();
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let input = OpeningsQuery {
            kind: Some(kind.clone()),
            level: level.clone().map(Some),
            ..OpeningsQuery::default()
        };
        match kentos_native_application::openings_query::query(&doc.model, &input) {
            CommandResult::Completed { output, .. } => {
                let slots: Vec<Slot> = output
                    .openings
                    .iter()
                    .filter_map(|r| Uuid::parse_str(&r.uid).ok())
                    .filter_map(|u| doc.model.slot_of(u))
                    .collect();
                let name = output
                    .openings
                    .first()
                    .and_then(|r| r.type_name.clone())
                    .unwrap_or(kind);
                let scope = if level.is_some() {
                    "etkin katta"
                } else {
                    "çizimde"
                };
                let n = slots.len();
                self.selection.set(slots);
                self.say(
                    Level::Success,
                    format!("{name} tipinde {n} açıklık seçildi ({scope})."),
                );
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
            }
            _ => {}
        }
    }

    /// Tipten ayır, Tipe uygula, Tipe döndür on the selected openings through `cad.openings.edit`, one undo step.
    pub(crate) fn opening_type_action(&mut self, operation: OpeningsOperation) {
        let uids: Vec<String> = self
            .selected_openings()
            .into_iter()
            .filter(|(_, s)| s.is_some())
            .map(|(u, _)| u)
            .collect();
        if uids.is_empty() {
            self.warn(
                "Seçimde tipi izleyen açıklık yok; bir tipin açıklığını seçip yeniden deneyin.",
            );
            return;
        }
        if matches!(operation, OpeningsOperation::Push { .. }) && uids.len() != 1 {
            self.warn("Tipe uygula tek açıklıkla çalışır: değerleri tipin olacak açıklığı seçin.");
            return;
        }
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let result = kentos_native_application::openings_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            OpeningsEdit {
                uids,
                operation,
                expected_revision: None,
            },
        );
        match result {
            CommandResult::Completed {
                output, warnings, ..
            } => {
                let mut said = format!("{} açıklık değişti.", output.openings.len());
                if let Some(r) = &output.report {
                    said = format!("{said} {}.", report_words(r));
                }
                self.say(Level::Success, said);
                for w in warnings {
                    self.warn(w.message);
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
            }
            _ => {}
        }
    }

    /// The change typed against the chosen type: none when nothing changed, else the change or why it cannot be read.
    fn opening_type_change(&self) -> Option<Result<(String, OpeningTypeChange), String>> {
        let w = self.opening_types.as_ref()?;
        let f = self.opening_types_format()?;
        let t = &w.uses.get(w.chosen)?.kind;
        let mut top = Map::new();
        let mut build = Map::new();
        let mut unset: Vec<OpeningTypeField> = Vec::new();
        let put = |field: OpeningTypeField,
                   v: Value,
                   top: &mut Map<String, Value>,
                   build: &mut Map<String, Value>| {
            if field.in_build() {
                build.insert(field.name().to_owned(), v);
            } else {
                top.insert(field.name().to_owned(), v);
            }
        };
        for (field, text) in &w.typed {
            let text = text.trim();
            let old = t.value(*field);
            let new = if text.is_empty() {
                Value::Null
            } else if LENGTHS.contains(field) {
                match kentos_geometry_core::tools::point_text::parse_length(text) {
                    Some(l) => Value::from(l.metres(|n| f.to_metres(n))),
                    None => {
                        return Some(Err(format!(
                            "{}: “{text}” okunamadı; bir uzunluk yazın.",
                            field.label()
                        )));
                    }
                }
            } else if SWITCHES.contains(field) {
                match text.to_lowercase().as_str() {
                    "evet" | "e" | "1" => Value::Bool(true),
                    "hayır" | "h" | "0" => Value::Null,
                    _ => {
                        return Some(Err(format!(
                            "{}: “{text}” okunamadı; Evet yazın ya da boş bırakın.",
                            field.label()
                        )));
                    }
                }
            } else if *field == OpeningTypeField::Split {
                match text.replace(',', ".").parse::<f64>() {
                    Ok(v) => Value::from(v),
                    Err(_) => {
                        return Some(Err(format!(
                            "{}: “{text}” okunamadı; 0 ile 1 arası bir sayı yazın.",
                            field.label()
                        )));
                    }
                }
            } else {
                match text.parse::<u8>() {
                    Ok(v) => Value::from(v),
                    Err(_) => {
                        return Some(Err(format!(
                            "{}: “{text}” okunamadı; bir tam sayı yazın.",
                            field.label()
                        )));
                    }
                }
            };
            let same = match (old.as_f64(), new.as_f64()) {
                (Some(a), Some(b)) => (a - b).abs() < 1e-9,
                _ => old == new,
            };
            if same {
                continue;
            }
            if new.is_null() {
                unset.push(*field);
            } else {
                put(*field, new, &mut top, &mut build);
            }
        }
        // A special door's fields another operation does not take go with the old one (docs/adr/0777).
        for k in [
            OpeningTypeField::Drum,
            OpeningTypeField::Overlap,
            OpeningTypeField::Track,
            OpeningTypeField::BothSides,
            OpeningTypeField::Lip,
            OpeningTypeField::Tracks,
        ] {
            if !shown(t.form, w.operation, k) && !t.value(k).is_null() && !unset.contains(&k) {
                unset.push(k);
            }
        }
        let old_by = t.build.as_ref().and_then(|b| b.size_by).unwrap_or_default();
        if w.size_by != old_by {
            build.insert(
                "sizeBy".into(),
                serde_json::to_value(w.size_by).unwrap_or(Value::Null),
            );
        }
        if w.operation != t.operation {
            top.insert(
                "operation".into(),
                serde_json::to_value(w.operation).unwrap_or(Value::Null),
            );
        }
        let nominal = t.fields().sizes().nominal;
        let typed = w.nominal.trim();
        match kentos_geometry_core::tools::point_text::parse_length(typed) {
            Some(l) => {
                let v = l.metres(|n| f.to_metres(n));
                if (v - nominal).abs() > 1e-9 || w.size_by != old_by {
                    let mut n = Map::new();
                    n.insert(
                        "by".into(),
                        serde_json::to_value(w.size_by).unwrap_or(Value::Null),
                    );
                    n.insert("value".into(), Value::from(v));
                    top.insert("nominal".into(), Value::Object(n));
                }
            }
            None => {
                return Some(Err(format!(
                    "Ölçü: “{typed}” okunamadı; bir uzunluk yazın."
                )));
            }
        }
        if !build.is_empty() {
            top.insert("build".into(), Value::Object(build));
        }
        if !unset.is_empty() {
            top.insert(
                "unset".into(),
                serde_json::to_value(&unset).unwrap_or(Value::Null),
            );
        }
        if top.is_empty() {
            return None;
        }
        Some(
            serde_json::from_value::<OpeningTypeChange>(Value::Object(top))
                .map(|c| (t.id.clone(), c))
                .map_err(|e| format!("Değişiklik okunamadı: {e}")),
        )
    }

    /// The plan of the change typed: what it would do, nothing written.
    fn opening_types_preview(&mut self) {
        let preview = match self.opening_type_change() {
            None => None,
            Some(Err(why)) => Some(Err(why)),
            Some(Ok((kind, change))) => self.document.as_ref().map(|doc| {
                let input = OpeningTypesEdit {
                    operation: OpeningTypesOperation::Update { kind, change },
                    on_refused: OpeningTypeRefused::Stop,
                    expected_revision: None,
                };
                match kentos_native_application::opening_types_edit::plan_in(&doc.model, &input) {
                    CommandResult::Completed { output, .. } => Ok(output),
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => Err(error.message),
                    _ => Err("Önizlenemedi.".into()),
                }
            }),
        };
        if let Some(w) = self.opening_types.as_mut() {
            w.preview = preview;
        }
    }

    /// `cad.openingtypes.edit` over the drawing; its answer, or none with the refusal said.
    fn opening_types_command(
        &mut self,
        operation: OpeningTypesOperation,
    ) -> Option<OpeningTypesEdited> {
        let doc = self.document.as_mut()?;
        let result = kentos_native_application::opening_types_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            OpeningTypesEdit {
                operation,
                on_refused: OpeningTypeRefused::Stop,
                expected_revision: None,
            },
        );
        match result {
            CommandResult::Completed {
                output, warnings, ..
            } => {
                for w in warnings {
                    self.warn(w.message);
                }
                Some(output)
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
                None
            }
            _ => None,
        }
    }

    fn opening_types_chosen(&self) -> Option<OpeningType> {
        let w = self.opening_types.as_ref()?;
        w.uses.get(w.chosen).map(|u| u.kind.clone())
    }

    pub(crate) fn opening_types_event(&mut self, event: Event) -> Task<Message> {
        let format = self.opening_types_format();
        match event {
            Event::Choose(i) => {
                let t = self
                    .opening_types
                    .as_ref()
                    .and_then(|w| w.uses.get(i))
                    .map(|u| u.kind.clone());
                if let (Some(w), Some(t), Some(f)) = (self.opening_types.as_mut(), t, format) {
                    w.chosen = i;
                    fill(w, &t, &f);
                }
            }
            Event::Name(t) => {
                if let Some(w) = self.opening_types.as_mut() {
                    w.name = t;
                }
            }
            Event::Nominal(t) => {
                if let Some(w) = self.opening_types.as_mut() {
                    w.nominal = t;
                }
                self.opening_types_preview();
            }
            Event::SizeBy(by) => {
                if let Some(w) = self.opening_types.as_mut() {
                    w.size_by = by;
                }
                self.opening_types_preview();
            }
            Event::Operation(op) => {
                let t = self.opening_types_chosen();
                let format = self.opening_types_format();
                if let Some(w) = self.opening_types.as_mut() {
                    w.operation = op;
                    // The fields of the operation chosen (docs/adr/0777): those typed kept, the others the type's.
                    if let (Some(t), Some(f)) = (t, format) {
                        refit(w, &t, &f);
                    }
                }
                self.opening_types_preview();
            }
            Event::Field(field, t) => {
                if let Some(w) = self.opening_types.as_mut()
                    && let Some(slot) = w.typed.iter_mut().find(|(f, _)| *f == field)
                {
                    slot.1 = t;
                }
                self.opening_types_preview();
            }
            Event::Apply => {
                let Some(t) = self.opening_types_chosen() else {
                    return Task::none();
                };
                let name = self
                    .opening_types
                    .as_ref()
                    .map(|w| w.name.trim().to_owned())
                    .unwrap_or_default();
                if !name.is_empty() && name != t.name {
                    self.opening_types_command(OpeningTypesOperation::Rename {
                        kind: t.id.clone(),
                        name: name.clone(),
                    });
                }
                match self.opening_type_change() {
                    Some(Ok((kind, change))) => {
                        if let Some(done) = self
                            .opening_types_command(OpeningTypesOperation::Update { kind, change })
                        {
                            let version = done
                                .types
                                .iter()
                                .find(|x| x.id == t.id)
                                .map_or(t.version, |x| x.version);
                            self.say(
                                Level::Success,
                                format!(
                                    "{} tipi değişti (sürüm {version}): {}.",
                                    t.name,
                                    report_words(&done.report)
                                ),
                            );
                        }
                    }
                    Some(Err(why)) => {
                        self.warn(why);
                        return Task::none();
                    }
                    None => {}
                }
                self.opening_types_read(Some(t.id));
            }
            Event::New => {
                let chosen = self.opening_types_chosen();
                let base = chosen.as_ref().map_or_else(
                    || "Yeni kapı tipi".to_owned(),
                    |t| format!("{} kopyası", t.name),
                );
                let names: Vec<String> = self
                    .opening_types
                    .as_ref()
                    .map(|w| w.uses.iter().map(|u| u.kind.name.to_lowercase()).collect())
                    .unwrap_or_default();
                let mut name = base.clone();
                let mut n = 2;
                while names.contains(&name.to_lowercase()) {
                    name = format!("{base} {n}");
                    n += 1;
                }
                let done = self.opening_types_command(OpeningTypesOperation::Define {
                    name: name.clone(),
                    id: None,
                    copy_of: chosen.as_ref().map(|t| t.id.clone()),
                    form: chosen.as_ref().map_or(Some(OpeningForm::Door), |_| None),
                    change: OpeningTypeChange::default(),
                });
                if let Some(done) = done {
                    let id = done
                        .types
                        .iter()
                        .find(|t| t.name == name)
                        .map(|t| t.id.clone());
                    self.say(Level::Success, format!("{name} tipi eklendi."));
                    self.opening_types_read(id);
                }
            }
            Event::Remove => {
                let Some(t) = self.opening_types_chosen() else {
                    return Task::none();
                };
                if self
                    .opening_types_command(OpeningTypesOperation::Remove {
                        kind: t.id.clone(),
                        detach: true,
                    })
                    .is_some()
                {
                    self.say(
                        Level::Success,
                        format!("{} tipi silindi; açıklıkları değerleriyle kaldı.", t.name),
                    );
                }
                self.opening_types_read(None);
            }
            Event::Import | Event::Export => {
                let open = matches!(event, Event::Import);
                return Task::perform(
                    async move {
                        let dialog = rfd::AsyncFileDialog::new()
                            .set_title(if open {
                                "Tip kitaplığından al"
                            } else {
                                "Tip kitaplığına yaz"
                            })
                            .add_filter("KentOS kapı ve pencere tipleri (.json)", &["json"]);
                        let file = if open {
                            dialog.pick_file().await?
                        } else {
                            dialog
                                .set_file_name("kapi-pencere-tipleri.json")
                                .save_file()
                                .await?
                        };
                        Some(file.path().to_path_buf())
                    },
                    move |path| {
                        msg(if open {
                            Event::Imported(path)
                        } else {
                            Event::Exported(path)
                        })
                    },
                );
            }
            Event::Imported(Some(path)) => self.import_opening_types(&path),
            Event::Exported(Some(path)) => self.export_opening_types(&path),
            Event::Imported(None) | Event::Exported(None) => {}
            Event::Close => {
                self.opening_types = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// Kitaplıktan al…: an office library's types, a name the project has taken under a new one.
    pub(crate) fn import_opening_types(&mut self, path: &std::path::Path) {
        let library = match std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|t| {
                serde_json::from_str::<kentos_commands::OpeningTypesFile>(&t)
                    .map_err(|e| e.to_string())
            }) {
            Ok(l) => l,
            Err(e) => {
                self.warn(format!(
                    "Tip kitaplığı okunamadı ({e}); kentos.opening-types biçiminde bir dosya seçin."
                ));
                return;
            }
        };
        let count = library.types.len();
        if self
            .opening_types_command(OpeningTypesOperation::Import {
                library,
                on_conflict: OpeningTypeConflict::Rename,
            })
            .is_some()
        {
            self.say(Level::Success, format!("Kitaplıktan {count} tip alındı."));
        }
        let keep = self.opening_types_chosen().map(|t| t.id);
        self.opening_types_read(keep);
    }

    /// Kitaplığa yaz…: the project's types as a `kentos.opening-types` file.
    pub(crate) fn export_opening_types(&mut self, path: &std::path::Path) {
        let library = path
            .file_stem()
            .map_or_else(|| "Ofis".to_owned(), |s| s.to_string_lossy().into_owned());
        let Some(file) = self
            .opening_types_command(OpeningTypesOperation::Export {
                library,
                types: Vec::new(),
            })
            .and_then(|d| d.library)
        else {
            return;
        };
        let text = serde_json::to_string_pretty(&file).unwrap_or_default();
        match std::fs::write(path, text + "\n") {
            Ok(()) => self.say(
                Level::Success,
                format!("{} tip kitaplığa yazıldı.", file.types.len()),
            ),
            Err(e) => self.warn(format!(
                "Kitaplık yazılamadı ({e}); başka bir klasör seçip yeniden deneyin."
            )),
        }
    }

    pub(crate) fn opening_types_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(f)) = (&self.opening_types, self.opening_types_format()) else {
            return iced::widget::text("").into();
        };
        let mut list = Column::new().spacing(4);
        for (i, u) in w.uses.iter().enumerate() {
            // A door's icon by how it moves (docs/adr/0777): sliding, revolving, pocket, folding, double.
            let glyph = match (u.kind.form, u.kind.operation, u.kind.leaves) {
                (OpeningForm::Window, _, _) => "window",
                (_, OpeningOperation::Slide, _) => "doorSliding",
                (_, OpeningOperation::Revolving, _) => "doorRevolving",
                (_, OpeningOperation::Pocket, _) => "doorPocket",
                (_, OpeningOperation::Folding, _) => "doorFolding",
                (_, _, 2) => "doorDouble",
                _ => "door",
            };
            let mut facts = format!(
                "{} · sürüm {} · {} açıklık",
                u.kind.form.label(),
                u.kind.version,
                u.count
            );
            if u.exceptions > 0 {
                facts.push_str(&format!(" · {} istisna", u.exceptions));
            }
            if u.stale > 0 {
                facts.push_str(&format!(" · {} eski", u.stale));
            }
            let text = Column::new()
                .spacing(2)
                .push(label::body(u.kind.name.clone()))
                .push(label::caption(facts));
            let entry = row![
                icon(crate::icons::from_web(Some(glyph))).size(18.0),
                container(text).width(Fill)
            ]
            .spacing(8)
            .align_y(Center);
            list = list.push(
                button(entry)
                    .on_press(msg(Event::Choose(i)))
                    .padding([6, 8])
                    .width(Fill)
                    .style(style::button::row(i == w.chosen)),
            );
        }
        if w.uses.is_empty() {
            list = list.push(label::caption(
                "Projede tip yok. Yeni tip KentOS'un kapısından başlar; Kitaplıktan al ofisin tiplerini getirir.",
            ));
        }
        let left = container(iced::widget::scrollable(list).height(Fill)).width(260.0);
        let right: Element<'_, Message> = match w.uses.get(w.chosen) {
            None => label::caption("").into(),
            Some(u) => self.opening_type_form(w, &u.kind, &f),
        };
        let body = row![left, container(right).width(Fill)]
            .spacing(16)
            .height(440.0);
        let chosen = !w.uses.is_empty();
        let changed = self.opening_type_change().is_some()
            || w.uses
                .get(w.chosen)
                .is_some_and(|u| !w.name.trim().is_empty() && w.name.trim() != u.kind.name);
        Frame::new(TITLE)
            .push(body)
            .action(words::secondary(NEW, Some(msg(Event::New))))
            .action(words::secondary(REMOVE, chosen.then(|| msg(Event::Remove))))
            .action(words::secondary(IMPORT, Some(msg(Event::Import))))
            .action(words::secondary(EXPORT, chosen.then(|| msg(Event::Export))))
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .action(words::primary(
                APPLY,
                (chosen && changed).then(|| msg(Event::Apply)),
            ))
            .width(900.0)
            .modal(msg(Event::Close))
    }

    fn opening_type_form<'a>(
        &'a self,
        w: &'a Window,
        t: &'a OpeningType,
        f: &Format,
    ) -> Element<'a, Message> {
        let unit = f.length_unit_label();
        let name =
            kentos_ui::widget::text_field("Tipin adı", &w.name).on_input(|t| msg(Event::Name(t)));
        let by = Select::new(
            OpeningSizeBy::ALL.iter().map(|b| Choice::new(b.label())),
            OpeningSizeBy::ALL.iter().position(|b| *b == w.size_by),
            |i| {
                msg(Event::SizeBy(
                    OpeningSizeBy::ALL.get(i).copied().unwrap_or_default(),
                ))
            },
        )
        .searchable(false);
        let nominal =
            kentos_ui::widget::text_field("", &w.nominal).on_input(|t| msg(Event::Nominal(t)));
        let operation = Select::new(
            OpeningOperation::ALL.iter().map(|o| Choice::new(o.label())),
            OpeningOperation::ALL.iter().position(|o| *o == w.operation),
            |i| {
                msg(Event::Operation(
                    OpeningOperation::ALL
                        .get(i)
                        .copied()
                        .unwrap_or(OpeningOperation::Swing),
                ))
            },
        )
        .searchable(false);
        let source = t
            .source
            .clone()
            .map_or_else(|| "Proje".to_owned(), |s| format!("Kitaplık: {s}"));
        let mut cells: Vec<Element<'a, Message>> = vec![
            words::field("Ad", name, Some(format!("{} · {source}", t.form.label()))),
            words::field("Ölçü referansı", by, None),
            words::field(
                "Ölçü",
                nominal,
                Some(format!("{} ({unit})", w.size_by.label())),
            ),
        ];
        if t.form != OpeningForm::Opening {
            cells.push(words::field("Açılış", operation, None));
        }
        for (field, text) in &w.typed {
            let field = *field;
            let input = kentos_ui::widget::text_field("Yok", text)
                .on_input(move |t| msg(Event::Field(field, t)));
            cells.push(words::field(field.label(), input, None));
        }
        let mut grid = Column::new().spacing(10);
        let mut cells = cells.into_iter();
        loop {
            let a: Vec<Element<'a, Message>> = cells.by_ref().take(3).collect();
            if a.is_empty() {
                break;
            }
            let mut r = row![].spacing(12).align_y(iced::Top);
            let n = a.len();
            for c in a {
                r = r.push(container(c).width(Fill));
            }
            for _ in n..3 {
                r = r.push(container(label::caption("")).width(Fill));
            }
            grid = grid.push(r);
        }
        // The four widths of the saved type, and of the change typed when it can be planned.
        let sizes = |s: &kentos_commands::OpeningSizes| {
            let mut parts = vec![
                format!("Kaba boşluk {}", f.length(s.rough)),
                format!("Kasa dış eni {}", f.length(s.unit)),
                format!("Kanat eni {}", f.length(s.leaf)),
            ];
            if let Some(c) = s.clear {
                parts.push(format!("Net geçiş {}", f.length(c)));
            }
            parts.join(" · ")
        };
        let mut summary = vec![words::text_line(
            Kind::Ok,
            format!("Şimdi: {}", sizes(&t.fields().sizes())),
        )];
        match &w.preview {
            Some(Ok(done)) => {
                if let Some(n) = done.types.iter().find(|x| x.id == t.id) {
                    summary.push(words::text_line(Kind::Ok, format!("Değişince: {}", sizes(&n.fields().sizes()))));
                }
                summary.push(words::text_line(
                    if done.report.refused.is_empty() { Kind::Ok } else { Kind::Warn },
                    format!("Uygula: {}.", report_words(&done.report)),
                ));
                for k in done.report.kept.iter().take(3) {
                    let fields: Vec<&str> = k.fields.iter().map(|x| x.label()).collect();
                    summary.push(words::text_line(Kind::Warn, format!("İstisna korunur: {}", fields.join(", "))));
                }
            }
            Some(Err(why)) => summary.push(words::text_line(Kind::Warn, why.clone())),
            None => summary.push(words::text_line(
                Kind::Ok,
                format!(
                    "{} açıklık bu tipi izliyor; değer yazınca Uygula'dan önce etkilenenler sayılır.",
                    w.uses.get(w.chosen).map_or(0, |u| u.count)
                ),
            )),
        }
        // What the change does first, the values below it, scrolled.
        let note = label::caption(format!(
            "Uzunluklar {unit} cinsinden; boş bırakılan alan tipte yok."
        ));
        Column::new()
            .spacing(12)
            .push(words::summary(summary))
            .push(note)
            .push(
                iced::widget::scrollable(container(grid).padding(iced::Padding {
                    right: 16.0,
                    ..iced::Padding::ZERO
                }))
                .height(Fill),
            )
            .into()
    }

    /// The window's buttons by their words (a trace's `dialog` step).
    pub(crate) fn opening_types_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.opening_types.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The window's fields filled from the type `t`.
fn fill(w: &mut Window, t: &OpeningType, f: &Format) {
    w.name = t.name.clone();
    w.size_by = t.build.as_ref().and_then(|b| b.size_by).unwrap_or_default();
    w.nominal = f.length_bare(t.fields().sizes().nominal);
    w.operation = t.operation;
    w.typed.clear();
    refit(w, t, f);
    w.preview = None;
}

/// The fields `t` takes moving as the window's operation: those already typed kept, the others `t`'s own.
fn refit(w: &mut Window, t: &OpeningType, f: &Format) {
    let typed = std::mem::take(&mut w.typed);
    w.typed = LENGTHS
        .iter()
        .chain(COUNTS.iter())
        .chain(SWITCHES.iter())
        .copied()
        .filter(|&x| shown(t.form, w.operation, x))
        // A leaves' share only for two leaves.
        .filter(|&x| x != OpeningTypeField::Split || t.leaves == 2)
        .map(|x| {
            if let Some((_, text)) = typed.iter().find(|(k, _)| *k == x) {
                return (x, text.clone());
            }
            let v = t.value(x);
            let text = if LENGTHS.contains(&x) {
                length_text(f, &v)
            } else if SWITCHES.contains(&x) {
                if v == Value::Bool(true) { "Evet".to_owned() } else { String::new() }
            } else {
                v.as_f64().map_or_else(String::new, |n| {
                    if n.fract() == 0.0 {
                        format!("{n:.0}")
                    } else {
                        format!("{n}")
                    }
                })
            };
            (x, text)
        })
        .collect();
}

#[cfg(test)]
#[path = "opening_types_tests.rs"]
mod tests;
