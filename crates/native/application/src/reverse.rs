//! `cad.entities.reverse` v1, Yönü çevir (docs/adr/0296; AutoCAD's REVERSE):
//! objects turned the other way round, the same outline, as one undo step
//! “Yönü çevir”. The desktop's handler over the native document, run by the
//! Yönü çevir tool, the headless server, Python and MCP. The shared cases are
//! `fixtures/commands/v1/cad.entities.reverse.json`.
//!
//! The reversed shape is the shared core's (`ops::reshape::reverse`), written
//! through `cad.entities.edit`'s `reverse` operation ([`crate::edit`]): each
//! object keeps its id and all it carries, its elevations, measures and
//! widths go with their vertices (so they run backwards too), and an
//! associative dimension's point keeps its place on it
//! (`Document::relocate_ties`).
//!
//! The checks, in order (the first that fails answers):
//! 1. at least one id, each lowercase UUID text with hyphens;
//! 2. the expected revision (every command's, `checks.rs`);
//! 3. every id names an object;
//! 4. not every object on a locked layer (those that are, left out and counted);
//! 5. at least one object with a direction of its own (each without one, a warning).

use kentos_contracts::{
    CommandResult, CommandWarning, EditOperation, EntitiesEdit, EntitiesEditPlan, EntitiesEdited,
    EntitiesReverse, EntityEdit,
};
use kentos_domain::Document;
use kentos_geometry_core::entity::Entity as CoreEntity;
use kentos_geometry_core::ops::reshape::reverse;

use crate::ExecutionContext;
use crate::break_join::{Checked, Ties, execute_with, plan_with, validate_with};
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::{edit_geometry, shape};

/// The undo step of `cad.entities.reverse`: the tool's name.
pub const REVERSE_LABEL: &str = "Yönü çevir";

/// The words of an object with no direction of its own.
const NO_DIRECTION: &str = "Nesnenin kendi yönü yok: yay, daire ve elips hep saat yönünün tersine gider; nokta, yazı ve benzerleri yönsüz.";

/// The checks of `cad.entities.reverse`, in order; the edit it writes.
fn check(doc: &Document, input: &EntitiesReverse) -> Result<Checked, Stop> {
    checks::uids(&input.uids, "Yönü çevrilecek nesne yok.")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let named = checks::named(doc, &input.uids)?;
    let (open, skipped) = checks::skip_locked(doc, named)?;
    let mut warnings: Vec<CommandWarning> = skipped.into_iter().collect();
    let mut changes = Vec::new();
    let mut ties = Ties::default();
    for (i, slot, e, uid) in open {
        let turned = reverse(&CoreEntity::new(shape(e))).and_then(|r| edit_geometry(r.shape));
        let Some(geometry) = turned else {
            warnings.push(CommandWarning {
                code: codes::NOT_REVERSIBLE.into(),
                message: NO_DIRECTION.into(),
                path: Some(format!("uids[{i}]")),
            });
            continue;
        };
        changes.push(EntityEdit::Update {
            uid: uid.clone(),
            geometry,
        });
        if let Some(u) = doc.uid(slot) {
            ties.onto.push((u, vec![u]));
        }
    }
    if changes.is_empty() {
        return Err(Stop::failed(error(
            codes::NOT_REVERSIBLE,
            format!("Yönü çevrilecek nesne kalmadı. {NO_DIRECTION}"),
            Some("uids".into()),
        )));
    }
    Ok((
        EntitiesEdit {
            operation: EditOperation::Reverse,
            changes,
            expected_revision: None,
        },
        warnings,
        ties,
    ))
}

/// `cad.entities.reverse` v1: Yönü çevir.
pub mod entities_reverse {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesReverse) -> CommandResult<()> {
        validate_with(check(cx.doc, input))
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &EntitiesReverse,
    ) -> CommandResult<EntitiesEditPlan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] over the document alone (the tool's preview).
    pub fn plan_in(doc: &Document, input: &EntitiesReverse) -> CommandResult<EntitiesEditPlan> {
        plan_with(doc, check(doc, input))
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: EntitiesReverse,
    ) -> CommandResult<EntitiesEdited> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_ENTITIES_REVERSE,
            kentos_contracts::CAD_ENTITIES_REVERSE_VERSION,
            input,
            |cx, input| {
                let checked = check(cx.doc, &input);
                execute_with(cx, checked)
            },
        )
    }
}
