//! IFC dışa aktar (bina) (plan/architecture-advanced.md M47, docs/adr/8047, `arch.ifcExport`): the drawing's building
//! written to an IFC 4.3 file where the user says, by `cad.arch.ifc`; the drawing does not change. What the file does
//! not hold (an object without a body) and a body that is not closed are said as the command's warnings. IFC dışa aktar
//! (bina, kesilmiş gövde) (`arch.ifcExportCut`, docs/adr/8071 §4) writes the bodies already cut (Reference View).

use std::path::PathBuf;

use iced::Task;
use kentos_commands::{ExportIfcBuilding, IfcView};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, arch_ifc};

use crate::app::{App, Message};

impl App {
    /// IFC dışa aktar (bina) from Dosya › Dışa aktar: where to write the file; `view` its bodies whole or cut.
    pub(crate) fn export_ifc_building(&mut self, view: IfcView) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        // EK-2's model file name from the künye (`260123_00_MM_XXXX_MD_XXX_NNN_01_000.ifc`), else the drawing's.
        let name = arch_ifc::file_name(&doc.model);
        let (title, then): (&str, fn(Option<PathBuf>) -> Message) = match view {
            IfcView::DesignTransfer => ("IFC dışa aktar (bina)", Message::IfcBuildingExport),
            IfcView::Reference => ("IFC dışa aktar (bina, kesilmiş gövde)", Message::IfcBuildingExportCut),
        };
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title(title)
                    .set_file_name(name)
                    .add_filter("IFC 4.3 (.ifc)", &["ifc"])
                    .save_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            then,
        )
    }

    /// The file chosen: the building whole (`cad.arch.ifc`; the dialog asked before writing over), its bodies as `view`
    /// asks.
    pub(crate) fn ifc_building_chosen(&mut self, path: Option<PathBuf>, view: IfcView) -> Task<Message> {
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
            view,
        };
        let result = arch_ifc::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            // The other classes it holds, those it has (docs/adr/8070 §9).
            let more: String = [
                (out.ramps, "rampa"),
                (out.chimneys, "baca"),
                (out.curtain_walls, "giydirme cephe"),
                (out.members, "eleman"),
                (out.fixtures, "donatı"),
                (out.grid_axes, "aks"),
                (out.coverings, "kaplama"),
            ]
            .iter()
            .filter(|(n, _)| *n > 0)
            .map(|(n, what)| format!(", {n} {what}"))
            .collect();
            let cut = if view == IfcView::Reference { " (gövdeler açıklıkları kesilmiş)" } else { "" };
            self.say(
                Level::Success,
                format!(
                    "IFC 4.3 yazıldı{cut}: {} kat, {} duvar, {} kapı, {} pencere, {} mahal, {} döşeme, {} kolon, {} kiriş, \
                     {} merdiven{more} ({}).",
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
