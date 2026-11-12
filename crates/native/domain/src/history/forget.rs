//! The undo and redo steps let go when another editor's change makes them wrong (the web's `forgetHistoryOf`).

use std::collections::{HashSet, VecDeque};

use kentos_contracts::{BlockDefinition, BlockId, Entity};

use super::apply::node_ids;
use super::{Op, Step};
use crate::document::Document;
use crate::identity::{Slot, Uuid};
use crate::store::Stored;

impl Document {
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
                | Op::ReviewNotes { .. }
                | Op::PatternTables { .. }
                | Op::ApprovedSheetTemplates { .. }
                | Op::StyleTables(_)
                | Op::Sheets { .. }
                | Op::Units { .. }
                | Op::Road(_)
                | Op::Map(_)
                | Op::Arch(_)
                | Op::Gis(_)
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
}
