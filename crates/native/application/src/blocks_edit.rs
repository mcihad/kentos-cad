//! `cad.blocks.edit` v1 (docs/adr/0144 §4): one change of the drawing's
//! block definitions as one undo step named after it: a new name, new
//! objects, a new base point or new attribute definitions (“Blok değiştir”),
//! a definition deleted (“Blok sil”) or every unused one (“Blokları
//! temizle”). Every insert shows a changed definition. The desktop's handler over the native document; the
//! web's is `apps/web/src/product/blocksEdit.ts`. Both pass the shared cases
//! in `fixtures/commands/v1/cad.blocks.edit.json`.
//!
//! What would change nothing writes nothing and completes with nothing in
//! `changed` and `removed`.
//!
//! The checks, in order (the first that fails answers):
//! 1. the block given where the operation needs one; what the operation
//!    needs from the input (a name, objects and a base point, a layer, the
//!    attribute definitions);
//! 2. the expected revision;
//! 3. the block is the drawing's;
//! 4. the operation's own: the block rules with the definition as it would
//!    be, the objects (`redefine`), a definition an insert uses (`remove`).

use std::collections::HashSet;

use kentos_contracts::{
    AttributeDefinition, BlockDefinition, BlockEditOperation, BlockId, BlocksEdit, BlocksEditPlan,
    BlocksEdited, CommandResult, CommandWarning, Entity,
};
use kentos_domain::{Document, Slot, labels};

use crate::ExecutionContext;
use crate::blocks_define::{block_rules, copies, name, no_layer, replaced, write};
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &BlocksEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: (),
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &BlocksEdit) -> CommandResult<BlocksEditPlan> {
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: BlocksEditPlan {
                removed: in_drawing_order(cx.doc, &checked.removed),
                changed: checked.changed.into_iter().collect(),
                insert: checked.insert,
                deleted: checked.deleted.into_iter().map(|(_, uid)| uid).collect(),
                revision: cx.doc.revision().to_string(),
            },
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes it as one undo step named after the
/// operation, into the open transaction or group, if one is.
pub fn execute(cx: &mut ExecutionContext<'_>, input: BlocksEdit) -> CommandResult<BlocksEdited> {
    let checked = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let changed: Vec<BlockId> = checked.changed.iter().map(|b| b.id).collect();
    let removed = in_drawing_order(cx.doc, &checked.removed);
    let mut slot = None;
    if !checked.removed.is_empty() {
        if let Err(stop) = remove(cx.doc, label(input.operation), &checked.removed) {
            return stop.into();
        }
    } else if checked.changed.is_some() || checked.insert.is_some() {
        let block = checked.changed.clone().or_else(|| {
            // The objects replaced by an insert of an unchanged definition.
            input.block.and_then(|id| cx.doc.block(id).cloned())
        });
        let Some(block) = block else {
            return CommandResult::Completed {
                output: done(cx.doc, Vec::new(), Vec::new(), None, Vec::new()),
                warnings: checked.warnings,
            };
        };
        let gone: Vec<Slot> = checked.deleted.iter().map(|(s, _)| *s).collect();
        match write(cx.doc, labels::BLOCK_CHANGE, block, &gone, checked.insert) {
            Ok(written) => slot = written,
            Err(stop) => return stop.into(),
        }
    }
    let deleted = checked.deleted.into_iter().map(|(_, uid)| uid).collect();
    CommandResult::Completed {
        output: done(cx.doc, changed, removed, slot, deleted),
        warnings: checked.warnings,
    }
}

/// `ids` in the order the drawing lists its definitions.
fn in_drawing_order(doc: &Document, ids: &[BlockId]) -> Vec<BlockId> {
    doc.blocks()
        .iter()
        .map(|b| b.id)
        .filter(|id| ids.contains(id))
        .collect()
}

/// The undo step's name (docs/adr/0144 §4).
pub fn label(operation: BlockEditOperation) -> &'static str {
    match operation {
        BlockEditOperation::Rename
        | BlockEditOperation::Redefine
        | BlockEditOperation::Rebase
        | BlockEditOperation::Attributes => labels::BLOCK_CHANGE,
        BlockEditOperation::Remove => labels::BLOCK_REMOVE,
        BlockEditOperation::Purge => labels::BLOCK_PURGE,
    }
}

fn done(
    doc: &Document,
    changed: Vec<BlockId>,
    removed: Vec<BlockId>,
    slot: Option<Slot>,
    deleted: Vec<String>,
) -> BlocksEdited {
    BlocksEdited {
        changed,
        removed,
        insert: slot.and_then(|s| doc.uid(s)).map(|u| u.to_string()),
        id: slot.map(|s| s.0),
        deleted,
        revision: doc.revision().to_string(),
    }
}

/// What may be written: the definition as it will be (when it changes), the
/// definitions to delete (the order they go in), and with `replace` the
/// objects to delete and the insert.
struct Checked {
    changed: Option<BlockDefinition>,
    removed: Vec<BlockId>,
    deleted: Vec<(Slot, String)>,
    insert: Option<Entity>,
    warnings: Vec<CommandWarning>,
}

impl Checked {
    fn nothing() -> Checked {
        Checked {
            changed: None,
            removed: Vec::new(),
            deleted: Vec::new(),
            insert: None,
            warnings: Vec::new(),
        }
    }
}

/// The checks in the contract's order.
fn check(doc: &Document, input: &BlocksEdit) -> Result<Checked, Stop> {
    let needs_block = input.operation != BlockEditOperation::Purge;
    if needs_block && input.block.is_none() {
        return Err(Stop::Failed(error(
            codes::NO_BLOCK,
            "Değiştirilecek blok verilmedi. Bloğun kimliğini verin.".into(),
            Some("block".into()),
        )));
    }
    let empty = Vec::new();
    let uids = input.uids.as_ref().unwrap_or(&empty);
    match input.operation {
        BlockEditOperation::Rename => name(input.name.as_deref().unwrap_or(""))?,
        BlockEditOperation::Redefine => {
            checks::uids(uids, "Bloğa girecek nesne verilmedi.")?;
            if let Some(base) = input.base {
                checks::point(base, "Taban noktasının", "base")?;
            }
            if input.replace == Some(true) && input.layer_id.is_none() {
                return Err(no_layer());
            }
        }
        BlockEditOperation::Rebase => {
            let Some(base) = input.base else {
                return Err(Stop::Failed(error(
                    codes::NO_BASE,
                    "Yeni taban noktası verilmedi. Bloğun taban noktasını verin.".into(),
                    Some("base".into()),
                )));
            };
            checks::point(base, "Taban noktasının", "base")?;
        }
        BlockEditOperation::Attributes => attributes(input.attributes.as_deref())?,
        BlockEditOperation::Remove | BlockEditOperation::Purge => {}
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    if input.operation == BlockEditOperation::Purge {
        return Ok(Checked {
            removed: unused(doc),
            ..Checked::nothing()
        });
    }
    let Some(old) = input.block.and_then(|id| doc.block(id)) else {
        let id = input.block.map(|b| b.to_text()).unwrap_or_default();
        return Err(Stop::Failed(error(
            codes::UNKNOWN_BLOCK,
            format!(
                "“{id}” kimlikli blok çizimde tanımlı değil: silinmiş ya da başka bir çizimin olabilir. Çizimde tanımlı bir bloğun kimliğini verin."
            ),
            Some("block".into()),
        )));
    };
    let mut checked = Checked::nothing();
    let mut next = old.clone();
    match input.operation {
        BlockEditOperation::Rename => {
            next.name = input.name.clone().unwrap_or_default();
            rules_with(doc, &next)?;
        }
        BlockEditOperation::Rebase => {
            next.base = input.base.unwrap_or(old.base);
            rules_with(doc, &next)?;
        }
        BlockEditOperation::Attributes => {
            next.attributes = input.attributes.clone().unwrap_or_default();
            rules_with(doc, &next)?;
        }
        BlockEditOperation::Redefine => {
            let named = checks::named(doc, uids)?;
            crate::blocks_define::no_tables(&named)?;
            next.entities = copies(named.iter().map(|(_, _, e, _)| *e));
            next.base = input.base.unwrap_or(old.base);
            rules_with(doc, &next)?;
            if let (true, Some(layer)) = (input.replace == Some(true), &input.layer_id) {
                let mut r = replaced(doc, &named, layer, next.base)?;
                if let Entity::Insert(i) = &mut r.insert {
                    i.block = old.id;
                }
                checked.deleted = r.removed;
                checked.insert = Some(r.insert);
                checked.warnings = r.warnings;
            }
        }
        BlockEditOperation::Remove => {
            if let Some(reason) = doc.block_removal_refused(old.id) {
                return Err(Stop::Failed(error(
                    codes::BLOCK_IN_USE,
                    reason,
                    Some("block".into()),
                )));
            }
            checked.removed = vec![old.id];
            return Ok(checked);
        }
        BlockEditOperation::Purge => {}
    }
    if next != *old {
        checked.changed = Some(next);
    }
    Ok(checked)
}

/// The attribute definitions `attributes` writes (docs/adr/0144 §7): the
/// list given, then each in its order: a tag, not one an earlier one has
/// (exactly), a finite point and turn, a height above zero.
fn attributes(list: Option<&[AttributeDefinition]>) -> Result<(), Stop> {
    let Some(list) = list else {
        return Err(Stop::Failed(error(
            codes::NO_ATTRIBUTES,
            "Öznitelik listesi verilmedi. Bloğun bütün özniteliklerini verin; boş liste hepsini kaldırır.".into(),
            Some("attributes".into()),
        )));
    };
    for (i, a) in list.iter().enumerate() {
        let at = format!("attributes[{i}]");
        if checks::is_blank(&a.tag) {
            return Err(Stop::Failed(error(
                codes::EMPTY_TAG,
                "Öznitelik etiketi boş olamaz; her özniteliğe bir etiket verin.".into(),
                Some(format!("{at}.tag")),
            )));
        }
        if list[..i].iter().any(|b| b.tag == a.tag) {
            return Err(Stop::Failed(error(
                codes::DUPLICATE_TAG,
                format!(
                    "“{}” etiketi listede iki kez var; her etiket bir kez olmalı.",
                    a.tag
                ),
                Some(format!("{at}.tag")),
            )));
        }
        let whose = format!("“{}” özniteliğinin yerinin", a.tag);
        checks::point(a.p, &whose, &format!("{at}.p"))?;
        checks::finite(
            a.rotation,
            &format!("“{}” özniteliğinin açısı", a.tag),
            "Açıyı sonlu bir sayıyla verin.",
            &format!("{at}.rotation"),
        )?;
        if !(a.height.is_finite() && a.height > 0.0) {
            return Err(Stop::Failed(error(
                codes::INVALID_HEIGHT,
                format!(
                    "“{}” özniteliğinin yazı yüksekliği sıfırdan büyük, sonlu bir sayı olmalı (metre, bloğun kendi ölçüsünde).",
                    a.tag
                ),
                Some(format!("{at}.height")),
            )));
        }
    }
    Ok(())
}

/// The block rules with `block` in its definition's place.
fn rules_with(doc: &Document, block: &BlockDefinition) -> Result<(), Stop> {
    let next: Vec<&BlockDefinition> = doc
        .blocks()
        .iter()
        .map(|b| if b.id == block.id { block } else { &**b })
        .collect();
    block_rules(&next)
}

/// The definitions no insert uses, in the order they can go: first those no
/// object or other definition places, then those only they placed, and so on.
fn unused(doc: &Document) -> Vec<BlockId> {
    let mut gone: Vec<BlockId> = Vec::new();
    let mut left: Vec<&BlockDefinition> = doc.blocks().iter().map(|b| &**b).collect();
    loop {
        let mut used: HashSet<BlockId> = HashSet::new();
        for e in doc.entities() {
            if let Entity::Insert(i) = e {
                used.insert(i.block);
            }
        }
        for b in &left {
            for e in &b.entities {
                if let Entity::Insert(i) = e {
                    used.insert(i.block);
                }
            }
        }
        let round: Vec<BlockId> = left
            .iter()
            .map(|b| b.id)
            .filter(|id| !used.contains(id))
            .collect();
        if round.is_empty() {
            return gone;
        }
        left.retain(|b| !round.contains(&b.id));
        gone.extend(round);
    }
}

/// Deletes the definitions in their order as one undo step named `label`.
fn remove(doc: &mut Document, label: &str, ids: &[BlockId]) -> Result<(), Stop> {
    let written = doc.transact(label, |doc| {
        for id in ids {
            doc.remove_block(*id).map_err(|r| r.to_string())?;
        }
        Ok::<(), String>(())
    });
    written.map_err(|message| Stop::Failed(error(codes::BLOCK_REFUSED, message, None)))
}
