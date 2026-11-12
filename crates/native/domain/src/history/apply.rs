//! An operation put on the drawing (docs/adr/0003): a step done, undone or redone, a change from outside.

use std::sync::Arc;

use kentos_contracts::LayerNode;

use super::Op;
use crate::document::Document;

impl Document {
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

    pub(super) fn apply(&mut self, op: &Op) {
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
            Op::ReviewNotes { after, .. } => {
                Arc::make_mut(&mut self.settings).review_notes = after.as_ref().clone();
                return;
            }
            Op::PatternTables { after, .. } => {
                Arc::make_mut(&mut self.settings).line_types = after.0.clone();
                Arc::make_mut(&mut self.settings).hatch_patterns = after.1.clone();
                return;
            }
            Op::ApprovedSheetTemplates { after, .. } => {
                Arc::make_mut(&mut self.settings).approved_sheet_templates = after.as_ref().clone();
                return;
            }
            Op::StyleTables(tables) => {
                tables.apply_to(Arc::make_mut(&mut self.settings));
                return;
            }
            Op::ImportMetadata { before, after } => {
                after.apply_from(before, self);
                return;
            }
            Op::Units { before, after } => {
                Arc::make_mut(&mut self.settings).drawing_unit = after.drawing_unit;
                self.home_view = after.home_view;
                self.origin = after.origin;
                // A keep-sizes unit change did not edit frames. Do not undo
                // a UCS the user chose later through ordinary project settings.
                if before.ucs != after.ucs {
                    Arc::make_mut(&mut self.settings).ucs = after.ucs;
                }
                if before.named_ucs != after.named_ucs {
                    Arc::make_mut(&mut self.settings)
                        .named_ucs
                        .clone_from(&after.named_ucs);
                }
                return;
            }
            Op::Road(op) => {
                op.apply(self);
                return;
            }
            Op::Map(op) => {
                op.apply(self);
                return;
            }
            Op::Arch(op) => {
                op.apply(self);
                return;
            }
            Op::Gis(op) => {
                op.apply(self);
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
pub(super) fn node_ids(node: &LayerNode, out: &mut Vec<String>) {
    out.push(node.id.clone());
    for child in &node.children {
        node_ids(child, out);
    }
}
