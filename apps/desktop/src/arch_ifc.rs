//! IFC dışa aktar (bina) (plan/architecture-advanced.md M47, docs/adr/8047, `arch.ifcExport`): the drawing's building
//! written to an IFC 4.3 file where the user says, by `cad.arch.ifc`; the drawing does not change. What the file does
//! not hold (a ramp, an object without a body) is said as the command's warnings.

use std::path::PathBuf;

use iced::Task;
use kentos_commands::ExportIfcBuilding;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, arch_ifc};

use crate::app::{App, Message};

impl App {
    /// IFC dışa aktar (bina) from Dosya › Dışa aktar: where to write the file.
    pub(crate) fn export_ifc_building(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let name = format!("{}.ifc", doc.model.name());
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("IFC dışa aktar (bina)")
                    .set_file_name(name)
                    .add_filter("IFC 4.3 (.ifc)", &["ifc"])
                    .save_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            Message::IfcBuildingExport,
        )
    }

    /// The file chosen: the building whole (`cad.arch.ifc`; the dialog asked before writing over).
    pub(crate) fn ifc_building_chosen(&mut self, path: Option<PathBuf>) -> Task<Message> {
        let Some(mut path) = path else {
            return Task::none();
        };
        if path.extension().is_none() {
            path.set_extension("ifc");
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let input = ExportIfcBuilding {
            path: path.display().to_string(),
            stamp: None,
            overwrite: true,
        };
        let result = arch_ifc::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(
                Level::Success,
                format!(
                    "IFC 4.3 yazıldı: {} kat, {} duvar, {} kapı, {} pencere, {} mahal, {} döşeme, {} kolon, {} kiriş, \
                     {} merdiven ({}).",
                    out.storeys,
                    out.walls,
                    out.doors,
                    out.windows,
                    out.spaces,
                    out.slabs,
                    out.columns,
                    out.beams,
                    out.stairs,
                    path.display()
                ),
            );
        }
        Task::none()
    }
}
