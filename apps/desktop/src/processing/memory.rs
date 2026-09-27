//! What İşlemler remembers between runs of the program (the web's
//! `kentos.processing.v1`): each tool's and model's last values, and where
//! each tool is asked to run. It is kept in `islemler.json` beside the
//! program's other history (`$XDG_STATE_HOME/kentos-cad`, as
//! `son-dosyalar.json`): app state, not a setting and not the drawing. The
//! file is written whole and renamed into place; an unreadable one is an
//! empty memory, never a reason to stop. Values that no longer fit a
//! tool's parameters are dropped when a window opens
//! (`parameters::restore_values`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kentos_processing::Values;
use serde::{Deserialize, Serialize};

const FORMAT: &str = "kentos.processing-memory";
const FILE: &str = "islemler.json";

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stored {
    format: String,
    version: u32,
    #[serde(default)]
    last_values: BTreeMap<String, Values>,
    /// Where each tool is asked to run ("auto", "client" …), as the web keeps it.
    #[serde(default)]
    targets: BTreeMap<String, String>,
}

/// The memory and where it is kept; in memory only when there is no folder.
#[derive(Debug, Default)]
pub struct Memory {
    folder: Option<PathBuf>,
    last_values: BTreeMap<String, Values>,
    targets: BTreeMap<String, String>,
}

impl Memory {
    /// Only in memory (tests, and a program without a state folder).
    pub fn temporary() -> Self {
        Self::default()
    }

    /// The memory kept in `folder`.
    pub fn open(folder: &Path) -> Self {
        let stored = std::fs::read_to_string(folder.join(FILE))
            .ok()
            .and_then(|text| serde_json::from_str::<Stored>(&text).ok())
            .filter(|s| s.format == FORMAT && s.version == 1)
            .unwrap_or_default();
        Self {
            folder: Some(folder.to_path_buf()),
            last_values: stored.last_values,
            targets: stored.targets,
        }
    }

    /// The values of the tool's last run, if any.
    pub fn last_values(&self, tool: &str) -> Option<&Values> {
        self.last_values.get(tool)
    }

    /// Keeps the values a tool ran with.
    pub fn remember(&mut self, tool: &str, values: &Values) {
        self.last_values.insert(tool.to_owned(), values.clone());
        self.write();
    }

    /// Keeps where a tool is asked to run.
    pub fn set_target(&mut self, tool: &str, choice: &str) {
        if self.targets.get(tool).map(String::as_str) != Some(choice) {
            self.targets.insert(tool.to_owned(), choice.to_owned());
            self.write();
        }
    }

    fn write(&self) {
        let Some(folder) = &self.folder else { return };
        let stored = Stored {
            format: FORMAT.to_owned(),
            version: 1,
            last_values: self.last_values.clone(),
            targets: self.targets.clone(),
        };
        let Ok(text) = serde_json::to_string_pretty(&stored) else {
            return;
        };
        if std::fs::create_dir_all(folder).is_err() {
            return;
        }
        let temp = folder.join(format!("{FILE}.yaziliyor"));
        if std::fs::write(&temp, text).is_ok() {
            let _ = std::fs::rename(&temp, folder.join(FILE));
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn last_values_outlive_the_program_and_a_broken_file_is_empty() {
        let dir = std::env::temp_dir().join(format!("kentos-islemler-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut memory = Memory::open(&dir);
        assert!(memory.last_values("points.numberVertices").is_none());
        let values: Values = [("prefix".to_owned(), json!("K"))].into_iter().collect();
        memory.remember("points.numberVertices", &values);
        assert_eq!(
            Memory::open(&dir).last_values("points.numberVertices"),
            Some(&values)
        );
        std::fs::write(dir.join(FILE), "{ bozuk").expect("writes");
        assert!(
            Memory::open(&dir)
                .last_values("points.numberVertices")
                .is_none()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
