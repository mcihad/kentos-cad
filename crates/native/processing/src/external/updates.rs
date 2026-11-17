//! An extension tool's changes to its own inputs (docs/adr/8057 §49, PY-TOOL-12): its code answers `update`, a list of
//! `{uid, attrs}`, and KentOS writes the attributes into the objects it was given, in the run's one undo step, the
//! layer's fields checking each value as for a built-in tool. Only an input whose parameter says so (`writes: true`
//! in the descriptor) can be changed: changing the source is the tool's declared, separate choice, never a side
//! effect; an object not given to the tool, or given through an input that does not write, is refused with nothing
//! written.

use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::Entity;
use serde_json::Value;

use super::{ExternalTool, MAX_OBJECTS, attribute};
use crate::types::{ParamKind, Patch, Resolved, RunContext};

/// The patches `answer`'s `update` asks for, or why not (in the answer's words, with the change's place): each
/// object's attributes as they are with the named ones set (`None` takes one away).
pub(super) fn updates_of(
    tool: &ExternalTool,
    answer: &Value,
    resolved: &Resolved<'_>,
    ctx: &RunContext<'_>,
    given: &BTreeMap<String, &Entity>,
) -> Result<Vec<Patch>, String> {
    let Some(list) = answer.get("update").filter(|v| !v.is_null()) else {
        return Ok(Vec::new());
    };
    let list = list
        .as_array()
        .ok_or_else(|| "update bir değişiklik listesi ([{uid, attrs}]) olmalı.".to_owned())?;
    if list.len() > MAX_OBJECTS {
        return Err(format!("{MAX_OBJECTS} nesneden fazla değişiklik."));
    }
    // The objects of the inputs that say they write.
    let writable: BTreeMap<String, &Entity> = tool
        .tool
        .parameters
        .iter()
        .filter(|p| matches!(p.kind, ParamKind::Features { writes: true, .. }))
        .filter_map(|p| resolved.features.get(&p.name))
        .flat_map(|set| set.entities.iter().copied())
        .filter_map(|e| {
            Some((
                ctx.doc.uid(kentos_domain::Slot(e.base().id))?.to_string(),
                e,
            ))
        })
        .collect();
    let mut patches = Vec::with_capacity(list.len());
    let mut changed = BTreeSet::new();
    for (i, change) in list.iter().enumerate() {
        let at = format!("update[{i}]");
        let uid = change
            .get("uid")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(e) = writable.get(uid) else {
            return Err(if given.contains_key(uid) {
                format!(
                    "{at}: “{uid}” değiştirilemez: verildiği girdinin tanımında writes: true yok (kaynağı değiştirmek \
                     aracın tanımında açıkça söylenir)."
                )
            } else {
                format!("{at}: “{uid}” araca verilen nesnelerden biri değil.")
            });
        };
        if !changed.insert(uid) {
            return Err(format!("{at}: “{uid}” iki kez değiştiriliyor."));
        }
        let attrs = change
            .get("attrs")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{at}.attrs: öznitelikleri bir sözlük olmalı."))?;
        let mut merged: BTreeMap<String, String> = e.base().attrs.clone().into_iter().collect();
        for (key, value) in attrs {
            let key = key.trim();
            if key.is_empty() {
                return Err(format!("{at}.attrs: öznitelik adı boş olamaz."));
            }
            match attribute(key, value).map_err(|why| format!("{at}: {why}"))? {
                Some(text) => {
                    merged.insert(key.to_owned(), text);
                }
                None => {
                    merged.remove(key);
                }
            }
        }
        patches.push(Patch {
            id: kentos_domain::Slot(e.base().id),
            attrs: Some(merged),
            label: None,
            zs: None,
            geometry: None,
            layer: None,
        });
    }
    Ok(patches)
}
