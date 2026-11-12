//! A layer's look, or the whole map's, as a MapLibre GL style (docs/adr/0560, plan/gis.md CBS-17): Katmanlar' menu
//! writes the one layer's with `file.export.maplibre` and Dosya alışverişi (Veri › Dosya alışverişi, CAD › Ekle ›
//! İçe aktar) the visible layers' as one style, each with its GeoJSON beside it; the same commands take a style's
//! looks back with `file.import.maplibre`, one undo step “Katman stili”. What the style cannot say, and what KentOS
//! cannot take, is told in the command line. The path is asked; tests give it through `Picker::File`. The SLD's and
//! the QML's twins are `layer_sld.rs` and `layer_qml.rs`.

use std::path::PathBuf;

use iced::Task;
use kentos_commands::{CommandResult, ExportMaplibre, ImportMaplibre};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, maplibre};

use crate::app::{App, Message, Picker};
use crate::layering::Event;

impl App {
    /// Görünüşü MapLibre stili olarak kaydet…: the path asked, then written (the layer's, or the visible layers').
    pub(crate) fn maplibre_export_ask(&mut self, layer: Option<String>) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(Message::Layer(Event::MaplibreExportTo(
                layer,
                Some(path.clone()),
            )));
        }
        let name = match (&layer, &self.document) {
            (Some(id), Some(d)) => d
                .model
                .layers()
                .get(id)
                .map_or_else(|| "katman".to_owned(), |n| n.name.clone()),
            (None, Some(d)) => d.name().trim_end_matches(".kcad").to_owned(),
            _ => "harita".to_owned(),
        };
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .set_title("Görünüşü MapLibre stili olarak kaydet")
                    .add_filter("MapLibre stili (JSON)", &["json"])
                    .set_file_name(format!("{name}.json"))
                    .save_file()
                    .await
                    .map(|f| {
                        let mut p = f.path().to_path_buf();
                        if p.extension().is_none() {
                            p.set_extension("json");
                        }
                        p
                    })
            },
            move |p| Message::Layer(Event::MaplibreExportTo(layer.clone(), p)),
        )
    }

    /// MapLibre stilinden görünüş al…: the file asked, then read (into the layer, or every layer it is for).
    pub(crate) fn maplibre_import_ask(&mut self, layer: Option<String>) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(Message::Layer(Event::MaplibreImportFrom(
                layer,
                Some(path.clone()),
            )));
        }
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .set_title("MapLibre stilinden görünüş al")
                    .add_filter("MapLibre stili (JSON)", &["json"])
                    .pick_file()
                    .await
                    .map(|f| f.path().to_path_buf())
            },
            move |p| Message::Layer(Event::MaplibreImportFrom(layer.clone(), p)),
        )
    }

    /// Writes the look at `path`, with each layer's GeoJSON beside it (the save dialog asked about writing over).
    pub(crate) fn maplibre_export_to(&mut self, layer: Option<String>, path: Option<PathBuf>) {
        let (Some(path), Some(doc)) = (path, &self.document) else {
            return;
        };
        let input = ExportMaplibre {
            path: path.display().to_string(),
            layer,
            overwrite: true,
            source_url: None,
            source_kind: None,
            glyphs: None,
        };
        match kentos_headless::file_commands::export_maplibre(&doc.model, &input) {
            Ok((out, warnings)) => {
                self.announce(
                    if warnings.is_empty() {
                        Level::Success
                    } else {
                        Level::Warn
                    },
                    format!(
                        "{} yazıldı: {} katman, {} stil katmanı{}, yanında {} GeoJSON.",
                        out.path,
                        out.layers.len(),
                        out.style_layers,
                        out.labels
                            .map(|n| format!(" ({n} etiket)"))
                            .unwrap_or_default(),
                        out.data.len()
                    ),
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
            }
            Err(e) => self.announce(
                Level::Error,
                format!("MapLibre stili yazılamadı. {}", e.message),
            ),
        }
    }

    /// Makes the style's looks the layers', one undo step.
    pub(crate) fn maplibre_import_from(&mut self, layer: Option<String>, path: Option<PathBuf>) {
        let (Some(path), Some(doc)) = (path, self.document.as_mut()) else {
            return;
        };
        let input = ImportMaplibre {
            layer,
            from: None,
            path: Some(path.display().to_string()),
            text: None,
            expected_revision: None,
        };
        let answer = maplibre::execute(&mut ExecutionContext::new(&mut doc.model), input);
        match answer {
            CommandResult::Completed { output, warnings } => {
                let names: Vec<String> = output
                    .layers
                    .iter()
                    .map(|l| {
                        let words =
                            crate::layer_qml::look_words(&l.kind, l.rules, l.labels.as_ref());
                        let name = self
                            .document
                            .as_ref()
                            .and_then(|d| d.model.layers().get(&l.layer).map(|n| n.name.clone()))
                            .unwrap_or_else(|| l.layer.clone());
                        format!("{name}: {words}")
                    })
                    .collect();
                self.announce(
                    if warnings.is_empty() {
                        Level::Success
                    } else {
                        Level::Warn
                    },
                    format!(
                        "MapLibre stilinin görünüşü alındı ({}). Ctrl+Z geri alır.",
                        names.join("; ")
                    ),
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                self.announce(
                    Level::Error,
                    format!("MapLibre stili alınamadı. {}", error.message),
                );
            }
            _ => {}
        }
    }
}
