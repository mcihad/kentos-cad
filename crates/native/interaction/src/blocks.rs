//! What the Bloklar panel and the block commands do (the web's
//! `apps/web/src/app/blocks.ts`, docs/adr/0144 §6), in the same words: every
//! change goes through `cad.blocks.edit`, one undo step, a refusal in its own
//! words; how often each block is placed; the insert a new base point is
//! shown on; why a block cannot be deleted. What a call has to say comes back
//! as lines for the host's log.

use kentos_contracts::blocks::{Placements, placements};
use kentos_contracts::{
    BlockEditOperation, BlockId, BlocksEdit, BlocksEdited, CommandResult, Entity,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, blocks_edit};

use crate::Vec2;
use crate::log::Level;
use crate::selection::Selection;

/// A line for the host's log: its level and words.
pub type Said = (Level, String);

fn name_of(doc: &Document, id: BlockId) -> String {
    doc.block(id).map_or_else(String::new, |b| b.name.clone())
}

fn input(operation: BlockEditOperation, block: Option<BlockId>) -> BlocksEdit {
    BlocksEdit {
        operation,
        block,
        name: None,
        uids: None,
        base: None,
        replace: None,
        layer_id: None,
        attributes: None,
        expected_revision: None,
    }
}

/// Runs `cad.blocks.edit`: its output, its warnings said; or its refusal said.
fn edit(doc: &mut Document, input: BlocksEdit, said: &mut Vec<Said>) -> Option<BlocksEdited> {
    match blocks_edit::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { output, warnings } => {
            said.extend(warnings.into_iter().map(|w| (Level::Warn, w.message)));
            Some(output)
        }
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => {
            said.push((Level::Warn, error.message));
            None
        }
        CommandResult::Cancelled | CommandResult::Queued { .. } => None,
    }
}

/// Every definition's placements, in the definitions' order.
pub fn placements_of(doc: &Document) -> Vec<Placements> {
    placements(doc.blocks(), doc.entities())
}

/// Blokları temizle: every definition no insert uses, in one step.
pub fn purge(doc: &mut Document) -> Vec<Said> {
    let mut said = Vec::new();
    if let Some(out) = edit(doc, input(BlockEditOperation::Purge, None), &mut said) {
        said.push(if out.removed.is_empty() {
            (Level::Info, "Kullanılmayan blok yok.".to_owned())
        } else {
            let n = out.removed.len();
            (Level::Success, format!("{n} kullanılmayan blok silindi."))
        });
    }
    said
}

/// A new name for a block: whether the command took it, and what to say.
pub fn rename(doc: &mut Document, id: BlockId, name: &str) -> (bool, Vec<Said>) {
    let was = name_of(doc, id);
    let mut said = Vec::new();
    let out = edit(
        doc,
        BlocksEdit {
            name: Some(name.to_owned()),
            ..input(BlockEditOperation::Rename, Some(id))
        },
        &mut said,
    );
    if out.as_ref().is_some_and(|o| !o.changed.is_empty()) {
        said.push((
            Level::Success,
            format!("“{was}” bloğunun adı “{name}” oldu."),
        ));
    }
    (out.is_some(), said)
}

/// Deletes a block no insert uses (the command refuses one in use, in its words).
pub fn remove(doc: &mut Document, id: BlockId) -> Vec<Said> {
    let name = name_of(doc, id);
    let mut said = Vec::new();
    if edit(doc, input(BlockEditOperation::Remove, Some(id)), &mut said).is_some() {
        said.push((Level::Success, format!("“{name}” bloğu silindi.")));
    }
    said
}

/// The drawing's inserts of a block, in the drawing's order.
pub fn inserts_of(doc: &Document, id: BlockId) -> Vec<Slot> {
    doc.entities()
        .filter(|e| matches!(e, Entity::Insert(i) if i.block == id))
        .map(|e| Slot(e.base().id))
        .collect()
}

/// Selects the drawing's inserts of a block.
pub fn select_inserts(doc: &Document, selection: &mut Selection, id: BlockId) -> Said {
    let name = name_of(doc, id);
    let slots = inserts_of(doc, id);
    if slots.is_empty() {
        return (
            Level::Info,
            format!("“{name}” bloğunun çizimde yerleştirmesi yok."),
        );
    }
    let n = slots.len();
    selection.set(slots);
    (
        Level::Info,
        format!("“{name}” bloğunun {n} yerleştirmesi seçildi."),
    )
}

/// The insert of a block a point is shown on: its selected one, or its only
/// one; else whether it has none (`Err(true)`) or several.
fn shown_on(doc: &Document, selection: &Selection, id: BlockId) -> Result<Slot, bool> {
    let all = inserts_of(doc, id);
    let chosen: Vec<Slot> = all
        .iter()
        .copied()
        .filter(|s| selection.contains(*s))
        .collect();
    match (chosen.as_slice(), all.as_slice()) {
        ([one], _) | (_, [one]) => Ok(*one),
        (_, []) => Err(true),
        _ => Err(false),
    }
}

/// The insert a new base point is shown on: the selected insert of the
/// block, or its only one in the drawing; else why there is none to show it on.
pub fn rebase_insert(doc: &Document, selection: &Selection, id: BlockId) -> Result<Slot, String> {
    let name = name_of(doc, id);
    shown_on(doc, selection, id).map_err(|none| {
        if none {
            format!(
                "“{name}” bloğu çizimde yerleştirilmemiş: yeni taban noktası bir yerleştirmesinde gösterilir. Önce Blok ekle ile yerleştirin."
            )
        } else {
            format!(
                "“{name}” bloğunun birden çok yerleştirmesi var: yeni taban noktasını göstereceğiniz yerleştirmeyi seçin."
            )
        }
    })
}

/// The insert an attribute's place is shown on (Blok öznitelikleri, the
/// web's `attributesInsert`), as the base point's; else why none.
pub fn attributes_insert(
    doc: &Document,
    selection: &Selection,
    id: BlockId,
) -> Result<Slot, String> {
    let name = name_of(doc, id);
    shown_on(doc, selection, id).map_err(|none| {
        if none {
            format!(
                "“{name}” bloğu çizimde yerleştirilmemiş: özniteliğin yeri bir yerleştirmesinde gösterilir. Yeri yazın ya da önce Blok ekle ile yerleştirin."
            )
        } else {
            format!(
                "“{name}” bloğunun birden çok yerleştirmesi var: yeri göstereceğiniz yerleştirmeyi pencereyi açmadan önce seçin ya da yeri yazın."
            )
        }
    })
}

/// The one block the selection's inserts place (Blok öznitelikleri's
/// command, the web's `selectedBlock`); none for none or several.
pub fn selected_block(doc: &Document, selection: &Selection) -> Option<BlockId> {
    let mut found: Option<BlockId> = None;
    for slot in selection.ids() {
        if let Some(kentos_contracts::Entity::Insert(i)) = doc.get(*slot) {
            match found {
                Some(b) if b != i.block => return None,
                _ => found = Some(i.block),
            }
        }
    }
    found
}

/// What `cad.blocks.edit` answers for an attribute list now (Blok
/// öznitelikleri's check at every change): nothing, or its refusal's words.
pub fn check_attributes(
    doc: &mut Document,
    id: BlockId,
    attributes: Vec<kentos_contracts::AttributeDefinition>,
) -> Result<(), String> {
    let input = BlocksEdit {
        attributes: Some(attributes),
        ..input(BlockEditOperation::Attributes, Some(id))
    };
    match blocks_edit::validate(&ExecutionContext::new(doc), &input) {
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Ok(()),
    }
}

/// Blok öznitelikleri's Kaydet: the definition's whole attribute list
/// through `cad.blocks.edit` (`attributes`); what it said, and whether it
/// completed. Nothing is said when the list was already so.
pub fn set_attributes(
    doc: &mut Document,
    id: BlockId,
    attributes: Vec<kentos_contracts::AttributeDefinition>,
) -> (Vec<Said>, bool) {
    let name = name_of(doc, id);
    let n = attributes.len();
    let mut said = Vec::new();
    let out = edit(
        doc,
        BlocksEdit {
            attributes: Some(attributes),
            ..input(BlockEditOperation::Attributes, Some(id))
        },
        &mut said,
    );
    let Some(out) = out else {
        return (said, false);
    };
    if !out.changed.is_empty() {
        said.push((
            Level::Success,
            if n > 0 {
                format!("“{name}” bloğunun öznitelikleri kaydedildi: {n} öznitelik.")
            } else {
                format!("“{name}” bloğunun öznitelikleri kaldırıldı.")
            },
        ));
    }
    (said, true)
}

/// A new base point, in the definition's own coordinates (the host takes the
/// point shown on an insert back with the store's `insert_local`).
pub fn rebase(doc: &mut Document, id: BlockId, base: Vec2) -> Vec<Said> {
    let name = name_of(doc, id);
    let mut said = Vec::new();
    let out = edit(
        doc,
        BlocksEdit {
            base: Some(kentos_contracts::Vec2 {
                x: base.x,
                y: base.y,
            }),
            ..input(BlockEditOperation::Rebase, Some(id))
        },
        &mut said,
    );
    if out.is_some_and(|o| !o.changed.is_empty()) {
        said.push((
            Level::Success,
            format!("“{name}” bloğunun taban noktası değişti."),
        ));
    }
    said
}

/// Why redefining from the selection cannot start: nothing is selected.
pub fn redefine_refusal(doc: &Document, id: BlockId) -> String {
    format!(
        "“{}” bloğunun yeni nesnelerini önce seçin.",
        name_of(doc, id)
    )
}

/// The block made of these objects (persistent ids) from `base`; the objects stay.
pub fn redefine(doc: &mut Document, id: BlockId, uids: Vec<String>, base: Vec2) -> Vec<Said> {
    let name = name_of(doc, id);
    let n = uids.len();
    let mut said = Vec::new();
    let out = edit(
        doc,
        BlocksEdit {
            uids: Some(uids),
            base: Some(kentos_contracts::Vec2 {
                x: base.x,
                y: base.y,
            }),
            ..input(BlockEditOperation::Redefine, Some(id))
        },
        &mut said,
    );
    if out.is_some_and(|o| !o.changed.is_empty()) {
        said.push((
            Level::Success,
            format!("“{name}” bloğu {n} nesneyle yeniden tanımlandı."),
        ));
    }
    said
}

/// Why a block cannot be deleted now (an insert places it), or `None`.
pub fn remove_refusal(doc: &Document, id: BlockId) -> Option<String> {
    let at = doc.blocks().iter().position(|b| b.id == id)?;
    let p = *placements_of(doc).get(at)?;
    if !p.used() {
        return None;
    }
    let mut where_ = Vec::new();
    if p.drawing > 0 {
        where_.push(format!("çizimde {} yerleştirmesi", p.drawing));
    }
    if p.nested > 0 {
        where_.push(format!("{} bloğun içinde yerleştirmesi", p.nested));
    }
    Some(format!(
        "“{}” bloğu kullanılıyor ({}); önce onları silin ya da patlatın.",
        name_of(doc, id),
        where_.join(", ")
    ))
}

/// “Çizimde 3 yerleştirme; 1 bloğun içinde” (a row's tip; the web's `placedText`).
pub fn placed_text(p: Placements) -> String {
    let first = if p.drawing > 0 {
        format!("Çizimde {} yerleştirme", p.drawing)
    } else {
        "Çizimde yerleştirmesi yok".to_owned()
    };
    if p.nested > 0 {
        format!("{first}; {} bloğun içinde", p.nested)
    } else {
        first
    }
}
