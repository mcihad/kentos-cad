//! Sending a database project's changes (docs/adr/0026, 0040): the web's
//! tracker and autosave (apps/web/src/app/cloud/tracker.ts, sync.ts) as a
//! state the desktop drives with its own timers and messages.
//!
//! - **What the server has** of every object is kept: its version and its
//!   content at that version. A change is found by comparison, not by
//!   replaying edits: an undo back to the saved state sends nothing, an
//!   undone deletion creates the object again under the same id.
//! - **What may differ** is followed through the document's change journal
//!   (`Document::changes_since`), slot by slot; the persistent id of a slot
//!   is remembered, so a removed object is named too.
//! - **One command at a time** (`project.changes` v1, at most [`BATCH`]
//!   objects and the metadata): [`ProjectSync::next`] gives it, and the same
//!   one, same idempotency key, again until it is answered, so a lost answer
//!   never commits it twice.
//! - **Refusals** as on the web: a conflict stops sending until the user
//!   chooses; a deleted or archived project, or access taken away, ends it;
//!   a passing failure waits a little longer each time (1 s … 30 s); a
//!   refusal for good says why and waits for the next edit.
//!
//! Not in this slice (docs/adr/0040): other editors' events (the live
//! channel), taking the server's copy in a conflict, and the device draft
//! that keeps unsent changes over a crash. Until then “Kaydedildi” still
//! means only what the server acknowledged.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;

use kentos_contracts::{
    CommandEnvelope, CommitResult, ConflictReason, Entity, FeatureChange, FeatureRecord, LayerNode,
    PROJECT_CHANGES, PROJECT_CHANGES_VERSION, ProjectChanges, ProjectPatch, ProjectSettings,
    ProjectStyles,
};
use kentos_domain::{ChangeMark, Changes, Document, Slot};
use uuid::Uuid;

use crate::failure::ApiFailure;
use crate::open::{Opened, Source};
use crate::saving::envelope;

/// Objects in one command at most (the web's).
pub const BATCH: usize = 2000;

/// The expected-version key of the project's metadata.
pub const PROJECT_KEY: &str = "@project";

/// Where the autosave stands (the web's `SaveState`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveState {
    /// The server has everything.
    Saved,
    /// Edits wait to be sent.
    Pending,
    /// A command is on its way.
    Saving,
    /// The server did not answer; the same command goes again later.
    Offline,
    /// Someone else changed what was sent: nothing more goes until the user chooses.
    Conflict,
    /// The server refused the last command for good; the next edit tries again.
    Error,
    /// This account may only view: edits stay on screen, unsent.
    ReadOnly,
    /// The project was moved to the trash: nothing more is sent.
    Deleted,
    /// This account lost its access: nothing more is sent.
    Revoked,
    /// The project was archived: read-only until it is unarchived and opened again.
    Archived,
}

impl SaveState {
    /// Nothing more is ever sent to the project from this opening.
    pub fn ended(self) -> bool {
        matches!(self, Self::Deleted | Self::Revoked | Self::Archived)
    }
}

/// An object, or the metadata (`@project`), someone else changed first.
#[derive(Clone, Debug, PartialEq)]
pub struct Conflict {
    /// The object's persistent id as text, or `@project`.
    pub id: String,
    pub reason: ConflictReason,
    /// The server's copy now (none when it was deleted, and for the metadata).
    pub server: Option<FeatureRecord>,
    /// The server's version now (none when it was deleted).
    pub actual: Option<String>,
}

/// What to do after a failed send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum After {
    /// Send the same command again after this long.
    Retry(Duration),
    /// Stop until something changes (an edit, the user's choice, a new opening).
    Stop,
}

/// What the server has of an object: its version and its content then.
#[derive(Clone, Debug)]
struct Tracked {
    version: String,
    entity: Entity,
}

/// What one object needs so the server matches the drawing.
#[derive(Clone, Debug, PartialEq)]
enum Planned {
    Create {
        id: Uuid,
        entity: Entity,
    },
    Update {
        id: Uuid,
        entity: Entity,
        expected: String,
    },
    Delete {
        id: Uuid,
        expected: String,
    },
}

impl Planned {
    fn id(&self) -> Uuid {
        match self {
            Self::Create { id, .. } | Self::Update { id, .. } | Self::Delete { id, .. } => *id,
        }
    }

    fn change(&self) -> FeatureChange {
        match self {
            Self::Create { id, entity } => FeatureChange::Create {
                id: id.to_string(),
                entity: entity.clone(),
            },
            Self::Update { id, entity, .. } => FeatureChange::Update {
                id: id.to_string(),
                entity: entity.clone(),
            },
            Self::Delete { id, .. } => FeatureChange::Delete { id: id.to_string() },
        }
    }
}

/// The project's shared metadata (the active layer is each user's own).
#[derive(Clone, Debug, PartialEq)]
struct Meta {
    name: String,
    settings: ProjectSettings,
    layers: Vec<LayerNode>,
    styles: ProjectStyles,
}

impl Meta {
    fn of(doc: &Document) -> Self {
        Self {
            name: doc.name().to_string(),
            settings: doc.settings().clone(),
            layers: doc.layers().nodes().to_vec(),
            styles: doc.styles().clone(),
        }
    }

    /// The metadata of `doc` that differs from this, or none.
    fn patch(&self, doc: &Document) -> Option<ProjectPatch> {
        let mut patch = ProjectPatch::default();
        if doc.name() != self.name {
            patch.name = Some(doc.name().to_string());
        }
        if *doc.settings() != self.settings {
            patch.settings = Some(doc.settings().clone());
        }
        if doc.layers().nodes() != self.layers.as_slice() {
            // A new layer tree carries the active layer, which must be in it.
            patch.layers = Some(doc.layers().nodes().to_vec());
            patch.active_layer = Some(doc.layers().active().to_string());
        }
        if *doc.styles() != self.styles {
            patch.styles = Some(doc.styles().clone());
        }
        (patch != ProjectPatch::default()).then_some(patch)
    }
}

/// The command on its way, or waiting for an answer that was lost.
#[derive(Clone, Debug)]
struct Inflight {
    envelope: CommandEnvelope,
    planned: Vec<Planned>,
    meta: Option<Meta>,
}

/// Whether two objects are the same but for their slots.
fn same(a: &Entity, b: &Entity) -> bool {
    if a.base().id == b.base().id {
        return a == b;
    }
    let mut b = b.clone();
    b.base_mut().id = a.base().id;
    *a == b
}

/// The object with this persistent id in the drawing, if it is there.
fn current(doc: &Document, id: Uuid) -> Option<&Entity> {
    doc.slot_of(id).and_then(|slot| doc.get(slot))
}

/// The autosave of one open database project.
#[derive(Debug)]
pub struct ProjectSync {
    tenant: Uuid,
    project: Uuid,
    known: HashMap<Uuid, Tracked>,
    /// Objects that may differ from the server, in id order (batches are the same every time).
    dirty: BTreeSet<Uuid>,
    /// The persistent id each slot held when last seen.
    slots: HashMap<Slot, Uuid>,
    mark: ChangeMark,
    meta_base: Meta,
    meta_version: String,
    meta_dirty: bool,
    can_write: bool,
    can_edit_meta: bool,
    inflight: Option<Inflight>,
    state: SaveState,
    conflicts: Vec<Conflict>,
    error: Option<String>,
    /// The event cursor of the opening (the live channel follows from it, docs/adr/0040).
    cursor: String,
    tries: u32,
}

impl ProjectSync {
    /// The autosave of a database project just opened; `None` for a file project.
    pub fn new(opened: &Opened) -> Option<Self> {
        let Source::Database { versions } = &opened.source else {
            return None;
        };
        let doc = &opened.document;
        let mut known = HashMap::with_capacity(versions.len());
        for (id, version) in versions {
            if let Some(entity) = current(doc, *id) {
                known.insert(
                    *id,
                    Tracked {
                        version: version.clone(),
                        entity: entity.clone(),
                    },
                );
            }
        }
        let slots = known
            .keys()
            .filter_map(|&id| doc.slot_of(id).map(|slot| (slot, id)))
            .collect();
        let state = if opened.archived() {
            SaveState::Archived
        } else if opened.can_write() {
            SaveState::Saved
        } else {
            SaveState::ReadOnly
        };
        Some(Self {
            tenant: opened.tenant,
            project: opened.project,
            known,
            dirty: BTreeSet::new(),
            slots,
            mark: doc.change_mark(),
            meta_base: Meta::of(doc),
            meta_version: opened.info.meta_version.clone(),
            meta_dirty: false,
            can_write: opened.can_write(),
            can_edit_meta: opened.can_edit_meta(),
            inflight: None,
            state,
            conflicts: Vec::new(),
            error: None,
            cursor: opened.info.event_cursor.clone(),
            tries: 0,
        })
    }

    pub fn state(&self) -> SaveState {
        self.state
    }

    /// The conflicts waiting for the user's choice.
    pub fn conflicts(&self) -> &[Conflict] {
        &self.conflicts
    }

    /// Why the last command was refused for good, in the server's words.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The event cursor of the opening.
    pub fn cursor(&self) -> &str {
        &self.cursor
    }

    /// The server's version of an object as last seen (tests, diagnostics).
    pub fn version_of(&self, id: Uuid) -> Option<&str> {
        self.known.get(&id).map(|t| t.version.as_str())
    }

    fn sends_meta(&self) -> bool {
        self.meta_dirty && self.can_edit_meta
    }

    /// Whether metadata changes stay on this device because the account may not make them.
    pub fn keeps_meta_here(&self) -> bool {
        self.meta_dirty && !self.can_edit_meta
    }

    /// Objects (and the metadata) waiting to be sent.
    pub fn pending(&self) -> usize {
        self.dirty.len() + usize::from(self.sends_meta())
    }

    /// Whether sending can go on: a writer's project that has not ended, with no conflict open.
    fn may_send(&self) -> bool {
        self.can_write && !self.state.ended() && self.conflicts.is_empty()
    }

    /// Whether anything waits to go and may go now: the desktop starts its timer.
    pub fn wants_to_send(&self) -> bool {
        self.may_send() && (self.pending() > 0 || self.inflight.is_some())
    }

    /// Takes in the document's edits since the last look: the objects they
    /// touched, and the metadata when it differs from the server's.
    pub fn observe(&mut self, doc: &Document) {
        match doc.changes_since(self.mark) {
            Changes::Slots(slots) => {
                for &slot in slots {
                    let now = doc.uid(slot);
                    let before = match now {
                        Some(id) => self.slots.insert(slot, id),
                        None => self.slots.remove(&slot),
                    };
                    self.dirty.extend(now);
                    self.dirty.extend(before.filter(|b| Some(*b) != now));
                }
            }
            Changes::All => {
                // More changed than the journal keeps: every object is looked at again.
                self.slots.clear();
                for (&id, _) in self.known.iter() {
                    self.dirty.insert(id);
                }
                let mut seen = Vec::new();
                for entity in doc.entities() {
                    let slot = Slot(entity.base().id);
                    if let Some(id) = doc.uid(slot) {
                        seen.push((slot, id));
                    }
                }
                for (slot, id) in seen {
                    self.slots.insert(slot, id);
                    self.dirty.insert(id);
                }
            }
        }
        self.mark = doc.change_mark();
        self.meta_dirty = self.meta_base.patch(doc).is_some();
        if !self.can_write {
            if !self.state.ended() {
                self.state = SaveState::ReadOnly;
            }
        } else if self.pending() > 0 && matches!(self.state, SaveState::Saved | SaveState::Error) {
            self.state = SaveState::Pending;
        }
    }

    /// What the object with this id needs so the server matches the drawing.
    fn plan(&self, doc: &Document, id: Uuid) -> Option<Planned> {
        match (current(doc, id), self.known.get(&id)) {
            (Some(entity), None) => Some(Planned::Create {
                id,
                entity: entity.clone(),
            }),
            (Some(entity), Some(t)) if !same(entity, &t.entity) => Some(Planned::Update {
                id,
                entity: entity.clone(),
                expected: t.version.clone(),
            }),
            (Some(_), Some(_)) | (None, None) => None,
            (None, Some(t)) => Some(Planned::Delete {
                id,
                expected: t.version.clone(),
            }),
        }
    }

    /// The command to send now: the one on its way again (its answer did not
    /// come), or the next batch of what differs. `None` when nothing waits,
    /// sending is stopped, or an edit is open in the drawing (look again soon).
    pub fn next(&mut self, doc: &Document) -> Option<CommandEnvelope> {
        if !self.may_send() || doc.is_busy() {
            return None;
        }
        if let Some(f) = &self.inflight {
            self.state = SaveState::Saving;
            return Some(f.envelope.clone());
        }
        self.observe(doc);
        let mut planned = Vec::new();
        let mut settled = Vec::new();
        for &id in &self.dirty {
            match self.plan(doc, id) {
                Some(p) => planned.push(p),
                None => settled.push(id),
            }
            if planned.len() >= BATCH {
                break;
            }
        }
        for id in settled {
            self.dirty.remove(&id);
        }
        let patch = if self.sends_meta() {
            self.meta_base.patch(doc)
        } else {
            None
        };
        if planned.is_empty() && patch.is_none() {
            if self.state != SaveState::ReadOnly {
                self.state = SaveState::Saved;
            }
            return None;
        }
        let mut expected = BTreeMap::new();
        for p in &planned {
            match p {
                Planned::Update {
                    id, expected: v, ..
                }
                | Planned::Delete { id, expected: v } => {
                    expected.insert(id.to_string(), v.clone());
                }
                Planned::Create { .. } => {}
            }
        }
        if patch.is_some() {
            expected.insert(PROJECT_KEY.to_string(), self.meta_version.clone());
        }
        let input = ProjectChanges {
            features: planned.iter().map(Planned::change).collect(),
            project: patch.clone(),
        };
        let envelope = envelope(
            self.tenant,
            self.project,
            PROJECT_CHANGES,
            PROJECT_CHANGES_VERSION,
            Uuid::new_v4(),
            expected,
            serde_json::to_value(&input).unwrap_or_default(),
        );
        self.inflight = Some(Inflight {
            envelope: envelope.clone(),
            planned,
            meta: patch.map(|_| Meta::of(doc)),
        });
        self.state = SaveState::Saving;
        Some(envelope)
    }

    /// Whether the server has everything that may be sent: the drawing is
    /// saved (`Document::mark_saved`). Metadata this account may not change
    /// stays on this device and does not count (it was told so).
    pub fn all_sent(&self) -> bool {
        self.pending() == 0 && self.inflight.is_none()
    }

    /// The server committed the command on its way (`result` is its answer).
    pub fn answered(&mut self, doc: &Document, result: &CommitResult) {
        let Some(f) = self.inflight.take() else {
            return;
        };
        // Edits made while it was on its way wait for the next one.
        self.observe(doc);
        for p in &f.planned {
            let id = p.id();
            match p {
                Planned::Delete { .. } => {
                    self.known.remove(&id);
                }
                Planned::Create { entity, .. } | Planned::Update { entity, .. } => {
                    match result.versions.get(&id.to_string()) {
                        Some(version) => {
                            self.known.insert(
                                id,
                                Tracked {
                                    version: version.clone(),
                                    entity: entity.clone(),
                                },
                            );
                        }
                        None => {
                            self.known.remove(&id);
                        }
                    }
                }
            }
            // Edited again meanwhile: it still waits.
            if self.plan(doc, id).is_none() {
                self.dirty.remove(&id);
            }
        }
        if let Some(meta) = f.meta {
            self.meta_version = result.meta_version.clone();
            self.meta_base = meta;
            self.meta_dirty = self.meta_base.patch(doc).is_some();
        }
        self.tries = 0;
        self.error = None;
        self.state = if self.pending() > 0 {
            SaveState::Pending
        } else {
            SaveState::Saved
        };
    }

    /// The command on its way failed: what to do now.
    pub fn failed(&mut self, failure: &ApiFailure) -> After {
        if failure.conflict() {
            self.inflight = None;
            let found = failure.conflicts.iter().map(|c| Conflict {
                id: c.id.clone(),
                reason: c.reason,
                server: c.current.clone(),
                actual: c.actual.clone(),
            });
            for c in found {
                match self.conflicts.iter_mut().find(|k| k.id == c.id) {
                    Some(k) => *k = c,
                    None => self.conflicts.push(c),
                }
            }
            self.state = SaveState::Conflict;
            return After::Stop;
        }
        if failure.deleted() {
            // Refused, not committed: its changes stay waiting.
            self.inflight = None;
            self.state = SaveState::Deleted;
            return After::Stop;
        }
        if failure.archived() {
            self.inflight = None;
            self.state = SaveState::Archived;
            return After::Stop;
        }
        if failure.not_found() {
            // Gone for this account. The command keeps its key: shared again and
            // opened, it goes once more and the server answers it once.
            self.state = SaveState::Revoked;
            return After::Stop;
        }
        if failure.transient() {
            self.tries += 1;
            self.state = SaveState::Offline;
            return After::Retry(failure.backoff(self.tries));
        }
        // Refused for good (a locked layer, a missing right, the session gone): the
        // changes stay waiting, and the next edit, or signing in again, sends them.
        self.inflight = None;
        self.tries = 0;
        self.error = Some(failure.message.clone());
        self.state = SaveState::Error;
        After::Stop
    }

    /// Ends the conflicts by keeping this drawing's copies: each goes over the
    /// server's version now, or is created again under its id when the server
    /// no longer has it; the metadata goes over the server's metadata version.
    pub fn keep_mine(&mut self) {
        for c in std::mem::take(&mut self.conflicts) {
            if c.reason == ConflictReason::Project {
                if let Some(v) = c.actual {
                    self.meta_version = v;
                }
                continue;
            }
            let Ok(id) = Uuid::parse_str(&c.id) else {
                continue;
            };
            match (c.actual, c.server) {
                (Some(version), Some(record)) => {
                    self.known.insert(
                        id,
                        Tracked {
                            version,
                            entity: record.entity,
                        },
                    );
                }
                _ => {
                    self.known.remove(&id);
                }
            }
            self.dirty.insert(id);
        }
        self.state = if self.can_write {
            SaveState::Pending
        } else {
            SaveState::ReadOnly
        };
    }

    /// The server's answer to what this account may do in the project now
    /// (its role changed while it was open).
    pub fn set_access(&mut self, can_write: bool, can_edit_meta: bool) {
        self.can_edit_meta = can_edit_meta;
        if self.state.ended() || can_write == self.can_write {
            return;
        }
        self.can_write = can_write;
        self.state = if !can_write {
            SaveState::ReadOnly
        } else if self.pending() > 0 || self.inflight.is_some() {
            SaveState::Pending
        } else {
            SaveState::Saved
        };
    }
}

#[cfg(test)]
mod tests;
