//! `file.import.shp` v1 (docs/adr/0563 §1): a Shapefile taken into the drawing as one undo step “Shapefile: ad”, by the
//! rules of [`crate::file_import_gis`] (the file's system as a question with an answer, the layers' routing, the
//! losses said). The shared cases are `fixtures/commands/v1/file.import.shp.json`
//! (`scripts/fixtures/gis_import_cases.py`).

use kentos_commands::{CommandResult, GisImported, ImportShp};

use crate::ExecutionContext;
use crate::file_import_gis::{self as gis, Format, Opts, ReadGis};

/// Checks `input`, reading the file.
pub fn validate(cx: &ExecutionContext<'_>, input: &ImportShp) -> CommandResult<()> {
    gis::validate(cx, Format::Shp, &Opts::from(input.clone()))
}

/// What execute would take in, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ImportShp) -> CommandResult<GisImported> {
    gis::plan(cx, Format::Shp, &Opts::from(input.clone()))
}

/// Reads the file and takes it in, one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ImportShp) -> CommandResult<GisImported> {
    gis::execute(cx, Format::Shp, input)
}

/// [`execute`] with the file its host already read (the desktop's window).
pub fn execute_read(
    cx: &mut ExecutionContext<'_>,
    input: ImportShp,
    read: ReadGis,
) -> CommandResult<GisImported> {
    gis::execute_read(cx, Format::Shp, input, read)
}
