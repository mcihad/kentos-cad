//! What İşlemler remembers between runs of the program (the web's
//! `kentos.processing.v1`): each tool's and model's last values, where
//! each tool is asked to run, and the user's models (the model designer's
//! Kaydet, in the web's JSON; docs/adr/0116). It is kept in `islemler.json` beside the
//! program's other history (`$XDG_STATE_HOME/kentos-cad`, as
//! `son-dosyalar.json`): app state, not a setting and not the drawing. The
//! file is written whole and renamed into place; an unreadable one is an
//! empty memory, never a reason to stop. Values that no longer fit a
//! tool's parameters are dropped when a window opens
//! (`parameters::restore_values`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kentos_processing::{Model, Values};
use serde::{Deserialize, Serialize};

const FORMAT: &str = "kentos.processing-memory";
const FILE: &str = "islemler.json";

#[derive(Default, Deserialize)]
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

/// The file as it is written: the user's models as the web writes them,
/// each step's values in their order (read back by `Models`).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Writing<'a> {
    format: &'a str,
    version: u32,
    last_values: &'a BTreeMap<String, Values>,
    targets: &'a BTreeMap<String, String>,
    models: &'a [Model],
}

/// The user's models read from the file's text: each step's values in the
/// order they are written (a serde_json value would sort them).
#[derive(Default, Deserialize)]
struct Models {
    #[serde(default)]
    models: Vec<Model>,
}

/// The memory and where it is kept; in memory only when there is no folder.
#[derive(Default)]
pub struct Memory {
    folder: Option<PathBuf>,
    last_values: BTreeMap<String, Values>,
    targets: BTreeMap<String, String>,
    models: Vec<Model>,
}

impl Memory {
    /// Only in memory (tests, and a program without a state folder).
    pub fn temporary() -> Self {
        Self::default()
    }

    /// The memory kept in `folder`.
    pub fn open(folder: &Path) -> Self {
        let text = std::fs::read_to_string(folder.join(FILE)).ok();
        let stored = text
            .as_deref()
            .and_then(|text| serde_json::from_str::<Stored>(text).ok())
            .filter(|s| s.format == FORMAT && s.version == 1);
        let models = match (&stored, text.as_deref()) {
            (Some(_), Some(text)) => serde_json::from_str::<Models>(text)
                .map(|m| m.models)
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let stored = stored.unwrap_or_default();
        Self {
            folder: Some(folder.to_path_buf()),
            last_values: stored.last_values,
            targets: stored.targets,
            models,
        }
    }

    /// The user's models, as kept.
    pub fn models(&self) -> &[Model] {
        &self.models
    }

    /// Keeps a user's model: a known one in its place, a new one after the others.
    pub fn save_model(&mut self, model: &Model) {
        match self.models.iter_mut().find(|m| m.id == model.id) {
            Some(slot) => *slot = model.clone(),
            None => self.models.push(model.clone()),
        }
        self.write();
    }

    /// Removes a user's model.
    pub fn remove_model(&mut self, id: &str) {
        self.models.retain(|m| m.id != id);
        self.write();
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
        let stored = Writing {
            format: FORMAT,
            version: 1,
            last_values: &self.last_values,
            targets: &self.targets,
            models: &self.models,
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

    #[test]
    fn a_saved_model_reads_back_the_same_with_its_values_in_order() {
        let dir = std::env::temp_dir().join(format!("kentos-modeller-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut memory = Memory::open(&dir);
        let mut model = kentos_processing::model_edit::new_model("m-deneme".into());
        let lookup = |id: &str| kentos_processing::Registry::builtin().tool(id);
        kentos_processing::model_edit::add_input(&mut model, "string", "Önek", None);
        let step = kentos_processing::model_edit::add_step(
            &mut model,
            "points.numberVertices",
            &lookup,
            None,
            None,
        );
        // Set out of the tool's order: read back as set.
        for param in ["prefix", "input"] {
            kentos_processing::model_edit::set_source(
                &mut model,
                &step,
                param,
                Some(kentos_processing::ValueSource::Value(json!("P"))),
            );
        }
        memory.save_model(&model);
        let again = Memory::open(&dir);
        let read = &again.models()[0];
        assert_eq!(read.to_json(), model.to_json());
        let order: Vec<&str> = read.steps[0]
            .values
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(order, ["prefix", "input"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
