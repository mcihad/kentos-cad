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

mod apply;
mod arch;
mod forget;
mod gis;
mod map;
mod op;
mod road;

pub(crate) use arch::ArchOp;
pub(crate) use gis::GisOp;
pub(crate) use map::MapOp;
pub use op::{LayerInfo, ServedBy, Temporal};
pub(crate) use op::{LayerPlace, Op, Served, SourceState};
pub(crate) use road::RoadOp;

use std::collections::VecDeque;

use crate::changes::Journal;
use crate::document::Document;

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

/// One undo step: its name (shown after undo and redo), its ops in order,
/// and its number in this session (from 1, given when it is recorded; the
/// journal's rows name their step by it, docs/adr/0253 §3).
#[derive(Clone, Debug)]
pub(crate) struct Step {
    pub(crate) label: String,
    pub(crate) ops: Vec<Op>,
    pub(crate) seq: u64,
}

impl Step {
    pub(crate) fn named(label: &str) -> Self {
        Step {
            label: label.to_owned(),
            ops: Vec::new(),
            seq: 0,
        }
    }
}

#[derive(Clone, Debug)]
struct OpenGroup {
    id: u64,
    step: Step,
    /// The journal's waiting rows when it opened: a cancel drops the later ones.
    calls: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct History {
    undo: VecDeque<Step>,
    redo: Vec<Step>,
    /// The open transaction (with its savepoints, it is one).
    pub(crate) pending: Option<Step>,
    group: Option<OpenGroup>,
    groups_begun: u64,
    /// The last step's number given (`Step::seq`).
    steps_given: u64,
    /// The open transaction was opened without a name (a command's step,
    /// `run_step`): it takes the first name an edit inside it gives.
    pub(crate) adopt: bool,
    /// The open steps' backups of what is no undo step (step.rs), innermost last.
    pub(crate) frames: Vec<crate::step::Frame>,
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

/// A place in the open step or group (`Document::group_savepoint`) to go back to with `Document::rollback_to`.
#[derive(Clone, Copy, Debug)]
pub struct Savepoint {
    /// The edits are in an open transaction (a command's step), not yet in the group.
    pending: bool,
    ops: usize,
    calls: usize,
}

/// The most rounds of linked texts and hatches after a step (docs/adr/0332 §2).
const HATCH_TEXT_ROUNDS: usize = 4;

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
            self.history.pending = Some(Step::named(label));
        } else {
            self.adopt_label(label);
        }
        let mark = self
            .history
            .pending
            .as_ref()
            .map_or(0, |step| step.ops.len());
        let calls = self.calls.waiting();
        let result = body(self);
        if result.is_err() {
            self.rollback(mark);
            self.calls.drop_waiting(calls);
        }
        if !outer && let Some(step) = self.history.pending.take() {
            if result.is_ok() && !step.ops.is_empty() {
                self.commit(step);
            } else if result.is_ok() && self.history.group.is_none() {
                // Nothing to undo; what was run inside (a command writing only
                // parts outside the undo history) is in the journal all the same.
                self.flush_calls(None, false);
            }
        }
        result
    }

    /// Gives the open transaction `label` when it was opened without a name
    /// and has none yet (a command's step takes its first edit's name).
    fn adopt_label(&mut self, label: &str) {
        if self.history.adopt
            && let Some(step) = &mut self.history.pending
            && step.label.is_empty()
            && !label.is_empty()
        {
            label.clone_into(&mut step.label);
            self.history.adopt = false;
        }
    }

    /// Opens a group: every step committed until [`Document::end_group`]
    /// becomes one undo step named `label`, across as many transactions as the
    /// work needs (a processing model runs several tools).
    pub fn begin_group(&mut self, label: &str) -> Group {
        // A group opened inside a command's unnamed step names that step: it
        // is the command's own name for what it does (docs/adr/0253 §1).
        self.adopt_label(label);
        if self.history.group.is_some() {
            return Group(None);
        }
        self.history.groups_begun += 1;
        let id = self.history.groups_begun;
        let calls = self.calls.waiting();
        self.history.group = Some(OpenGroup {
            id,
            step: Step::named(label),
            calls,
        });
        Group(Some(id))
    }

    /// Closes a group: what it gathered becomes one undo step.
    pub fn end_group(&mut self, group: Group) {
        if let Some(open) = self.take_group(group) {
            if !open.step.ops.is_empty() {
                self.commit(open.step);
            } else if self.history.pending.is_none() {
                // Nothing to undo; what ran inside is in the journal all the same.
                self.flush_calls(None, false);
            }
        }
    }

    /// Closes a group and reverts what it did. Nothing is recorded; undo, redo,
    /// the dirty flag and the revision stay as they were before the group
    /// (the generation moves: what the drawing shows changed back).
    pub fn cancel_group(&mut self, group: Group) {
        if let Some(open) = self.take_group(group) {
            let changed = !open.step.ops.is_empty();
            self.revert(&open.step.ops);
            self.calls.drop_waiting(open.calls);
            if changed {
                self.refresh_viewmarks();
                self.refresh_level_marks();
                self.refresh_derived_tags();
                self.refresh_zoning_tables();
                self.refresh_site();
                self.refresh_dimension_set_rows();
                self.mark_changed();
            }
        }
    }

    /// Marks the place of the edits now, in the open command step (a command run through the protocol writes inside
    /// one) or else in the open group: [`Document::rollback_to`] takes back what was written after it and leaves
    /// what was written before (a batch row that stops, docs/adr/0551 §1). None when neither is open.
    pub fn group_savepoint(&self) -> Option<Savepoint> {
        let calls = self.calls.waiting();
        if let Some(step) = &self.history.pending {
            return Some(Savepoint {
                pending: true,
                ops: step.ops.len(),
                calls,
            });
        }
        self.history.group.as_ref().map(|open| Savepoint {
            pending: false,
            ops: open.step.ops.len(),
            calls,
        })
    }

    /// Takes back what was written since `at`, newest first, and the command rows queued since; what was written
    /// before stays and the step or group goes on. Nothing is recorded (the generation moves, as for a taken-back
    /// group). Nothing happens when what holds the edits is not open any more, or holds less than at the mark.
    pub fn rollback_to(&mut self, at: Savepoint) {
        let undone = if at.pending {
            match &mut self.history.pending {
                Some(step) if at.ops <= step.ops.len() => step.ops.split_off(at.ops),
                _ => return,
            }
        } else {
            match &mut self.history.group {
                Some(open) if self.history.pending.is_none() && at.ops <= open.step.ops.len() => {
                    open.step.ops.split_off(at.ops)
                }
                _ => return,
            }
        };
        self.revert(&undone);
        self.calls.drop_waiting(at.calls);
        if !undone.is_empty() {
            self.refresh_viewmarks();
            self.refresh_level_marks();
            self.refresh_derived_tags();
            self.refresh_zoning_tables();
            self.refresh_site();
            self.refresh_dimension_set_rows();
            self.mark_changed();
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

    /// The current group's net object changes, without closing or recording it. Preview callers can
    /// inspect a command's settled result and cancel the group; only touched objects are visited.
    pub fn group_report(&self) -> Option<crate::StepReport> {
        self.history
            .group
            .as_ref()
            .map(|open| crate::step::report_of(&open.step.ops))
    }
    /// Whether a transaction or a group is open.
    pub fn is_busy(&self) -> bool {
        self.history.pending.is_some() || self.history.group.is_some()
    }

    pub fn can_undo(&self) -> bool {
        !self.history.undo.is_empty()
    }

    /// The undo steps' names, the last step first (the first one `undo` would revert).
    pub fn undo_labels(&self) -> Vec<String> {
        self.history
            .undo
            .iter()
            .rev()
            .map(|step| step.label.clone())
            .collect()
    }

    /// The drawing as it was `steps` undo steps back, a copy: this one, its history and its revision stay as they
    /// are (docs/adr/0576 §3). The name of the oldest step it undid says what it stands before; none when `steps`
    /// is 0 or the history holds fewer, or while a transaction or a group is open.
    pub fn past(&self, steps: usize) -> Option<(Document, String)> {
        if steps == 0 || self.is_busy() || self.history.undo.len() < steps {
            return None;
        }
        let mut copy = self.clone();
        let mut label = String::new();
        for _ in 0..steps {
            label = copy.undo()?;
        }
        Some((copy, label))
    }

    /// What the step `undo` would revert changed, without reverting it (docs/adr/0598 §4): its objects as they are
    /// now (`after`) and as undo would leave them (`before`). None: no step to undo.
    pub fn undo_report(&self) -> Option<crate::StepReport> {
        let step = self.history.undo.back()?;
        Some(crate::step::report_of(&step.ops))
    }

    pub fn can_redo(&self) -> bool {
        !self.history.redo.is_empty()
    }

    /// Reverts the last step and returns its name; `None` when there is none,
    /// or while a transaction or a group is open (the web would undo the step
    /// before the group then; docs/adr/0020).
    pub fn undo(&mut self) -> Option<String> {
        if self.is_busy() || self.sheet_history_problem(false).is_some() {
            return None;
        }
        let step = self.history.undo.pop_back()?;
        self.revert(&step.ops);
        let label = step.label.clone();
        self.calls
            .set_status(step.seq, kentos_contracts::CallStatus::Undone);
        self.history.redo.push(step);
        self.refresh_viewmarks();
        self.refresh_level_marks();
        self.refresh_derived_tags();
        self.refresh_zoning_tables();
        self.refresh_site();
        self.refresh_dimension_set_rows();
        self.mark_edited();
        Some(label)
    }

    /// Applies the last undone step again and returns its name.
    pub fn redo(&mut self) -> Option<String> {
        if self.is_busy() || self.sheet_history_problem(true).is_some() {
            return None;
        }
        let step = self.history.redo.pop()?;
        for op in &step.ops {
            self.apply(op);
        }
        let label = step.label.clone();
        self.calls
            .set_status(step.seq, kentos_contracts::CallStatus::Done);
        self.history.undo.push_back(step);
        self.refresh_viewmarks();
        self.refresh_level_marks();
        self.refresh_derived_tags();
        self.refresh_zoning_tables();
        self.refresh_site();
        self.refresh_dimension_set_rows();
        self.mark_edited();
        Some(label)
    }

    /// Why undo/redo would overwrite a later independent paper edit. Checked
    /// before popping a step or applying any model operation (ADR 0590).
    pub fn sheet_history_problem(&self, redo: bool) -> Option<&'static str> {
        if self.is_busy() {
            return None;
        }
        let step = if redo {
            self.history.redo.last()
        } else {
            self.history.undo.back()
        }?;
        let edge = if redo {
            step.ops.iter().find_map(Op::sheet_states)
        } else {
            step.ops.iter().rev().find_map(Op::sheet_states)
        }?;
        let expected = if redo { edge.0 } else { edge.1 };
        let same = match (self.sheets.as_deref(), expected) {
            (Some(now), Some(expected)) => std::ptr::eq(now, expected) || now == expected,
            (None, None) => true,
            _ => false,
        };
        (!same).then_some(
            "Pafta ayrıca değiştirildi. Önce sonraki pafta düzenlemesini geri alın. Model ve pafta birlikte değişeceği için işlem durduruldu; veri değişmedi."
        )
    }

    /// Applies `ops` as one change: into the open transaction, or as a step of its own.
    pub(crate) fn record(&mut self, ops: Vec<Op>, label: &str) {
        if ops.is_empty() {
            return;
        }
        for op in &ops {
            self.apply(op);
        }
        self.adopt_label(label);
        match &mut self.history.pending {
            Some(step) => step.ops.extend(ops),
            None => self.commit(Step {
                ops,
                ..Step::named(label)
            }),
        }
    }

    /// A finished step: into the open group, or onto the undo history as a
    /// new edit, with what keeps its linked texts with their objects
    /// (linked.rs, docs/adr/0175 §4) and its associative hatches with theirs
    /// (hatch_ties.rs, docs/adr/0186 §6).
    fn commit(&mut self, mut step: Step) {
        if self.history.group.is_some() {
            self.fold_into_group(step);
            return;
        }
        self.follow_into(&mut step.ops);
        self.push_step(step);
    }

    /// A step into the open group.
    pub(crate) fn fold_into_group(&mut self, step: Step) {
        if let Some(open) = &mut self.history.group {
            open.step.ops.extend(step.ops);
            // The drawing changed all the same, and what shows it follows each
            // step (a large import fills in as it goes, as the web's listeners
            // hear a group's changes as they come); the edit is recorded, and
            // the revision moves, when the group ends.
            self.mark_changed();
        }
    }

    /// Whether a group is open.
    pub(crate) fn group_open(&self) -> bool {
        self.history.group.is_some()
    }

    /// What keeps the linked texts with their objects (linked.rs,
    /// docs/adr/0175 §4), the associative hatches with theirs (hatch_ties.rs,
    /// docs/adr/0186 §6) and the dimensions with the places their points
    /// name (dimension_ties.rs, docs/adr/0249 §3) and the records of a layer
    /// whose form enforces its rules complete (form_follow.rs, docs/adr/0402
    /// §4) after `ops`, applied and joined to them.
    pub(crate) fn follow_into(&mut self, ops: &mut Vec<Op>) {
        // First: the children of the parents the step deletes (docs/adr/0405 §2), so the follows
        // below see them gone or unlinked.
        let related = self.follow_relations(ops);
        ops.extend(related);
        // Then: the openings with their walls (docs/adr/0341 §3), before the texts linked to them.
        let openings = self.follow_openings(ops);
        ops.extend(openings);
        // Then: the kitchen runs with their walls and objects (docs/adr/0727 §5), before what follows their objects
        // (tags, hatches, dimensions, schedules).
        let casework = self.follow_casework(ops);
        ops.extend(casework);
        // Settle grid-driven columns before a space reads their physical
        // footprints (ADR 0672); tags and schedules see both in the same step.
        let columns = self.follow_columns(ops);
        ops.extend(columns);
        let spaces = self.follow_spaces(ops);
        ops.extend(spaces);
        // Then: the bound slabs' outlines with their walls and spaces (docs/adr/0771 §4), before their edges.
        let bound = self.follow_bound_slabs(ops);
        ops.extend(bound);
        // Then: the slabs' edges with their walls (docs/adr/0722 §3).
        let slabs = self.follow_slabs(ops);
        ops.extend(slabs);
        // Then: the Kots and ramps bound to a level with it (docs/adr/0721 §2, docs/adr/0723 §2), before the tags that
        // read a ramp's elevations.
        let levelled = self.follow_level_marks(ops);
        ops.extend(levelled);
        // Then: the tags with their objects (docs/adr/0344 §1).
        let tags = self.follow_tags(ops);
        ops.extend(tags);
        // Then: the view marks with their targets (docs/adr/0345 §2).
        let marks = self.follow_viewmarks(ops);
        ops.extend(marks);
        // Then: a hatch's ring drawn from its exact boundary, or the boundary dropped (docs/adr/0328 §1).
        let settled = self.settle_boundaries(ops);
        ops.extend(settled);
        let follow = self.follow_links(ops);
        ops.extend(follow);
        // After the linked texts: a text a hatch leaves open may have moved with its object.
        let mut ties = self.follow_hatches(ops);
        ops.extend(ties.iter().cloned());
        // A hatch cut again moves the texts linked to it, which may be left open by other hatches (docs/adr/0332 §2):
        // a few rounds, each from the last one's changes; a hatch's own label is never left open, so they settle.
        for _ in 0..HATCH_TEXT_ROUNDS {
            if ties.is_empty() {
                break;
            }
            let texts = self.follow_links(&ties);
            if texts.is_empty() {
                break;
            }
            ops.extend(texts.iter().cloned());
            ties = self.follow_hatches(&texts);
            ops.extend(ties.iter().cloned());
        }
        // After the hatches: an array's items and path (docs/adr/0300 §3), before the dimensions tied to the items.
        let arrays = self.follow_arrays(ops);
        ops.extend(arrays);
        // After the arrays: the leaders' tips and content blocks (docs/adr/0326 §2, §3), before the dimensions tied to them.
        let leaders = self.follow_leaders(ops);
        ops.extend(leaders);
        // Recipes see the settled real wall faces and opening jambs, including new/rehosted openings.
        let sets = self.follow_dimension_sets(ops);
        ops.extend(sets);
        // After the hatches: a dimension follows the places its points name.
        let relocated = std::mem::take(&mut self.relocated);
        let ends = std::mem::take(&mut self.relocated_ends);
        let dims = self.follow_dimensions(ops, &relocated, &ends);
        ops.extend(dims);
        // After the objects settled: the texts' fields read their values (docs/adr/0325 §2).
        let fields = self.follow_fields(ops);
        ops.extend(fields);
        // After the fields: the tables' schedules, fields and formulas (docs/adr/0327).
        // Not on a dry run's copy nor in a preview (docs/adr/8051 §3): the step works them out.
        let tables = if self.works_tables {
            self.follow_tables(ops)
        } else {
            Vec::new()
        };
        ops.extend(tables);
        // And the joinery list's block with its types and openings (docs/adr/8038 §6).
        let joinery = self.follow_joinery(ops);
        ops.extend(joinery);
        // And the site plan's setback line (docs/adr/8032) on its parcel, its storeys and its footprint.
        let site = self.follow_site(ops);
        ops.extend(site);
        // Then: the style fields the step set apart by hand (docs/adr/0322 §5).
        let marks = self.mark_overrides(ops);
        ops.extend(marks);
        // Last: the records the step left on a layer whose form enforces its rules, their geometry
        // settled (docs/adr/0402 §4).
        let forms = self.follow_forms(ops);
        ops.extend(forms);
        // After everything: the constraints of the objects the step deleted go with them (docs/adr/0334 §1).
        let constraints = self.follow_constraints(ops);
        ops.extend(constraints);
    }

    /// A finished, followed step onto the undo history as a new edit; its
    /// number, which the journal's waiting rows take (calls.rs).
    pub(crate) fn push_step(&mut self, mut step: Step) -> u64 {
        self.history.steps_given += 1;
        step.seq = self.history.steps_given;
        let seq = step.seq;
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
        self.flush_calls(Some(seq), true);
        seq
    }

    /// Takes the last step off the undo history, keeping what it did: a
    /// reading copy hands its job's write on (patch.rs).
    pub(crate) fn take_last_step(&mut self) -> Option<Step> {
        self.history.undo.pop_back()
    }

    /// The last step on the undo history, its name and ops (the journal's
    /// row of an edit made without a command, calls.rs).
    pub(crate) fn last_step(&self) -> Option<&Step> {
        self.history.undo.back()
    }

    /// Reverts the ops the open transaction applied after `mark` and forgets
    /// them; how many there were.
    pub(crate) fn rollback(&mut self, mark: usize) -> usize {
        let undone = match &mut self.history.pending {
            Some(step) => step.ops.split_off(mark.min(step.ops.len())),
            None => return 0,
        };
        self.revert(&undone);
        undone.len()
    }
}
