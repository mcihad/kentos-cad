//! A road project's settings in the undo history (docs/adr/8054 §3): each before and after shared, so turning a
//! step round copies nothing.

use std::sync::Arc;

use crate::document::Document;

#[derive(Clone, Debug)]
pub(crate) enum RoadOp {
    /// A road project's horizontal alignments (docs/adr/0901 §10), before and after: a setting kept in the undo
    /// history, so the Güzergâh editor's every write is one step.
    Alignments {
        before: Arc<Vec<kentos_contracts::AlignmentDef>>,
        after: Arc<Vec<kentos_contracts::AlignmentDef>>,
    },
    /// A road project's vertical alignments (docs/adr/0902 §9), before and after.
    Profiles {
        before: Arc<Vec<kentos_contracts::ProfileDef>>,
        after: Arc<Vec<kentos_contracts::ProfileDef>>,
    },
    /// A road project's templates and sample line sets (docs/adr/0903 §6), before and after.
    Templates {
        before: Arc<Vec<kentos_contracts::TemplateDef>>,
        after: Arc<Vec<kentos_contracts::TemplateDef>>,
    },
    Sections {
        before: Arc<Vec<kentos_contracts::SectionSetDef>>,
        after: Arc<Vec<kentos_contracts::SectionSetDef>>,
    },
    /// A road project's superelevation schedules (docs/adr/0904 §5), before and after.
    Superelevations {
        before: Arc<Vec<kentos_contracts::SuperelevationDef>>,
        after: Arc<Vec<kentos_contracts::SuperelevationDef>>,
    },
    /// A road project's corridors (docs/adr/0905 §1), before and after.
    Corridors {
        before: Arc<Vec<kentos_contracts::CorridorDef>>,
        after: Arc<Vec<kentos_contracts::CorridorDef>>,
    },
    /// A road project's earthwork volumes (docs/adr/0906), before and after.
    Volumes {
        before: Arc<Vec<kentos_contracts::VolumeDef>>,
        after: Arc<Vec<kentos_contracts::VolumeDef>>,
    },
    /// A road project's mass haul diagrams (docs/adr/0907), before and after.
    Hauls {
        before: Arc<Vec<kentos_contracts::HaulDef>>,
        after: Arc<Vec<kentos_contracts::HaulDef>>,
    },
    /// A road project's plan–profile sheet sets (docs/adr/0908 §10), before and after.
    SheetSets {
        before: Arc<Vec<kentos_contracts::RoadSheetSetDef>>,
        after: Arc<Vec<kentos_contracts::RoadSheetSetDef>>,
    },
    /// A road project's guardrails (docs/adr/0909 §1), before and after.
    Guardrails {
        before: Arc<Vec<kentos_contracts::GuardrailDef>>,
        after: Arc<Vec<kentos_contracts::GuardrailDef>>,
    },
    /// A road project's culverts (docs/adr/0909 §2), before and after.
    Culverts {
        before: Arc<Vec<kentos_contracts::CulvertDef>>,
        after: Arc<Vec<kentos_contracts::CulvertDef>>,
    },
    /// A road project's junctions (docs/adr/0910 §2), before and after.
    Junctions {
        before: Arc<Vec<kentos_contracts::JunctionDef>>,
        after: Arc<Vec<kentos_contracts::JunctionDef>>,
    },
    /// What a road project's commands drew (docs/adr/0911 §1), before and after.
    Drawings {
        before: Arc<Vec<kentos_contracts::RoadDrawingDef>>,
        after: Arc<Vec<kentos_contracts::RoadDrawingDef>>,
    },
    /// Whether a road project draws its stale drawings again by itself (docs/adr/0911 §3), before and after.
    AutoRebuild { before: bool, after: bool },
    /// Whether drawing again draws each drawing whole rather than only what changed (docs/adr/4740), before and after.
    RebuildWhole { before: bool, after: bool },
    /// A road project's pay item book (docs/adr/0909 §4), before and after.
    Pay {
        before: Arc<Option<kentos_contracts::PayBookDef>>,
        after: Arc<Option<kentos_contracts::PayBookDef>>,
    },
    /// A road project's retaining walls (docs/adr/0909 §3), before and after.
    Walls {
        before: Arc<Vec<kentos_contracts::RetainingWallDef>>,
        after: Arc<Vec<kentos_contracts::RetainingWallDef>>,
    },
    /// A road project's terrain surfaces when a repair writes them (docs/adr/0916 §6), before and after.
    Surfaces {
        before: Arc<Vec<kentos_contracts::TerrainSurfaceDef>>,
        after: Arc<Vec<kentos_contracts::TerrainSurfaceDef>>,
    },
    /// A road project's graded pads (docs/adr/0927), before and after.
    Platforms {
        before: Arc<Vec<kentos_contracts::PlatformDef>>,
        after: Arc<Vec<kentos_contracts::PlatformDef>>,
    },
}

impl RoadOp {
    pub(crate) fn inverse(&self) -> Self {
        match self {
            RoadOp::Alignments { before, after } => RoadOp::Alignments {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::Profiles { before, after } => RoadOp::Profiles {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::Templates { before, after } => RoadOp::Templates {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::Sections { before, after } => RoadOp::Sections {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::Superelevations { before, after } => RoadOp::Superelevations {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::Corridors { before, after } => RoadOp::Corridors {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::Volumes { before, after } => RoadOp::Volumes {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::Hauls { before, after } => RoadOp::Hauls {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::SheetSets { before, after } => RoadOp::SheetSets {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::Guardrails { before, after } => RoadOp::Guardrails {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::Culverts { before, after } => RoadOp::Culverts {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::Junctions { before, after } => RoadOp::Junctions {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::Drawings { before, after } => RoadOp::Drawings {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::AutoRebuild { before, after } => RoadOp::AutoRebuild {
                before: *after,
                after: *before,
            },
            RoadOp::RebuildWhole { before, after } => RoadOp::RebuildWhole {
                before: *after,
                after: *before,
            },
            RoadOp::Pay { before, after } => RoadOp::Pay {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::Walls { before, after } => RoadOp::Walls {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::Surfaces { before, after } => RoadOp::Surfaces {
                before: after.clone(),
                after: before.clone(),
            },
            RoadOp::Platforms { before, after } => RoadOp::Platforms {
                before: after.clone(),
                after: before.clone(),
            },
        }
    }

    pub(crate) fn apply(&self, doc: &mut Document) {
        match self {
            RoadOp::Alignments { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.alignments = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Profiles { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.profiles = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Templates { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.templates = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Sections { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.sections = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Superelevations { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.superelevations = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Corridors { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.corridors = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Volumes { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.volumes = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Hauls { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.hauls = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::SheetSets { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.sheet_sets = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Guardrails { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.guardrails = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Culverts { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.culverts = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Junctions { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.junctions = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Drawings { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.drawings = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::AutoRebuild { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.auto_rebuild = *after;
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::RebuildWhole { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.rebuild_whole = *after;
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Pay { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.pay = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Walls { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.walls = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Surfaces { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.surfaces = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
            RoadOp::Platforms { after, .. } => {
                let mut road = Arc::make_mut(&mut doc.settings)
                    .road
                    .take()
                    .unwrap_or_default();
                road.platforms = (**after).clone();
                Arc::make_mut(&mut doc.settings).road = (!road.is_empty()).then_some(road);
            }
        }
    }
}
