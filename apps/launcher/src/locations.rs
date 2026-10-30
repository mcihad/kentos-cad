//! The places the user showed for programs the launcher did not find
//! (Konumu seç…, docs/adr/0800 §4): by app id, in
//! `$XDG_CONFIG_HOME/kentos-launcher/konumlar.json`. A place given here
//! comes before what the manifest says and before the search. The file is
//! written whole and renamed into place; one that cannot be read is empty.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::files;

const FORMAT: &str = "kentos.launcher-locations";
const FILE: &str = "konumlar.json";

/// The places, and the file they are kept in; in memory only without one.
#[derive(Debug, Default)]
pub struct Locations {
    path: Option<PathBuf>,
    programs: BTreeMap<String, PathBuf>,
}

impl Locations {
    /// Only in memory (the snapshot, tests).
    pub fn memory() -> Self {
        Self::default()
    }

    /// `…/kentos-launcher/konumlar.json`.
    pub fn default_path() -> Option<PathBuf> {
        files::launcher_config().map(|c| c.join(FILE))
    }

    /// The places kept in `path`.
    pub fn open(path: &Path) -> Self {
        let programs = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .filter(|v| v.get("format").and_then(Value::as_str) == Some(FORMAT))
            .and_then(|v| v.get("programs").and_then(Value::as_object).cloned())
            .map(|programs| {
                programs
                    .into_iter()
                    .filter_map(|(id, place)| Some((id, PathBuf::from(place.as_str()?))))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            path: Some(path.to_path_buf()),
            programs,
        }
    }

    /// The place shown for an app.
    pub fn get(&self, id: &str) -> Option<&Path> {
        self.programs.get(id).map(PathBuf::as_path)
    }

    /// Keeps a place for an app; the file is written at once.
    pub fn set(&mut self, id: &str, place: PathBuf) -> std::io::Result<()> {
        self.programs.insert(id.to_owned(), place);
        self.write()
    }

    /// Forgets an app's place.
    pub fn clear(&mut self, id: &str) -> std::io::Result<()> {
        if self.programs.remove(id).is_some() {
            self.write()?;
        }
        Ok(())
    }

    fn write(&self) -> std::io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let programs: serde_json::Map<String, Value> = self
            .programs
            .iter()
            .map(|(id, place)| {
                (
                    id.clone(),
                    Value::from(place.to_string_lossy().into_owned()),
                )
            })
            .collect();
        let mut text = serde_json::to_string_pretty(&json!({
            "format": FORMAT,
            "version": 1,
            "programs": programs,
        }))
        .unwrap_or_default();
        text.push('\n');
        files::write_whole(path, &text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_are_kept_forgotten_and_a_broken_file_is_empty() {
        let dir = files::scratch("locations");
        let path = dir.join(FILE);
        let mut places = Locations::open(&path);
        assert_eq!(places.get("kentos-gis"), None);
        places
            .set("kentos-gis", PathBuf::from("/opt/kentos/kentos-cad"))
            .expect("kept");
        places
            .set("rapor", PathBuf::from("/usr/bin/python3"))
            .expect("kept");
        let again = Locations::open(&path);
        assert_eq!(
            again.get("kentos-gis"),
            Some(Path::new("/opt/kentos/kentos-cad"))
        );
        places.clear("rapor").expect("forgotten");
        assert_eq!(Locations::open(&path).get("rapor"), None);
        std::fs::write(&path, "[bozuk").expect("written");
        assert_eq!(Locations::open(&path).get("kentos-gis"), None);
        let mut memory = Locations::memory();
        memory.set("a", PathBuf::from("/a")).expect("in memory");
        assert_eq!(memory.get("a"), Some(Path::new("/a")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
