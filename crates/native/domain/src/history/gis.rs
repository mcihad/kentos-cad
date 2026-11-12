//! A GIS project's settings in the undo history (docs/adr/8054 §3): value domains, bound sources and lineage.

use std::sync::Arc;

use super::SourceState;
use crate::document::Document;

#[derive(Clone, Debug)]
pub(crate) enum GisOp {
    /// The project's value lists (docs/adr/0430), before and after: a setting, written in the step
    /// with the linked fields it rewrote, so undo gives both back.
    Domains {
        before: Arc<Vec<kentos_contracts::FieldDomain>>,
        after: Arc<Vec<kentos_contracts::FieldDomain>>,
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
}

impl GisOp {
    pub(crate) fn inverse(&self) -> Self {
        match self {
            GisOp::Domains { before, after } => GisOp::Domains {
                before: after.clone(),
                after: before.clone(),
            },
            GisOp::Source { id, before, after } => GisOp::Source {
                id: id.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            GisOp::Lineage { before, after } => GisOp::Lineage {
                before: after.clone(),
                after: before.clone(),
            },
        }
    }

    pub(crate) fn apply(&self, doc: &mut Document) {
        match self {
            GisOp::Domains { after, .. } => {
                Arc::make_mut(&mut doc.settings).domains = (**after).clone();
            }
            GisOp::Source { id, after, .. } => {
                let sources = &mut Arc::make_mut(&mut doc.settings).sources;
                match (&after.link, sources.iter().position(|l| l.id == *id)) {
                    (Some(link), Some(i)) => sources[i] = link.clone(),
                    (Some(link), None) => sources.push(link.clone()),
                    (None, Some(i)) => {
                        sources.remove(i);
                    }
                    (None, None) => {}
                }
                let bases = &mut Arc::make_mut(&mut doc.settings).source_bases;
                bases.retain(|b| b.source != *id);
                if let Some(b) = &after.base {
                    bases.push(b.clone());
                }
            }
            GisOp::Lineage { after, .. } => {
                Arc::make_mut(&mut doc.settings).lineage = (**after).clone();
            }
        }
    }
}
