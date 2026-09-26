//! `cad.entities.delete` v1 (docs/adr/0029): objects named by their
//! persistent ids deleted as one undo step (“Sil”, the document's own
//! `remove`). The desktop's handler over the native document; the web's is
//! `apps/web/src/product/entitiesDelete.ts`. Both pass the shared cases in
//! `fixtures/commands/v1/cad.entities.delete.json`.
//!
//! The erase tool (Sil, the Delete key) makes the selection explicit here
//! (TODOS.md CMD-07): it gives the selected objects' ids, or the one it
//! picked. Objects on a locked layer stay, as the tool always left them: with
//! others to delete, with a warning; alone, the answer is a refusal.
//!
//! The checks, in order (the first that fails answers):
//! 1. at least one id; every id lowercase UUID text with hyphens (in order);
//! 2. the expected revision (every command's, `checks.rs`);
//! 3. every id names an object of the document (in order);
//! 4. not every object on a locked layer.
//!
//! Ids are checked before the document is read, and the revision before the
//! objects: an input that is wrong by itself is refused whatever the drawing
//! holds, and one prepared against another revision may name objects that
//! have changed since (docs/adr/0022).

use std::collections::HashSet;

use kentos_contracts::{
    CommandResult, CommandWarning, EntitiesDelete, EntitiesDeletePlan, EntitiesDeleted,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// Checks `input` against the document, deleting nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesDelete) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: (),
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would delete now and leave in place, and the revision to
/// expect for exactly that; deletes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &EntitiesDelete) -> CommandResult<EntitiesDeletePlan> {
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: EntitiesDeletePlan {
                removed: checked.removed.into_iter().map(|(_, uid)| uid).collect(),
                locked: checked.locked,
                revision: cx.doc.revision().to_string(),
            },
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and deletes the objects not on a locked layer as one
/// undo step (“Sil”): into the open transaction or group, if one is.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesDelete,
) -> CommandResult<EntitiesDeleted> {
    let checked = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let slots: Vec<Slot> = checked.removed.iter().map(|(slot, _)| *slot).collect();
    cx.doc.remove(&slots);
    CommandResult::Completed {
        output: EntitiesDeleted {
            removed: checked.removed.into_iter().map(|(_, uid)| uid).collect(),
            locked: checked.locked,
            revision: cx.doc.revision().to_string(),
        },
        warnings: checked.warnings,
    }
}

/// What may be deleted: the objects to delete (slot and id), those that
/// stay on locked layers, and the warning about them.
struct Checked {
    removed: Vec<(Slot, String)>,
    locked: Vec<String>,
    warnings: Vec<CommandWarning>,
}

/// The checks in the contract's order.
fn check(doc: &Document, input: &EntitiesDelete) -> Result<Checked, Stop> {
    if input.uids.is_empty() {
        return Err(Stop::Failed(checks::error(
            codes::NO_ENTITIES,
            "Silinecek nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.".into(),
            Some("uids".into()),
        )));
    }
    for (i, uid) in input.uids.iter().enumerate() {
        if !is_uid_text(uid) {
            return Err(Stop::Failed(checks::error(
                codes::INVALID_UID,
                format!(
                    "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."
                ),
                Some(format!("uids[{i}]")),
            )));
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut seen = HashSet::new();
    let mut removed = Vec::new();
    let mut locked = Vec::new();
    for (i, uid) in input.uids.iter().enumerate() {
        if !seen.insert(uid.as_str()) {
            continue;
        }
        let found = Uuid::parse_str(uid)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .and_then(|slot| Some((slot, doc.get(slot)?)));
        let Some((slot, entity)) = found else {
            return Err(Stop::Failed(checks::error(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                ),
                Some(format!("uids[{i}]")),
            )));
        };
        if doc.layers().is_locked(&entity.base().layer_id) {
            locked.push(uid.clone());
        } else {
            removed.push((slot, uid.clone()));
        }
    }
    let message = || {
        format!(
            "{} nesne kilitli katmanda olduğu için silinmedi. Silmek için katmanın kilidini Katmanlar panelinden açın.",
            locked.len()
        )
    };
    if removed.is_empty() {
        return Err(Stop::Failed(checks::error(
            codes::LAYER_LOCKED,
            message(),
            Some("uids".into()),
        )));
    }
    let mut warnings = Vec::new();
    if !locked.is_empty() {
        warnings.push(CommandWarning {
            code: codes::LAYER_LOCKED.into(),
            message: message(),
            path: Some("uids".into()),
        });
    }
    Ok(Checked {
        removed,
        locked,
        warnings,
    })
}

/// A persistent id as the contract writes it: lowercase hexadecimal with
/// hyphens, 8-4-4-4-12 (the web's `isUuid`).
fn is_uid_text(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => b.is_ascii_digit() || (b'a'..=b'f').contains(b),
        })
}

#[cfg(test)]
mod tests {
    use super::is_uid_text;

    #[test]
    fn ids_are_lowercase_uuid_text_with_hyphens() {
        assert!(is_uid_text("01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"));
        for bad in [
            "",
            "01925F3E-7C1A-7D2B-9E4F-0A1B2C3D4E5F",
            "01925f3e7c1a7d2b9e4f0a1b2c3d4e5f",
            "{01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f}",
            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5",
            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5g",
            "01925f3e_7c1a-7d2b-9e4f-0a1b2c3d4e5f",
            "12",
        ] {
            assert!(!is_uid_text(bad), "{bad}");
        }
    }
}
