//! What an extension's audit rule is given, and what the drawing it read is known by, the same on the desktop and in a
//! cloud job (plan/eklenti.md ADIM 20, docs/adr/8057 §42): one function each, so the two surfaces cannot drift apart.
//!
//! - [`facts_of`]: the facts the rule reads (`facts`): the layers' objects as data (`external::audit_facts`, at most
//!   [`MOST_FEATURES`], a field the drawing hides cut) and what KentOS measured in the drawing (`project`,
//!   `audit_facts::project_facts`, at most its `MOST_OBJECTS`). A measure that breaks on some drawing leaves the rule
//!   the layers (`facts.project` then says it is missing), as the desktop's audit always did.
//! - [`message`]: the `audit` invoke its process is sent (the caller adds the rule's local id and handler).
//! - [`drawing_stamp`]: the drawing's content stamp: every object's stamp in persistent-id order, every block
//!   definition's in id order, and the layer tree's, settings', styles', constraints', dimension sets' and sheet
//!   book's (`kentos_domain::etag`, docs/adr/0253 §2). It comes from the content alone, so a drawing opened on the
//!   desktop and the same drawing a job read from the server's file stamp alike.

use std::panic::AssertUnwindSafe;

use kentos_commands::canonical::stamp_of;
use kentos_domain::Document;
use kentos_domain::etag::{block_stamp, entity_stamp};
use serde_json::{Value, json};

/// The most objects a rule is given as data: past them the facts are cut and the rule is told (`facts.cut`). The
/// desktop's Eklenti denetimi and a cloud job give the same.
pub const MOST_FEATURES: usize = 50_000;

/// The facts an audit rule reads; none when `stopped` said to stop while the drawing was measured.
pub fn facts_of(doc: &Document, stopped: &dyn Fn() -> bool) -> Option<Value> {
    let mut facts = crate::external::audit_facts(doc, MOST_FEATURES);
    let measured = std::panic::catch_unwind(AssertUnwindSafe(|| {
        crate::audit_facts::project_facts(doc, crate::audit_facts::MOST_OBJECTS, stopped)
    }));
    match measured {
        Ok(None) => return None,
        Ok(Some(project)) => {
            facts["project"] = serde_json::to_value(&project).unwrap_or_default();
        }
        Err(_) => {}
    }
    Some(facts)
}

/// The `audit` invoke a rule's process is sent, without its id, local id and handler.
pub fn message(facts: Value) -> Value {
    json!({ "kind": "audit", "facts": facts })
}

/// The drawing's content stamp: 16 hex digits of the SHA-256 of its parts' stamps.
pub fn drawing_stamp(doc: &Document) -> String {
    let mut objects: Vec<_> = doc
        .entities_with_uids()
        .map(|(uid, e)| (uid, entity_stamp(uid, e)))
        .collect();
    objects.sort_by_key(|(uid, _)| *uid);
    let mut blocks: Vec<_> = doc
        .blocks()
        .iter()
        .map(|b| (b.id, block_stamp(b)))
        .collect();
    blocks.sort_by_key(|(id, _)| *id);
    stamp_of(&json!({
        "objects": objects.into_iter().map(|(_, s)| s).collect::<Vec<_>>(),
        "blocks": blocks.into_iter().map(|(_, s)| s).collect::<Vec<_>>(),
        "layers": doc.layers_stamp(),
        "settings": doc.settings_stamp(),
        "styles": doc.styles_stamp(),
        "constraints": doc.constraints_stamp(),
        "dimensionSets": doc.dimension_sets_stamp(),
        "sheets": doc.sheets_stamp(),
    }))
}

#[cfg(test)]
mod tests;
