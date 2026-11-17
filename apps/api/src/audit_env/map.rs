//! The interpreter map the worker's audit source reads (docs/adr/8057 §66): the file the server writes when it prepares
//! an environment, over the setting's mapping; written whole, read again at every job.

use std::path::Path;

use kentos_application::audit_catalog::AuditPythons;

use super::MAP;

/// The interpreter map the audit source reads: the file the server writes, over the setting's mapping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditMap {
    pub(super) file: std::path::PathBuf,
    pub(super) setting: AuditPythons,
}

impl AuditMap {
    /// The file's mapping; empty when there is no file yet.
    pub fn file_entries(&self) -> Result<AuditPythons, String> {
        match std::fs::read_to_string(&self.file) {
            Ok(text) => {
                AuditPythons::parse(&text).map_err(|e| format!("{}: {e}", self.file.display()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AuditPythons::default()),
            Err(e) => Err(format!("{} okunamadı: {e}", self.file.display())),
        }
    }

    /// The setting's mapping with the file's over it.
    pub fn current(&self) -> Result<AuditPythons, String> {
        Ok(self.setting.clone().overlaid(&self.file_entries()?))
    }

    /// The interpreter prepared for `extension` at `version`: read again at every call, so a version prepared while the
    /// server runs is found. A map that cannot be read is the setting's alone.
    pub fn get(&self, extension: &str, version: &str) -> Option<String> {
        self.current()
            .unwrap_or_else(|_| self.setting.clone())
            .get(extension, version)
    }

    /// Writes the file's mapping whole: a temporary file beside it, then a rename over it.
    fn write(&self, map: &AuditPythons) -> Result<(), String> {
        let dir = self.file.parent().unwrap_or(Path::new("."));
        let part = dir.join(format!("{MAP}.yazilan-{}", std::process::id()));
        std::fs::create_dir_all(dir)
            .and_then(|()| std::fs::write(&part, map.to_json()))
            .and_then(|()| std::fs::rename(&part, &self.file))
            .map_err(|e| {
                let _ = std::fs::remove_file(&part);
                format!("{} yazılamadı: {e}", self.file.display())
            })
    }

    /// `extension` at `version` mapped to `python` in the file.
    pub fn set(&self, extension: &str, version: &str, python: &Path) -> Result<(), String> {
        let mut map = self.file_entries()?;
        map.insert(extension, version, &python.display().to_string());
        self.write(&map)
    }

    /// `extension` at `version` taken out of the file; whether it was there.
    pub fn unset(&self, extension: &str, version: &str) -> Result<bool, String> {
        let mut map = self.file_entries()?;
        let was = map.remove(extension, version);
        if was {
            self.write(&map)?;
        }
        Ok(was)
    }
}
