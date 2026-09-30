//! The device draft of an open database project (docs/adr/0040), as the web
//! keeps it (apps/web/src/app/cloud/drafts.ts, syncRestore.ts; CLAUDE.md
//! §21.3): what is not sent yet, and the command on its way with its
//! idempotency key, written on this device as edits happen, so a crash or a
//! lost connection loses nothing and a command whose answer was lost is
//! answered once. The file's shape is the web's draft, format 2: changes by
//! the object's persistent id (its id on the server), each with the server
//! version it was based on. A draft belongs to one account and one project;
//! it never holds a session or a password (drafts.rs stores it).
//!
//! Putting a draft back into a fresh opening ([`ProjectSync::restore`]):
//!
//! - the command that was on its way goes first, as it was: if it had been
//!   committed the server answers from its log, and its changes are the
//!   server's then. Its changes are in the drawing at once (this device may
//!   have no connection to wait for) and count as unsent until it is answered;
//! - every other change goes into the drawing as unsent local work, without
//!   an undo step; one whose base the server moved past meanwhile is a
//!   conflict (the drawing shows the local copy until the user chooses), and
//!   a deletion the server has too is done;
//! - a change the drawing cannot take (its layer is gone) stays in the next
//!   draft, unsent, and the user is told;
//! - a layer the draft's tree drops that still holds the server's objects
//!   (someone drew on it before the removal went) stays, and the user is
//!   told (the web's `restoreDraft`, 36d87de; docs/adr/0081);
//! - block definitions (docs/adr/0144 §5) the same way: made and changed
//!   ones go in inner first, before the objects that may place them, a name
//!   someone else took meanwhile giving way (“Kapı (2)”); a removal is left
//!   out while the drawing's objects or definitions still place it; one the
//!   list cannot take stays in the next draft, unsent.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use kentos_contracts::{
    BlockChange, BlockDefinition, BlockId, CommandEnvelope, ConflictReason, Entity, FeatureChange,
    FeatureRecord, ProjectChanges, ProjectPatch,
};
use kentos_domain::{Document, External, ExternalMeta, Slot};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::blocks::{block_key, renamed_block_text, takes};
use super::remote::{is_layer, tree_ids, with_nodes_from};
use super::{Conflict, Inflight, Meta, PROJECT_KEY, Planned, PlannedBlock, ProjectSync, SaveState};

/// The draft format this code writes (the web's `DRAFT_VERSION`).
pub const DRAFT_VERSION: u32 = 2;

/// One object's unsent state: what it should become (`None`: deleted) and
/// the server version it was based on (`None`: the server does not have it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DraftChange {
    pub base: Option<String>,
    pub entity: Option<Entity>,
}

/// One block definition's unsent state: what it should become (`None`:
/// removed) and the server version it was based on (`None`: the server does not have it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DraftBlock {
    pub base: Option<String>,
    pub block: Option<BlockDefinition>,
}

/// Unsent metadata: the patch and the metadata version it was based on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DraftMeta {
    pub base: String,
    pub patch: ProjectPatch,
}

/// What of one project is on this device and not on the server.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub version: u32,
    /// The account it belongs to: another account's draft is never put back.
    pub user_id: String,
    /// By the object's persistent id, lowercase with hyphens.
    pub changes: BTreeMap<String, DraftChange>,
    /// By the block definition's id; none when no definition waits.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub blocks: BTreeMap<String, DraftBlock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<DraftMeta>,
    /// The command on its way: sent again with its key when the draft comes back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inflight: Option<CommandEnvelope>,
    /// When it was written, in milliseconds since 1970.
    pub updated: u64,
}

/// What putting a draft back did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Restored {
    /// Objects (and the metadata) that went back into the drawing, unsent.
    pub changed: usize,
    /// Of those, the ones the server moved past meanwhile: now conflicts.
    pub conflicts: usize,
    /// A command was on its way: it is sent again first, with its key.
    pub resends: bool,
    /// Changes kept aside, unsent (the drawing could not take them), with the reason, in Turkish.
    pub held: Vec<String>,
    /// What else the user hears (a block definition that gave way to a name, a removal left out).
    pub notes: Vec<String>,
    /// Layers the draft's tree drops that stay for the server's objects on
    /// them, by name (said with `given_back_text`).
    pub given_back: Vec<String>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// A definition change a command was planned from, read back from its envelope.
fn planned_block_of(change: &BlockChange, expected: &BTreeMap<String, String>) -> Option<PlannedBlock> {
    Some(match change {
        BlockChange::Create { block } => PlannedBlock::Create {
            block: std::sync::Arc::new(block.clone()),
        },
        BlockChange::Update { block } => PlannedBlock::Update {
            block: std::sync::Arc::new(block.clone()),
            expected: expected.get(&block_key(block.id))?.clone(),
        },
        BlockChange::Delete { id } => PlannedBlock::Delete {
            id: *id,
            expected: expected.get(&block_key(*id))?.clone(),
        },
    })
}

/// The work a command was planned from, read back from its envelope.
fn planned_of(change: &FeatureChange, expected: &BTreeMap<String, String>) -> Option<Planned> {
    Some(match change {
        FeatureChange::Create { id, entity } => Planned::Create {
            id: Uuid::parse_str(id).ok()?,
            entity: entity.clone(),
        },
        FeatureChange::Update { id, entity } => Planned::Update {
            id: Uuid::parse_str(id).ok()?,
            entity: entity.clone(),
            expected: expected.get(id)?.clone(),
        },
        FeatureChange::Delete { id } => Planned::Delete {
            id: Uuid::parse_str(id).ok()?,
            expected: expected.get(id)?.clone(),
        },
    })
}

impl Meta {
    /// This metadata with a patch applied.
    fn patched(&self, patch: &ProjectPatch) -> Meta {
        Meta {
            name: patch.name.clone().unwrap_or_else(|| self.name.clone()),
            settings: patch
                .settings
                .clone()
                .unwrap_or_else(|| self.settings.clone()),
            layers: patch.layers.clone().unwrap_or_else(|| self.layers.clone()),
            styles: patch.styles.clone().unwrap_or_else(|| self.styles.clone()),
        }
    }
}

impl ProjectSync {
    /// A draft's block definition changes as the list the drawing takes (none:
    /// it keeps its own), the conflicts of those the server moved past, and
    /// what could not go in, held for the next draft (see the module comment).
    /// `change`: the draft's objects, which the removals are counted after.
    fn restore_blocks(
        &mut self,
        doc: &Document,
        changes: &BTreeMap<String, DraftBlock>,
        carried: &HashSet<BlockId>,
        change: &External,
        conflicts: &mut Vec<Conflict>,
        restored: &mut Restored,
    ) -> Option<Vec<BlockDefinition>> {
        let mut upserts: Vec<(BlockId, DraftBlock)> = Vec::new();
        let mut removals: Vec<BlockId> = Vec::new();
        for (key, c) in changes {
            let Some(id) = Uuid::parse_str(key).ok().map(|u| BlockId(u.into_bytes())) else {
                restored.held.push(format!("{key}: bir blok kimliği değil; değişiklik atlandı"));
                continue;
            };
            // Changed in this opening already: the newer change wins.
            if self.block_differs(doc, id) {
                continue;
            }
            let server = self.known_blocks.get(&id).map(|t| t.version.clone());
            if c.block.is_none() && server.is_none() && doc.block(id).is_none() {
                continue;
            }
            if !carried.contains(&id) && c.base != server {
                conflicts.push(Conflict {
                    id: block_key(id),
                    reason: if server.is_some() {
                        ConflictReason::Changed
                    } else {
                        ConflictReason::Deleted
                    },
                    server: None,
                    expected: c.base.clone(),
                    actual: server,
                });
            }
            match &c.block {
                Some(_) => upserts.push((id, c.clone())),
                None => removals.push(id),
            }
        }
        if upserts.is_empty() && removals.is_empty() {
            return None;
        }
        let mut list: Vec<BlockDefinition> = doc.blocks().iter().map(|b| (**b).clone()).collect();
        // Inner first: a definition after the ones it places.
        let depth = {
            let mut all = list.clone();
            for (id, c) in &upserts {
                if let Some(b) = &c.block {
                    match all.iter_mut().find(|x| x.id == *id) {
                        Some(slot) => *slot = b.clone(),
                        None => all.push(b.clone()),
                    }
                }
            }
            let index: HashMap<BlockId, usize> = all.iter().enumerate().map(|(i, b)| (b.id, i)).collect();
            let depth = kentos_contracts::blocks::nesting(&all, &index).unwrap_or_default();
            move |id: BlockId| index.get(&id).and_then(|&i| depth.get(i)).copied().unwrap_or(0)
        };
        upserts.sort_by_key(|(id, _)| depth(*id));
        for (id, c) in upserts {
            let Some(mut block) = c.block.clone() else {
                continue;
            };
            let others: Vec<&str> = list.iter().filter(|b| b.id != id).map(|b| b.name.as_str()).collect();
            if others.iter().any(|n| kentos_contracts::blocks::name_key(n) == kentos_contracts::blocks::name_key(&block.name)) {
                let name = kentos_contracts::blocks::import_names(others.iter().copied(), [block.name.as_str()])
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| block.name.clone());
                restored.notes.push(renamed_block_text(&block.name, &name));
                block.name = name;
            }
            let mut candidate = list.clone();
            match candidate.iter_mut().find(|b| b.id == id) {
                Some(slot) => *slot = block.clone(),
                None => candidate.push(block.clone()),
            }
            let refs: Vec<&BlockDefinition> = candidate.iter().collect();
            match takes(&refs) {
                Ok(()) => list = candidate,
                Err(why) => {
                    restored.held.push(format!(
                        "“{}” blok tanımı çizime konamadı ({why}); değişiklik bu cihazda saklanıyor, gönderilmedi",
                        block.name
                    ));
                    conflicts.retain(|k| k.id != block_key(id));
                    self.held_blocks.insert(id, c);
                }
            }
        }
        // A removal waits for the draft's objects: one still placed by them, or by a definition that stays, stays.
        let removed_objects: HashSet<Uuid> = change.remove.iter().copied().collect();
        let put: HashMap<Uuid, &Entity> = change.put.iter().map(|(id, e)| (*id, e)).collect();
        let mut placed: HashSet<BlockId> = HashSet::new();
        for e in doc.entities() {
            let uid = doc.uid(Slot(e.base().id));
            if uid.is_some_and(|u| removed_objects.contains(&u) || put.contains_key(&u)) {
                continue;
            }
            if let Entity::Insert(i) = e {
                placed.insert(i.block);
            }
        }
        for e in put.values() {
            if let Entity::Insert(i) = e {
                placed.insert(i.block);
            }
        }
        let mut gone: HashSet<BlockId> = removals.iter().copied().filter(|id| list.iter().any(|b| b.id == *id)).collect();
        loop {
            let inside: HashSet<BlockId> = list
                .iter()
                .filter(|b| !gone.contains(&b.id))
                .flat_map(|b| b.entities.iter())
                .filter_map(|e| match e {
                    Entity::Insert(i) => Some(i.block),
                    _ => None,
                })
                .collect();
            let stays: Vec<BlockId> = gone
                .iter()
                .copied()
                .filter(|id| placed.contains(id) || inside.contains(id))
                .collect();
            if stays.is_empty() {
                break;
            }
            for id in stays {
                gone.remove(&id);
                conflicts.retain(|k| k.id != block_key(id));
                let name = list.iter().find(|b| b.id == id).map_or_else(String::new, |b| b.name.clone());
                restored
                    .notes
                    .push(format!("“{name}” bloğu başka birinin yerleştirmesinde kullanılıyor; taslaktaki silinmesi uygulanmadı."));
            }
        }
        list.retain(|b| !gone.contains(&b.id));
        let same = list.len() == doc.blocks().len() && list.iter().zip(doc.blocks()).all(|(a, b)| *a == **b);
        (!same).then_some(list)
    }

    /// What of this project is not on the server, for the device: `None`
    /// when nothing is (the stored draft is then removed). A viewer's own
    /// edits are never kept (they were told so).
    pub fn draft(&mut self, doc: &Document, user_id: &str) -> Option<Draft> {
        if !self.can_write && !self.state.ended() {
            return None;
        }
        self.observe(doc);
        let mut changes: BTreeMap<String, DraftChange> = self
            .held
            .iter()
            .map(|(id, c)| (id.to_string(), c.clone()))
            .collect();
        for &id in &self.dirty {
            let change = match self.plan(doc, id) {
                Some(Planned::Create { entity, .. }) => DraftChange {
                    base: None,
                    entity: Some(entity),
                },
                Some(Planned::Update {
                    entity, expected, ..
                }) => DraftChange {
                    base: Some(expected),
                    entity: Some(entity),
                },
                Some(Planned::Delete { expected, .. }) => DraftChange {
                    base: Some(expected),
                    entity: None,
                },
                None => continue,
            };
            changes.insert(id.to_string(), change);
        }
        let mut blocks: BTreeMap<String, DraftBlock> = self
            .held_blocks
            .iter()
            .map(|(id, b)| (id.to_string(), b.clone()))
            .collect();
        for p in self.plan_blocks(doc) {
            let (id, change) = match p {
                PlannedBlock::Create { block } => (
                    block.id,
                    DraftBlock {
                        base: None,
                        block: Some((*block).clone()),
                    },
                ),
                PlannedBlock::Update { block, expected } => (
                    block.id,
                    DraftBlock {
                        base: Some(expected),
                        block: Some((*block).clone()),
                    },
                ),
                PlannedBlock::Delete { id, expected } => (
                    id,
                    DraftBlock {
                        base: Some(expected),
                        block: None,
                    },
                ),
            };
            blocks.insert(id.to_string(), change);
        }
        let meta = if self.sends_meta() {
            self.meta_base.patch(doc).map(|patch| DraftMeta {
                base: self.meta_version.clone(),
                patch,
            })
        } else {
            None
        };
        let inflight = self.inflight.as_ref().map(|f| f.envelope.clone());
        if changes.is_empty() && blocks.is_empty() && meta.is_none() && inflight.is_none() {
            return None;
        }
        Some(Draft {
            version: DRAFT_VERSION,
            user_id: user_id.to_owned(),
            changes,
            blocks,
            meta,
            inflight,
            updated: now_ms(),
        })
    }

    /// Puts a draft of this device back into the project just opened (see
    /// the module comment). Refused, with nothing changed, while an edit is
    /// open. Call it before any edit of the new opening.
    pub fn restore(&mut self, doc: &mut Document, draft: Draft) -> Result<Restored, String> {
        if doc.is_busy() {
            return Err(
                "Açık bir düzenleme var; cihazdaki taslak o bitince geri konur.".to_owned(),
            );
        }
        self.observe(doc);
        let mut restored = Restored::default();
        // The command on its way goes again first, with its key; its changes are its own.
        let mut carried: HashSet<Uuid> = HashSet::new();
        let mut carried_blocks: HashSet<BlockId> = HashSet::new();
        if let Some(envelope) = draft.inflight
            && self.inflight.is_none()
        {
            match serde_json::from_value::<ProjectChanges>(envelope.input.clone()) {
                Ok(input) => {
                    let planned: Vec<Planned> = input
                        .features
                        .iter()
                        .filter_map(|c| planned_of(c, &envelope.expected_versions))
                        .collect();
                    carried.extend(planned.iter().map(Planned::id));
                    let blocks: Vec<PlannedBlock> = input
                        .blocks
                        .iter()
                        .filter_map(|c| planned_block_of(c, &envelope.expected_versions))
                        .collect();
                    carried_blocks.extend(blocks.iter().map(PlannedBlock::id));
                    let meta = input.project.as_ref().map(|patch| self.meta_base.patched(patch));
                    self.own.insert(envelope.request_id.clone());
                    self.inflight = Some(Inflight {
                        envelope,
                        planned,
                        blocks,
                        meta,
                    });
                    restored.resends = true;
                }
                Err(e) => restored.held.push(format!(
                    "gönderilmekte olan komut okunamadı ({e}); değişiklikleri taslaktan geri konuyor"
                )),
            }
        }
        // The metadata first: an object may sit on a layer it brings.
        let mut change = External::default();
        let mut meta_conflict = None;
        if let Some(m) = &draft.meta
            && self.can_edit_meta
        {
            let mut after = self.meta_base.patched(&m.patch);
            // A layer the draft's tree drops that still holds the server's
            // objects (ones the draft neither deletes nor moves off it):
            // someone else drew on it before the removal went. Data wins: it
            // stays, from the tree just opened.
            let kept = tree_ids(&after.layers);
            let stays = |id: Option<Uuid>, layer: &str| {
                id.is_some_and(|id| match draft.changes.get(&id.to_string()) {
                    None => true,
                    Some(c) => c
                        .entity
                        .as_ref()
                        .is_some_and(|e| e.base().layer_id == layer),
                })
            };
            let back: Vec<(String, String)> = doc
                .layers()
                .leaves()
                .into_iter()
                .filter(|l| !kept.contains(l.id.as_str()))
                .filter(|l| {
                    doc.by_layer(&l.id)
                        .any(|e| stays(doc.uid(Slot(e.base().id)), &l.id))
                })
                .map(|l| (l.id.clone(), l.name.clone()))
                .collect();
            if !back.is_empty() {
                let ids: Vec<String> = back.iter().map(|(id, _)| id.clone()).collect();
                after.layers = with_nodes_from(&after.layers, doc.layers().nodes(), &ids);
                restored.given_back = back.into_iter().map(|(_, name)| name).collect();
            }
            change.meta = Some(ExternalMeta {
                name: Some(after.name),
                settings: Some(after.settings),
                layers: Some(after.layers),
                styles: Some(after.styles),
            });
            if m.base != self.meta_version {
                meta_conflict = Some((m.base.clone(), self.meta_version.clone()));
            }
        }
        let layers = match &change.meta {
            Some(ExternalMeta {
                layers: Some(layers),
                ..
            }) => layers.clone(),
            _ => doc.layers().nodes().to_vec(),
        };
        let mut conflicts = Vec::new();
        let mut touched = Vec::new();
        for (key, c) in draft.changes {
            let Ok(id) = Uuid::parse_str(&key) else {
                restored.held.push(format!(
                    "{key}: bir nesne kimliği değil; değişiklik atlandı"
                ));
                continue;
            };
            // Edited in this opening already: the newer edit wins.
            if self.plan(doc, id).is_some() {
                continue;
            }
            let server = self.known.get(&id);
            // Deleted here and on the server alike: nothing is left to do.
            if c.entity.is_none() && server.is_none() && doc.slot_of(id).is_none() {
                continue;
            }
            if let Some(e) = &c.entity
                && !is_layer(&layers, &e.base().layer_id)
            {
                restored.held.push(format!(
                    "{id}: “{}” katmanı çizimde yok; değişiklik bu cihazda saklanıyor, gönderilmedi",
                    e.base().layer_id
                ));
                self.held.insert(id, c);
                continue;
            }
            // The server moved past its base meanwhile. What the command on its way carries is
            // not checked here: its answer (or its refusal, a conflict then) settles it, and the
            // drawing shows it at once, since this device may have no connection to wait for.
            if !carried.contains(&id) && c.base.as_deref() != server.map(|t| t.version.as_str()) {
                conflicts.push(Conflict {
                    id: id.to_string(),
                    reason: if server.is_some() {
                        ConflictReason::Changed
                    } else {
                        ConflictReason::Deleted
                    },
                    server: server.map(|t| FeatureRecord {
                        id: id.to_string(),
                        version: t.version.clone(),
                        entity: t.entity.clone(),
                    }),
                    expected: c.base.clone(),
                    actual: server.map(|t| t.version.clone()),
                });
            }
            match c.entity {
                Some(entity) => change.put.push((id, entity)),
                None => {
                    if doc.slot_of(id).is_some() {
                        change.remove.push(id);
                    }
                }
            }
            touched.push(id);
        }
        // The block definitions: made and changed ones into the list before the objects
        // that may place them, removals once those objects are counted.
        let blocks_back = self.restore_blocks(doc, &draft.blocks, &carried_blocks, &change, &mut conflicts, &mut restored);
        if let Some(list) = &blocks_back {
            change.blocks = Some(list.clone());
        }
        let meta_back = change.meta.is_some();
        self.apply(doc, change)?;
        // It is this user's work, not sent yet.
        restored.changed = touched.len() + usize::from(meta_back) + usize::from(blocks_back.is_some());
        self.pending_blocks = self.plan_blocks(doc).len();
        self.dirty.extend(touched);
        if meta_back {
            self.meta_dirty = true;
        }
        if let Some((expected, actual)) = meta_conflict {
            conflicts.push(Conflict {
                id: PROJECT_KEY.to_owned(),
                reason: ConflictReason::Project,
                server: None,
                expected: Some(expected),
                actual: Some(actual),
            });
        }
        restored.conflicts = conflicts.len();
        for c in conflicts {
            self.conflict(c);
        }
        if restored.changed > 0 {
            doc.mark_unsaved();
        }
        if self.conflicts.is_empty() && self.can_write && !self.state.ended() {
            self.state = if self.pending() > 0 || self.inflight.is_some() {
                SaveState::Pending
            } else {
                SaveState::Saved
            };
        }
        Ok(restored)
    }
}
