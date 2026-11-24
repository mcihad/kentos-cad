//! The CBS module's parts the shell reaches (docs/adr/9199 §4.12): its profile, Ağlar and Alt ağlar with their
//! usage-trace answers, the network tools' questions answered on the network thread after every message, the networks
//! forgotten when another drawing comes on screen, and Ağ denetimi's mark on the drawing. The GIS work it shares with
//! the other modules (data layers, sources, processing, rasters …) is the core's.

use crate::cbs_ext::*;
use std::any::Any;
use std::sync::Arc;

use iced::widget::canvas;
use iced::{Element, Task};
use kentos_launch::Module;
use kentos_render_wgpu::Camera;
use kentos_ui::theme::Tokens;

use crate::app::{App, Message, Panel};
use crate::bottom::BottomTab;
use crate::cbs::{CbsDialogKind as K, CbsState, CbsUiState, Event};
use crate::module_parts::{Building3dParts, ModuleEvent, ModuleMarks, ModuleParts, PropertiesAsk};
use crate::traces::Control;
use crate::viewport::MarkColors;

/// The CBS module's parts.
pub struct Cbs;

impl ModuleParts for Cbs {
    fn module(&self) -> Module {
        Module::Gis
    }

    fn profile(&self) -> &'static dyn crate::module_profile::ModuleProfile {
        &crate::gis_profile::Gis
    }

    fn state(&self) -> Box<dyn Any> {
        Box::new(CbsUiState::default())
    }

    fn event(&self, app: &mut App, event: ModuleEvent) -> Task<Message> {
        match event.take::<Event>() {
            Some(event) => app.cbs_event(event),
            None => Task::none(),
        }
    }

    fn command(&self, app: &mut App, id: &str) -> Option<Task<Message>> {
        match id {
            // Ağlar (networks/, docs/adr/0209 §10).
            "network.manage" => Some(app.open_networks()),
            // Alt ağlar (subnetworks_window.rs, docs/adr/0557).
            "network.subnetworks" => {
                app.open_subnetworks();
                Some(Task::none())
            }
            _ => None,
        }
    }

    fn enabled(&self, _app: &App, _id: &str) -> Option<bool> {
        None
    }

    fn dialog_view<'a>(&self, app: &'a App, kind: u16) -> Element<'a, Message> {
        match K::of(kind) {
            Some(kind) => app.cbs_dialog_view(kind),
            None => iced::widget::Column::new().into(),
        }
    }

    fn dialog_title(&self, kind: u16) -> String {
        match K::of(kind) {
            Some(K::Networks) => crate::networks::window::TITLE.to_owned(),
            Some(K::Subnetworks) => crate::subnetworks_window::SUBNETWORKS_TITLE.to_owned(),
            None => format!("CBS penceresi {kind}"),
        }
    }

    fn dialog_control(&self, app: &App, kind: u16, control: Control<'_>) -> Result<Option<Message>, String> {
        match K::of(kind) {
            Some(K::Networks) => app.networks_control(control),
            Some(K::Subnetworks) => app.subnetworks_control(control),
            None => Err(format!("CBS penceresi {kind} izden yanıtlanamıyor")),
        }
    }

    fn dialog_closed(&self, _app: &mut App, _kind: u16) {}

    fn document_changed(&self, _app: &mut App) {}

    fn unmark(&self, _app: &mut App) {}

    fn marks(&self, app: &App) -> Vec<Arc<dyn ModuleMarks>> {
        // Ağ denetimi's finding over the drawing (networks/window.rs, docs/adr/0209 §12).
        match app.modules.cbs().networks.mark.clone() {
            Some(mark) => vec![Arc::new(NetworkMark(mark))],
            None => Vec::new(),
        }
    }

    fn area_layers<'a>(&self, _app: &'a App) -> Vec<Element<'a, Message>> {
        Vec::new()
    }

    fn panel_view<'a>(&self, _app: &'a App, _panel: Panel) -> Option<Element<'a, Message>> {
        None
    }

    fn started(&self, _app: &mut App) {}

    fn device(&self, _app: &mut App, _history: &std::path::Path) {}

    fn follow(&self, app: &mut App) -> Task<Message> {
        // The network tools' questions, each network built first when it must be (networks/mod.rs).
        app.network_tasks()
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

    fn bottom_tab_view<'a>(&self, _app: &'a App, _tab: BottomTab) -> Option<Element<'a, Message>> {
        None
    }

    fn properties_ask(&self, _app: &mut App, _ask: PropertiesAsk<'_>) -> bool {
        false
    }

    fn building3d(&self) -> Option<&dyn Building3dParts> {
        None
    }

    fn drawing_shown(&self, app: &mut App) {
        // The networks built from the drawing left go with it, and Denetle's mark (docs/adr/0209 §12).
        app.networks_reset();
    }
}

/// Ağ denetimi's finding, drawn as a topology finding is.
struct NetworkMark(crate::topology::ProblemMark);

impl ModuleMarks for NetworkMark {
    fn draw(&self, frame: &mut canvas::Frame, camera: &Camera, colors: &MarkColors, tokens: &Tokens) {
        crate::marks::problem(frame, &self.0, camera, colors, tokens.danger);
    }
}
