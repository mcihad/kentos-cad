//! Explicit tag-identity migration; definition-local edits are opt-in and share root rename rules.
use std::collections::{BTreeMap, HashMap, HashSet};

use kentos_commands::{BlockDefinition, Entity, InsertEntity};
use kentos_domain::Document;

use crate::blocks_edit::DefinitionChanges;
use crate::checks::{Stop, error};
use crate::codes;

pub(super) struct Renamed {
    pub changed: bool,
    /// Preserve the published root report: tags with a stored value, not position-only migrations.
    pub tags: HashSet<String>,
}

/// Read every source before writing any target, including simultaneous swaps and chains.
pub(super) fn apply(
    insert: &mut InsertEntity,
    renames: &BTreeMap<String, String>,
    protect_targets: bool,
    context: &str,
) -> Result<Renamed, Stop> {
    for (from, to) in renames {
        let value_conflict = insert
            .base
            .attrs
            .get(from)
            .zip(insert.base.attrs.get(to))
            .is_some_and(|(a, b)| a != b);
        let place_conflict = insert
            .attribute_places
            .get(from)
            .zip(insert.attribute_places.get(to))
            .is_some_and(|(a, b)| a != b);
        if protect_targets && !renames.contains_key(to) && (value_conflict || place_conflict) {
            return Err(Stop::failed(error(
                codes::DUPLICATE_TAG,
                format!(
                    "“{from}” → “{to}”: {context} hedef etikette başka bir değer veya özel konum var. Değerin üzerine yazılmadı; farklı bir etiket seçin ya da mevcut değerleri önce düzenleyin."
                ),
                Some(format!("renames.{from}")),
            )));
        }
    }
    let values: Vec<_> = renames
        .iter()
        .filter_map(|(from, to)| {
            insert
                .base
                .attrs
                .get(from)
                .map(|v| (from.clone(), to.clone(), v.clone()))
        })
        .collect();
    let places: Vec<_> = renames
        .iter()
        .filter_map(|(from, to)| {
            insert
                .attribute_places
                .get(from)
                .map(|p| (from.clone(), to.clone(), *p))
        })
        .collect();
    let mut result = Renamed {
        changed: !values.is_empty() || !places.is_empty(),
        tags: HashSet::new(),
    };
    for (from, _, _) in &values {
        insert.base.attrs.remove(from);
    }
    for (from, _, _) in &places {
        insert.attribute_places.remove(from);
    }
    for (from, to, value) in values {
        insert.base.attrs.insert(to, value);
        result.tags.insert(from);
    }
    for (_, to, place) in places {
        insert.attribute_places.insert(to, place);
    }
    Ok(result)
}

fn locked(message: String) -> Stop {
    Stop::failed(error(
        codes::LAYER_LOCKED,
        message,
        Some("renameNested".into()),
    ))
}

/// Migrate only the explicit tag identities in inserts owned by other definitions.
/// Defaults, unrelated attributes, field caches, geometry and parameter inputs stay as published.
pub(crate) fn nested(
    doc: &Document,
    old: &BlockDefinition,
    next: &BlockDefinition,
    renames: &BTreeMap<String, String>,
    roots: &[(kentos_domain::Slot, Entity)],
) -> Result<DefinitionChanges, Stop> {
    super::renames_ok(next, renames)?;
    let mut blocks = Vec::new();
    for parent in doc.blocks() {
        if parent.id == old.id {
            continue;
        }
        let mut changed = (**parent).clone();
        for entity in &mut changed.entities {
            let Entity::Insert(insert) = entity else {
                continue;
            };
            if insert.block != old.id {
                continue;
            }
            let previous = insert.clone();
            let context = format!(
                "“{}” tanımının {} numaralı iç yerleştirmesinde",
                parent.name, insert.base.id
            );
            apply(insert, renames, true, &context)?;
            if *insert != previous && doc.layers().is_locked(&insert.base.layer_id) {
                return Err(locked(format!(
                    "{context} katman kilitli. İç etiketler ve çizim değerleri değiştirilmedi."
                )));
            }
        }
        if changed != **parent {
            if parent.xref.is_some() {
                return Err(Stop::failed(error(
                    codes::BLOCK_REFUSED,
                    format!(
                        "“{}” dış başvuruya ait; iç etiketlerini kaynak dosyasında düzenleyin. Çizim değiştirilmedi.",
                        parent.name
                    ),
                    Some("renameNested".into()),
                )));
            }
            if let Some(card) = changed.card.as_mut() {
                card.version = card.version.saturating_add(1);
            }
            blocks.push(changed);
        }
    }
    // A definition cannot be partly applied to a locked use of its own or any ancestor.
    let mut affected: HashSet<_> = blocks.iter().map(|b| b.id).collect();
    if old.attributes != next.attributes {
        affected.insert(old.id);
    }
    loop {
        let count = affected.len();
        for parent in doc.blocks() {
            if parent
                .entities
                .iter()
                .any(|e| matches!(e,Entity::Insert(i) if affected.contains(&i.block)))
            {
                affected.insert(parent.id);
            }
        }
        if count == affected.len() {
            break;
        }
    }
    for parent in doc.blocks() {
        for entity in &parent.entities {
            if let Entity::Insert(insert) = entity
                && affected.contains(&insert.block)
                && doc.layers().is_locked(&insert.base.layer_id)
            {
                return Err(locked(format!(
                    "“{}” tanımının {} numaralı iç yerleştirmesi etkilenen tanımı kilitli katmanda kullanıyor. Hiçbir değişiklik yazılmadı.",
                    parent.name, insert.base.id
                )));
            }
        }
    }
    for entity in doc.entities() {
        let Entity::Insert(insert) = entity else {
            continue;
        };
        if !doc.layers().is_locked(&insert.base.layer_id) {
            continue;
        }
        let mut changes_literal = false;
        if insert.block == old.id {
            let mut copy = insert.clone();
            let context = format!("{} numaralı yerleştirmede", insert.base.id);
            apply(
                &mut copy,
                renames,
                old.attributes != next.attributes,
                &context,
            )?;
            changes_literal = copy != *insert;
        }
        if affected.contains(&insert.block) || changes_literal {
            return Err(locked(format!(
                "{} numaralı yerleştirme etkilenen blok veya üst tanımın kilitli kullanımı. İç içe etiket taşımadan önce katmanın kilidini açın; Hiçbir değişiklik yazılmadı.",
                insert.base.id
            )));
        }
    }
    let replacements: HashMap<_, _> = blocks
        .iter()
        .map(|b| (b.id, b))
        .chain([(next.id, next)])
        .collect();
    let definitions: Vec<_> = doc
        .blocks()
        .iter()
        .map(|b| replacements.get(&b.id).copied().unwrap_or(b))
        .collect();
    let roots: HashMap<_, _> = roots.iter().map(|(slot, e)| (slot.0, e)).collect();
    let entities: Vec<_> = doc
        .entities()
        .map(|e| roots.get(&e.base().id).copied().unwrap_or(e))
        .collect();
    crate::blocks_define::block_rules_with_entities(&definitions, &entities)?;
    Ok(DefinitionChanges {
        blocks,
        entities: Vec::new(),
    })
}
