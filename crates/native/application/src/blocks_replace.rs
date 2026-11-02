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

use std::collections::{BTreeMap, HashSet};

use kentos_commands::{
    BlockId, BlocksReplace, BlocksReplaced, CommandResult, CommandWarning, Entity,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

mod upgrade;

/// The undo step's name.
pub const STEP: &str = "Bloğu başkasıyla değiştir";

struct Checked {
    added: Vec<kentos_commands::BlockDefinition>,
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
        kentos_commands::CAD_BLOCKS_REPLACE,
        kentos_commands::CAD_BLOCKS_REPLACE_VERSION,
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
    if !c.added.is_empty()
        && let Err(why) = doc.merge_blocks(&c.added)
    {
        return Stop::failed(error(codes::INVALID_LIBRARY, why.to_string(), Some("library".into()))).into();
    }
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
    if let Some(library) = &input.library {
        return upgrade::check(doc, input, library);
    }
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
    let mut replaced: Vec<(Slot, kentos_commands::InsertEntity)> = Vec::with_capacity(open.len());
    for (index, slot, e, _) in &open {
        let Entity::Insert(i) = e else { continue };
        let mut i = i.clone();
        i.parameters = parameters(old, new, &i.parameters).map_err(|(name, why)| {
            Stop::failed(error(
                codes::INVALID_PARAMETER_VALUE,
                format!(
                    "“{}” bloğunun örnek değerleri “{}” bloğuna korunarak taşınamıyor: {why} Önce uyumsuz değeri açıkça sıfırlayın veya uyumlu bir hedef seçin. Hiçbir yerleştirme değiştirilmedi.",
                    old.name, new.name
                ),
                Some(if name.is_empty() { format!("uids[{index}].parameters") } else { format!("uids[{index}].parameters.{name}") }),
            ))
        })?;
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
        target: None,
        added: Vec::new(),
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
        added: Vec::new(),
        changes: replaced
            .into_iter()
            .map(|(slot, i)| (slot, Entity::Insert(i)))
            .collect(),
        output,
        purge: (input.purge == Some(true)).then_some(from),
        warnings: warning.into_iter().collect(),
    })
}

/// Instance inputs survive a compatible replacement; calculated values are recalculated by the target.
/// Unlike a definition edit's explicit refit, replacement must not silently discard incompatible user inputs.
fn parameters(
    old: &kentos_commands::BlockDefinition,
    new: &kentos_commands::BlockDefinition,
    kept: &BTreeMap<String, kentos_commands::ParamValue>,
) -> Result<BTreeMap<String, kentos_commands::ParamValue>, (String, String)> {
    let mut inputs = BTreeMap::new();
    for (name, value) in kept {
        let source = old
            .dynamic
            .as_ref()
            .and_then(|d| d.parameters.iter().find(|p| p.name() == name));
        if source.is_some_and(|p| p.rule().is_some_and(|r| r.expression.is_some())) {
            continue;
        }
        let target = new
            .dynamic
            .as_ref()
            .and_then(|d| d.parameters.iter().find(|p| p.name() == name));
        let Some(target) = target else {
            return Err((name.clone(), format!("Hedefte “{name}” parametresi yok.")));
        };
        if source.is_none_or(|p| std::mem::discriminant(p) != std::mem::discriminant(target)) {
            return Err((
                name.clone(),
                format!("“{name}” parametresinin türü aynı değil."),
            ));
        }
        if matches!(
            source,
            Some(
                kentos_commands::BlockParameter::Linear { .. }
                    | kentos_commands::BlockParameter::Polar { .. }
                    | kentos_commands::BlockParameter::Point { .. }
            )
        ) && (old.units == Some(kentos_commands::BlockUnits::Unitless))
            != (new.units == Some(kentos_commands::BlockUnits::Unitless))
        {
            return Err((
                name.clone(),
                format!("“{name}” için birimsiz ve fiziksel tanım sayıları doğrudan eşlenemez."),
            ));
        }
        inputs.insert(name.clone(), value.clone());
    }
    match &new.dynamic {
        Some(d) => crate::block_params::resolve(d, &BTreeMap::new(), &inputs)
            .map_err(|(_, path, why)| (path.strip_prefix("values.").unwrap_or("").to_owned(), why)),
        None => Ok(BTreeMap::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::{BlocksParameters, ParametersOperation};
    use serde_json::json;

    #[test]
    fn a_selected_version_keeps_overrides_and_nested_dimension_reference_and_roundtrips_undo() {
        let mut snapshot = kentos_kcad::decode(include_bytes!(
            "../../../../fixtures/kcad/v2/dynamic-blocks.kcad"
        ))
        .unwrap();
        let from = snapshot.blocks[0].id;
        let mut target = snapshot.blocks[0].clone();
        target.id = serde_json::from_value(json!("0192f5a1-2900-7000-8000-0000000000e1")).unwrap();
        target.name = "Yeni kapı".into();
        target.card.as_mut().unwrap().version = 2;
        target.entities.push(
            serde_json::from_value(json!({
                "kind":"line","id":5,"layerId":"","attrs":{},
                "a":{"x":0.06,"y":0.45},"b":{"x":0.3,"y":0.45}
            }))
            .unwrap(),
        );
        let to = target.id;
        snapshot.blocks.push(target);
        let uid = snapshot.uids[0];
        let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
        let set = crate::block_params::execute(
            &mut ExecutionContext::new(&mut doc),
            BlocksParameters {
                operation: ParametersOperation::Set,
                uids: vec![uid.to_text()],
                values: Some(
                    serde_json::from_value(json!({"Genişlik":1.05,"Görünüş":"Sade"})).unwrap(),
                ),
                names: None,
                expected_revision: None,
            },
        );
        assert!(matches!(set, CommandResult::Completed { .. }));
        let dimension = doc
            .add(
                serde_json::from_value(json!({
                    "kind":"dimension","id":0,"layerId":"duvar","attrs":{},
                    "a":{"x":0.,"y":0.},"b":{"x":0.,"y":1.05},"offset":0.2,"height":0.1,
            "refs":{"a":{"uid":uid,"path":[0],"place":{"kind":"start"}},
                    "b":{"uid":uid,"path":[0],"place":{"kind":"end"}}}
                }))
                .unwrap(),
            )
            .unwrap();
        // Reopen a clean baseline, exactly as a saved office drawing is opened.
        let before = kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap();
        let mut doc = Document::from_snapshot_v2(kentos_kcad::decode(&before).unwrap()).unwrap();
        let input = BlocksReplace {
            library: None,
            from: Some(from),
            to: Some(to),
            uids: Some(vec![uid.to_text()]),
            purge: None,
            expected_revision: None,
        };
        let answer = execute(&mut ExecutionContext::new(&mut doc), input);
        assert!(
            matches!(answer, CommandResult::Completed { .. }),
            "{answer:?}"
        );
        let slot = doc.slot_of(kentos_domain::Uuid::from_bytes(uid.0)).unwrap();
        let Entity::Insert(now) = doc.get(slot).unwrap() else {
            panic!("insert")
        };
        assert_eq!(now.block, to);
        assert_eq!(
            now.parameters["Genişlik"],
            kentos_commands::ParamValue::Number(1.05)
        );
        assert_eq!(
            now.parameters["Görünüş"],
            kentos_commands::ParamValue::Text("Sade".into())
        );
        let Entity::Dimension(dim) = doc.get(dimension).unwrap() else {
            panic!("dimension")
        };
        let refs = dim.refs.as_ref().unwrap();
        assert_eq!(refs.b.as_ref().unwrap().uid, uid);
        assert_eq!(refs.b.as_ref().unwrap().path, vec![0]);
        assert_eq!(refs.b.as_ref().unwrap().broken, None);
        assert!((dim.b.y - 1.05).abs() < 1e-12, "{dim:?}");
        assert_eq!(doc.block(from).unwrap().card.as_ref().unwrap().version, 1);
        let saved = kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap();
        let reopened = Document::from_snapshot_v2(kentos_kcad::decode(&saved).unwrap()).unwrap();
        assert_eq!(
            kentos_kcad::encode_verified(&reopened.to_snapshot_v2()).unwrap(),
            saved
        );
        doc.undo();
        assert_eq!(
            kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap(),
            before
        );
    }
}
