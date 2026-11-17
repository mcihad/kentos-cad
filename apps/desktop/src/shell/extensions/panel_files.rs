//! A panel's file picker (docs/adr/8057 §41): a file the user gives to be read, its path typed or chosen in the
//! computer's file window (Seç…). Its value is the file's absolute path; a path that is no file, or of a kind the
//! picker does not offer, is said by its label before anything runs. Reading the file is the extension's, which asks
//! for `files.userSelected` (Kapı 1 says so of a package whose panel has one).
//!
//! With `save` (§44) it is where a file is to be written: the computer's save window (it asks before going over a
//! file), the file need not be there but its folder must, a name typed without an extension takes the first kind's.
//! Writing is the extension's, which asks for `files.userSelectedWrite`.

use std::path::{Path, PathBuf};

use iced::Task;
use kentos_extensions::ui_panel::FileFilter;
use serde_json::{Value, json};

use crate::app::Message;

/// A file picker as its component writes it: what the value is checked against and what the window offers.
#[derive(Clone, Debug)]
pub(super) struct Picker {
    pub(super) label: String,
    pub(super) filters: Vec<FileFilter>,
    pub(super) save: bool,
    pub(super) default_name: Option<String>,
}

/// The kinds `filters` offer, in words: “CSV (.csv) ya da GeoJSON (.geojson, .json)”.
fn kinds(filters: &[FileFilter]) -> String {
    let kinds: Vec<String> = filters
        .iter()
        .map(|f| format!("{} (.{})", f.name, f.extensions.join(", .")))
        .collect();
    kinds.join(" ya da ")
}

/// Whether `path`'s extension is one of `filters`' (any without filters).
fn offered(filters: &[FileFilter], path: &Path) -> bool {
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    filters.is_empty() || filters.iter().any(|f| f.extensions.contains(&extension))
}

/// The picker's value: none when nothing is given; to read, the file's absolute path when it is a file of one of
/// `filters`' kinds (any kind without filters); to `save`, where it is to be written ([`save_value`]); why not, by
/// `label`.
pub(super) fn file_value(
    label: &str,
    filters: &[FileFilter],
    save: bool,
    typed: &str,
) -> Result<Value, String> {
    let typed = typed.trim();
    if typed.is_empty() {
        return Ok(Value::Null);
    }
    if save {
        return save_value(label, filters, typed);
    }
    let path = Path::new(typed);
    if !path.is_file() {
        return Err(format!(
            "“{label}”: “{typed}” bulunamadı ya da bir dosya değil."
        ));
    }
    if !offered(filters, path) {
        return Err(format!("“{label}”: {} olmalı.", kinds(filters)));
    }
    let absolute = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(typed));
    Ok(json!(absolute.display().to_string()))
}

/// Where a file is to be written: its absolute path, its folder there; a name without an extension takes the first
/// kind's (the save window does not add it everywhere), another kind is said; a folder is not a file's place.
fn save_value(label: &str, filters: &[FileFilter], typed: &str) -> Result<Value, String> {
    let mut path = PathBuf::from(typed);
    if path.is_dir() || typed.ends_with(['/', '\\']) {
        return Err(format!(
            "“{label}”: “{typed}” bir klasör; dosyanın adı da yazılmalı."
        ));
    }
    if path.extension().is_none()
        && let Some(first) = filters.first().and_then(|f| f.extensions.first())
    {
        path.set_extension(first);
    }
    if !offered(filters, &path) {
        return Err(format!("“{label}”: {} olmalı.", kinds(filters)));
    }
    let folder = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let Ok(folder) = std::fs::canonicalize(folder) else {
        return Err(format!("“{label}”: “{}” klasörü yok.", folder.display()));
    };
    if !folder.is_dir() {
        return Err(format!(
            "“{label}”: “{}” bir klasör değil.",
            folder.display()
        ));
    }
    let name = path.file_name().unwrap_or_default();
    Ok(json!(folder.join(name).display().to_string()))
}

/// The computer's file window for `picker`, titled by its label and offering its kinds: the open window, or the save
/// window offering its name; the chosen path to `done`, nothing when the user closes it.
pub(super) fn pick(
    picker: &Picker,
    done: impl Fn(String) -> Message + Send + 'static,
) -> Task<Message> {
    let picker = picker.clone();
    Task::perform(
        async move {
            let mut dialog = rfd::AsyncFileDialog::new().set_title(&picker.label);
            for f in &picker.filters {
                dialog = dialog.add_filter(&f.name, &f.extensions);
            }
            let chosen = if picker.save {
                if let Some(name) = &picker.default_name {
                    dialog = dialog.set_file_name(name);
                }
                dialog.save_file().await
            } else {
                dialog.pick_file().await
            };
            chosen.map(|f| f.path().display().to_string())
        },
        |path| path,
    )
    .and_then(move |path| Task::done(done(path)))
}

/// Picker `id` in `panel`; none when it is no file picker.
pub(super) fn picker_of(panel: &super::panels::OpenPanel, id: &str) -> Option<Picker> {
    panel.definition.body.iter().find_map(|c| match c {
        kentos_extensions::ui_panel::Component::FilePicker {
            id: own,
            label,
            filters,
            save,
            default_name,
            ..
        } if own == id => Some(Picker {
            label: label.clone(),
            filters: filters.clone(),
            save: *save,
            default_name: default_name.clone(),
        }),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// None when empty; a file of the offered kind is its absolute path; a missing path and another kind are said by
    /// the label.
    #[test]
    fn a_picked_file_is_its_absolute_path_and_a_wrong_one_is_said() {
        let dir = std::env::temp_dir().join(format!("kentos-dosya-secici-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let csv = dir.join("noktalar.CSV");
        std::fs::write(&csv, "x,y\n1,2\n").unwrap();
        let filters = [FileFilter {
            name: "CSV".into(),
            extensions: vec!["csv".into()],
        }];
        assert_eq!(file_value("Dosya", &filters, false, "  "), Ok(Value::Null));
        let value = file_value("Dosya", &filters, false, &csv.display().to_string()).unwrap();
        assert!(
            value
                .as_str()
                .is_some_and(|p| p.ends_with("noktalar.CSV") && Path::new(p).is_absolute())
        );
        let missing = file_value(
            "Dosya",
            &filters,
            false,
            &dir.join("yok.csv").display().to_string(),
        )
        .unwrap_err();
        assert!(
            missing.starts_with("“Dosya”: “")
                && missing.ends_with("bulunamadı ya da bir dosya değil.")
        );
        let other = dir.join("notlar.txt");
        std::fs::write(&other, "x").unwrap();
        assert_eq!(
            file_value("Dosya", &filters, false, &other.display().to_string()).unwrap_err(),
            "“Dosya”: CSV (.csv) olmalı."
        );
        assert!(
            file_value("Dosya", &[], false, &other.display().to_string()).is_ok(),
            "any kind without filters"
        );
        // As a panel's input: asked when required, its path in the command's parameters.
        let value = serde_json::json!({ "format": kentos_extensions::ui_panel::FORMAT,
            "schemaVersion": kentos_extensions::ui_panel::VERSION, "title": "İçe aktar",
            "body": [{ "type": "filePicker", "id": "dosya", "label": "CSV dosyası", "required": true,
                       "filters": [{ "name": "CSV", "extensions": ["csv"] }] }] });
        let mut p = super::super::panels::OpenPanel::new(
            "tr.ornek.deneme",
            "ice-aktar",
            kentos_extensions::ui_panel::read(&value).expect("a panel"),
        );
        assert_eq!(
            p.check_params().unwrap_err(),
            (
                "dosya".to_owned(),
                "“CSV dosyası”: dosya verilmedi (yolunu yazın ya da Seç…'e basın).".to_owned()
            )
        );
        p.texts.insert("dosya".into(), csv.display().to_string());
        let params = p.check_params().expect("the values");
        assert!(
            params["dosya"]
                .as_str()
                .is_some_and(|f| f.ends_with("noktalar.CSV"))
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Where a file is to be written (docs/adr/8057 §44): a name without an extension takes the first kind's, the file
    /// need not be there but its folder must; another kind and a folder are said by the label.
    #[test]
    fn a_save_place_is_where_the_file_is_to_be_written() {
        let dir = std::env::temp_dir().join(format!("kentos-kaydetme-yeri-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let filters = [FileFilter {
            name: "CSV".into(),
            extensions: vec!["csv".into()],
        }];
        let at = |name: &str| dir.join(name).display().to_string();
        let made = std::fs::canonicalize(&dir).unwrap().join("parseller.csv");
        let value = file_value("Yer", &filters, true, &at("parseller")).unwrap();
        assert_eq!(value.as_str().map(Path::new), Some(made.as_path()));
        assert!(
            file_value("Yer", &filters, true, &at("parseller.csv")).is_ok(),
            "the file need not be there"
        );
        assert_eq!(
            file_value("Yer", &filters, true, &at("notlar.txt")).unwrap_err(),
            "“Yer”: CSV (.csv) olmalı."
        );
        let nowhere = dir.join("yok").join("a.csv").display().to_string();
        let missing = file_value("Yer", &filters, true, &nowhere).unwrap_err();
        assert!(missing.ends_with("klasörü yok."), "{missing}");
        let folder = file_value("Yer", &filters, true, &dir.display().to_string()).unwrap_err();
        assert!(folder.contains("bir klasör"), "{folder}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
