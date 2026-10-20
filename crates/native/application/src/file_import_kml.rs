//! `file.import.kml` v1 (docs/adr/0563 §1): a KML taken into the drawing as one undo step “KML: ad”, by the
//! rules of [`crate::file_import_gis`] (the file's system as a question with an answer, the layers' routing, the
//! losses said). The shared cases are `fixtures/commands/v1/file.import.kml.json`
//! (`scripts/fixtures/gis_import_cases.py`).

use kentos_commands::{CommandResult, GisImported, ImportKml};

use crate::ExecutionContext;
use crate::file_import_gis::{self as gis, Format, Opts, ReadGis};

/// Checks `input`, reading the file.
pub fn validate(cx: &ExecutionContext<'_>, input: &ImportKml) -> CommandResult<()> {
    gis::validate(cx, Format::Kml, &Opts::from(input.clone()))
}

/// What execute would take in, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ImportKml) -> CommandResult<GisImported> {
    gis::plan(cx, Format::Kml, &Opts::from(input.clone()))
}

/// Reads the file and takes it in, one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ImportKml) -> CommandResult<GisImported> {
    gis::execute(cx, Format::Kml, input)
}

/// [`execute`] with the file its host already read (the desktop's window).
pub fn execute_read(
    cx: &mut ExecutionContext<'_>,
    input: ImportKml,
    read: ReadGis,
) -> CommandResult<GisImported> {
    gis::execute_read(cx, Format::Kml, input, read)
}
