//! What each surface's window remembers (docs/adr/8060 §4). Between runs, in `ayri-pencereler.json` beside the
//! settings: whether it opens in a window of its own and its size. For this run only: where it was on the screen. A
//! screen unplugged since the last run would put the window where nobody can see it, and the platform does not
//! always bring such a window back. A file that cannot be read or written is passed over: the window opens in the
//! main window at its own size.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use iced::{Point, Size};
use serde_json::{Map, Value, json};

use super::Surface;

/// The file, beside `ayarlar.json`.
pub(crate) const FILE_NAME: &str = "ayri-pencereler.json";

/// What a surface keeps between runs.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Kept {
    /// It opens in a window of its own.
    pub(crate) own_window: bool,
    /// Its window's inner size when it last closed.
    pub(crate) size: Option<Size>,
}

/// The surfaces' choices and where they are written; none: kept in memory only (tests).
#[derive(Debug, Default)]
pub(crate) struct Memory {
    path: Option<PathBuf>,
    kept: BTreeMap<String, Kept>,
    /// Where each surface's window was this run (its outer position).
    places: HashMap<Surface, Point>,
}

impl Memory {
    /// The choices kept in `folder`.
    pub(crate) fn open(folder: &Path) -> Self {
        let path = folder.join(FILE_NAME);
        Self {
            kept: read(std::fs::read_to_string(&path).ok().as_deref()),
            path: Some(path),
            places: HashMap::new(),
        }
    }

    /// Whether `surface` opens in a window of its own.
    pub(crate) fn apart(&self, surface: Surface) -> bool {
        self.kept.get(surface.key()).is_some_and(|k| k.own_window)
    }

    /// Keeps whether `surface` opens in a window of its own, written at once.
    pub(crate) fn set_apart(&mut self, surface: Surface, apart: bool) {
        let kept = self.kept.entry(surface.key().to_owned()).or_default();
        if kept.own_window != apart {
            kept.own_window = apart;
            self.write();
        }
    }

    /// The size `surface`'s window had when it last closed.
    pub(crate) fn size(&self, surface: Surface) -> Option<Size> {
        self.kept.get(surface.key()).and_then(|k| k.size)
    }

    /// Keeps `surface`'s window size; written when the window closes (a resize sends many).
    pub(crate) fn keep_size(&mut self, surface: Surface, size: Size) {
        if size.width > 0.0 && size.height > 0.0 {
            self.kept.entry(surface.key().to_owned()).or_default().size = Some(size);
        }
    }

    /// Where `surface`'s window was this run.
    pub(crate) fn place(&self, surface: Surface) -> Option<Point> {
        self.places.get(&surface).copied()
    }

    pub(crate) fn keep_place(&mut self, surface: Surface, at: Point) {
        self.places.insert(surface, at);
    }

    /// Writes the choices through a file beside the file, renamed over it.
    pub(crate) fn write(&self) {
        let Some(path) = &self.path else { return };
        let partial = path.with_extension("json.yaziliyor");
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&partial, text(&self.kept)))
            .and_then(|()| std::fs::rename(&partial, path));
        if written.is_err() {
            let _ = std::fs::remove_file(&partial);
        }
    }
}

/// The choices in `text` (`{"islem-araci": {"ownWindow": true, "width": 940, "height": 660}}`); what is not one is
/// passed over.
pub(crate) fn read(text: Option<&str>) -> BTreeMap<String, Kept> {
    let Some(Value::Object(map)) = text.and_then(|t| serde_json::from_str::<Value>(t).ok()) else {
        return BTreeMap::new();
    };
    map.into_iter()
        .filter(|(_, v)| v.is_object())
        .map(|(key, v)| {
            let number = |k: &str| {
                v.get(k)
                    .and_then(Value::as_f64)
                    .filter(|n| n.is_finite() && *n > 0.0)
                    .map(|n| n as f32)
            };
            let size = number("width")
                .zip(number("height"))
                .map(|(w, h)| Size::new(w, h));
            let own_window = v.get("ownWindow").and_then(Value::as_bool).unwrap_or(false);
            (key, Kept { own_window, size })
        })
        .collect()
}

pub(crate) fn text(kept: &BTreeMap<String, Kept>) -> String {
    let map: Map<String, Value> = kept
        .iter()
        .map(|(key, k)| {
            let mut v = json!({ "ownWindow": k.own_window });
            if let Some(size) = k.size {
                v["width"] = json!(size.width);
                v["height"] = json!(size.height);
            }
            (key.clone(), v)
        })
        .collect();
    serde_json::to_string_pretty(&Value::Object(map)).unwrap_or_default()
}
