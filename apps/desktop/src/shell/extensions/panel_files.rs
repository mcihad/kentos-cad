//! A panel's file picker (docs/adr/8057 §41): a file the user gives to be read, its path typed or chosen in the
//! computer's file window (Seç…). Its value is the file's absolute path; a path that is no file, or of a kind the
//! picker does not offer, is said by its label before anything runs. Reading the file is the extension's, which asks
//! for `files.userSelected` (Kapı 1 says so of a package whose panel has one).

use std::path::{Path, PathBuf};

use iced::Task;
use kentos_extensions::ui_panel::FileFilter;
use serde_json::{Value, json};

use crate::app::Message;

/// The picker's value: none when nothing is given, the file's absolute path when it is a file of one of `filters`'
/// kinds (any kind without filters); why not, by `label`.
pub(super) fn file_value(
    label: &str,
    filters: &[FileFilter],
    typed: &str,
) -> Result<Value, String> {
    let typed = typed.trim();
    if typed.is_empty() {
        return Ok(Value::Null);
    }
    let path = Path::new(typed);
    if !path.is_file() {
        return Err(format!(
            "“{label}”: “{typed}” bulunamadı ya da bir dosya değil."
        ));
    }
    if !filters.is_empty() {
        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if !filters
            .iter()
            .any(|f| f.extensions.contains(&extension))
        {
            let kinds: Vec<String> = filters
                .iter()
                .map(|f| format!("{} (.{})", f.name, f.extensions.join(", .")))
                .collect();
            return Err(format!("“{label}”: {} olmalı.", kinds.join(" ya da ")));
        }
    }
    let absolute = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(typed));
    Ok(json!(absolute.display().to_string()))
}

/// The computer's file window, titled `title`, offering `filters`' kinds: the chosen file's path to `done`; nothing
/// when the user closes it.
pub(super) fn pick(
    title: &str,
    filters: &[FileFilter],
    done: impl Fn(String) -> Message + Send + 'static,
) -> Task<Message> {
    let title = title.to_owned();
    let filters = filters.to_vec();
    Task::perform(
        async move {
            let mut dialog = rfd::AsyncFileDialog::new().set_title(&title);
            for f in &filters {
                dialog = dialog.add_filter(&f.name, &f.extensions);
            }
            dialog
                .pick_file()
                .await
                .map(|f| f.path().display().to_string())
        },
        |path| path,
    )
    .and_then(move |path| Task::done(done(path)))
}

/// Picker `id`'s label and kinds in `panel`; none when it is no file picker.
pub(super) fn picker_of(
    panel: &super::panels::OpenPanel,
    id: &str,
) -> Option<(String, Vec<FileFilter>)> {
    panel.definition.body.iter().find_map(|c| match c {
        kentos_extensions::ui_panel::Component::FilePicker {
            id: own,
            label,
            filters,
            ..
        } if own == id => Some((label.clone(), filters.clone())),
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
        assert_eq!(file_value("Dosya", &filters, "  "), Ok(Value::Null));
        let value = file_value("Dosya", &filters, &csv.display().to_string()).unwrap();
        assert!(
            value
                .as_str()
                .is_some_and(|p| p.ends_with("noktalar.CSV") && Path::new(p).is_absolute())
        );
        let missing = file_value(
            "Dosya",
            &filters,
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
            file_value("Dosya", &filters, &other.display().to_string()).unwrap_err(),
            "“Dosya”: CSV (.csv) olmalı."
        );
        assert!(
            file_value("Dosya", &[], &other.display().to_string()).is_ok(),
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
}
