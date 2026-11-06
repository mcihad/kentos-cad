//! `file.recover` v1 (Kurtar, docs/adr/1013): a damaged `.kcad` recovered by `kentos_kcad::salvage` and written as a
//! separate copy (by default `<name>-kurtarilan.kcad` beside it). The source is only read; the copy is a new file,
//! written through the save's own path (temporary file, read back, renamed) and never over an existing one. A plan
//! says what the copy would hold and writes nothing; a file that reads whole is said whole and nothing is written.

use std::path::{Path, PathBuf};

use kentos_commands::{CallOp, CommandResult, KcadRecovered, RecoverKcad, RecoveredLoss, codes};
use serde_json::Value;

use crate::file_commands::{Refused, refuse};

fn kcad_path(text: &str, field: &str) -> Result<PathBuf, Refused> {
    let path = PathBuf::from(text);
    let is_kcad = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("kcad"));
    if !path.is_absolute() || !is_kcad {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("“{text}” mutlak bir .kcad dosya yolu değil; dosyanın tam yolunu verin."),
            field,
        ));
    }
    Ok(path)
}

/// The copy's default place: `<name>-kurtarilan.kcad` beside the source.
pub fn default_target(source: &Path) -> PathBuf {
    let stem = source.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "cizim".into());
    source.with_file_name(format!("{stem}-kurtarilan.kcad"))
}

fn planned(input: &RecoverKcad) -> Result<(PathBuf, kentos_kcad::salvage::Salvaged), Refused> {
    let source = kcad_path(&input.path, "path")?;
    let target = match &input.target {
        Some(t) => kcad_path(t, "target")?,
        None => default_target(&source),
    };
    if target == source {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            "Kurtarılan kopya kaynağın üzerine yazılmaz; başka bir dosya adı verin.",
            "target",
        ));
    }
    let bytes = std::fs::read(&source).map_err(|_| {
        refuse(codes::FILE_NOT_FOUND, format!("“{}” okunamadı: dosya yok ya da açılamıyor.", source.display()), "path")
    })?;
    if target.exists() {
        return Err(refuse(
            codes::FILE_EXISTS,
            format!("“{}” zaten var; kurtarılan kopya var olan bir dosyanın üzerine yazılmaz. Başka bir ad verin.", target.display()),
            "target",
        ));
    }
    let salvaged = kentos_kcad::salvage::salvage(&bytes)
        .map_err(|e| refuse(codes::KCAD_UNRECOVERABLE, e.message, "path"))?;
    Ok((target, salvaged))
}

fn output(target: &Path, s: &kentos_kcad::salvage::Salvaged, written: bool) -> KcadRecovered {
    KcadRecovered {
        target: target.display().to_string(),
        whole: s.whole,
        written,
        container: s.container.clone(),
        objects_read: s.objects_read as u32,
        objects_kept: s.objects_kept as u32,
        lost: s
            .lost
            .iter()
            .map(|l| RecoveredLoss { part: l.part.clone(), uid: l.uid.clone(), reason: l.reason.clone() })
            .collect(),
    }
}

pub fn run(input: &RecoverKcad, op: CallOp) -> CommandResult<Value> {
    let (target, salvaged) = match planned(input) {
        Ok(p) => p,
        Err(error) => return CommandResult::Failed { error: *error },
    };
    let mut written = false;
    match op {
        CallOp::Validate => return CommandResult::Completed { output: Value::Null, warnings: Vec::new() },
        CallOp::Execute if !salvaged.whole => {
            let bytes = match kentos_kcad::encode_verified(&salvaged.document) {
                Ok(b) => b,
                Err(e) => {
                    return CommandResult::Failed {
                        error: *refuse(codes::FILE_UNWRITABLE, format!("Kurtarılan çizim yazılamadı: {}", e.message), "target"),
                    };
                }
            };
            if let Err(e) = crate::files::write_bytes_with_overwrite(&bytes, &target, false) {
                return CommandResult::Failed { error: *refuse(e.code, e.message, "target") };
            }
            written = true;
        }
        _ => {}
    }
    CommandResult::Completed {
        output: serde_json::to_value(output(&target, &salvaged, written)).unwrap_or(Value::Null),
        warnings: Vec::new(),
    }
}
