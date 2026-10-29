//! The drawing document of the desktop (docs/adr/0020): the native counterpart
//! of the web's `CadDocument` (apps/web/src/model/document.ts). Both are held
//! to the same behaviour by the shared fixtures in fixtures/document-ops/v1;
//! neither calls the other. The web keeps its TypeScript document.
//!
//! What a `.kcad` file holds lives here: settings, the layer tree, the objects
//! with their persistent ids, the block definitions, the project's styles, and
//! the project's id and migration source when it has them
//! (docs/specs/kcad-v2.md). The edits are in `edit.rs` and `blocks.rs`,
//! transactions and the undo history in `history.rs`, reading and writing
//! snapshots in `snapshot.rs`.

use std::ops::Range;
use std::sync::Arc;

use kentos_contracts::{
    BlockDefinition, Bounds, Entity, MigrationSource, ProjectSettings, ProjectSheets,
    ProjectStyles, Vec2,
};

use crate::calls::Calls;
use crate::history::History;
use crate::identity::{Slot, Uuid};
use crate::layers::LayerTree;
use crate::store::Store;

/// An open drawing. Every change goes through its methods: the edits
/// (`add`, `update`, `remove` … in one undo step each, or in a transaction),
/// undo and redo, and the layer tree's own changes.
#[derive(Clone, Debug)]
pub struct Document {
    pub(crate) name: String,
    pub(crate) settings: ProjectSettings,
    /// Local anchor near the data: the GPU works in float32 relative to it.
    pub(crate) origin: Vec2,
    pub(crate) home_view: Option<Bounds>,
    pub(crate) styles: ProjectStyles,
    /// The block definitions (docs/adr/0144, blocks.rs), each shared: a reader
    /// that follows them keeps the list and compares its entries by pointer.
    pub(crate) blocks: Vec<Arc<BlockDefinition>>,
    /// The project's persistent id, when it has one: derived from a v1 file,
    /// or read from a v2 file (docs/adr/0014). Not an edit; a v2 file keeps it.
    pub(crate) project_id: Option<Uuid>,
    /// The v1 file this drawing was migrated from; every v2 save keeps it.
    pub(crate) migrated_from: Option<MigrationSource>,
    /// The project's sheet book and its pictures (docs/adr/0251), shared: a
    /// reading copy does not copy the pictures.
    pub(crate) sheets: Option<Arc<ProjectSheets>>,
    /// The associative arrays (docs/adr/0300, arrays.rs), each shared as the blocks are.
    pub(crate) arrays: Vec<Arc<kentos_contracts::AssocArray>>,
    /// Persistent architectural dimension recipes; the visible members are ordinary dimensions.
    pub(crate) dimension_sets: Vec<Arc<kentos_contracts::DimensionSet>>,
    /// The kitchen runs (docs/adr/0727, casework.rs): each one's recipe, walls and objects.
    pub(crate) casework: Vec<Arc<kentos_contracts::CaseworkRecord>>,
    /// The constraints and the parameter table (docs/adr/0334, constraints.rs), one shared value an edit replaces.
    pub(crate) constraints: Arc<kentos_contracts::ConstraintSet>,
    pub(crate) layers: LayerTree,
    pub(crate) store: Store,
    /// The next slot to give out; u64 so the last `u32` can be given without overflow.
    pub(crate) next_slot: u64,
    /// Within a step: objects whose associative dimensions follow them by where
    /// their points are, onto the objects listed (`relocate_ties`, docs/adr/0296).
    pub(crate) relocated: std::collections::BTreeMap<Uuid, Vec<Uuid>>,
    /// What a step may do to the objects (docs/adr/0411 §5): a Veri girişi's cloud project changes
    /// existing records' attributes alone. Not the file's: the host sets it from the account's role.
    pub(crate) object_policy: ObjectPolicy,
    /// Of those, the objects whose ends a point on none of the objects follows by its place (Buda, docs/adr/0320 §2).
    pub(crate) relocated_ends: std::collections::BTreeSet<Uuid>,
    /// Whether writes relax the constraints instead of keeping them (docs/adr/0337 §1): the session's, not the file's.
    pub(crate) relax: crate::constraint_follow::Relax,
    pub(crate) history: History,
    /// The journal of what was written and the ledger of requests executed
    /// (calls.rs, docs/adr/0253 §3, §6).
    pub(crate) calls: Calls,
    /// Counts changes to what a saved file holds (see `mark_saved`).
    pub(crate) edits: u64,
    /// Counts every change to what the drawing holds, changes from outside included.
    pub(crate) generation: u64,
    pub(crate) dirty: bool,
}

/// What a step may do to the drawing's objects (docs/adr/0411 §5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ObjectPolicy {
    /// Anything.
    #[default]
    Free,
    /// Existing objects' attributes alone: no object added or removed, nothing else of one changed.
    AttributesOnly,
}

impl Document {
    /// What a step may do to the objects (docs/adr/0411 §5).
    pub fn object_policy(&self) -> ObjectPolicy {
        self.object_policy
    }

    /// Sets what a step may do to the objects: the host's, from the account's role.
    pub fn set_object_policy(&mut self, policy: ObjectPolicy) {
        self.object_policy = policy;
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn settings(&self) -> &ProjectSettings {
        &self.settings
    }

    pub fn origin(&self) -> Vec2 {
        self.origin
    }

    /// Where the view opens; all objects when unset.
    pub fn home_view(&self) -> Option<Bounds> {
        self.home_view
    }

    pub fn styles(&self) -> &ProjectStyles {
        &self.styles
    }

    /// The project's persistent id, when it has one (docs/adr/0014).
    pub fn project_id(&self) -> Option<Uuid> {
        self.project_id
    }

    /// The v1 file this drawing was migrated from, when it was (docs/specs/kcad-v2.md §6.8).
    pub fn migrated_from(&self) -> Option<&MigrationSource> {
        self.migrated_from.as_ref()
    }

    /// The project's sheet book and its pictures, when it has one (docs/adr/0251).
    pub fn sheets(&self) -> Option<&ProjectSheets> {
        self.sheets.as_deref()
    }

    /// The same, shared: a host that shows the book compares it by pointer to
    /// know whether something else (a sync, a script) changed it.
    pub fn sheets_shared(&self) -> Option<&Arc<ProjectSheets>> {
        self.sheets.as_ref()
    }

    pub fn layers(&self) -> &LayerTree {
        &self.layers
    }

    /// Within a step: the associative dimensions' points on `of` follow it
    /// by where they are rather than by their place's index (docs/adr/0296):
    /// when the step is recorded, each such point is looked for on the
    /// objects of `onto`, in their order, as they are then; it takes the
    /// first one it lies on, at the place it lies on there, and keeps its
    /// coordinates; on none its reference breaks (`vertex_gone`). Kır gives
    /// an object's pieces, Birleştir a joined object for each of its members,
    /// Yönü çevir the object itself. Forgotten when the step ends.
    pub fn relocate_ties(&mut self, of: Uuid, onto: Vec<Uuid>) {
        self.relocated.insert(of, onto);
    }

    /// [`Self::relocate_ties`] for Buda (docs/adr/0320 §2): a point on none of `onto` that was the object's start,
    /// end or vertex follows that place on `of` as it is now (the end the cut moved), rather than breaking.
    pub fn relocate_ties_keeping_ends(&mut self, of: Uuid, onto: Vec<Uuid>) {
        self.relocated.insert(of, onto);
        self.relocated_ends.insert(of);
    }

    // ── Objects ──────────────────────────────────────────────────────────

    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.len() == 0
    }

    /// An object by slot; its `id` is the slot.
    pub fn get(&self, slot: Slot) -> Option<&Entity> {
        self.store.get(slot).map(|stored| &*stored.entity)
    }

    /// An object's persistent id (docs/adr/0014).
    pub fn uid(&self, slot: Slot) -> Option<Uuid> {
        self.store.get(slot).map(|stored| stored.uid)
    }

    /// Where the object with this persistent id is, if it is in the document.
    pub fn slot_of(&self, uid: Uuid) -> Option<Slot> {
        self.store.slot_of(uid)
    }

    /// Every object in document order: drawing order, and the order a file lists them.
    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.store.iter().map(|stored| &*stored.entity)
    }

    /// Every object in document order with its persistent id.
    pub fn entities_with_uids(&self) -> impl Iterator<Item = (Uuid, &Entity)> {
        self.store
            .iter()
            .map(|stored| (stored.uid, &*stored.entity))
    }

    /// The objects of a layer in document order.
    pub fn by_layer(&self, layer: &str) -> impl Iterator<Item = &Entity> {
        self.store.on_layer(layer).map(|stored| &*stored.entity)
    }

    /// A layer's objects in document order with their persistent ids, in one pass (a layer's
    /// filter asks every object's id, docs/adr/0211 §3).
    pub fn by_layer_with_uids(&self, layer: &str) -> impl Iterator<Item = (Uuid, &Entity)> {
        self.store
            .on_layer(layer)
            .map(|stored| (stored.uid, &*stored.entity))
    }

    /// The objects of a layer whose places in the document fall in
    /// `places`, in document order, with their places. A place is kept by an
    /// object that changes and taken again when it is put back (undo), so a
    /// range of places is a stable part of a layer (the desktop draws a large
    /// layer in such parts).
    pub fn by_layer_placed(
        &self,
        layer: &str,
        places: Range<u64>,
    ) -> impl Iterator<Item = (u64, &Entity)> {
        self.store
            .on_layer_placed(layer, places)
            .map(|(place, stored)| (place, &*stored.entity))
    }

    /// The places and slots of a layer's objects in document order, without
    /// reading the objects (a pass over a large layer that needs only who is where).
    pub fn layer_slots(&self, layer: &str) -> impl Iterator<Item = (u64, Slot)> {
        self.store.layer_slots(layer)
    }

    /// An object's place in the document: larger is later.
    pub fn place(&self, slot: Slot) -> Option<u64> {
        self.store.place(slot)
    }

    /// How many objects a layer has.
    pub fn count(&self, layer: &str) -> usize {
        self.store.count(layer)
    }

    /// What the drawing holds now, without its undo history, for reading
    /// elsewhere (the desktop computes a large processing job on another
    /// thread). The objects are shared, not copied; the copy's revision and
    /// generation are the drawing's.
    pub fn reading_copy(&self) -> Document {
        Document {
            name: self.name.clone(),
            settings: self.settings.clone(),
            origin: self.origin,
            home_view: self.home_view,
            styles: self.styles.clone(),
            blocks: self.blocks.clone(),
            arrays: self.arrays.clone(),
            dimension_sets: self.dimension_sets.clone(),
            casework: self.casework.clone(),
            constraints: self.constraints.clone(),
            project_id: self.project_id,
            migrated_from: self.migrated_from.clone(),
            sheets: self.sheets.clone(),
            layers: self.layers.clone(),
            store: self.store.clone(),
            next_slot: self.next_slot,
            relocated: Default::default(),
            object_policy: self.object_policy,
            relocated_ends: Default::default(),
            relax: self.relax,
            history: History::default(),
            // A copy starts its own journal; the ledger it keeps, so a call
            // on the copy (a plan, a job) knows the requests already executed.
            calls: self.calls.reading_copy(),
            edits: self.edits,
            generation: self.generation,
            dirty: self.dirty,
        }
    }

    /// A [`Document::reading_copy`] without the fields `names` (docs/adr/0743 §2): their values gone from every
    /// object and their definitions from every layer. What a file an agent writes is made from; the objects
    /// that hold none of them stay shared.
    pub fn reading_copy_without(&self, names: &[String]) -> Document {
        let mut copy = self.reading_copy();
        if names.is_empty() {
            return copy;
        }
        let hidden = |k: &str| names.iter().any(|n| n == k);
        let cut: Vec<crate::store::Stored> = copy
            .store
            .iter()
            .filter(|s| s.entity.base().attrs.keys().any(|k| hidden(k)))
            .map(|s| {
                let mut entity = (*s.entity).clone();
                entity.base_mut().attrs.retain(|k, _| !hidden(k));
                crate::store::Stored {
                    uid: s.uid,
                    entity: Arc::new(entity),
                }
            })
            .collect();
        for stored in cut {
            copy.store.put(stored);
        }
        let kept: Vec<(String, Vec<kentos_contracts::LayerField>)> = copy
            .layers
            .leaves()
            .into_iter()
            .filter(|l| l.fields.iter().any(|f| hidden(&f.name) || hidden(f.key())))
            .map(|l| {
                let fields = l
                    .fields
                    .iter()
                    .filter(|f| !hidden(&f.name) && !hidden(f.key()))
                    .cloned()
                    .collect();
                (l.id.clone(), fields)
            })
            .collect();
        for (id, fields) in kept {
            copy.layers.replace_fields(&id, &fields);
        }
        copy
    }

    // ── Saving ───────────────────────────────────────────────────────────

    /// The dimensions with a broken reference, in slot order: what the drawing marks (docs/adr/0249 §3).
    pub fn broken_dimensions(&self) -> impl Iterator<Item = Slot> + '_ {
        self.store.broken_dimensions()
    }

    /// The spaces whose walls no longer close round them (docs/adr/0342 §3), in slot order.
    pub fn lost_spaces(&self) -> impl Iterator<Item = Slot> + '_ {
        self.store.lost_spaces()
    }

    /// The revision of what a file would hold: it changes with every edit,
    /// undo, redo and layer change. Only comparisons mean something.
    ///
    /// While a command's step is open (`run_step`), it is the revision the
    /// step makes once recorded: a command's answer, made before its step
    /// is recorded, names the revision its write gives, as it did when each
    /// command recorded its own step (docs/adr/0253 §1).
    pub fn revision(&self) -> u64 {
        self.edits + u64::from(self.step_will_record())
    }

    /// Changes with every change to what the drawing holds: this user's
    /// edits, undo and redo, layer changes, and changes from outside
    /// (`apply_external`), which leave `revision` alone. Readers that redraw
    /// or follow the drawing key on this; saving keys on `revision`.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The slot the next new object takes: every object added after reading
    /// it has this slot or a later one, as slots are never reused (a group
    /// template's members find the objects a tool wrote by it, docs/adr/0176
    /// §5; the web's `nextSlot`).
    pub fn next_slot(&self) -> u64 {
        self.next_slot
    }

    /// Whether the drawing has changes a save has not written.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// A save of `revision` finished: the drawing is clean unless it changed
    /// since, so an edit made during a slow save stays unsaved (CLAUDE.md §4.8, §21.3).
    pub fn mark_saved(&mut self, revision: u64) {
        if revision == self.edits {
            self.dirty = false;
        }
    }

    /// The drawing has changes no save has written (web: cloud sync restoring a device draft).
    pub fn mark_unsaved(&mut self) {
        self.mark_edited();
    }

    pub(crate) fn mark_edited(&mut self) {
        self.edits += 1;
        self.generation += 1;
        self.dirty = true;
    }

    /// What the drawing holds changed, though no edit is recorded yet (a step
    /// an open group gathered, or a group taken back): what shows it follows.
    pub(crate) fn mark_changed(&mut self) {
        self.generation += 1;
    }
}
