//! The CBS module's own hub (docs/adr/9199 §4.12): Ağlar and Alt ağlar — the project's networks built on the network
//! thread and their windows — in the modules' state (`App::modules`, [`CbsState`]), their messages and windows as the
//! shell's (`Message::Module`, `Dialog::Module`). The GIS work every module shares (data layers, sources, processing,
//! rasters, point clouds, forms, topology …) is the core's hub (gis/state.rs).

use crate::cbs_ext::*;
use iced::{Element, Task};

use crate::app::{App, Message};

/// The CBS module's windows and what they keep.
#[derive(Default)]
pub struct CbsUiState {
    /// The project's networks as built on the network thread, and Ağlar's window.
    pub networks: crate::networks::State,
    /// Alt ağlar's window (subnetworks_window.rs, docs/adr/0557).
    pub subnetworks_window: Option<crate::subnetworks_window::Window>,
}

/// The CBS module's messages (`Message::Module`).
#[derive(Debug, Clone)]
pub enum Event {
    /// Ağ analizi: the networks' answers and Ağlar (networks/, docs/adr/0209).
    Networks(crate::networks::Event),
    /// Alt ağlar's window (subnetworks_window.rs, docs/adr/0557).
    Subnetworks(crate::subnetworks_window::Event),
}

/// The CBS module's windows (`Dialog::Module`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CbsDialogKind {
    /// Ağlar (networks/window.rs, docs/adr/0209 §10); the window is `CbsUiState::networks.window`.
    Networks,
    /// Alt ağlar (subnetworks_window.rs, docs/adr/0557); the window is `CbsUiState::subnetworks_window`.
    Subnetworks,
}

impl CbsDialogKind {
    /// Every window, in the order their numbers go (`ModuleDialog::kind`).
    pub const ALL: [CbsDialogKind; 2] = [CbsDialogKind::Networks, CbsDialogKind::Subnetworks];

    /// The window's number (`ModuleDialog::kind`), its place in [`Self::ALL`]; a match with no arm for the rest, so a
    /// new window does not build without its number.
    pub const fn number(self) -> u16 {
        match self {
            CbsDialogKind::Networks => 0,
            CbsDialogKind::Subnetworks => 1,
        }
    }

    /// The window of number `kind`; none for a number the module has not.
    pub fn of(kind: u16) -> Option<Self> {
        Self::ALL.get(usize::from(kind)).copied()
    }
}

/// The module's window `kind` as the shell's.
pub fn dialog(kind: CbsDialogKind) -> crate::app::Dialog {
    crate::app::Dialog::Module(crate::module_parts::ModuleDialog { module: kentos_launch::Module::Gis, kind: kind.number() })
}

impl From<Event> for Message {
    fn from(event: Event) -> Self {
        Message::Module(crate::module_parts::ModuleEvent::new(kentos_launch::Module::Gis, event))
    }
}

/// The module's state among the modules' (`App::modules`).
pub trait CbsState {
    fn cbs(&self) -> &CbsUiState;
    fn cbs_mut(&mut self) -> &mut CbsUiState;
}

impl CbsState for crate::module_parts::ModuleStates {
    fn cbs(&self) -> &CbsUiState {
        // Every program with the module's parts starts its state (ModuleStates::start).
        self.get::<CbsUiState>().expect("CBS modülünün durumu başlatılmamış")
    }

    fn cbs_mut(&mut self) -> &mut CbsUiState {
        self.get_mut::<CbsUiState>().expect("CBS modülünün durumu başlatılmamış")
    }
}

/// The module's `App` methods of this file, as an extension trait (docs/adr/9199 §3.5).
pub(crate) trait CbsStateApp {
    fn cbs_event(&mut self, event: Event) -> Task<Message>;
    fn cbs_dialog_view(&self, kind: CbsDialogKind) -> Element<'_, Message>;
}

impl CbsStateApp for App {
    /// A message of the module; a window's own handler is in its file.
    fn cbs_event(&mut self, event: Event) -> Task<Message>{
        match event {
            Event::Networks(event) => self.networks_event(event),
            Event::Subnetworks(event) => self.subnetworks_event(event),
        }
    }

    /// A window of the module.
    fn cbs_dialog_view(&self, kind: CbsDialogKind) -> Element<'_, Message>{
        match kind {
            CbsDialogKind::Networks => self.networks_view(),
            CbsDialogKind::Subnetworks => self.subnetworks_view(),
        }
    }
}
