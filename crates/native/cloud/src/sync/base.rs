//! What the server has of an open database project, as this device knows it
//! (docs/adr/0043): the base the local copy (replica.rs) keeps, so the
//! project opens without a connection exactly as it was last seen.
//!
//! Every change of that knowledge — a command the server acknowledged,
//! another editor's change taken in, a conflict's server copy kept or taken,
//! a new event cursor — is gathered as a [`BaseStep`] until the desktop takes
//! it ([`ProjectSync::take_base_step`]) and appends it to the copy's log,
//! before it writes the next device draft. The copy and the draft on disk
//! then always agree: a crash leaves either both before a step or the new
//! base with the old draft, whose command on its way the server answers again
//! from its log.

use std::collections::{BTreeMap, BTreeSet};

use kentos_contracts::{
    DOCUMENT_FORMAT, DOCUMENT_VERSION_2, DocumentSnapshotV2, Entity, EntityId, LayerNode,
    ProjectId, ProjectSettings, ProjectStyles,
};
use kentos_domain::Document;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ProjectSync, Tracked};

/// An object as the server has it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BaseObject {
    pub id: String,
    pub version: String,
    pub entity: Entity,
}

/// The project's shared metadata as the server has it, with its version.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BaseMeta {
    pub version: String,
    pub name: String,
    pub settings: ProjectSettings,
    pub layers: Vec<LayerNode>,
    pub styles: ProjectStyles,
}

/// One change of what this device knows the server has.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BaseStep {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub put: Vec<BaseObject>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<BaseMeta>,
}

impl BaseStep {
    pub fn is_empty(&self) -> bool {
        self.cursor.is_none()
            && self.put.is_empty()
            && self.remove.is_empty()
            && self.meta.is_none()
    }
}

/// What changed since the last step was taken.
#[derive(Clone, Debug, Default)]
pub(super) struct Gathered {
    put: BTreeSet<Uuid>,
    remove: BTreeSet<Uuid>,
    meta: bool,
    cursor: bool,
}

/// The whole base at one moment, for rewriting the copy (a compaction).
#[derive(Clone, Debug, PartialEq)]
pub struct BaseSnapshot {
    /// The server's drawing: its objects in id order, as an opening numbers them.
    pub snapshot: DocumentSnapshotV2,
    pub versions: Vec<(Uuid, String)>,
    pub meta_version: String,
    pub cursor: String,
}

impl ProjectSync {
    /// The server has this object at this version.
    pub(super) fn know(&mut self, id: Uuid, t: Tracked) {
        self.known.insert(id, t);
        self.gathered.remove.remove(&id);
        self.gathered.put.insert(id);
    }

    /// The server does not have this object.
    pub(super) fn forget(&mut self, id: Uuid) {
        self.known.remove(&id);
        self.gathered.put.remove(&id);
        self.gathered.remove.insert(id);
    }

    /// The server's metadata version or content changed.
    pub(super) fn meta_known(&mut self) {
        self.gathered.meta = true;
    }

    /// The event cursor moved.
    pub(super) fn cursor_moved(&mut self) {
        self.gathered.cursor = true;
    }

    /// What this device came to know of the server since the last call:
    /// append it to the local copy before writing the next device draft.
    pub fn take_base_step(&mut self) -> Option<BaseStep> {
        let g = std::mem::take(&mut self.gathered);
        let step = BaseStep {
            cursor: g.cursor.then(|| self.cursor.clone()),
            put: g
                .put
                .iter()
                .filter_map(|id| {
                    self.known.get(id).map(|t| BaseObject {
                        id: id.to_string(),
                        version: t.version.clone(),
                        entity: t.entity.clone(),
                    })
                })
                .collect(),
            remove: g.remove.iter().map(Uuid::to_string).collect(),
            meta: g.meta.then(|| BaseMeta {
                version: self.meta_version.clone(),
                name: self.meta_base.name.clone(),
                settings: self.meta_base.settings.clone(),
                layers: self.meta_base.layers.clone(),
                styles: self.meta_base.styles.clone(),
            }),
        };
        (!step.is_empty()).then_some(step)
    }

    /// The whole base now, for rewriting the local copy: the objects the
    /// server has at their versions, its metadata, and the cursor. The
    /// origin, view and this user's active layer are the drawing's.
    pub fn base(&self, doc: &Document) -> BaseSnapshot {
        let mut ids: Vec<&Uuid> = self.known.keys().collect();
        ids.sort();
        let mut entities = Vec::with_capacity(ids.len());
        let mut uids = Vec::with_capacity(ids.len());
        let mut versions = Vec::with_capacity(ids.len());
        for (i, id) in ids.into_iter().enumerate() {
            let t = &self.known[id];
            let mut entity = t.entity.clone();
            entity.base_mut().id = u32::try_from(i + 1).unwrap_or(u32::MAX);
            entities.push(entity);
            uids.push(EntityId(id.into_bytes()));
            versions.push((*id, t.version.clone()));
        }
        let active = doc.layers().active().to_owned();
        BaseSnapshot {
            snapshot: DocumentSnapshotV2 {
                format: DOCUMENT_FORMAT.to_owned(),
                version: DOCUMENT_VERSION_2,
                name: self.meta_base.name.clone(),
                settings: self.meta_base.settings.clone(),
                origin: doc.origin(),
                home_view: doc.home_view(),
                layers: self.meta_base.layers.clone(),
                active_layer: active,
                entities,
                uids,
                styles: self.meta_base.styles.clone(),
                project_id: Some(ProjectId(self.project.into_bytes())),
                migrated_from: None,
            },
            versions,
            meta_version: self.meta_version.clone(),
            cursor: self.cursor.clone(),
        }
    }
}

/// A base's objects by id, for applying steps to it (replica.rs).
pub(crate) fn objects_by_id(
    snapshot: &DocumentSnapshotV2,
    versions: &[(Uuid, String)],
) -> BTreeMap<Uuid, (String, Entity)> {
    let version: BTreeMap<Uuid, &String> = versions.iter().map(|(id, v)| (*id, v)).collect();
    snapshot
        .uids
        .iter()
        .zip(&snapshot.entities)
        .filter_map(|(uid, e)| {
            let id = Uuid::from_bytes(uid.0);
            version.get(&id).map(|v| (id, ((*v).clone(), e.clone())))
        })
        .collect()
}
