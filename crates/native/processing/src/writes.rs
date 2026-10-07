//! A run's attribute writes against the layers' fields (docs/adr/0199 §1,
//! 0200 §3; the web's `processing/writeCheck.ts`), before the runner applies
//! them: each value given to a field becomes the field's canonical text, and
//! the first that does not keep the field's rules refuses the whole run with
//! its reason and its object. Updates are checked first, in their order, each
//! one's changed keys in their code points' order; then new objects, every
//! key they are given. An attribute taken away is checked as empty (a
//! required field refuses it). Objects gone or on locked layers are not
//! checked: the runner leaves them out anyway.

use std::collections::BTreeSet;

use kentos_contracts::fields::check_value;
use kentos_domain::Document;

use crate::types::ChangeSet;

/// The changes with every value a field takes in its canonical text, or why
/// the run cannot write them.
pub fn check(doc: &Document, ch: &mut ChangeSet) -> Result<(), String> {
    let layers = doc.layers();
    for u in &mut ch.update {
        let Some(e) = doc.get(u.id) else {
            continue;
        };
        let Some(attrs) = u.attrs.as_mut() else {
            continue;
        };
        let base = e.base();
        let Some(node) = layers
            .get(&base.layer_id)
            .filter(|n| !n.fields.is_empty() && !layers.is_locked(&base.layer_id))
        else {
            continue;
        };
        let keys: BTreeSet<String> = base.attrs.keys().chain(attrs.keys()).cloned().collect();
        for key in keys {
            let Some(field) = node.fields.iter().find(|f| f.name == key) else {
                continue;
            };
            let given = attrs.get(&key);
            if given == base.attrs.get(&key) {
                continue;
            }
            match check_value(field, given.map_or("", String::as_str)) {
                Err(r) => {
                    return Err(format!(
                        "Öznitelik yazılamadı (#{}): {}",
                        base.id, r.message
                    ));
                }
                Ok(value) => {
                    if let Some(v) = attrs.get_mut(&key) {
                        *v = value;
                    }
                }
            }
        }
    }
    for n in &mut ch.add {
        let layer = n.base().layer_id.clone();
        let Some(node) = layers
            .get(&layer)
            .filter(|n| !n.fields.is_empty() && !layers.is_locked(&layer))
        else {
            continue;
        };
        for (key, value) in n.base_mut().attrs.iter_mut() {
            let Some(field) = node.fields.iter().find(|f| f.name == *key) else {
                continue;
            };
            match check_value(field, value) {
                Err(r) => return Err(format!("Öznitelik yazılamadı (yeni nesne): {}", r.message)),
                Ok(v) => *value = v,
            }
        }
    }
    Ok(())
}
