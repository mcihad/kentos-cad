//! The local copy of a cloud project on this device (docs/adr/0043): what
//! the server had when this device last saw it, so the project opens, and
//! is worked on, without a connection. The owner's rule: the desktop works
//! offline and keeps every project in step with the server by itself.
//!
//! One folder per server, account and project, locked while a program holds
//! it (a second KentOS gets [`ReplicaError::InUse`]):
//!
//! - `bilgi.json`: the project as it was last listed (name, workspace, role,
//!   state, storage), replaced whole; the offline catalog reads only these.
//! - `taban-<n>.kcad` and `taban-<n>.json`: the server's drawing at one
//!   moment (KCAD v2, every object under its persistent id) with each
//!   object's version, the metadata version, the event cursor and, for a
//!   file project, the revision it is.
//! - `taban-<n>.log` (database projects): what came to be known after that
//!   moment, one [`BaseStep`] per line, each flushed to the disk as it is
//!   added (`ProjectSync::take_base_step`). A line cut short by a crash is
//!   the last one and is dropped; any other unreadable line makes the copy
//!   unreadable, and the project is opened from the server again.
//! - `guncel`: which `n` is current. A compaction writes generation `n + 1`
//!   whole, flushes it, then replaces `guncel`; a crash leaves the old
//!   generation or the new one, never a mix.
//! - `bekleyen.kcad` and `bekleyen.json` (file projects): a save made
//!   without a connection, with the revision it is based on, waiting to be
//!   sent.
//!
//! The copy holds drawing data only: no session, password or address beyond
//! the server's name in its folder (TODOS.md SYNC-13). The device draft of
//! unsent work stays apart (drafts.rs). Everything here reads and writes
//! files: call it off the interface thread for large projects.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kentos_contracts::{
    DOCUMENT_FORMAT, DOCUMENT_VERSION_2, DocumentSnapshotV2, EntityId, ProjectId, ProjectInfo,
    ProjectStorage,
};
use kentos_domain::Document;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::open::{Opened, Revision, Source};
use crate::sync::{BaseSnapshot, BaseStep, SaveState, objects_by_id};

/// What the copy's files say they are.
const FORMAT: &str = "kentos.cloud-replica";
const VERSION: u32 = 1;

/// Why a copy cannot be used.
#[derive(Debug)]
pub enum ReplicaError {
    /// Another KentOS on this device has the project open.
    InUse,
    /// It is damaged or of another format (the reason, in Turkish): open the project from the server.
    Unreadable(String),
    Io(io::Error),
}

impl std::fmt::Display for ReplicaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InUse => f.write_str(
                "Bu proje bu bilgisayarda başka bir KentOS penceresinde açık; önce orada kapatın.",
            ),
            Self::Unreadable(why) => write!(
                f,
                "Projenin bu cihazdaki kopyası okunamadı ({why}); proje sunucudan yeniden açılmalı."
            ),
            Self::Io(e) => write!(f, "Projenin bu cihazdaki kopyasına erişilemedi: {e}"),
        }
    }
}

impl std::error::Error for ReplicaError {}

impl From<io::Error> for ReplicaError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// The server's drawing at one moment, besides its KCAD file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Head {
    format: String,
    version: u32,
    storage: ProjectStorage,
    /// Each object's version (database projects).
    versions: BTreeMap<String, String>,
    meta_version: String,
    cursor: String,
    /// The revision it is (file projects).
    revision: Option<Revision>,
}

/// The project as it was last listed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Listed {
    format: String,
    version: u32,
    info: ProjectInfo,
    /// When it was written, in milliseconds since 1970.
    saved_at: u64,
    /// The server ended the project for this account since it was listed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ended: Option<Ended>,
}

/// Why the server no longer takes this account's work on a project.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Ended {
    /// Moved to the trash.
    Deleted,
    /// This account's access was taken away.
    Revoked,
    /// Archived: read-only until it is unarchived.
    Archived,
}

impl Ended {
    /// The ending a sync reached, if it reached one.
    pub fn of(state: SaveState) -> Option<Self> {
        match state {
            SaveState::Deleted => Some(Self::Deleted),
            SaveState::Revoked => Some(Self::Revoked),
            SaveState::Archived => Some(Self::Archived),
            _ => None,
        }
    }
}

/// A copy in the offline catalog.
#[derive(Clone, Debug, PartialEq)]
pub struct Kept {
    pub info: ProjectInfo,
    pub saved_at: u64,
    /// The server ended the project for this account: it opens read-only here.
    pub ended: Option<Ended>,
}

#[derive(Serialize, Deserialize)]
struct Current {
    generation: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pending {
    based_on: u64,
    saved_at: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// A name part safe in a file name (as drafts.rs makes them).
fn part(text: &str) -> String {
    let safe: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() || safe.chars().all(|c| c == '.') {
        "_".to_owned()
    } else {
        safe
    }
}

/// Writes `bytes` to `path` durably: a temporary file, flushed, renamed over it, the folder flushed.
fn write_durably(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let folder = path
        .parent()
        .ok_or_else(|| io::Error::other("kopya klasörü yok"))?;
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    let written = (|| {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, path)?;
        File::open(folder)?.sync_all()
    })();
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
}

fn unreadable(what: &str, e: impl std::fmt::Display) -> ReplicaError {
    ReplicaError::Unreadable(format!("{what}: {e}"))
}

/// Where this device keeps its copies of cloud projects.
#[derive(Clone, Debug)]
pub struct ReplicaStore {
    root: PathBuf,
}

impl ReplicaStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn account(&self, server: &str, user: &str) -> PathBuf {
        let host = server
            .split_once("://")
            .map_or(server, |(_, rest)| rest)
            .trim_end_matches('/');
        self.root.join(part(host)).join(part(user))
    }

    /// The copy of one account's project on one server (its address as
    /// `Cloud::server` gives it), locked for this program while it is held.
    /// It may be empty: nothing was kept yet.
    pub fn open(
        &self,
        server: &str,
        user: &str,
        tenant: Uuid,
        project: Uuid,
    ) -> Result<Replica, ReplicaError> {
        let dir = self
            .account(server, user)
            .join(format!("{tenant}_{project}"));
        fs::create_dir_all(&dir)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join("kilit"))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => return Err(ReplicaError::InUse),
            Err(std::fs::TryLockError::Error(e)) => return Err(ReplicaError::Io(e)),
        }
        Ok(Replica {
            dir,
            _lock: lock,
            tenant,
            project,
        })
    }

    /// Removes a project's copy from this device (to free the disk); refused
    /// while a program holds it. The device draft of unsent work is apart
    /// (drafts.rs): remove it only when nothing in it is still wanted.
    pub fn remove(
        &self,
        server: &str,
        user: &str,
        tenant: Uuid,
        project: Uuid,
    ) -> Result<(), ReplicaError> {
        let held = self.open(server, user, tenant, project)?;
        let dir = held.dir.clone();
        drop(held);
        match fs::remove_dir_all(&dir) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }

    /// The projects this account keeps on this device from this server, as
    /// they were last listed (what opens without a connection).
    pub fn list(&self, server: &str, user: &str) -> Vec<Kept> {
        let Ok(entries) = fs::read_dir(self.account(server, user)) else {
            return Vec::new();
        };
        let mut kept: Vec<Kept> = entries
            .filter_map(Result::ok)
            .filter_map(|e| fs::read(e.path().join("bilgi.json")).ok())
            .filter_map(|bytes| serde_json::from_slice::<Listed>(&bytes).ok())
            .filter(|l| l.format == FORMAT && l.version == VERSION)
            .map(|l| Kept {
                info: l.info,
                saved_at: l.saved_at,
                ended: l.ended,
            })
            .collect();
        kept.sort_by_key(|k| std::cmp::Reverse(k.saved_at));
        kept
    }
}

/// One project's copy, held by this program.
#[derive(Debug)]
pub struct Replica {
    dir: PathBuf,
    /// Held while the copy is in use; released when it is dropped.
    _lock: File,
    tenant: Uuid,
    project: Uuid,
}

impl Replica {
    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    fn generation(&self) -> Result<Option<u64>, ReplicaError> {
        match fs::read(self.path("guncel")) {
            Ok(bytes) => serde_json::from_slice::<Current>(&bytes)
                .map(|c| Some(c.generation))
                .map_err(|e| unreadable("guncel", e)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn files(&self, generation: u64) -> [PathBuf; 3] {
        ["kcad", "json", "log"].map(|ext| self.path(&format!("taban-{generation}.{ext}")))
    }

    /// Replaces the project's listing (`bilgi.json`): name, workspace, role, state.
    pub fn list_as(&self, info: &ProjectInfo) -> io::Result<()> {
        let listed = Listed {
            format: FORMAT.into(),
            version: VERSION,
            info: info.clone(),
            saved_at: now_ms(),
            ended: None,
        };
        write_durably(
            &self.path("bilgi.json"),
            &serde_json::to_vec(&listed).map_err(io::Error::other)?,
        )
    }

    /// Records that the server ended the project for this account (a sync's
    /// `Deleted`, `Revoked` or `Archived`): the offline catalog says so, and
    /// an opening from the copy is read-only. The next listing from the
    /// server (`list_as`, `reset`) clears it.
    pub fn mark_ended(&self, ended: Ended) -> Result<(), ReplicaError> {
        let mut listed: Listed = serde_json::from_slice(&fs::read(self.path("bilgi.json"))?)
            .map_err(|e| unreadable("bilgi.json", e))?;
        listed.ended = Some(ended);
        write_durably(
            &self.path("bilgi.json"),
            &serde_json::to_vec(&listed).map_err(io::Error::other)?,
        )?;
        Ok(())
    }

    /// Starts the copy again from a project just opened from the server:
    /// call it before a device draft goes back in, so the copy is the
    /// server's drawing, not this device's unsent work.
    pub fn reset(&mut self, opened: &Opened) -> Result<(), ReplicaError> {
        let (versions, revision) = match &opened.source {
            Source::Database { versions } => (versions.clone(), None),
            Source::File { revision } => (Vec::new(), revision.clone()),
        };
        self.list_as(&opened.info)?;
        self.write_generation(
            &opened.document.to_snapshot_v2(),
            Head {
                format: FORMAT.into(),
                version: VERSION,
                storage: opened.info.storage,
                versions: versions
                    .into_iter()
                    .map(|(id, v)| (id.to_string(), v))
                    .collect(),
                meta_version: opened.info.meta_version.clone(),
                cursor: opened.info.event_cursor.clone(),
                revision,
            },
        )
    }

    /// Rewrites the copy from the sync's whole base (`ProjectSync::base`),
    /// dropping the log: when it grows long, and when the project is closed.
    pub fn compact(&mut self, base: &BaseSnapshot) -> Result<(), ReplicaError> {
        self.write_generation(
            &base.snapshot,
            Head {
                format: FORMAT.into(),
                version: VERSION,
                storage: ProjectStorage::Database,
                versions: base
                    .versions
                    .iter()
                    .map(|(id, v)| (id.to_string(), v.clone()))
                    .collect(),
                meta_version: base.meta_version.clone(),
                cursor: base.cursor.clone(),
                revision: None,
            },
        )
    }

    fn write_generation(
        &mut self,
        snapshot: &DocumentSnapshotV2,
        head: Head,
    ) -> Result<(), ReplicaError> {
        let old = self.generation()?;
        let next = old.map_or(1, |g| g + 1);
        let [kcad, json, log] = self.files(next);
        let bytes = kentos_kcad::encode_verified(snapshot)
            .map_err(|e| ReplicaError::Io(io::Error::other(e.to_string())))?;
        write_durably(&kcad, &bytes)?;
        write_durably(&json, &serde_json::to_vec(&head).map_err(io::Error::other)?)?;
        write_durably(&log, b"")?;
        write_durably(
            &self.path("guncel"),
            &serde_json::to_vec(&Current { generation: next }).map_err(io::Error::other)?,
        )?;
        if let Some(g) = old {
            for f in self.files(g) {
                let _ = fs::remove_file(f);
            }
        }
        Ok(())
    }

    /// Adds what came to be known of the server (`ProjectSync::take_base_step`),
    /// flushed to the disk before it returns.
    pub fn append(&mut self, step: &BaseStep) -> Result<(), ReplicaError> {
        let generation = self
            .generation()?
            .ok_or_else(|| ReplicaError::Unreadable("kopyanın tabanı yok".into()))?;
        let [_, _, log] = self.files(generation);
        let mut line = serde_json::to_vec(step).map_err(io::Error::other)?;
        line.push(b'\n');
        let mut file = OpenOptions::new().append(true).open(log)?;
        file.write_all(&line)?;
        file.sync_data()?;
        Ok(())
    }

    /// How many steps the log holds (compact when it grows long).
    pub fn steps(&self) -> Result<usize, ReplicaError> {
        let Some(generation) = self.generation()? else {
            return Ok(0);
        };
        let [_, _, log] = self.files(generation);
        match File::open(log) {
            Ok(f) => Ok(BufReader::new(f).lines().count()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
            Err(e) => Err(e.into()),
        }
    }

    /// The project as this device last knew the server had it, opened
    /// without a connection; `None` when nothing was kept yet.
    pub fn load(&self) -> Result<Option<Opened>, ReplicaError> {
        let Some(generation) = self.generation()? else {
            return Ok(None);
        };
        let listed: Listed = serde_json::from_slice(&fs::read(self.path("bilgi.json"))?)
            .map_err(|e| unreadable("bilgi.json", e))?;
        let [kcad, json, log] = self.files(generation);
        let mut head: Head =
            serde_json::from_slice(&fs::read(&json)?).map_err(|e| unreadable("taban", e))?;
        if head.format != FORMAT || head.version != VERSION {
            return Err(ReplicaError::Unreadable(format!(
                "biçim {} {}, bu sürüm {FORMAT} {VERSION} okur",
                head.format, head.version
            )));
        }
        let mut snapshot =
            kentos_kcad::decode(&fs::read(&kcad)?).map_err(|e| unreadable("taban çizimi", e))?;
        let mut info = listed.info;
        if listed.ended.is_some() {
            // Nothing more goes to the server from here: the opening is read-only.
            info.access
                .permissions
                .retain(|p| *p == kentos_contracts::ProjectPermission::Read);
        }
        let source = match head.storage {
            ProjectStorage::File => Source::File {
                revision: head.revision.clone(),
            },
            ProjectStorage::Database => {
                let versions: Vec<(Uuid, String)> = head
                    .versions
                    .iter()
                    .filter_map(|(id, v)| Uuid::parse_str(id).ok().map(|id| (id, v.clone())))
                    .collect();
                let mut objects = objects_by_id(&snapshot, &versions);
                let steps = read_steps(&log)?;
                for step in steps {
                    if let Some(c) = step.cursor {
                        head.cursor = c;
                    }
                    for o in step.put {
                        if let Ok(id) = Uuid::parse_str(&o.id) {
                            objects.insert(id, (o.version, o.entity));
                        }
                    }
                    for id in step.remove {
                        if let Ok(id) = Uuid::parse_str(&id) {
                            objects.remove(&id);
                        }
                    }
                    if let Some(m) = step.meta {
                        head.meta_version = m.version;
                        snapshot.name = m.name;
                        snapshot.settings = m.settings;
                        snapshot.layers = m.layers;
                        snapshot.styles = m.styles;
                    }
                }
                let mut versions = Vec::with_capacity(objects.len());
                snapshot.entities.clear();
                snapshot.uids.clear();
                for (i, (id, (version, mut entity))) in objects.into_iter().enumerate() {
                    entity.base_mut().id = u32::try_from(i + 1).unwrap_or(u32::MAX);
                    snapshot.entities.push(entity);
                    snapshot.uids.push(EntityId(id.into_bytes()));
                    versions.push((id, version));
                }
                Source::Database { versions }
            }
        };
        snapshot.format = DOCUMENT_FORMAT.to_owned();
        snapshot.version = DOCUMENT_VERSION_2;
        // A database project's drawing is the project's metadata and objects, as an opening
        // makes it; a file project's is its revision, as the file says.
        if head.storage == ProjectStorage::Database {
            snapshot.project_id = Some(ProjectId(self.project.into_bytes()));
            info.name.clone_from(&snapshot.name);
            info.settings = snapshot.settings.clone();
            info.layers = snapshot.layers.clone();
            info.styles = snapshot.styles.clone();
            info.meta_version = head.meta_version;
        }
        info.event_cursor = head.cursor;
        let document =
            Document::from_snapshot_v2(snapshot).map_err(|e| unreadable("taban çizimi", e))?;
        Ok(Some(Opened {
            tenant: self.tenant,
            project: self.project,
            info,
            document,
            source,
        }))
    }

    /// Keeps a file project's save made without a connection, based on revision `based_on`.
    pub fn keep_save(&mut self, bytes: &[u8], based_on: u64) -> Result<(), ReplicaError> {
        write_durably(&self.path("bekleyen.kcad"), bytes)?;
        let facts = Pending {
            based_on,
            saved_at: now_ms(),
        };
        write_durably(
            &self.path("bekleyen.json"),
            &serde_json::to_vec(&facts).map_err(io::Error::other)?,
        )?;
        Ok(())
    }

    /// The save waiting to be sent, with the revision it is based on.
    pub fn kept_save(&self) -> Result<Option<(Vec<u8>, u64)>, ReplicaError> {
        let facts = match fs::read(self.path("bekleyen.json")) {
            Ok(b) => serde_json::from_slice::<Pending>(&b)
                .map_err(|e| unreadable("bekleyen kayıt", e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        Ok(Some((
            fs::read(self.path("bekleyen.kcad"))?,
            facts.based_on,
        )))
    }

    /// The waiting save reached the server (or was dropped on purpose).
    pub fn clear_save(&mut self) -> Result<(), ReplicaError> {
        for name in ["bekleyen.json", "bekleyen.kcad"] {
            match fs::remove_file(self.path(name)) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e.into()),
                _ => {}
            }
        }
        Ok(())
    }
}

/// The steps of a log: a last line cut short by a crash is dropped; any
/// other unreadable line makes the copy unreadable.
fn read_steps(path: &Path) -> Result<Vec<BaseStep>, ReplicaError> {
    let text = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let lines: Vec<&[u8]> = text.split(|&b| b == b'\n').collect();
    let last = lines.len().saturating_sub(1);
    let mut steps = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        match serde_json::from_slice::<BaseStep>(line) {
            Ok(step) => steps.push(step),
            // Only the very last line can be cut short: it had no newline yet.
            Err(_) if i == last => break,
            Err(e) => return Err(unreadable("taban günlüğü", e)),
        }
    }
    Ok(steps)
}

#[cfg(test)]
mod tests;
