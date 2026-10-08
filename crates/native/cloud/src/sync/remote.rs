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
//! - a layer the new tree drops that still holds this device's unsent
//!   objects stays in the drawing and goes back to the server with them
//!   (the web's `keepUnsentLayers`, 88ca558): losing the user's objects is
//!   worse than bringing back a layer someone removed;
//! - an object on a layer the drawing lacks waits for it, unsaid (a copy
//!   here leaves meanwhile), and is fetched once the drawing has the layer
//!   again (`arrived`; the web's waiting.ts, b3c022f): the drawing lacks it
//!   when its removal here has not gone through;
//! - when the server refuses our tree because a layer it drops still holds
//!   others' objects, that layer is given back (`give_back`, 36d87de);
//! - a deletion of the project ends the sync; an archiving ends it after
//!   the events before it came in;
//! - block definitions the events name come from the server's list with
//!   the objects, in one change (blocks.rs, docs/adr/0144 §5); a refused
//!   removal of one still placed there puts it back (`give_back_blocks`).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use kentos_contracts::{
    BlockId, BlockRecord, ConflictReason, Entity, EventPage, FeatureOp, FeatureRecord, LayerNode,
    LayerNodeType, PROJECT_ACCESS_CHANGED, PROJECT_ARCHIVED, PROJECT_DELETED, ProjectInfo,
};
use kentos_domain::{Document, External, ExternalMeta, Slot};
use uuid::Uuid;

use super::blocks::{Taking, block_of_key, restored_block_text, unmerged_text};
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
    /// Block definitions others made, changed or removed: fetch the server's list.
    pub blocks: Vec<BlockId>,
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
        !self.fetch.is_empty() || self.meta || !self.blocks.is_empty()
    }
}

/// What the server has now of what the events name (`follow::fetch`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Remote {
    /// The objects of `Incoming::fetch` the server still has.
    pub records: Vec<FeatureRecord>,
    /// The project's metadata now, when it changed.
    pub info: Option<ProjectInfo>,
    /// The project's block definitions now, when the events named some or
    /// an object they bring is an insert (`GET …/blocks`).
    pub blocks: Option<Vec<BlockRecord>>,
}

/// What taking others' changes did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Taken {
    /// Objects put in or taken out.
    pub changed: usize,
    /// Objects (and the metadata) with changes here too: now conflicts.
    pub conflicts: usize,
    /// Layers the new tree dropped that stay for their unsent objects.
    pub kept: Vec<KeptLayer>,
    /// What else the user hears (block definitions kept, put back or renamed).
    pub notes: Vec<String>,
}

/// A layer another editor removed that stays in this drawing: it holds
/// objects with changes the server does not have yet, and goes back to the
/// server with them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptLayer {
    pub id: String,
    pub name: String,
    /// How many objects on it have unsent changes.
    pub unsent: usize,
}

impl KeptLayer {
    /// What the user hears (the web's `keptText`).
    pub fn text(&self) -> String {
        format!(
            "“{}” katmanını başka biri sildi; üzerinde gönderilmemiş {} nesneniz olduğu için katman bu çizimde kaldı ve yeniden kaydedilecek.",
            self.name, self.unsent
        )
    }
}

/// What giving layers back did ([`ProjectSync::give_back`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GivenBack {
    /// The layers given back, by name (said with [`given_back_text`]).
    pub names: Vec<String>,
    /// Nothing but the server's guard refused: its conflict ended, what is left goes again.
    pub guard_only: bool,
}

/// What the user hears for a layer given back (the web's `givenBackText`).
pub fn given_back_text(name: &str) -> String {
    format!(
        "“{name}” katmanında başkasının nesnesi olduğu için katman silinmedi; sizin nesneleriniz silindi."
    )
}

/// The node with this id in a tree.
fn find<'a>(nodes: &'a [LayerNode], id: &str) -> Option<&'a LayerNode> {
    nodes.iter().find_map(|n| {
        if n.id == id {
            Some(n)
        } else {
            find(&n.children, id)
        }
    })
}

fn find_mut<'a>(nodes: &'a mut [LayerNode], id: &str) -> Option<&'a mut LayerNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(found) = find_mut(&mut node.children, id) {
            return Some(found);
        }
    }
    None
}

/// The ids of every node of a tree.
pub(super) fn tree_ids(nodes: &[LayerNode]) -> BTreeSet<&str> {
    fn walk<'a>(nodes: &'a [LayerNode], out: &mut BTreeSet<&'a str>) {
        for n in nodes {
            out.insert(n.id.as_str());
            walk(&n.children, out);
        }
    }
    let mut out = BTreeSet::new();
    walk(nodes, &mut out);
    out
}

/// A node of a tree, with its parent group's id (none at the top) and its place there.
fn locate<'a>(
    nodes: &'a [LayerNode],
    id: &str,
    parent: Option<&str>,
) -> Option<(&'a LayerNode, Option<String>, usize)> {
    for (index, n) in nodes.iter().enumerate() {
        if n.id == id {
            return Some((n, parent.map(str::to_owned), index));
        }
        if let Some(found) = locate(&n.children, id, Some(&n.id)) {
            return Some(found);
        }
    }
    None
}

/// `into` with the nodes `ids` of `from` put in, as copies: each into its
/// parent group in `from` when `into` has that group, else at the top; at
/// its place there when it fits (the web's `withNodesFrom`).
pub(super) fn with_nodes_from(
    into: &[LayerNode],
    from: &[LayerNode],
    ids: &[String],
) -> Vec<LayerNode> {
    let mut out = into.to_vec();
    for id in ids {
        let Some((node, parent, index)) = locate(from, id, None) else {
            continue;
        };
        let node = node.clone();
        let group =
            parent.filter(|p| find(&out, p).is_some_and(|g| g.kind == LayerNodeType::Group));
        match group.and_then(|p| find_mut(&mut out, &p)) {
            Some(group) => {
                let at = index.min(group.children.len());
                group.children.insert(at, node);
            }
            None => {
                let at = index.min(out.len());
                out.insert(at, node);
            }
        }
    }
    out
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
                self.cursor_moved();
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
                for b in &event.blocks {
                    if let Ok(id) = Uuid::parse_str(&b.id) {
                        let id = BlockId(id.into_bytes());
                        if !incoming.blocks.contains(&id) {
                            incoming.blocks.push(id);
                        }
                    }
                }
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
    /// instead; one on a layer the drawing does not have waits for it.
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
        // The server's tree, when the drawing takes one with a layer kept (below).
        let mut server_tree = None;
        if incoming.meta
            && let Some(info) = &remote.info
        {
            if self.sends_meta() {
                self.conflict(Conflict {
                    id: PROJECT_KEY.to_owned(),
                    reason: ConflictReason::Project,
                    server: None,
                    expected: Some(self.meta_version.clone()),
                    actual: Some(info.meta_version.clone()),
                });
                taken.conflicts += 1;
            } else {
                let mut meta = meta_of(info);
                if let Some((layers, kept)) = self.keep_unsent(doc, &info.layers) {
                    meta.layers = Some(layers);
                    taken.kept = kept;
                    server_tree = Some(info.layers.clone());
                }
                change.meta = Some(meta);
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
        let mut waits = Vec::new();
        for id in incoming.fetch.iter().chain(&incoming.removed) {
            let record = records.get(id).copied();
            // A version this device knows already (its own commit, taken in
            // before a restart; an event read twice): nothing new.
            if let Some(r) = record
                && self.known.get(id).is_some_and(|t| t.version == r.version)
            {
                continue;
            }
            // Removed, and neither known here nor in the drawing: nothing to do
            // (it may have waited for its layer).
            if record.is_none() && !self.known.contains_key(id) && doc.slot_of(*id).is_none() {
                self.unwait(*id);
                continue;
            }
            if self.busy_locally(doc, *id) {
                // The conflict decides it; it does not wait as well.
                self.unwait(*id);
                self.conflict(Conflict {
                    id: id.to_string(),
                    reason: if record.is_some() {
                        ConflictReason::Changed
                    } else {
                        ConflictReason::Deleted
                    },
                    server: record.cloned(),
                    expected: self.known.get(id).map(|t| t.version.clone()),
                    actual: record.map(|r| r.version.clone()),
                });
                taken.conflicts += 1;
                continue;
            }
            match record {
                // Made on, or moved onto, a layer the drawing lacks: it waits
                // for the layer, and the copy here goes meanwhile.
                Some(r) if !is_layer(layers, &r.entity.base().layer_id) => {
                    if doc.slot_of(*id).is_some() {
                        change.remove.push(*id);
                    }
                    waits.push((r.entity.base().layer_id.clone(), *id));
                    tracked.push((*id, None));
                }
                Some(r) => {
                    self.unwait(*id);
                    change.put.push((*id, r.entity.clone()));
                    tracked.push((*id, Some(r)));
                }
                // Removed, or changed and removed before it was fetched.
                None => {
                    self.unwait(*id);
                    if doc.slot_of(*id).is_some() {
                        change.remove.push(*id);
                    }
                    tracked.push((*id, None));
                }
            }
        }
        // Block definitions (docs/adr/0144 §5): the server's list, in the same change as the objects.
        let mut learned = None;
        if let Some(server) = &remote.blocks {
            let named: HashSet<BlockId> = incoming.blocks.iter().copied().collect();
            match self.merge_with_objects(doc, server, Some(&named), &HashSet::new(), &change) {
                Ok(merge) => {
                    taken.changed += differing(doc, &merge.list);
                    for c in merge.conflicts {
                        self.conflict(c);
                        taken.conflicts += 1;
                    }
                    taken.notes.extend(merge.notes);
                    change.blocks = Some(merge.list);
                    learned = Some(merge.learned);
                }
                Err(why) => {
                    taken.notes.push(unmerged_text(&why));
                    // Objects placing a block this drawing lacks wait for the next opening.
                    let have: HashSet<BlockId> = doc.blocks().iter().map(|b| b.id).collect();
                    let lacking: HashSet<Uuid> = change
                        .put
                        .iter()
                        .filter(|(_, e)| matches!(e, Entity::Insert(i) if !have.contains(&i.block)))
                        .map(|(id, _)| *id)
                        .collect();
                    change.put.retain(|(id, _)| !lacking.contains(id));
                    tracked.retain(|(id, _)| !lacking.contains(id));
                }
            }
        }
        taken.changed += change.put.len() + change.remove.len();
        self.apply(doc, change)?;
        if let Some(learned) = learned {
            self.learned_blocks(doc, learned);
        }
        self.pending_blocks = self.plan_blocks(doc).len();
        for (layer, id) in waits {
            self.wait(layer, id);
        }
        for (id, record) in tracked {
            match record {
                Some(r) => self.know(
                    id,
                    Tracked {
                        version: r.version.clone(),
                        entity: r.entity.clone(),
                    },
                ),
                None => self.forget(id),
            }
            self.dirty.remove(&id);
        }
        if let Some(version) = new_meta {
            self.took_meta(doc, version, server_tree);
        }
        if self.cursor != incoming.cursor {
            self.cursor = incoming.cursor;
            self.cursor_moved();
        }
        if incoming.archived {
            self.inflight = None;
            self.state = SaveState::Archived;
        }
        Ok(taken)
    }

    /// Ends the conflicts by taking the server's copies (`info`: the project's
    /// metadata now, for a conflict of the metadata; `blocks`: its block
    /// definitions now, for a definition's conflict, which stays without it):
    /// the drawing's own changes of those objects are dropped, as a change
    /// from outside. A server copy on a layer the drawing will not have waits
    /// for it, as in `take_remote`. Refused, with nothing changed, while an
    /// edit is open.
    pub fn take_theirs(
        &mut self,
        doc: &mut Document,
        info: Option<&ProjectInfo>,
        blocks: Option<&[BlockRecord]>,
    ) -> Result<Taken, String> {
        if doc.is_busy() {
            return Err(BUSY.to_owned());
        }
        self.observe(doc);
        let mut change = External::default();
        let mut new_meta = None;
        let mut tracked = Vec::new();
        let project = self
            .conflicts
            .iter()
            .any(|c| c.reason == ConflictReason::Project);
        // The tree the objects will be in: the server's, when it is taken too.
        let tree: Vec<LayerNode> = match info {
            Some(info) if project => info.layers.clone(),
            _ => doc.layers().nodes().to_vec(),
        };
        let mut waits = Vec::new();
        let mut taken = Taken::default();
        for c in &self.conflicts {
            if c.reason == ConflictReason::Project || block_of_key(&c.id).is_some() {
                continue;
            }
            let Ok(id) = Uuid::parse_str(&c.id) else {
                continue;
            };
            match &c.server {
                Some(record) if !is_layer(&tree, &record.entity.base().layer_id) => {
                    if doc.slot_of(id).is_some() {
                        change.remove.push(id);
                    }
                    waits.push((record.entity.base().layer_id.clone(), id));
                    tracked.push((id, None));
                }
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
        // The definitions in conflict as the server has them, with the objects that may place them.
        let force: HashSet<BlockId> = self
            .conflicts
            .iter()
            .filter_map(|c| block_of_key(&c.id))
            .collect();
        let mut learned = None;
        let mut blocks_left = !force.is_empty() && blocks.is_none();
        if let Some(server) = blocks
            && !force.is_empty()
        {
            match self.merge_with_objects(doc, server, Some(&HashSet::new()), &force, &change) {
                Ok(merge) => {
                    taken.notes.extend(merge.notes);
                    change.blocks = Some(merge.list);
                    learned = Some(merge.learned);
                }
                Err(why) => {
                    taken.notes.push(unmerged_text(&why));
                    blocks_left = true;
                }
            }
        }
        // The objects' conflicts end first, so they no longer count as unsent
        // when the server's tree comes (a layer kept, as in take_remote).
        self.apply(doc, change)?;
        if let Some(learned) = learned {
            self.learned_blocks(doc, learned);
        }
        for (layer, id) in waits {
            self.wait(layer, id);
        }
        for (id, record) in tracked {
            match record {
                Some(r) => self.know(
                    id,
                    Tracked {
                        version: r.version,
                        entity: r.entity,
                    },
                ),
                None => self.forget(id),
            }
            self.dirty.remove(&id);
        }
        let mut kept = Vec::new();
        if project && let Some(info) = info {
            self.observe(doc);
            let mut meta = meta_of(info);
            let mut server_tree = None;
            if let Some((layers, k)) = self.keep_unsent(doc, &info.layers) {
                meta.layers = Some(layers);
                kept = k;
                server_tree = Some(info.layers.clone());
            }
            self.apply(
                doc,
                External {
                    meta: Some(meta),
                    ..External::default()
                },
            )?;
            new_meta = Some((info.meta_version.clone(), server_tree));
        }
        if let Some((version, server_tree)) = new_meta {
            self.took_meta(doc, version, server_tree);
        }
        // A definition's conflict without the server's list stays, for the next try.
        self.conflicts
            .retain(|c| blocks_left && block_of_key(&c.id).is_some());
        self.pending_blocks = self.plan_blocks(doc).len();
        self.state = if !self.conflicts.is_empty() {
            SaveState::Conflict
        } else if !self.can_write {
            SaveState::ReadOnly
        } else if self.pending() > 0 {
            SaveState::Pending
        } else {
            SaveState::Saved
        };
        taken.kept = kept;
        Ok(taken)
    }

    /// The server's definitions merged with a change of objects coming in
    /// (`change`: what places a block once it is in).
    fn merge_with_objects(
        &self,
        doc: &Document,
        server: &[BlockRecord],
        events: Option<&HashSet<BlockId>>,
        force: &HashSet<BlockId>,
        change: &External,
    ) -> Result<super::blocks::BlockMerge, String> {
        let leaving: HashSet<Uuid> = change
            .remove
            .iter()
            .copied()
            .chain(change.put.iter().map(|(id, _)| *id))
            .collect();
        let arriving: Vec<BlockId> = change
            .put
            .iter()
            .filter_map(|(_, e)| match e {
                Entity::Insert(i) => Some(i.block),
                _ => None,
            })
            .collect();
        let mut placed = || {
            let mut out: HashSet<BlockId> = arriving.iter().copied().collect();
            for e in doc.entities() {
                if let Entity::Insert(i) = e
                    && !doc
                        .uid(Slot(e.base().id))
                        .is_some_and(|u| leaving.contains(&u))
                {
                    out.insert(i.block);
                }
            }
            out
        };
        self.merge_blocks(doc, server, &Taking { events, force }, &mut placed)
    }

    /// Whether a refusal is the server's guard against removing a block
    /// definition still placed there (someone else's insert): a definition's
    /// conflict whose version is the one expected (a version conflict's
    /// differ). Then the missed events come in first, and `give_back_blocks`
    /// with the server's list.
    pub fn may_give_back_blocks(&self) -> bool {
        self.conflicts.iter().any(|c| {
            block_of_key(&c.id).is_some() && c.expected.is_some() && c.expected == c.actual
        })
    }

    /// The server refused to remove block definitions still placed there:
    /// data wins over the removal, as with layers. Each such definition the
    /// drawing still lacks (the events that came in did not bring it back)
    /// comes back from the server's list (`server`), said once; those
    /// conflicts end, and what is left goes again when nothing else refused.
    /// Refused, with nothing changed, while an edit is open.
    pub fn give_back_blocks(
        &mut self,
        doc: &mut Document,
        server: &[BlockRecord],
    ) -> Result<Vec<String>, String> {
        if doc.is_busy() {
            return Err(BUSY.to_owned());
        }
        self.observe(doc);
        let guarded: Vec<BlockId> = self
            .conflicts
            .iter()
            .filter(|c| c.expected.is_some() && c.expected == c.actual)
            .filter_map(|c| block_of_key(&c.id))
            .collect();
        let missing: HashSet<BlockId> = guarded
            .iter()
            .copied()
            .filter(|id| doc.block(*id).is_none())
            .collect();
        let mut notes = Vec::new();
        if !missing.is_empty() {
            let merge = self
                .merge_with_objects(
                    doc,
                    server,
                    Some(&HashSet::new()),
                    &missing,
                    &External::default(),
                )
                .map_err(|why| unmerged_text(&why))?;
            let learned = merge.learned;
            self.apply(
                doc,
                External {
                    blocks: Some(merge.list),
                    ..External::default()
                },
            )?;
            self.learned_blocks(doc, learned);
            for id in &missing {
                if let Some(b) = doc.block(*id) {
                    notes.push(restored_block_text(&b.name));
                }
            }
        }
        let keys: HashSet<String> = guarded
            .iter()
            .map(|id| super::blocks::block_key(*id))
            .collect();
        self.conflicts.retain(|c| !keys.contains(&c.id));
        self.pending_blocks = self.plan_blocks(doc).len();
        if self.conflicts.is_empty() && !self.state.ended() {
            self.state = if !self.can_write {
                SaveState::ReadOnly
            } else if self.pending() > 0 {
                SaveState::Pending
            } else {
                SaveState::Saved
            };
        }
        Ok(notes)
    }

    /// The server's metadata is the drawing's now, at `version`. With a
    /// layer kept (`server_tree`: the server's tree without it), the base is
    /// the server's and the drawing's tree differs from it: the kept layer
    /// goes back with the next command.
    fn took_meta(&mut self, doc: &Document, version: String, server_tree: Option<Vec<LayerNode>>) {
        self.meta_version = version;
        self.meta_base = Meta::of(doc);
        if let Some(tree) = server_tree {
            self.meta_base.layers = tree;
        }
        self.meta_dirty = self.meta_base.patch(doc).is_some();
        self.meta_known();
    }

    /// `incoming` with the layers of this drawing it drops that hold objects
    /// with changes the server does not have (unsent, or on their way) put
    /// back, each a copy of this drawing's node: into its group when the
    /// incoming tree has that group, else at the top, at its place there when
    /// it fits (the web's `keepUnsentLayers`). None when it drops none of those.
    fn keep_unsent(
        &self,
        doc: &Document,
        incoming: &[LayerNode],
    ) -> Option<(Vec<LayerNode>, Vec<KeptLayer>)> {
        let mut kept = Vec::new();
        for leaf in doc.layers().leaves() {
            if find(incoming, &leaf.id).is_some() {
                continue;
            }
            let unsent = doc
                .by_layer(&leaf.id)
                .filter(|e| {
                    doc.uid(Slot(e.base().id))
                        .is_some_and(|id| self.busy_locally(doc, id))
                })
                .count();
            if unsent > 0 {
                kept.push(KeptLayer {
                    id: leaf.id.clone(),
                    name: leaf.name.clone(),
                    unsent,
                });
            }
        }
        if kept.is_empty() {
            return None;
        }
        let ids: Vec<String> = kept.iter().map(|k| k.id.clone()).collect();
        Some((with_nodes_from(incoming, doc.layers().nodes(), &ids), kept))
    }

    /// Another editor's object waits for its layer.
    fn wait(&mut self, layer: String, id: Uuid) {
        self.unwait(id);
        self.waiting.entry(layer).or_default().insert(id);
    }

    /// An object waits no more (put, deleted, or a conflict now).
    fn unwait(&mut self, id: Uuid) {
        self.waiting.retain(|_, ids| {
            ids.remove(&id);
            !ids.is_empty()
        });
    }

    /// Whether any object waits for its layer.
    pub fn is_waiting(&self) -> bool {
        !self.waiting.is_empty()
    }

    /// The waiting objects whose layer the drawing has now (another editor's
    /// tree brought it, an undo here, a layer given back): fetch them and
    /// take them in (`take_remote` with them to fetch). They wait until
    /// taken in, so a fetch that fails loses none.
    pub fn arrived(&self, doc: &Document) -> Vec<Uuid> {
        let tree = doc.layers().nodes();
        self.waiting
            .iter()
            .filter(|(layer, _)| is_layer(tree, layer))
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect()
    }

    /// What the user hears when the project is left while objects still
    /// wait: one line per layer, named as the server's tree last named it
    /// (the web's `waitingTexts`).
    pub fn waiting_texts(&self) -> Vec<String> {
        self.waiting
            .iter()
            .map(|(layer, ids)| {
                let name = find(&self.meta_base.layers, layer).map_or(layer.as_str(), |n| n.name.as_str());
                format!(
                    "“{name}” katmanı bu çizimde olmadığı için başka birinin {} nesnesi burada gösterilmedi; proje yeniden açılınca görünür.",
                    ids.len()
                )
            })
            .collect()
    }

    /// Whether a refusal may be the server's guard against our tree dropping
    /// a layer that still holds objects (docs/adr/0072): the metadata
    /// conflicts while the tree we send drops layers. Then the missed events
    /// come in first, and `give_back` with the server's tree.
    pub fn may_give_back(&self, doc: &Document) -> bool {
        self.conflicts.iter().any(|c| c.id == PROJECT_KEY)
            && self
                .meta_base
                .patch(doc)
                .is_some_and(|p| p.layers.is_some())
    }

    /// The server refused our tree because a layer it drops still holds
    /// objects: someone else drew on it before our removal went (the web's
    /// `giveBackUsedLayers`, 36d87de). Data wins over the removal: each
    /// layer our tree drops that the server's tree (`server`, read after the
    /// missed events came in) still has, and on which others' objects wait,
    /// goes back into our tree from the server's, at its place there. The
    /// rest of our tree stays ours (renames, styles, other removals), and a
    /// layer that held only our objects still goes; our deletions of the
    /// objects on it are still waiting and go with the next command. When
    /// the guard alone refused, its conflict ends here and what is left goes
    /// again: the guard answers with the metadata version it has and the one
    /// we expected, the same when no one else changed the metadata meanwhile.
    /// Otherwise the conflict stays for the user, our tree already holding
    /// the layers given back. Nothing happens without a metadata conflict.
    /// Refused, with nothing changed, while an edit is open.
    pub fn give_back(
        &mut self,
        doc: &mut Document,
        server: &ProjectInfo,
    ) -> Result<GivenBack, String> {
        if doc.is_busy() {
            return Err(BUSY.to_owned());
        }
        let Some(project) = self.conflicts.iter().find(|c| c.id == PROJECT_KEY) else {
            return Ok(GivenBack::default());
        };
        let guard_only = project.expected.is_some() && project.expected == project.actual;
        self.observe(doc);
        let back: Vec<String> = {
            let ours = tree_ids(doc.layers().nodes());
            tree_ids(&server.layers)
                .into_iter()
                .filter(|id| !ours.contains(id) && self.waiting.contains_key(*id))
                .map(str::to_owned)
                .collect()
        };
        if back.is_empty() {
            return Ok(GivenBack::default());
        }
        let layers = with_nodes_from(doc.layers().nodes(), &server.layers, &back);
        self.apply(
            doc,
            External {
                meta: Some(ExternalMeta {
                    layers: Some(layers),
                    ..ExternalMeta::default()
                }),
                ..External::default()
            },
        )?;
        let names = back
            .iter()
            .map(|id| {
                doc.layers()
                    .get(id)
                    .map_or_else(|| id.clone(), |l| l.name.clone())
            })
            .collect();
        self.meta_dirty = self.meta_base.patch(doc).is_some();
        if guard_only {
            self.conflicts.retain(|c| c.id != PROJECT_KEY);
            if self.conflicts.is_empty() && !self.state.ended() {
                self.state = if !self.can_write {
                    SaveState::ReadOnly
                } else if self.pending() > 0 {
                    SaveState::Pending
                } else {
                    SaveState::Saved
                };
            }
        }
        Ok(GivenBack { names, guard_only })
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

/// How many definitions differ between the drawing's list and `list`: made, changed or removed.
fn differing(doc: &Document, list: &[kentos_contracts::BlockDefinition]) -> usize {
    let now: HashMap<BlockId, &kentos_contracts::BlockDefinition> =
        list.iter().map(|b| (b.id, b)).collect();
    let was: HashSet<BlockId> = doc.blocks().iter().map(|b| b.id).collect();
    let changed = doc
        .blocks()
        .iter()
        .filter(|b| now.get(&b.id).is_none_or(|n| **n != ***b))
        .count();
    changed + list.iter().filter(|b| !was.contains(&b.id)).count()
}
