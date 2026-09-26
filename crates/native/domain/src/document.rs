//! The drawing document of the desktop (docs/adr/0020): the native counterpart
//! of the web's `CadDocument` (apps/web/src/model/document.ts). Both are held
//! to the same behaviour by the shared fixtures in fixtures/document-ops/v1;
//! neither calls the other. The web keeps its TypeScript document.
//!
//! What a `.kcad` file holds lives here: settings, the layer tree, the objects
//! with their persistent ids, the project's styles, and the project's id and
//! migration source when it has them (docs/specs/kcad-v2.md). The edits are
//! in `edit.rs`, transactions and the undo history in `history.rs`, reading
//! and writing snapshots in `snapshot.rs`.

use kentos_contracts::{Bounds, Entity, MigrationSource, ProjectSettings, ProjectStyles, Vec2};

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
    /// The project's persistent id, when it has one: derived from a v1 file,
    /// or read from a v2 file (docs/adr/0014). Not an edit; a v2 file keeps it.
    pub(crate) project_id: Option<Uuid>,
    /// The v1 file this drawing was migrated from; every v2 save keeps it.
    pub(crate) migrated_from: Option<MigrationSource>,
    pub(crate) layers: LayerTree,
    pub(crate) store: Store,
    /// The next slot to give out; u64 so the last `u32` can be given without overflow.
    pub(crate) next_slot: u64,
    pub(crate) history: History,
    /// Counts changes to what a saved file holds (see `mark_saved`).
    pub(crate) edits: u64,
    /// Counts every change to what the drawing holds, changes from outside included.
    pub(crate) generation: u64,
    pub(crate) dirty: bool,
}

impl Document {
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

    pub fn layers(&self) -> &LayerTree {
        &self.layers
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

    /// The objects of a layer in document order.
    pub fn by_layer(&self, layer: &str) -> impl Iterator<Item = &Entity> {
        self.store.on_layer(layer).map(|stored| &*stored.entity)
    }

    /// How many objects a layer has.
    pub fn count(&self, layer: &str) -> usize {
        self.store.count(layer)
    }

    // ── Saving ───────────────────────────────────────────────────────────

    /// The revision of what a file would hold: it changes with every edit,
    /// undo, redo and layer change. Only comparisons mean something.
    pub fn revision(&self) -> u64 {
        self.edits
    }

    /// Changes with every change to what the drawing holds: this user's
    /// edits, undo and redo, layer changes, and changes from outside
    /// (`apply_external`), which leave `revision` alone. Readers that redraw
    /// or follow the drawing key on this; saving keys on `revision`.
    pub fn generation(&self) -> u64 {
        self.generation
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
}
