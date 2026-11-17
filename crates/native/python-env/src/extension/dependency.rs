//! A package of the personal environment added to an extension's own Python project (plan/eklenti.md ADIM 18A, the last
//! step of PY-OPT-06's example: "betik/eklenti bağımlılığına ekle"; docs/adr/8057 §95): the version the user tried in
//! the console written into the project's dependencies and locked, as `uv add` does it.
//!
//! - **The extension's own settings hold:** uv runs without `--no-config`, so the project's `[tool.uv]` (its cooling
//!   period, its indexes, its environments) shapes the lock. Measured with uv 0.11.26: `--no-config` keeps
//!   `environments` but drops `exclude-newer`, and an agent's lock without its cooling period is a warning (§83).
//! - **Beside, then over:** uv works on a copy of the project's files in KentOS's folder; only a lock that resolved
//!   replaces the project's `pyproject.toml` and `uv.lock`, so a failed resolution leaves the folder as it was.
//! - **Nothing installed:** `--no-sync`; the extension's environment is made from the new lock as any other (§5).

use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use super::refs;
use crate::python::{self, Cause, Interpreter};
use crate::uv;

/// Where uv works on the copy, in KentOS's Python folder.
const STAGING: &str = "eklenti-bagimliligi";

/// How long resolving may take: the index is asked.
const LIMIT: Duration = Duration::from_secs(10 * 60);

/// The project's files uv reads and writes; `uv.toml` beside them is the project's settings too.
const FILES: [&str; 3] = ["pyproject.toml", "uv.lock", "uv.toml"];

/// The requirement an extension writes for `version` of `name`: that version and the later ones of its series
/// (`numpy>=2.5.4,<3`; a 0.x series is its minor's: `fiona>=0.13.1,<0.14`); a version that does not read, from it on.
pub fn requirement_for(name: &str, version: &str) -> String {
    let parts: Vec<Option<u64>> = version.split('.').map(|p| p.parse().ok()).collect();
    let upper = match parts.as_slice() {
        [Some(0), Some(minor), ..] => Some(format!("0.{}", minor + 1)),
        [Some(major), ..] if *major > 0 => Some((major + 1).to_string()),
        _ => None,
    };
    match upper {
        Some(upper) => format!("{name}>={version},<{upper}"),
        None => format!("{name}>={version}"),
    }
}

/// Adds `requirement` to the Python project in `project` (the folder of its `pyproject.toml` and `uv.lock`) and locks
/// it with uv, the project's own settings holding; its files are replaced only when the lock resolved. Why not, in
/// Turkish.
pub fn add_dependency(
    uv_path: &Path,
    root: &Path,
    interpreter: &Interpreter,
    project: &Path,
    requirement: &str,
) -> Result<(), String> {
    if !project.join("pyproject.toml").is_file() {
        return Err(format!("{} içinde pyproject.toml yok.", project.display()));
    }
    let aside = root.join(STAGING);
    let _ = std::fs::remove_dir_all(&aside);
    std::fs::create_dir_all(&aside).map_err(|e| format!("{} açılamadı: {e}", aside.display()))?;
    for file in FILES {
        let from = project.join(file);
        if from.is_file() {
            std::fs::copy(&from, aside.join(file))
                .map_err(|e| format!("{} kopyalanamadı: {e}", from.display()))?;
        }
    }
    let mut args: Vec<OsString> = [
        "add",
        "--no-sync",
        "--no-python-downloads",
        "--system-certs",
        "--python",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    args.push(interpreter.path.clone().into_os_string());
    args.push(OsString::from(requirement));
    let env = python::uv_env(root);
    let ran = uv::run(
        uv_path,
        &uv::os_args(&args),
        Some(&aside),
        &refs(&env),
        LIMIT,
    )?;
    if ran.timed_out {
        let _ = std::fs::remove_dir_all(&aside);
        return Err("uv on dakikada yanıt vermedi; eklentinin dosyaları değişmedi.".to_owned());
    }
    if ran.status != Some(0) {
        let _ = std::fs::remove_dir_all(&aside);
        return Err(add_words(&ran.stderr));
    }
    for file in ["pyproject.toml", "uv.lock"] {
        replace(&aside.join(file), &project.join(file))?;
    }
    let _ = std::fs::remove_dir_all(&aside);
    Ok(())
}

/// `to` replaced by `from`'s bytes through a file beside it, renamed over: a half-written project file never stays.
fn replace(from: &Path, to: &Path) -> Result<(), String> {
    let bytes = std::fs::read(from).map_err(|e| format!("{} okunamadı: {e}", from.display()))?;
    let next = to.with_extension("kentos-yeni");
    std::fs::write(&next, bytes).map_err(|e| format!("{} yazılamadı: {e}", next.display()))?;
    std::fs::rename(&next, to).map_err(|e| {
        let _ = std::fs::remove_file(&next);
        format!("{} yazılamadı: {e}", to.display())
    })
}

/// Why uv could not add the package, and that the extension's files did not change.
pub fn add_words(stderr: &str) -> String {
    let said = python::said(stderr).unwrap_or_else(|| stderr.trim().chars().take(300).collect());
    let lower = stderr.to_lowercase();
    let sentence = if lower.contains("not found in the package registry")
        || lower.contains("no versions of")
    {
        "Paket dizininde bulunamadı."
    } else if lower.contains("no solution found") || lower.contains("unsatisfiable") {
        "Bu sürüm eklentinin Python aralığı ve sistemleriyle (requires-python, [tool.uv] environments) çözülemedi."
    } else {
        match python::cause_of(stderr) {
            Cause::Network => {
                "Paket dizinine ulaşılamadı; bağlantıyı ya da vekil sunucu ayarını (HTTPS_PROXY) denetleyin."
            }
            Cause::Certificate => {
                "Paket dizininin sertifikası bu bilgisayarda doğrulanamadı; kurumun sertifikası yüklü olmalı."
            }
            _ => "Eklentinin kilidi yenilenemedi.",
        }
    };
    format!("{sentence} Eklentinin dosyaları değişmedi. uv: “{said}”")
}

#[cfg(test)]
mod tests;
