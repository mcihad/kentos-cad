//! A drawing open without a window.

use std::path::{Path, PathBuf};

use kentos_contracts::{Bounds, CallEnvelope, CommandResult, DocumentSnapshotV1, LayerNode};
use kentos_domain::{Document, Group};
use serde_json::{Value, json};

use crate::HeadlessError;
use crate::dispatch::{self, Op};
use crate::files;
use crate::query::{self, Filter, Measure, Page};

/// What a new drawing starts as: the new project's choices
/// (`kentos_project::new_project`, the web's `model/newProject.ts`).
pub use kentos_project::new_project::NewProject as NewDrawing;

/// A new drawing's choices asked by name (docs/adr/0245): `name`, `workspace`,
/// `coords`, `srid`, `province`, `drawingUnit`, `plotScale`, `drawingFont`;
/// the wizard's rules make the project (`kentos_project::new_request`) and
/// its refusals are the wizard's words.
pub fn new_drawing(input: &Value) -> Result<NewDrawing, HeadlessError> {
    kentos_project::new_request::NewRequest::read(input)
        .and_then(|r| r.project())
        .map_err(|r| HeadlessError::new(r.code, r.message))
}

/// A drawing, where it was read from, the script's open group and the
/// commands it runs as jobs.
pub struct Session {
    doc: Document,
    path: Option<PathBuf>,
    /// Read from a v1 JSON file: saved only to a new path, never over it.
    legacy: bool,
    group: Option<Group>,
    jobs: crate::rpc::Jobs,
}

impl Session {
    /// A new project: the standard layer tree, the zone's work area, the
    /// default units (as Yeni proje makes it).
    pub fn new(choices: &NewDrawing) -> Result<Self, HeadlessError> {
        let snapshot: DocumentSnapshotV1 = kentos_project::new_project::new_project(choices)
            .map_err(|e| HeadlessError::new("unknown_system", e))?;
        let doc = Document::from_snapshot(snapshot)
            .map_err(|e| HeadlessError::new("invalid_input", e))?;
        Ok(Self::of(doc, None, false))
    }

    /// A `.kcad` file (v2, or v1 JSON).
    pub fn open(path: &Path) -> Result<Self, HeadlessError> {
        let (doc, legacy) = files::read(path)?;
        Ok(Self::of(doc, Some(path.to_path_buf()), legacy))
    }

    /// A drawing from a file's bytes (no path: a save names one).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HeadlessError> {
        let (doc, legacy) = files::from_bytes(bytes)?;
        Ok(Self::of(doc, None, legacy))
    }

    fn of(doc: Document, path: Option<PathBuf>, legacy: bool) -> Self {
        Self {
            doc,
            path,
            legacy,
            group: None,
            jobs: crate::rpc::Jobs::default(),
        }
    }

    /// The document itself, for Rust hosts.
    pub fn document(&self) -> &Document {
        &self.doc
    }

    /// Saves as `.kcad` v2 to `path`, or where it was read from. A v1 file
    /// is not written over: its drawing goes to a new path. The drawing is
    /// clean afterwards; the answer says where and how much.
    pub fn save(&mut self, path: Option<&Path>) -> Result<Value, HeadlessError> {
        if self.group.is_some() {
            return Err(HeadlessError::new(
                "busy",
                "Açık bir işlem grubu varken kaydedilmez: önce grubu bitirin ya da iptal edin.",
            ));
        }
        let target = match (path, &self.path) {
            (Some(p), _) => p.to_path_buf(),
            (None, Some(p)) if !self.legacy => p.clone(),
            (None, Some(p)) => {
                return Err(HeadlessError::new(
                    "legacy_file",
                    format!(
                        "{} eski (v1 JSON) bir dosya; üzerine yazılmaz. KCAD v2 için yeni bir yol verin.",
                        p.display()
                    ),
                ));
            }
            (None, None) => {
                return Err(HeadlessError::new(
                    "no_path",
                    "Çizim bir dosyadan açılmadı: kaydetmek için bir yol verin.",
                ));
            }
        };
        let revision = self.doc.revision();
        let bytes = files::write(&self.doc, &target)?;
        self.doc.mark_saved(revision);
        self.path = Some(target.clone());
        self.legacy = false;
        Ok(json!({
            "path": target.display().to_string(),
            "bytes": bytes,
            "revision": revision.to_string(),
        }))
    }

    /// The whole drawing as an AutoCAD 2007 DXF file at `path` (docs/adr/0252),
    /// the desktop's DXF dışa aktar with Tümü: `file.export.dxf` (docs/adr/0253
    /// §10), over a file that is there. An export is not a save: the drawing
    /// stays as dirty as it was. The answer is the command's output.
    pub fn export_dxf(&mut self, path: &Path) -> Result<Value, HeadlessError> {
        self.file_command(
            kentos_contracts::FILE_EXPORT_DXF,
            json!({ "path": path.display().to_string(), "overwrite": true }),
        )
    }

    /// The project's sheet book as a PDF at `path` (docs/adr/0252 §3):
    /// `file.export.pdf` (docs/adr/0253 §10) of the sheets named (none: every
    /// sheet, in the book's order), over a file that is there. What a headless
    /// host cannot draw (the maps' texts) is said in `notes`, the core's
    /// findings in `findings`. An export is not a save.
    pub fn export_pdf(&mut self, path: &Path, sheets: &[String]) -> Result<Value, HeadlessError> {
        let mut input = json!({ "path": path.display().to_string(), "overwrite": true });
        if !sheets.is_empty() {
            input["sheets"] = json!(sheets);
        }
        self.file_command(kentos_contracts::FILE_EXPORT_PDF, input)
    }

    /// A file command's execute: its output, or its refusal as the host's error.
    fn file_command(&mut self, command: &str, input: Value) -> Result<Value, HeadlessError> {
        let answer = crate::file_commands::call(
            &mut self.doc,
            CallEnvelope {
                relax_constraints: false,
                command: command.to_owned(),
                version: None,
                op: kentos_contracts::CallOp::Execute,
                input,
                request_id: None,
                expected: None,
            },
        );
        match answer.result {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                Err(HeadlessError::new(static_code(&error.code), error.message))
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => {
                Err(HeadlessError::new("file_unwritable", "Dosya yazılmadı."))
            }
        }
    }

    /// A command by the protocol (docs/adr/0253 §4): `envelope` is a
    /// `CallEnvelope`'s JSON, the answer a `CallAnswer`'s.
    pub fn call(&mut self, envelope: Value) -> Result<Value, HeadlessError> {
        let envelope = crate::rpc::envelope(&envelope)?;
        to_json(&crate::dispatch::call_envelope(
            &mut self.doc,
            None,
            envelope,
        ))
    }

    /// Starts a command as a job (docs/adr/0253 §9): computed on a copy of
    /// the drawing on another thread; its `JobStatus`.
    pub fn start(&mut self, envelope: Value) -> Result<Value, HeadlessError> {
        let envelope = crate::rpc::envelope(&envelope)?;
        to_json(&self.jobs.start(&self.doc, envelope))
    }

    /// A job as it is now; once computed, its write is in the drawing.
    pub fn job(&mut self, id: &str) -> Result<Value, HeadlessError> {
        to_json(&self.jobs.status(&mut self.doc, id)?)
    }

    /// Waits for a job; its write is in the drawing then.
    pub fn wait(&mut self, id: &str) -> Result<Value, HeadlessError> {
        to_json(&self.jobs.wait(&mut self.doc, id)?)
    }

    /// Asks a job to stop; refused once written (`already_committed`).
    pub fn cancel(&mut self, id: &str) -> Result<Value, HeadlessError> {
        to_json(&self.jobs.cancel(&mut self.doc, id)?)
    }

    /// The drawing's journal (`kentos.replay/1`, docs/adr/0253 §3).
    pub fn journal(&self) -> Value {
        serde_json::to_value(kentos_native_application::replay::journal_of(&self.doc))
            .unwrap_or(Value::Null)
    }

    /// Runs a journal's commands again on this drawing; the report. A row
    /// that wrote a file is planned again and compared, not written.
    pub fn replay(&mut self, journal: Value) -> Result<Value, HeadlessError> {
        let journal: kentos_contracts::ReplayJournal =
            serde_json::from_value(journal).map_err(|e| {
                HeadlessError::new(
                    "invalid_input",
                    format!("kayıt bir kentos.replay/1 kaydı olarak okunamadı: {e}"),
                )
            })?;
        to_json(&kentos_native_application::replay::replay_with(
            &mut self.doc,
            None,
            &journal,
            &mut crate::file_commands::replay_row,
        ))
    }

    /// The `.kcad` v2 bytes of the drawing as it is now, verified.
    pub fn to_bytes(&self) -> Result<Vec<u8>, HeadlessError> {
        kentos_kcad::encode_verified(&self.doc.to_snapshot_v2()).map_err(|e| {
            HeadlessError::new(
                "file_unwritable",
                format!("çizim KCAD v2'ye yazılamadı: {e}"),
            )
        })
    }

    /// Runs a catalog command (see [`crate::dispatch::run`]).
    pub fn run(
        &mut self,
        command: &str,
        version: Option<u32>,
        op: Op,
        input: Value,
    ) -> Result<Value, HeadlessError> {
        dispatch::run(&mut self.doc, command, version, op, input)
    }

    /// The drawing's name, revision, dirty flag, settings and active layer
    /// ([`crate::rpc::summary`]), with the file it came from.
    pub fn summary(&self) -> Value {
        let mut summary = crate::rpc::summary(&self.doc);
        if let Some(fields) = summary.as_object_mut() {
            if let Some(path) = &self.path {
                fields.insert("path".into(), json!(path.display().to_string()));
            }
            fields.insert("legacy".into(), json!(self.legacy));
        }
        summary
    }

    /// The layer tree (groups and layers, their styles and flags).
    pub fn layers(&self) -> &[LayerNode] {
        self.doc.layers().nodes()
    }

    /// The block definitions in the drawing's order (docs/adr/0144).
    pub fn blocks(&self) -> Vec<&kentos_contracts::BlockDefinition> {
        self.doc.blocks().iter().map(|b| &**b).collect()
    }

    /// The associative arrays in the order they were made (docs/adr/0300).
    pub fn arrays(&self) -> Vec<&kentos_contracts::AssocArray> {
        self.doc.arrays().iter().map(|a| &**a).collect()
    }

    /// The constraints and the parameter table (docs/adr/0334).
    pub fn constraints(&self) -> &kentos_contracts::ConstraintSet {
        self.doc.constraints()
    }

    /// A page of objects.
    pub fn entities(
        &self,
        layer: Option<String>,
        kinds: Option<Vec<String>>,
        bbox: Option<Bounds>,
        after: Option<String>,
        limit: Option<usize>,
    ) -> Result<Page<'_>, HeadlessError> {
        query::page(
            &self.doc,
            &Filter {
                layer,
                kinds,
                bbox,
                after,
                limit,
            },
        )
    }

    /// One object by its persistent id, as the wire carries it.
    pub fn entity(&self, uid: &str) -> Result<Value, HeadlessError> {
        let slot = query::slot_of(&self.doc, uid)?;
        let e = self.doc.get(slot).ok_or_else(|| {
            HeadlessError::new(
                "unknown_object",
                format!("{uid} kimlikli nesne bu çizimde yok."),
            )
        })?;
        Ok(json!({ "uid": uid, "stamp": self.doc.stamp(slot), "entity": e }))
    }

    /// An object's area, length and box.
    pub fn measure(&self, uid: &str) -> Result<Measure, HeadlessError> {
        query::measure(&self.doc, uid)
    }

    /// An object's measures taken by `method` (docs/adr/0256 §4).
    pub fn measure_by(&self, uid: &str, method: query::Method) -> Result<Measure, HeadlessError> {
        query::measure_by(&self.doc, uid, method)
    }

    /// Many objects' measures taken by `method`, at most [`query::MEASURE_MAX`].
    pub fn measure_many(
        &self,
        uids: &[String],
        method: query::Method,
    ) -> Result<query::Measures, HeadlessError> {
        query::measure_many(&self.doc, uids, method)
    }

    /// Undoes the last step; its name, or none when there is nothing to undo.
    pub fn undo(&mut self) -> Option<String> {
        self.doc.undo()
    }

    /// Redoes the last undone step; its name, or none.
    pub fn redo(&mut self) -> Option<String> {
        self.doc.redo()
    }

    /// Opens a group: every command until [`end_group`](Self::end_group) is
    /// one undo step named `label` (TODOS.md PY-14); [`cancel_group`](Self::cancel_group)
    /// takes all of it back.
    /// Whether the writes relax the constraints (docs/adr/0337 §1): on until set off.
    pub fn relax(&mut self, on: bool) {
        self.doc.set_relax(if on {
            kentos_domain::Relax::On
        } else {
            kentos_domain::Relax::Off
        });
    }

    pub fn begin_group(&mut self, label: &str) -> Result<(), HeadlessError> {
        if self.group.is_some() {
            return Err(HeadlessError::new(
                "busy",
                "Zaten açık bir işlem grubu var; iç içe grup yok.",
            ));
        }
        self.group = Some(self.doc.begin_group(label));
        Ok(())
    }

    /// Closes the open group as one undo step; false when none was open.
    pub fn end_group(&mut self) -> bool {
        match self.group.take() {
            Some(g) => {
                self.doc.end_group(g);
                true
            }
            None => false,
        }
    }

    /// Takes back everything the open group wrote; false when none was open.
    pub fn cancel_group(&mut self) -> bool {
        match self.group.take() {
            Some(g) => {
                self.doc.cancel_group(g);
                true
            }
            None => false,
        }
    }
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<Value, HeadlessError> {
    serde_json::to_value(value).map_err(|e| {
        HeadlessError::new("unwritable_result", format!("yanıt JSON'a yazılamadı: {e}"))
    })
}

/// A command's code as the host's error code (the host's codes are static).
fn static_code(code: &str) -> &'static str {
    kentos_contracts::codes::info(code).map_or("file_unwritable", |c| c.code)
}
