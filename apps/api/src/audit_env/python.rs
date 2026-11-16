//! KentOS's Python an audit environment is made from, and what kept one from being made, as the administrator reads
//! it (docs/adr/8057 §66).

use std::path::Path;
use std::time::Duration;

use kentos_python_env::python::{Cause, InstallError, Interpreter};
use kentos_python_env::{compat, uv};

use super::EnvSettings;

/// How long KentOS's Python may take to say its version.
const ASK_LIMIT: Duration = Duration::from_secs(20);

/// KentOS's Python the environments are made from when no setting names one: the one uv installed in the server's
/// folder.
pub(super) fn installed(settings: &EnvSettings, uv_path: &Path) -> Result<Interpreter, String> {
    let range = &compat::compat().python.range;
    kentos_python_env::python::installed(uv_path, &settings.dir, range)?.ok_or_else(|| {
        format!(
            "Ortamların yapılacağı Python yok: KENTOS_AUDIT_BASE_PYTHON ile KentOS'un desteklediği ({range}) bir \
             CPython gösterin."
        )
    })
}

/// The setting's Python (`KENTOS_AUDIT_BASE_PYTHON`), asked its version and held to the range KentOS's library runs in.
pub(super) fn asked(path: &Path) -> Result<Interpreter, String> {
    let range = &compat::compat().python.range;
    let ran = uv::run(
        path,
        &[
            std::ffi::OsStr::new("-I"),
            std::ffi::OsStr::new("-c"),
            std::ffi::OsStr::new("import sys; print('%d.%d.%d' % sys.version_info[:3])"),
        ],
        None,
        &[],
        ASK_LIMIT,
    )
    .map_err(|e| {
        format!(
            "KENTOS_AUDIT_BASE_PYTHON ({}) çalıştırılamadı: {e}",
            path.display()
        )
    })?;
    let version = ran.stdout.trim().to_owned();
    if ran.status != Some(0) || compat::triple(&version).is_none() {
        return Err(format!(
            "KENTOS_AUDIT_BASE_PYTHON ({}) sürümünü söylemedi.",
            path.display()
        ));
    }
    if !in_range(&version, range) {
        return Err(format!(
            "KENTOS_AUDIT_BASE_PYTHON ({}) Python {version}; KentOS'un kitaplığı {range} ister.",
            path.display()
        ));
    }
    Ok(Interpreter {
        version,
        path: path.to_path_buf(),
    })
}

/// Whether `version` (`3.12.13`) is within `range` (`>=3.10,<3.15`): each clause an operator and a version.
pub fn in_range(version: &str, range: &str) -> bool {
    let Some(v) = compat::triple(version) else {
        return false;
    };
    let triple = |text: &str| {
        let mut parts = text.trim().split('.').map(|p| p.parse::<u64>());
        let major = parts.next()?.ok()?;
        let minor = parts.next().transpose().ok()?.unwrap_or(0);
        let patch = parts.next().transpose().ok()?.unwrap_or(0);
        Some((major, minor, patch))
    };
    range.split(',').all(|clause| {
        let clause = clause.trim();
        let (op, rest) = ["==", "!=", ">=", "<=", ">", "<"]
            .iter()
            .find_map(|op| clause.strip_prefix(op).map(|rest| (*op, rest)))
            .unwrap_or(("==", clause));
        triple(rest).is_some_and(|b| match op {
            ">=" => v >= b,
            "<=" => v <= b,
            ">" => v > b,
            "<" => v < b,
            "!=" => v != b,
            _ => v == b,
        })
    })
}

/// What kept an environment from being made, as the administrator reads it.
pub(super) fn words(error: &InstallError) -> String {
    match error {
        InstallError::Cancelled => "Ortamın hazırlanması durduruldu.".to_owned(),
        InstallError::TimedOut => {
            "Ortamın hazırlanması zamanında bitmedi ve durduruldu.".to_owned()
        }
        InstallError::NotRun(why) => format!("uv çalıştırılamadı: {why}."),
        InstallError::Failed { cause, said } => match cause {
            Cause::Network => format!(
                "Paket dizinine ulaşılamadı; bağlantıyı ya da vekil sunucu ayarını (HTTPS_PROXY) denetleyin. \
                 uv: “{said}”"
            ),
            Cause::Certificate => {
                format!("Paket dizininin sertifikası bu sunucuda doğrulanamadı. uv: “{said}”")
            }
            Cause::Disk => format!("Ortam hazırlanamadı: diskte yer yok. uv: “{said}”"),
            Cause::Folder => format!("Ortam klasörüne yazılamadı. ({said})"),
            Cause::Unavailable | Cause::Build | Cause::Other => said.clone(),
        },
    }
}
