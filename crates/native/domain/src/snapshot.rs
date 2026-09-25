//! The document to and from a `.kcad` v1 snapshot (`DocumentSnapshotV1`, the
//! shared contract; docs/adr/0002, 0011). Reading keeps every field so that
//! writing gives back what was read; object ids become slots.

use std::collections::HashSet;
use std::sync::Arc;

use kentos_contracts::{
    DOCUMENT_FORMAT, DOCUMENT_VERSION, DocumentSnapshotV1, LayerNode, LayerNodeType,
};

use crate::document::Document;
use crate::history::History;
use crate::identity::v1_entity_uids;
use crate::layers::LayerTree;
use crate::store::{Store, Stored};

impl Document {
    /// A drawing read from a snapshot, clean and without history (web:
    /// `readSnapshot` then `replaceWith`). The objects keep their ids as slots;
    /// new objects are numbered after the largest. Refused, with the reason in
    /// Turkish, when the layer tree has no layer, or an object's id is not a
    /// unique positive number or its layer is not a layer of the file (as the
    /// web's reader refuses them).
    pub fn from_snapshot(snapshot: DocumentSnapshotV1) -> Result<Self, String> {
        check(&snapshot)?;
        let uids = v1_entity_uids(&snapshot);
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
        } = snapshot;
        let largest = entities.iter().map(|e| e.base().id).max().unwrap_or(0);
        let mut store = Store::default();
        for (entity, uid) in entities.into_iter().zip(uids) {
            store.put(Stored {
                uid,
                entity: Arc::new(entity),
            });
        }
        Ok(Self {
            name,
            settings,
            origin,
            home_view,
            styles,
            layers: LayerTree::new(layers, &active_layer),
            store,
            next_slot: u64::from(largest) + 1,
            history: History::default(),
            edits: 0,
            dirty: false,
        })
    }

    /// The drawing as a snapshot to write: deep copies, objects in document order.
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
}

/// What the document itself needs of a file; field by field validation of the
/// objects is the reader's (web: `model/snapshot.ts`).
fn check(snapshot: &DocumentSnapshotV1) -> Result<(), String> {
    let mut leaves = HashSet::new();
    collect_leaves(&snapshot.layers, &mut leaves);
    if leaves.is_empty() {
        return Err("Katmanlar: en az bir katman olmalı".into());
    }
    let mut ids = HashSet::new();
    for (i, entity) in snapshot.entities.iter().enumerate() {
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
