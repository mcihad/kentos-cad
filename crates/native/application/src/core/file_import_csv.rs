//! `file.import.csv` v1 (docs/adr/0522, plan/gis.md CBS-05): a CSV's rows taken into the drawing as one undo step
//! “CSV: ad.csv”, each row an object (its WKT or X and Y) or a record, its other columns its attributes; read by
//! `csv_table::read` and taken in by the GeoPackage import's own path (`file_import_gpkg::run_read`): the same layers,
//! fields, lasting ids, system check and losses. The shared cases are `fixtures/commands/v1/file.import.csv.json`
//! (`scripts/fixtures/csv_exchange_cases.py`).

use std::collections::BTreeMap;
use std::path::Path;

use kentos_commands::{CommandResult, CommandWarning, GpkgImported, ImportCsv, ImportGpkg};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn run(doc: &mut Document, input: &ImportCsv, write: bool) -> Result<(GpkgImported, Vec<CommandWarning>), Stop> {
    let path = Path::new(&input.path);
    if input.path.trim().is_empty() || !path.is_absolute() || path.is_dir() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{}” bir dosyanın mutlak yolu değil; okunacak CSV'nin tam yolunu verin (/klasör/ad.csv).",
                input.path
            ),
            "path",
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| {
        refuse(
            codes::FILE_NOT_FOUND,
            format!("{} okunamadı: {e}. Dosyanın yolunu denetleyin.", input.path),
            "path",
        )
    })?;
    let read = crate::csv_table::read(path, &bytes, input).map_err(|why| {
        refuse(
            codes::IMPORT_UNREADABLE,
            format!("{} CSV olarak okunamadı: {why}", input.path),
            "path",
        )
    })?;
    let mut layers = BTreeMap::new();
    if let (Some(layer), Some(t)) = (&input.layer, read.tables.first()) {
        layers.insert(t.name.clone(), layer.clone());
    }
    let as_gpkg = ImportGpkg {
        path: input.path.clone(),
        tables: Vec::new(),
        layers,
        reproject: input.reproject,
        merge: input.merge.clone(),
        rejects: input.rejects.clone(),
    };
    let ext = path
        .extension()
        .map_or_else(String::new, |e| format!(".{}", e.to_string_lossy()));
    crate::file_import_gpkg::run_read(doc, &as_gpkg, read, false, ("CSV", &ext), write)
}

/// Checks `input`, reading the file.
pub fn validate(cx: &ExecutionContext<'_>, input: &ImportCsv) -> CommandResult<()> {
    let mut copy = cx.doc.clone();
    match run(&mut copy, input, false) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would take; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ImportCsv) -> CommandResult<GpkgImported> {
    let mut copy = cx.doc.clone();
    match run(&mut copy, input, false) {
        Ok((mut output, warnings)) => {
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output, warnings }
        }
        Err(stop) => stop.into(),
    }
}

/// Takes the CSV's rows into the drawing, one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ImportCsv) -> CommandResult<GpkgImported> {
    // One step of the drawing and its own row in the journal (docs/adr/0253 §3, 0548 §4).
    crate::call::recorded(
        cx,
        kentos_commands::FILE_IMPORT_CSV,
        kentos_commands::FILE_IMPORT_CSV_VERSION,
        input,
        written_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step and journals it.
fn written_execute(cx: &mut ExecutionContext<'_>, input: ImportCsv) -> CommandResult<GpkgImported> {
    match run(cx.doc, &input, true) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}
