//! The undo history's operations (docs/adr/0003): each change as its before and after, turned round to undo it; a
//! discipline's settings in its own module (docs/adr/8054 §3).

use std::sync::Arc;

use kentos_contracts::{
    AssocArray, BlockDefinition, Entity, FeatureFeed, LayerField, LayerNode, LayerStyle,
    ServiceLayer,
};

use super::{ArchOp, GisOp, MapOp, RoadOp};
use crate::identity::Uuid;
use crate::store::Stored;
use crate::units::UnitState;

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
    /// The project's line types and hatch patterns (docs/adr/1002 §4), before and after: a standard's resources
    /// installed as a step of their own.
    PatternTables {
        before: std::sync::Arc<(
            Vec<kentos_contracts::LineTypeDef>,
            Vec<kentos_contracts::HatchPatternDef>,
        )>,
        after: std::sync::Arc<(
            Vec<kentos_contracts::LineTypeDef>,
            Vec<kentos_contracts::HatchPatternDef>,
        )>,
    },
    /// The approved sheet template revisions (docs/adr/1100 §3), before and after: recorded by a standard's install
    /// in the same step as its resources.
    ApprovedSheetTemplates {
        before: std::sync::Arc<Vec<kentos_contracts::ApprovedSheetTemplate>>,
        after: std::sync::Arc<Vec<kentos_contracts::ApprovedSheetTemplate>>,
    },
    /// The review notes (docs/adr/1000 §3), before and after: a note is not an object but its change is a step.
    ReviewNotes {
        before: std::sync::Arc<Vec<kentos_contracts::ReviewNote>>,
        after: std::sync::Arc<Vec<kentos_contracts::ReviewNote>>,
    },
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
    /// The constraints and the parameter table (docs/adr/0334), before and after; written in the step with the
    /// objects a solve moved, so undo gives both back.
    Constraints {
        before: Arc<kentos_contracts::ConstraintSet>,
        after: Arc<kentos_contracts::ConstraintSet>,
    },
    /// A road project's settings (history/road.rs, docs/adr/8054 §3).
    Road(RoadOp),
    /// A map project's settings (history/map.rs).
    Map(MapOp),
    /// An architectural project's settings (history/arch.rs).
    Arch(ArchOp),
    /// A GIS project's settings (history/gis.rs).
    Gis(GisOp),
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
    pub(super) fn is_tree(&self) -> bool {
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
    pub(super) fn inverse(&self) -> Op {
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
            Op::ReviewNotes { before, after } => Op::ReviewNotes {
                before: after.clone(),
                after: before.clone(),
            },
            Op::PatternTables { before, after } => Op::PatternTables {
                before: after.clone(),
                after: before.clone(),
            },
            Op::ApprovedSheetTemplates { before, after } => Op::ApprovedSheetTemplates {
                before: after.clone(),
                after: before.clone(),
            },
            Op::ImportMetadata { before, after } => Op::ImportMetadata {
                before: after.clone(),
                after: before.clone(),
            },
            Op::Units { before, after } => Op::Units {
                before: after.clone(),
                after: before.clone(),
            },
            Op::Constraints { before, after } => Op::Constraints {
                before: after.clone(),
                after: before.clone(),
            },
            Op::Road(op) => Op::Road(op.inverse()),
            Op::Map(op) => Op::Map(op.inverse()),
            Op::Arch(op) => Op::Arch(op.inverse()),
            Op::Gis(op) => Op::Gis(op.inverse()),
        }
    }
}
