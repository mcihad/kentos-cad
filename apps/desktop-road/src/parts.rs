//! The road design (Yol) module's parts the shell reaches (docs/adr/9199 §3.3): its messages, its 22 windows with their
//! usage-trace answers, its 37 commands (the editors, LandXML, IFC and glTF among them) and its alignments drawn over
//! the drawing. The terrain work it shares with Harita is the core's (terrain_work.rs).

use crate::road_ext::*;
use std::any::Any;
use std::sync::Arc;

use iced::{Element, Task};
use kentos_launch::Module;
use kentos_ui::theme::Tokens;

use crate::app::{App, Message, Panel};
use crate::module_parts::{ModuleEvent, ModuleMarks, ModuleParts};
use crate::road::{Event, RoadDialogKind as K, RoadUiState};
use crate::traces::Control;

/// The Yol module's parts.
pub struct Road;

impl ModuleParts for Road {
    fn module(&self) -> Module {
        Module::Road
    }

    fn profile(&self) -> &'static dyn crate::module_profile::ModuleProfile {
        &crate::road_profile::Road
    }

    fn state(&self) -> Box<dyn Any> {
        Box::new(RoadUiState::default())
    }

    fn event(&self, app: &mut App, event: ModuleEvent) -> Task<Message> {
        match event.take::<Event>() {
            Some(event) => app.road_event(event),
            None => Task::none(),
        }
    }

    fn command(&self, app: &mut App, id: &str) -> Option<Task<Message>> {
        Some(match id {
            "alignment.editor" => {
                app.open_alignments();
                Task::none()
            }
            "alignment.new" => {
                app.new_alignment();
                Task::none()
            }
            "profile.editor" => {
                app.open_profiles();
                Task::none()
            }
            "profile.new" => {
                app.new_profile();
                Task::none()
            }
            "section.editor" => {
                app.open_sections();
                Task::none()
            }
            "section.new" => {
                app.new_section_set();
                Task::none()
            }
            "section.template" => {
                app.new_template();
                Task::none()
            }
            "superelevation.editor" => {
                app.open_superelevations();
                Task::none()
            }
            "superelevation.new" => {
                app.new_superelevation();
                Task::none()
            }
            "corridor.editor" => {
                app.open_corridors();
                Task::none()
            }
            "corridor.new" => {
                app.new_corridor();
                Task::none()
            }
            "corridor.slopePlan" => {
                app.corridor_slope_plan();
                Task::none()
            }
            "volume.editor" => {
                app.open_volumes();
                Task::none()
            }
            "volume.new" => {
                app.new_volume();
                Task::none()
            }
            "volume.export" => {
                app.volume_sheet();
                Task::none()
            }
            "haul.editor" => {
                app.open_hauls();
                Task::none()
            }
            "haul.new" => {
                app.new_haul();
                Task::none()
            }
            "haul.plan" => {
                app.open_haul_plan();
                Task::none()
            }
            "stakeout.table" => {
                app.open_stakeout();
                Task::none()
            }
            "road.sheets" => {
                app.open_road_sheets();
                Task::none()
            }
            "road.views" => {
                app.open_road_views();
                Task::none()
            }
            "alignment.fromLine" => {
                app.open_road_alignment_fit();
                Task::none()
            }
            "profile.fromLine" => {
                app.open_road_profile_fit();
                Task::none()
            }
            "guardrail.editor" => {
                app.open_guardrails();
                Task::none()
            }
            "culvert.editor" => {
                app.open_culverts();
                Task::none()
            }
            "retainingWall.editor" => {
                app.open_retaining_walls();
                Task::none()
            }
            "swept.editor" => {
                app.open_swept();
                Task::none()
            }
            "profile.sight" => {
                app.open_sight();
                Task::none()
            }
            "corridor.water" => {
                app.open_platform_water();
                Task::none()
            }
            "payItems.editor" => {
                app.open_pay();
                Task::none()
            }
            "route.finder" => {
                app.open_route();
                Task::none()
            }
            "junction.editor" => {
                app.open_junctions();
                Task::none()
            }
            "landxml.export" => app.export_landxml(),
            "landxml.machine" => app.export_landxml_machine(),
            "landxml.importAlignments" => app.import_landxml_alignments(),
            "ifc.export" => app.export_ifc(),
            "gltf.export" => app.export_gltf(),
            // Tasarım denetimi (design_check_window.rs, plan/road.md Y21).
            "alignment.check" => {
                app.open_design_check();
                Task::none()
            }
            _ => return None,
        })
    }

    fn enabled(&self, _app: &App, _id: &str) -> Option<bool> {
        None
    }

    fn dialog_view<'a>(&self, app: &'a App, kind: u16) -> Element<'a, Message> {
        match K::of(kind) {
            Some(kind) => app.road_dialog_view(kind),
            None => iced::widget::Column::new().into(),
        }
    }

    fn dialog_title(&self, kind: u16) -> String {
        let Some(kind) = K::of(kind) else {
            return format!("Yol penceresi {kind}");
        };
        match kind {
            K::Alignments => crate::alignment_window::TITLE,
            K::Profiles => crate::profile_window::TITLE,
            K::Sections => crate::section_window::TITLE,
            K::Superelevations => crate::superelevation_window::TITLE,
            K::Corridors => crate::corridor_window::TITLE,
            K::Volumes => crate::volume_window::TITLE,
            K::Hauls => crate::haul_window::TITLE,
            K::Stakeout => crate::stakeout_window::TITLE,
            K::RoadSheets => crate::road_sheets_window::TITLE,
            K::RoadViews => crate::road_views_window::TITLE,
            K::RoadAlignmentFit => crate::road_alignment_fit_window::TITLE,
            K::RoadProfileFit => crate::road_profile_fit_window::TITLE,
            K::Guardrails => crate::guardrail_window::TITLE,
            K::Culverts => crate::culvert_window::TITLE,
            K::RetainingWalls => crate::retaining_wall_window::TITLE,
            K::Swept => crate::swept_window::TITLE,
            K::Sight => crate::sight_window::TITLE,
            K::PlatformWater => crate::platform_water_window::TITLE,
            K::HaulPlan => crate::haul_plan_window::TITLE,
            K::Pay => crate::pay_window::TITLE,
            K::Route => crate::route_window::TITLE,
            K::Junctions => crate::junction_window::TITLE,
            K::DesignCheck => crate::design_check_window::TITLE,
        }
        .to_owned()
    }

    fn dialog_control(
        &self,
        app: &App,
        kind: u16,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(kind) = K::of(kind) else {
            return Err(format!("Yol penceresi {kind} izden yanıtlanamıyor"));
        };
        match kind {
            K::Alignments => app.alignment_control(control),
            K::Profiles => app.profile_control(control),
            K::Sections => app.section_control(control),
            K::Superelevations => app.superelevation_control(control),
            K::Corridors => app.corridor_control(control),
            K::Volumes => app.volume_control(control),
            K::Hauls => app.haul_control(control),
            K::Stakeout => app.stakeout_control(control),
            K::RoadSheets => app.road_sheets_control(control),
            K::RoadViews => app.road_views_control(control),
            K::RoadAlignmentFit => app.road_alignment_fit_control(control),
            K::RoadProfileFit => app.road_profile_fit_control(control),
            K::Guardrails => app.guardrail_control(control),
            K::Culverts => app.culvert_control(control),
            K::RetainingWalls => app.retaining_wall_control(control),
            K::Swept => app.swept_control(control),
            K::Sight => app.sight_control(control),
            K::PlatformWater => app.platform_water_control(control),
            K::HaulPlan => app.haul_plan_control(control),
            K::Pay => app.pay_control(control),
            K::Route => app.route_control(control),
            K::Junctions => app.junction_control(control),
            K::DesignCheck => app.design_check_control(control),
        }
    }

    fn dialog_closed(&self, _app: &mut App, _kind: u16) {}

    fn document_changed(&self, _app: &mut App) {}

    fn unmark(&self, _app: &mut App) {}

    fn marks(&self, _app: &App) -> Vec<Arc<dyn ModuleMarks>> {
        Vec::new()
    }

    fn area_layers<'a>(&self, app: &'a App) -> Vec<Element<'a, Message>> {
        // A road project's alignments, km and curve points (alignment_layer.rs, plan/road.md Y3).
        let Some(doc) = &app.document else {
            return Vec::new();
        };
        let tokens = Tokens::of(&app.theme());
        crate::alignment_layer::layer(
            &doc.model,
            &app.viewport.camera,
            crate::alignment_layer::Colors {
                ink: crate::viewport::mark_colors(app.canvas()).fg,
                accent: tokens.accent,
                warning: tokens.warning,
            },
            &app.gpu_layers.alignment,
        )
        .into_iter()
        .collect()
    }

    fn panel_view<'a>(&self, _app: &'a App, _panel: Panel) -> Option<Element<'a, Message>> {
        None
    }

    fn started(&self, _app: &mut App) {}

    fn device(&self, _app: &mut App, _history: &std::path::Path) {}

    fn follow(&self, _app: &mut App) -> Task<Message> {
        Task::none()
    }

    fn escape_closes(&self, _kind: u16) -> bool {
        false
    }

    fn dialog_confirmed(&self, _app: &mut App, _kind: u16) {}

    fn checked(&self, _app: &App, _id: &str) -> Option<bool> {
        None
    }

    fn picked(&self, _app: &mut App, _at: Option<kentos_interaction::Vec2>) -> bool {
        false
    }

    fn preview(&self, _app: &App) -> Option<kentos_interaction::Preview> {
        None
    }

    fn bottom_tab_view<'a>(
        &self,
        _app: &'a App,
        _tab: crate::bottom::BottomTab,
    ) -> Option<Element<'a, Message>> {
        None
    }

    fn properties_ask(&self, _app: &mut App, _ask: crate::module_parts::PropertiesAsk<'_>) -> bool {
        false
    }

    fn building3d(&self) -> Option<&dyn crate::module_parts::Building3dParts> {
        None
    }

    fn drawing_shown(&self, _app: &mut App) {}
}
