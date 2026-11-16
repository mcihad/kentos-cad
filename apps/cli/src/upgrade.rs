//! `kentos kek upgrade` (docs/adr/8061 §8): the SDK's deprecated names with a plain rename rewritten in a working
//! folder's Python, in the code (a docstring and a comment left as written); what has no mechanical rewrite stays a
//! `verify` warning that says what to do. `--check` writes nothing and exits 1 when something would change, for an
//! author's own checks.

use std::path::Path;
use std::process::ExitCode;

/// Rewrites the folder's deprecated names, or with `check` only says them.
pub(super) fn run(folder: &Path, check: bool, json: bool) -> ExitCode {
    let (files, _) = match super::read_folder(folder) {
        Ok(read) => read,
        Err(message) => return super::fail(message),
    };
    let (changed, rewrites) = kentos_extensions::sdk_surface::upgrade(&files);
    if json {
        let list: Vec<serde_json::Value> = rewrites
            .iter()
            .map(|r| serde_json::json!({ "file": r.file, "line": r.line, "from": r.from, "to": r.to }))
            .collect();
        let out = serde_json::json!({ "check": check, "files": changed.keys().collect::<Vec<_>>(), "rewrites": list });
        say!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else if rewrites.is_empty() {
        say!(
            "Yükseltilecek bir şey yok: klasörün Python'u bu KentOS'un eskittiği bir adı kullanmıyor."
        );
    } else {
        for r in &rewrites {
            say!("{} satır {}: {} → {}", r.file, r.line, r.from, r.to);
        }
    }
    if check {
        return if rewrites.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }
    for (path, text) in &changed {
        let target = folder.join(path);
        if let Err(e) = std::fs::write(&target, text) {
            return super::fail(format!("{} yazılamadı: {e}", target.display()));
        }
    }
    if !changed.is_empty() && !json {
        say!(
            "{} dosya yazıldı; kentos kek verify {} ile doğrulayın.",
            changed.len(),
            folder.display()
        );
    }
    ExitCode::SUCCESS
}
