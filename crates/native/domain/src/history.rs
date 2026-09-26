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
//! Only undoable data are ops: objects and layer styles. Layer visibility,
//! lock, names and fold are changed at once and never recorded (web).

use std::collections::{HashSet, VecDeque};

use kentos_contracts::LayerStyle;

use crate::changes::Journal;
use crate::document::Document;
use crate::identity::{Slot, Uuid};
use crate::store::Stored;

/// Undo steps kept; an older step is dropped when a new one comes (web; TODOS.md TX-06).
pub const UNDO_LIMIT: usize = 200;

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
    /// the dirty flag and the revision stay as they were before the group.
    pub fn cancel_group(&mut self, group: Group) {
        if let Some(open) = self.take_group(group) {
            self.revert(&open.step.ops);
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

    /// A finished step: into the open group, or onto the undo history as a new edit.
    fn commit(&mut self, step: Step) {
        if let Some(open) = &mut self.history.group {
            open.step.ops.extend(step.ops);
            return;
        }
        self.history.undo.push_back(step);
        if self.history.undo.len() > UNDO_LIMIT {
            self.history.undo.pop_front();
        }
        self.history.redo.clear();
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
                Op::LayerStyle { .. } => false,
            })
        };
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
        };
        let objects = self.store.len();
        self.history.journal.touch(touched, objects);
    }
}
