//! What a block's insert draws, as plain objects, and a dynamic block's inserts made plain (docs/adr/0319 §4):
//! `cad.blocks.proxy` v1 (nothing written) and `cad.blocks.freeze` v1 (one undo step “Sabit bloğa çevir”).
//!
//! A dynamic block's insert draws its definition's objects as its values vary them (`Dynamic::vary_traced`): each
//! object keeps every field of the one it came from (an array's copy, of the object copied) and only its geometry is
//! the variant's; its local id is the variant's, a copy's a new one past the definition's. A plain block's insert
//! draws its definition. Sabit bloğa çevir makes one plain block of each distinct block and values given, holding
//! those objects with the source's base point, unit, attribute definitions, description and card, and points the
//! inserts at it, their values dropped.
//!
//! The checks, in order: the ids (`no_entities`, `invalid_uid`); a name given (`empty_name`); the expected revision;
//! the inserts (`entity_not_found`, `not_insert`; Sabit bloğa çevir's `not_dynamic`); the names the new blocks take
//! (`duplicate_block`); the inserts on a locked layer left out (every one: `layer_locked`).

use std::collections::BTreeMap;

use kentos_contracts::blocks::{name_key, name_ok};
use kentos_contracts::{
    BlockDefinition, BlockId, BlocksFreeze, BlocksFrozen, BlocksProxied, BlocksProxy,
    CommandResult, CommandWarning, Entity, FrozenBlock, InsertProxy, ParamValue,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::api::json::Json;

use crate::ExecutionContext;
use crate::blocks::core_entity;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::{edit_geometry, entity_of, shape, with_shape};

/// The undo step's name.
pub const STEP: &str = "Sabit bloğa çevir";

/// `block`'s objects as an insert keeping `values` draws them (docs/adr/0319 §4): the definition's for a plain block
/// or the defaults.
pub fn variant_entities(
    block: &BlockDefinition,
    values: &BTreeMap<String, ParamValue>,
) -> Vec<Entity> {
    let Some(d) = block
        .dynamic
        .as_ref()
        .and_then(kentos_formats::blocks::core_dynamic)
    else {
        return block.entities.clone();
    };
    let params = kentos_formats::blocks::core_params(values);
    if d.is_default(params.as_ref()) {
        return block.entities.clone();
    }
    let core: Vec<_> = block.entities.iter().map(core_entity).collect();
    d.vary_traced(&core, params.as_ref())
        .into_iter()
        .filter_map(|(e, source)| {
            let original = block.entities.get(source)?;
            let id = e
                .rest
                .iter()
                .find_map(|(k, v)| match (k.as_str(), v) {
                    ("id", Json::Num(n)) if *n >= 0.0 => Some(*n as u32),
                    _ => None,
                })
                .unwrap_or(original.base().id);
            let mut out = if e.shape == shape(original) {
                original.clone()
            } else {
                with_shape(original, e.shape.clone()).or_else(|| {
                    edit_geometry(e.shape.clone()).map(|g| entity_of(&g, original.base().clone()))
                })?
            };
            crate::blocks_define::base_mut(&mut out).id = id;
            Some(out)
        })
        .collect()
}

/// A value as a report says it.
fn said(v: &ParamValue) -> String {
    match v {
        ParamValue::Flag(b) => if *b { "evet" } else { "hayır" }.to_owned(),
        ParamValue::Number(x) => format!("{}", (x * 1e6).round() / 1e6),
        ParamValue::Text(t) => t.clone(),
        ParamValue::Point(p) => format!(
            "({}, {})",
            (p.x * 1e6).round() / 1e6,
            (p.y * 1e6).round() / 1e6
        ),
    }
}

/// A drawing's objects made fit for a reader that does not know dynamic blocks (docs/adr/0319 §5, DXF): each insert
/// of a dynamic block keeping values points at a plain block of what it draws, one a block and values, named the
/// block's name and “V1”, “V2” … past the names taken, its description the values; the blocks to add and what to
/// say in the report. An insert at its defaults places its block as it is; a dynamic part is never written.
pub fn plain_for_export(
    blocks: &[std::sync::Arc<BlockDefinition>],
    entities: &mut [Entity],
) -> (Vec<BlockDefinition>, Vec<kentos_contracts::ReportItem>) {
    let mut taken: Vec<String> = blocks.iter().map(|b| name_key(&b.name)).collect();
    let mut ids: Vec<BlockId> = blocks.iter().map(|b| b.id).collect();
    let mut made: Vec<(BlockId, BTreeMap<String, ParamValue>, BlockDefinition)> = Vec::new();
    // Each dynamic block's inserts: how many, how many varied.
    let mut counts: Vec<(BlockId, u32, u32)> = Vec::new();
    for e in entities.iter_mut() {
        let Entity::Insert(ins) = e else { continue };
        let Some(source) = blocks.iter().find(|b| b.id == ins.block) else {
            continue;
        };
        if source.dynamic.is_none() {
            continue;
        }
        match counts.iter_mut().find(|c| c.0 == source.id) {
            Some(c) => c.1 += 1,
            None => counts.push((source.id, 1, 0)),
        }
        if ins.parameters.is_empty() {
            continue;
        }
        if let Some(c) = counts.iter_mut().find(|c| c.0 == source.id) {
            c.2 += 1;
        }
        let id = match made
            .iter()
            .find(|(b, v, _)| *b == source.id && *v == ins.parameters)
        {
            Some((_, _, def)) => def.id,
            None => {
                let mut n = 1;
                let name = loop {
                    let name = format!("{} V{n}", source.name);
                    if !taken.contains(&name_key(&name)) {
                        break name;
                    }
                    n += 1;
                };
                taken.push(name_key(&name));
                // An id of its own, made from its source's: the same drawing writes the same file.
                let mut bytes = source.id.0;
                let mut k: u16 = u16::try_from(made.len() + 1).unwrap_or(u16::MAX);
                let id = loop {
                    bytes[14] = source.id.0[14] ^ (k >> 8) as u8 ^ 0x5A;
                    bytes[15] = source.id.0[15] ^ (k & 0xFF) as u8;
                    let id = BlockId(bytes);
                    if !ids.contains(&id) {
                        break id;
                    }
                    k = k.wrapping_add(1);
                };
                ids.push(id);
                let values: Vec<String> = ins
                    .parameters
                    .iter()
                    .map(|(k, v)| format!("{k} {}", said(v)))
                    .collect();
                let def = BlockDefinition {
                    id,
                    name,
                    base: source.base,
                    entities: variant_entities(source, &ins.parameters),
                    units: source.units,
                    attributes: source.attributes.clone(),
                    description: Some(format!(
                        "KentOS dinamik bloğu “{}”, değerleri: {}",
                        source.name,
                        values.join("; ")
                    )),
                    card: source.card.clone(),
                    dynamic: None,
                };
                made.push((source.id, ins.parameters.clone(), def));
                id
            }
        };
        ins.block = id;
        ins.parameters = BTreeMap::new();
    }
    let notes = counts
        .into_iter()
        .filter_map(|(id, all, varied)| {
            let source = blocks.iter().find(|b| b.id == id)?;
            let names: Vec<&str> = made
                .iter()
                .filter(|(b, ..)| *b == id)
                .map(|(.., d)| d.name.as_str())
                .collect();
            let drawn = if varied == 0 {
                "hepsi varsayılan değerlerinde, blok olduğu gibi yazıldı".to_owned()
            } else {
                format!(
                    "{varied} yerleştirme değerleriyle çizdiği hâliyle {} bloğu olarak yazıldı",
                    names.iter().map(|n| format!("“{n}”")).collect::<Vec<_>>().join(", ")
                )
            };
            Some(kentos_contracts::ReportItem {
                what: "Dinamik blok".into(),
                count: all,
                reason: format!(
                    "“{}”: parametreleri ve eylemleri DXF'e yazılmadı; {drawn}. AutoCAD'de dinamik blok olarak açılmaz, KentOS'ta yeniden alınınca düz bloklardır.",
                    source.name
                ),
                lines: Vec::new(),
            })
        })
        .collect();
    (made.into_iter().map(|(.., d)| d).collect(), notes)
}

/// The inserts the input names, each with where it names it.
fn inserts<'a>(
    doc: &'a Document,
    uids: &'a [String],
    empty: &str,
) -> Result<Vec<checks::Named<'a>>, Stop> {
    checks::uids(uids, empty)?;
    let named = checks::named(doc, uids)?;
    for (i, _, e, _) in &named {
        if !matches!(e, Entity::Insert(_)) {
            return Err(Stop::failed(error(
                codes::NOT_INSERT,
                "Verilen nesne bir blok yerleştirmesi değil. Bir bloğun yerleştirmesini verin."
                    .into(),
                Some(format!("uids[{i}]")),
            )));
        }
    }
    Ok(named)
}

fn proxied(doc: &Document, input: &BlocksProxy) -> Result<BlocksProxied, Stop> {
    let named = inserts(
        doc,
        &input.uids,
        "Çizdiği nesneler sorulacak yerleştirme verilmedi.",
    )?;
    let mut out = BlocksProxied {
        inserts: Vec::with_capacity(named.len()),
    };
    for (_, _, e, uid) in named {
        let Entity::Insert(ins) = e else { continue };
        let Some(block) = doc.block(ins.block) else {
            continue;
        };
        let values = block
            .dynamic
            .as_ref()
            .map(|d| crate::block_params::current(d, &ins.parameters))
            .unwrap_or_default();
        out.inserts.push(InsertProxy {
            uid: uid.clone(),
            block: ins.block,
            default: ins.parameters.is_empty(),
            values,
            entities: variant_entities(block, &ins.parameters),
        });
    }
    Ok(out)
}

/// Reads what the inserts draw; nothing is written.
pub fn proxy_execute(
    cx: &mut ExecutionContext<'_>,
    input: BlocksProxy,
) -> CommandResult<BlocksProxied> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_BLOCKS_PROXY,
        kentos_contracts::CAD_BLOCKS_PROXY_VERSION,
        input,
        |cx, input| match proxied(cx.doc, &input) {
            Ok(output) => CommandResult::Completed {
                output,
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        },
    )
}

pub fn proxy_validate(cx: &ExecutionContext<'_>, input: &BlocksProxy) -> CommandResult<()> {
    match proxied(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn proxy_plan(cx: &ExecutionContext<'_>, input: &BlocksProxy) -> CommandResult<BlocksProxied> {
    match proxied(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// A block, the values its inserts keep, and those inserts (slot, insert, persistent id).
type Group<'a> = (
    BlockId,
    BTreeMap<String, ParamValue>,
    Vec<(Slot, &'a kentos_contracts::InsertEntity, String)>,
);

struct Frozen {
    out: BlocksFrozen,
    blocks: Vec<BlockDefinition>,
    changes: Vec<(Slot, Entity)>,
    warnings: Vec<CommandWarning>,
}

/// The new blocks' names (docs/adr/0319 §4): `name`, then “name 2”, “name 3” …, each refused when taken; without a
/// name each its source's with “(sabit)”, “(sabit 2)” … past the names taken.
fn names(
    doc: &Document,
    given: Option<&str>,
    sources: &[&BlockDefinition],
) -> Result<Vec<String>, Stop> {
    let mut taken: Vec<String> = doc.blocks().iter().map(|b| name_key(&b.name)).collect();
    let mut out = Vec::with_capacity(sources.len());
    for (k, source) in sources.iter().enumerate() {
        let name = match given {
            Some(name) => {
                let name = if k == 0 {
                    name.to_owned()
                } else {
                    format!("{name} {}", k + 1)
                };
                if taken.contains(&name_key(&name)) {
                    return Err(Stop::failed(error(
                        codes::DUPLICATE_BLOCK,
                        format!("Çizimde “{name}” adında bir blok var; başka bir ad verin."),
                        Some("name".into()),
                    )));
                }
                name
            }
            None => {
                let mut n = 1;
                loop {
                    let name = if n == 1 {
                        format!("{} (sabit)", source.name)
                    } else {
                        format!("{} (sabit {n})", source.name)
                    };
                    if !taken.contains(&name_key(&name)) {
                        break name;
                    }
                    n += 1;
                }
            }
        };
        taken.push(name_key(&name));
        out.push(name);
    }
    Ok(out)
}

fn frozen(
    doc: &Document,
    input: &BlocksFreeze,
    mut mint: impl FnMut() -> BlockId,
) -> Result<Frozen, Stop> {
    checks::uids(&input.uids, "Sabit bloğa çevrilecek yerleştirme verilmedi.")?;
    if let Some(name) = &input.name
        && !name_ok(name)
    {
        return Err(Stop::failed(error(
            codes::EMPTY_NAME,
            "Blok adı boş olamaz; bir ad yazın ya da adı vermeyin (kaynağın adı ve “(sabit)”)."
                .into(),
            Some("name".into()),
        )));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let named = inserts(
        doc,
        &input.uids,
        "Sabit bloğa çevrilecek yerleştirme verilmedi.",
    )?;
    for (i, _, e, _) in &named {
        let Entity::Insert(ins) = e else { continue };
        if doc.block(ins.block).is_none_or(|b| b.dynamic.is_none()) {
            let name = doc
                .block(ins.block)
                .map_or("", |b| b.name.as_str())
                .to_owned();
            return Err(Stop::failed(error(
                codes::NOT_DYNAMIC,
                format!(
                    "“{name}” bloğu dinamik değil; zaten sabit. Dinamik bir bloğun yerleştirmesini verin."
                ),
                Some(format!("uids[{i}]")),
            )));
        }
    }
    let (open, warning) = checks::skip_locked(doc, named)?;
    // One new block for each distinct block and values, in the order first given.
    let mut groups: Vec<Group<'_>> = Vec::new();
    for (_, slot, e, uid) in &open {
        let Entity::Insert(ins) = e else { continue };
        match groups
            .iter_mut()
            .find(|(b, v, _)| *b == ins.block && *v == ins.parameters)
        {
            Some(g) => g.2.push((*slot, ins, (*uid).clone())),
            None => groups.push((
                ins.block,
                ins.parameters.clone(),
                vec![(*slot, ins, (*uid).clone())],
            )),
        }
    }
    let sources: Vec<&BlockDefinition> =
        groups.iter().filter_map(|(b, ..)| doc.block(*b)).collect();
    let names = names(doc, input.name.as_deref(), &sources)?;
    let mut out = BlocksFrozen {
        blocks: Vec::with_capacity(groups.len()),
        revision: doc.revision().to_string(),
    };
    let mut blocks = Vec::with_capacity(groups.len());
    let mut changes = Vec::new();
    for ((from, values, members), (source, name)) in
        groups.into_iter().zip(sources.iter().zip(names))
    {
        let id = mint();
        blocks.push(BlockDefinition {
            id,
            name: name.clone(),
            base: source.base,
            entities: variant_entities(source, &values),
            units: source.units,
            attributes: source.attributes.clone(),
            description: source.description.clone(),
            card: source.card.clone(),
            dynamic: None,
        });
        let shown = source
            .dynamic
            .as_ref()
            .map(|d| crate::block_params::current(d, &values))
            .unwrap_or_default();
        let mut uids = Vec::with_capacity(members.len());
        for (slot, ins, uid) in members {
            let mut i = ins.clone();
            i.block = id;
            i.parameters = BTreeMap::new();
            changes.push((slot, Entity::Insert(i)));
            uids.push(uid);
        }
        out.blocks.push(FrozenBlock {
            block: id,
            name,
            from,
            values: shown,
            inserts: uids,
        });
    }
    Ok(Frozen {
        out,
        blocks,
        changes,
        warnings: warning.into_iter().collect(),
    })
}

/// A plan's new blocks have the nil id: their ids are made when they are written.
fn nil() -> BlockId {
    BlockId([0; 16])
}

pub fn freeze_validate(cx: &ExecutionContext<'_>, input: &BlocksFreeze) -> CommandResult<()> {
    match frozen(cx.doc, input, nil) {
        Ok(f) => CommandResult::Completed {
            output: (),
            warnings: f.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn freeze_plan(cx: &ExecutionContext<'_>, input: &BlocksFreeze) -> CommandResult<BlocksFrozen> {
    match frozen(cx.doc, input, nil) {
        Ok(f) => CommandResult::Completed {
            output: f.out,
            warnings: f.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Makes the plain blocks and points the inserts at them, one undo step.
pub fn freeze_execute(
    cx: &mut ExecutionContext<'_>,
    input: BlocksFreeze,
) -> CommandResult<BlocksFrozen> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_BLOCKS_FREEZE,
        kentos_contracts::CAD_BLOCKS_FREEZE_VERSION,
        input,
        |cx, input| {
            let f = match frozen(cx.doc, &input, || BlockId(*Uuid::now_v7().as_bytes())) {
                Ok(f) => f,
                Err(stop) => return stop.into(),
            };
            let Frozen {
                mut out,
                blocks,
                changes,
                warnings,
            } = f;
            let written = cx.doc.transact(STEP, |doc| {
                for b in blocks {
                    doc.add_block(b).map_err(|r| r.to_string())?;
                }
                doc.update_many(changes, STEP);
                Ok::<(), String>(())
            });
            if let Err(message) = written {
                return Stop::failed(error(codes::BLOCK_REFUSED, message, None)).into();
            }
            out.revision = cx.doc.revision().to_string();
            CommandResult::Completed {
                output: out,
                warnings,
            }
        },
    )
}
