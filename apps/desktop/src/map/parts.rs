//! The surveying, cadastre and zoning (Harita) module's parts the shell reaches (docs/adr/9199 §3.3): its messages,
//! windows and their usage-trace answers, its ribbon commands, its marks and layers over the drawing, its two dock
//! panels (Harita işi, GNSS arşivi) and what it does when the drawing changes or the marks are cleared.

use std::any::Any;
use std::sync::Arc;

use iced::widget::canvas;
use iced::{Element, Task};
use kentos_launch::Module;
use kentos_render_wgpu::Camera;
use kentos_ui::theme::Tokens;

use crate::app::{App, Message, Panel};
use crate::map::{Event, MapDialogKind as K, MapState, MapUiState};
use crate::module_parts::{ModuleEvent, ModuleMarks, ModuleParts};
use crate::traces::Control;
use crate::viewport::MarkColors;

/// The Harita module's parts.
pub(crate) struct Map;

impl ModuleParts for Map {
    fn module(&self) -> Module {
        Module::Map
    }

    fn state(&self) -> Box<dyn Any> {
        Box::new(MapUiState::default())
    }

    fn event(&self, app: &mut App, event: ModuleEvent) -> Task<Message> {
        match event.take::<Event>() {
            Some(event) => app.map_event(event),
            None => Task::none(),
        }
    }

    fn command(&self, app: &mut App, id: &str) -> Option<Task<Message>> {
        Some(match id {
            "survey.baselines" => app.open_baselines(),
            "parcels.check" => app.open_parcels_check(),
            "pipes.check" => app.open_pipes_check(),
            "asbuilt.check" => app.open_asbuilt(),
            "map.survey.adjustment" => app.open_survey_network(),
            "zoning.check" => app.open_zoning(),
            "readjustment.distribute" => app.open_readjustment(),
            "raster.accuracy" => app.open_ortho(),
            "raster.compare" => app.open_ortho_compare(),
            "raster.seamlines" => app.open_ortho_seamlines(),
            "pointcloud.overlap" => app.open_overlap(),
            "pointcloud.register" => app.open_register(),
            "pointcloud.network" => app.open_scan_network(),
            "pointcloud.place" => app.open_cloud_place(),
            // Harita işi on a contextual tab (map_work_panel.rs, docs/adr/1201 §3).
            "map.workSource" => {
                app.map_work_show_source();
                Task::none()
            }
            "map.workRelease" => {
                app.map_work_release();
                Task::none()
            }
            // GNSS arşivi (map_survey_panel.rs, docs/adr/1340 §6).
            "map.survey" => {
                app.show_map_survey();
                Task::none()
            }
            "map.survey.import" => app.map_survey_event(crate::map_survey_panel::Event::Import),
            "map.survey.select" => {
                app.select_accepted_points();
                Task::none()
            }
            // Kayıtlı ölçüler (docs/adr/0180): the check's window, and the selection's recorded values from the drawing.
            "cogo.check" => {
                app.open_cogo_check();
                Task::none()
            }
            "cogo.update" => {
                app.cogo_update();
                Task::none()
            }
            _ => return None,
        })
    }

    fn enabled(&self, app: &App, id: &str) -> Option<bool> {
        match id {
            "map.workSource" | "map.workRelease" => Some(app.map_work_available(id)),
            _ => None,
        }
    }

    fn dialog_view<'a>(&self, app: &'a App, kind: u16) -> Element<'a, Message> {
        match K::of(kind) {
            Some(kind) => app.map_dialog_view(kind),
            None => iced::widget::Column::new().into(),
        }
    }

    fn dialog_title(&self, kind: u16) -> String {
        let Some(kind) = K::of(kind) else {
            return format!("Harita penceresi {kind}");
        };
        match kind {
            K::Baselines => crate::baselines_window::TITLE,
            K::ParcelsCheck => crate::parcels_check_window::TITLE,
            K::ParcelBoundaries => crate::parcel_boundaries_window::TITLE,
            K::ParcelRegister => crate::parcel_register_window::TITLE,
            K::PipesCheck => crate::pipes_check_window::TITLE,
            K::Asbuilt => crate::asbuilt_window::TITLE,
            K::SurveyNetwork => crate::survey_network_window::TITLE,
            K::Zoning => crate::zoning_window::TITLE,
            K::ZoningSymbols => crate::zoning_symbols_window::TITLE,
            K::ZoningDraft => crate::zoning_draft_window::TITLE,
            K::Readjustment => crate::readjustment_window::TITLE,
            K::Ortho => crate::ortho_window::TITLE,
            K::OrthoCompare => crate::ortho_compare_window::TITLE,
            K::OrthoSeamlines => crate::ortho_seamlines_window::TITLE,
            K::Overlap => crate::overlap_window::TITLE,
            K::Register => crate::register_window::TITLE,
            K::ScanNetwork => crate::scan_network_window::TITLE,
            K::CloudPlace => crate::cloud_place_window::TITLE,
            K::Cogo => crate::cogo::COGO_TITLE,
        }
        .to_owned()
    }

    fn dialog_control(&self, app: &App, kind: u16, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(kind) = K::of(kind) else {
            return Err(format!("Harita penceresi {kind} izden yanıtlanamıyor"));
        };
        match kind {
            K::Baselines => app.baselines_control(control),
            K::ParcelsCheck => app.parcels_check_control(control),
            K::ParcelBoundaries => app.parcel_boundaries_control(control),
            K::ParcelRegister => app.parcel_register_control(control),
            K::PipesCheck => app.pipes_check_control(control),
            K::Asbuilt => app.asbuilt_control(control),
            K::SurveyNetwork => app.survey_network_control(control),
            K::Zoning => app.zoning_control(control),
            K::ZoningSymbols => app.zoning_symbols_control(control),
            K::ZoningDraft => app.zoning_draft_control(control),
            K::Readjustment => app.readjustment_control(control),
            K::Ortho => app.ortho_control(control),
            K::OrthoCompare => app.ortho_compare_control(control),
            K::OrthoSeamlines => app.ortho_seamlines_control(control),
            K::Overlap => app.overlap_control(control),
            K::Register => app.register_control(control),
            K::ScanNetwork => app.scan_network_control(control),
            K::CloudPlace => app.cloud_place_control(control),
            K::Cogo => app.cogo_control(control),
        }
    }

    fn dialog_closed(&self, app: &mut App, kind: u16) {
        // Kayıtlı ölçüleri denetle keeps its window apart from the dialog (cogo.rs).
        if K::of(kind) == Some(K::Cogo) {
            app.modules.map_mut().cogo = None;
        }
    }

    fn document_changed(&self, app: &mut App) {
        // Boru hattı denetimi open: its answer read again from the network as it is now (docs/adr/4360).
        if app.dialog == Some(crate::map::dialog(K::PipesCheck)) {
            app.pipes_check_run();
        }
    }

    fn unmark(&self, app: &mut App) {
        let map = app.modules.map_mut();
        map.parcel_graph = None;
        map.zoning_shown_marks = None;
        map.pipes_shown_marks = None;
        map.baseline_shown = None;
    }

    fn marks(&self, app: &App) -> Vec<Arc<dyn ModuleMarks>> {
        let mut out: Vec<Arc<dyn ModuleMarks>> = Vec::new();
        // The order the drawing had them in: İmar planı denetimi, GNSS arşivi's baseline, Boru hattı denetimi, Sınır
        // grafiği.
        if let Some(m) = app.zoning_marks() {
            out.push(Arc::new(Zoning(m)));
        }
        if let Some(m) = app.baseline_marks() {
            out.push(Arc::new(Baseline(m)));
        }
        if let Some(m) = app.pipes_marks() {
            out.push(Arc::new(Pipes(m)));
        }
        if let Some(m) = app.parcel_graph_marks() {
            out.push(Arc::new(Graph(m)));
        }
        out
    }

    fn area_layers<'a>(&self, app: &'a App) -> Vec<Element<'a, Message>> {
        // Harita işi's approved shapes of the objects whose geometry changed (approved_layer.rs, docs/adr/4480), while
        // Değişenler's selection stands.
        crate::approved_layer::layer(
            app.modules.map().map_work.approved.as_ref(),
            app.selection.version(),
            &app.viewport.camera,
            Tokens::of(&app.theme()).warning,
        )
        .into_iter()
        .collect()
    }

    fn panel_view<'a>(&self, app: &'a App, panel: Panel) -> Option<Element<'a, Message>> {
        match panel {
            // Harita işi: a map project's work structure (map_work_panel.rs, docs/adr/1200 §4).
            Panel::MapWork => Some(app.map_work_view()),
            // GNSS arşivi: a map project's arazi ölçümleri (map_survey_panel.rs, docs/adr/1340 §6).
            Panel::MapSurvey => Some(app.map_survey_view()),
            _ => None,
        }
    }
}

/// İmar planı denetimi's road overlaps, hatched (docs/adr/3040).
struct Zoning(Arc<crate::zoning_marks::ZoningMarks>);
/// GNSS arşivi's baseline drawn between its ends (docs/adr/3320).
struct Baseline(Arc<crate::survey_baseline_marks::BaselineMarks>);
/// Boru hattı denetimi's crossings and near passes (docs/adr/3160).
struct Pipes(Arc<crate::pipes_marks::PipesMarks>);
/// Sınır grafiği left on the drawing (docs/adr/2220).
struct Graph(Arc<crate::parcel_boundaries_marks::GraphMarks>);

impl ModuleMarks for Zoning {
    fn draw(&self, frame: &mut canvas::Frame, camera: &Camera, colors: &MarkColors, tokens: &Tokens) {
        crate::zoning_marks::draw(frame, &self.0, camera, colors, tokens);
    }
}

impl ModuleMarks for Baseline {
    fn draw(&self, frame: &mut canvas::Frame, camera: &Camera, colors: &MarkColors, tokens: &Tokens) {
        crate::survey_baseline_marks::draw(frame, &self.0, camera, colors, tokens);
    }
}

impl ModuleMarks for Pipes {
    fn draw(&self, frame: &mut canvas::Frame, camera: &Camera, colors: &MarkColors, tokens: &Tokens) {
        crate::pipes_marks::draw(frame, &self.0, camera, colors, tokens);
    }
}

impl ModuleMarks for Graph {
    fn draw(&self, frame: &mut canvas::Frame, camera: &Camera, colors: &MarkColors, tokens: &Tokens) {
        crate::parcel_boundaries_marks::draw(frame, &self.0, camera, colors, tokens.accent, tokens.danger);
    }
}
