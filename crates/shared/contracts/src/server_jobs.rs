//! Durable server jobs (docs/adr/0542; plan/gis.md CBS-39): a job a user asks the server to run for a project, its
//! state, progress, attempts and staged artifact. The input is kept as given with its SHA-256 and the project's data
//! revision when asked; a worker claims it with a lease and a fence, and finishes it as the user who asked, with the
//! access asked again. The kinds are the server's own: `project.export`, the project as one `.kcad` file, and
//! `project.checkpoint`, a checkpoint made as a job (docs/adr/0547), and `project.processing`, an İşlemler tool run on
//! the server, against the project at a pinned revision (docs/adr/0559), `project.import`, a staged file taken into the
//! project by the worker, and `project.atlas`, the atlas of a sheet as one PDF (docs/adr/0561).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The kinds of server jobs.
pub const SERVER_JOB_KINDS: &[&str] = &[
    PROJECT_EXPORT_JOB,
    PROJECT_CHECKPOINT_JOB,
    PROJECT_PROCESSING_JOB,
    PROJECT_IMPORT_JOB,
    PROJECT_ATLAS_JOB,
];
/// A staged file taken into a database project by the worker (docs/adr/0561 §1): the input is a [`ProjectImportInput`],
/// the result a [`ProjectImportResult`], committed in the job's finish like a processing job's.
pub const PROJECT_IMPORT_JOB: &str = "project.import";
/// The atlas of a sheet as one PDF (docs/adr/0561 §2): the input is a [`ProjectAtlasInput`], the result a
/// [`ProjectAtlasResult`], the artifact the PDF.
pub const PROJECT_ATLAS_JOB: &str = "project.atlas";
/// An İşlemler tool run as a job on a database project (docs/adr/0559): the input is a [`ProcessingJobInput`], the
/// result a [`ProcessingJobResult`]; with `mode: sink` the artifact is a GeoPackage.
pub const PROJECT_PROCESSING_JOB: &str = "project.processing";
/// A checkpoint made as a job (docs/adr/0547): the input is a [`CheckpointCreate`](crate::CheckpointCreate), the
/// result the [`CheckpointChange`](crate::CheckpointChange) the command answered, no artifact.
pub const PROJECT_CHECKPOINT_JOB: &str = "project.checkpoint";
/// The project (a database project) as one KCAD v2 file, staged in the object store.
pub const PROJECT_EXPORT_JOB: &str = "project.export";
/// The most jobs one listing gives.
pub const SERVER_JOB_LIST_MAX: usize = 100;
/// How many jobs one organisation may have queued and running together, unless the server says otherwise
/// (`KENTOS_JOB_LIMIT`), and the biggest artifact a job may stage, in bytes (`KENTOS_JOB_ARTIFACT_MAX_MB`).
pub const SERVER_JOB_TENANT_ACTIVE_DEFAULT: u32 = 50;
pub const SERVER_JOB_ARTIFACT_MAX_DEFAULT: u64 = 2 * 1024 * 1024 * 1024;
/// How many days a finished job (and its artifact) is kept unless the server says otherwise
/// (`KENTOS_JOB_RETENTION_DAYS`).
pub const SERVER_JOB_RETENTION_DAYS_DEFAULT: u64 = 14;
/// What one `project.processing` job may take unless the server says otherwise (`KENTOS_JOB_CPU_SECONDS`,
/// `KENTOS_JOB_MEMORY_MB`, docs/adr/0559 §4): CPU seconds of the process the tool runs in, and its resident memory.
pub const SERVER_JOB_CPU_DEFAULT_SECONDS: u32 = 1_800;
pub const SERVER_JOB_MEMORY_DEFAULT_MB: u32 = 4_096;
/// The biggest file a job input may be, in bytes unless the server says otherwise (`KENTOS_JOB_INPUT_MAX_MB`).
pub const SERVER_JOB_INPUT_MAX_DEFAULT: u64 = 2 * 1024 * 1024 * 1024;
/// The longest time a job may run, in seconds, and its default.
pub const SERVER_JOB_TIMEOUT_MAX: u32 = 86_400;
pub const SERVER_JOB_TIMEOUT_DEFAULT: u32 = 600;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ServerJobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl ServerJobState {
    pub fn of(text: &str) -> Option<Self> {
        Some(match text {
            "queued" => Self::Queued,
            "running" => Self::Running,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => return None,
        })
    }

    /// Whether it will not change again.
    pub fn is_over(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

/// What a user asks for (`POST …/projects/{project}/jobs`). A request with a request id the same user sent before for
/// the project gets that job, whatever this one says.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ServerJobCreate {
    pub kind: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub input: Value,
    /// 1–200 characters: the client's own id of this request.
    pub request_id: String,
    /// Seconds, 1–86 400; absent: 600.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub timeout_seconds: Option<u32>,
}

/// `project.export`'s input: the export refused (`revision_conflict`) when the project is no longer at this data
/// revision when it runs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectExportInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// What a `project.processing` job does with the tool's result (docs/adr/0559 §2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProcessingJobMode {
    /// The objects and layers the tool made, changed or removed are committed to the project, one command, when the
    /// project is still at the revision the tool read.
    #[default]
    Commit,
    /// The objects the tool made are written to a GeoPackage, the job's artifact; the project does not change.
    Sink,
}

/// `project.processing`'s input (docs/adr/0559 §1): the same run as `cad.processing.run`, by the tool's id and its
/// values as the command takes them, with no local file (a table is given as rows) and no sink path (the server
/// stages the file). The tool reads the project as it is at the revision the job was asked at, or at
/// `expectedRevision`: a job refused (`revision_conflict`) when the project is not at it when the job runs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProcessingJobInput {
    /// The tool's id (`cad.processing.tools`).
    pub tool: String,
    /// The tool's name as the window shows it, for lists; only said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// The tool's values by parameter name; one left out takes the project's default.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>", optional))]
    pub parameters: BTreeMap<String, Value>,
    /// The objects taken as chosen (persistent ids): a features input's `selection` scope.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub selection: Vec<String>,
    /// The box taken as on screen (minimum east, north, maximum east, north): a features input's `visible` scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub view: Option<[f64; 4]>,
    #[serde(default)]
    pub mode: ProcessingJobMode,
    /// The data revision the tool must read; absent: the one the job was asked at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
    /// A smaller CPU budget than the server's, in seconds; never a bigger one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cpu_seconds: Option<u32>,
    /// A smaller memory budget than the server's, in megabytes; never a bigger one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub memory_mb: Option<u32>,
}

/// What a user says of a file to stage for a job (`POST …/projects/{project}/job-inputs`, docs/adr/0561 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JobInputBegin {
    /// The file's name as the user has it (1–200 characters, no folder): `parcels.gpkg`.
    pub name: String,
    /// Its size in bytes, as the bytes will come.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub size: u64,
    /// Its SHA-256: 64 lowercase hexadecimal digits.
    pub sha256: String,
}

/// A file staged for a job: asked for, then sent in one streamed body (`PUT …/job-inputs/{id}`), then named by a
/// `project.import` job's `upload`. It is kept as long as finished jobs are (`KENTOS_JOB_RETENTION_DAYS`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JobInput {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub size: u64,
    pub sha256: String,
    pub created_at: String,
    /// Whether its bytes arrived and are the declared ones.
    pub received: bool,
}

/// The formats a `project.import` job takes (docs/adr/0561 §1): the ones `file.import.*` commands read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ImportFormat {
    Gpkg,
    Fgb,
    Gml,
    Csv,
    Dxf,
    /// A Shapefile as a zip archive: its `.shp`, `.shx`, `.dbf`, `.prj` and `.cpg` staged together (docs/adr/0563 §3).
    Shp,
    Geojson,
    /// A KML file or a KMZ archive.
    Kml,
}

impl ImportFormat {
    pub const ALL: [ImportFormat; 8] = [
        Self::Gpkg,
        Self::Fgb,
        Self::Gml,
        Self::Csv,
        Self::Dxf,
        Self::Shp,
        Self::Geojson,
        Self::Kml,
    ];

    /// The command that reads the file.
    pub fn command(self) -> &'static str {
        match self {
            Self::Gpkg => "file.import.gpkg",
            Self::Fgb => "file.import.fgb",
            Self::Gml => "file.import.gml",
            Self::Csv => "file.import.csv",
            Self::Dxf => "file.import.dxf",
            Self::Shp => "file.import.shp",
            Self::Geojson => "file.import.geojson",
            Self::Kml => "file.import.kml",
        }
    }

    /// The file name's extension the command reads it by.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Gpkg => "gpkg",
            Self::Fgb => "fgb",
            Self::Gml => "gml",
            Self::Csv => "csv",
            Self::Dxf => "dxf",
            Self::Shp => "zip",
            Self::Geojson => "geojson",
            Self::Kml => "kml",
        }
    }

    /// The format of a file name by its extension, case aside.
    pub fn of_name(name: &str) -> Option<Self> {
        let ext = name.rsplit_once('.')?.1.to_lowercase();
        match ext.as_str() {
            "gpkg" => Some(Self::Gpkg),
            "fgb" => Some(Self::Fgb),
            "gml" | "xml" => Some(Self::Gml),
            "csv" | "txt" => Some(Self::Csv),
            "dxf" => Some(Self::Dxf),
            "zip" => Some(Self::Shp),
            "geojson" | "json" => Some(Self::Geojson),
            "kml" | "kmz" => Some(Self::Kml),
            _ => None,
        }
    }
}

/// `project.import`'s input (docs/adr/0561 §1): a file staged for the project (`upload`, its id) read by the `format`'s
/// import command with `options` (that command's input without its `path`), and what was read committed to the project
/// when it is still at the revision the worker read. A file's local path is never an option: the server gives the file
/// it staged.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectImportInput {
    /// The staged file's id (`JobInput.id`).
    pub upload: String,
    /// The format; absent: the staged file's name tells (`ImportFormat::of_name`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub format: Option<ImportFormat>,
    /// The import command's input without `path`: `tables`, `layers`, `reproject`, `srid`, `layer`, `geometry`,
    /// `fileUnit`, `explodeBlocks`, `group` … as the command takes them.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>", optional))]
    pub options: BTreeMap<String, Value>,
    /// For lists; only said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// The data revision the worker must read; absent: the one the job was asked at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cpu_seconds: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub memory_mb: Option<u32>,
}

/// `project.atlas`'s input (docs/adr/0561 §2): `file.export.atlas`'s input without its `path`, run on the project at the
/// revision the worker read (or `expectedRevision`), the pages made from the state `pin` names: a pin that is not the
/// project's state refuses the job (`pin_mismatch`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectAtlasInput {
    /// The sheet that has the atlas, by id.
    pub sheet: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fields: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub crs: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pin: Option<crate::AtlasPin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cpu_seconds: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub memory_mb: Option<u32>,
}

/// A succeeded `project.import` job's result (docs/adr/0561 §1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectImportResult {
    /// The import command that read the file.
    pub command: String,
    /// The file's name, bytes and SHA-256 (as the command read them).
    pub file: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub bytes: u64,
    pub sha256: String,
    /// The data revision the worker read, and the one the commit made, as text.
    pub base_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub revision: Option<String>,
    /// The command's own words: what came in.
    pub summary: String,
    /// How many objects were created, changed and removed; whether the project's layer tree (or other metadata) changed
    /// with them, and how many block definitions the file brought.
    pub added: u64,
    pub changed: u64,
    pub removed: u64,
    #[serde(default)]
    pub metadata: bool,
    #[serde(default)]
    pub blocks: u64,
    /// What the file could not give or the import left out: the command's warnings (`import_losses`, `crs_undeclared`,
    /// `unit_assumed` …) and the notes of the run, in words, with the warning's code in front of its words.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProcessingJobNote>>", optional))]
    pub notes: Vec<ProcessingJobNote>,
    /// What the import command answered (the tables or layers read, the counts, the coordinate system).
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub details: Value,
    pub usage: ProcessingUsage,
}

/// A succeeded `project.atlas` job's result (docs/adr/0561 §2): the artifact is the PDF.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectAtlasResult {
    pub sheet: String,
    /// The data revision the worker read, as text.
    pub base_revision: String,
    /// How many pages the PDF has.
    pub pages: u64,
    /// The first page's name and the last's, for lists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub first: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub last: Option<String>,
    /// The state the pages were made from (`AtlasPin`'s digest) and what the run was bound to.
    pub pin_digest: String,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub bindings: Value,
    /// The attributes the pages carry and the ones they do not (a form's `redact`, or not asked for).
    pub fields: Vec<String>,
    pub withheld: Vec<String>,
    /// The sheet core's findings and the notes of a host without a window, in words.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProcessingJobNote>>", optional))]
    pub notes: Vec<ProcessingJobNote>,
    pub usage: ProcessingUsage,
}

/// What a `project.processing` job took (docs/adr/0559 §4).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProcessingUsage {
    /// CPU seconds of the process the tool ran in, as last measured.
    pub cpu_seconds: f64,
    /// The most resident memory of that process, as measured, in megabytes.
    pub peak_memory_mb: f64,
}

/// A note the tool made while it ran.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProcessingJobNote {
    /// `info` or `warn`.
    pub level: String,
    pub text: String,
}

/// A succeeded `project.processing` job's result (docs/adr/0559 §2).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProcessingJobResult {
    pub tool: String,
    pub mode: ProcessingJobMode,
    /// The data revision the tool read, as text.
    pub base_revision: String,
    /// `commit`: the data revision the commit made, as text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub revision: Option<String>,
    /// The tool's own summary line.
    pub summary: String,
    /// `commit`: how many objects were created, changed and removed; whether the layer tree (or another part of the
    /// project's metadata) changed with them. `sink`: how many objects the file holds, in `added`.
    pub added: u64,
    pub changed: u64,
    pub removed: u64,
    #[serde(default)]
    pub metadata: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProcessingJobNote>>", optional))]
    pub notes: Vec<ProcessingJobNote>,
    pub usage: ProcessingUsage,
}

/// What the estimate takes for the process itself and the tool's buffers, in bytes.
pub const PROCESSING_MEMORY_BASE: u64 = 192 * 1024 * 1024;
/// How many times the file's size the drawing and its working copy take.
pub const PROCESSING_MEMORY_FACTOR: u64 = 24;

/// How much memory a `project.processing` job is expected to need for a project whose `.kcad` file is `snapshot_bytes`
/// long, in bytes (docs/adr/0559 §4): the drawing is read whole and a copy is run on, so many times its file size and
/// a base for the tool's own buffers. A job whose estimate is past its budget is refused before it runs.
pub fn processing_memory_estimate(snapshot_bytes: u64) -> u64 {
    PROCESSING_MEMORY_BASE.saturating_add(snapshot_bytes.saturating_mul(PROCESSING_MEMORY_FACTOR))
}

/// How many times the staged file's size an import takes in memory beyond the project's (docs/adr/0561 §1): the file is
/// read whole and its objects are built next to it.
pub const IMPORT_MEMORY_FACTOR: u64 = 32;

/// How much memory a `project.import` job is expected to need for a project whose `.kcad` file is `snapshot_bytes` long
/// and a staged file of `file_bytes`, in bytes: a processing job's estimate and the file's share.
pub fn import_memory_estimate(snapshot_bytes: u64, file_bytes: u64) -> u64 {
    processing_memory_estimate(snapshot_bytes)
        .saturating_add(file_bytes.saturating_mul(IMPORT_MEMORY_FACTOR))
}

/// A succeeded job's artifact in the object store.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ServerJobArtifact {
    /// Bytes, as text.
    pub size: String,
    pub sha256: String,
    /// The file name a download gives it.
    pub name: String,
}

/// Why a job failed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ServerJobError {
    /// `timeout`, `lease_lost`, `forbidden`, `revision_conflict`, `gone`, `invalid`, `storage` (the artifact could
    /// not be written: no space, no folder), `too_large` (the artifact is past the server's limit) or `failed`; a
    /// `project.processing` job also `cpu_limit`, `memory_limit` (stopped at its budget), `memory_estimate` (refused
    /// before it ran), `tool_failed`, `tool_unavailable`, `unsupported_change`, `too_many_changes` and `crashed`
    /// (docs/adr/0559); a `project.import` or `project.atlas` job also the refusal code of the command the worker ran
    /// (`import_unreadable`, `crs_mismatch`, `nothing_to_import`, `pin_mismatch`, `atlas_empty` …, docs/adr/0561).
    pub code: String,
    pub message: String,
}

/// A job as its project's users see it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ServerJob {
    pub id: String,
    pub project: String,
    pub kind: String,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub input: Value,
    pub input_sha256: String,
    /// The project's data revision when the job was asked for, as text.
    pub base_revision: String,
    pub state: ServerJobState,
    /// 0–1.
    pub progress: f32,
    /// What the worker says it is doing, a few Turkish words, while the job runs (docs/adr/0559 §3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub progress_text: Option<String>,
    pub attempts: u32,
    pub max_attempts: u32,
    pub timeout_seconds: u32,
    pub cancel_requested: bool,
    pub created_by: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub finished_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub artifact: Option<ServerJobArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub error: Option<ServerJobError>,
}

/// A project's jobs, newest first, at most [`SERVER_JOB_LIST_MAX`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ServerJobList {
    pub jobs: Vec<ServerJob>,
}
