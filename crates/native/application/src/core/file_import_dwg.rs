//! DWG import delegates to the same unit/routing/undo/journal policy as DXF (ADR 0586).
use crate::{
    ExecutionContext,
    file_import::{self, CadFormat},
};
use kentos_commands::{CommandResult, DwgImported, ImportDwg};

pub fn validate(cx: &ExecutionContext<'_>, input: &ImportDwg) -> CommandResult<()> {
    file_import::validate_format(cx, input, CadFormat::Dwg)
}
pub fn plan(cx: &ExecutionContext<'_>, input: &ImportDwg) -> CommandResult<DwgImported> {
    file_import::plan_format(cx, input, CadFormat::Dwg)
}
pub fn execute(cx: &mut ExecutionContext<'_>, input: ImportDwg) -> CommandResult<DwgImported> {
    file_import::execute_format(cx, input, CadFormat::Dwg)
}
