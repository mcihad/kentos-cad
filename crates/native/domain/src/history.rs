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

use kentos_contracts::{
    AssocArray, BlockDefinition, BlockId, Entity, FeatureFeed, LayerField, LayerNode, LayerStyle,
    ServiceLayer,
};

use crate::changes::Journal;
use crate::document::Document;
use crate::identity::{Slot, Uuid};
use crate::store::Stored;
use crate::units::UnitState;

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

/// What a layer is drawn from or took its objects from (docs/adr/0208):
/// none, a map service, or a source (the web's `ServedBy`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ServedBy {
    pub service: Option<ServiceLayer>,
    pub feed: Option<FeatureFeed>,
}

/// A node's time setting, scenario and base layer (docs/adr/0210 §2), as
/// their undo step keeps them (the web's `Temporal`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Temporal {
    pub time: Option<kentos_contracts::LayerTime>,
    pub scenario: Option<kentos_contracts::ScenarioInfo>,
    pub replaces: Option<String>,
}

/// A layer's actions and card (docs/adr/0225 §4, §5) and its description as a dataset and derivation record
/// (docs/adr/0633), as their undo step keeps them (the web's `LayerInfo`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayerInfo {
    pub actions: Vec<kentos_contracts::LayerAction>,
    pub card: Option<kentos_contracts::InfoCard>,
    pub dataset: Option<kentos_contracts::DatasetInfo>,
    pub derivation: Option<kentos_contracts::LayerDerivation>,
}

/// A binding as its undo step keeps it (docs/adr/0636 §5): the link and the base it was last reconciled from.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SourceState {
    pub link: Option<kentos_contracts::SourceLink>,
    pub base: Option<kentos_contracts::SourceBase>,
}

/// A service layer as its undo step keeps it: its name too.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Served {
    pub name: String,
    pub by: ServedBy,
}

#[derive(Clone, Debug)]
pub(crate) enum Op {
    /// A paper book written inside an explicit model/script group (ADR 0590).
    Sheets {
        before: Option<Arc<kentos_contracts::ProjectSheets>>,
        after: Option<Arc<kentos_contracts::ProjectSheets>>,
    },
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
    /// A layer's fields (docs/adr/0199 §3), before and after.
    LayerFields {
        layer: String,
        before: Vec<LayerField>,
        after: Vec<LayerField>,
    },
    /// A layer's name, map service and source (docs/adr/0208 §2, §10), before and after.
    LayerService {
        layer: String,
        before: Box<Served>,
        after: Box<Served>,
    },
    /// A node's time setting, scenario and base layer (docs/adr/0210 §2), before and after.
    LayerTemporal {
        layer: String,
        before: Box<Temporal>,
        after: Box<Temporal>,
    },
    /// A layer's filter (docs/adr/0211 §2), before and after.
    LayerFilter {
        layer: String,
        before: Option<Box<kentos_contracts::LayerFilter>>,
        after: Option<Box<kentos_contracts::LayerFilter>>,
    },
    /// A layer's actions and card (docs/adr/0225 §4, §5), before and after.
    LayerInfo {
        layer: String,
        before: Box<LayerInfo>,
        after: Box<LayerInfo>,
    },
    /// A layer's source system record (docs/adr/0583 §1), before and after.
    LayerSourceCrs {
        layer: String,
        before: Option<Box<kentos_contracts::LayerSourceCrs>>,
        after: Option<Box<kentos_contracts::LayerSourceCrs>>,
    },
    /// A layer's type (docs/adr/0622 §5), before and after.
    LayerGeometry {
        layer: String,
        before: Option<kentos_contracts::LayerGeometry>,
        after: Option<kentos_contracts::LayerGeometry>,
    },
    /// A layer's composition (docs/adr/0226 §3, §4), before and after.
    LayerCompose {
        layer: String,
        before: Option<Box<kentos_contracts::LayerCompose>>,
        after: Option<Box<kentos_contracts::LayerCompose>>,
    },
    /// A layer's table rules (Koşullu biçim, docs/adr/0413 §2), before and after.
    LayerTableStyles {
        layer: String,
        before: Vec<kentos_contracts::TableStyle>,
        after: Vec<kentos_contracts::TableStyle>,
    },
    /// A layer's subtypes (Alt türler, docs/adr/0422), before and after.
    LayerSubtypes {
        layer: String,
        before: Option<Box<kentos_contracts::LayerSubtypes>>,
        after: Option<Box<kentos_contracts::LayerSubtypes>>,
    },
    /// A layer's value combinations (Değer bileşimleri, docs/adr/0423), before and after.
    LayerContingents {
        layer: String,
        before: Vec<kentos_contracts::FieldGroup>,
        after: Vec<kentos_contracts::FieldGroup>,
    },
    /// A layer's record rules (Kayıt kuralları, docs/adr/0438 §1), before and after.
    LayerRules {
        layer: String,
        before: Vec<kentos_contracts::LayerRule>,
        after: Vec<kentos_contracts::LayerRule>,
    },
    /// A layer's indexes (Dizinler, docs/adr/0758), before and after.
    LayerIndexes {
        layer: String,
        before: Vec<kentos_contracts::LayerIndex>,
        after: Vec<kentos_contracts::LayerIndex>,
    },
    /// A layer or a group taken out of the tree with everything under it
    /// (`Document::remove_layer`), and its inverse, which puts it back; a
    /// new one (`Document::add_layer`) is a `LayerAdd`.
    LayerRemove(Box<LayerPlace>),
    LayerAdd(Box<LayerPlace>),
    /// A node moved with everything under it (`Document::move_layer`,
    /// docs/adr/0248 §1): from its group and place to others.
    LayerMove {
        layer: String,
        from: (Option<String>, usize),
        to: (Option<String>, usize),
    },
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
    /// An associative array added at `index` of the list (arrays.rs,
    /// docs/adr/0300), its inverse, which takes it out, and the same array
    /// (its id) before and after; shared, so a step keeps no copy.
    ArrayAdd {
        index: usize,
        array: Arc<AssocArray>,
    },
    ArrayRemove {
        index: usize,
        array: Arc<AssocArray>,
    },
    ArrayUpdate {
        before: Arc<AssocArray>,
        after: Arc<AssocArray>,
    },
    /// A kitchen run added at `index` of the list (casework.rs, docs/adr/0727 §5), its inverse, and the same run (its
    /// id) before and after.
    CaseworkAdd {
        index: usize,
        run: Arc<kentos_contracts::CaseworkRecord>,
    },
    CaseworkRemove {
        index: usize,
        run: Arc<kentos_contracts::CaseworkRecord>,
    },
    CaseworkUpdate {
        before: Arc<kentos_contracts::CaseworkRecord>,
        after: Arc<kentos_contracts::CaseworkRecord>,
    },
    DimensionSetAdd {
        index: usize,
        set: Arc<kentos_contracts::DimensionSet>,
    },
    DimensionSetRemove {
        index: usize,
        set: Arc<kentos_contracts::DimensionSet>,
    },
    DimensionSetUpdate {
        before: Arc<kentos_contracts::DimensionSet>,
        after: Arc<kentos_contracts::DimensionSet>,
        /// Exact private child operations; metadata elsewhere in a transaction grants no authority.
        writes: Vec<(Uuid, Arc<Entity>, Arc<Entity>)>,
    },
    /// Evidence of a forbidden user edit, even if another operation subsequently hid that edit.
    DimensionSetViolation {
        uid: Uuid,
    },
    /// The style tables a `cad.styles.edit` changed, style by style (docs/adr/0699).
    StyleTables(Box<crate::style_tables::StyleTables>),
    /// Only the metadata installed by an explicit architectural package (ADR 0659).
    TemplateSettings {
        before: Box<crate::template_settings::TemplateSettings>,
        after: Box<crate::template_settings::TemplateSettings>,
    },
    ImportMetadata {
        before: Box<crate::import_metadata::ImportMetadata>,
        after: Box<crate::import_metadata::ImportMetadata>,
    },
    /// The drawing unit, home view and anchor changed with the scaled objects (ADR 0256).
    Units {
        before: UnitState,
        after: UnitState,
    },
    /// The project's value lists (docs/adr/0430), before and after: a setting, written in the step
    /// with the linked fields it rewrote, so undo gives both back.
    Domains {
        before: Vec<kentos_contracts::FieldDomain>,
        after: Vec<kentos_contracts::FieldDomain>,
    },
    /// Mimari denetim's exceptions (docs/adr/0732 §3), before and after: a setting, written as a step of its own so a
    /// mark given by mistake is undone.
    AuditExceptions {
        before: Vec<kentos_contracts::ArchAuditException>,
        after: Vec<kentos_contracts::ArchAuditException>,
    },
    /// The project's door and window types (docs/adr/0772), before and after: a setting, written in the step with the
    /// openings that followed them, so undo gives both back.
    OpeningTypes {
        before: Vec<kentos_contracts::OpeningType>,
        after: Vec<kentos_contracts::OpeningType>,
    },
    /// The project's building materials (docs/adr/0785), before and after: a setting, written in the step with the
    /// styles, walls and slabs bound to them, so undo gives both back.
    Materials {
        before: Vec<kentos_contracts::ArchMaterial>,
        after: Vec<kentos_contracts::ArchMaterial>,
    },
    /// The building program (docs/adr/0773), before and after: a setting, written in the step with the spaces' groups
    /// it gives, so undo gives both back.
    BuildingProgram {
        before: kentos_contracts::BuildingProgram,
        after: kentos_contracts::BuildingProgram,
    },
    /// The project's room modules (docs/adr/0775), before and after: a setting, written in the step with the objects
    /// placed or updated, so undo gives both back.
    ArchModules {
        before: Vec<kentos_contracts::ArchModule>,
        after: Vec<kentos_contracts::ArchModule>,
    },
    /// A binding to a source table and its reconciling base (docs/adr/0636 §5), before and after: written in the step
    /// that changed its key or its area, so undo gives the binding back with the objects (none: no binding).
    Source {
        id: String,
        before: Box<SourceState>,
        after: Box<SourceState>,
    },
    /// What the split and merged objects came from (docs/adr/0598 §2), before and after: a setting, written in the
    /// step whose split or merge made the records, so undo takes them back with the objects.
    Lineage {
        before: Arc<Vec<kentos_contracts::LineageRecord>>,
        after: Arc<Vec<kentos_contracts::LineageRecord>>,
    },
    /// The constraints and the parameter table (docs/adr/0334), before and after; written in the step with the
    /// objects a solve moved, so undo gives both back.
    Constraints {
        before: Arc<kentos_contracts::ConstraintSet>,
        after: Arc<kentos_contracts::ConstraintSet>,
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
        matches!(
            self,
            Op::LayerRemove(_) | Op::LayerAdd(_) | Op::LayerMove { .. }
        )
    }
}

type SheetStates<'a> = (
    Option<&'a kentos_contracts::ProjectSheets>,
    Option<&'a kentos_contracts::ProjectSheets>,
);

impl Op {
    pub(crate) fn sheet_states(&self) -> Option<SheetStates<'_>> {
        match self {
            Op::Sheets { before, after } => Some((before.as_deref(), after.as_deref())),
            Op::ImportMetadata { before, after } if before.sheets() != after.sheets() => {
                Some((before.sheets(), after.sheets()))
            }
            _ => None,
        }
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
            Op::LayerFields {
                layer,
                before,
                after,
            } => Op::LayerFields {
                layer: layer.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Op::LayerService {
                layer,
                before,
                after,
            } => Op::LayerService {
                layer: layer.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Op::LayerTemporal {
                layer,
                before,
                after,
            } => Op::LayerTemporal {
                layer: layer.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Op::LayerFilter {
                layer,
                before,
                after,
            } => Op::LayerFilter {
                layer: layer.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Op::LayerInfo {
                layer,
                before,
                after,
            } => Op::LayerInfo {
                layer: layer.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Op::LayerSourceCrs {
                layer,
                before,
                after,
            } => Op::LayerSourceCrs {
                layer: layer.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Op::LayerGeometry {
                layer,
                before,
                after,
            } => Op::LayerGeometry {
                layer: layer.clone(),
                before: *after,
                after: *before,
            },
            Op::LayerCompose {
                layer,
                before,
                after,
            } => Op::LayerCompose {
                layer: layer.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Op::LayerTableStyles {
                layer,
                before,
                after,
            } => Op::LayerTableStyles {
                layer: layer.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Op::LayerSubtypes {
                layer,
                before,
                after,
            } => Op::LayerSubtypes {
                layer: layer.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Op::LayerContingents {
                layer,
                before,
                after,
            } => Op::LayerContingents {
                layer: layer.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Op::LayerRules {
                layer,
                before,
                after,
            } => Op::LayerRules {
                layer: layer.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Op::LayerIndexes {
                layer,
                before,
                after,
            } => Op::LayerIndexes {
                layer: layer.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Op::LayerRemove(place) => Op::LayerAdd(place.clone()),
            Op::LayerAdd(place) => Op::LayerRemove(place.clone()),
            Op::LayerMove { layer, from, to } => Op::LayerMove {
                layer: layer.clone(),
                from: to.clone(),
                to: from.clone(),
            },
            Op::LayerActive { before, after } => Op::LayerActive {
                before: after.clone(),
                after: before.clone(),
            },
            Op::BlockAdd { index, block } => Op::BlockRemove {
                index: *index,
                block: block.clone(),
            },
            Op::ArrayAdd { index, array } => Op::ArrayRemove {
                index: *index,
                array: array.clone(),
            },
            Op::ArrayRemove { index, array } => Op::ArrayAdd {
                index: *index,
                array: array.clone(),
            },
            Op::ArrayUpdate { before, after } => Op::ArrayUpdate {
                before: after.clone(),
                after: before.clone(),
            },
            Op::CaseworkAdd { index, run } => Op::CaseworkRemove {
                index: *index,
                run: run.clone(),
            },
            Op::CaseworkRemove { index, run } => Op::CaseworkAdd {
                index: *index,
                run: run.clone(),
            },
            Op::CaseworkUpdate { before, after } => Op::CaseworkUpdate {
                before: after.clone(),
                after: before.clone(),
            },
            Op::DimensionSetAdd { index, set } => Op::DimensionSetRemove {
                index: *index,
                set: set.clone(),
            },
            Op::DimensionSetRemove { index, set } => Op::DimensionSetAdd {
                index: *index,
                set: set.clone(),
            },
            Op::DimensionSetUpdate {
                before,
                after,
                writes,
            } => Op::DimensionSetUpdate {
                before: after.clone(),
                after: before.clone(),
                writes: writes
                    .iter()
                    .map(|(uid, b, a)| (*uid, a.clone(), b.clone()))
                    .collect(),
            },
            Op::DimensionSetViolation { uid } => Op::DimensionSetViolation { uid: *uid },
            Op::BlockRemove { index, block } => Op::BlockAdd {
                index: *index,
                block: block.clone(),
            },
            Op::BlockUpdate { before, after } => Op::BlockUpdate {
                before: after.clone(),
                after: before.clone(),
            },
            Op::Sheets { before, after } => Op::Sheets {
                before: after.clone(),
                after: before.clone(),
            },
            Op::StyleTables(tables) => Op::StyleTables(Box::new(tables.inverse())),
            Op::TemplateSettings { before, after } => Op::TemplateSettings {
                before: after.clone(),
                after: before.clone(),
            },
            Op::ImportMetadata { before, after } => Op::ImportMetadata {
                before: after.clone(),
                after: before.clone(),
            },
            Op::Units { before, after } => Op::Units {
                before: *after,
                after: *before,
            },
            Op::Domains { before, after } => Op::Domains {
                before: after.clone(),
                after: before.clone(),
            },
            Op::AuditExceptions { before, after } => Op::AuditExceptions {
                before: after.clone(),
                after: before.clone(),
            },
            Op::OpeningTypes { before, after } => Op::OpeningTypes {
                before: after.clone(),
                after: before.clone(),
            },
            Op::Materials { before, after } => Op::Materials {
                before: after.clone(),
                after: before.clone(),
            },
            Op::BuildingProgram { before, after } => Op::BuildingProgram {
                before: after.clone(),
                after: before.clone(),
            },
            Op::ArchModules { before, after } => Op::ArchModules {
                before: after.clone(),
                after: before.clone(),
            },
            Op::Source { id, before, after } => Op::Source {
                id: id.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Op::Lineage { before, after } => Op::Lineage {
                before: after.clone(),
                after: before.clone(),
            },
            Op::Constraints { before, after } => Op::Constraints {
                before: after.clone(),
                after: before.clone(),
            },
        }
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
        let tables = self.follow_tables(ops);
        ops.extend(tables);
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

    /// Applies the inverse of `ops`, newest first.
    pub(crate) fn revert(&mut self, ops: &[Op]) {
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
                Op::DimensionSetViolation { uid } => uids.contains(uid),
                Op::LayerStyle { .. }
                | Op::LayerFields { .. }
                | Op::LayerService { .. }
                | Op::LayerTemporal { .. }
                | Op::LayerFilter { .. }
                | Op::LayerInfo { .. }
                | Op::LayerCompose { .. }
                | Op::LayerSourceCrs { .. }
                | Op::LayerGeometry { .. }
                | Op::LayerTableStyles { .. }
                | Op::LayerSubtypes { .. }
                | Op::LayerContingents { .. }
                | Op::LayerRules { .. }
                | Op::LayerIndexes { .. }
                | Op::LayerRemove(_)
                | Op::LayerAdd(_)
                | Op::LayerMove { .. }
                | Op::LayerActive { .. }
                | Op::BlockAdd { .. }
                | Op::BlockRemove { .. }
                | Op::BlockUpdate { .. }
                | Op::ArrayAdd { .. }
                | Op::ArrayRemove { .. }
                | Op::ArrayUpdate { .. }
                | Op::CaseworkAdd { .. }
                | Op::CaseworkRemove { .. }
                | Op::CaseworkUpdate { .. }
                | Op::DimensionSetAdd { .. }
                | Op::DimensionSetRemove { .. }
                | Op::DimensionSetUpdate { .. }
                | Op::ImportMetadata { .. }
                | Op::TemplateSettings { .. }
                | Op::StyleTables(_)
                | Op::Sheets { .. }
                | Op::Units { .. }
                | Op::Domains { .. }
                | Op::AuditExceptions { .. }
                | Op::OpeningTypes { .. }
                | Op::Materials { .. }
                | Op::BuildingProgram { .. }
                | Op::ArchModules { .. }
                | Op::Source { .. }
                | Op::Lineage { .. }
                | Op::Constraints { .. } => false,
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
    pub(crate) fn forget_block_history(
        &mut self,
        changed: &HashSet<BlockId>,
        gone: &HashSet<BlockId>,
    ) {
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

    /// Drops the undo and redo steps that change an associative array: another editor's arrays came, and
    /// undoing such a step would put back one they replaced (docs/adr/0300 §5).
    pub(crate) fn forget_array_history(&mut self) {
        let touches = |step: &Step| {
            step.ops.iter().any(|op| {
                matches!(
                    op,
                    Op::ArrayAdd { .. } | Op::ArrayRemove { .. } | Op::ArrayUpdate { .. }
                )
            })
        };
        self.history.undo.retain(|step| !touches(step));
        self.history.redo.retain(|step| !touches(step));
    }

    /// Drops the undo and redo steps that change a kitchen run: another editor's runs came (docs/adr/0727 §5).
    pub(crate) fn forget_casework_history(&mut self) {
        let touches = |step: &Step| {
            step.ops.iter().any(|op| {
                matches!(
                    op,
                    Op::CaseworkAdd { .. } | Op::CaseworkRemove { .. } | Op::CaseworkUpdate { .. }
                )
            })
        };
        self.history.undo.retain(|step| !touches(step));
        self.history.redo.retain(|step| !touches(step));
    }

    pub(crate) fn forget_dimension_set_history(&mut self) {
        let touches = |step: &Step| {
            step.ops.iter().any(|op| {
                matches!(
                    op,
                    Op::DimensionSetAdd { .. }
                        | Op::DimensionSetRemove { .. }
                        | Op::DimensionSetUpdate { .. }
                )
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
            Op::LayerFields { layer, after, .. } => {
                self.layers.replace_fields(layer, after);
                return;
            }
            Op::LayerService { layer, after, .. } => {
                self.layers.replace_service(layer, after);
                return;
            }
            Op::LayerTemporal { layer, after, .. } => {
                self.layers.replace_temporal(layer, after);
                return;
            }
            Op::LayerFilter { layer, after, .. } => {
                self.layers.replace_filter(layer, after.as_deref());
                return;
            }
            Op::LayerInfo { layer, after, .. } => {
                self.layers.replace_info(layer, after);
                return;
            }
            Op::LayerSourceCrs { layer, after, .. } => {
                self.layers.replace_source_crs(layer, after.as_deref());
                return;
            }
            Op::LayerGeometry { layer, after, .. } => {
                self.layers.replace_geometry(layer, *after);
                return;
            }
            Op::LayerCompose { layer, after, .. } => {
                self.layers.replace_compose(layer, after.as_deref());
                return;
            }
            Op::LayerTableStyles { layer, after, .. } => {
                self.layers.replace_table_styles(layer, after);
                return;
            }
            Op::LayerSubtypes { layer, after, .. } => {
                self.layers.replace_subtypes(layer, after.as_deref());
                return;
            }
            Op::LayerContingents { layer, after, .. } => {
                self.layers.replace_contingents(layer, after);
                return;
            }
            Op::LayerRules { layer, after, .. } => {
                self.layers.replace_rules(layer, after);
                return;
            }
            Op::LayerIndexes { layer, after, .. } => {
                self.layers.replace_indexes(layer, after);
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
            Op::LayerMove { layer, to, .. } => {
                self.layers.move_node(layer, to.0.as_deref(), to.1);
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
            Op::ArrayAdd { .. } | Op::ArrayRemove { .. } | Op::ArrayUpdate { .. } => {
                self.apply_array_op(op);
                return;
            }
            Op::CaseworkAdd { .. } | Op::CaseworkRemove { .. } | Op::CaseworkUpdate { .. } => {
                self.apply_casework_op(op);
                return;
            }
            Op::DimensionSetAdd { .. }
            | Op::DimensionSetRemove { .. }
            | Op::DimensionSetUpdate { .. } => {
                self.apply_dimension_set_op(op);
                return;
            }
            Op::DimensionSetViolation { .. } => return,
            Op::Sheets { after, .. } => {
                self.sheets.clone_from(after);
                return;
            }
            Op::TemplateSettings { before, after } => {
                after.apply_from(before, self);
                return;
            }
            Op::StyleTables(tables) => {
                tables.apply_to(&mut self.settings);
                return;
            }
            Op::ImportMetadata { before, after } => {
                after.apply_from(before, self);
                return;
            }
            Op::Units { after, .. } => {
                self.settings.drawing_unit = after.drawing_unit;
                self.home_view = after.home_view;
                self.origin = after.origin;
                return;
            }
            Op::Domains { after, .. } => {
                self.settings.domains = after.clone();
                return;
            }
            Op::OpeningTypes { after, .. } => {
                let mut arch = self.settings.arch.take().unwrap_or_default();
                arch.opening_types = after.clone();
                self.settings.arch = (!arch.is_empty()).then_some(arch);
                return;
            }
            Op::Materials { after, .. } => {
                let mut arch = self.settings.arch.take().unwrap_or_default();
                arch.materials = after.clone();
                self.settings.arch = (!arch.is_empty()).then_some(arch);
                return;
            }
            Op::ArchModules { after, .. } => {
                let mut arch = self.settings.arch.take().unwrap_or_default();
                arch.modules = after.clone();
                self.settings.arch = (!arch.is_empty()).then_some(arch);
                return;
            }
            Op::AuditExceptions { after, .. } => {
                let mut arch = self.settings.arch.take().unwrap_or_default();
                arch.audit_exceptions = after.clone();
                // No empty settings are kept (a file refuses them).
                self.settings.arch = (!arch.is_empty()).then_some(arch);
                return;
            }
            Op::BuildingProgram { after, .. } => {
                let mut arch = self.settings.arch.take().unwrap_or_default();
                arch.program = after.clone();
                self.settings.arch = (!arch.is_empty()).then_some(arch);
                return;
            }
            Op::Source { id, after, .. } => {
                let sources = &mut self.settings.sources;
                match (&after.link, sources.iter().position(|l| l.id == *id)) {
                    (Some(link), Some(i)) => sources[i] = link.clone(),
                    (Some(link), None) => sources.push(link.clone()),
                    (None, Some(i)) => {
                        sources.remove(i);
                    }
                    (None, None) => {}
                }
                let bases = &mut self.settings.source_bases;
                bases.retain(|b| b.source != *id);
                if let Some(b) = &after.base {
                    bases.push(b.clone());
                }
                return;
            }
            Op::Lineage { after, .. } => {
                self.settings.lineage = (**after).clone();
                return;
            }
            Op::Constraints { after, .. } => {
                self.constraints = after.clone();
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
