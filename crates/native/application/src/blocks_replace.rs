//! `cad.blocks.replace` v1 (docs/adr/0317 §3): BLOCKREPLACE. The drawing's inserts of one block place another, as
//! one undo step “Bloğu başkasıyla değiştir”: each keeps its place, scale, turn, mirror, layer, look and other data;
//! of its attribute values only those of the new block's attributes that are not constant stay, of its places only
//! those of the new block's movable attributes; the new block's fields are filled. With `purge` the old block is
//! deleted when nothing places it any more. Only the drawing's own inserts are replaced; a definition that places the
//! old block keeps placing it.
//!
//! The checks, in order: both blocks given (`no_block`), not the same (`same_block`); the expected revision; both the
//! drawing's (`unknown_block`); the inserts named (`entity_not_found`, `not_insert`: not an insert of `from`); those on
//! a locked layer left out (every one: `layer_locked`); the new block's fields (`invalid_field`).

use std::collections::HashSet;

use kentos_contracts::{
    BlockId, BlocksReplace, BlocksReplaced, CommandResult, CommandWarning, Entity,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const STEP: &str = "Bloğu başkasıyla değiştir";

struct Checked {
    changes: Vec<(Slot, Entity)>,
    output: BlocksReplaced,
    purge: Option<BlockId>,
    warnings: Vec<CommandWarning>,
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &BlocksReplace) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would replace, writing nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &BlocksReplace) -> CommandResult<BlocksReplaced> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: c.output,
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Replaces the inserts as one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: BlocksReplace,
) -> CommandResult<BlocksReplaced> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_BLOCKS_REPLACE,
        kentos_contracts::CAD_BLOCKS_REPLACE_VERSION,
        input,
        |cx, input| write(cx.doc, &input),
    )
}

fn write(doc: &mut Document, input: &BlocksReplace) -> CommandResult<BlocksReplaced> {
    let c = match check(doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut output = c.output;
    if !c.changes.is_empty() {
        doc.update_many(c.changes, STEP);
    }
    // The old block deleted when nothing places it any more (the inserts written first, in the same step).
    if let Some(id) = c.purge
        && doc.block_removal_refused(id).is_none()
        && doc.remove_block(id).is_ok_and(|removed| removed)
    {
        output.purged = true;
    }
    output.revision = doc.revision().to_string();
    CommandResult::Completed {
        output,
        warnings: c.warnings,
    }
}

fn unknown(id: BlockId, path: &str) -> Stop {
    Stop::failed(error(
        codes::UNKNOWN_BLOCK,
        format!(
            "“{}” kimlikli blok çizimde tanımlı değil. Çizimde tanımlı bir bloğun kimliğini verin.",
            id.to_text()
        ),
        Some(path.into()),
    ))
}

fn check(doc: &Document, input: &BlocksReplace) -> Result<Checked, Stop> {
    let (Some(from), Some(to)) = (input.from, input.to) else {
        let path = if input.from.is_none() { "from" } else { "to" };
        return Err(Stop::failed(error(
            codes::NO_BLOCK,
            "Değişecek blok ya da yerine geçecek blok verilmedi. from ve to ile iki bloğun kimliğini verin.".into(),
            Some(path.into()),
        )));
    };
    if from == to {
        return Err(Stop::failed(error(
            codes::SAME_BLOCK,
            "Yerleştirmeler aynı bloğa geçirilemez; to ile başka bir blok verin.".into(),
            Some("to".into()),
        )));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let old = doc.block(from).ok_or_else(|| unknown(from, "from"))?;
    let new = doc.block(to).ok_or_else(|| unknown(to, "to"))?;
    // The inserts: those named, or every one of the drawing's.
    let every: Vec<String>;
    let uids: &[String] = match &input.uids {
        Some(list) => {
            checks::uids(list, "Değişecek yerleştirme verilmedi.")?;
            list
        }
        None => {
            every = doc
                .entities()
                .filter(|e| matches!(e, Entity::Insert(i) if i.block == from))
                .filter_map(|e| doc.uid(Slot(e.base().id)).map(|u| u.to_string()))
                .collect();
            &every
        }
    };
    let named = checks::named(doc, uids)?;
    for (i, _, e, _) in &named {
        if !matches!(e, Entity::Insert(ins) if ins.block == from) {
            return Err(Stop::failed(error(
                codes::NOT_INSERT,
                format!(
                    "Verilen nesne “{}” bloğunun bir yerleştirmesi değil. Bu bloğun yerleştirmelerini verin.",
                    old.name
                ),
                Some(format!("uids[{i}]")),
            )));
        }
    }
    let (open, warning) = if named.is_empty() {
        (named, None)
    } else {
        checks::skip_locked(doc, named)?
    };
    let fields = crate::blocks_sync::fields_of(doc, &new.attributes, &|_| "to".to_owned())?;
    let kept: HashSet<&str> = new
        .attributes
        .iter()
        .filter(|a| !a.options.constant)
        .map(|a| a.tag.as_str())
        .collect();
    let movable: HashSet<&str> = new
        .attributes
        .iter()
        .filter(|a| a.options.movable)
        .map(|a| a.tag.as_str())
        .collect();
    let mut dropped: HashSet<String> = HashSet::new();
    let mut places = 0u32;
    let mut replaced: Vec<(Slot, kentos_contracts::InsertEntity)> = Vec::with_capacity(open.len());
    for (_, slot, e, _) in &open {
        let Entity::Insert(i) = e else { continue };
        let mut i = i.clone();
        i.block = to;
        for a in &old.attributes {
            if !kept.contains(a.tag.as_str()) && i.base.attrs.remove(&a.tag).is_some() {
                dropped.insert(a.tag.clone());
            }
        }
        let before = i.attribute_places.len();
        i.attribute_places
            .retain(|tag, _| movable.contains(tag.as_str()));
        places += u32::try_from(before - i.attribute_places.len()).unwrap_or(u32::MAX);
        replaced.push((*slot, i));
    }
    // The new block's fields, with the values the inserts keep.
    let mut filled = 0u32;
    if !fields.is_empty() && !replaced.is_empty() {
        let entities: Vec<Entity> = replaced
            .iter()
            .map(|(_, i)| Entity::Insert(i.clone()))
            .collect();
        let shapes: Vec<_> = entities.iter().map(crate::geometry::shape).collect();
        let list: Vec<&Entity> = entities.iter().collect();
        let layer_name = |id: &str| {
            doc.layers()
                .get(id)
                .map_or_else(|| id.to_owned(), |n| n.name.clone())
        };
        for (tag, template) in &fields {
            let texts = template.texts(&list, &layer_name, &|k| shapes.get(k), &|_| None);
            for ((_, i), text) in replaced.iter_mut().zip(texts) {
                i.base.attrs.insert(tag.clone(), text);
                filled += 1;
            }
        }
    }
    let uids_out: Vec<String> = replaced
        .iter()
        .filter_map(|(slot, _)| doc.uid(*slot).map(|u| u.to_string()))
        .collect();
    let output = BlocksReplaced {
        replaced: uids_out,
        dropped: old
            .attributes
            .iter()
            .filter(|a| dropped.contains(&a.tag))
            .map(|a| a.tag.clone())
            .collect(),
        places,
        fields: filled,
        purged: false,
        revision: doc.revision().to_string(),
    };
    Ok(Checked {
        changes: replaced
            .into_iter()
            .map(|(slot, i)| (slot, Entity::Insert(i)))
            .collect(),
        output,
        purge: (input.purge == Some(true)).then_some(from),
        warnings: warning.into_iter().collect(),
    })
}
