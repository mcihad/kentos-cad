//! The user's choice (PY-OPT-01, PY-OPT-11): `<python>/ayarlar.json`. Off unless the user turns it on; while off nothing
//! looks for uv or Python and nothing is downloaded. A file that cannot be read is the default (off), never an error
//! that keeps the program from starting.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// The settings file's name in the Python folder.
pub const FILE: &str = "ayarlar.json";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PythonSettings {
    /// The user turned Python and Python extensions on.
    pub enabled: bool,
    /// uv chosen by hand: looked for there alone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uv_path: Option<String>,
}

impl PythonSettings {
    /// The settings in `root`; off when there is no file or it cannot be read.
    pub fn load(root: &Path) -> PythonSettings {
        std::fs::read_to_string(root.join(FILE))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Writes them whole: a temporary file, then a rename over the old one.
    pub fn save(&self, root: &Path) -> Result<(), String> {
        let file = root.join(FILE);
        let temp = root.join(format!("{FILE}.yazilan"));
        let text = serde_json::to_string_pretty(self).unwrap_or_default();
        std::fs::create_dir_all(root)
            .and_then(|()| std::fs::write(&temp, text))
            .and_then(|()| std::fs::rename(&temp, &file))
            .map_err(|e| format!("{} yazılamadı: {e}.", file.display()))
    }
}
