//! The trace format (`kentos.interaction-trace` v1,
//! fixtures/interaction/README.md). Every field is named: a field the player
//! does not know stops it, so a new one cannot be skipped unnoticed.

use std::path::Path;

use serde::Deserialize;

use super::folder;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Trace {
    pub format: String,
    pub version: u32,
    pub id: String,
    /// What the trace shows; the runner's report names it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub title: String,
    /// For the reader: where the trace comes from and the TODOS items it covers.
    #[allow(dead_code)]
    pub source: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    pub covers: Vec<String>,
    pub document: String,
    pub view: View,
    #[serde(default)]
    pub draft: DraftSpec,
    #[serde(default)]
    pub prefs: Prefs,
    pub click_tolerance: f64,
    pub steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct View {
    pub center: [f64; 2],
    pub metres_per_pixel: f64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftSpec {
    #[serde(default)]
    pub(super) snap: bool,
    #[serde(default)]
    pub(super) grid: bool,
    #[serde(default)]
    pub(super) ortho: bool,
    #[serde(default)]
    pub(super) polar: bool,
    #[serde(default)]
    pub(super) tracking: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Prefs {
    pub(super) cursor_input: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Step {
    pub(super) run: Option<String>,
    pub(super) key: Option<String>,
    pub(super) text: Option<String>,
    #[serde(rename = "move")]
    pub(super) move_to: Option<[f64; 2]>,
    pub(super) click: Option<[f64; 2]>,
    /// The left button down at the first point, moved to the second, released there.
    pub(super) drag: Option<[[f64; 2]; 2]>,
    pub(super) double_click: Option<[f64; 2]>,
    pub(super) right_click: Option<[f64; 2]>,
    pub(super) focus: Option<String>,
    pub(super) save_and_reopen: Option<bool>,
    /// Shift held during the step's click or drag.
    pub(super) shift: Option<bool>,
    pub(super) expect: Option<Expect>,
    /// For the reader; not checked.
    #[allow(dead_code)]
    pub(super) note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Expect {
    pub(super) tool: Option<String>,
    pub(super) points: Option<usize>,
    pub(super) options: Option<Vec<String>>,
    /// `null` (closed) and absent (not compared) differ.
    #[serde(default, deserialize_with = "present")]
    pub(super) dynamic_input: Option<Option<String>>,
    pub(super) command_line: Option<String>,
    pub(super) entities: Option<usize>,
    pub(super) newest: Option<Newest>,
    pub(super) can_undo: Option<bool>,
    pub(super) can_redo: Option<bool>,
    pub(super) dirty: Option<bool>,
    pub(super) log: Option<String>,
    pub(super) metres_per_pixel: Option<f64>,
    /// The selected objects' ids, in the order they were selected.
    pub(super) selected: Option<Vec<u32>>,
    /// The hovered object's id; `null` (none) and absent differ.
    #[serde(default, deserialize_with = "present")]
    pub(super) hover: Option<Option<u32>>,
    /// The object snap's kind the marker shows (`endpoint` …); `null` (none) and absent differ.
    #[serde(default, deserialize_with = "present")]
    pub(super) snap: Option<Option<String>>,
    /// Every object's id, in the drawing's order.
    pub(super) ids: Option<Vec<u32>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Newest {
    pub(super) kind: String,
    pub(super) points: Option<Vec<[f64; 2]>>,
    pub(super) edges: Option<Vec<[f64; 2]>>,
    pub(super) arcs: Option<usize>,
}

/// A field that is there, even as `null`.
fn present<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(d).map(Some)
}

impl Trace {
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("{} okunamadı: {e}", path.display()))?;
        let trace: Trace =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if trace.format != "kentos.interaction-trace" || trace.version != 1 {
            return Err(format!("{}: v1 etkileşim izi değil", path.display()));
        }
        Ok(trace)
    }

    /// Every trace of the folder, by file name.
    #[cfg(test)]
    pub fn all() -> Result<Vec<Self>, String> {
        let dir = folder();
        let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
            .map_err(|e| format!("{} okunamadı: {e}", dir.display()))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect();
        paths.sort();
        paths.iter().map(|p| Self::read(p)).collect()
    }

    pub fn by_id(id: &str) -> Result<Self, String> {
        Self::read(&folder().join(format!("{id}.json")))
    }
}
