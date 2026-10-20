//! What an İşlemler tool, a file import or an atlas run as a server job decides without a database, and what its process
//! answers (docs/adr/0559, 0561; plan/gis.md CBS-39): shared by the server, which asks for the run and commits its
//! result, and by the process that runs it (`kentos-jobrun`), which must not carry the server.
//!
//! - **The request** ([`check_input`]): a `project.processing` job's input is checked when it is asked for (a tool's
//!   id, values with no local file in them, the objects and the box as the command takes them, the revision and a
//!   budget no bigger than the server's), and again by the process, whatever the server did.
//! - **What a run did** ([`Processed`], [`ProcessFailure`]): the answer of the process, a [`Delta`] to commit or a
//!   file (a GeoPackage, the atlas's PDF), or why it did not finish; what it was asked ([`RunRequest`]).
//! - **An import and an atlas** ([`check_import_input`], [`check_atlas_input`]): their inputs, checked as a tool's.
//! - **The delta** ([`delta`]): what the tool changed, found by comparing the drawing as it was opened with the
//!   drawing after the run, both through the same document code, so an object the tool did not touch is never sent
//!   back. It is one `project.changes` command's input; the commit is the job's finish.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    BlockChange, BlockDefinition, BlockId, DocumentSnapshotV2, Entity, EntityId, FeatureChange,
    ImportFormat, ProcessingJobInput, ProcessingJobNote, ProcessingUsage, ProjectAtlasInput,
    ProjectChanges, ProjectImportInput, ProjectPatch,
};

/// The most object changes one `project.changes` command carries (the server's limit).
pub const PROJECT_CHANGES_MAX: usize = 5000;
/// The most object and block changes a job's result may carry (docs/adr/0561 §1): a job's commit is made inside the
/// server, not read from a request, so it may be bigger than a request's.
pub const JOB_CHANGES_MAX: usize = 100_000;
/// The most objects a request names as chosen.
pub const PROCESSING_SELECTION_MAX: usize = 100_000;
/// The most values a request gives.
pub const PROCESSING_PARAMETERS_MAX: usize = 256;
/// The smallest memory budget a request may ask for, in megabytes.
pub const PROCESSING_MEMORY_FLOOR_MB: u32 = 64;

/// A fault of a request: the input's path and what to fix.
pub type InputFault = (String, String);

fn fault(path: impl Into<String>, message: impl Into<String>) -> InputFault {
    (path.into(), message.into())
}

/// Whether `value` holds a file path the way the tools' file values do (`{ "path": "…" }`); where, as a path of keys.
fn file_value(value: &serde_json::Value, at: &str) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            if map.get("path").is_some_and(serde_json::Value::is_string) {
                return Some(format!("{at}.path"));
            }
            map.iter()
                .find_map(|(k, v)| file_value(v, &format!("{at}.{k}")))
        }
        serde_json::Value::Array(items) => items
            .iter()
            .enumerate()
            .find_map(|(i, v)| file_value(v, &format!("{at}[{i}]"))),
        _ => None,
    }
}

/// The input of a `project.processing` job, checked (docs/adr/0559 §1): the fault, with its path, or none.
pub fn check_input(input: &ProcessingJobInput) -> Option<InputFault> {
    let tool = input.tool.trim();
    if tool.is_empty() || tool.chars().count() > 200 || tool.chars().any(char::is_control) {
        return Some(fault(
            "input.tool",
            "tool 1–200 karakterlik bir araç kimliği olmalı.",
        ));
    }
    if input.parameters.len() > PROCESSING_PARAMETERS_MAX {
        return Some(fault(
            "input.parameters",
            format!("Bir iş en çok {PROCESSING_PARAMETERS_MAX} değer taşır."),
        ));
    }
    for (name, value) in &input.parameters {
        if let Some(at) = file_value(value, &format!("input.parameters.{name}")) {
            return Some(fault(
                at,
                "Sunucu işi yerel dosya okumaz: tabloyu { name, rows } olarak, nesneleri katmanlarıyla verin.",
            ));
        }
    }
    if input.selection.len() > PROCESSING_SELECTION_MAX {
        return Some(fault(
            "input.selection",
            format!("Bir iş en çok {PROCESSING_SELECTION_MAX} seçili nesne alır."),
        ));
    }
    if let Some(i) = input
        .selection
        .iter()
        .position(|s| EntityId::parse(s).is_none())
    {
        return Some(fault(
            format!("input.selection[{i}]"),
            format!(
                "“{}” bir nesne kimliği (küçük harfli UUID) değil.",
                input.selection[i]
            ),
        ));
    }
    if let Some(v) = input.view
        && (v.iter().any(|n| !n.is_finite()) || v[0] > v[2] || v[1] > v[3])
    {
        return Some(fault(
            "input.view",
            "view (en küçük doğu, en küçük kuzey, en büyük doğu, en büyük kuzey) sonlu ve sıralı olmalı.",
        ));
    }
    if let Some(r) = &input.expected_revision
        && r.parse::<i64>().ok().filter(|n| *n >= 0).is_none()
    {
        return Some(fault(
            "input.expectedRevision",
            format!("expectedRevision bir veri revizyonu (sıfır ya da artı tam sayı) olmalı: {r}"),
        ));
    }
    if input.cpu_seconds == Some(0) {
        return Some(fault("input.cpuSeconds", "cpuSeconds en az 1 olmalı."));
    }
    if input
        .memory_mb
        .is_some_and(|m| m < PROCESSING_MEMORY_FLOOR_MB)
    {
        return Some(fault(
            "input.memoryMb",
            format!("memoryMb en az {PROCESSING_MEMORY_FLOOR_MB} olmalı."),
        ));
    }
    None
}

fn check_run_limits(
    expected: Option<&str>,
    cpu_seconds: Option<u32>,
    memory_mb: Option<u32>,
) -> Option<InputFault> {
    if let Some(r) = expected
        && r.parse::<i64>().ok().filter(|n| *n >= 0).is_none()
    {
        return Some(fault(
            "input.expectedRevision",
            format!("expectedRevision bir veri revizyonu (sıfır ya da artı tam sayı) olmalı: {r}"),
        ));
    }
    if cpu_seconds == Some(0) {
        return Some(fault("input.cpuSeconds", "cpuSeconds en az 1 olmalı."));
    }
    if memory_mb.is_some_and(|m| m < PROCESSING_MEMORY_FLOOR_MB) {
        return Some(fault(
            "input.memoryMb",
            format!("memoryMb en az {PROCESSING_MEMORY_FLOOR_MB} olmalı."),
        ));
    }
    None
}

fn check_text(path: &str, text: &str, what: &str, most: usize) -> Option<InputFault> {
    if text.trim().is_empty() || text.chars().count() > most || text.chars().any(char::is_control) {
        return Some(fault(path, format!("{what} 1–{most} karakter olmalı.")));
    }
    None
}

/// The input of a `project.import` job, checked (docs/adr/0561 §1): the fault, with its path, or none. The staged
/// file's id is a UUID, the options hold no file path (the server gives the file), the budget is no smaller than the
/// floor; whether the staged file exists, is the caller's and the format fits it, is asked of the file's name.
pub fn check_import_input(input: &ProjectImportInput) -> Option<InputFault> {
    if EntityId::parse(&input.upload).is_none() {
        return Some(fault(
            "input.upload",
            format!(
                "“{}” hazırlanmış bir dosyanın kimliği (küçük harfli UUID) değil; önce dosyayı job-inputs ile yükleyin.",
                input.upload
            ),
        ));
    }
    if input.options.len() > PROCESSING_PARAMETERS_MAX {
        return Some(fault(
            "input.options",
            format!("Bir iş en çok {PROCESSING_PARAMETERS_MAX} seçenek taşır."),
        ));
    }
    if input.options.contains_key("path") {
        return Some(fault(
            "input.options.path",
            "Dosyanın yolunu sunucu verir: yüklediğiniz dosya okunur, seçeneklerde yol verilmez.",
        ));
    }
    for (name, value) in &input.options {
        if let Some(at) = file_value(value, &format!("input.options.{name}")) {
            return Some(fault(
                at,
                "Sunucu işi yerel dosya okumaz: seçeneklerde dosya yolu verilmez.",
            ));
        }
    }
    if let Some(label) = &input.label
        && label.chars().count() > 200
    {
        return Some(fault("input.label", "label en çok 200 karakter olmalı."));
    }
    check_run_limits(
        input.expected_revision.as_deref(),
        input.cpu_seconds,
        input.memory_mb,
    )
}

/// The input of a `project.atlas` job, checked (docs/adr/0561 §2).
pub fn check_atlas_input(input: &ProjectAtlasInput) -> Option<InputFault> {
    if let Some(f) = check_text("input.sheet", &input.sheet, "sheet (pafta kimliği)", 200) {
        return Some(f);
    }
    if let Some(fields) = &input.fields {
        if fields.len() > PROCESSING_PARAMETERS_MAX {
            return Some(fault(
                "input.fields",
                format!("Bir iş en çok {PROCESSING_PARAMETERS_MAX} alan adı taşır."),
            ));
        }
        for (i, f) in fields.iter().enumerate() {
            if let Some(fault) = check_text(&format!("input.fields[{i}]"), f, "alan adı", 200) {
                return Some(fault);
            }
        }
    }
    if let Some(theme) = &input.theme
        && let Some(f) = check_text("input.theme", theme, "theme (katman durumunun adı)", 200)
    {
        return Some(f);
    }
    if let Some(time) = &input.time
        && let Some(f) = check_text(
            "input.time",
            time,
            "time (2024-06-30 ya da 2024-06-30T12:00)",
            40,
        )
    {
        return Some(f);
    }
    if input.crs == Some(0) {
        return Some(fault("input.crs", "crs bir EPSG kodu olmalı."));
    }
    if let Some(label) = &input.label
        && label.chars().count() > 200
    {
        return Some(fault("input.label", "label en çok 200 karakter olmalı."));
    }
    check_run_limits(
        input.expected_revision.as_deref(),
        input.cpu_seconds,
        input.memory_mb,
    )
}

// ── What the process is asked and what it answers ───────────────────────────

/// What the process `kentos-jobrun` is asked to do (`request.json` of its folder, docs/adr/0561 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "input", rename_all = "camelCase")]
pub enum RunRequest {
    /// An İşlemler tool (docs/adr/0559).
    Processing(ProcessingJobInput),
    /// A file of the folder (`file`, its name there) read by the format's import command.
    Import(ImportRun),
    /// The atlas of a sheet, written to the folder as `atlas.pdf`.
    Atlas(ProjectAtlasInput),
}

/// An import the process runs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRun {
    pub format: ImportFormat,
    /// The staged file's name as its owner gave it, for the command's words.
    pub name: String,
    #[serde(default)]
    pub options: BTreeMap<String, Value>,
}

/// What a run did.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Processed {
    /// The tool's (or the command's) own sentence.
    pub summary: String,
    pub notes: Vec<ProcessingJobNote>,
    /// `commit`, import: what to write to the project.
    pub delta: Option<Delta>,
    /// A tool's `sink` GeoPackage, the atlas's PDF: the file the run made (beside the answer, not in it).
    #[serde(skip)]
    pub file: Option<Vec<u8>>,
    /// `sink`: how many rows the file holds.
    pub rows: u64,
    /// What the command answered (an import's tables or layers, an atlas's pages and pin), as it said it.
    #[serde(default)]
    pub details: Value,
    /// What the process took, as the parent measured it.
    pub usage: ProcessingUsage,
}

/// Why a tool did not finish: a code (`tool_failed`, `tool_unavailable`, `cpu_limit`, `memory_limit`,
/// `unsupported_change`, `too_many_changes`, `crashed`, `invalid`, or `stopped` for a run the worker stopped) and words.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessFailure {
    pub code: String,
    pub message: String,
}

impl ProcessFailure {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        ProcessFailure {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

// ── The delta ───────────────────────────────────────────────────────────────

/// What a tool changed in the drawing (docs/adr/0559 §2).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Delta {
    /// Created, changed and removed objects, in that order, as one `project.changes` command's.
    pub features: Vec<FeatureChange>,
    /// The block definitions made, replaced and removed, as that command's.
    #[serde(default)]
    pub blocks: Vec<BlockChange>,
    /// The project's metadata the tool changed, whole: the layer tree, the settings, the styles, the sheet book, the
    /// arrays, the constraints.
    pub project: Option<ProjectPatch>,
    pub added: u64,
    pub changed: u64,
    pub removed: u64,
}

impl Delta {
    /// Nothing changed.
    pub fn is_empty(&self) -> bool {
        self.features.is_empty() && self.blocks.is_empty() && self.project.is_none()
    }

    /// The command's input.
    pub fn changes(&self) -> ProjectChanges {
        ProjectChanges {
            features: self.features.clone(),
            project: self.project.clone(),
            blocks: self.blocks.clone(),
        }
    }

    /// The block definitions the command replaces or removes (their versions are expected).
    pub fn guarded_blocks(&self) -> Vec<BlockId> {
        self.blocks
            .iter()
            .filter_map(|c| match c {
                BlockChange::Update { block } => Some(block.id),
                BlockChange::Delete { id } => Some(*id),
                BlockChange::Create { .. } => None,
            })
            .collect()
    }

    /// The objects the command updates or deletes (their versions are expected), by their persistent ids as text.
    pub fn guarded(&self) -> Vec<String> {
        self.features
            .iter()
            .filter_map(|c| match c {
                FeatureChange::Update { id, .. } | FeatureChange::Delete { id } => Some(id.clone()),
                FeatureChange::Create { .. } => None,
            })
            .collect()
    }
}

/// Why a result cannot be a delta: a code and words.
pub type DeltaRefusal = ProcessFailure;

/// The block definitions `after` has that `before` has not, has differently, and has lost, in that order.
fn block_changes(before: &[BlockDefinition], after: &[BlockDefinition]) -> Vec<BlockChange> {
    let was: HashMap<BlockId, &BlockDefinition> = before.iter().map(|b| (b.id, b)).collect();
    let kept: HashSet<BlockId> = after.iter().map(|b| b.id).collect();
    let mut creates = Vec::new();
    let mut updates = Vec::new();
    for block in after {
        match was.get(&block.id) {
            None => creates.push(BlockChange::Create {
                block: block.clone(),
            }),
            Some(old) if !same(*old, block) => updates.push(BlockChange::Update {
                block: block.clone(),
            }),
            Some(_) => {}
        }
    }
    let mut changes = creates;
    changes.extend(updates);
    changes.extend(
        before
            .iter()
            .filter(|b| !kept.contains(&b.id))
            .map(|b| BlockChange::Delete { id: b.id }),
    );
    changes
}

fn same(a: &impl Serialize, b: &impl Serialize) -> bool {
    match (serde_json::to_value(a), serde_json::to_value(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// What the tool changed: `after` against `before`, both as the same document code writes them. An object is created
/// when its persistent id is new, changed when it differs from itself before (its slot number is not content), and
/// removed when it is gone; the metadata is what differs whole. A block definition is created, replaced whole or
/// removed the same way, by its id (docs/adr/0561 §1), and a result takes at most [`JOB_CHANGES_MAX`] object and block
/// changes (`too_many_changes`).
pub fn delta(
    before: &DocumentSnapshotV2,
    after: &DocumentSnapshotV2,
) -> Result<Delta, DeltaRefusal> {
    let blocks = block_changes(&before.blocks, &after.blocks);
    let mut was: HashMap<&EntityId, &Entity> = HashMap::with_capacity(before.uids.len());
    for (uid, e) in before.uids.iter().zip(&before.entities) {
        was.insert(uid, e);
    }
    let (mut added, mut changed) = (0u64, 0u64);
    let mut kept = HashSet::with_capacity(after.uids.len());
    let mut creates = Vec::new();
    let mut updates = Vec::new();
    for (uid, entity) in after.uids.iter().zip(&after.entities) {
        kept.insert(uid);
        match was.get(uid) {
            None => {
                let mut e = entity.clone();
                e.base_mut().id = 0;
                creates.push(FeatureChange::Create {
                    id: uid.to_text(),
                    entity: e,
                });
                added += 1;
            }
            Some(old) => {
                let mut same_slot = entity.clone();
                same_slot.base_mut().id = old.base().id;
                if same_slot != **old {
                    let mut e = entity.clone();
                    e.base_mut().id = 0;
                    updates.push(FeatureChange::Update {
                        id: uid.to_text(),
                        entity: e,
                    });
                    changed += 1;
                }
            }
        }
    }
    let mut features = creates;
    features.extend(updates);
    let mut removed = 0u64;
    for uid in &before.uids {
        if !kept.contains(uid) {
            features.push(FeatureChange::Delete { id: uid.to_text() });
            removed += 1;
        }
    }
    if features.len() + blocks.len() > JOB_CHANGES_MAX {
        return Err(ProcessFailure::new(
            "too_many_changes",
            format!(
                "Sonuç {} nesne ve blok değişikliği; bir iş en çok {JOB_CHANGES_MAX} yazar. Girdiyi daraltın ya da dosyayı parçalara bölün.",
                features.len() + blocks.len()
            ),
        ));
    }
    let mut patch = ProjectPatch::default();
    let mut metadata = false;
    if before.name != after.name {
        patch.name = Some(after.name.clone());
        metadata = true;
    }
    if !same(&before.settings, &after.settings) {
        patch.settings = Some(after.settings.clone());
        metadata = true;
    }
    if !same(&before.layers, &after.layers) {
        patch.layers = Some(after.layers.clone());
        metadata = true;
    }
    if before.active_layer != after.active_layer {
        patch.active_layer = Some(after.active_layer.clone());
        metadata = true;
    }
    if !same(&before.styles, &after.styles) {
        patch.styles = Some(after.styles.clone());
        metadata = true;
    }
    if !same(&before.sheets, &after.sheets) {
        patch.sheets = Some(after.sheets.clone());
        metadata = true;
    }
    if !same(&before.arrays, &after.arrays) {
        patch.arrays = Some(after.arrays.clone());
        metadata = true;
    }
    if !same(&before.constraints, &after.constraints) {
        patch.constraints = Some(after.constraints.clone());
        metadata = true;
    }
    Ok(Delta {
        features,
        blocks,
        project: metadata.then_some(patch),
        added,
        changed,
        removed,
    })
}
