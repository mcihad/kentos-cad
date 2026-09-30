//! `cad.blocks.define` v1 (docs/adr/0144 §4): a block definition made of a
//! drawing's objects, with the base point given, as one undo step (“Blok
//! tanımla”); with `replace` the objects are deleted and an insert of the new
//! block takes their place in the same step. The desktop's handler over the
//! native document; the web's is `apps/web/src/product/blocksDefine.ts`. Both
//! pass the shared cases in `fixtures/commands/v1/cad.blocks.define.json`.
//!
//! The objects are copied as they are: their geometry, layer, colour, line
//! weight, attributes, label and symbol, with local ids 1, 2, … in the order
//! the input first names them; an id given twice is one object.
//!
//! The checks, in order (the first that fails answers):
//! 1. a name that is not blank; at least one id, each lowercase UUID text; a
//!    finite base point; with `replace`, the insert's layer given;
//! 2. the expected revision;
//! 3. every id names an object of the drawing (in order);
//! 4. the block rules with the new definition (its name once; nesting);
//! 5. with `replace`: the insert's layer, then no object on a locked layer.

use kentos_contracts::blocks::{self as rules};
use kentos_contracts::{
    BlockDefined, BlockDefinition, BlockId, BlocksDefine, BlocksDefinePlan, CommandResult,
    CommandWarning, Entity, EntityBase, InsertEntity, Vec2,
};
use kentos_domain::{Document, Slot, Uuid, labels};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &BlocksDefine) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: (),
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now: the definition (its id the nil UUID, given
/// when written), the insert and the objects it would delete; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &BlocksDefine) -> CommandResult<BlocksDefinePlan> {
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: BlocksDefinePlan {
                block: checked.block,
                insert: checked.insert,
                removed: checked.removed.into_iter().map(|(_, uid)| uid).collect(),
                revision: cx.doc.revision().to_string(),
            },
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the definition, and with `replace` the
/// deletion and the insert, as one undo step “Blok tanımla”: into the open
/// transaction or group, if one is.
pub fn execute(cx: &mut ExecutionContext<'_>, input: BlocksDefine) -> CommandResult<BlockDefined> {
    let mut checked = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let id = BlockId(*Uuid::now_v7().as_bytes());
    checked.block.id = id;
    if let Some(Entity::Insert(insert)) = &mut checked.insert {
        insert.block = id;
    }
    let gone: Vec<Slot> = checked.removed.iter().map(|(slot, _)| *slot).collect();
    let written = write(
        cx.doc,
        labels::BLOCK_ADD,
        checked.block,
        &gone,
        checked.insert,
    );
    let slot = match written {
        Ok(slot) => slot,
        Err(stop) => return stop.into(),
    };
    let doc = &*cx.doc;
    CommandResult::Completed {
        output: BlockDefined {
            block: id,
            insert: slot.and_then(|s| doc.uid(s)).map(|u| u.to_string()),
            id: slot.map(|s| s.0),
            removed: checked.removed.into_iter().map(|(_, uid)| uid).collect(),
            revision: doc.revision().to_string(),
        },
        warnings: checked.warnings,
    }
}

/// What may be written: the definition (nil id), the objects to delete and
/// the insert (slot 0, nil block) with `replace`, and the warnings.
struct Checked {
    block: BlockDefinition,
    removed: Vec<(Slot, String)>,
    insert: Option<Entity>,
    warnings: Vec<CommandWarning>,
}

/// The checks in the contract's order.
fn check(doc: &Document, input: &BlocksDefine) -> Result<Checked, Stop> {
    name(&input.name)?;
    checks::uids(&input.uids, "Bloğa girecek nesne verilmedi.")?;
    checks::point(input.base, "Taban noktasının", "base")?;
    let replace = input.replace == Some(true);
    if replace && input.layer_id.is_none() {
        return Err(no_layer());
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let named = checks::named(doc, &input.uids)?;
    let block = BlockDefinition {
        id: BlockId([0; 16]),
        name: input.name.clone(),
        base: input.base,
        entities: copies(named.iter().map(|(_, _, e, _)| *e)),
        attributes: Vec::new(),
        description: input.description.clone(),
    };
    let mut next: Vec<&BlockDefinition> = doc.blocks().iter().map(|b| &**b).collect();
    next.push(&block);
    block_rules(&next)?;
    let mut checked = Checked {
        removed: Vec::new(),
        insert: None,
        warnings: Vec::new(),
        block: block.clone(),
    };
    if let (true, Some(layer)) = (replace, &input.layer_id) {
        let r = replaced(doc, &named, layer, input.base)?;
        checked.removed = r.removed;
        checked.insert = Some(r.insert);
        checked.warnings = r.warnings;
    }
    Ok(checked)
}

/// A block's name: something besides white space (`empty_name`).
pub(crate) fn name(name: &str) -> Result<(), Stop> {
    if rules::name_ok(name) {
        return Ok(());
    }
    Err(Stop::Failed(error(
        codes::EMPTY_NAME,
        "Blok adı boş olamaz; bir ad yazın.".into(),
        Some("name".into()),
    )))
}

/// `replace` without the layer the insert goes on (`no_layer`).
pub(crate) fn no_layer() -> Stop {
    Stop::Failed(error(
        codes::NO_LAYER,
        "Yerleştirmenin katmanı verilmedi. Seçilenleri blokla değiştirmek için bir katman verin (Blok oluştur etkin katmanı verir).".into(),
        Some("layerId".into()),
    ))
}

/// The objects as a definition holds them: every field kept, local ids 1, 2, … in order.
pub(crate) fn copies<'a>(objects: impl Iterator<Item = &'a Entity>) -> Vec<Entity> {
    objects
        .enumerate()
        .map(|(k, e)| {
            let mut copy = e.clone();
            base_mut(&mut copy).id = u32::try_from(k + 1).unwrap_or(u32::MAX);
            copy
        })
        .collect()
}

fn base_mut(e: &mut Entity) -> &mut EntityBase {
    match e {
        Entity::Point(x) => &mut x.base,
        Entity::Line(x) => &mut x.base,
        Entity::Polyline(x) | Entity::Polygon(x) => &mut x.base,
        Entity::Circle(x) => &mut x.base,
        Entity::Arc(x) => &mut x.base,
        Entity::Ellipse(x) => &mut x.base,
        Entity::Spline(x) => &mut x.base,
        Entity::Xline(x) | Entity::Ray(x) => &mut x.base,
        Entity::Text(x) => &mut x.base,
        Entity::Dimension(x) => &mut x.base,
        Entity::Hatch(x) => &mut x.base,
        Entity::Insert(x) => &mut x.base,
    }
}

/// The drawing's definitions as they would be, by the block rules (docs/adr/0144):
/// a name taken (`duplicate_block`), a definition holding itself (`block_cycle`),
/// nesting too deep (`block_too_deep`), in the documents' words.
pub(crate) fn block_rules(next: &[&BlockDefinition]) -> Result<(), Stop> {
    let checked = rules::definitions(next).and_then(|index| {
        for (i, block) in next.iter().enumerate() {
            rules::inserts(&block.entities, Some(i), &index)?;
        }
        rules::nesting(next, &index).map(|_| ())
    });
    checked.map_err(|fault| {
        let message = fault.message(|i| next.get(i).map_or("", |b| b.name.as_str()));
        let (code, path) = match fault {
            rules::BlockFault::EmptyName { .. } => (codes::EMPTY_NAME, "name"),
            rules::BlockFault::DuplicateId { .. } | rules::BlockFault::DuplicateName { .. } => {
                (codes::DUPLICATE_BLOCK, "name")
            }
            rules::BlockFault::Cycle { .. } => (codes::BLOCK_CYCLE, "uids"),
            rules::BlockFault::TooDeep { .. } => (codes::BLOCK_TOO_DEEP, "uids"),
            rules::BlockFault::BadScale { .. } => (codes::INVALID_SCALE, "uids"),
            rules::BlockFault::UnknownBlock { .. } => (codes::UNKNOWN_BLOCK, "uids"),
            // `attributes` checks its list first: never expected.
            rules::BlockFault::EmptyTag { attribute, .. } => {
                return Stop::Failed(error(
                    codes::EMPTY_TAG,
                    message,
                    Some(format!("attributes[{attribute}].tag")),
                ));
            }
            rules::BlockFault::DuplicateTag { attribute, .. } => {
                return Stop::Failed(error(
                    codes::DUPLICATE_TAG,
                    message,
                    Some(format!("attributes[{attribute}].tag")),
                ));
            }
        };
        Stop::Failed(error(code, message, Some(path.into())))
    })
}

/// What `replace` writes besides the definition: the objects to delete
/// (slot and id), the insert (slot 0) and the insert's layer's warning.
pub(crate) struct Replaced {
    pub removed: Vec<(Slot, String)>,
    pub insert: Entity,
    pub warnings: Vec<CommandWarning>,
}

/// With `replace`: the insert's layer (known, a layer, not locked; hidden
/// with a warning), then every object off locked layers; the objects to
/// delete and the insert of the block at `base` (nil block, slot 0).
pub(crate) fn replaced(
    doc: &Document,
    named: &[(usize, Slot, &Entity, &String)],
    layer: &str,
    base: Vec2,
) -> Result<Replaced, Stop> {
    let warnings = checks::layer(doc, layer)?;
    let layers = doc.layers();
    for (at, _, entity, _) in named {
        let id = &entity.base().layer_id;
        if layers.is_locked(id) {
            let name = layers.get(id).map_or(id.as_str(), |n| n.name.as_str());
            return Err(Stop::Failed(error(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki nesne bloğa alınır ama yerine yerleştirme konamaz. Kilidi Katmanlar panelinden açın ya da nesneleri yerinde bırakın."
                ),
                Some(format!("uids[{at}]")),
            )));
        }
    }
    let removed = named
        .iter()
        .map(|(_, slot, _, uid)| (*slot, (*uid).clone()))
        .collect();
    let insert = Entity::Insert(InsertEntity {
        base: EntityBase {
            id: 0,
            layer_id: layer.to_owned(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
        },
        block: BlockId([0; 16]),
        p: base,
        scale: 1.0,
        rotation: 0.0,
        mirror: false,
    });
    Ok(Replaced {
        removed,
        insert,
        warnings,
    })
}

/// Writes a definition op and, with objects to delete, their deletion and the
/// insert, as one undo step named `label`: the insert's slot. A refusal of the
/// document (a block rule the checks above passed: never expected) is said in
/// its words; a document with no slot left writes nothing.
pub(crate) fn write(
    doc: &mut Document,
    label: &str,
    block: BlockDefinition,
    gone: &[Slot],
    insert: Option<Entity>,
) -> Result<Option<Slot>, Stop> {
    enum Written {
        Refused(String),
        Full(String),
    }
    let adding = !doc.blocks().iter().any(|b| b.id == block.id);
    let written = doc.transact(label, |doc| {
        let applied = if adding {
            doc.add_block(block)
        } else {
            doc.update_block(block).map(|_| ())
        };
        applied.map_err(|r| Written::Refused(r.to_string()))?;
        let Some(insert) = insert else {
            return Ok(None);
        };
        doc.remove(gone);
        let slots = doc
            .add_many(vec![insert], label)
            .map_err(|full| Written::Full(full.to_string()))?;
        Ok(slots.first().copied())
    });
    written.map_err(|w| match w {
        Written::Refused(message) => Stop::Failed(error(codes::BLOCK_REFUSED, message, None)),
        Written::Full(message) => Stop::Failed(error(codes::SLOTS_EXHAUSTED, message, None)),
    })
}
