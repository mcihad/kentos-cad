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
        }
    }
}
