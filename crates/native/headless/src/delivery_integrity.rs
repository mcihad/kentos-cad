//! A replayed publication means its complete recorded delivery still exists, with no path/symlink escape.
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, Component},
};
use kentos_commands::{
    SheetsPublished, CommandError, CommandResult, CallAnswer,
    encoding::{sha256_hex, sha256_reader_hex},
};
pub(crate) fn portable_relative(path: &str) -> bool {
    !path.is_empty()
        && !Path::new(path).is_absolute()
        && path.split('/').all(|part| {
            if part.is_empty()
                || part == "."
                || part == ".."
                || part.len() > 240
                || part.trim() != part
                || part.ends_with('.')
                || part
                    .chars()
                    .any(|c| c.is_control() || "\\:*?\"<>|".contains(c))
            {
                return false;
            }
            let base = part.split('.').next().unwrap_or("").to_ascii_uppercase();
            !matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                && !(1..=9).any(|i| base == format!("COM{i}") || base == format!("LPT{i}"))
        })
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
pub(crate) fn file(root: &Path, relative: &str, bytes: u64, sha: &str) -> Result<(), String> {
    if !portable_relative(relative) {
        return Err(format!("Geçersiz teslim yolu: {relative}"));
    }
    if fs::symlink_metadata(root)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Teslim klasörü sembolik bağla değiştirilmiş.".into());
    }
    let canonical = root.canonicalize().map_err(|e| e.to_string())?;
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
        if fs::symlink_metadata(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .file_type()
            .is_symlink()
        {
            return Err(format!("Teslim yolu sembolik bağ içeriyor: {relative}"));
        }
    }
    if !path
        .canonicalize()
        .map_err(|e| e.to_string())?
        .starts_with(&canonical)
        || !path.is_file()
    {
        return Err(format!(
            "Teslim dosyası klasörün dışında veya normal dosya değil: {relative}"
        ));
    }
    let (got, hash) = fs::File::open(&path)
        .and_then(sha256_reader_hex)
        .map_err(|e| format!("{relative}: {e}"))?;
    if got != bytes || hash != sha {
        return Err(format!(
            "Teslim dosyası değişmiş: {relative}; eski teslimin tamamı korunmuyor."
        ));
    }
    Ok(())
}
pub(crate) fn verify(report: &SheetsPublished) -> Result<(), String> {
    let root = Path::new(&report.path);
    let path = root.join("yayin.json");
    let size = fs::symlink_metadata(&path)
        .map_err(|e| format!("Teslim manifesti okunamadı: {e}"))?
        .len();
    file(root, "yayin.json", size, &report.sha256)?;
    let bytes = fs::read(&path).map_err(|e| format!("Teslim manifesti okunamadı: {e}"))?;
    let manifest: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if manifest["format"] != "kentos.publish/1" || sha256_hex(&bytes) != report.sha256 {
        return Err("Teslim manifesti değişmiş.".into());
    }
    let mut names = BTreeSet::new();
    for f in &report.files {
        if !names.insert(f.name.to_lowercase()) {
            return Err("Teslim dosya adları çakışıyor.".into());
        }
        file(root, &f.name, f.bytes, &f.sha256)?;
    }
    if let Some(resources) = &report.resources {
        for f in &resources.files {
            if !names.insert(f.path.to_lowercase()) {
                return Err("Teslim kaynak adları çakışıyor.".into());
            }
            file(root, &f.path, f.bytes, &f.sha256)?;
        }
    }
    Ok(())
}
pub(crate) fn checked_replay(command: &str, mut answer: CallAnswer) -> CallAnswer {
    if command == kentos_commands::FILE_PUBLISH_SHEETS
        && answer.replayed
        && !answer.undone
        && let CommandResult::Completed { output, .. } = &answer.result
    {
        let checked = serde_json::from_value::<SheetsPublished>(output.clone())
            .map_err(|e| e.to_string())
            .and_then(|r| verify(&r));
        if let Err(message) = checked {
            answer.result = CommandResult::Failed {
                error: CommandError {
                    code: "integrity_failed".into(),
                    message,
                    path: Some("path".into()),
                    ..Default::default()
                },
            };
        }
    }
    answer
}

/// Recipe replay verifies an existing artifact separately from recomputing a past rendering context.
pub(crate) fn replay_row(row: &kentos_commands::CallRecord) -> kentos_commands::ReplayStep {
    use kentos_commands::{ReplayStep, ReplayOutcome};
    let checked = (|| -> Result<(), String> {
        let effect = row
            .effects
            .first()
            .ok_or("Teslim manifesti etkisi kayıtta yok.")?;
        let manifest = Path::new(&effect.path);
        let root = manifest.parent().ok_or("Teslim klasörü yok.")?;
        file(root, "yayin.json", effect.bytes, &effect.sha256)?;
        let v: serde_json::Value =
            serde_json::from_slice(&fs::read(manifest).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let report = SheetsPublished {
            path: root.display().to_string(),
            manifest: effect.path.clone(),
            sha256: effect.sha256.clone(),
            sheets: serde_json::from_value(v["sheets"].clone()).map_err(|e| e.to_string())?,
            files: serde_json::from_value(v["files"].clone()).map_err(|e| e.to_string())?,
            resources: serde_json::from_value(v["resources"].clone()).map_err(|e| e.to_string())?,
            source: serde_json::from_value(v["source"].clone()).map_err(|e| e.to_string())?,
            issues: Vec::new(),
            ready: true,
        };
        verify(&report)
    })();
    let (note,error)=match checked{
        Ok(())=>("Mevcut teslimin bütün dosya hash'leri doğrulandı; eski render bağlamı yeniden üretilmedi, dosya yazılmadı.".into(),None),
        Err(message)=>(format!("Teslim bütünlüğü doğrulanamadı: {message}"),Some(CommandError{code:"integrity_failed".into(),message,path:Some("path".into()),..Default::default()})),
    };
    ReplayStep {
        seq: row.seq,
        command: row.command.clone(),
        outcome: ReplayOutcome::Skipped,
        note: Some(note),
        error,
        created: Vec::new(),
    }
}
