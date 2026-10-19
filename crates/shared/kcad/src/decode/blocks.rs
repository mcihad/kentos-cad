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
            blocks::places(&list, &block.entities, Some(i), &index)?;
        }
        blocks::nesting(&list, &index)?;
        Ok(index)
    });
    match checked {
        Ok(index) => Ok((list, index)),
        Err(fault) => {
            let place = match &fault {
                BlockFault::UnknownBlock { at }
                | BlockFault::BadScale { at }
                | BlockFault::BadValues { at, .. }
                | BlockFault::BadPlace { at, .. } => at
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
                | BlockFault::TooDeep { definition }
                | BlockFault::TooLarge { definition }
                | BlockFault::BadAttribute { definition, .. }
                | BlockFault::BadCard { definition, .. }
                | BlockFault::BadDynamic { definition, .. } => *definition,
                BlockFault::UnknownBlock { at }
                | BlockFault::BadScale { at }
                | BlockFault::BadValues { at, .. }
                | BlockFault::BadPlace { at, .. } => {
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
    let (mut attributes, mut description, mut units, mut card) = (None, None, None, None);
    let mut dynamic = None;
    let mut xref = None;
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
            // Schema 58 (docs/adr/0317 §4).
            "card" if has.block_cards => card = Some(block_card(r)?),
            // Schema 59 (docs/adr/0318): its JSON, read back exactly as the writer writes it.
            "dynamic" if has.dynamic_blocks => {
                let at = r.position();
                let value = r.opaque()?;
                if let Some(key) = unknown_key(&value) {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        &format!("dinamik bölümde bilinmeyen alan: {key}"),
                    ));
                }
                let read: kentos_contracts::DynamicBlock =
                    serde_json::from_value(value.clone()).map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("dinamik bölüm okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "dinamik bölüm yazıcının biçiminde değil (varsayılan değer yazılmış ya da ondalık yerine tam sayı)",
                    ));
                }
                dynamic = Some(read);
            }
            // Schema 83 (docs/adr/0348 §1): its JSON, read back exactly as the writer writes it.
            "xref" if has.xrefs => {
                let at = r.position();
                let value = r.opaque()?;
                const KEYS: &[&str] = &["path", "relative", "kind", "digest", "unloaded", "group", "parent", "objects"];
                if let Some(key) = value.as_object().and_then(|m| m.keys().find(|k| !KEYS.contains(&k.as_str()))) {
                    return Err(r.fail_at(Code::UnknownField, at, &format!("dış başvuruda bilinmeyen alan: {key}")));
                }
                let read: kentos_contracts::XrefSource = serde_json::from_value(value.clone()).map_err(|e| {
                    let words = e.to_string();
                    let code = if words.contains("unknown field") { Code::UnknownField } else { Code::BadValue };
                    r.fail_at(code, at, &format!("dış başvuru okunamadı: {words}"))
                })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "dış başvuru yazıcının biçiminde değil (varsayılan değer yazılmış)",
                    ));
                }
                if let Some(why) = read.problem() {
                    return Err(r.fail_at(Code::BadValue, at, &why));
                }
                xref = Some(read);
            }
            // Schema 57 (docs/adr/0315 §4).
            "units" if has.block_extras => {
                let at = r.position();
                let word = text(r)?;
                use kentos_contracts::BlockUnits as B;
                units = Some(match word.as_str() {
                    "mm" => B::Mm,
                    "cm" => B::Cm,
                    "m" => B::M,
                    "in" => B::In,
                    "ft" => B::Ft,
                    "unitless" => B::Unitless,
                    _ => {
                        return Err(r.fail_at(
                            Code::BadValue,
                            at,
                            &format!("“{word}” bir blok birimi değil (mm, cm, m, in, ft, unitless)"),
                        ));
                    }
                });
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    // An external reference names one source object for each of its own (docs/adr/0348 §1).
    if let (Some(x), Some(es)) = (&xref, &entities)
        && !x.objects.is_empty()
        && x.objects.len() != es.len()
    {
        let x: &kentos_contracts::XrefSource = x;
        return Err(r.fail_at(
            Code::BadValue,
            r.position(),
            &format!(
                "dış başvurunun {} nesne kimliği var, tanımının {} nesnesi",
                x.objects.len(),
                es.len()
            ),
        ));
    }
    Ok(BlockDefinition {
        dynamic,
        xref,
        id: required(r, id, "id")?,
        name: required(r, name, "name")?,
        base: required(r, base, "base")?,
        entities: required(r, entities, "entities")?,
        attributes: attributes.unwrap_or_default(),
        description,
        units,
        card,
    })
}

/// A block's card (docs/adr/0317 §4); its rules are the definitions' (`blocks::card_problem`).
fn block_card(r: &mut Reader<'_>) -> Result<kentos_contracts::BlockCard, KcadError> {
    let (mut category, mut tags, mut version) = (None, None, None);
    map(r, |r, key| {
        match key {
            "category" => category = Some(text(r)?),
            "tags" => {
                let list = list(r, |r, _| text(r))?;
                if list.is_empty() {
                    return Err(r.fail(Code::BadValue, "boş etiket listesi yazılmaz"));
                }
                tags = Some(list);
            }
            "version" => {
                let at = r.position();
                let v = r.uint(u64::from(u32::MAX))?;
                version = Some(u32::try_from(v).map_err(|_| r.fail_at(Code::BadValue, at, "sürüm çok büyük"))?);
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(kentos_contracts::BlockCard {
        category,
        tags: tags.unwrap_or_default(),
        version: required(r, version, "version")?,
    })
}

/// An attribute definition; `has`: whether the payload's schema gives it an
/// alignment and a width factor (schema 7, docs/adr/0145).
fn attribute(r: &mut Reader<'_>, has: Features) -> Result<AttributeDefinition, KcadError> {
    let (mut p, mut tag, mut value, mut height, mut prompt, mut rotation) =
        (None, None, None, None, None, None);
    let (mut align, mut width_factor) = (None, None);
    let mut options = kentos_contracts::AttributeOptions::default();
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
            // Schema 58 (docs/adr/0317 §1): an option only when on.
            "invisible" | "constant" | "verify" | "preset" | "movable" | "multiline" if has.block_cards => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(Code::BadValue, at, "kapalı seçenek yazılmaz"));
                }
                match key {
                    "invisible" => options.invisible = true,
                    "constant" => options.constant = true,
                    "verify" => options.verify = true,
                    "preset" => options.preset = true,
                    "movable" => options.movable = true,
                    _ => options.multiline = true,
                }
            }
            "boxWidth" if has.block_cards => options.box_width = Some(r.float()?),
            "field" if has.block_cards => options.field = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(AttributeDefinition {
        options,
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


/// A key the dynamic part's contract does not know (docs/adr/0318), at any depth: the readers refuse it as an unknown
/// field before reading the part.
fn unknown_key(v: &serde_json::Value) -> Option<String> {
    use serde_json::Value;
    fn only(v: &Value, keys: &[&str]) -> Option<String> {
        let Value::Object(m) = v else { return None };
        m.keys().find(|k| !keys.contains(&k.as_str())).cloned()
    }
    fn each<'v>(v: &'v Value, key: &str, check: impl Fn(&'v Value) -> Option<String>) -> Option<String> {
        match v.get(key) {
            Some(Value::Array(list)) => list.iter().find_map(check),
            _ => None,
        }
    }
    const POINT: &[&str] = &["x", "y"];
    let points = |p: &Value, keys: &[&str]| keys.iter().find_map(|k| p.get(*k).and_then(|q| only(q, POINT)));
    only(v, &["parameters", "actions"])
        .or_else(|| {
            each(v, "parameters", |p| {
                let kind = p.get("kind").and_then(Value::as_str).unwrap_or("");
                let (keys, at): (&[&str], &[&str]) = match kind {
                    "linear" | "polar" => (&["kind", "name", "from", "to", "rule"], &["from", "to"]),
                    "angle" => (&["kind", "name", "center", "base", "rule"], &["center"]),
                    "point" => (&["kind", "name", "at"], &["at"]),
                    "flip" => (&["kind", "name", "a", "b"], &["a", "b"]),
                    "visibility" => (&["kind", "name", "states"], &[]),
                    "lookup" => (&["kind", "name", "rows"], &[]),
                    _ => return None,
                };
                only(p, keys)
                    .or_else(|| points(p, at))
                    .or_else(|| p.get("rule").and_then(|r| only(r, &["min", "max", "step", "list", "expression"])))
                    .or_else(|| each(p, "states", |s| only(s, &["name", "hidden"])))
                    .or_else(|| {
                        each(p, "rows", |row| {
                            only(row, &["label", "values"]).or_else(|| match row.get("values") {
                                Some(Value::Object(m)) => m.values().find_map(|x| x.is_object().then(|| only(x, POINT)).flatten()),
                                _ => None,
                            })
                        })
                    })
            })
        })
        .or_else(|| {
            each(v, "actions", |a| {
                let kind = a.get("kind").and_then(Value::as_str).unwrap_or("");
                let keys: &[&str] = match kind {
                    "stretch" => &["kind", "parameter", "pieces", "min", "max"],
                    "array" => &["kind", "parameter", "pieces", "spacing"],
                    _ => &["kind", "parameter", "pieces"],
                };
                only(a, keys).or_else(|| points(a, &["min", "max"]))
            })
        })
}
