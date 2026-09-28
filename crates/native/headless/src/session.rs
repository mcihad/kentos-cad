//! A drawing open without a window.

use std::path::{Path, PathBuf};

use kentos_contracts::{Bounds, DocumentSnapshotV1, LayerNode};
use kentos_domain::{Document, Group};
use serde_json::{Value, json};

use crate::HeadlessError;
use crate::dispatch::{self, Op};
use crate::files;
use crate::query::{self, Filter, Measure, Page};

/// What a new drawing starts as: the new project's choices
/// (`kentos_project::new_project`, the web's `model/newProject.ts`).
pub use kentos_project::new_project::NewProject as NewDrawing;

/// A drawing, where it was read from, and the script's open group.
pub struct Session {
    doc: Document,
    path: Option<PathBuf>,
    /// Read from a v1 JSON file: saved only to a new path, never over it.
    legacy: bool,
    group: Option<Group>,
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
        Ok(json!({ "uid": uid, "entity": e }))
    }

    /// An object's area, length and box.
    pub fn measure(&self, uid: &str) -> Result<Measure, HeadlessError> {
        query::measure(&self.doc, uid)
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
