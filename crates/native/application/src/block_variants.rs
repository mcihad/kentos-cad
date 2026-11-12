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

use kentos_commands::blocks::{name_key, name_ok};
use kentos_commands::{
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

/// The block graph made static for DXF, including unused library definitions. Every reference, including those
/// inside definitions,
/// keeps the geometry of its own parameter values. Source definitions and the drawing are never edited.
/// Returned definitions replace (rather than append to) the input list.
pub fn plain_for_export(
    blocks: &[std::sync::Arc<BlockDefinition>],
    entities: &mut [Entity],
) -> (Vec<BlockDefinition>, Vec<kentos_commands::ReportItem>) {
    let mut graph = ExportGraph {
        sources: blocks.iter().map(|b| (b.id, &**b)).collect(),
        taken: blocks.iter().map(|b| name_key(&b.name)).collect(),
        ids: blocks.iter().map(|b| b.id).collect(),
        cached: Default::default(),
        out: Vec::new(),
        variants: Vec::new(),
        counts: Vec::new(),
    };
    graph.nodes(entities);
    // Exporting a drawing must not silently purge its unused block library. Preserve every source definition at
    // its default appearance, resolving nested values through the same cache as references in the scene.
    for source in blocks {
        graph.definition(source, &BTreeMap::new());
        if source.dynamic.is_some() && !graph.counts.iter().any(|c| c.0 == source.id) {
            graph.counts.push((source.id, 0, 0));
        }
    }
    let notes = graph
        .counts
        .iter()
        .filter_map(|(id, all, varied)| {
            let source = graph.sources.get(id)?;
            let names: Vec<_> = graph.variants.iter()
                .filter(|(b, _)| b == id)
                .filter_map(|(_, id)| graph.out.iter().find(|b| b.id == *id))
                .map(|b| format!("“{}”", b.name))
                .collect();
            let drawn = if *varied == 0 {
                "varsayılan görünüşü sabit geometri olarak yazıldı".to_owned()
            } else {
                format!(
                    "{varied} yerleştirme kaydı değerleriyle çizdiği hâliyle {} bloğu olarak yazıldı",
                    names.join(", ")
                )
            };
            Some(kentos_commands::ReportItem {
                severity: Some(kentos_commands::ReportSeverity::Warning),
                details: Vec::new(),
                what: "Dinamik blok".into(),
                count: *all,
                reason: format!(
                    "“{}”: parametreleri ve eylemleri DXF'e yazılmadı; {drawn}. Sayı, çizimdeki ve yazılan tanımlardaki yerleştirme kayıtlarıdır. İç içe yerleştirmeler de kendi görünüş ve değerleriyle sabittir. Başka programlarda dinamik blok olarak açılmaz, KentOS'ta yeniden alınınca düz bloklardır.",
                    source.name
                ),
                lines: Vec::new(),
            })
        })
        .collect();
    (graph.out, notes)
}

struct ExportGraph<'a> {
    sources: std::collections::HashMap<BlockId, &'a BlockDefinition>,
    taken: std::collections::HashSet<String>,
    ids: std::collections::HashSet<BlockId>,
    cached: std::collections::HashMap<ExportKey, usize>,
    out: Vec<BlockDefinition>,
    variants: Vec<(BlockId, BlockId)>,
    counts: Vec<(BlockId, u32, u32)>,
}

#[derive(PartialEq, Eq, Hash)]
struct ExportKey {
    source: BlockId,
    values: Vec<(String, ExportValue)>,
}

#[derive(PartialEq, Eq, Hash)]
enum ExportValue {
    Flag(bool),
    Number(u64),
    Text(String),
    Point(u64, u64),
}

impl ExportKey {
    fn of(source: BlockId, values: &BTreeMap<String, ParamValue>) -> Self {
        // Values are finite in a valid document. Normalize signed zero to preserve ParamValue equality.
        let bits = |n: f64| if n == 0.0 { 0 } else { n.to_bits() };
        Self {
            source,
            values: values
                .iter()
                .map(|(name, value)| {
                    (
                        name.clone(),
                        match value {
                            ParamValue::Flag(v) => ExportValue::Flag(*v),
                            ParamValue::Number(v) => ExportValue::Number(bits(*v)),
                            ParamValue::Text(v) => ExportValue::Text(v.clone()),
                            ParamValue::Point(v) => ExportValue::Point(bits(v.x), bits(v.y)),
                        },
                    )
                })
                .collect(),
        }
    }
}

impl ExportGraph<'_> {
    fn nodes(&mut self, entities: &mut [Entity]) {
        for e in entities {
            let Entity::Insert(ins) = e else { continue };
            let Some(source) = self.sources.get(&ins.block).copied() else {
                continue;
            };
            let varied = source.dynamic.is_some() && !ins.parameters.is_empty();
            if source.dynamic.is_some() {
                if let Some(c) = self.counts.iter_mut().find(|c| c.0 == source.id) {
                    c.1 = c.1.saturating_add(1);
                    c.2 = c.2.saturating_add(u32::from(varied));
                } else {
                    self.counts.push((source.id, 1, u32::from(varied)));
                }
            }
            ins.block = self.definition(source, &ins.parameters);
            ins.parameters.clear();
        }
    }

    fn definition(
        &mut self,
        source: &BlockDefinition,
        values: &BTreeMap<String, ParamValue>,
    ) -> BlockId {
        let key = ExportKey::of(source.id, values);
        if let Some(&index) = self.cached.get(&key) {
            return self.out[index].id;
        }
        let mut plain = source.clone();
        if source.dynamic.is_some() && !values.is_empty() {
            plain.id = self.mint(source.id);
            let mut n = 1u64;
            loop {
                let name = format!("{} V{n}", source.name);
                if self.taken.insert(name_key(&name)) {
                    plain.name = name;
                    break;
                }
                n += 1;
            }
            let values = values
                .iter()
                .map(|(k, v)| format!("{k} {}", said(v)))
                .collect::<Vec<_>>();
            plain.description = Some(format!(
                "KentOS dinamik bloğu “{}”, değerleri: {}",
                source.name,
                values.join("; ")
            ));
            self.variants.push((source.id, plain.id));
        }
        let mut pieces = variant_entities(source, values);
        plain.dynamic = None;
        plain.entities.clear();
        let id = plain.id;
        let index = self.out.len();
        self.cached.insert(key, index);
        self.out.push(plain);
        // Cache before descent: shared child references reuse the same static variant.
        self.nodes(&mut pieces);
        self.out[index].entities = pieces;
        id
    }

    fn mint(&mut self, source: BlockId) -> BlockId {
        let mut n = self.cached.len() as u64 + 1;
        loop {
            let mut bytes = source.0;
            for (i, mix) in n.to_be_bytes().into_iter().enumerate() {
                bytes[8 + i] ^= mix;
            }
            bytes[14] ^= 0x5A;
            bytes[8] = (bytes[8] & 0x3f) | 0x80;
            let id = BlockId(bytes);
            if self.ids.insert(id) {
                return id;
            }
            n = n.wrapping_add(1);
        }
    }
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
        kentos_commands::CAD_BLOCKS_PROXY,
        kentos_commands::CAD_BLOCKS_PROXY_VERSION,
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
    Vec<(Slot, &'a kentos_commands::InsertEntity, String)>,
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
            xref: None,
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
        kentos_commands::CAD_BLOCKS_FREEZE,
        kentos_commands::CAD_BLOCKS_FREEZE_VERSION,
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

#[cfg(test)]
mod export_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn nested_static_export_reuses_values_across_parents_and_preserves_unused_definitions() {
        let snapshot = kentos_kcad::decode(include_bytes!(
            "../../../../fixtures/kcad/v2/dynamic-blocks.kcad"
        ))
        .unwrap();
        let source = snapshot.blocks[0].clone();
        let mut defs = vec![source.clone()];
        let placed = |block: BlockId, id: u32| -> Entity {
            serde_json::from_value(json!({"kind":"insert","id":id,"layerId":"","attrs":{},"block":block,
                "p":{"x":0.,"y":0.},"scale":1.,"rotation":0.,"parameters":{"Genişlik":1.2,"Görünüş":"Sade"}})).unwrap()
        };
        let mut roots = Vec::new();
        for n in 1..=2 {
            let mut parent = source.clone();
            parent.id =
                serde_json::from_value(json!(format!("0192f5a1-2900-7000-8000-0000000000e{n}")))
                    .unwrap();
            parent.name = format!("Modül {n}");
            parent.dynamic = None;
            parent.entities = vec![placed(source.id, 1)];
            let mut root = placed(parent.id, n);
            if let Entity::Insert(i) = &mut root {
                i.parameters.clear();
            }
            roots.push(root);
            defs.push(parent);
        }
        roots.push(placed(source.id, 3));
        let mut unused = source.clone();
        unused.id = serde_json::from_value(json!("0192f5a1-2900-7000-8000-0000000000ef")).unwrap();
        unused.name = format!("{} V1", source.name);
        for parameter in &mut unused.dynamic.as_mut().unwrap().parameters {
            if let kentos_commands::BlockParameter::Visibility { states, .. } = parameter {
                states.reverse();
            }
        }
        defs.push(unused);
        let original = defs.clone();
        let before = roots.clone();
        let blocks = defs
            .into_iter()
            .map(std::sync::Arc::new)
            .collect::<Vec<_>>();
        let (plain, notes) = plain_for_export(&blocks, &mut roots);
        assert_eq!(
            plain.len(),
            5,
            "two parents, a common child variant and both original library definitions"
        );
        let child = plain
            .iter()
            .find(|b| b.name == format!("{} V2", source.name))
            .unwrap();
        assert!(plain.iter().all(|b| b.dynamic.is_none()));
        assert_eq!(child.entities.len(), 3, "Sade does not write the swing");
        for def in plain.iter().filter(|b| b.name.starts_with("Modül")) {
            let Entity::Insert(i) = &def.entities[0] else {
                panic!("nested insert")
            };
            assert_eq!(i.block, child.id);
            assert!(i.parameters.is_empty());
        }
        let Entity::Insert(i) = &roots[2] else {
            panic!("direct insert")
        };
        assert_eq!(i.block, child.id);
        assert!(i.parameters.is_empty());
        let library = plain.iter().find(|b| b.id == original[3].id).unwrap();
        assert_eq!(library.name, original[3].name);
        assert_eq!(
            library.entities.len(), 3,
            "the unused definition also keeps its default Sade appearance"
        );
        assert_eq!(notes.len(), 2);
        assert_eq!(
            notes[1].count, 0,
            "the unused dynamic library definition is also disclosed"
        );
        assert_eq!(
            notes[0].count, 3,
            "two definition records and one direct record"
        );
        assert_eq!(
            blocks.iter().map(|b| (**b).clone()).collect::<Vec<_>>(),
            original
        );
        let mut again = before;
        assert_eq!(plain_for_export(&blocks, &mut again), (plain, notes));
        assert_eq!(again, roots);
    }
}
