//! Örnekleme (docs/adr/0220; the web's `builtin/sampling/`): grids of cells
//! and lines, regular and seeded random points, points along paths and
//! objects cut into pieces of at most so many vertices. The geometry is the
//! core's (`ops::sampling`); what the tools share is here.

pub mod along;
pub mod grid;
pub mod points;
pub mod subdivide;

use kentos_contracts::Entity;
use serde_json::json;

use crate::types::{ChangeSet, Feedback, RunResult, ScopeKind};

/// The scopes the tools offer, a layer first.
pub fn scopes() -> Option<Vec<ScopeKind>> {
    Some(vec![
        ScopeKind::Layer,
        ScopeKind::Selection,
        ScopeKind::Visible,
        ScopeKind::All,
    ])
}

/// The kinds that are paths (Rastgele noktalar on lines, Hat boyunca nokta).
pub const LINE_KINDS: [&str; 3] = ["line", "polyline", "arc"];

/// The note about curves taken as chords (an ellipse, a curve).
pub fn curved_note(feedback: &mut dyn Feedback, inputs: &[&Entity]) {
    let c = inputs
        .iter()
        .filter(|e| matches!(e, Entity::Ellipse(_) | Entity::Spline(_)))
        .count();
    if c > 0 {
        feedback.warn(format!(
            "{c} nesnenin eğrileri 0,1 mm'lik kirişlerle alındı."
        ));
    }
}

/// A run that wrote these objects.
pub fn written(add: Vec<Entity>, summary: String) -> RunResult {
    let count = add.len();
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [("count".to_owned(), json!(count))].into_iter().collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}
