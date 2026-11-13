//! Where the registry and the organisation's policy are on this computer (docs/adr/8057 §2).
//!
//! The user's extension folder is the program's data folder's `eklentiler` (`$XDG_DATA_HOME/kentos-cad/eklentiler`,
//! else `~/.local/share/kentos-cad/eklentiler`; `%APPDATA%\kentos-cad\eklentiler` on Windows; `~/Library/Application
//! Support/kentos-cad/eklentiler` on macOS). The policy is IT's file in a system folder a user cannot write
//! (`/etc/kentos-cad`, `%ProgramData%\kentos-cad`, `/Library/Application Support/kentos-cad`). `KENTOS_EXTENSIONS_DIR`
//! and `KENTOS_EXTENSIONS_POLICY` point elsewhere: tests and real-window sessions never touch the user's own.

use std::path::{Path, PathBuf};

use crate::policy::Policy;

/// The policy file's name.
pub const POLICY_FILE: &str = "eklenti-ilkesi.json";

fn env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// The user's extension folder; none when this computer gives no data folder.
pub fn user_root() -> Option<PathBuf> {
    if let Some(dir) = env("KENTOS_EXTENSIONS_DIR") {
        return Some(dir);
    }
    let base = if cfg!(target_os = "windows") {
        env("APPDATA")
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library").join("Application Support"))
    } else {
        env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local").join("share")))
    };
    base.map(|b| b.join("kentos-cad").join("eklentiler"))
}

/// The organisation's policy file.
pub fn policy_path() -> PathBuf {
    if let Some(file) = env("KENTOS_EXTENSIONS_POLICY") {
        return file;
    }
    let folder = if cfg!(target_os = "windows") {
        env("ProgramData")
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
            .join("kentos-cad")
    } else if cfg!(target_os = "macos") {
        PathBuf::from("/Library/Application Support/kentos-cad")
    } else {
        PathBuf::from("/etc/kentos-cad")
    };
    folder.join(POLICY_FILE)
}

/// The policy in `file`: none when there is no such file, a policy refusing everything when it cannot be read.
pub fn load_policy(file: &Path) -> Option<Policy> {
    match std::fs::read_to_string(file) {
        Ok(text) => Some(Policy::read(&text).unwrap_or_else(|p| Policy::broken(p.message))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => Some(Policy::broken(format!(
            "Kurum eklenti ilkesi ({}) okunamadı: {e}.",
            file.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_policy_is_none_and_a_broken_one_refuses_everything() {
        let dir = std::env::temp_dir().join(format!("kentos-ilke-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(load_policy(&dir.join("yok.json")), None);
        let broken = dir.join(POLICY_FILE);
        std::fs::write(&broken, "{ bozuk").unwrap();
        let policy = load_policy(&broken).unwrap();
        let manifest = kentos_extensions::Manifest::read(include_str!(
            "../../../../fixtures/extensions/v1/ornek-altlik/manifest.json"
        ))
        .unwrap();
        let refusal = policy
            .refusal(&manifest, crate::store::SourceKind::Organization)
            .unwrap();
        assert!(
            refusal.contains("okunamadı") && refusal.contains("Bilgi işlem"),
            "{refusal}"
        );
        std::fs::write(
            &broken,
            r#"{"format": "kentos.extension-policy", "schemaVersion": 1}"#,
        )
        .unwrap();
        assert_eq!(
            load_policy(&broken)
                .unwrap()
                .refusal(&manifest, crate::store::SourceKind::File),
            None
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
