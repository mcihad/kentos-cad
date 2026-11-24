//! The terrain work Harita and Yol share (docs/adr/9199 §4.4): Arazi yüzeyleri and their edits, pafta eğrileri, yığın
//! hacmi, bulut, yüzey analizi, Eğimle kot ver, platforms, drainage, and the project tree with its work graph and Kesit
//! kontrolü. Both modules' command sets offer them (catalog/v1/modules.json), so they are the core's, not a module's:
//! their state is `App::terrain_work`, their messages `Message::TerrainWork`, their windows `Dialog::TerrainWork`.

use iced::{Element, Task};

use crate::app::{App, Message};

/// The shared terrain work's windows and panel.
#[derive(Default)]
pub struct TerrainWorkState {
    pub road_project: crate::road_project_panel::PanelState,
    /// Arazi yüzeyleri (terrain_window.rs, plan/road.md Y2.4).
    pub terrain_window: Option<crate::terrain_window::Window>,
    /// Platform (platform_window.rs, plan/road.md Y14.3).
    pub(crate) platform_window: Option<Box<crate::platform_window::Window>>,
    /// Buluttan arazi (terrain_cloud_window.rs, plan/road.md Y15.3).
    pub(crate) terrain_cloud_window: Option<Box<crate::terrain_cloud_window::Window>>,
    /// Drenaj (drainage_window.rs, plan/road.md Y16.3).
    pub(crate) drainage_window: Option<Box<crate::drainage_window::Window>>,
    /// Yüzey analizi (surface_analysis_window.rs, plan/road.md Y18.5).
    pub surface_analysis_window: Option<Box<crate::surface_analysis_window::Window>>,
    pub(crate) grade: crate::grade_window::State,
    pub(crate) project_graph: crate::project_graph_window::State,
    pub sheet_contours: crate::sheet_contours_window::State,
    pub(crate) section_check: crate::section_check_window::State,
    pub(crate) stockpile: crate::stockpile_window::State,
}

/// The shared terrain work's messages, `Message::TerrainWork`.
#[derive(Debug, Clone)]
pub enum Event {
    RoadProject(crate::road_project_panel::Event),
    /// An event of Arazi yüzeyleri (terrain_window.rs, plan/road.md Y2.4).
    Terrains(crate::terrain_window::Event),
    /// An event of Platform (platform_window.rs, plan/road.md Y14.3).
    Platforms(crate::platform_window::Event),
    /// An event of Buluttan arazi (terrain_cloud_window.rs, plan/road.md Y15.3).
    TerrainCloud(crate::terrain_cloud_window::Event),
    /// An event of Drenaj (drainage_window.rs, plan/road.md Y16.3).
    Drainage(crate::drainage_window::Event),
    /// An event of Yüzey analizi (surface_analysis_window.rs, plan/road.md Y18.5).
    SurfaceAnalysis(crate::surface_analysis_window::Event),
    Grade(crate::grade_window::Event),
    ProjectGraph(crate::project_graph_window::Event),
    SheetContours(crate::sheet_contours_window::Event),
    SectionCheck(crate::section_check_window::Event),
    Stockpile(crate::stockpile_window::Event),
}

/// The shared terrain work's windows, `Dialog::TerrainWork`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainDialogKind {
    /// Arazi yüzeyleri (terrain_window.rs, plan/road.md Y2.4); the window is `RoadUiState::terrain_window`.
    Terrains,
    /// Platform (platform_window.rs, plan/road.md Y14.3); the window is `RoadUiState::platform_window`.
    Platforms,
    /// Buluttan arazi (terrain_cloud_window.rs, plan/road.md Y15.3); the window is `RoadUiState::terrain_cloud_window`.
    TerrainCloud,
    /// Drenaj (drainage_window.rs, plan/road.md Y16.3); the window is `RoadUiState::drainage_window`.
    Drainage,
    /// Yüzey analizi (surface_analysis_window.rs, plan/road.md Y18.5); the window is `RoadUiState::surface_analysis_window`.
    SurfaceAnalysis,
    /// Eğimle kot ver (grade_window.rs, docs/adr/1290); its state is `RoadUiState::grade`.
    Grade,
    /// İş grafiği (project_graph_window.rs, docs/adr/1940); its state is `RoadUiState::project_graph`.
    ProjectGraph,
    /// Pafta eğrileri (sheet_contours_window.rs, docs/adr/1820); its state is `RoadUiState::sheet_contours`.
    SheetContours,
    /// Kesit kontrolü (section_check_window.rs, docs/adr/5120); its state is `RoadUiState::section_check`.
    SectionCheck,
    /// Stok hacmi (stockpile_window.rs, docs/adr/3140); its state is `RoadUiState::stockpile`.
    Stockpile,
}

impl App {
    /// A message of the shared terrain work; a window's own handler is in its file.
    pub(crate) fn terrain_work_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::RoadProject(e) => self.road_project_event(e),
            Event::Terrains(event) => self.terrain_event(event),
            Event::Platforms(event) => self.platform_event(event),
            Event::TerrainCloud(event) => self.terrain_cloud_event(event),
            Event::Drainage(event) => self.drainage_event(event),
            Event::SurfaceAnalysis(event) => self.surface_analysis_event(event),
            Event::Grade(event) => self.grade_event(event),
            Event::ProjectGraph(event) => self.project_graph_event(event),
            Event::SheetContours(event) => self.sheet_contours_event(event),
            Event::SectionCheck(event) => self.section_check_event(event),
            Event::Stockpile(event) => self.stockpile_event(event),
        }
    }

    /// A window of the shared terrain work.
    pub(crate) fn terrain_work_dialog_view(&self, kind: TerrainDialogKind) -> Element<'_, Message> {
        match kind {
            TerrainDialogKind::Terrains => self.terrain_view(),
            TerrainDialogKind::Platforms => self.platform_view(),
            TerrainDialogKind::TerrainCloud => self.terrain_cloud_view(),
            TerrainDialogKind::Drainage => self.drainage_view(),
            TerrainDialogKind::SurfaceAnalysis => self.surface_analysis_view(),
            TerrainDialogKind::Grade => self.grade_view(),
            TerrainDialogKind::ProjectGraph => self.project_graph_view(),
            TerrainDialogKind::SheetContours => self.sheet_contours_view(),
            TerrainDialogKind::SectionCheck => self.section_check_view(),
            TerrainDialogKind::Stockpile => self.stockpile_view(),
        }
    }
}
