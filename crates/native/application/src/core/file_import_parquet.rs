//! `file.import.parquet` v1 (docs/adr/0628, plan/gis.md CBS-05): a GeoParquet's rows taken into the drawing as one
//! undo step “GeoParquet: ad.parquet”, each an object (or a record), its columns its attributes; read by
//! `parquet_table::read` and taken in by the GeoPackage import's own path (`file_import_gpkg::run_read`): the same
//! layers, fields, lasting ids, system check, losses and merge. The shared cases are
//! `fixtures/commands/v1/file.import.parquet.json` (`scripts/fixtures/geoparquet_cases.py`).

use std::collections::BTreeMap;
use std::path::Path;

use kentos_commands::{CommandResult, CommandWarning, GpkgImported, ImportGpkg, ImportParquet};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn run(
    doc: &mut Document,
    input: &ImportParquet,
    write: bool,
) -> Result<(GpkgImported, Vec<CommandWarning>), Stop> {
    let path = Path::new(&input.path);
    if input.path.trim().is_empty() || !path.is_absolute() || path.is_dir() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{}” bir dosyanın mutlak yolu değil; okunacak GeoParquet'in tam yolunu verin (/klasör/ad.parquet).",
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
    let read = crate::parquet_table::read(path, bytes, input).map_err(|why| {
        refuse(
            codes::IMPORT_UNREADABLE,
            format!("{} GeoParquet olarak okunamadı: {why}", input.path),
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
    crate::file_import_gpkg::run_read(doc, &as_gpkg, read, false, ("GeoParquet", &ext), write)
}

/// Checks `input`, reading the file.
pub fn validate(cx: &ExecutionContext<'_>, input: &ImportParquet) -> CommandResult<()> {
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
pub fn plan(cx: &ExecutionContext<'_>, input: &ImportParquet) -> CommandResult<GpkgImported> {
    let mut copy = cx.doc.clone();
    match run(&mut copy, input, false) {
        Ok((mut output, warnings)) => {
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output, warnings }
        }
        Err(stop) => stop.into(),
    }
}

/// Takes the GeoParquet's rows into the drawing, one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ImportParquet) -> CommandResult<GpkgImported> {
    // One step of the drawing and its own row in the journal (docs/adr/0253 §3, 0548 §4).
    crate::call::recorded(
        cx,
        kentos_commands::FILE_IMPORT_PARQUET,
        kentos_commands::FILE_IMPORT_PARQUET_VERSION,
        input,
        written_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step and journals it.
fn written_execute(
    cx: &mut ExecutionContext<'_>,
    input: ImportParquet,
) -> CommandResult<GpkgImported> {
    match run(cx.doc, &input, true) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}
