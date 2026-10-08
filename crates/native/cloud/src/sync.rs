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
//! Other editors' commits come in through remote.rs. What is not sent yet,
//! and the command on its way, go to a device draft (draft.rs, drafts.rs)
//! that a new opening puts back, so a crash loses nothing and a command
//! whose answer was lost is still answered once. Block definitions go with
//! the objects (blocks.rs, docs/adr/0144 §5).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::Duration;

use kentos_contracts::{
    BlockId, BlockRecord, CommandEnvelope, CommitResult, ConflictReason, Entity, FeatureChange,
    FeatureRecord, LayerNode, PROJECT_CHANGES, PROJECT_CHANGES_VERSION, ProjectChanges,
    ProjectPatch, ProjectSettings, ProjectStyles,
};
use kentos_domain::{ChangeMark, Changes, Document, Slot};
use std::sync::Arc;
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
    /// The version the refused change was based on, when known.
    pub expected: Option<String>,
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
    blocks: Vec<PlannedBlock>,
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

/// Every node id of a layer tree.
fn node_ids<'a>(nodes: &'a [LayerNode], out: &mut HashSet<&'a str>) {
    for node in nodes {
        out.insert(node.id.as_str());
        node_ids(&node.children, out);
    }
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
    /// What the server has of the block definitions (blocks.rs).
    known_blocks: HashMap<BlockId, TrackedBlock>,
    /// How many definition changes wait, as the last look found them (`observe`).
    pending_blocks: usize,
    /// Definition changes from a device draft the drawing could not take: kept in the next draft, unsent, as `held`.
    held_blocks: BTreeMap<BlockId, DraftBlock>,
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
    /// The newest event cursor taken in: the opening's, then the events' (remote.rs).
    cursor: String,
    /// Request ids of this sync's own commands: their events are skipped.
    own: HashSet<String>,
    /// Changes from a device draft the drawing could not take (an object on a
    /// layer that is gone): never sent, but written to the next draft again,
    /// so unsent work is not lost, until an edit here replaces them (draft.rs).
    held: BTreeMap<Uuid, DraftChange>,
    /// What the server's side changed since the local copy last took it (base.rs).
    gathered: base::Gathered,
    /// Other editors' objects on a layer this drawing lacks, by that layer's
    /// id: fetched once the drawing has it (remote.rs `arrived`; the web's
    /// waiting.ts). Kept only while the project is open: opened again, the
    /// project comes whole from the server.
    waiting: BTreeMap<String, BTreeSet<Uuid>>,
    tries: u32,
}

impl ProjectSync {
    /// The autosave of a database project just opened; `None` for a file project.
    pub fn new(opened: &Opened) -> Option<Self> {
        let Source::Database { versions, blocks } = &opened.source else {
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
        let mut sync = Self {
            tenant: opened.tenant,
            project: opened.project,
            known,
            known_blocks: HashMap::new(),
            pending_blocks: 0,
            held_blocks: BTreeMap::new(),
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
            own: HashSet::new(),
            held: BTreeMap::new(),
            gathered: base::Gathered::default(),
            waiting: BTreeMap::new(),
            tries: 0,
        };
        sync.opened_blocks(doc, blocks);
        // What the opening read is known already: not a step for the local copy.
        sync.gathered = base::Gathered::default();
        Some(sync)
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

    /// The newest event cursor taken in: ask for the events after it.
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

    /// The server's version of a block definition as last seen (tests, diagnostics).
    pub fn block_version_of(&self, id: BlockId) -> Option<&str> {
        self.known_blocks.get(&id).map(|t| t.version.as_str())
    }

    /// Objects, block definitions (as the last look found them) and the metadata waiting to be sent.
    pub fn pending(&self) -> usize {
        self.dirty.len() + self.pending_blocks + usize::from(self.sends_meta())
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
                    // Edited here: this edit replaces what an earlier draft could not put back.
                    for id in now.into_iter().chain(before) {
                        self.held.remove(&id);
                    }
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
        self.pending_blocks = self.plan_blocks(doc).len();
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

    /// When `tree` drops nodes the server's tree has: the objects waiting to
    /// be sent whose change the server must have before or with that tree,
    /// in id order (the web's `leavingObjects`). They are every deletion (a
    /// deletion never needs the new tree) and every change of an object the
    /// server holds on a dropped layer. Empty when the tree drops nothing.
    fn leaving(&self, doc: &Document, tree: &[LayerNode]) -> BTreeSet<Uuid> {
        let mut kept = HashSet::new();
        node_ids(tree, &mut kept);
        let mut had = HashSet::new();
        node_ids(&self.meta_base.layers, &mut had);
        let dropped: HashSet<&str> = had.difference(&kept).copied().collect();
        if dropped.is_empty() {
            return BTreeSet::new();
        }
        self.dirty
            .iter()
            .copied()
            .filter(|id| {
                // Not on the server: a creation, which may need the new tree.
                self.known.get(id).is_some_and(|t| {
                    current(doc, *id).is_none()
                        || dropped.contains(t.entity.base().layer_id.as_str())
                })
            })
            .collect()
    }

    /// Whether the object has changes here the server does not have: not sent
    /// yet, or in the command on its way.
    pub(super) fn busy_locally(&self, doc: &Document, id: Uuid) -> bool {
        self.plan(doc, id).is_some()
            || self
                .inflight
                .as_ref()
                .is_some_and(|f| f.planned.iter().any(|p| p.id() == id))
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
        let full = if self.sends_meta() {
            self.meta_base.patch(doc)
        } else {
            None
        };
        // A tree without a removed layer is refused while the server holds an
        // object on that layer that the same command neither deletes nor
        // moves off: those changes go first and the tree with the last of
        // them; objects on a layer the tree adds go with it or after it (the
        // web's `leavingObjects`).
        let leaving = full
            .as_ref()
            .and_then(|patch| patch.layers.as_deref())
            .map_or_else(BTreeSet::new, |tree| self.leaving(doc, tree));
        // Block definitions: made and changed ones first, inner before outer, in
        // commands of at most BATCH changes; removed ones once every object goes.
        let (upserts, deletes) = self.ordered_blocks(doc, self.plan_blocks(doc));
        let mut blocks: Vec<PlannedBlock> = upserts.iter().take(BATCH).cloned().collect();
        let order = leaving
            .iter()
            .chain(self.dirty.iter().filter(|id| !leaving.contains(id)));
        let mut planned = Vec::new();
        let mut settled = Vec::new();
        let mut seen = 0;
        let waiting = self.dirty.len().max(leaving.len());
        for &id in order {
            if planned.len() + blocks.len() >= BATCH {
                break;
            }
            seen += 1;
            match self.plan(doc, id) {
                Some(p) => planned.push(p),
                None => settled.push(id),
            }
        }
        for id in settled {
            self.dirty.remove(&id);
        }
        if seen >= waiting && blocks.len() == upserts.len() {
            let room = BATCH.saturating_sub(planned.len() + blocks.len());
            blocks.extend(deletes.into_iter().take(room));
        }
        let patch = if seen < leaving.len() { None } else { full };
        if planned.is_empty() && blocks.is_empty() && patch.is_none() {
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
        for b in &blocks {
            if let Some(v) = b.expected() {
                expected.insert(block_key(b.id()), v.to_owned());
            }
        }
        if patch.is_some() {
            expected.insert(PROJECT_KEY.to_string(), self.meta_version.clone());
        }
        let input = ProjectChanges {
            blocks: blocks.iter().map(PlannedBlock::change).collect(),
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
        self.own.insert(envelope.request_id.clone());
        self.inflight = Some(Inflight {
            envelope: envelope.clone(),
            planned,
            blocks,
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
                Planned::Delete { .. } => self.forget(id),
                Planned::Create { entity, .. } | Planned::Update { entity, .. } => {
                    match result.versions.get(&id.to_string()) {
                        Some(version) => self.know(
                            id,
                            Tracked {
                                version: version.clone(),
                                entity: entity.clone(),
                            },
                        ),
                        None => self.forget(id),
                    }
                }
            }
            // Edited again meanwhile: it still waits.
            if self.plan(doc, id).is_none() {
                self.dirty.remove(&id);
            }
        }
        for b in &f.blocks {
            let id = b.id();
            match b {
                PlannedBlock::Delete { .. } => self.forget_block(id),
                PlannedBlock::Create { block } | PlannedBlock::Update { block, .. } => {
                    match result.versions.get(&block_key(id)) {
                        Some(version) => self.know_block(
                            id,
                            TrackedBlock {
                                version: version.clone(),
                                block: block.clone(),
                            },
                        ),
                        None => self.forget_block(id),
                    }
                }
            }
            // A change of it sent from here replaces one an earlier draft could not put in.
            self.held_blocks.remove(&id);
        }
        self.pending_blocks = self.plan_blocks(doc).len();
        if let Some(meta) = f.meta {
            self.meta_version = result.meta_version.clone();
            self.meta_base = meta;
            self.meta_dirty = self.meta_base.patch(doc).is_some();
            self.meta_known();
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
                expected: c.expected.clone(),
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
    /// A block definition's conflict carries no server copy: `blocks` is the
    /// server's list read for it (`GET …/blocks`); without it such a conflict
    /// stays, for the next try.
    pub fn keep_mine(&mut self, doc: &Document, blocks: Option<&[BlockRecord]>) {
        let mut left = Vec::new();
        for c in std::mem::take(&mut self.conflicts) {
            if c.reason == ConflictReason::Project {
                if let Some(v) = c.actual {
                    self.meta_version = v;
                    self.meta_known();
                }
                continue;
            }
            if let Some(id) = block_of_key(&c.id) {
                let Some(list) = blocks else {
                    left.push(c);
                    continue;
                };
                // Mine goes over the server's version as it is now, or is made again.
                match list.iter().find(|r| r.block.id == id) {
                    Some(r) => self.know_block(
                        id,
                        TrackedBlock {
                            version: r.version.clone(),
                            block: Arc::new(r.block.clone()),
                        },
                    ),
                    None => self.forget_block(id),
                }
                continue;
            }
            let Ok(id) = Uuid::parse_str(&c.id) else {
                continue;
            };
            match (c.actual, c.server) {
                (Some(version), Some(record)) => self.know(
                    id,
                    Tracked {
                        version,
                        entity: record.entity,
                    },
                ),
                _ => self.forget(id),
            }
            self.dirty.insert(id);
        }
        self.conflicts = left;
        self.pending_blocks = self.plan_blocks(doc).len();
        self.state = if !self.conflicts.is_empty() {
            SaveState::Conflict
        } else if self.can_write {
            SaveState::Pending
        } else {
            SaveState::ReadOnly
        };
    }

    /// Whether a block definition is among the conflicts: choosing needs the server's list first.
    pub fn conflicts_blocks(&self) -> bool {
        self.conflicts.iter().any(|c| block_of_key(&c.id).is_some())
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

mod base;
#[cfg(test)]
mod block_tests;
mod blocks;
mod draft;
mod remote;
#[cfg(test)]
mod tests;

use blocks::{PlannedBlock, TrackedBlock, block_key};
pub use blocks::{
    block_of_key, kept_block_text, renamed_block_text, restored_block_text, unmerged_text,
};

pub(crate) use base::objects_by_id;
pub use base::{BaseBlock, BaseMeta, BaseObject, BaseSnapshot, BaseStep};
pub use draft::{DRAFT_VERSION, Draft, DraftBlock, DraftChange, DraftMeta, Restored};
pub use remote::{GivenBack, Incoming, KeptLayer, Remote, Taken, given_back_text};
