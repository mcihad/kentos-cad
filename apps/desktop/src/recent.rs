//! Drawings opened or saved lately (Son dosyalar), newest first, for the
//! start screen and the application menu (the web's `app/recentFiles.ts`).
//! The desktop keeps the paths in a small file of its own,
//! `son-dosyalar.json` under `$XDG_STATE_HOME/kentos-cad`
//! (`~/.local/state/kentos-cad` when unset): history the program keeps, not
//! a setting and not the drawing, and nothing of the drawing itself. The
//! file is written whole and renamed into place, so a crash leaves the old
//! list or the new one.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Files kept (the web's `MAX`).
const MAX: usize = 10;
const FORMAT: &str = "kentos.recent-files";
const FILE: &str = "son-dosyalar.json";

/// One drawing of the list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recent {
    pub path: PathBuf,
    /// The file's name as it was opened or saved.
    pub name: String,
    /// When, in milliseconds since the epoch.
    pub at: u64,
    /// A line about it as it was then: objects and coordinate system.
    pub info: String,
}

#[derive(Serialize, Deserialize)]
struct Stored {
    format: String,
    version: u32,
    files: Vec<Recent>,
}

/// The list and where it is kept; in memory only when there is no folder.
#[derive(Debug, Default)]
pub struct RecentFiles {
    folder: Option<PathBuf>,
    list: Vec<Recent>,
}

impl RecentFiles {
    /// Only in memory (tests, and a program without a state folder).
    pub fn memory() -> Self {
        Self::default()
    }

    /// The list kept in `folder`; an unreadable file is an empty list (the
    /// next file opened or saved writes it again, never a reason to stop).
    pub fn open(folder: &Path) -> Self {
        let list = std::fs::read_to_string(folder.join(FILE))
            .ok()
            .and_then(|text| serde_json::from_str::<Stored>(&text).ok())
            .filter(|s| s.format == FORMAT && s.version == 1)
            .map(|s| s.files)
            .unwrap_or_default();
        Self {
            folder: Some(folder.to_path_buf()),
            list,
        }
    }

    /// `$XDG_STATE_HOME/kentos-cad`, else `~/.local/state/kentos-cad`.
    pub fn default_folder() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_STATE_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
        Some(base.join("kentos-cad"))
    }

    /// Newest first.
    pub fn list(&self) -> &[Recent] {
        &self.list
    }

    /// Puts a file first (or moves it there) with a line about it.
    pub fn add(&mut self, path: &Path, info: String) {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        self.list.retain(|r| r.path != path);
        self.list.insert(
            0,
            Recent {
                path: path.to_path_buf(),
                name,
                at: now_ms(),
                info,
            },
        );
        self.list.truncate(MAX);
        self.write();
    }

    /// Forgets a file (it was moved or deleted, or the user asked).
    pub fn remove(&mut self, path: &Path) {
        let before = self.list.len();
        self.list.retain(|r| r.path != path);
        if self.list.len() != before {
            self.write();
        }
    }

    /// Writes the list whole, then renames it into place. A failure is not
    /// something the user must act on: the list is a convenience.
    fn write(&self) {
        let Some(folder) = &self.folder else { return };
        let stored = Stored {
            format: FORMAT.to_owned(),
            version: 1,
            files: self.list.clone(),
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

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kentos-recent-{name}-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn newest_first_ten_at_most_and_kept_across_openings() {
        let dir = scratch("order");
        let mut recent = RecentFiles::open(&dir);
        for i in 0..12 {
            recent.add(
                &dir.join(format!("{i}.kcad")),
                format!("{i} nesne · TUREF / TM36"),
            );
        }
        recent.add(&dir.join("5.kcad"), "yeniden".into());
        let names: Vec<&str> = recent.list().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names.len(), MAX);
        assert_eq!(names[0], "5.kcad", "the file opened again comes first");
        assert_eq!(names[1], "11.kcad");
        assert_eq!(names.iter().filter(|n| **n == "5.kcad").count(), 1, "once");
        assert!(!names.contains(&"0.kcad"), "the oldest went");
        let mut again = RecentFiles::open(&dir);
        assert_eq!(again.list(), recent.list(), "the list outlives the program");
        again.remove(&dir.join("5.kcad"));
        assert_eq!(RecentFiles::open(&dir).list().len(), MAX - 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unreadable_list_is_empty_and_is_written_again() {
        let dir = scratch("broken");
        std::fs::create_dir_all(&dir).expect("a folder");
        std::fs::write(dir.join(FILE), "{bozuk").expect("written");
        let mut recent = RecentFiles::open(&dir);
        assert!(recent.list().is_empty());
        recent.add(&dir.join("a.kcad"), "1 nesne".into());
        assert_eq!(RecentFiles::open(&dir).list().len(), 1);
        assert!(
            !dir.join(format!("{FILE}.yaziliyor")).exists(),
            "renamed into place"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn in_memory_nothing_is_written() {
        let mut recent = RecentFiles::memory();
        recent.add(Path::new("/yok/a.kcad"), "1 nesne".into());
        assert_eq!(recent.list().len(), 1);
        assert!(RecentFiles::default().list().is_empty());
    }
}
