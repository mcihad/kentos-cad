//! An architectural project's settings in the undo history (docs/adr/8054 §3): opening types, materials, the
//! building programme, view areas, modules, design options and audit exceptions.

use std::sync::Arc;

use crate::document::Document;

#[derive(Clone, Debug)]
pub(crate) enum ArchOp {
    /// The project's door and window types (docs/adr/0772), before and after: a setting, written in the step with the
    /// openings that followed them, so undo gives both back.
    OpeningTypes {
        before: Arc<Vec<kentos_contracts::OpeningType>>,
        after: Arc<Vec<kentos_contracts::OpeningType>>,
    },
    /// The project's building materials (docs/adr/0785), before and after: a setting, written in the step with the
    /// styles, walls and slabs bound to them, so undo gives both back.
    Materials {
        before: Arc<Vec<kentos_contracts::ArchMaterial>>,
        after: Arc<Vec<kentos_contracts::ArchMaterial>>,
    },
    /// The building program (docs/adr/0773), before and after: a setting, written in the step with the spaces' groups
    /// it gives, so undo gives both back.
    BuildingProgram {
        before: Arc<kentos_contracts::BuildingProgram>,
        after: Arc<kentos_contracts::BuildingProgram>,
    },
    /// A named view's area (docs/adr/0789 §1), by the view's key, before and after: the crop region its linked maps cut
    /// to, its own undo step.
    ViewArea {
        key: String,
        before: crate::view_area::ViewArea,
        after: crate::view_area::ViewArea,
    },
    /// The project's room modules (docs/adr/0775), before and after: a setting, written in the step with the objects
    /// placed or updated, so undo gives both back.
    Modules {
        before: Arc<Vec<kentos_contracts::ArchModule>>,
        after: Arc<Vec<kentos_contracts::ArchModule>>,
    },
    /// The project's design option sets (docs/adr/0788), before and after: a setting, written in the step with the
    /// objects an option copies or deletes, so undo gives both back.
    Options {
        before: Arc<Vec<kentos_contracts::OptionSet>>,
        after: Arc<Vec<kentos_contracts::OptionSet>>,
    },
    /// The project's combinations of design options (docs/adr/0808), before and after.
    OptionCombos {
        before: Arc<Vec<kentos_contracts::OptionCombo>>,
        after: Arc<Vec<kentos_contracts::OptionCombo>>,
    },
    /// Mimari denetim's exceptions (docs/adr/0732 §3), before and after: a setting, written as a step of its own so a
    /// mark given by mistake is undone.
    AuditExceptions {
        before: Arc<Vec<kentos_contracts::ArchAuditException>>,
        after: Arc<Vec<kentos_contracts::ArchAuditException>>,
    },
    /// The building's declared fire protection class (docs/adr/8068), before and after: a step of its own.
    Performance {
        before: Option<kentos_contracts::BuildingPerformance>,
        after: Option<kentos_contracts::BuildingPerformance>,
    },
    /// The author's answers to the model quality control form (docs/adr/8077), before and after: a step of its own.
    FormAnswers {
        before: Arc<Vec<kentos_contracts::FormAnswer>>,
        after: Arc<Vec<kentos_contracts::FormAnswer>>,
    },
    /// The delivery record (docs/adr/8099), before and after: a step of its own.
    Deliveries {
        before: Arc<Vec<kentos_contracts::Delivery>>,
        after: Arc<Vec<kentos_contracts::Delivery>>,
    },
    /// The underlay models (docs/adr/8101), before and after: a step of its own.
    Underlays {
        before: Arc<Vec<kentos_contracts::ModelUnderlay>>,
        after: Arc<Vec<kentos_contracts::ModelUnderlay>>,
    },
    /// The project's element types (docs/adr/8106), before and after: a setting, written in the step with the
    /// elements that follow them, so undo gives both back.
    ElementTypes {
        before: Arc<Vec<kentos_contracts::ElementType>>,
        after: Arc<Vec<kentos_contracts::ElementType>>,
    },
}

impl ArchOp {
    pub(crate) fn inverse(&self) -> Self {
        match self {
            ArchOp::OpeningTypes { before, after } => ArchOp::OpeningTypes {
                before: after.clone(),
                after: before.clone(),
            },
            ArchOp::Materials { before, after } => ArchOp::Materials {
                before: after.clone(),
                after: before.clone(),
            },
            ArchOp::BuildingProgram { before, after } => ArchOp::BuildingProgram {
                before: after.clone(),
                after: before.clone(),
            },
            ArchOp::ViewArea { key, before, after } => ArchOp::ViewArea {
                key: key.clone(),
                before: *after,
                after: *before,
            },
            ArchOp::Modules { before, after } => ArchOp::Modules {
                before: after.clone(),
                after: before.clone(),
            },
            ArchOp::Options { before, after } => ArchOp::Options {
                before: after.clone(),
                after: before.clone(),
            },
            ArchOp::OptionCombos { before, after } => ArchOp::OptionCombos {
                before: after.clone(),
                after: before.clone(),
            },
            ArchOp::AuditExceptions { before, after } => ArchOp::AuditExceptions {
                before: after.clone(),
                after: before.clone(),
            },
            ArchOp::Performance { before, after } => ArchOp::Performance {
                before: after.clone(),
                after: before.clone(),
            },
            ArchOp::FormAnswers { before, after } => ArchOp::FormAnswers {
                before: after.clone(),
                after: before.clone(),
            },
            ArchOp::Deliveries { before, after } => ArchOp::Deliveries {
                before: after.clone(),
                after: before.clone(),
            },
            ArchOp::Underlays { before, after } => ArchOp::Underlays {
                before: after.clone(),
                after: before.clone(),
            },
            ArchOp::ElementTypes { before, after } => ArchOp::ElementTypes {
                before: after.clone(),
                after: before.clone(),
            },
        }
    }

    pub(crate) fn apply(&self, doc: &mut Document) {
        match self {
            ArchOp::OpeningTypes { after, .. } => {
                let mut arch = Arc::make_mut(&mut doc.settings)
                    .arch
                    .take()
                    .unwrap_or_default();
                arch.opening_types = (**after).clone();
                Arc::make_mut(&mut doc.settings).arch = (!arch.is_empty()).then_some(arch);
            }
            ArchOp::Materials { after, .. } => {
                let mut arch = Arc::make_mut(&mut doc.settings)
                    .arch
                    .take()
                    .unwrap_or_default();
                arch.materials = (**after).clone();
                Arc::make_mut(&mut doc.settings).arch = (!arch.is_empty()).then_some(arch);
            }
            ArchOp::BuildingProgram { after, .. } => {
                let mut arch = Arc::make_mut(&mut doc.settings)
                    .arch
                    .take()
                    .unwrap_or_default();
                arch.program = (**after).clone();
                Arc::make_mut(&mut doc.settings).arch = (!arch.is_empty()).then_some(arch);
            }
            ArchOp::ViewArea { key, after, .. } => {
                let (key, after) = (key.clone(), *after);
                doc.put_view_area(&key, after);
            }
            ArchOp::Modules { after, .. } => {
                let mut arch = Arc::make_mut(&mut doc.settings)
                    .arch
                    .take()
                    .unwrap_or_default();
                arch.modules = (**after).clone();
                Arc::make_mut(&mut doc.settings).arch = (!arch.is_empty()).then_some(arch);
            }
            ArchOp::Options { after, .. } => {
                let mut arch = Arc::make_mut(&mut doc.settings)
                    .arch
                    .take()
                    .unwrap_or_default();
                arch.options = (**after).clone();
                Arc::make_mut(&mut doc.settings).arch = (!arch.is_empty()).then_some(arch);
            }
            ArchOp::OptionCombos { after, .. } => {
                let mut arch = Arc::make_mut(&mut doc.settings)
                    .arch
                    .take()
                    .unwrap_or_default();
                arch.option_combos = (**after).clone();
                Arc::make_mut(&mut doc.settings).arch = (!arch.is_empty()).then_some(arch);
            }
            ArchOp::AuditExceptions { after, .. } => {
                let mut arch = Arc::make_mut(&mut doc.settings)
                    .arch
                    .take()
                    .unwrap_or_default();
                arch.audit_exceptions = (**after).clone();
                // No empty settings are kept (a file refuses them).
                Arc::make_mut(&mut doc.settings).arch = (!arch.is_empty()).then_some(arch);
            }
            ArchOp::Performance { after, .. } => {
                let mut arch = Arc::make_mut(&mut doc.settings)
                    .arch
                    .take()
                    .unwrap_or_default();
                arch.performance = after.clone().filter(|p| !p.is_empty());
                Arc::make_mut(&mut doc.settings).arch = (!arch.is_empty()).then_some(arch);
            }
            ArchOp::FormAnswers { after, .. } => {
                let mut arch = Arc::make_mut(&mut doc.settings)
                    .arch
                    .take()
                    .unwrap_or_default();
                arch.form_answers = (**after).clone();
                Arc::make_mut(&mut doc.settings).arch = (!arch.is_empty()).then_some(arch);
            }
            ArchOp::Deliveries { after, .. } => {
                let mut arch = Arc::make_mut(&mut doc.settings)
                    .arch
                    .take()
                    .unwrap_or_default();
                arch.deliveries = (**after).clone();
                Arc::make_mut(&mut doc.settings).arch = (!arch.is_empty()).then_some(arch);
            }
            ArchOp::Underlays { after, .. } => {
                let mut arch = Arc::make_mut(&mut doc.settings)
                    .arch
                    .take()
                    .unwrap_or_default();
                arch.underlays = (**after).clone();
                Arc::make_mut(&mut doc.settings).arch = (!arch.is_empty()).then_some(arch);
            }
            ArchOp::ElementTypes { after, .. } => {
                let mut arch = Arc::make_mut(&mut doc.settings)
                    .arch
                    .take()
                    .unwrap_or_default();
                arch.element_types = (**after).clone();
                Arc::make_mut(&mut doc.settings).arch = (!arch.is_empty()).then_some(arch);
            }
        }
    }
}
