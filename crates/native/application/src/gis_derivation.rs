//! `cad.gis.derivation` (docs/adr/0530 §4): whether a derived GIS dataset (`file.export.gis`) still matches the
//! drawing it was made from. Read only: it opens the GeoPackage, finds each row's source by its lasting id and
//! compares digests, then the row against what the derivation gives now (`gis_derive::check`). A stale or edited
//! derivative changes nothing in the drawing, and nothing of the GIS comes back to it.

use std::path::Path;

use kentos_commands::{CommandResult, CommandWarning, GisDerivation, GisDerived};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::gis_derive;

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn run(
    cx: &ExecutionContext<'_>,
    input: &GisDerivation,
) -> Result<(GisDerived, Vec<CommandWarning>), Stop> {
    let path = input.path.as_str();
    let p = Path::new(path);
    if path.trim().is_empty() || !p.is_absolute() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{path}” mutlak bir dosya yolu değil; GeoPackage dosyasının tam yolunu verin (/klasör/ad.gpkg)."
            ),
            "path",
        ));
    }
    if p.is_dir() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("{path} bir klasör; GeoPackage dosyasının adını da verin."),
            "path",
        ));
    }
    if !p.exists() {
        return Err(refuse(
            codes::FILE_NOT_FOUND,
            format!("{path} yok; dosyanın yolunu denetleyin."),
            "path",
        ));
    }
    let found = gis_derive::check(cx.doc, p, input.limit)
        .map_err(|(code, message)| refuse(code, message, "path"))?;
    let mut warnings = Vec::new();
    let behind = found.stale + found.source_gone + found.not_exported;
    if behind > 0 {
        warnings.push(CommandWarning {
            code: codes::DERIVATION_STALE.into(),
            message: format!(
                "Türev eski: {} satırın kaynağı dışa aktarmadan sonra değişti, {} satırın kaynağı silindi, {} nesnenin satırı yok. file.export.gis ile yeniden dışa aktarın.",
                found.stale, found.source_gone, found.not_exported
            ),
            path: Some("path".into()),
        });
    }
    if found.edited > 0 {
        warnings.push(CommandWarning {
            code: codes::DERIVATION_EDITED.into(),
            message: format!(
                "{} satır GIS'te değiştirilmiş. Kaynak olan çizimdeki nesneler değişmedi; türevin değişikliği çizime geri yazılmaz, kaynağı çizimde düzenleyin.",
                found.edited
            ),
            path: Some("path".into()),
        });
    }
    Ok((found, warnings))
}

fn completed(r: Result<(GisDerived, Vec<CommandWarning>), Stop>) -> CommandResult<GisDerived> {
    match r {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &GisDerivation) -> CommandResult<()> {
    match run(cx, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &GisDerivation) -> CommandResult<GisDerived> {
    completed(run(cx, input))
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: GisDerivation) -> CommandResult<GisDerived> {
    completed(run(cx, &input))
}
