//! A tool's own marks (docs/adr/9401 §10): what a tool draws anew each time it runs (a colour plan's legend, a storey's
//! escape walks, the shadows, a site plan's dimensions) carries an attribute, and before the new marks are drawn the
//! old ones go. They go by the drawing's rules, as any object does:
//!
//! - An object on a locked layer is not removed. Then nothing is drawn either: the new marks beside the old ones would
//!   be two sets on the plan. The refusal names the layer and how to open it.
//! - The removal is the delete command's own (`cad.entities.delete`): what it checks for any object is checked for a
//!   mark (a relation that keeps a parent, one that would change a locked child).
//!
//! The caller holds the undo group the removal and the new marks share.

use kentos_commands::{CommandResult, EntitiesDelete};
use kentos_domain::{Document, Slot};

use crate::{ExecutionContext, delete};

/// Takes away every object that carries the attribute `attr`, before its tool draws them again; how many went.
///
/// `old` names them where a refusal says them (“Eski lejant”). Refused, with nothing removed, when one of them is on a
/// locked layer (locked by itself or by a group above it).
pub fn take_away(doc: &mut Document, attr: &str, old: &str) -> Result<usize, String> {
    let layers = doc.layers();
    let mut uids = Vec::new();
    for e in doc.entities().filter(|e| e.base().attrs.contains_key(attr)) {
        let layer = &e.base().layer_id;
        if layers.is_locked(layer) {
            let name = layers
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            return Err(format!(
                "{old} kilitli “{name}” katmanında; yerine yenisi çizilemez. Kilidi Katmanlar panelinden açın."
            ));
        }
        if let Some(uid) = doc.uid(Slot(e.base().id)) {
            uids.push(uid.to_string());
        }
    }
    if uids.is_empty() {
        return Ok(0);
    }
    let input = EntitiesDelete {
        uids,
        expected_revision: None,
    };
    match delete::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { output, .. } => Ok(output.removed.len()),
        other => Err(crate::said_of(other)
            .into_iter()
            .next()
            .unwrap_or_else(|| format!("{old} kaldırılamadı; aracı yeniden çalıştırın."))),
    }
}
