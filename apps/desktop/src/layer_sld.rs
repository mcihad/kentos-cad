//! A layer's look as SLD (docs/adr/0516, plan/gis.md CBS-17): Katmanlar's menu writes it with `file.export.sld`
//! (what SE cannot say told in the command line) and takes an SLD's look with `file.import.sld`, one undo step
//! “Katman stili” (what KentOS cannot take told). The path is asked; tests give it through `Picker::File`.

use std::path::PathBuf;

use iced::Task;
use kentos_commands::{CommandResult, ExportSld, ImportSld};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, sld};

use crate::app::{App, Message, Picker};
use crate::layering::Event;

impl App {
    /// Görünüşü SLD olarak kaydet…: the path asked, then written.
    pub(crate) fn sld_export_ask(&mut self, layer: String) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(Message::Layer(Event::SldExportTo(
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
                    .set_title("Görünüşü SLD olarak kaydet")
                    .add_filter("SLD", &["sld"])
                    .set_file_name(format!("{name}.sld"))
                    .save_file()
                    .await
                    .map(|f| {
                        let mut p = f.path().to_path_buf();
                        if p.extension().is_none() {
                            p.set_extension("sld");
                        }
                        p
                    })
            },
            move |p| Message::Layer(Event::SldExportTo(layer.clone(), p)),
        )
    }

    /// SLD'den görünüş al…: the file asked, then read.
    pub(crate) fn sld_import_ask(&mut self, layer: String) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(Message::Layer(Event::SldImportFrom(
                layer,
                Some(path.clone()),
            )));
        }
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .set_title("SLD'den görünüş al")
                    .add_filter("SLD", &["sld", "xml"])
                    .pick_file()
                    .await
                    .map(|f| f.path().to_path_buf())
            },
            move |p| Message::Layer(Event::SldImportFrom(layer.clone(), p)),
        )
    }

    /// Writes the layer's look at `path` (the save dialog asked about writing over it).
    pub(crate) fn sld_export_to(&mut self, layer: String, path: Option<PathBuf>) {
        let (Some(path), Some(doc)) = (path, &self.document) else {
            return;
        };
        let input = ExportSld {
            path: path.display().to_string(),
            layer,
            overwrite: true,
        };
        match kentos_headless::file_commands::export_sld(&doc.model, &input) {
            Ok((out, warnings)) => {
                self.announce(
                    if warnings.is_empty() {
                        Level::Success
                    } else {
                        Level::Warn
                    },
                    format!("{} yazıldı: {} kural.", out.path, out.rules),
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
            }
            Err(e) => self.announce(Level::Error, format!("SLD yazılamadı. {}", e.message)),
        }
    }

    /// Makes the SLD's look the layer's, one undo step.
    pub(crate) fn sld_import_from(&mut self, layer: String, path: Option<PathBuf>) {
        let (Some(path), Some(doc)) = (path, self.document.as_mut()) else {
            return;
        };
        let input = ImportSld {
            layer,
            path: Some(path.display().to_string()),
            text: None,
            expected_revision: None,
        };
        let answer = sld::execute(&mut ExecutionContext::new(&mut doc.model), input);
        match answer {
            CommandResult::Completed { output, warnings } => {
                let kind = match output.kind.as_str() {
                    "categorized" => "kategorili",
                    "graduated" => "aralıklı",
                    "rules" => "kurallı",
                    _ => "tek simge",
                };
                self.announce(
                    if warnings.is_empty() {
                        Level::Success
                    } else {
                        Level::Warn
                    },
                    format!(
                        "SLD'nin görünüşü alındı: {kind}, {} kural. Ctrl+Z geri alır.",
                        output.rules
                    ),
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                self.announce(Level::Error, format!("SLD alınamadı. {}", error.message));
            }
            _ => {}
        }
    }
}
