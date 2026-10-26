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
    BlockDefinition, DOCUMENT_FORMAT, DOCUMENT_VERSION, DOCUMENT_VERSION_2, DocumentSnapshotV1,
    DocumentSnapshotV2, Entity, EntityId, LayerNode, LayerNodeType, MigrationSource, ProjectId,
    ProjectSettings, ProjectSheets, v1_uids,
};

use crate::document::Document;
use crate::history::History;
use crate::identity::{Slot, Uuid};
use crate::layers::LayerTree;
use crate::store::{Store, Stored};

/// What every snapshot gives the document, whichever version it came from.
struct Parts {
    snapshot: DocumentSnapshotV1,
    uids: Vec<Uuid>,
    project_id: Option<Uuid>,
    migrated_from: Option<MigrationSource>,
    sheets: Option<ProjectSheets>,
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
        check(&snapshot.layers, &snapshot.entities, &snapshot.blocks)?;
        let ids = v1_uids(&snapshot)?;
        Ok(Self::build(Parts {
            uids: ids
                .entities
                .iter()
                .map(|&(_, uid)| Uuid::from_bytes(uid))
                .collect(),
            project_id: Some(Uuid::from_bytes(ids.project)),
            migrated_from: Some(MigrationSource::v1(ids.source_sha256)),
            sheets: None,
            snapshot,
        }))
    }

    /// A drawing read from a v2 snapshot (docs/specs/kcad-v2.md), clean and
    /// without history: every object keeps the persistent id the file gave
    /// it, the project its id and source record. Refused like `from_snapshot`,
    /// and when the ids are not one per object, unique and not nil.
    pub fn from_snapshot_v2(snapshot: DocumentSnapshotV2) -> Result<Self, String> {
        check(&snapshot.layers, &snapshot.entities, &snapshot.blocks)?;
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
        if let Some((at, why)) = snapshot
            .sheets
            .as_ref()
            .and_then(kentos_contracts::sheets_problem)
        {
            return Err(format!("{at}: {why}"));
        }
        if let Some((_, why)) = kentos_contracts::requests_problem(&snapshot.requests) {
            return Err(format!("İstekler: {why}"));
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
            blocks,
            project_id,
            migrated_from,
            sheets,
            requests,
            arrays,
            constraints,
            ..
        } = snapshot;
        let mut doc = Self::build(Parts {
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
                blocks,
            },
            uids: uids.iter().map(|u| Uuid::from_bytes(u.0)).collect(),
            project_id: project_id.map(|p| Uuid::from_bytes(p.0)),
            migrated_from,
            sheets,
        });
        doc.load_requests(requests);
        if !arrays.is_empty() {
            let list: Vec<&kentos_contracts::AssocArray> = arrays.iter().collect();
            if let Some(why) = doc.arrays_refused(&list) {
                return Err(why);
            }
            doc.arrays = arrays.into_iter().map(Arc::new).collect();
        }
        if !constraints.is_empty() {
            if let Some(why) = doc.constraints_refused(&constraints) {
                return Err(why);
            }
            doc.constraints = Arc::new(constraints);
        }
        Ok(doc)
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
            blocks,
            ..
        } = parts.snapshot;
        // The former Hibrit mode is held as a type not asked yet, as the web holds it (docs/adr/0165 §1),
        // and the systems and choices as a project keeps them (docs/adr/0167 §1, 0168).
        let settings = ProjectSettings {
            workspace: settings.project_type(),
            ..settings
        }
        .sanitized();
        let largest = entities.iter().map(|e| e.base().id).max().unwrap_or(0);
        let mut store = Store::default();
        for (entity, uid) in shared(entities).into_iter().zip(parts.uids) {
            store.put(Stored { uid, entity });
        }
        Self {
            name,
            settings,
            origin,
            home_view,
            styles,
            blocks: blocks.into_iter().map(Arc::new).collect(),
            project_id: parts.project_id,
            migrated_from: parts.migrated_from,
            sheets: parts.sheets.map(Arc::new),
            arrays: Vec::new(),
            constraints: Default::default(),
            layers: LayerTree::new(layers, &active_layer),
            store,
            next_slot: u64::from(largest) + 1,
            relocated: Default::default(),
            object_policy: Default::default(),
            relocated_ends: Default::default(),
            relax: Default::default(),
            history: History::default(),
            calls: crate::calls::Calls::default(),
            edits: 0,
            generation: 0,
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
            blocks: self.block_list(),
        }
    }

    /// The drawing as a v2 snapshot to write (docs/specs/kcad-v2.md): deep
    /// copies, objects in document order with their persistent ids, the
    /// project's id and source record, its sheet book (docs/adr/0251).
    pub fn to_snapshot_v2(&self) -> DocumentSnapshotV2 {
        let (head, objects) = self.to_snapshot_v2_shared();
        DocumentSnapshotV2 {
            entities: objects.iter().map(|e| (**e).clone()).collect(),
            ..head
        }
    }

    /// `to_snapshot_v2` with the objects not copied (docs/adr/0654): the drawing with no objects but their ids, and
    /// the objects themselves shared with the document, in the same order. What a save writes from
    /// (`kentos_kcad::encode_verified_parts_watched`): a large drawing is not copied whole to be written.
    pub fn to_snapshot_v2_shared(&self) -> (DocumentSnapshotV2, Vec<Arc<Entity>>) {
        let (objects, uids): (Vec<Arc<Entity>>, Vec<EntityId>) = self
            .store
            .iter()
            .map(|stored| (stored.entity.clone(), EntityId(stored.uid.into_bytes())))
            .unzip();
        let head = DocumentSnapshotV2 {
            constraints: (*self.constraints).clone(),
            format: DOCUMENT_FORMAT.to_owned(),
            version: DOCUMENT_VERSION_2,
            name: self.name.clone(),
            settings: self.settings.clone(),
            origin: self.origin,
            home_view: self.home_view,
            layers: self.layers.nodes().to_vec(),
            active_layer: self.layers.active().to_owned(),
            entities: Vec::new(),
            uids,
            styles: self.styles.clone(),
            blocks: self.block_list(),
            project_id: self.project_id.map(|p| ProjectId(p.into_bytes())),
            migrated_from: self.migrated_from.clone(),
            sheets: self.sheets.as_deref().cloned(),
            requests: self.requests(),
            arrays: self.arrays.iter().map(|a| (**a).clone()).collect(),
        };
        (head, objects)
    }
}

impl Document {
    /// The drawing's head as a v2 snapshot: its name, settings, layers, blocks and library with only the objects for
    /// which `keep` holds, in document order, and no sheet book, requests or arrays. What the exchange's rules read
    /// of a drawing (docs/adr/0309 §6): a copy or a paste costs what it moves, not the whole drawing.
    pub fn to_snapshot_v2_of(&self, keep: impl Fn(Slot) -> bool) -> DocumentSnapshotV2 {
        let (entities, uids) = self
            .store
            .iter()
            .filter(|stored| keep(Slot(stored.entity.base().id)))
            .map(|stored| ((*stored.entity).clone(), EntityId(stored.uid.into_bytes())))
            .unzip();
        DocumentSnapshotV2 {
            constraints: Default::default(),
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
            blocks: self.block_list(),
            project_id: self.project_id.map(|p| ProjectId(p.into_bytes())),
            migrated_from: self.migrated_from.clone(),
            sheets: None,
            requests: Vec::new(),
            arrays: Vec::new(),
        }
    }

    /// Deep copies of the definitions, in order, for a snapshot.
    fn block_list(&self) -> Vec<BlockDefinition> {
        self.blocks.iter().map(|b| (**b).clone()).collect()
    }
}

/// The objects shared, the list given back as it empties (docs/adr/0654): each object leaves the list from its end and
/// the list is shrunk every so often, so a large drawing's objects are not held twice while they move.
fn shared(mut list: Vec<Entity>) -> Vec<Arc<Entity>> {
    const STEP: usize = 1 << 16;
    let mut out = Vec::with_capacity(list.len());
    while let Some(entity) = list.pop() {
        out.push(Arc::new(entity));
        if list.len().is_multiple_of(STEP) {
            list.shrink_to_fit();
        }
    }
    out.reverse();
    out
}

/// What the document itself needs of a file; field by field validation of the
/// objects is the reader's (web: `model/snapshot.ts`; v2: `kentos-kcad`). The
/// blocks keep their rules (docs/adr/0144); a definition's objects keep their
/// own layers, which need not be in the tree (an insert draws them on its own).
fn check(
    layers: &[LayerNode],
    entities: &[Entity],
    blocks: &[BlockDefinition],
) -> Result<(), String> {
    kentos_contracts::blocks::check(blocks, entities).map_err(|fault| {
        format!(
            "Bloklar: {}",
            fault.message(|i| blocks.get(i).map_or("", |b| b.name.as_str()))
        )
    })?;
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
