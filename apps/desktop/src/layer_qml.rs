//! A layer's look as a QGIS layer style file (docs/adr/0555, plan/gis.md CBS-17): Katmanlar's menu writes it with
//! `file.export.qml` (what the QML cannot say told in the command line) and takes a QML's look with `file.import.qml`,
//! one undo step “Katman stili” (what KentOS cannot take told). The path is asked; tests give it through
//! `Picker::File`. The SLD's twin is `layer_sld.rs`.

use std::path::PathBuf;

use iced::Task;
use kentos_commands::{CommandResult, ExportQml, ImportQml};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, qml};

use crate::app::{App, Message, Picker};
use crate::layering::Event;

impl App {
    /// Görünüşü QML olarak kaydet…: the path asked, then written.
    pub(crate) fn qml_export_ask(&mut self, layer: String) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(Message::Layer(Event::QmlExportTo(
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
                rfd::AsyncFileDialog::new()
                    .set_title("Görünüşü QML olarak kaydet")
                    .add_filter("QGIS stili (QML)", &["qml"])
                    .set_file_name(format!("{name}.qml"))
                    .save_file()
                    .await
                    .map(|f| {
                        let mut p = f.path().to_path_buf();
                        if p.extension().is_none() {
                            p.set_extension("qml");
                        }
                        p
                    })
            },
            move |p| Message::Layer(Event::QmlExportTo(layer.clone(), p)),
        )
    }

    /// QML'den görünüş al…: the file asked, then read.
    pub(crate) fn qml_import_ask(&mut self, layer: String) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(Message::Layer(Event::QmlImportFrom(
                layer,
                Some(path.clone()),
            )));
        }
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .set_title("QML'den görünüş al")
                    .add_filter("QGIS stili (QML)", &["qml"])
                    .pick_file()
                    .await
                    .map(|f| f.path().to_path_buf())
            },
            move |p| Message::Layer(Event::QmlImportFrom(layer.clone(), p)),
        )
    }

    /// Writes the layer's look at `path` (the save dialog asked about writing over it).
    pub(crate) fn qml_export_to(&mut self, layer: String, path: Option<PathBuf>) {
        let (Some(path), Some(doc)) = (path, &self.document) else {
            return;
        };
        let input = ExportQml {
            path: path.display().to_string(),
            layer,
            overwrite: true,
        };
        match kentos_headless::file_commands::export_qml(&doc.model, &input) {
            Ok((out, warnings)) => {
                self.announce(
                    if warnings.is_empty() {
                        Level::Success
                    } else {
                        Level::Warn
                    },
                    match out.labels {
                        Some(n) => format!(
                            "{} yazıldı: {} sınıf ya da kural, {n} etiket.",
                            out.path, out.rules
                        ),
                        None => format!("{} yazıldı: {} sınıf ya da kural.", out.path, out.rules),
                    },
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
            }
            Err(e) => self.announce(Level::Error, format!("QML yazılamadı. {}", e.message)),
        }
    }

    /// Makes the QML's look the layer's, one undo step.
    pub(crate) fn qml_import_from(&mut self, layer: String, path: Option<PathBuf>) {
        let (Some(path), Some(doc)) = (path, self.document.as_mut()) else {
            return;
        };
        let input = ImportQml {
            layer,
            path: Some(path.display().to_string()),
            text: None,
            expected_revision: None,
        };
        let answer = qml::execute(&mut ExecutionContext::new(&mut doc.model), input);
        match answer {
            CommandResult::Completed { output, warnings } => {
                let text = taken_text("QML", &output.kind, output.rules, output.labels.as_ref());
                self.announce(
                    if warnings.is_empty() {
                        Level::Success
                    } else {
                        Level::Warn
                    },
                    text,
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                self.announce(Level::Error, format!("QML alınamadı. {}", error.message));
            }
            _ => {}
        }
    }
}

/// What a style file gave a layer, in words: its renderer's kind and classes, and its label classes (docs/adr/0635
/// §1).
pub(crate) fn look_words(
    kind: &str,
    rules: u32,
    labels: Option<&kentos_commands::LayerLabels>,
) -> String {
    let mut parts = Vec::new();
    if !kind.is_empty() {
        let k = match kind {
            "categorized" => "kategorili",
            "graduated" => "aralıklı",
            "rules" => "kurallı",
            _ => "tek simge",
        };
        parts.push(format!("{k}, {rules} sınıf ya da kural"));
    }
    if let Some(l) = labels {
        let n = if l.mode == kentos_commands::LabelsMode::Rules {
            l.classes.len()
        } else {
            1
        };
        parts.push(format!("{n} etiket"));
    }
    parts.join(", ")
}

/// The line said when a style file's look was taken: `file` is SLD or QML.
pub(crate) fn taken_text(
    file: &str,
    kind: &str,
    rules: u32,
    labels: Option<&kentos_commands::LayerLabels>,
) -> String {
    let whose = if file == "SLD" { "SLD'nin" } else { "QML'in" };
    format!(
        "{whose} görünüşü alındı: {}. Ctrl+Z geri alır.",
        look_words(kind, rules, labels)
    )
}
