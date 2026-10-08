//! A database project's block definitions in the autosave (docs/adr/0144
//! §5), as the web keeps them (apps/web/src/app/cloud/tracker.ts
//! `BlockTracker`, syncBlocks.ts):
//!
//! - **What the server has** of each definition is kept, its version and the
//!   definition then; the changes are found by comparison, so an undo back
//!   to the saved state sends nothing. The document keeps an unchanged
//!   definition's `Arc`, so most comparisons are a pointer's.
//! - **Sending:** made and changed definitions go before any object, inner
//!   before outer (an insert inside names one the server has or gets in the
//!   same command); removed ones after every object, outer before inner;
//!   each under `expectedVersions["block:<id>"]`.
//! - **The server's list** (`GET …/blocks`) goes in with the objects that
//!   come with it: a definition with unsent changes here keeps them (an
//!   event that changed it too makes it a conflict); data wins over a
//!   removal, as with layers (a definition removed there stays while this
//!   drawing's objects or definitions place it and is made there again; one
//!   removed here comes back when something arriving places it); a name
//!   someone else took first makes this drawing's unsent one give way
//!   (“Kapı (2)”). Each is said.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use kentos_contracts::blocks::{self as rules, check, import_names, name_key, nesting};
use kentos_contracts::{BlockChange, BlockDefinition, BlockId, BlockRecord, ConflictReason};
use kentos_domain::Document;

use super::{Conflict, ProjectSync};

/// The key a definition goes under in `expectedVersions`, a commit's
/// `versions` and `deleted`, and a conflict.
pub fn block_key(id: BlockId) -> String {
    kentos_contracts::block_key(id)
}

/// The definition a conflict's or a commit's key names; none for an object's or the metadata's.
pub fn block_of_key(key: &str) -> Option<BlockId> {
    key.strip_prefix(kentos_contracts::BLOCK_KEY_PREFIX)
        .and_then(|id| uuid::Uuid::parse_str(id).ok())
        .map(|id| BlockId(id.into_bytes()))
}

/// What the server has of a definition: its version and the definition then.
#[derive(Clone, Debug)]
pub(super) struct TrackedBlock {
    pub version: String,
    pub block: Arc<BlockDefinition>,
}

/// What one definition needs so the server matches the drawing.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum PlannedBlock {
    Create {
        block: Arc<BlockDefinition>,
    },
    Update {
        block: Arc<BlockDefinition>,
        expected: String,
    },
    Delete {
        id: BlockId,
        expected: String,
    },
}

impl PlannedBlock {
    pub fn id(&self) -> BlockId {
        match self {
            Self::Create { block } | Self::Update { block, .. } => block.id,
            Self::Delete { id, .. } => *id,
        }
    }

    pub fn change(&self) -> BlockChange {
        match self {
            Self::Create { block } => BlockChange::Create {
                block: (**block).clone(),
            },
            Self::Update { block, .. } => BlockChange::Update {
                block: (**block).clone(),
            },
            Self::Delete { id, .. } => BlockChange::Delete { id: *id },
        }
    }

    /// The version it goes over, unless it makes the definition.
    pub fn expected(&self) -> Option<&str> {
        match self {
            Self::Create { .. } => None,
            Self::Update { expected, .. } | Self::Delete { expected, .. } => Some(expected),
        }
    }
}

/// What the user hears when another editor removed a definition this drawing's unsent objects place.
pub fn kept_block_text(name: &str) -> String {
    format!(
        "“{name}” bloğunu başka biri sildi; bu çizimde onu yerleştiren gönderilmemiş nesneleriniz olduğu için blok kaldı ve yeniden kaydedilecek."
    )
}

/// … when a definition removed here, or refused removal there, comes back because someone else placed it.
pub fn restored_block_text(name: &str) -> String {
    format!(
        "“{name}” bloğu başka birinin yerleştirmesinde kullanılıyor; silinmedi, çizime geri kondu."
    )
}

/// … when an unsent definition gives way to a name someone else took first.
pub fn renamed_block_text(was: &str, now: &str) -> String {
    format!("Başka biri de “{was}” adında bir blok tanımladı; sizinki “{now}” oldu.")
}

/// … when the server's definitions cannot go in with the ones changed here.
pub fn unmerged_text(why: &str) -> String {
    format!(
        "Sunucudaki blok tanımları bu çizimdekilerle birleştirilemedi ({why}); bloklardaki gönderilmemiş değişikliklerinizi gözden geçirin."
    )
}

/// How the server's definitions are taken (the web's `Taking`).
pub(super) struct Taking<'a> {
    /// The definitions the events named: one of them with unsent changes
    /// here that moved on the server is a conflict. `None`: the server's
    /// version of such a definition is only learned (a draft's command went again).
    pub events: Option<&'a HashSet<BlockId>>,
    /// Taken as the server has them even with unsent changes here (a conflict answered with the server's copy).
    pub force: &'a HashSet<BlockId>,
}

/// What the tracker learns of one definition once the merge is in the drawing.
#[derive(Clone, Debug)]
pub(super) enum Learned {
    /// The server has it at this version, as the drawing now holds it.
    Taken(String),
    /// The server has it at this version, as given (unsent changes here stay over it).
    Over(String, Arc<BlockDefinition>),
    /// The server does not have it.
    Gone,
}

/// The server's definitions merged into the drawing's, not applied yet.
#[derive(Clone, Debug, Default)]
pub(super) struct BlockMerge {
    /// The drawing's definitions afterwards: its own order, then the server's new ones in the server's.
    pub list: Vec<BlockDefinition>,
    pub learned: Vec<(BlockId, Learned)>,
    pub conflicts: Vec<Conflict>,
    pub notes: Vec<String>,
}

impl ProjectSync {
    /// Whether the drawing's definition with this id (or its absence) differs from what the server has.
    pub(super) fn block_differs(&self, doc: &Document, id: BlockId) -> bool {
        let here = doc.blocks().iter().find(|b| b.id == id);
        match (here, self.known_blocks.get(&id)) {
            (None, None) => false,
            (Some(b), Some(t)) => !Arc::ptr_eq(b, &t.block) && **b != *t.block,
            _ => true,
        }
    }

    /// What the server needs so its definitions match the drawing's: made and
    /// changed ones in the drawing's order, then removed ones in id order.
    pub(super) fn plan_blocks(&self, doc: &Document) -> Vec<PlannedBlock> {
        let mut out = Vec::new();
        for b in doc.blocks() {
            match self.known_blocks.get(&b.id) {
                None => out.push(PlannedBlock::Create { block: b.clone() }),
                Some(t) if !Arc::ptr_eq(b, &t.block) && **b != *t.block => {
                    out.push(PlannedBlock::Update {
                        block: b.clone(),
                        expected: t.version.clone(),
                    });
                }
                Some(_) => {}
            }
        }
        if self.known_blocks.len() + out.len() > doc.blocks().len() {
            let here: HashSet<BlockId> = doc.blocks().iter().map(|b| b.id).collect();
            let mut gone: Vec<(&BlockId, &TrackedBlock)> = self
                .known_blocks
                .iter()
                .filter(|(id, _)| !here.contains(id))
                .collect();
            gone.sort_by_key(|(id, _)| **id);
            for (id, t) in gone {
                out.push(PlannedBlock::Delete {
                    id: *id,
                    expected: t.version.clone(),
                });
            }
        }
        out
    }

    /// `plan_blocks`' changes in the order the server takes them a part at a
    /// time: made and changed ones inner first, removed ones outer first (the
    /// removed as the server last had them). Within a depth, the plan's order.
    pub(super) fn ordered_blocks(
        &self,
        doc: &Document,
        planned: Vec<PlannedBlock>,
    ) -> (Vec<PlannedBlock>, Vec<PlannedBlock>) {
        let (mut upserts, mut deletes): (Vec<_>, Vec<_>) = planned
            .into_iter()
            .partition(|p| !matches!(p, PlannedBlock::Delete { .. }));
        let mut list: Vec<&BlockDefinition> = doc.blocks().iter().map(|b| &**b).collect();
        for p in &deletes {
            if let Some(t) = self.known_blocks.get(&p.id()) {
                list.push(&t.block);
            }
        }
        let index: HashMap<BlockId, usize> =
            list.iter().enumerate().map(|(i, b)| (b.id, i)).collect();
        if index.len() == list.len()
            && let Ok(depth) = nesting(&list, &index)
        {
            let at = |p: &PlannedBlock| {
                index
                    .get(&p.id())
                    .and_then(|&i| depth.get(i))
                    .copied()
                    .unwrap_or(0)
            };
            upserts.sort_by_key(at);
            deletes.sort_by_key(|p| std::cmp::Reverse(at(p)));
        }
        (upserts, deletes)
    }

    /// The server has this definition at this version.
    pub(super) fn know_block(&mut self, id: BlockId, t: TrackedBlock) {
        self.known_blocks.insert(id, t);
        self.gathered_blocks();
    }

    /// The server does not have this definition.
    pub(super) fn forget_block(&mut self, id: BlockId) {
        if self.known_blocks.remove(&id).is_some() {
            self.gathered_blocks();
        }
    }

    /// The drawing's definitions with the server's taken in (the module's
    /// comment says how); the reason instead when the result would break a
    /// block rule (a nesting both sides changed at once): then nothing is taken.
    /// `placed`: the blocks the drawing's objects place once the incoming
    /// change is in, asked only when a definition would go.
    pub(super) fn merge_blocks(
        &self,
        doc: &Document,
        server: &[BlockRecord],
        how: &Taking<'_>,
        placed: &mut dyn FnMut() -> HashSet<BlockId>,
    ) -> Result<BlockMerge, String> {
        let by_server: HashMap<BlockId, &BlockRecord> =
            server.iter().map(|r| (r.block.id, r)).collect();
        let mut merge = BlockMerge::default();
        let mut next: HashMap<BlockId, BlockDefinition> = HashMap::new();
        let mut mine: HashSet<BlockId> = HashSet::new();
        let mut ids: Vec<BlockId> = doc.blocks().iter().map(|b| b.id).collect();
        ids.extend(server.iter().map(|r| r.block.id));
        ids.extend(self.known_blocks.keys().copied());
        let mut seen = HashSet::new();
        ids.retain(|id| seen.insert(*id));
        for &id in &ids {
            let here = doc.block(id);
            let s = by_server.get(&id).copied();
            let moved = s.map(|r| r.version.as_str())
                != self.known_blocks.get(&id).map(|t| t.version.as_str());
            if !self.block_differs(doc, id) || how.force.contains(&id) {
                // Nothing unsent here: the server's, whatever it is.
                if let Some(r) = s {
                    next.insert(id, r.block.clone());
                }
                merge.learned.push((
                    id,
                    s.map_or(Learned::Gone, |r| Learned::Taken(r.version.clone())),
                ));
                continue;
            }
            if here.is_none() && s.is_none() {
                // Removed here and there alike: nothing is left to send or to ask.
                merge.learned.push((id, Learned::Gone));
                continue;
            }
            mine.insert(id);
            if let Some(b) = here {
                next.insert(id, b.clone());
            }
            if !moved {
                continue;
            }
            match how.events {
                Some(named) if named.contains(&id) => merge.conflicts.push(Conflict {
                    id: block_key(id),
                    reason: if s.is_some() {
                        ConflictReason::Changed
                    } else {
                        ConflictReason::Deleted
                    },
                    server: None,
                    expected: self.known_blocks.get(&id).map(|t| t.version.clone()),
                    actual: s.map(|r| r.version.clone()),
                }),
                Some(_) => {}
                None => merge.learned.push((
                    id,
                    s.map_or(Learned::Gone, |r| {
                        Learned::Over(r.version.clone(), Arc::new(r.block.clone()))
                    }),
                )),
            }
        }
        // Data wins over a removal: whatever the objects or the definitions left place stays, or comes back.
        let mut placed_now: Option<HashSet<BlockId>> = None;
        loop {
            let mut grew = false;
            let inside: HashSet<BlockId> = next
                .values()
                .flat_map(|b| b.entities.iter())
                .filter_map(|e| match e {
                    kentos_contracts::Entity::Insert(i) => Some(i.block),
                    _ => None,
                })
                .collect();
            for &id in &ids {
                if next.contains_key(&id) {
                    continue;
                }
                let by_objects = placed_now.get_or_insert_with(&mut *placed).contains(&id);
                if !by_objects && !inside.contains(&id) {
                    continue;
                }
                match (doc.block(id), by_server.get(&id)) {
                    (Some(here), None) => {
                        // Removed on the server, placed here: it stays, and is made there again, as this drawing's.
                        next.insert(id, here.clone());
                        merge.learned.retain(|(l, _)| *l != id);
                        merge.learned.push((id, Learned::Gone));
                        mine.insert(id);
                        merge.notes.push(kept_block_text(&here.name));
                    }
                    (_, Some(r)) => {
                        // Removed here, placed by what arrives: the server's comes back, and the removal goes with its conflict.
                        next.insert(id, r.block.clone());
                        merge.learned.retain(|(l, _)| *l != id);
                        merge.learned.push((id, Learned::Taken(r.version.clone())));
                        mine.remove(&id);
                        let key = block_key(id);
                        merge.conflicts.retain(|c| c.id != key);
                        merge.notes.push(restored_block_text(&r.block.name));
                    }
                    (None, None) => continue,
                }
                grew = true;
            }
            if !grew {
                break;
            }
        }
        // The drawing's order, then the server's new definitions in the server's.
        let mut used = HashSet::new();
        for id in doc
            .blocks()
            .iter()
            .map(|b| b.id)
            .chain(server.iter().map(|r| r.block.id))
        {
            if used.insert(id)
                && let Some(b) = next.remove(&id)
            {
                merge.list.push(b);
            }
        }
        // A name taken by someone else first: ours gives way.
        let theirs: HashSet<String> = merge
            .list
            .iter()
            .filter(|b| !mine.contains(&b.id))
            .map(|b| name_key(&b.name))
            .collect();
        for i in 0..merge.list.len() {
            if !mine.contains(&merge.list[i].id) || !theirs.contains(&name_key(&merge.list[i].name))
            {
                continue;
            }
            let others: Vec<&str> = merge
                .list
                .iter()
                .enumerate()
                .filter(|(k, _)| *k != i)
                .map(|(_, b)| b.name.as_str())
                .collect();
            let was = merge.list[i].name.clone();
            let name = import_names(others, [was.as_str()])
                .into_iter()
                .next()
                .unwrap_or_else(|| was.clone());
            merge.notes.push(renamed_block_text(&was, &name));
            merge.list[i].name = name;
        }
        check(&merge.list, &[]).map_err(|fault| {
            fault.message(|i| merge.list.get(i).map_or("", |b| b.name.as_str()))
        })?;
        Ok(merge)
    }

    /// Tells the tracker what the server has once a merge is in the drawing.
    pub(super) fn learned_blocks(&mut self, doc: &Document, learned: Vec<(BlockId, Learned)>) {
        for (id, l) in learned {
            match l {
                Learned::Gone => self.forget_block(id),
                Learned::Taken(version) => match doc.blocks().iter().find(|b| b.id == id) {
                    Some(block) => self.know_block(
                        id,
                        TrackedBlock {
                            version,
                            block: block.clone(),
                        },
                    ),
                    None => self.forget_block(id),
                },
                Learned::Over(version, block) => {
                    self.know_block(id, TrackedBlock { version, block })
                }
            }
        }
    }

    /// The block definitions of the server's list, as the tracker keeps them
    /// when a project opens (the list read with its metadata).
    pub(super) fn opened_blocks(&mut self, doc: &Document, versions: &[(BlockId, String)]) {
        for (id, version) in versions {
            if let Some(block) = doc.blocks().iter().find(|b| b.id == *id) {
                self.known_blocks.insert(
                    *id,
                    TrackedBlock {
                        version: version.clone(),
                        block: block.clone(),
                    },
                );
            }
        }
    }

    /// Every definition the server has, with its version, in id order (the local copy's base).
    pub(super) fn known_block_list(&self) -> Vec<(BlockId, String, Arc<BlockDefinition>)> {
        let sorted: BTreeMap<BlockId, &TrackedBlock> =
            self.known_blocks.iter().map(|(id, t)| (*id, t)).collect();
        sorted
            .into_iter()
            .map(|(id, t)| (id, t.version.clone(), t.block.clone()))
            .collect()
    }
}

/// Whether `rules` would take this list (a draft's definitions put back one by one).
pub(super) fn takes(list: &[&BlockDefinition]) -> Result<(), String> {
    rules::check(list, &[])
        .map_err(|fault| fault.message(|i| list.get(i).map_or("", |b| b.name.as_str())))
}
