//! The plot scale or the project's annotation heights changed from the
//! interface (docs/adr/0205 §3; the web's `app/annotationScale.ts`): the
//! texts, leaders, dimensions and tables still at the old height take the
//! new one (`follow_annotation_scale`, the shared rule) in one undo step
//! “Yazı yüksekliklerini uydur” (`cad.entities.edit`'s `annotationScale`);
//! what was changed by hand stays; those on a locked layer are not written
//! and are counted. The setting itself is an edit of the project, not an
//! undo step.

use kentos_contracts::{
    CommandResult, EditOperation, EntitiesEdit, EntityEdit, ScaleChange, follow_annotation_scale,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::geometry::{edit_geometry, shape};
use kentos_native_application::{ExecutionContext, edit};

/// What following a change did: the objects written, those a locked layer
/// kept, and the command's refusal when it refused.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Followed {
    pub written: usize,
    pub locked: usize,
    pub refused: Option<String>,
}

/// The annotations at the old height given the new one, in one step; none
/// when nothing they depend on changed.
pub fn follow_annotation_change(doc: &mut Document, change: &ScaleChange) -> Followed {
    if change.is_none() {
        return Followed::default();
    }
    let settings = doc.settings();
    let (texts, dims) = (
        settings.text_styles.clone(),
        settings.dimension_styles.clone(),
    );
    let mut locked = 0;
    let mut changes = Vec::new();
    let slots: Vec<Slot> = doc.entities().map(|e| Slot(e.base().id)).collect();
    for slot in slots {
        let Some(e) = doc.get(slot) else {
            continue;
        };
        let Some(next) = follow_annotation_scale(e, change, &texts, &dims) else {
            continue;
        };
        if doc.layers().is_locked(&e.base().layer_id) {
            locked += 1;
            continue;
        }
        let (Some(uid), Some(geometry)) = (doc.uid(slot), edit_geometry(shape(&next))) else {
            continue;
        };
        changes.push(EntityEdit::Update {
            uid: uid.to_string(),
            geometry,
        });
    }
    if changes.is_empty() {
        return Followed {
            locked,
            ..Followed::default()
        };
    }
    let count = changes.len();
    let input = EntitiesEdit {
        operation: EditOperation::AnnotationScale,
        changes,
        expected_revision: None,
    };
    match edit::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { .. } => Followed {
            written: count,
            locked,
            refused: None,
        },
        other => Followed {
            written: 0,
            locked,
            refused: crate::properties::said_of(other).into_iter().next(),
        },
    }
}
