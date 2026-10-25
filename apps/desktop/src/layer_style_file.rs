//! Stil yükle… and Stili kaydet… (docs/adr/0635 §9, plan/gis.md CBS-17): a layer's look from or to one style file,
//! the file's extension choosing the format, the format's own command doing the work (`.qml` QGIS's
//! `file.import.qml`, `.sld` `file.import.sld`, `.json` a MapLibre style's `file.import.maplibre`, and the exports).
//! What each format carries is the style exchange's profile (docs/style/exchange.md). Tests give the path through
//! `Picker::File`.

use std::path::{Path, PathBuf};

use iced::Task;
use kentos_interaction::Level;

use crate::app::{App, Message, Picker};
use crate::layering::Event;

/// The style formats by their file's extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StyleFile {
    Qml,
    Sld,
    Maplibre,
}

impl StyleFile {
    pub(crate) fn of(path: &Path) -> Option<StyleFile> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "qml" => Some(StyleFile::Qml),
            "sld" | "xml" => Some(StyleFile::Sld),
            "json" => Some(StyleFile::Maplibre),
            _ => None,
        }
    }
}

const FILTERS: [(&str, &[&str]); 4] = [
    ("Stil dosyaları", &["qml", "sld", "xml", "json"]),
    ("QGIS stili (QML)", &["qml"]),
    ("SLD", &["sld", "xml"]),
    ("MapLibre stili", &["json"]),
];

impl App {
    /// Stil yükle…: the file asked, then read by its format.
    pub(crate) fn style_load_ask(&mut self, layer: String) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(Message::Layer(Event::StyleLoadFrom(
                layer,
                Some(path.clone()),
            )));
        }
        Task::perform(
            async move {
                let mut d = rfd::AsyncFileDialog::new().set_title("Stil yükle");
                for (name, ext) in FILTERS {
                    d = d.add_filter(name, ext);
                }
                d.pick_file().await.map(|f| f.path().to_path_buf())
            },
            move |p| Message::Layer(Event::StyleLoadFrom(layer.clone(), p)),
        )
    }

    /// Stili kaydet…: the path asked (QML when it has no extension), then written by its format.
    pub(crate) fn style_save_ask(&mut self, layer: String) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(Message::Layer(Event::StyleSaveTo(
                layer,
                Some(path.clone()),
            )));
        }
        let name = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&layer).map(|n| n.name.clone()))
            .unwrap_or_else(|| "katman".into());
        Task::perform(
            async move {
                let mut d = rfd::AsyncFileDialog::new()
                    .set_title("Stili kaydet")
                    .set_file_name(format!("{name}.qml"));
                for (name, ext) in &FILTERS[1..] {
                    d = d.add_filter(*name, ext);
                }
                d.save_file().await.map(|f| {
                    let mut p = f.path().to_path_buf();
                    if p.extension().is_none() {
                        p.set_extension("qml");
                    }
                    p
                })
            },
            move |p| Message::Layer(Event::StyleSaveTo(layer.clone(), p)),
        )
    }

    pub(crate) fn style_load_from(&mut self, layer: String, path: Option<PathBuf>) {
        let Some(path) = path else {
            return;
        };
        match StyleFile::of(&path) {
            Some(StyleFile::Qml) => self.qml_import_from(layer, Some(path)),
            Some(StyleFile::Sld) => self.sld_import_from(layer, Some(path)),
            Some(StyleFile::Maplibre) => self.maplibre_import_from(Some(layer), Some(path)),
            None => self.announce(Level::Error, unknown(&path)),
        }
    }

    pub(crate) fn style_save_to(&mut self, layer: String, path: Option<PathBuf>) {
        let Some(path) = path else {
            return;
        };
        match StyleFile::of(&path) {
            Some(StyleFile::Qml) => self.qml_export_to(layer, Some(path)),
            Some(StyleFile::Sld) => self.sld_export_to(layer, Some(path)),
            Some(StyleFile::Maplibre) => self.maplibre_export_to(Some(layer), Some(path)),
            None => self.announce(Level::Error, unknown(&path)),
        }
    }
}

fn unknown(path: &Path) -> String {
    format!(
        "{} bir stil dosyası değil: uzantısı .qml (QGIS), .sld ya da .json (MapLibre) olmalı.",
        path.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_extension_chooses_the_format() {
        assert_eq!(
            StyleFile::of(Path::new("a/parsel.QML")),
            Some(StyleFile::Qml)
        );
        assert_eq!(StyleFile::of(Path::new("parsel.sld")), Some(StyleFile::Sld));
        assert_eq!(StyleFile::of(Path::new("parsel.xml")), Some(StyleFile::Sld));
        assert_eq!(
            StyleFile::of(Path::new("stil.json")),
            Some(StyleFile::Maplibre)
        );
        assert_eq!(StyleFile::of(Path::new("stil.lyrx")), None);
        assert_eq!(StyleFile::of(Path::new("stil")), None);
    }
}
