//! Transactions, groups, undo and redo (docs/adr/0003), as the web's
//! `CadDocument` has them (apps/web/src/model/document.ts):
//!
//! - every edit is applied at once and recorded as ops; outside a transaction
//!   it is one undo step;
//! - `transact` gathers the edits of its body into one step, all or nothing:
//!   an error reverts what the body did (newest first) and records nothing; a
//!   transaction inside a transaction is a savepoint of the outer one;
//! - a group gathers every step committed until it ends into one step, and its
//!   cancel reverts them;
//! - the undo history keeps the last 200 steps; a new step clears redo.
//!
//! Only undoable data are ops: objects, layer styles, a layer's addition and
//! removal (with the objects on it), the active layer an addition set, and
//! the block definitions (docs/adr/0144).
//! Layer visibility, lock, names and fold, and a layer made active by hand,
//! are changed at once and never recorded (web).

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use kentos_contracts::{BlockDefinition, BlockId, Entity, LayerNode, LayerStyle};

use crate::changes::Journal;
use crate::document::Document;
use crate::identity::{Slot, Uuid};
use crate::store::Stored;

/// Undo steps kept; an older step is dropped when a new one comes (web; TODOS.md TX-06).
pub const UNDO_LIMIT: usize = 200;

/// Steps holding at least this many operations are let go on another thread.
const FREE_APART: usize = 20_000;

/// Drops `value`, holding `ops` operations: a large one on another thread, so
/// an edit after undoing a run of hundreds of thousands of objects does not
/// wait for them to be freed (TODOS.md PERF-08).
fn free_apart<T: Send + 'static>(value: T, ops: usize) {
    if ops >= FREE_APART {
        std::thread::spawn(move || drop(value));
    } else {
        drop(value);
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Op {
    Add(Stored),
    Remove(Stored),
    /// The same object (slot and persistent id) before and after.
    Update {
        before: Stored,
        after: Stored,
    },
    /// Boxed: a style is large and most ops are objects.
    LayerStyle {
        layer: String,
        before: Box<LayerStyle>,
        after: Box<LayerStyle>,
    },
    /// A layer or a group taken out of the tree with everything under it
    /// (`Document::remove_layer`), and its inverse, which puts it back; a
    /// new one (`Document::add_layer`) is a `LayerAdd`.
    LayerRemove(Box<LayerPlace>),
    LayerAdd(Box<LayerPlace>),
    /// The active layer an addition set (`add_layer` activating): applied
    /// only while the active layer is still `before`, so undo and redo leave
    /// a layer made active by hand in between as it is (the web's `layerActive`).
    LayerActive {
        before: String,
        after: String,
    },
    /// A block definition added at `index` of the list (blocks.rs), and its
    /// inverse, which takes it out; shared, so a step keeps no copy.
    BlockAdd {
        index: usize,
        block: Arc<BlockDefinition>,
    },
    BlockRemove {
        index: usize,
        block: Arc<BlockDefinition>,
    },
    /// The same definition (its id) before and after.
    BlockUpdate {
        before: Arc<BlockDefinition>,
        after: Arc<BlockDefinition>,
    },
}

/// A tree node as a step keeps it (children, flags, style), and where it
/// was: its group (none: the top of the tree) and its place there.
#[derive(Clone, Debug)]
pub(crate) struct LayerPlace {
    pub(crate) node: LayerNode,
    pub(crate) parent: Option<String>,
    pub(crate) index: usize,
}

impl Op {
    /// Whether the op changes the layer tree rather than an object.
    fn is_tree(&self) -> bool {
        matches!(self, Op::LayerRemove(_) | Op::LayerAdd(_))
    }
}

impl Op {
    fn inverse(&self) -> Op {
        match self {
            Op::Add(stored) => Op::Remove(stored.clone()),
            Op::Remove(stored) => Op::Add(stored.clone()),
            Op::Update { before, after } => Op::Update {
                before: after.clone(),
                after: before.clone(),
            },
            Op::LayerStyle {
                layer,
                before,
                after,
            } => Op::LayerStyle {
                layer: layer.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Op::LayerRemove(place) => Op::LayerAdd(place.clone()),
            Op::LayerAdd(place) => Op::LayerRemove(place.clone()),
            Op::LayerActive { before, after } => Op::LayerActive {
                before: after.clone(),
                after: before.clone(),
            },
            Op::BlockAdd { index, block } => Op::BlockRemove {
                index: *index,
                block: block.clone(),
            },
            Op::BlockRemove { index, block } => Op::BlockAdd {
                index: *index,
                block: block.clone(),
            },
            Op::BlockUpdate { before, after } => Op::BlockUpdate {
                before: after.clone(),
                after: before.clone(),
            },
        }
    }
}

/// One undo step: its name (shown after undo and redo) and its ops in order.
#[derive(Clone, Debug)]
pub(crate) struct Step {
    label: String,
    ops: Vec<Op>,
}

#[derive(Clone, Debug)]
struct OpenGroup {
    id: u64,
    step: Step,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct History {
    undo: VecDeque<Step>,
    redo: Vec<Step>,
    /// The open transaction (with its savepoints, it is one).
    pending: Option<Step>,
    group: Option<OpenGroup>,
    groups_begun: u64,
    /// The slots every applied op touched, for readers that follow the
    /// document (`changes.rs`); kept with the ops since every change passes here.
    pub(crate) journal: Journal,
}

/// An open group (`Document::begin_group`); hand it back to `end_group` or
/// `cancel_group`. A group begun while another is open joins that one: its
/// handle ends and cancels nothing.
#[must_use = "a group stays open until it is ended or cancelled"]
#[derive(Debug)]
pub struct Group(Option<u64>);

impl Document {
    /// Runs `body` as one undo step named `label`, all or nothing.
    ///
    /// If `body` returns an error, what it changed is reverted, newest first,
    /// and the error goes on to the caller: nothing is recorded, the dirty flag
    /// and the revision stay as they were, undo and redo are untouched. Inside
    /// another transaction this one is a savepoint: its error reverts only its
    /// own edits, and the outer one reverts everything if the error reaches it.
    /// Slots given out meanwhile are not given again (docs/adr/0003).
    ///
    /// A panic in `body` leaves the transaction open; drop the document then.
    pub fn transact<T, E>(
        &mut self,
        label: &str,
        body: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        let outer = self.history.pending.is_some();
        if !outer {
            self.history.pending = Some(Step {
                label: label.to_owned(),
                ops: Vec::new(),
            });
        }
        let mark = self
            .history
            .pending
            .as_ref()
            .map_or(0, |step| step.ops.len());
        let result = body(self);
        if result.is_err() {
            self.rollback(mark);
        }
        if !outer
            && let Some(step) = self.history.pending.take()
            && result.is_ok()
            && !step.ops.is_empty()
        {
            self.commit(step);
        }
        result
    }

    /// Opens a group: every step committed until [`Document::end_group`]
    /// becomes one undo step named `label`, across as many transactions as the
    /// work needs (a processing model runs several tools).
    pub fn begin_group(&mut self, label: &str) -> Group {
        if self.history.group.is_some() {
            return Group(None);
        }
        self.history.groups_begun += 1;
        let id = self.history.groups_begun;
        self.history.group = Some(OpenGroup {
            id,
            step: Step {
                label: label.to_owned(),
                ops: Vec::new(),
            },
        });
        Group(Some(id))
    }

    /// Closes a group: what it gathered becomes one undo step.
    pub fn end_group(&mut self, group: Group) {
        if let Some(open) = self.take_group(group)
            && !open.step.ops.is_empty()
        {
            self.commit(open.step);
        }
    }

    /// Closes a group and reverts what it did. Nothing is recorded; undo, redo,
    /// the dirty flag and the revision stay as they were before the group
    /// (the generation moves: what the drawing shows changed back).
    pub fn cancel_group(&mut self, group: Group) {
        if let Some(open) = self.take_group(group) {
            let changed = !open.step.ops.is_empty();
            self.revert(&open.step.ops);
            if changed {
                self.mark_changed();
            }
        }
    }

    fn take_group(&mut self, group: Group) -> Option<OpenGroup> {
        let id = group.0?;
        if self
            .history
            .group
            .as_ref()
            .is_some_and(|open| open.id == id)
        {
            return self.history.group.take();
        }
        None
    }

    /// How many changes the open group has gathered; none without one. Inside
    /// a group nothing is an edit before it ends (the revision and the dirty
    /// flag stay; only the generation follows what is shown), so this says
    /// whether ending it will record a step.
    pub fn group_changes(&self) -> usize {
        self.history
            .group
            .as_ref()
            .map_or(0, |open| open.step.ops.len())
    }

    /// Whether a transaction or a group is open.
    pub fn is_busy(&self) -> bool {
        self.history.pending.is_some() || self.history.group.is_some()
    }

    pub fn can_undo(&self) -> bool {
        !self.history.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.history.redo.is_empty()
    }

    /// Reverts the last step and returns its name; `None` when there is none,
    /// or while a transaction or a group is open (the web would undo the step
    /// before the group then; docs/adr/0020).
    pub fn undo(&mut self) -> Option<String> {
        if self.is_busy() {
            return None;
        }
        let step = self.history.undo.pop_back()?;
        self.revert(&step.ops);
        let label = step.label.clone();
        self.history.redo.push(step);
        self.mark_edited();
        Some(label)
    }

    /// Applies the last undone step again and returns its name.
    pub fn redo(&mut self) -> Option<String> {
        if self.is_busy() {
            return None;
        }
        let step = self.history.redo.pop()?;
        for op in &step.ops {
            self.apply(op);
        }
        let label = step.label.clone();
        self.history.undo.push_back(step);
        self.mark_edited();
        Some(label)
    }

    /// Applies `ops` as one change: into the open transaction, or as a step of its own.
    pub(crate) fn record(&mut self, ops: Vec<Op>, label: &str) {
        if ops.is_empty() {
            return;
        }
        for op in &ops {
            self.apply(op);
        }
        match &mut self.history.pending {
            Some(step) => step.ops.extend(ops),
            None => self.commit(Step {
                label: label.to_owned(),
                ops,
            }),
        }
    }

    /// A finished step: into the open group, or onto the undo history as a
    /// new edit, with what keeps its linked texts with their objects
    /// (linked.rs, docs/adr/0175 §4).
    fn commit(&mut self, mut step: Step) {
        if let Some(open) = &mut self.history.group {
            open.step.ops.extend(step.ops);
            // The drawing changed all the same, and what shows it follows each
            // step (a large import fills in as it goes, as the web's listeners
            // hear a group's changes as they come); the edit is recorded, and
            // the revision moves, when the group ends.
            self.mark_changed();
            return;
        }
        let follow = self.follow_links(&step.ops);
        step.ops.extend(follow);
        self.history.undo.push_back(step);
        if self.history.undo.len() > UNDO_LIMIT
            && let Some(oldest) = self.history.undo.pop_front()
        {
            let ops = oldest.ops.len();
            free_apart(oldest, ops);
        }
        let redo = std::mem::take(&mut self.history.redo);
        let ops = redo.iter().map(|s| s.ops.len()).sum();
        free_apart(redo, ops);
        self.mark_edited();
    }

    /// Reverts the ops the open transaction applied after `mark` and forgets them.
    fn rollback(&mut self, mark: usize) {
        let undone = match &mut self.history.pending {
            Some(step) => step.ops.split_off(mark.min(step.ops.len())),
            None => return,
        };
        self.revert(&undone);
    }

    /// Applies the inverse of `ops`, newest first.
    fn revert(&mut self, ops: &[Op]) {
        for op in ops.iter().rev() {
            self.apply(&op.inverse());
        }
    }

    /// Applies one op outside the history (a change from outside, external.rs).
    pub(crate) fn apply_op(&mut self, op: &Op) {
        self.apply(op);
    }

    /// Drops the undo and redo steps that touch any of these objects, named
    /// by slot or by persistent id (an object deleted here that someone else
    /// brought back sits in a new slot under the same id). Web: `forgetHistoryOf`.
    pub(crate) fn forget_history_of(&mut self, slots: &HashSet<Slot>, uids: &HashSet<Uuid>) {
        if slots.is_empty() && uids.is_empty() {
            return;
        }
        let hit = |s: &Stored| slots.contains(&s.slot()) || uids.contains(&s.uid);
        let touches = |step: &Step| {
            step.ops.iter().any(|op| match op {
                Op::Add(s) | Op::Remove(s) => hit(s),
                Op::Update { before, .. } => hit(before),
                Op::LayerStyle { .. }
                | Op::LayerRemove(_)
                | Op::LayerAdd(_)
                | Op::LayerActive { .. }
                | Op::BlockAdd { .. }
                | Op::BlockRemove { .. }
                | Op::BlockUpdate { .. } => false,
            })
        };
        self.history.undo.retain(|step| !touches(step));
        self.history.redo.retain(|step| !touches(step));
    }

    /// Drops the undo and redo steps that add or remove a node holding one
    /// of these layers, as the step recorded it or as it is in the tree now
    /// (a layer put into an added group since): another editor put objects
    /// there, and undoing the step would take the layer from under them
    /// (the web's `applyExternal`).
    pub(crate) fn forget_layer_history(&mut self, layers: &HashSet<String>) {
        if layers.is_empty() {
            return;
        }
        let tree = &self.layers;
        let holds = |step: &Step| {
            step.ops.iter().any(|op| match op {
                Op::LayerRemove(place) | Op::LayerAdd(place) => {
                    let mut ids = Vec::new();
                    node_ids(&place.node, &mut ids);
                    if let Some(now) = tree.get(&place.node.id) {
                        node_ids(now, &mut ids);
                    }
                    ids.iter().any(|id| layers.contains(id))
                }
                _ => false,
            })
        };
        let undo: VecDeque<Step> = self
            .history
            .undo
            .iter()
            .filter(|step| !holds(step))
            .cloned()
            .collect();
        let redo: Vec<Step> = self
            .history
            .redo
            .iter()
            .filter(|step| !holds(step))
            .cloned()
            .collect();
        self.history.undo = undo;
        self.history.redo = redo;
    }

    /// Drops the undo and redo steps that change one of the `changed`
    /// definitions (someone else's now), or that place or hold one of the
    /// `gone` ones: undoing them would revert that change, or put back an
    /// insert of a block the drawing no longer has (the web's
    /// `forgetBlockHistory`, docs/adr/0144 §5).
    pub(crate) fn forget_block_history(&mut self, changed: &HashSet<BlockId>, gone: &HashSet<BlockId>) {
        if changed.is_empty() {
            return;
        }
        let places = |e: &Entity| matches!(e, Entity::Insert(i) if gone.contains(&i.block));
        let holds = |b: &BlockDefinition| !gone.is_empty() && b.entities.iter().any(places);
        let touches = |step: &Step| {
            step.ops.iter().any(|op| match op {
                Op::BlockUpdate { before, after } => {
                    changed.contains(&after.id) || holds(before) || holds(after)
                }
                Op::BlockAdd { block, .. } | Op::BlockRemove { block, .. } => {
                    changed.contains(&block.id) || holds(block)
                }
                Op::Update { before, after } => places(&before.entity) || places(&after.entity),
                Op::Add(s) | Op::Remove(s) => places(&s.entity),
                _ => false,
            })
        };
        self.history.undo.retain(|step| !touches(step));
        self.history.redo.retain(|step| !touches(step));
    }

    /// Drops the undo and redo steps that change the layer tree: another
    /// editor's tree came, and a removed layer's place may not fit it (web).
    pub(crate) fn forget_tree_history(&mut self) {
        let touches = |step: &Step| step.ops.iter().any(Op::is_tree);
        self.history.undo.retain(|step| !touches(step));
        self.history.redo.retain(|step| !touches(step));
    }

    fn apply(&mut self, op: &Op) {
        let touched = match op {
            Op::Add(stored) => {
                self.store.put(stored.clone());
                stored.slot()
            }
            Op::Remove(stored) => {
                self.store.remove(stored.slot());
                stored.slot()
            }
            Op::Update { after, .. } => {
                self.store.put(after.clone());
                after.slot()
            }
            Op::LayerStyle { layer, after, .. } => {
                self.layers.replace_style(layer, after);
                return;
            }
            // A node still holding objects stays: taking it away would leave
            // them on no layer (its step's own objects go before it; another
            // editor's objects drop the step, `forget_layer_history`).
            Op::LayerRemove(place) => {
                if !self.holds_objects(&place.node.id) {
                    self.layers.detach(&place.node.id);
                }
                return;
            }
            Op::LayerAdd(place) => {
                self.layers
                    .attach(&place.node, place.parent.as_deref(), place.index);
                return;
            }
            Op::LayerActive { before, after } => {
                if self.layers.active() == before {
                    self.layers.set_active(after);
                }
                return;
            }
            // Not objects: a reader compares the definitions themselves (blocks.rs).
            Op::BlockAdd { .. } | Op::BlockRemove { .. } | Op::BlockUpdate { .. } => {
                self.apply_block_op(op);
                return;
            }
        };
        let objects = self.store.len();
        self.history.journal.touch(touched, objects);
    }
}

/// The ids of a node and of every node under it.
fn node_ids(node: &LayerNode, out: &mut Vec<String>) {
    out.push(node.id.clone());
    for child in &node.children {
        node_ids(child, out);
    }
}
