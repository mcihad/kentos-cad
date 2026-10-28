//! A command's step (docs/adr/0253 §1): one write, all or nothing, with
//! what it changed reported.
//!
//! [`Document::run_step`] runs a body as one undo step, like `transact`,
//! and more:
//!
//! - what is no undo step is taken back too when the body or the seal
//!   refuses: the drawing's name, the project's settings, its style library,
//!   its sheet book, the layers' visibility, lock, snapping, names and fold,
//!   the active layer, and the revision and the dirty flag; each is kept the
//!   first time the step writes it (a frame);
//! - the linked texts, associative hatches and dimensions follow inside the
//!   step, before the seal, so the seal sees the step as it will be recorded;
//! - the seal sees a [`StepReport`] (objects added, changed and removed with
//!   their states before and after, the layers and blocks that changed, the
//!   project's parts that changed) and may still refuse; it gives the
//!   journal its row ([`Sealed`]);
//! - the step takes the name of the first edit inside it.
//!
//! Inside an open transaction or step it is a savepoint: its refusal takes
//! back only its own writes, and the outer one follows and records. Inside a
//! group without a transaction it follows and records into the group.

use std::collections::HashMap;
use std::sync::Arc;

use kentos_contracts::{
    BlockDefinition, BlockId, CallRecord, Entity, ProjectPart, ProjectSettings, ProjectSheets,
    ProjectStyles, RequestRecord,
};

use crate::document::Document;
use crate::history::{Op, Step};
use crate::identity::{Slot, Uuid};
use crate::layers::LayerFlags;
use crate::store::Stored;

/// What a step kept of what is no undo step, the first time it wrote it.
#[derive(Clone, Debug)]
pub(crate) struct Frame {
    name: Option<String>,
    settings: Option<ProjectSettings>,
    styles: Option<ProjectStyles>,
    sheets: Option<Option<Arc<ProjectSheets>>>,
    flags: Option<LayerFlags>,
    /// The active layer when the step began.
    active: String,
    edits: u64,
    dirty: bool,
    /// The open transaction's ops when the step began.
    mark: usize,
    /// The journal's waiting rows when the step began.
    calls: usize,
    /// Whether this step opened the transaction.
    opened: bool,
    /// Derived offset cache writes are reported in their order but never enter model undo.
    dimension_cache: Vec<(usize, Stored, Stored)>,
}

impl Frame {
    /// What `inner` kept that this frame did not: an inner step that
    /// succeeded leaves its writes to this one to take back.
    fn absorb(&mut self, inner: Frame) {
        self.dimension_cache.extend(inner.dimension_cache);
        if self.name.is_none() {
            self.name = inner.name;
        }
        if self.settings.is_none() {
            self.settings = inner.settings;
        }
        if self.styles.is_none() {
            self.styles = inner.styles;
        }
        if self.sheets.is_none() {
            self.sheets = inner.sheets;
        }
        if self.flags.is_none() {
            self.flags = inner.flags;
        }
    }
}

/// An object a step added, changed or removed: its first state before the
/// step and its last after (none: not there).
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectChange {
    pub uid: Uuid,
    pub slot: Slot,
    pub before: Option<Arc<Entity>>,
    pub after: Option<Arc<Entity>>,
}

impl ObjectChange {
    pub fn added(&self) -> bool {
        self.before.is_none() && self.after.is_some()
    }

    pub fn removed(&self) -> bool {
        self.before.is_some() && self.after.is_none()
    }

    pub fn changed(&self) -> bool {
        matches!((&self.before, &self.after), (Some(b), Some(a)) if b != a)
    }
}

/// Ids added, changed and removed, in the order the step first touched them.
#[derive(Clone, Debug, PartialEq)]
pub struct IdChanges<T> {
    pub added: Vec<T>,
    pub changed: Vec<T>,
    pub removed: Vec<T>,
}

impl<T> Default for IdChanges<T> {
    fn default() -> Self {
        Self {
            added: Vec::new(),
            changed: Vec::new(),
            removed: Vec::new(),
        }
    }
}

impl<T: PartialEq + Clone> IdChanges<T> {
    fn add(&mut self, id: T) {
        if let Some(i) = self.removed.iter().position(|x| *x == id) {
            // Taken out and put back: changed at most.
            self.removed.remove(i);
            if !self.changed.contains(&id) {
                self.changed.push(id);
            }
        } else if !self.added.contains(&id) {
            self.added.push(id);
        }
    }

    fn change(&mut self, id: T) {
        if !self.added.contains(&id) && !self.changed.contains(&id) {
            self.changed.push(id);
        }
    }

    fn remove(&mut self, id: T) {
        if let Some(i) = self.added.iter().position(|x| *x == id) {
            // Made and taken out in one step: nothing.
            self.added.remove(i);
            return;
        }
        self.changed.retain(|x| *x != id);
        if !self.removed.contains(&id) {
            self.removed.push(id);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.changed.is_empty() && self.removed.is_empty()
    }
}

/// What a step changed (docs/adr/0253 §1, §5).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepReport {
    /// The objects it added, changed or removed (an object changed back to
    /// what it was is listed too: `ObjectChange::changed` says).
    pub objects: Vec<ObjectChange>,
    /// Layers and groups by id: added (with everything under them), changed
    /// (their style, fields, service, time, filter, card, composition or
    /// place), removed.
    pub layers: IdChanges<String>,
    pub blocks: IdChanges<BlockId>,
    /// The associative arrays it added, changed or removed (docs/adr/0300).
    pub arrays: IdChanges<kentos_contracts::ArrayId>,
    pub dimension_sets: IdChanges<kentos_contracts::DimensionSetId>,
    /// Forbidden user writes found before source reconciliation; checked by the atomic seal.
    pub dimension_set_violations: Vec<Uuid>,
    /// Trusted derived row offsets outside undo; the settings that cause them are reported separately.
    pub dimension_set_cached: Vec<Uuid>,
    /// Whether it changed the constraints or the parameter table (docs/adr/0334).
    pub constraints: bool,
    /// The block definitions it added, changed or removed: their first
    /// state before the step and their last after.
    pub block_states: Vec<BlockChange>,
    /// The project's parts that it changed.
    pub parts: Vec<ProjectPart>,
    /// Parts fully restored by this step's undo operations (ADR 0590).
    pub undo_parts: Vec<ProjectPart>,
    /// The step's name, once an edit gave it one.
    pub label: Option<String>,
    /// The undo step it became (none: inside a transaction or a group, or no
    /// undo step at all); known once the step is recorded.
    pub step: Option<u64>,
    /// The journal's row the seal gave it, once queued.
    pub call: Option<u64>,
    /// Whether it was the outermost step: it followed its objects and is
    /// recorded by itself (a savepoint's outer step does both).
    pub outermost: bool,
}

/// A block definition a step added, changed or removed.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockChange {
    pub id: BlockId,
    pub before: Option<Arc<BlockDefinition>>,
    pub after: Option<Arc<BlockDefinition>>,
}

/// What a seal gives the journal (calls.rs): the command's row and, for a
/// request, what the ledger keeps of it.
#[derive(Clone, Debug)]
pub struct Sealed {
    pub row: CallRecord,
    pub request: Option<RequestRecord>,
}

/// An object a step wrote, as it was before and after (none: not there).
pub type Written = (Option<Arc<Entity>>, Option<Arc<Entity>>);

impl Document {
    /// Runs `body` as one step (see the module), then `seal` over what it
    /// changed. Either refusing takes everything back, the parts outside the
    /// undo history too, and the refusal goes on to the caller; otherwise
    /// the step is recorded (or, inside a transaction, joins it) and its
    /// report comes back with the body's value.
    ///
    /// A panic in `body` or `seal` leaves the step open; drop the document then.
    pub fn run_step<T, E>(
        &mut self,
        body: impl FnOnce(&mut Self) -> Result<T, E>,
        seal: impl FnOnce(&Self, &StepReport, &T) -> Result<Option<Sealed>, E>,
    ) -> Result<(T, StepReport), E> {
        let opened = self.history.pending.is_none();
        if opened {
            self.history.pending = Some(Step::named(""));
            self.history.adopt = true;
        }
        let mark = self.history.pending.as_ref().map_or(0, |s| s.ops.len());
        let frame = Frame {
            name: None,
            settings: None,
            styles: None,
            sheets: None,
            flags: None,
            active: self.layers.active().to_owned(),
            edits: self.edits,
            dirty: self.dirty,
            mark,
            calls: self.calls.waiting(),
            opened,
            dimension_cache: Vec::new(),
        };
        self.history.frames.push(frame);
        let value = match body(self) {
            Ok(value) => value,
            Err(e) => {
                self.unwind();
                return Err(e);
            }
        };
        if opened && let Some(mut step) = self.history.pending.take() {
            self.follow_into(&mut step.ops);
            self.history.pending = Some(step);
        }
        let mut report = self.report_of_frame();
        report.outermost = opened;
        let sealed = match seal(self, &report, &value) {
            Ok(sealed) => sealed,
            Err(e) => {
                self.unwind();
                return Err(e);
            }
        };
        let Some(frame) = self.history.frames.pop() else {
            return Ok((value, report));
        };
        if let Some(sealed) = sealed {
            report.call = Some(self.calls.queue(sealed.row, sealed.request));
        }
        if opened {
            self.history.adopt = false;
            if let Some(step) = self.history.pending.take() {
                if !step.ops.is_empty() {
                    if self.group_open() {
                        self.fold_into_group(step);
                    } else {
                        report.step = Some(self.push_step(step));
                    }
                } else if !self.group_open() {
                    self.flush_calls(None, false);
                }
            }
        } else if let Some(outer) = self.history.frames.last_mut() {
            outer.absorb(frame);
        }
        Ok((value, report))
    }

    /// Takes back what the innermost step wrote and closes it.
    fn unwind(&mut self) {
        let Some(frame) = self.history.frames.pop() else {
            return;
        };
        let reverted = self.rollback(frame.mark);
        self.calls.drop_waiting(frame.calls);
        let mut restored = false;
        if let Some(name) = frame.name {
            restored |= self.name != name;
            self.name = name;
        }
        if let Some(settings) = frame.settings {
            restored |= self.settings != settings;
            self.settings = settings;
        }
        if let Some(styles) = frame.styles {
            restored |= self.styles != styles;
            self.styles = styles;
        }
        if let Some(sheets) = frame.sheets {
            restored |= self.sheets != sheets;
            self.sheets = sheets;
        }
        if let Some(flags) = frame.flags {
            restored |= self.layers.flags() != flags;
            self.layers.restore_flags(&flags);
        }
        if self.layers.active() != frame.active {
            self.layers.restore_active(&frame.active);
            restored = true;
        }
        self.edits = frame.edits;
        self.dirty = frame.dirty;
        if frame.opened {
            self.history.pending = None;
            self.history.adopt = false;
            self.relocated.clear();
            self.relocated_ends.clear();
        }
        if reverted > 0 || restored {
            self.refresh_viewmarks();
            self.refresh_level_marks();
            self.refresh_derived_tags();
            self.refresh_dimension_set_rows_quietly();
            self.mark_changed();
        }
    }

    /// The open step's ops so far (none outside a step).
    fn step_ops(&self) -> &[Op] {
        let Some(frame) = self.history.frames.last() else {
            return &[];
        };
        self.history
            .pending
            .as_ref()
            .map_or(&[][..], |s| &s.ops[frame.mark.min(s.ops.len())..])
    }

    /// The objects the open step changed so far (not those it added), each with its state before the step, in the
    /// order first changed (docs/adr/0337 §1).
    pub fn step_befores(&self) -> Vec<(Slot, Arc<Entity>)> {
        let mut added = std::collections::HashSet::new();
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for op in self.step_ops() {
            match op {
                Op::Add(s) => {
                    added.insert(s.slot());
                }
                Op::Update { before, .. }
                    if !added.contains(&before.slot()) && seen.insert(before.slot()) =>
                {
                    out.push((before.slot(), before.entity.clone()));
                }
                _ => {}
            }
        }
        out
    }

    /// The objects the open step added, removed or changed so far, each write as it was before and after it (none:
    /// not there), in the order written (docs/adr/0641 §2): a follower that keeps others in step with them reads what
    /// both sides held.
    pub fn step_objects(&self) -> Vec<Written> {
        self.step_ops()
            .iter()
            .filter_map(|op| match op {
                Op::Add(s) => Some((None, Some(s.entity.clone()))),
                Op::Remove(s) => Some((Some(s.entity.clone()), None)),
                Op::Update { before, after } => {
                    Some((Some(before.entity.clone()), Some(after.entity.clone())))
                }
                _ => None,
            })
            .collect()
    }

    /// The layers whose objects the open step added, removed or changed so far (an object that changed layer counts
    /// for both), each once, in the order first touched (docs/adr/0562 §3).
    pub fn step_layers(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut note = |layer: &str| {
            if !out.iter().any(|l| l == layer) {
                out.push(layer.to_owned());
            }
        };
        for op in self.step_ops() {
            match op {
                Op::Add(s) | Op::Remove(s) => note(&s.entity.base().layer_id),
                Op::Update { before, after } => {
                    note(&before.entity.base().layer_id);
                    note(&after.entity.base().layer_id);
                }
                _ => {}
            }
        }
        out
    }

    /// Whether the open step changed the constraints itself (a constraint or parameter command, a copy).
    pub fn step_touches_constraints(&self) -> bool {
        self.step_ops()
            .iter()
            .any(|op| matches!(op, Op::Constraints { .. }))
    }

    /// Within a step: takes away now the constraints its writes so far left without their parts (docs/adr/0337
    /// §2), into the step; how many went.
    pub fn settle_constraints_now(&mut self) -> usize {
        if self.history.pending.is_none() || self.history.frames.is_empty() {
            return 0;
        }
        let ops = self.step_ops().to_vec();
        let before = self.constraints.constraints.len();
        let more = self.follow_constraints(&ops);
        let after = self.constraints.constraints.len();
        if let Some(step) = &mut self.history.pending {
            step.ops.extend(more);
        }
        before.saturating_sub(after)
    }

    /// Whether a step (`run_step`) is open: a command is being written.
    pub fn in_step(&self) -> bool {
        !self.history.frames.is_empty()
    }

    /// Whether a group is open (`begin_group`): its steps become one undo
    /// step when it ends, under its name.
    pub fn in_group(&self) -> bool {
        self.group_open()
    }

    /// Whether the open step, once its body is done, records an undo step of
    /// its own: the outermost step opened the transaction, it holds ops, and
    /// no group gathers it.
    pub(crate) fn step_will_record(&self) -> bool {
        self.history.frames.first().is_some_and(|f| f.opened)
            && self
                .history
                .pending
                .as_ref()
                .is_some_and(|s| !s.ops.is_empty())
            && !self.group_open()
    }

    /// Names the open command step: a change set names its own, whatever its
    /// steps' edits are called. Nothing when no unnamed step is open.
    pub fn name_step(&mut self, label: &str) {
        if self.history.adopt
            && let Some(step) = &mut self.history.pending
        {
            label.clone_into(&mut step.label);
            self.history.adopt = false;
        }
    }

    /// Keeps `part` as it is now in the open step, the first time the step
    /// writes it; whether a step is open (then the step's journal row says
    /// what it changed, and no edit row of its own is written).
    pub(crate) fn keep(&mut self, part: ProjectPart) -> bool {
        let Some(frame) = self.history.frames.last_mut() else {
            return false;
        };
        match part {
            ProjectPart::Name => {
                if frame.name.is_none() {
                    frame.name = Some(self.name.clone());
                }
            }
            ProjectPart::Settings => {
                if frame.settings.is_none() {
                    frame.settings = Some(self.settings.clone());
                }
            }
            ProjectPart::Styles => {
                if frame.styles.is_none() {
                    frame.styles = Some(self.styles.clone());
                }
            }
            ProjectPart::Sheets => {
                if frame.sheets.is_none() {
                    frame.sheets = Some(self.sheets.clone());
                }
            }
            ProjectPart::LayerFlags => {
                if frame.flags.is_none() {
                    frame.flags = Some(self.layers.flags());
                }
            }
            // The constraints change only in undo steps (`Op::Constraints`), which the ops report.
            ProjectPart::Constraints | ProjectPart::DimensionSets => {}
        }
        true
    }

    /// What the innermost step has changed so far.
    fn report_of_frame(&self) -> StepReport {
        let Some(frame) = self.history.frames.last() else {
            return StepReport::default();
        };
        let ops = self
            .history
            .pending
            .as_ref()
            .map_or(&[][..], |s| &s.ops[frame.mark.min(s.ops.len())..]);
        let mut report = if frame.dimension_cache.is_empty() {
            report_of_ops(ops)
        } else {
            let mut events = frame.dimension_cache.iter().peekable();
            let mut all = Vec::with_capacity(ops.len() + frame.dimension_cache.len());
            for (index, op) in ops.iter().enumerate() {
                while let Some((_, before, after)) =
                    events.next_if(|(at, _, _)| *at <= frame.mark + index)
                {
                    all.push(Op::Update {
                        before: before.clone(),
                        after: after.clone(),
                    });
                }
                all.push(op.clone());
            }
            for (_, before, after) in events {
                all.push(Op::Update {
                    before: before.clone(),
                    after: after.clone(),
                });
            }
            report_of_ops(&all)
        };
        let mut cached = std::collections::HashSet::new();
        for (_, _, after) in &frame.dimension_cache {
            if cached.insert(after.uid) {
                report.dimension_set_cached.push(after.uid);
            }
        }
        if let Some(name) = &frame.name
            && *name != self.name
        {
            report.parts.push(ProjectPart::Name);
        }
        if let Some(settings) = &frame.settings
            && *settings != self.settings
            && !report.parts.contains(&ProjectPart::Settings)
        {
            report.parts.push(ProjectPart::Settings);
        }
        if let Some(styles) = &frame.styles
            && *styles != self.styles
        {
            report.parts.push(ProjectPart::Styles);
        }
        if let Some(sheets) = &frame.sheets
            && *sheets != self.sheets
            && !report.parts.contains(&ProjectPart::Sheets)
        {
            report.parts.push(ProjectPart::Sheets);
        }
        if frame
            .flags
            .as_ref()
            .is_some_and(|flags| *flags != self.layers.flags())
            || frame.active != self.layers.active()
        {
            report.parts.push(ProjectPart::LayerFlags);
        }
        if !ops.is_empty() {
            report.label = self
                .history
                .pending
                .as_ref()
                .map(|s| s.label.clone())
                .filter(|l| !l.is_empty());
        }
        report
    }

    pub(crate) fn note_dimension_row_cache(&mut self, before: Stored, after: Stored) {
        let at = self.history.pending.as_ref().map_or(0, |p| p.ops.len());
        if let Some(frame) = self.history.frames.last_mut() {
            frame.dimension_cache.push((at, before, after));
        }
    }
}

/// What `ops` changed, an object once (its first before, its last after).
/// What `ops` changed, as a step's report says it.
pub(crate) fn report_of(ops: &[Op]) -> StepReport {
    report_of_ops(ops)
}

fn report_of_ops(ops: &[Op]) -> StepReport {
    let mut report = StepReport::default();
    let mut at: HashMap<Uuid, usize> = HashMap::new();
    let mut touch = |report: &mut StepReport,
                     uid: Uuid,
                     slot: Slot,
                     before: Option<&Arc<Entity>>,
                     after: Option<&Arc<Entity>>| {
        match at.get(&uid) {
            Some(&i) => {
                if let Some(o) = report.objects.get_mut(i) {
                    o.after = after.cloned();
                    o.slot = slot;
                }
            }
            None => {
                at.insert(uid, report.objects.len());
                report.objects.push(ObjectChange {
                    uid,
                    slot,
                    before: before.cloned(),
                    after: after.cloned(),
                });
            }
        }
    };
    for op in ops {
        if matches!(
            op,
            Op::DimensionSetAdd { .. }
                | Op::DimensionSetRemove { .. }
                | Op::DimensionSetUpdate { .. }
        ) {
            if !report.parts.contains(&ProjectPart::DimensionSets) {
                report.parts.push(ProjectPart::DimensionSets);
            }
            if !report.undo_parts.contains(&ProjectPart::DimensionSets) {
                report.undo_parts.push(ProjectPart::DimensionSets);
            }
        }
        match op {
            Op::Add(s) => touch(&mut report, s.uid, s.slot(), None, Some(&s.entity)),
            Op::Remove(s) => touch(&mut report, s.uid, s.slot(), Some(&s.entity), None),
            Op::Update { before, after } => touch(
                &mut report,
                after.uid,
                after.slot(),
                Some(&before.entity),
                Some(&after.entity),
            ),
            Op::LayerStyle { layer, .. }
            | Op::LayerFields { layer, .. }
            | Op::LayerService { layer, .. }
            | Op::LayerTemporal { layer, .. }
            | Op::LayerFilter { layer, .. }
            | Op::LayerInfo { layer, .. }
            | Op::LayerCompose { layer, .. }
            | Op::LayerSourceCrs { layer, .. }
            | Op::LayerGeometry { layer, .. }
            | Op::LayerTableStyles { layer, .. }
            | Op::LayerSubtypes { layer, .. }
            | Op::LayerContingents { layer, .. }
            | Op::LayerRules { layer, .. }
            | Op::LayerMove { layer, .. } => report.layers.change(layer.clone()),
            Op::LayerAdd(place) => {
                let mut ids = Vec::new();
                node_ids(&place.node, &mut ids);
                for id in ids {
                    report.layers.add(id);
                }
            }
            Op::LayerRemove(place) => {
                let mut ids = Vec::new();
                node_ids(&place.node, &mut ids);
                for id in ids {
                    report.layers.remove(id);
                }
            }
            // The active layer is a flag, said with the flags.
            Op::LayerActive { .. } => {}
            Op::BlockAdd { block, .. } => {
                report.blocks.add(block.id);
                block_state(&mut report, block.id, None, Some(block));
            }
            Op::BlockRemove { block, .. } => {
                report.blocks.remove(block.id);
                block_state(&mut report, block.id, Some(block), None);
            }
            Op::BlockUpdate { before, after } => {
                report.blocks.change(after.id);
                block_state(&mut report, after.id, Some(before), Some(after));
            }
            Op::ArrayAdd { array, .. } => report.arrays.add(array.id),
            Op::ArrayRemove { array, .. } => report.arrays.remove(array.id),
            Op::ArrayUpdate { after, .. } => report.arrays.change(after.id),
            Op::DimensionSetAdd { set, .. } => report.dimension_sets.add(set.id),
            Op::DimensionSetRemove { set, .. } => report.dimension_sets.remove(set.id),
            Op::DimensionSetUpdate { after, .. } => report.dimension_sets.change(after.id),
            Op::DimensionSetViolation { uid } => {
                if !report.dimension_set_violations.contains(uid) {
                    report.dimension_set_violations.push(*uid);
                }
            }
            Op::Constraints { .. } => {
                report.constraints = true;
                if !report.parts.contains(&ProjectPart::Constraints) {
                    report.parts.push(ProjectPart::Constraints);
                }
            }
            // The unit and the value lists are settings (docs/adr/0256 §1, 0430).
            Op::Sheets { .. } => {
                if !report.parts.contains(&ProjectPart::Sheets) {
                    report.parts.push(ProjectPart::Sheets);
                    report.undo_parts.push(ProjectPart::Sheets);
                }
            }
            Op::TemplateSettings { .. } | Op::StyleTables(_) => {
                if !report.parts.contains(&ProjectPart::Settings) {
                    report.parts.push(ProjectPart::Settings);
                }
                if !report.undo_parts.contains(&ProjectPart::Settings) {
                    report.undo_parts.push(ProjectPart::Settings);
                }
            }
            Op::ImportMetadata { .. } => {
                if !report.parts.contains(&ProjectPart::Settings) {
                    report.parts.push(ProjectPart::Settings);
                }
                if op.sheet_states().is_some() && !report.parts.contains(&ProjectPart::Sheets) {
                    report.parts.push(ProjectPart::Sheets);
                    report.undo_parts.push(ProjectPart::Sheets);
                }
            }
            Op::Units { .. } | Op::Domains { .. } | Op::Source { .. } | Op::Lineage { .. } => {
                if !report.parts.contains(&ProjectPart::Settings) {
                    report.parts.push(ProjectPart::Settings);
                }
            }
        }
    }
    report
}

/// Notes a definition's state: its first before, its last after.
fn block_state(
    report: &mut StepReport,
    id: BlockId,
    before: Option<&Arc<BlockDefinition>>,
    after: Option<&Arc<BlockDefinition>>,
) {
    match report.block_states.iter_mut().find(|b| b.id == id) {
        Some(change) => change.after = after.cloned(),
        None => report.block_states.push(BlockChange {
            id,
            before: before.cloned(),
            after: after.cloned(),
        }),
    }
}

fn node_ids(node: &kentos_contracts::LayerNode, out: &mut Vec<String>) {
    out.push(node.id.clone());
    for child in &node.children {
        node_ids(child, out);
    }
}
