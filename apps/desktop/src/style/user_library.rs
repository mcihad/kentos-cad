//! Kitaplığım on the desktop (the web keeps it in the browser, localStorage
//! `kentos.styles.v1`): the user's own symbols, drawings and categories, in
//! `kitaplik.kstil` in the user's data folder (`$XDG_DATA_HOME/kentos-cad`,
//! else `~/.local/share/kentos-cad`), as a .kstil style file, so it can be
//! shared as it is. Written whole and renamed into place after every change.
//! A file that cannot be read is set aside (`kitaplik-okunamadi-<time>.kstil`),
//! never written over; the app says so once.

use std::path::{Path, PathBuf};

use kentos_native_style::file::{STYLE_FORMAT, STYLE_VERSION, parse_style_file};
use kentos_native_style::library::{Source, StyleLibrary};
use serde_json::{Value, json};

pub const FILE: &str = "kitaplik.kstil";

/// Where Kitaplığım is kept; nowhere (only in memory) in tests.
#[derive(Debug, Default)]
pub struct UserLibrary {
    folder: Option<PathBuf>,
}

/// The user's data folder: `$XDG_DATA_HOME/kentos-cad`, else `~/.local/share/kentos-cad`.
pub fn default_folder() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
        })?;
    Some(base.join("kentos-cad"))
}

/// What Kitaplığım's file held.
pub struct Opened {
    pub items: Vec<Value>,
    pub categories: Vec<Value>,
    /// What to say when the file could not be read.
    pub problem: Option<String>,
}

impl UserLibrary {
    /// Kitaplığım in `folder`, and what its file holds.
    pub fn open(folder: &Path) -> (UserLibrary, Opened) {
        let path = folder.join(FILE);
        let lib = UserLibrary {
            folder: Some(folder.to_path_buf()),
        };
        let empty = |problem| Opened {
            items: Vec::new(),
            categories: Vec::new(),
            problem,
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (lib, empty(None)),
            Err(e) => {
                return (
                    lib,
                    empty(Some(format!(
                        "Kitaplığım okunamadı ({}): {e}. Semboller bu oturumda boş; dosya olduğu gibi duruyor.",
                        path.display()
                    ))),
                );
            }
        };
        match parse_style_file(&text) {
            Ok(file) => {
                let categories = file.categories.unwrap_or_default();
                (
                    lib,
                    Opened {
                        items: file.items,
                        categories,
                        problem: None,
                    },
                )
            }
            Err(issues) => {
                // Set aside, never written over: the user's symbols stay recoverable.
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                let aside = folder.join(format!("kitaplik-okunamadi-{stamp}.kstil"));
                let moved = std::fs::rename(&path, &aside).is_ok();
                let why = issues.first().cloned().unwrap_or_default();
                let problem = if moved {
                    format!(
                        "Kitaplığım okunamadı: {why} Dosya {} olarak ayrıldı; boş bir kitaplıkla başlandı. Düzeltip Stil yöneticisinden içe aktarabilirsiniz.",
                        aside.display()
                    )
                } else {
                    format!(
                        "Kitaplığım okunamadı: {why} Dosya yerinde duruyor; bu oturumda değişiklikler kaydedilmeyecek."
                    )
                };
                let lib = if moved {
                    lib
                } else {
                    UserLibrary { folder: None }
                };
                (lib, empty(Some(problem)))
            }
        }
    }

    /// Writes Kitaplığım whole and renames it into place; why not, when it fails.
    pub fn save(&self, library: &StyleLibrary) -> Result<(), String> {
        let Some(folder) = &self.folder else {
            return Ok(());
        };
        let (items, categories) = library.dump(Source::User);
        let exported = kentos_native_style::file::iso_now();
        let file = json!({
            "format": STYLE_FORMAT,
            "version": STYLE_VERSION,
            "exported": exported,
            "items": items,
            "categories": categories,
        });
        let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        let temp = folder.join(format!("{FILE}.yaziliyor"));
        std::fs::write(&temp, text).map_err(|e| format!("{}: {e}", temp.display()))?;
        std::fs::rename(&temp, folder.join(FILE)).map_err(|e| format!("{}: {e}", folder.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kentos-kitaplik-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn kitapligim_outlives_the_program() {
        let dir = scratch("tur");
        let (file, opened) = UserLibrary::open(&dir);
        assert!(opened.items.is_empty() && opened.problem.is_none());
        let mut lib = kentos_native_style::system::library();
        lib.add(
            Source::User,
            json!({ "kind": "symbol", "id": "u-1", "name": "Benim", "path": ["Sembollerim"], "symbol": { "type": "fill", "layers": [{ "id": "f", "type": "simpleFill", "color": "#EDC948" }] } }),
        )
        .expect("adds");
        lib.add_category(
            Source::User,
            json!({ "path": ["Sembollerim"], "description": "Benimkiler" }),
        );
        file.save(&lib).expect("writes");
        let (_, opened) = UserLibrary::open(&dir);
        assert_eq!(opened.items.len(), 1);
        assert_eq!(opened.items[0]["name"], "Benim");
        assert_eq!(opened.categories[0]["description"], "Benimkiler");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_broken_file_is_set_aside_not_written_over() {
        let dir = scratch("bozuk");
        std::fs::create_dir_all(&dir).expect("folder");
        std::fs::write(dir.join(FILE), "{ bozuk").expect("writes");
        let (file, opened) = UserLibrary::open(&dir);
        assert!(opened.problem.is_some_and(|p| p.contains("ayrıldı")));
        let aside: Vec<_> = std::fs::read_dir(&dir)
            .expect("lists")
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("kitaplik-okunamadi-")
            })
            .collect();
        assert_eq!(aside.len(), 1);
        assert_eq!(
            std::fs::read_to_string(aside[0].path()).expect("reads"),
            "{ bozuk"
        );
        file.save(&kentos_native_style::system::library())
            .expect("a new file");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
