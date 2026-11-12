//! The windows' sizes kept between runs (docs/adr/0746): each dialog's size and whether it is full screen, by its key,
//! in `pencereler.json` beside the settings. Read at start into KentOS UI's `sizable::sizes`, written when one changes
//! (the next message after a resize, and as the window closes). A file that cannot be read or written is passed over:
//! the windows open at their own sizes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kentos_ui::widget::sizable::sizes::{self, Kept};
use serde_json::{Map, Value, json};

/// The file, beside `ayarlar.json`.
pub(crate) const FILE_NAME: &str = "pencereler.json";

/// Where the sizes are written; none: kept in memory only (tests).
#[derive(Debug, Default)]
pub(crate) struct Keeper {
    path: Option<PathBuf>,
}

impl Keeper {
    /// The sizes kept in `folder`, loaded into the dialogs.
    pub(crate) fn open(folder: &Path) -> Self {
        let path = folder.join(FILE_NAME);
        sizes::load(read(std::fs::read_to_string(&path).ok().as_deref()));
        Self { path: Some(path) }
    }

    /// Writes the sizes if one changed since the last write.
    pub(crate) fn write_changed(&self) {
        if sizes::take_changed()
            && let Some(path) = &self.path
        {
            let _ = write_whole(path, &text(&sizes::all()));
        }
    }
}

/// The sizes in `text` (`{"Formla düzenle": {"width": 900, "height": 640, "maximized": false}}`): a size a number
/// does not give is passed over.
pub(crate) fn read(text: Option<&str>) -> BTreeMap<String, Kept> {
    let Some(Value::Object(map)) = text.and_then(|t| serde_json::from_str::<Value>(t).ok()) else {
        return BTreeMap::new();
    };
    map.into_iter()
        .filter_map(|(key, v)| {
            let number = |k: &str| {
                v.get(k)
                    .and_then(Value::as_f64)
                    .filter(|n| n.is_finite() && *n > 0.0)
            };
            Some((
                key,
                Kept {
                    width: number("width")? as f32,
                    height: number("height")? as f32,
                    maximized: v.get("maximized").and_then(Value::as_bool).unwrap_or(false),
                },
            ))
        })
        .collect()
}

pub(crate) fn text(kept: &BTreeMap<String, Kept>) -> String {
    let map: Map<String, Value> = kept
        .iter()
        .map(|(k, s)| {
            (
                k.clone(),
                json!({ "width": s.width, "height": s.height, "maximized": s.maximized }),
            )
        })
        .collect();
    serde_json::to_string_pretty(&Value::Object(map)).unwrap_or_default()
}

/// Writes `text` through a file beside `path`, renamed over it.
fn write_whole(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let partial = path.with_extension("json.yaziliyor");
    std::fs::write(&partial, text)?;
    std::fs::rename(&partial, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_go_out_and_come_back() {
        let mut kept = BTreeMap::new();
        kept.insert(
            "Formla düzenle".to_owned(),
            Kept {
                width: 900.0,
                height: 640.0,
                maximized: false,
            },
        );
        kept.insert(
            "Formlar".to_owned(),
            Kept {
                width: 1000.0,
                height: 700.0,
                maximized: true,
            },
        );
        assert_eq!(read(Some(&text(&kept))), kept);
        // What is not a size is passed over; a broken file is no sizes.
        let back = read(Some(
            r#"{"a": {"width": "x", "height": 3}, "b": {"width": 300, "height": 200}}"#,
        ));
        assert_eq!(back.keys().collect::<Vec<_>>(), ["b"]);
        assert!(read(Some("{")).is_empty());
        assert!(read(None).is_empty());
    }
}
