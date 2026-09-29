//! Blok oluştur: the web's `BlockDefineTool` (`apps/web/src/tools/blockTools.ts`)
//! on its `SelectionFirstTool` base ([`crate::modify`]), step for step
//! (docs/adr/0144 §6):
//!
//! - the objects (the selection, or picked first), then the base point,
//!   clicked or typed;
//! - the tool leaves and asks the host for the window that names the block
//!   ([`ViewChange::DefineBlock`]; the objects are the selection). The
//!   window checks what it would write at every change ([`check`]) and
//!   writes it through `cad.blocks.define` ([`define`]), one undo step
//!   “Blok tanımla”; the web's window is `ui/blocks/BlockDefineDialog.ts`.

use kentos_contracts::{BlockDefined, BlocksDefine, CommandResult};
use kentos_domain::Document;
use kentos_geometry_core::geom::affine::Affine;
use kentos_native_application::{ExecutionContext, blocks_define};

use crate::Vec2;
use crate::modify::{Modify, Stages};
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, ViewChange};

/// The tool's id: its command is `tool.blockDefine`.
pub const ID: &str = "blockDefine";
pub const LABEL: &str = "Blok oluştur";

/// Blok oluştur's one stage: the base point.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlockDefine;

impl BlockDefine {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self)
    }
}

impl Stages for BlockDefine {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn begin(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        Prompt::new(LABEL, "taban noktasına tıklayın ya da Y,X yazın")
    }

    /// The base point: the window that names the block opens, the tool leaves.
    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        cx.view_changes.push(ViewChange::DefineBlock(p));
        Flow::Exit
    }

    fn typed(&mut self, _text: &str, _cx: &mut Context<'_>) -> Option<Flow> {
        None
    }

    fn preview(&self, _hover: Vec2) -> Option<Affine> {
        None
    }
}

/// What `cad.blocks.define` answers for `input` now, writing nothing: the
/// warnings' words, or the refusal's (the web window's `refresh`).
pub fn check(doc: &mut Document, input: &BlocksDefine) -> Result<Vec<String>, String> {
    match blocks_define::validate(&ExecutionContext::new(doc), input) {
        CommandResult::Completed { warnings, .. } => {
            Ok(warnings.into_iter().map(|w| w.message).collect())
        }
        other => Err(refusal(other)),
    }
}

/// Writes the block through `cad.blocks.define` (the window's Oluştur): the
/// command's output with its warnings' words, or the refusal's words.
pub fn define(
    doc: &mut Document,
    input: BlocksDefine,
) -> Result<(BlockDefined, Vec<String>), String> {
    match blocks_define::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { output, warnings } => {
            Ok((output, warnings.into_iter().map(|w| w.message).collect()))
        }
        other => Err(refusal(other)),
    }
}

/// A result that did not complete, in its own words.
fn refusal<T>(result: CommandResult<T>) -> String {
    match result {
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => error.message,
        CommandResult::Cancelled => "Vazgeçildi.".to_owned(),
        CommandResult::Queued { .. } | CommandResult::Completed { .. } => String::new(),
    }
}
