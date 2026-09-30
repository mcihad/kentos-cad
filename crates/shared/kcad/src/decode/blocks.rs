//! Block definitions read from document schema 6 (docs/specs/kcad-v2.md
//! §6.9, docs/adr/0144): each read whole, its objects as the drawing's are
//! but without persistent ids; then the block rules (`kentos_contracts::blocks`)
//! checked over the whole list, since an insert may name a definition listed
//! after its own. A fault is said where it is: at the insert for an unknown
//! block, else at the definition.

use std::collections::HashMap;

use kentos_contracts::blocks::{self, BlockFault};
use kentos_contracts::{AttributeDefinition, BlockDefinition, BlockId, Entity};

use super::objects::{self, Features, object, text_align};
use super::{id16, list, map, point, required, text, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

/// The definitions and each id's place in them.
pub(super) type Definitions = (Vec<BlockDefinition>, HashMap<BlockId, usize>);

pub(super) fn definitions(r: &mut Reader<'_>, has: Features) -> Result<Definitions, KcadError> {
    let at = r.position();
    let mut starts = Vec::new();
    // Where each insert's `block` is, by (definition, object).
    let mut inserts = HashMap::new();
    let list = list(r, |r, i| {
        starts.push(r.position());
        definition(r, i, has, &mut inserts)
    })?;
    if list.is_empty() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "blok listesi boş; tanımı olmayan çizimde alan yazılmaz",
        ));
    }
    let checked = blocks::definitions(&list).and_then(|index| {
        for (i, block) in list.iter().enumerate() {
            blocks::inserts(&block.entities, Some(i), &index)?;
        }
        blocks::nesting(&list, &index)?;
        Ok(index)
    });
    match checked {
        Ok(index) => Ok((list, index)),
        Err(fault) => {
            let place = match &fault {
                BlockFault::UnknownBlock { at } | BlockFault::BadScale { at } => at
                    .definition
                    .and_then(|d| inserts.get(&(d, at.entity)).copied()),
                _ => None,
            };
            let definition = match &fault {
                BlockFault::EmptyName { definition }
                | BlockFault::DuplicateId { definition, .. }
                | BlockFault::DuplicateName { definition, .. }
                | BlockFault::EmptyTag { definition, .. }
                | BlockFault::DuplicateTag { definition, .. }
                | BlockFault::Cycle { definition }
                | BlockFault::TooDeep { definition } => *definition,
                BlockFault::UnknownBlock { at } | BlockFault::BadScale { at } => {
                    at.definition.unwrap_or_default()
                }
            };
            let said = crate::blocks::said(&fault, &list);
            for seg in said.path {
                r.push(seg);
            }
            let at = place.unwrap_or_else(|| starts.get(definition).copied().unwrap_or(at));
            Err(r.fail_at(said.code, at, &said.what))
        }
    }
}

fn definition(
    r: &mut Reader<'_>,
    i: usize,
    has: Features,
    inserts: &mut HashMap<(usize, usize), usize>,
) -> Result<BlockDefinition, KcadError> {
    let (mut id, mut base, mut name, mut entities) = (None, None, None, None);
    let (mut attributes, mut description) = (None, None);
    let inner = has.without_uids();
    map(r, |r, key| {
        match key {
            "id" => id = Some(BlockId(id16(r)?)),
            "base" => base = Some(point(r)?),
            "name" => name = Some(text(r)?),
            "entities" => {
                entities = Some(list(r, |r, j| {
                    let (entity, _, block_at) = object(r, j, inner)?;
                    if matches!(entity, Entity::Insert(_)) {
                        inserts.insert((i, j), block_at);
                    }
                    Ok(entity)
                })?)
            }
            "attributes" => {
                let at = r.position();
                let list = list(r, |r, _| attribute(r, has))?;
                if list.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "öznitelik listesi boş; öznitelik tanımı yoksa alan yazılmaz",
                    ));
                }
                attributes = Some(list);
            }
            "description" => description = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(BlockDefinition {
        id: required(r, id, "id")?,
        name: required(r, name, "name")?,
        base: required(r, base, "base")?,
        entities: required(r, entities, "entities")?,
        attributes: attributes.unwrap_or_default(),
        description,
    })
}

/// An attribute definition; `has`: whether the payload's schema gives it an
/// alignment and a width factor (schema 7, docs/adr/0145).
fn attribute(r: &mut Reader<'_>, has: Features) -> Result<AttributeDefinition, KcadError> {
    let (mut p, mut tag, mut value, mut height, mut prompt, mut rotation) =
        (None, None, None, None, None, None);
    let (mut align, mut width_factor) = (None, None);
    map(r, |r, key| {
        match key {
            "p" => p = Some(point(r)?),
            "tag" => tag = Some(text(r)?),
            "value" => value = Some(text(r)?),
            "height" => height = Some(r.float()?),
            "prompt" => prompt = Some(text(r)?),
            "rotation" => rotation = Some(r.float()?),
            "align" if has.texts => align = Some(text_align(r)?),
            "widthFactor" if has.texts => width_factor = Some(objects::width_factor(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(AttributeDefinition {
        tag: required(r, tag, "tag")?,
        prompt,
        value,
        p: required(r, p, "p")?,
        height: required(r, height, "height")?,
        rotation: required(r, rotation, "rotation")?,
        align,
        width_factor,
    })
}

/// The unknown block of the drawing's own insert `entity`, said as the
/// definitions' faults are; the path is at the drawing's objects.
pub(super) fn unknown_block(
    r: &mut Reader<'_>,
    entity: usize,
    at: usize,
    list: &[BlockDefinition],
) -> KcadError {
    let fault = BlockFault::UnknownBlock {
        at: blocks::Place {
            definition: None,
            entity,
        },
    };
    let said = crate::blocks::said(&fault, list);
    for seg in said.path {
        r.push(seg);
    }
    r.fail_at(said.code, at, &said.what)
}
