//! IFC dışa aktar (bina) (plan/architecture-advanced.md M47, docs/adr/8047, `arch.ifcExport`): the drawing's building
//! written to an IFC 4.3 file where the user says, by `cad.arch.ifc`; the drawing does not change. What the file does
//! not hold (an object without a body) and a body that is not closed are said as the command's warnings. IFC dışa aktar
//! (bina, kesilmiş gövde) (`arch.ifcExportCut`, docs/adr/8071 §4) writes the bodies already cut (Reference View). A
//! project whose program has several buildings writes one file for each (docs/adr/8072 §2): the first where the user
//! says, the others beside it, each named by EK-2 with its building code. The fire compartments the national
//! regulation's package finds are written as zones of their rooms (docs/adr/8088).

use std::path::PathBuf;

use iced::Task;
use kentos_commands::{ExportIfcBuilding, IfcCompartment, IfcView};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, arch_ifc};

use crate::app::{App, Message};

/// The national regulation's package and its module (docs/adr/8078).
const REGULATION: &str = "mimari-mevzuat";
const MODULE: &str = "mimari_mevzuat.mimari";

impl App {
    /// IFC dışa aktar (bina) from Dosya › Dışa aktar: where to write the file; `view` its bodies whole or cut.
    pub(crate) fn export_ifc_building(&mut self, view: IfcView) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        // EK-2's model file name from the künye (`260123_00_MM_XXXX_MD_XXX_NNN_01_000.ifc`), else the drawing's; the
        // first building's when the program has several.
        let name = match buildings(&doc.model).first() {
            Some(id) => arch_ifc::file_name_of(&doc.model, id),
            None => arch_ifc::file_name(&doc.model),
        };
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
        // The fire compartments the standards package finds, written as zones of their rooms (docs/adr/8088).
        let compartments = self.fire_compartments();
        // Several buildings: one file each, the first where the user said, the others beside it (docs/adr/8072 §2).
        let plans: Vec<(String, String, PathBuf)> = match self.document.as_ref() {
            Some(doc) => {
                let ids = buildings(&doc.model);
                let folder = path.parent().map(PathBuf::from).unwrap_or_default();
                ids.iter()
                    .enumerate()
                    .map(|(k, id)| {
                        let name = doc
                            .model
                            .settings()
                            .arch
                            .as_ref()
                            .and_then(|a| a.program.buildings.iter().find(|b| &b.id == id))
                            .map_or_else(|| id.clone(), |b| b.name.clone());
                        let file = if k == 0 { path.clone() } else { folder.join(arch_ifc::file_name_of(&doc.model, id)) };
                        (id.clone(), name, file)
                    })
                    .collect()
            }
            None => return Task::none(),
        };
        if plans.len() > 1 {
            let mut written = Vec::new();
            let files: Vec<PathBuf> = plans.iter().map(|p| p.2.clone()).collect();
            for (id, name, file) in plans {
                let input = ExportIfcBuilding {
                    path: file.display().to_string(),
                    stamp: None,
                    overwrite: true,
                    view,
                    building: Some(id),
                    compartments: compartments.clone(),
                };
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                let result = arch_ifc::execute(&mut ExecutionContext::new(&mut doc.model), input);
                match self.answered(result) {
                    Some(out) => written.push(format!("{name}: {} kat, {} duvar ({})", out.storeys, out.walls, file.display())),
                    None => return Task::none(),
                }
            }
            self.say(Level::Success, format!("IFC 4.3, {} bina ayrı dosyalara yazıldı: {}.", written.len(), written.join("; ")));
            if let Some(first) = files.first() {
                self.offer_ifc_view(first.clone(), format!("{} bina IFC 4.3 olarak yazıldı", written.len()));
            }
            return Task::none();
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let input = ExportIfcBuilding {
            path: path.display().to_string(),
            stamp: None,
            overwrite: true,
            view,
            building: None,
            compartments,
        };
        let result = arch_ifc::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            // The other classes it holds, those it has (docs/adr/8070 §9).
            let more: String = [
                (out.compartments, "yangın kompartımanı"),
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
            self.offer_ifc_view(path, "IFC 4.3 yazıldı".to_owned());
        }
        Task::none()
    }

    /// The fire compartments as the national regulation's package finds them (catalog/standards/mimari-mevzuat,
    /// `kompartimanlar`; docs/adr/8088), for the file's zones. A package that does not answer, or a day its rules do not
    /// know, writes none and says why in the output.
    fn fire_compartments(&mut self) -> Option<Vec<IfcCompartment>> {
        let facts = self.arch_audit_regulation_facts()?;
        match crate::standards::call(REGULATION, MODULE, "kompartimanlar", vec![facts]) {
            Ok(answer) => {
                if let Some(note) = answer.get("note").and_then(|n| n.as_str()) {
                    self.output(format!("Yangın kompartımanları IFC'ye yazılmadı: {note}"));
                }
                // Each compartment's rooms: the command's input (its storeys and area are the package's words).
                let list = answer.get("compartments").and_then(|v| v.as_array())?;
                Some(
                    list.iter()
                        .map(|c| IfcCompartment {
                            rooms: c.get("rooms").and_then(|r| serde_json::from_value(r.clone()).ok()).unwrap_or_default(),
                        })
                        .collect(),
                )
            }
            Err(why) => {
                self.output(format!(
                    "Yangın kompartımanları IFC'ye yazılmadı: mevzuat kuralları çalışmadı ({why}). Ayarlar › Python'dan \
                     bir Python seçin."
                ));
                None
            }
        }
    }

    /// The file written, with Görüntüle: it opens in IFC modeli (ifc_viewer.rs, plan/ifc-uyum.md IFC-12).
    fn offer_ifc_view(&mut self, path: PathBuf, title: String) {
        let open = Message::IfcViewer(crate::ifc_viewer::Event::Open(path.clone()));
        self.toast(kentos_ui::widget::Toast::success(title).body(path.display().to_string()).action("Görüntüle", open));
    }
}

/// The program's buildings by their ids, in its order: one file each when there are several (docs/adr/8072 §2).
fn buildings(doc: &kentos_domain::Document) -> Vec<String> {
    doc.settings().arch.as_ref().map(|a| a.program.buildings.iter().map(|b| b.id.clone()).collect()).unwrap_or_default()
}
