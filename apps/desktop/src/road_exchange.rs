//! LandXML ver (plan/road.md Y10.2, docs/adr/0908 §6, `landxml.export`): the road project's terrain surfaces,
//! alignments and their profiles written to a LandXML 1.2 file where the user says, by `file.export.landxml`; the
//! drawing does not change.

use std::path::PathBuf;

use iced::Task;
use kentos_commands::ExportLandxml;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_landxml};

use crate::app::{App, Message};

impl App {
    /// LandXML ver from the ribbon: where to write the file.
    pub(crate) fn export_landxml(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("LandXML ver yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return Task::none();
        }
        let name = format!("{}.xml", doc.model.name());
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("LandXML ver")
                    .set_file_name(name)
                    .add_filter("LandXML (.xml)", &["xml"])
                    .save_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            Message::LandxmlExport,
        )
    }

    /// The file chosen: every surface and alignment with its profiles written (the dialog asked before writing over).
    pub(crate) fn landxml_chosen(&mut self, path: Option<PathBuf>) -> Task<Message> {
        let Some(mut path) = path else { return Task::none() };
        if path.extension().is_none() {
            path.set_extension("xml");
        }
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = ExportLandxml {
            path: path.display().to_string(),
            surfaces: None,
            alignments: None,
            without_profiles: false,
            stamp: None,
            overwrite: true,
        };
        let result = road_landxml::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            let profiles: usize = out.alignments.iter().map(|a| a.profiles.len()).sum();
            self.say(
                Level::Success,
                format!(
                    "LandXML yazıldı: {} yüzey, {} aks, {profiles} boykesit ({}).",
                    out.surfaces.len(),
                    out.alignments.len(),
                    path.display()
                ),
            );
        }
        Task::none()
    }
}
