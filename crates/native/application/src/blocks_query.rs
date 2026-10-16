//! `cad.blocks.query` v1 (docs/adr/0317 §3): the drawing's block inserts counted (BCOUNT), found by their attributes
//! and scheduled, nothing written. The Blok çizelgesi window, Python and the agent ask the same.
//!
//! - An insert's value of a tag is the definition's when the attribute is constant, else its own, else the
//!   definition's default; a tag its block has not reads empty.
//! - `eq`, `ne` and `contains` compare Turkish-case-folded text; `empty` and `notEmpty` the value's emptiness; `lt` and
//!   `gt` numbers (a value that is not one does not match).
//! - `count` counts the drawing's own inserts on the layers asked, by layer, and how many times other blocks' inserts
//!   place each block through their definitions (`nested`).
//!
//! The checks, in order: each block asked (`unknown_block`), each layer (`layer_not_found`), each condition
//! (`invalid_condition`), each column (`unknown_tag`).

use std::collections::{BTreeMap, HashMap, HashSet};

use kentos_contracts::blocks::name_key;
use kentos_contracts::{
    AttributeFilter, AttributeOp, BlockCount, BlockDefinition, BlockId, BlockReport, BlocksQueried,
    BlocksQuery, CommandResult, Entity, InsertEntity, ScheduleCell,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

/// The columns every schedule starts with.
pub const FIXED_COLUMNS: [&str; 5] = ["Blok", "Katman", "X", "Y", "Dönüş"];

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &BlocksQuery) -> CommandResult<()> {
    match query(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The answer, as execute gives it.
pub fn plan(cx: &ExecutionContext<'_>, input: &BlocksQuery) -> CommandResult<BlocksQueried> {
    answer(cx.doc, input)
}

/// The answer; nothing is written.
pub fn execute(cx: &mut ExecutionContext<'_>, input: BlocksQuery) -> CommandResult<BlocksQueried> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_BLOCKS_QUERY,
        kentos_contracts::CAD_BLOCKS_QUERY_VERSION,
        input,
        |cx, input| answer(cx.doc, &input),
    )
}

fn answer(doc: &Document, input: &BlocksQuery) -> CommandResult<BlocksQueried> {
    match query(doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// An insert's value of `tag`.
pub fn value_of<'a>(block: &'a BlockDefinition, insert: &'a InsertEntity, tag: &str) -> &'a str {
    let Some(a) = block.attributes.iter().find(|a| a.tag == tag) else {
        return "";
    };
    if a.options.constant {
        return a.value.as_deref().unwrap_or("");
    }
    insert
        .base
        .attrs
        .get(tag)
        .map(String::as_str)
        .or(a.value.as_deref())
        .unwrap_or("")
}

fn number(text: &str) -> Option<f64> {
    text.trim().parse::<f64>().ok().filter(|x| x.is_finite())
}

/// Whether a value meets a condition (checked: a value where one is needed, a number for `lt`, `gt`).
pub fn meets(value: &str, f: &AttributeFilter) -> bool {
    let want = f.value.as_deref().unwrap_or("");
    match f.op {
        AttributeOp::Eq => name_key(value) == name_key(want),
        AttributeOp::Ne => name_key(value) != name_key(want),
        AttributeOp::Contains => name_key(value).contains(&name_key(want)),
        AttributeOp::Empty => value.is_empty(),
        AttributeOp::NotEmpty => !value.is_empty(),
        AttributeOp::Lt => matches!((number(value), number(want)), (Some(a), Some(b)) if a < b),
        AttributeOp::Gt => matches!((number(value), number(want)), (Some(a), Some(b)) if a > b),
    }
}

fn condition_problem(f: &AttributeFilter) -> Option<&'static str> {
    if crate::checks::is_blank(&f.tag) {
        return Some("koşulun etiketi boş");
    }
    let needs = !matches!(f.op, AttributeOp::Empty | AttributeOp::NotEmpty);
    if needs && f.value.is_none() {
        return Some("bu karşılaştırma bir değer ister");
    }
    if matches!(f.op, AttributeOp::Lt | AttributeOp::Gt)
        && f.value.as_deref().and_then(number).is_none()
    {
        return Some("küçüktür ve büyüktür bir sayı ister");
    }
    None
}

fn query(doc: &Document, input: &BlocksQuery) -> Result<BlocksQueried, Stop> {
    let all = doc.blocks();
    // The blocks asked, in the drawing's order.
    let asked: Vec<&BlockDefinition> = match &input.blocks {
        Some(ids) => {
            for (i, id) in ids.iter().enumerate() {
                if doc.block(*id).is_none() {
                    return Err(Stop::failed(error(
                        codes::UNKNOWN_BLOCK,
                        format!(
                            "“{}” kimlikli blok çizimde tanımlı değil. Çizimde tanımlı bir bloğun kimliğini verin.",
                            id.to_text()
                        ),
                        Some(format!("blocks[{i}]")),
                    )));
                }
            }
            all.iter()
                .filter(|b| ids.contains(&b.id))
                .map(|b| &**b)
                .collect()
        }
        None => all.iter().map(|b| &**b).collect(),
    };
    if let Some(layers) = &input.layers {
        for (i, id) in layers.iter().enumerate() {
            if doc.layers().get(id).is_none() {
                return Err(Stop::failed(error(
                    codes::LAYER_NOT_FOUND,
                    format!(
                        "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                    ),
                    Some(format!("layers[{i}]")),
                )));
            }
        }
    }
    let filters = input.filters.clone().unwrap_or_default();
    for (i, f) in filters.iter().enumerate() {
        if let Some(problem) = condition_problem(f) {
            return Err(Stop::failed(error(
                codes::INVALID_CONDITION,
                format!("{}. koşul okunamadı: {problem}.", i + 1),
                Some(format!("where[{i}]")),
            )));
        }
    }
    let on_layer = |e: &Entity| {
        input
            .layers
            .as_ref()
            .is_none_or(|l| l.contains(&e.base().layer_id))
    };
    let ids: HashSet<BlockId> = asked.iter().map(|b| b.id).collect();
    let mut out = BlocksQueried {
        counts: Vec::new(),
        uids: Vec::new(),
        columns: Vec::new(),
        rows: Vec::new(),
    };
    if input.report == BlockReport::Count {
        out.counts = counts(doc, &asked, &on_layer);
        return Ok(out);
    }
    // The inserts that match, in the drawing's order.
    let mut matched: Vec<(Slot, &InsertEntity, &BlockDefinition)> = Vec::new();
    for e in doc.entities() {
        let Entity::Insert(i) = e else { continue };
        if !ids.contains(&i.block) || !on_layer(e) {
            continue;
        }
        let Some(block) = doc.block(i.block) else {
            continue;
        };
        if filters.iter().all(|f| meets(value_of(block, i, &f.tag), f)) {
            matched.push((Slot(i.base.id), i, block));
        }
    }
    out.uids = matched
        .iter()
        .filter_map(|(slot, _, _)| doc.uid(*slot).map(|u| u.to_string()))
        .collect();
    if input.report == BlockReport::Instances {
        return Ok(out);
    }
    // The schedule's columns: the tags asked, or each of the blocks' once in their order.
    let tags: Vec<String> = match &input.columns {
        Some(list) => {
            for (i, tag) in list.iter().enumerate() {
                if !asked
                    .iter()
                    .any(|b| b.attributes.iter().any(|a| &a.tag == tag))
                {
                    return Err(Stop::failed(error(
                        codes::UNKNOWN_TAG,
                        format!("“{tag}” etiketli öznitelik istenen bloklarda yok."),
                        Some(format!("columns[{i}]")),
                    )));
                }
            }
            list.clone()
        }
        None => {
            let mut seen = HashSet::new();
            asked
                .iter()
                .flat_map(|b| b.attributes.iter().map(|a| a.tag.clone()))
                .filter(|t| seen.insert(t.clone()))
                .collect()
        }
    };
    out.columns = FIXED_COLUMNS
        .iter()
        .map(|c| (*c).to_owned())
        .chain(tags.iter().cloned())
        .collect();
    out.rows = matched
        .iter()
        .map(|(_, i, block)| {
            let layer = doc
                .layers()
                .get(&i.base.layer_id)
                .map_or_else(|| i.base.layer_id.clone(), |n| n.name.clone());
            let mut row = vec![
                ScheduleCell::Text(block.name.clone()),
                ScheduleCell::Text(layer),
                ScheduleCell::Number(i.p.x),
                ScheduleCell::Number(i.p.y),
                ScheduleCell::Number(i.rotation * 180.0 / std::f64::consts::PI),
            ];
            row.extend(
                tags.iter()
                    .map(|t| ScheduleCell::Text(value_of(block, i, t).to_owned())),
            );
            row
        })
        .collect();
    Ok(out)
}

/// Each block asked counted: the drawing's own inserts on the layers asked, by layer, and how many times the
/// drawing's inserts (on those layers) place it through their definitions.
fn counts(
    doc: &Document,
    asked: &[&BlockDefinition],
    on_layer: &dyn Fn(&Entity) -> bool,
) -> Vec<BlockCount> {
    // Inside each definition, how many times it places each block, its nested inserts counted through.
    let mut inside: HashMap<BlockId, HashMap<BlockId, u64>> = HashMap::new();
    fn within(
        doc: &Document,
        id: BlockId,
        memo: &mut HashMap<BlockId, HashMap<BlockId, u64>>,
        depth: usize,
    ) -> HashMap<BlockId, u64> {
        if let Some(m) = memo.get(&id) {
            return m.clone();
        }
        let mut m: HashMap<BlockId, u64> = HashMap::new();
        if let Some(b) = doc.block(id)
            && depth < kentos_contracts::MAX_BLOCK_DEPTH
        {
            for e in &b.entities {
                if let Entity::Insert(i) = e {
                    *m.entry(i.block).or_default() += 1;
                    for (k, n) in within(doc, i.block, memo, depth + 1) {
                        let v = m.entry(k).or_default();
                        *v = v.saturating_add(n);
                    }
                }
            }
        }
        memo.insert(id, m.clone());
        m
    }
    let mut own: HashMap<BlockId, (u32, BTreeMap<String, u32>)> = HashMap::new();
    let mut nested: HashMap<BlockId, u64> = HashMap::new();
    for e in doc.entities() {
        let Entity::Insert(i) = e else { continue };
        if !on_layer(e) {
            continue;
        }
        let entry = own.entry(i.block).or_default();
        entry.0 += 1;
        *entry.1.entry(i.base.layer_id.clone()).or_default() += 1;
        for (k, n) in within(doc, i.block, &mut inside, 1) {
            let v = nested.entry(k).or_default();
            *v = v.saturating_add(n);
        }
    }
    asked
        .iter()
        .map(|b| {
            let (count, layers) = own.remove(&b.id).unwrap_or_default();
            BlockCount {
                block: b.id,
                name: b.name.clone(),
                count,
                layers,
                nested: nested.get(&b.id).copied().unwrap_or(0),
            }
        })
        .collect()
}
