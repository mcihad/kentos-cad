//! What an İşlemler tool run as a server job decides without a database, and what its process answers (docs/adr/0559;
//! plan/gis.md CBS-39): shared by the server, which asks for the run and commits its result, and by the process that
//! runs the tool (`kentos-jobrun`), which must not carry the server.
//!
//! - **The request** ([`check_input`]): a `project.processing` job's input is checked when it is asked for (a tool's
//!   id, values with no local file in them, the objects and the box as the command takes them, the revision and a
//!   budget no bigger than the server's), and again by the process, whatever the server did.
//! - **What a tool did** ([`Processed`], [`ProcessFailure`]): the answer of the process, a [`Delta`] to commit or a
//!   GeoPackage, or why it did not finish.
//! - **The delta** ([`delta`]): what the tool changed, found by comparing the drawing as it was opened with the
//!   drawing after the run, both through the same document code, so an object the tool did not touch is never sent
//!   back. It is one `project.changes` command's input; the commit is the job's finish.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    DocumentSnapshotV2, Entity, EntityId, FeatureChange, ProcessingJobInput, ProcessingJobNote,
    ProcessingUsage, ProjectChanges, ProjectPatch,
};

/// The most object changes one `project.changes` command carries (the server's limit).
pub const PROJECT_CHANGES_MAX: usize = 5000;
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

// ── What the process answers ────────────────────────────────────────────────

/// What a tool did.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Processed {
    /// The tool's own sentence.
    pub summary: String,
    pub notes: Vec<ProcessingJobNote>,
    /// `commit`: what to write to the project.
    pub delta: Option<Delta>,
    /// `sink`: the GeoPackage the objects it made were written to (a file beside the answer, not in it).
    #[serde(skip)]
    pub sink: Option<Vec<u8>>,
    /// `sink`: how many rows the file holds.
    pub sink_rows: u64,
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
        self.features.is_empty() && self.project.is_none()
    }

    /// The command's input.
    pub fn changes(&self) -> ProjectChanges {
        ProjectChanges {
            features: self.features.clone(),
            project: self.project.clone(),
            blocks: Vec::new(),
        }
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

fn same(a: &impl Serialize, b: &impl Serialize) -> bool {
    match (serde_json::to_value(a), serde_json::to_value(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// What the tool changed: `after` against `before`, both as the same document code writes them. An object is created
/// when its persistent id is new, changed when it differs from itself before (its slot number is not content), and
/// removed when it is gone; the metadata is what differs whole. Blocks are not carried (`unsupported_change`), and a
/// command takes at most [`PROJECT_CHANGES_MAX`] objects (`too_many_changes`).
pub fn delta(
    before: &DocumentSnapshotV2,
    after: &DocumentSnapshotV2,
) -> Result<Delta, DeltaRefusal> {
    if !same(&before.blocks, &after.blocks) {
        return Err(ProcessFailure::new(
            "unsupported_change",
            "Aracın sonucu blok tanımlarını değiştiriyor; sunucudaki iş bu sürümde blok tanımı yazmaz. Sonucu dosyaya (sink) yazdırın ya da aracı bilgisayarınızda çalıştırın.",
        ));
    }
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
    if features.len() > PROJECT_CHANGES_MAX {
        return Err(ProcessFailure::new(
            "too_many_changes",
            format!(
                "Aracın sonucu {} nesne değişikliği; bir komut en çok {PROJECT_CHANGES_MAX} yazar. Girdiyi daraltın ya da sonucu dosyaya (sink) yazdırın.",
                features.len()
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
        project: metadata.then_some(patch),
        added,
        changed,
        removed,
    })
}
