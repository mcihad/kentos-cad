//! Eleman tipleri (docs/adr/8106; plan/bim-plan.md BIM-12): the project's column, beam, slab, roof, railing and stair
//! types, a setting the drawing keeps in its undo history, written in the step with the elements that follow them so
//! undo gives both back; and the rule that keeps an element's exceptions true whatever edits it.

use std::sync::Arc;

use kentos_contracts::{ElementType, ElementTypeKind, Entity, element_types_problem, exceptions_after};

use crate::Refusal;
use crate::document::Document;
use crate::history::ArchOp;
use crate::history::Op;

impl Document {
    /// The project's element types, in the order they were made.
    pub fn element_types(&self) -> &[ElementType] {
        self.settings.arch.as_ref().map_or(&[], |a| a.element_types.as_slice())
    }

    /// The project's element type of id `id`.
    pub fn element_type(&self, id: &str) -> Option<&ElementType> {
        self.element_types().iter().find(|t| t.id == id)
    }

    /// Gives the project the types `next` as one change named `label` (into the open transaction, else an undo step of
    /// its own). Refused with nothing changed: types with a problem. Whether anything changed.
    pub fn set_element_types(&mut self, next: Vec<ElementType>, label: &str) -> Result<bool, Refusal> {
        if let Some((_, problem)) = element_types_problem(&next) {
            return Err(Refusal(problem));
        }
        if self.element_types() == next.as_slice() {
            return Ok(false);
        }
        let before = Arc::new(self.element_types().to_vec());
        self.transact(label, |doc| {
            doc.record(vec![Op::Arch(ArchOp::ElementTypes { before, after: Arc::new(next) })], label);
            Ok(true)
        })
    }

    /// An element edited from `before` to `after` that follows a project type keeps its exceptions true (docs/adr/8106
    /// §2): every edit, whatever wrote it — Öznitelikler, a grip, a script — sets them again; one set back to the type's
    /// value follows the type again. An element of another kind than its type's, or whose type is gone, is left as it
    /// is.
    pub(crate) fn keep_exceptions(&self, before: &Entity, after: &mut Entity) {
        let Some(link) = after.arch_info().and_then(|a| a.element_type.as_ref()) else {
            return;
        };
        let Some(t) = self.element_type(&link.id) else {
            return;
        };
        if ElementTypeKind::of(after) != Some(t.kind) {
            return;
        }
        let next = exceptions_after(before, after, t);
        if let Some(link) = after.arch_info_mut().and_then(|a| a.element_type.as_mut())
            && link.overrides != next
        {
            link.overrides = next;
        }
    }
}
