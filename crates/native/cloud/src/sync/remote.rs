//! Other editors' commits, and the server's side of a conflict
//! (docs/adr/0040), as the web takes them (apps/web/src/app/cloud/syncRemote.ts):
//!
//! - events are read in order and this sync's own commits are skipped;
//! - each object they name is fetched once at its latest version
//!   (follow.rs) and goes into the drawing as a change from outside
//!   (`Document::apply_external`: no undo step, not an edit of this user's),
//!   under the server's id;
//! - an object with changes here the server does not have is not
//!   overwritten: it becomes a conflict;
//! - new metadata comes first, since new objects may sit on a layer it
//!   brings; with unsent metadata here it is a conflict too;
//! - a deletion of the project ends the sync; an archiving ends it after
//!   the events before it came in.

use std::collections::{BTreeMap, HashMap};

use kentos_contracts::{
    ConflictReason, EventPage, FeatureOp, FeatureRecord, LayerNode, LayerNodeType,
    PROJECT_ACCESS_CHANGED, PROJECT_ARCHIVED, PROJECT_DELETED, ProjectInfo,
};
use kentos_domain::{Document, External, ExternalMeta};
use uuid::Uuid;

use super::{Conflict, Meta, PROJECT_KEY, ProjectSync, SaveState, Tracked};

/// What a page of events asks of the drawing ([`ProjectSync::incoming`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Incoming {
    /// Objects others created or changed: fetched at their latest version.
    pub fetch: Vec<Uuid>,
    /// Objects others removed.
    pub removed: Vec<Uuid>,
    /// Their name, settings, layer tree or styles changed: fetch the project's info.
    pub meta: bool,
    /// Someone changed who may do what here: ask the server what this account may do (`set_access`).
    pub access: bool,
    /// The project was archived: nothing is sent after these events come in.
    pub archived: bool,
    /// The cursor after these events.
    pub cursor: String,
}

impl Incoming {
    /// Whether anything must be fetched before [`ProjectSync::take_remote`].
    pub fn needs_fetch(&self) -> bool {
        !self.fetch.is_empty() || self.meta
    }
}

/// What the server has now of what the events name (`follow::fetch`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Remote {
    /// The objects of `Incoming::fetch` the server still has.
    pub records: Vec<FeatureRecord>,
    /// The project's metadata now, when it changed.
    pub info: Option<ProjectInfo>,
}

/// What taking others' changes did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Taken {
    /// Objects put in or taken out.
    pub changed: usize,
    /// Objects (and the metadata) with changes here too: now conflicts.
    pub conflicts: usize,
    /// Objects the drawing could not take, with the reason, in Turkish.
    pub skipped: Vec<String>,
}

/// Whether `id` is a layer (not a group) of this tree.
pub(super) fn is_layer(nodes: &[LayerNode], id: &str) -> bool {
    nodes
        .iter()
        .any(|n| (n.id == id && n.kind == LayerNodeType::Layer) || is_layer(&n.children, id))
}

/// The metadata of the project's info as a change from outside.
fn meta_of(info: &ProjectInfo) -> ExternalMeta {
    ExternalMeta {
        name: Some(info.name.clone()),
        settings: Some(info.settings.clone()),
        layers: Some(info.layers.clone()),
        styles: Some(info.styles.clone()),
    }
}

pub(super) const BUSY: &str =
    "Açık bir düzenleme var; başkalarının değişiklikleri o bitince alınır.";

impl ProjectSync {
    /// Reads committed events after the cursor (a page of `GET …/events`), in
    /// order: what they ask of the drawing. This sync's own commits are
    /// skipped. A deletion of the project ends the sync at once.
    pub fn incoming(&mut self, page: &EventPage) -> Incoming {
        let mut ops: BTreeMap<Uuid, FeatureOp> = BTreeMap::new();
        let mut order = Vec::new();
        let mut incoming = Incoming {
            cursor: page.next.clone(),
            ..Incoming::default()
        };
        for event in &page.events {
            if event.kind == PROJECT_DELETED {
                // Its objects cannot be read any more, and nothing goes after it.
                self.inflight = None;
                self.state = SaveState::Deleted;
                self.cursor = page.next.clone();
                return Incoming {
                    cursor: page.next.clone(),
                    ..Incoming::default()
                };
            }
            incoming.access |= event.kind == PROJECT_ACCESS_CHANGED;
            let own = event
                .request_id
                .as_ref()
                .is_some_and(|r| self.own.contains(r));
            if !own {
                for f in &event.features {
                    if let Ok(id) = Uuid::parse_str(&f.id)
                        && ops.insert(id, f.op).is_none()
                    {
                        order.push(id);
                    }
                }
                incoming.meta |= event.meta;
            }
            if event.kind == PROJECT_ARCHIVED {
                // The events before it still come in; nothing is sent after it.
                incoming.archived = true;
                incoming.cursor = event.seq.clone();
                break;
            }
        }
        for id in order {
            match ops.get(&id) {
                Some(FeatureOp::Delete) => incoming.removed.push(id),
                Some(_) => incoming.fetch.push(id),
                None => {}
            }
        }
        incoming
    }

    /// Puts what the server has now into the drawing (`remote` fetched for
    /// `incoming`), each object under its id, as a change from outside. An
    /// object with changes here the server does not have becomes a conflict
    /// instead; one on a layer the drawing does not have is skipped and named.
    /// Refused, with nothing changed, while an edit is open: try again once it ends.
    pub fn take_remote(
        &mut self,
        doc: &mut Document,
        incoming: Incoming,
        remote: Remote,
    ) -> Result<Taken, String> {
        if doc.is_busy() {
            return Err(BUSY.to_owned());
        }
        // Everything edited here until now is this user's, to be sent.
        self.observe(doc);
        let mut taken = Taken::default();
        let mut change = External::default();
        let mut new_meta = None;
        if incoming.meta
            && let Some(info) = &remote.info
        {
            if self.sends_meta() {
                self.conflict(Conflict {
                    id: PROJECT_KEY.to_owned(),
                    reason: ConflictReason::Project,
                    server: None,
                    actual: Some(info.meta_version.clone()),
                });
                taken.conflicts += 1;
            } else {
                change.meta = Some(meta_of(info));
                new_meta = Some(info.meta_version.clone());
            }
        }
        let layers: &[LayerNode] = match &change.meta {
            Some(ExternalMeta {
                layers: Some(layers),
                ..
            }) => layers,
            _ => doc.layers().nodes(),
        };
        let records: HashMap<Uuid, &FeatureRecord> = remote
            .records
            .iter()
            .filter_map(|r| Uuid::parse_str(&r.id).ok().map(|id| (id, r)))
            .collect();
        let mut tracked = Vec::new();
        for id in incoming.fetch.iter().chain(&incoming.removed) {
            let record = records.get(id).copied();
            if self.busy_locally(doc, *id) {
                self.conflict(Conflict {
                    id: id.to_string(),
                    reason: if record.is_some() {
                        ConflictReason::Changed
                    } else {
                        ConflictReason::Deleted
                    },
                    server: record.cloned(),
                    actual: record.map(|r| r.version.clone()),
                });
                taken.conflicts += 1;
                continue;
            }
            match record {
                Some(r) if !is_layer(layers, &r.entity.base().layer_id) => {
                    taken.skipped.push(format!(
                        "{id}: “{}” katmanı çizimde yok",
                        r.entity.base().layer_id
                    ));
                }
                Some(r) => {
                    change.put.push((*id, r.entity.clone()));
                    tracked.push((*id, Some(r)));
                }
                // Removed, or changed and removed before it was fetched.
                None => {
                    if doc.slot_of(*id).is_some() {
                        change.remove.push(*id);
                    }
                    tracked.push((*id, None));
                }
            }
        }
        taken.changed = change.put.len() + change.remove.len();
        self.apply(doc, change)?;
        for (id, record) in tracked {
            match record {
                Some(r) => {
                    self.known.insert(
                        id,
                        Tracked {
                            version: r.version.clone(),
                            entity: r.entity.clone(),
                        },
                    );
                }
                None => {
                    self.known.remove(&id);
                }
            }
            self.dirty.remove(&id);
        }
        if let Some(version) = new_meta {
            self.meta_version = version;
            self.meta_base = Meta::of(doc);
            self.meta_dirty = false;
        }
        self.cursor = incoming.cursor;
        if incoming.archived {
            self.inflight = None;
            self.state = SaveState::Archived;
        }
        Ok(taken)
    }

    /// Ends the conflicts by taking the server's copies (`info`: the project's
    /// metadata now, for a conflict of the metadata): the drawing's own
    /// changes of those objects are dropped, as a change from outside.
    /// Refused, with nothing changed, while an edit is open.
    pub fn take_theirs(
        &mut self,
        doc: &mut Document,
        info: Option<&ProjectInfo>,
    ) -> Result<(), String> {
        if doc.is_busy() {
            return Err(BUSY.to_owned());
        }
        self.observe(doc);
        let mut change = External::default();
        let mut new_meta = None;
        let mut tracked = Vec::new();
        for c in &self.conflicts {
            if c.reason == ConflictReason::Project {
                if let Some(info) = info {
                    change.meta = Some(meta_of(info));
                    new_meta = Some(info.meta_version.clone());
                }
                continue;
            }
            let Ok(id) = Uuid::parse_str(&c.id) else {
                continue;
            };
            match &c.server {
                Some(record) => {
                    change.put.push((id, record.entity.clone()));
                    tracked.push((id, Some(record.clone())));
                }
                None => {
                    if doc.slot_of(id).is_some() {
                        change.remove.push(id);
                    }
                    tracked.push((id, None));
                }
            }
        }
        self.apply(doc, change)?;
        for (id, record) in tracked {
            match record {
                Some(r) => {
                    self.known.insert(
                        id,
                        Tracked {
                            version: r.version,
                            entity: r.entity,
                        },
                    );
                }
                None => {
                    self.known.remove(&id);
                }
            }
            self.dirty.remove(&id);
        }
        if let Some(version) = new_meta {
            self.meta_version = version;
            self.meta_base = Meta::of(doc);
            self.meta_dirty = false;
        }
        self.conflicts.clear();
        self.state = if !self.can_write {
            SaveState::ReadOnly
        } else if self.pending() > 0 {
            SaveState::Pending
        } else {
            SaveState::Saved
        };
        Ok(())
    }

    /// A conflict, replacing an earlier one of the same object; sending stops.
    pub(super) fn conflict(&mut self, c: Conflict) {
        match self.conflicts.iter_mut().find(|k| k.id == c.id) {
            Some(k) => *k = c,
            None => self.conflicts.push(c),
        }
        if !self.state.ended() {
            self.state = SaveState::Conflict;
        }
    }

    /// Applies a change from outside and moves past it: the slots it touched
    /// are not this user's edits, and the objects it brought are followed.
    pub(super) fn apply(&mut self, doc: &mut Document, change: External) -> Result<(), String> {
        let put: Vec<Uuid> = change.put.iter().map(|(id, _)| *id).collect();
        let gone: Vec<_> = change
            .remove
            .iter()
            .filter_map(|id| doc.slot_of(*id))
            .collect();
        if !change.is_empty() {
            doc.apply_external(change)?;
        }
        self.mark = doc.change_mark();
        for slot in gone {
            self.slots.remove(&slot);
        }
        for id in put {
            if let Some(slot) = doc.slot_of(id) {
                self.slots.insert(slot, id);
            }
        }
        Ok(())
    }
}
