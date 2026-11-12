//! Scoped block-unit correction: definition coordinates and every reference's local
//! values change together. Parent/world coordinates and unrelated definitions stay.
use std::collections::{BTreeMap, HashMap, HashSet};

use kentos_commands::{BlockDefinition, BlockId, Entity, InsertEntity, units::Scale};
use kentos_domain::{Document, Slot};
use serde::{Serialize, de::DeserializeOwned};

use crate::checks::{Stop, error};
use crate::codes;

use super::DefinitionChanges;

fn refused(code: &'static str, message: String) -> Stop {
    Stop::failed(error(code, message, Some("rescale".into())))
}

// JSON's null cannot stand for a non-finite coordinate. A typed round trip also
// catches an optional numeric field silently turning into None. This runs only
// on changed objects, never on the entire drawing for a metadata-only edit.
fn representable<T: Serialize + DeserializeOwned + PartialEq>(value: &T) -> Result<(), Stop> {
    let same = serde_json::to_value(value)
        .ok()
        .and_then(|v| serde_json::from_value::<T>(v).ok())
        .is_some_and(|v| v == *value);
    if same {
        Ok(())
    } else {
        Err(refused(codes::INVALID_SCALE,
            "Birim dönüşümü saklanabilir sayı aralığını aşıyor; hiçbir şey değiştirilmedi. Daha küçük bir dönüşüm seçin.".into()))
    }
}

fn reference(
    insert: &mut InsertEntity,
    old: &BlockDefinition,
    next: &BlockDefinition,
    scale: Scale,
) -> Result<(), Stop> {
    kentos_commands::units::scale_insert_details(insert, scale);
    if let (Some(old), Some(new)) = (&old.dynamic, &next.dynamic) {
        kentos_commands::units::scale_insert_parameters(insert, old, scale);
        // Formula outputs are caches, not user inputs. Recompute them using the
        // conjugated expression after scaling only the stored literal inputs.
        let inputs: BTreeMap<_, _> = insert
            .parameters
            .iter()
            .filter(|(name, _)| {
                !old.parameters.iter().any(|p| {
                    p.name() == name.as_str() && p.rule().is_some_and(|r| r.expression.is_some())
                })
            })
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        insert.parameters = crate::block_params::resolve(new, &BTreeMap::new(), &inputs)
            .map_err(|(code, _, why)| refused(code, why))?;
    }
    if let Some(why) = insert.clipping.as_ref().and_then(|c| c.problem()) {
        return Err(refused(
            codes::INVALID_SCALE,
            format!("Birim dönüşümünden sonra {why} Hiçbir şey değiştirilmedi."),
        ));
    }
    representable(insert)
}

pub(super) fn check(
    doc: &Document,
    old: &BlockDefinition,
    next: &BlockDefinition,
    scale: Scale,
) -> Result<DefinitionChanges, Stop> {
    // A locked placement of an ancestor is affected too. Unrelated locked
    // objects do not prevent this scoped correction.
    let mut affected = HashSet::from([old.id]);
    loop {
        let count = affected.len();
        for parent in doc.blocks() {
            if parent
                .entities
                .iter()
                .any(|e| matches!(e, Entity::Insert(i) if affected.contains(&i.block)))
            {
                affected.insert(parent.id);
            }
        }
        if affected.len() == count {
            break;
        }
    }
    for entity in doc.entities() {
        if let Entity::Insert(i) = entity
            && affected.contains(&i.block)
            && doc.layers().is_locked(&i.base.layer_id)
        {
            return Err(refused(codes::LAYER_LOCKED,
                "Bu blok veya onu içeren bir blok kilitli katmanda yerleştirilmiş. Boyut değiştiren birim dönüşümünden önce katmanın kilidini açın; boyları koruyan birim seçimi kullanılabilir.".into()));
        }
    }
    representable(next)?;
    if let Some((_, why)) = next
        .dynamic
        .as_ref()
        .and_then(crate::block_params::expressions_problem)
    {
        return Err(refused(codes::INVALID_DYNAMIC_BLOCK, why));
    }
    let mut blocks = Vec::new();
    for parent in doc.blocks() {
        if parent.id == old.id {
            continue;
        }
        if !parent
            .entities
            .iter()
            .any(|e| matches!(e, Entity::Insert(i) if i.block == old.id))
        {
            continue;
        }
        let mut changed = (**parent).clone();
        for entity in &mut changed.entities {
            if let Entity::Insert(i) = entity
                && i.block == old.id
            {
                reference(i, old, next, scale)?;
            }
        }
        if changed != **parent {
            blocks.push(changed);
        }
    }
    let mut entities = Vec::new();
    for entity in doc.entities() {
        if let Entity::Insert(i) = entity
            && i.block == old.id
        {
            let mut changed = i.clone();
            reference(&mut changed, old, next, scale)?;
            if changed != *i {
                entities.push((Slot(i.base.id), Entity::Insert(changed)));
            }
        }
    }
    let replacements: HashMap<BlockId, _> = blocks
        .iter()
        .map(|b| (b.id, b))
        .chain([(next.id, next)])
        .collect();
    let definitions: Vec<_> = doc
        .blocks()
        .iter()
        .map(|b| replacements.get(&b.id).copied().unwrap_or(b))
        .collect();
    let replacements: HashMap<_, _> = entities.iter().map(|(s, e)| (*s, e)).collect();
    let scene: Vec<_> = doc
        .entities()
        .map(|e| replacements.get(&Slot(e.base().id)).copied().unwrap_or(e))
        .collect();
    crate::blocks_define::block_rules_with_entities(&definitions, &scene)?;
    Ok(DefinitionChanges { blocks, entities })
}
