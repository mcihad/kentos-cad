//! Bind › Insert's layers for `cad.xrefs.edit` (docs/adr/0348 §3): the reference's layers joined to the drawing's of the
//! same path, its definitions' objects moved onto them. The rule is `kentos_domain::xref::bind_layers`'s.

use kentos_contracts::BlockDefinition;
use kentos_domain::Document;

use crate::exchange_write::{Counts, put_nodes};

/// Joins the layers under `group` and moves `defs`' objects onto the joined layers; gives the names that joined a
/// layer the drawing had. Inside the caller's undo group.
pub(crate) fn merge_layers(doc: &mut Document, group: &str, defs: &mut [BlockDefinition]) -> Result<Vec<String>, String> {
    let (layers, map, merged) = kentos_domain::xref::bind_layers(&doc.to_snapshot_v2(), group)?;
    let mut counts = Counts::default();
    put_nodes(doc, &layers, None, &mut counts, "Dış başvuruyu bağla")?;
    for d in defs.iter_mut() {
        for e in &mut d.entities {
            let base = e.base_mut();
            if let Some(to) = map.get(&base.layer_id) {
                base.layer_id = to.clone();
            }
        }
    }
    Ok(merged)
}
