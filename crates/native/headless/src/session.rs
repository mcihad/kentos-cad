//! A drawing open without a window.

use std::path::{Path, PathBuf};

use kentos_commands::{Bounds, CallEnvelope, CommandResult, DocumentSnapshotV1, LayerNode};
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
    selection: Vec<kentos_domain::Uuid>,
    opened_cad: Option<Value>,
    group: Option<Group>,
    jobs: crate::rpc::Jobs,
    /// What opening found of the bound files (docs/adr/0506).
    located: kentos_native_application::source_places::Located,
    /// The grids the project names and where opening found them (docs/adr/3580): a package's `izgaralar/` read in.
    grids: Vec<kentos_native_application::project_grids::GridFound>,
    /// The data source project it is (docs/adr/9107, 9116): a save without a path writes there.
    stored: Option<Stored>,
    /// A project made of a GeoPackage's tables and kept nowhere yet (docs/adr/9122): a save without a path keeps it in
    /// that file, under that name; opening wrote nothing.
    offered: Option<Stored>,
    /// What opening a GeoPackage found (docs/adr/9116): the project kept or made, kept in the file or not, the notes.
    opened_gpkg: Option<Value>,
}

/// A project kept in a data source: its address, its schema in a database and its name.
#[derive(Clone, Debug)]
struct Stored {
    address: String,
    schema: Option<String>,
    name: String,
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

    /// A `.kcad` file (v2/v1), a DWG/DXF/NCZ opened as a new CAD drawing, or a GeoPackage opened as a project
    /// ([`Session::open_geopackage`]).
    pub fn open(path: &Path) -> Result<Self, HeadlessError> {
        Self::open_in(path, None)
    }

    /// A GeoPackage opened as a project (docs/adr/9116): the project named `project`, or its only one; a file of none
    /// opens as a new project of its tables, made in memory: the file is not written (docs/adr/9122), and a save
    /// without a path keeps the project in it. A file of several projects and none named is refused with their names
    /// (`several_projects`).
    #[cfg(feature = "mod-gis")]
    pub fn open_geopackage(path: &Path, project: Option<&str>) -> Result<Self, HeadlessError> {
        use kentos_native_application::gpkg_open::{self, Opened};
        let address = gpkg_open::address_of(path);
        let file = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
        match gpkg_open::open(path, project).map_err(|e| HeadlessError::new("source_unreadable", e))? {
            Opened::Kept { name, .. } => {
                let mut session = Self::open_from_source(&address, None, &name)?;
                session.opened_gpkg = Some(json!({"geoPackage": address, "project": name, "made": false, "kept": true}));
                Ok(session)
            }
            Opened::Several(kept) => Err(HeadlessError::new(
                "several_projects",
                format!(
                    "“{file}” içinde birden çok proje var: {}. Açılacak projenin adını verin.",
                    kept.iter().map(|p| format!("“{}”", p.name)).collect::<Vec<_>>().join(", ")
                ),
            )),
            Opened::Built(built) => {
                let gpkg_open::Built { document, name, notes } = *built;
                let mut session = Self::of(document, None, false);
                let notes: Vec<Value> = notes.iter().map(|n| json!({"code": n.code, "text": n.text})).collect();
                session.opened_gpkg =
                    Some(json!({"geoPackage": address, "project": name, "made": true, "kept": false, "notes": notes}));
                session.offered = Some(Stored { address, schema: None, name });
                Ok(session)
            }
        }
    }

    /// Without the CBS module a GeoPackage is not opened as a project (docs/adr/8054 §11).
    #[cfg(not(feature = "mod-gis"))]
    pub fn open_geopackage(path: &Path, _project: Option<&str>) -> Result<Self, HeadlessError> {
        Err(HeadlessError::new(
            "module_not_built",
            format!("{}: GeoPackage'ı proje olarak açmak KentOS CBS modülüyle gelir; bu yapıda yok.", path.display()),
        ))
    }

    /// Open with an explicit source unit. Unitless external sources require a decision,
    /// as in the desktop's opening preview; native KCAD already has physical model lengths.
    pub fn open_in(path: &Path, file_unit: Option<kentos_commands::FileUnit>) -> Result<Self, HeadlessError> {
        if file_unit.is_none() && kentos_native_application::project_store::is_geopackage(path) {
            return Self::open_geopackage(path, None);
        }
        let native_content = (|| {
            use std::io::Read;
            let mut prefix = [0u8; 64];
            let count = std::fs::File::open(path).ok()?.read(&mut prefix).ok()?;
            Some(matches!(kentos_kcad::sniff(&prefix[..count]), kentos_kcad::Sniff::Kcad | kentos_kcad::Sniff::KcadDamaged | kentos_kcad::Sniff::Json))
        })().unwrap_or(false);
        if let Some(format) = kentos_native_application::cad_open::Format::of(path).filter(|_| !native_content) {
            let bytes = kentos_native_application::file_import::read_cad_bytes(path)
                .map_err(|e| HeadlessError::new("file_unreadable", e.message))?;
            let opened = kentos_native_application::cad_open::read_in(path, &bytes, format, file_unit, &mut kentos_formats::watch::Quiet)
                .map_err(|e| HeadlessError::new(e.code, e.message))?;
            if opened.unit.as_ref().is_some_and(|u| u.assumed) {
                return Err(HeadlessError::new("source_unit_required", "Dosya güvenilir bir birim bildirmiyor. file_unit ile kaynak birimini açıkça seçin veya bilinen bir ölçüyle içe aktarın."));
            }
            let source = json!({"path":path.display().to_string(), "unit":opened.unit, "report":opened.report});
            let mut session = Self::of(opened.document, None, false);
            session.opened_cad = Some(source);
            return Ok(session);
        }
        if file_unit.is_some() {
            return Err(HeadlessError::new("invalid_input", "KCAD dosyasının kaynak birimi değiştirilemez; kayıtlı model uzunlukları metredir. cad.units.change kullanın."));
        }
        let (mut doc, legacy) = files::read(path)?;
        // Bound GeoPackages moved with the project are found again (docs/adr/0506).
        let located = kentos_native_application::source_places::relocate(&mut doc, path);
        // The grids it names found beside it too: a delivery package's izgaralar/ (docs/adr/3580).
        let grids = kentos_native_application::project_grids::find(&doc, path);
        let mut session = Self::of(doc, Some(path.to_path_buf()), legacy);
        session.located = located;
        session.grids = grids;
        Ok(session)
    }

    /// What opening found of the bound files: moved to their relative path, missing, another content there.
    pub fn located(&self) -> &kentos_native_application::source_places::Located {
        &self.located
    }

    /// A drawing from a file's bytes (no path: a save names one).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HeadlessError> {
        let (doc, legacy) = files::from_bytes(bytes)?;
        Ok(Self::of(doc, None, legacy))
    }

    fn of(doc: Document, path: Option<PathBuf>, legacy: bool) -> Self {
        // İşlemler's tools for cad.processing.tools and .run (docs/adr/0508), CBS's (docs/adr/8054 §11).
        #[cfg(feature = "mod-gis")]
        kentos_processing::command::register();
        Self {
            doc,
            path,
            legacy,
            selection: Vec::new(),
            opened_cad: None,
            group: None,
            jobs: crate::rpc::Jobs::default(),
            located: Default::default(),
            grids: Vec::new(),
            stored: None,
            offered: None,
            opened_gpkg: None,
        }
    }

    /// Explicit session selection; unknown or malformed ids leave the previous selection intact.
    pub fn select(&mut self, given: &Value) -> Result<usize, HeadlessError> {
        let slots = crate::selection::resolve(&self.doc, given)?;
        self.selection = slots.iter().filter_map(|s| self.doc.uid(*s)).collect();
        Ok(self.selection.len())
    }

    /// The session's persistent ids, omitting objects removed since they were selected.
    pub fn selection(&self) -> Vec<String> {
        self.selection.iter().filter(|u| self.doc.slot_of(**u).is_some()).map(|u| u.to_string()).collect()
    }

    /// The document itself, for Rust hosts.
    pub fn document(&self) -> &Document {
        &self.doc
    }

    /// Saves as `.kcad` v2 to `path`, or where it was read from. A v1 file
    /// is not written over: its drawing goes to a new path. The drawing is
    /// clean afterwards; the answer says where and how much.
    /// Saves the drawing as a project kept in a data source (docs/adr/9107): a GeoPackage file's or a PostgreSQL
    /// schema's `kentos_projects` table, written and read back; a GeoPackage's file is where its relative paths start
    /// from. The answer: where, the bytes, the revision.
    pub fn save_to_source(&mut self, address: &str, schema: Option<&str>, name: &str) -> Result<Value, HeadlessError> {
        if self.group.is_some() {
            return Err(HeadlessError::new(
                "busy",
                "Açık bir işlem grubu varken kaydedilmez: önce grubu bitirin ya da iptal edin.",
            ));
        }
        let revision = self.doc.revision();
        let mut snapshot = self.doc.to_snapshot_v2();
        kentos_native_application::project_store::with_relative(&mut snapshot, address);
        let bytes = kentos_kcad::encode_verified(&snapshot)
            .map_err(|e| HeadlessError::new("file_unwritable", format!("çizim KCAD v2'ye yazılamadı: {e}")))?;
        kentos_native_application::project_store::write(address, schema, name, &bytes)
            .map_err(|e| HeadlessError::new("source_unwritable", e))?;
        self.doc.mark_saved(revision);
        self.stored = Some(Stored { address: address.to_owned(), schema: schema.map(str::to_owned), name: name.trim().to_owned() });
        self.offered = None;
        self.path = None;
        let mut saved = json!({
            "address": address,
            "schema": schema,
            "name": name.trim(),
            "bytes": bytes.len(),
            "revision": revision.to_string(),
        });
        // A GeoPackage's data layers give their looks to its style table (docs/adr/9117 §4).
        #[cfg(feature = "mod-gis")]
        if let Some(file) = kentos_native_application::project_store::file_of(address) {
            match kentos_native_application::gpkg_looks::write(&self.doc, &file) {
                Ok(looks) => {
                    saved["looks"] = json!(looks.written);
                    if !looks.left.is_empty() {
                        saved["looksLeft"] = json!(looks.left);
                    }
                }
                Err(why) => saved["looksUnwritten"] = json!(why),
            }
        }
        Ok(saved)
    }

    /// A project kept in a data source (docs/adr/9107), opened: a GeoPackage's data layers and bound files found from
    /// that file when they are not at their addresses.
    pub fn open_from_source(address: &str, schema: Option<&str>, name: &str) -> Result<Self, HeadlessError> {
        let bytes = kentos_native_application::project_store::read(address, schema, name)
            .map_err(|e| HeadlessError::new("source_unreadable", e))?;
        // A GeoPackage's linked rasters and pictures written relative to it come back absolute (docs/adr/9117 §3).
        let snapshot = kentos_native_application::project_store::decode(address, &bytes)
            .map_err(|e| HeadlessError::new("file_unreadable", format!("KCAD v2 dosyası okunamadı: {e}")))?;
        let mut doc = Document::from_snapshot_v2(snapshot).map_err(|e| HeadlessError::new("file_unreadable", e))?;
        let legacy = false;
        let located = match kentos_native_application::project_store::place_of(address) {
            Some(place) => kentos_native_application::source_places::relocate(&mut doc, &place),
            None => Default::default(),
        };
        let mut session = Self::of(doc, None, legacy);
        session.located = located;
        session.stored = Some(Stored { address: address.to_owned(), schema: schema.map(str::to_owned), name: name.trim().to_owned() });
        Ok(session)
    }

    /// The projects kept in a data source: name, who wrote it last and when, its bytes.
    pub fn projects_in(address: &str, schema: Option<&str>) -> Result<Value, HeadlessError> {
        let list = kentos_native_application::project_store::list(address, schema)
            .map_err(|e| HeadlessError::new("source_unreadable", e))?;
        Ok(Value::Array(
            list.into_iter()
                .map(|p| json!({"name": p.name, "modified": p.modified, "user": p.user, "bytes": p.size}))
                .collect(),
        ))
    }

    pub fn save(&mut self, path: Option<&Path>) -> Result<Value, HeadlessError> {
        if self.group.is_some() {
            return Err(HeadlessError::new(
                "busy",
                "Açık bir işlem grubu varken kaydedilmez: önce grubu bitirin ya da iptal edin.",
            ));
        }
        if path.is_some_and(|p| kentos_native_application::cad_open::Format::of(p).is_some()) {
            return Err(HeadlessError::new("invalid_file_path", "Kaydet yalnız KentOS (.kcad) biçimindedir. DWG/DXF/NCZ kaynağı üzerine yazılmaz; yeni bir .kcad yolu verin."));
        }
        // A data source project is written there again (docs/adr/9107, 9116); one made of a GeoPackage's tables is kept
        // in that file now, by this save (docs/adr/9122).
        if path.is_none()
            && self.path.is_none()
            && let Some(at) = self.stored.clone().or_else(|| self.offered.clone())
        {
            return self.save_to_source(&at.address, at.schema.as_deref(), &at.name);
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
                    "Kaydedilecek KentOS dosyası henüz seçilmedi; .kcad uzantılı bir yol verin.",
                ));
            }
        };
        let revision = self.doc.revision();
        let bytes = files::write(&self.doc, &target)?;
        self.doc.mark_saved(revision);
        self.path = Some(target.clone());
        self.legacy = false;
        // Saved to a file, the drawing is that file's from now on.
        self.stored = None;
        self.offered = None;
        Ok(json!({
            "path": target.display().to_string(),
            "bytes": bytes,
            "revision": revision.to_string(),
        }))
    }

    /// The whole drawing as a DXF R2007 (AC1021) file at `path` (docs/adr/0252),
    /// the desktop's DXF dışa aktar with Tümü: `file.export.dxf` (docs/adr/0253
    /// §10), over a file that is there. An export is not a save: the drawing
    /// stays as dirty as it was. The answer is the command's output.
    pub fn export_dxf(&mut self, path: &Path) -> Result<Value, HeadlessError> {
        self.file_command(
            kentos_commands::FILE_EXPORT_DXF,
            json!({ "path": path.display().to_string(), "overwrite": true }),
        )
    }

    /// The project's sheet book as a PDF at `path` (docs/adr/0252 §3):
    /// `file.export.pdf` (docs/adr/0253 §10) of the sheets named (none: every
    /// sheet, in the book's order), over a file that is there. Rasterization
    /// choices are said in `notes`, the core's findings in `findings`.
    /// An export is not a save.
    pub fn export_pdf(&mut self, path: &Path, sheets: &[String]) -> Result<Value, HeadlessError> {
        let mut input = json!({ "path": path.display().to_string(), "overwrite": true });
        if !sheets.is_empty() {
            input["sheets"] = json!(sheets);
        }
        self.file_command(kentos_commands::FILE_EXPORT_PDF, input)
    }

    /// A file command's execute: its output, or its refusal as the host's error.
    fn file_command(&mut self, command: &str, mut input: Value) -> Result<Value, HeadlessError> {
        dispatch::with_own_file(command, &mut input, self.path.as_deref());
        let answer = crate::file_commands::call(
            &mut self.doc,
            CallEnvelope {
                relax_constraints: false,
                command: command.to_owned(),
                version: None,
                op: kentos_commands::CallOp::Execute,
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
        let mut envelope = crate::rpc::envelope(&envelope)?;
        dispatch::with_own_file(&envelope.command, &mut envelope.input, self.path.as_deref());
        to_json(&crate::dispatch::call_envelope(
            &mut self.doc,
            None,
            envelope,
        ))
    }

    /// [`Session::call`] for an agent (docs/adr/0743): refused when it names a field the drawing hides from
    /// agents or sends a processing result to a file; a file it writes holds none of those fields.
    pub fn call_for_agent(&mut self, envelope: Value) -> Result<Value, HeadlessError> {
        let mut envelope = crate::rpc::envelope(&envelope)?;
        dispatch::with_own_file(&envelope.command, &mut envelope.input, self.path.as_deref());
        to_json(&crate::dispatch::call_envelope_for_agent(
            &mut self.doc,
            None,
            envelope,
        ))
    }

    /// [`Session::start`] for an agent: as [`Session::call_for_agent`] rules.
    pub fn start_for_agent(&mut self, envelope: Value) -> Result<Value, HeadlessError> {
        let mut envelope = crate::rpc::envelope(&envelope)?;
        dispatch::with_own_file(&envelope.command, &mut envelope.input, self.path.as_deref());
        to_json(&self.jobs.start_for_agent(&self.doc, envelope)?)
    }

    /// Starts a command as a job (docs/adr/0253 §9): computed on a copy of
    /// the drawing on another thread; its `JobStatus`.
    pub fn start(&mut self, envelope: Value) -> Result<Value, HeadlessError> {
        let mut envelope = crate::rpc::envelope(&envelope)?;
        dispatch::with_own_file(&envelope.command, &mut envelope.input, self.path.as_deref());
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
        let journal: kentos_commands::ReplayJournal =
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
        mut input: Value,
    ) -> Result<Value, HeadlessError> {
        dispatch::with_own_file(command, &mut input, self.path.as_deref());
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
            if let Some(source) = &self.opened_cad { fields.insert("openedCad".into(), source.clone()); }
            if let Some(at) = &self.stored {
                fields.insert("project".into(), json!({"address": at.address, "schema": at.schema, "name": at.name, "kept": true}));
            } else if let Some(at) = &self.offered {
                // Made of the file's tables, kept nowhere yet: a save without a path keeps it there (docs/adr/9122).
                fields.insert("project".into(), json!({"address": at.address, "schema": at.schema, "name": at.name, "kept": false}));
            }
            if let Some(opened) = &self.opened_gpkg {
                fields.insert("openedGeoPackage".into(), opened.clone());
            }
            // Docs/adr/3580: the grids the project names and where opening found them.
            if !self.grids.is_empty() {
                let grids: Vec<Value> = self
                    .grids
                    .iter()
                    .map(|g| json!({"name": g.name, "sha256": g.sha256, "kind": g.kind.name(), "setting": g.setting, "place": g.place.name()}))
                    .collect();
                fields.insert("grids".into(), Value::Array(grids));
            }
        }
        summary
    }

    /// The layer tree (groups and layers, their styles and flags).
    pub fn layers(&self) -> &[LayerNode] {
        self.doc.layers().nodes()
    }

    /// The block definitions in the drawing's order (docs/adr/0144).
    pub fn blocks(&self) -> Vec<&kentos_commands::BlockDefinition> {
        self.doc.blocks().iter().map(|b| &**b).collect()
    }

    /// The associative arrays in the order they were made (docs/adr/0300).
    pub fn arrays(&self) -> Vec<&kentos_commands::AssocArray> {
        self.doc.arrays().iter().map(|a| &**a).collect()
    }

    /// The constraints and the parameter table (docs/adr/0334).
    pub fn constraints(&self) -> &kentos_commands::ConstraintSet {
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
        Ok(query::page(
            &self.doc,
            &Filter {
                layer,
                kinds,
                bbox,
                after,
                limit,
            },
        )?)
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
        Ok(query::measure(&self.doc, uid)?)
    }

    /// An object's measures taken by `method` (docs/adr/0256 §4).
    pub fn measure_by(&self, uid: &str, method: query::Method) -> Result<Measure, HeadlessError> {
        Ok(query::measure_by(&self.doc, uid, method)?)
    }

    /// Many objects' measures taken by `method`, at most [`query::MEASURE_MAX`].
    pub fn measure_many(
        &self,
        uids: &[String],
        method: query::Method,
    ) -> Result<query::Measures, HeadlessError> {
        Ok(query::measure_many(&self.doc, uids, method)?)
    }

    /// Undoes the last step; its name, or none when there is nothing to undo.
    pub fn undo(&mut self) -> Option<String> {
        self.doc.undo()
    }

    /// Redoes the last undone step; its name, or none.
    pub fn redo(&mut self) -> Option<String> {
        self.doc.redo()
    }

    /// Undo with an explicit conflict instead of overwriting a later paper edit.
    pub fn try_undo(&mut self) -> Result<Option<String>, HeadlessError> {
        if let Some(problem) = self.doc.sheet_history_problem(false) {
            return Err(HeadlessError::new("sheet_history_conflict", problem));
        }
        Ok(self.undo())
    }

    /// Redo with the same paper ownership check.
    pub fn try_redo(&mut self) -> Result<Option<String>, HeadlessError> {
        if let Some(problem) = self.doc.sheet_history_problem(true) {
            return Err(HeadlessError::new("sheet_history_conflict", problem));
        }
        Ok(self.redo())
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
    kentos_commands::codes::info(code).map_or("file_unwritable", |c| c.code)
}

/// A headless drawing as the agents' tools' host (docs/adr/8056 §2): the drawing's requests as an agent may see
/// them, the session's own selection.
impl crate::agent_tools::Host for Session {
    fn request(&mut self, method: &str, params: &Value) -> Result<Value, HeadlessError> {
        if method == "selection" {
            return Ok(serde_json::json!({ "uids": self.selection() }));
        }
        crate::rpc::call_for_agent(
            &mut self.doc,
            Some(&mut self.jobs),
            self.path.as_deref(),
            method,
            params,
        )
    }
}

