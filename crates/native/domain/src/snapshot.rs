//! The document to and from snapshots (the shared contracts; docs/adr/0002,
//! 0011): a `.kcad` v1 (`DocumentSnapshotV1`, JSON) and a `.kcad` v2
//! (`DocumentSnapshotV2`, docs/specs/kcad-v2.md). Reading keeps every field so
//! that writing gives back what was read; object ids become slots.
//!
//! A v1 file keeps no persistent ids: they, the project's id and the source
//! record are derived from its content (docs/adr/0014). A v2 file keeps them,
//! and the document gives them back unchanged.

use std::collections::HashSet;
use std::sync::Arc;

use kentos_contracts::{
    DOCUMENT_FORMAT, DOCUMENT_VERSION, DOCUMENT_VERSION_2, DocumentSnapshotV1, DocumentSnapshotV2,
    Entity, EntityId, LayerNode, LayerNodeType, MigrationSource, ProjectId, v1_uids,
};

use crate::document::Document;
use crate::history::History;
use crate::identity::Uuid;
use crate::layers::LayerTree;
use crate::store::{Store, Stored};

/// What every snapshot gives the document, whichever version it came from.
struct Parts {
    snapshot: DocumentSnapshotV1,
    uids: Vec<Uuid>,
    project_id: Option<Uuid>,
    migrated_from: Option<MigrationSource>,
}

impl Document {
    /// A drawing read from a v1 snapshot, clean and without history (web:
    /// `readSnapshot` then `replaceWith`). The objects keep their ids as slots;
    /// new objects are numbered after the largest. The persistent ids, the
    /// project's id and the source record are derived from the content (the
    /// same everywhere, docs/adr/0014), so a v2 save records where they came
    /// from. Refused, with the reason in Turkish, when the layer tree has no
    /// layer, or an object's id is not a unique positive number or its layer
    /// is not a layer of the file (as the web's reader refuses them).
    pub fn from_snapshot(snapshot: DocumentSnapshotV1) -> Result<Self, String> {
        check(&snapshot.layers, &snapshot.entities)?;
        let ids = v1_uids(&snapshot)?;
        Ok(Self::build(Parts {
            uids: ids
                .entities
                .iter()
                .map(|&(_, uid)| Uuid::from_bytes(uid))
                .collect(),
            project_id: Some(Uuid::from_bytes(ids.project)),
            migrated_from: Some(MigrationSource::v1(ids.source_sha256)),
            snapshot,
        }))
    }

    /// A drawing read from a v2 snapshot (docs/specs/kcad-v2.md), clean and
    /// without history: every object keeps the persistent id the file gave
    /// it, the project its id and source record. Refused like `from_snapshot`,
    /// and when the ids are not one per object, unique and not nil.
    pub fn from_snapshot_v2(snapshot: DocumentSnapshotV2) -> Result<Self, String> {
        check(&snapshot.layers, &snapshot.entities)?;
        if snapshot.uids.len() != snapshot.entities.len() {
            return Err(format!(
                "Kalıcı kimlikler: {} nesne ama {} kimlik var",
                snapshot.entities.len(),
                snapshot.uids.len()
            ));
        }
        let mut seen = HashSet::with_capacity(snapshot.uids.len());
        for (i, (entity, uid)) in snapshot.entities.iter().zip(&snapshot.uids).enumerate() {
            if uid.is_nil() || !seen.insert(*uid) {
                return Err(format!(
                    "Nesne {} ({}) › kalıcı kimlik: {uid} boş ya da başka bir nesnede de var",
                    i + 1,
                    entity.kind()
                ));
            }
        }
        let DocumentSnapshotV2 {
            name,
            settings,
            origin,
            home_view,
            layers,
            active_layer,
            entities,
            uids,
            styles,
            project_id,
            migrated_from,
            ..
        } = snapshot;
        Ok(Self::build(Parts {
            snapshot: DocumentSnapshotV1 {
                format: DOCUMENT_FORMAT.to_owned(),
                version: DOCUMENT_VERSION,
                name,
                settings,
                origin,
                home_view,
                layers,
                active_layer,
                entities,
                styles,
            },
            uids: uids.iter().map(|u| Uuid::from_bytes(u.0)).collect(),
            project_id: project_id.map(|p| Uuid::from_bytes(p.0)),
            migrated_from,
        }))
    }

    fn build(parts: Parts) -> Self {
        let DocumentSnapshotV1 {
            name,
            settings,
            origin,
            home_view,
            layers,
            active_layer,
            entities,
            styles,
            ..
        } = parts.snapshot;
        let largest = entities.iter().map(|e| e.base().id).max().unwrap_or(0);
        let mut store = Store::default();
        for (entity, uid) in entities.into_iter().zip(parts.uids) {
            store.put(Stored {
                uid,
                entity: Arc::new(entity),
            });
        }
        Self {
            name,
            settings,
            origin,
            home_view,
            styles,
            project_id: parts.project_id,
            migrated_from: parts.migrated_from,
            layers: LayerTree::new(layers, &active_layer),
            store,
            next_slot: u64::from(largest) + 1,
            history: History::default(),
            edits: 0,
            dirty: false,
        }
    }

    /// The drawing as a v1 snapshot to write: deep copies, objects in document
    /// order. v1 keeps no persistent ids (docs/adr/0014).
    pub fn to_snapshot(&self) -> DocumentSnapshotV1 {
        DocumentSnapshotV1 {
            format: DOCUMENT_FORMAT.to_owned(),
            version: DOCUMENT_VERSION,
            name: self.name.clone(),
            settings: self.settings.clone(),
            origin: self.origin,
            home_view: self.home_view,
            layers: self.layers.nodes().to_vec(),
            active_layer: self.layers.active().to_owned(),
            entities: self.entities().cloned().collect(),
            styles: self.styles.clone(),
        }
    }

    /// The drawing as a v2 snapshot to write (docs/specs/kcad-v2.md): deep
    /// copies, objects in document order with their persistent ids, the
    /// project's id and source record.
    pub fn to_snapshot_v2(&self) -> DocumentSnapshotV2 {
        let (entities, uids) = self
            .store
            .iter()
            .map(|stored| {
                (
                    (*stored.entity).clone(),
                    EntityId(stored.uid.into_bytes()),
                )
            })
            .unzip();
        DocumentSnapshotV2 {
            format: DOCUMENT_FORMAT.to_owned(),
            version: DOCUMENT_VERSION_2,
            name: self.name.clone(),
            settings: self.settings.clone(),
            origin: self.origin,
            home_view: self.home_view,
            layers: self.layers.nodes().to_vec(),
            active_layer: self.layers.active().to_owned(),
            entities,
            uids,
            styles: self.styles.clone(),
            project_id: self.project_id.map(|p| ProjectId(p.into_bytes())),
            migrated_from: self.migrated_from.clone(),
        }
    }
}

/// What the document itself needs of a file; field by field validation of the
/// objects is the reader's (web: `model/snapshot.ts`; v2: `kentos-kcad`).
fn check(layers: &[LayerNode], entities: &[Entity]) -> Result<(), String> {
    let mut leaves = HashSet::new();
    collect_leaves(layers, &mut leaves);
    if leaves.is_empty() {
        return Err("Katmanlar: en az bir katman olmalı".into());
    }
    let mut ids = HashSet::new();
    for (i, entity) in entities.iter().enumerate() {
        let base = entity.base();
        let at = format!("Nesne {} ({})", i + 1, entity.kind());
        if base.id == 0 || !ids.insert(base.id) {
            return Err(format!("{at} › kimlik: benzersiz, pozitif tam sayı olmalı"));
        }
        if !leaves.contains(base.layer_id.as_str()) {
            return Err(format!(
                "{at} › katman: “{}” katmanı dosyada yok",
                base.layer_id
            ));
        }
    }
    Ok(())
}

fn collect_leaves<'a>(nodes: &'a [LayerNode], out: &mut HashSet<&'a str>) {
    for node in nodes {
        match node.kind {
            LayerNodeType::Layer => {
                out.insert(node.id.as_str());
            }
            LayerNodeType::Group => collect_leaves(&node.children, out),
        }
    }
}
