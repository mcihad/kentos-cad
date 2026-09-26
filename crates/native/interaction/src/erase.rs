//! Sil: the web's `EraseTool` (`apps/web/src/tools/drawTools.ts`), writing
//! through the product command `cad.entities.delete` (docs/adr/0029):
//!
//! - started with a selection (the Delete key after selecting), it deletes
//!   the selection and leaves;
//! - started without one, it waits: every click deletes the object under
//!   the pointer (picked as the select tool picks), which is hovered as the
//!   pointer moves, until Esc.
//!
//! The selection becomes the command's explicit input (TODOS.md CMD-07): the
//! persistent ids of the selected objects. Objects on a locked layer stay,
//! with the command's warning; when every one is locked the command refuses
//! and nothing is deleted. One undo step, “Sil”. The messages are the web's.

use kentos_contracts::EntitiesDelete;
use kentos_domain::Slot;
use kentos_native_application::{ExecutionContext, delete};

use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Tool};

/// The erase tool's id: its command is `tool.erase`.
pub const ID: &str = "erase";
pub const LABEL: &str = "Sil";

/// The erase tool.
#[derive(Clone, Debug, Default)]
pub struct Erase;

impl Erase {
    pub fn new() -> Self {
        Self
    }

    /// Deletes these objects through `cad.entities.delete`, saying what the
    /// web's tool says: the command's refusal or warning, then how many went.
    fn erase(&self, ids: &[Slot], cx: &mut Context<'_>) {
        let uids = ids
            .iter()
            .filter_map(|slot| cx.doc.uid(*slot))
            .map(|uid| uid.to_string())
            .collect();
        let input = EntitiesDelete {
            uids,
            expected_revision: None,
        };
        let result = delete::execute(&mut ExecutionContext::new(cx.doc), input);
        let Some(deleted) = points::written(result, cx) else {
            return;
        };
        let doc = &*cx.doc;
        cx.selection.retain(|slot| doc.get(slot).is_some());
        cx.selection.set_hover(None);
        cx.say(
            Level::Success,
            format!("{} nesne silindi.", deleted.removed.len()),
        );
    }
}

impl Tool for Erase {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(LABEL, "silinecek nesneye tıklayın")
    }

    fn point_count(&self) -> usize {
        0
    }

    /// With a selection: deletes it and leaves (the web's `activate`).
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        if cx.selection.is_empty() {
            return Flow::Stay;
        }
        let ids = cx.selection.ids().to_vec();
        self.erase(&ids, cx);
        Flow::Exit
    }

    fn snaps(&self) -> bool {
        false
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let hit = cx.spatial.pick(p.raw, cx.pick_tolerance());
        cx.selection.set_hover(hit);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if let Some(hit) = cx.spatial.pick(p.raw, cx.pick_tolerance()) {
            self.erase(&[hit], cx);
        }
    }

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// Enter: the web starts the tool again (`repeatLast`), which waits as this one does.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview::default()
    }
}
