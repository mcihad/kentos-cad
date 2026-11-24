//! The road design (Yol) module's part of the shell (plan/argon-tavsiye.md ADIM 06, docs/adr/8054 §6): its
//! windows' and panels' state in `App::road`, its messages under `Message::Road`, its windows under
//! `Dialog::Road`. The terrain work it shares with Harita (surfaces, Eğimle kot ver, platforms, drainage, the project
//! tree) is the core's (terrain_work.rs, docs/adr/9199 §4.4). A new window of the module is added here and in its own
//! file; `app.rs` stays as it is.

use iced::{Element, Task};

use crate::app::{App, Message};

/// The road design (Yol) module's windows and panels.
#[derive(Default)]
pub struct RoadUiState {
    /// Güzergâh (alignment_window.rs, plan/road.md Y3).
    pub(crate) alignment_window: Option<crate::alignment_window::Window>,
    /// Boykesit (profile_window.rs, plan/road.md Y4).
    pub(crate) profile_window: Option<crate::profile_window::Window>,
    /// Enkesit (section_window.rs, plan/road.md Y5).
    pub(crate) section_window: Option<crate::section_window::Window>,
    /// Dever (superelevation_window.rs, plan/road.md Y6).
    pub(crate) superelevation_window: Option<Box<crate::superelevation_window::Window>>,
    /// Koridor (corridor_window.rs, plan/road.md Y7).
    pub(crate) corridor_window: Option<Box<crate::corridor_window::Window>>,
    /// Kübaj (volume_window.rs, plan/road.md Y8).
    pub(crate) volume_window: Option<Box<crate::volume_window::Window>>,
    /// Brükner (haul_window.rs, plan/road.md Y9).
    pub(crate) haul_window: Option<Box<crate::haul_window::Window>>,
    /// Aplikasyon cetveli (stakeout_window.rs, plan/road.md Y10).
    pub(crate) stakeout_window: Option<Box<crate::stakeout_window::Window>>,
    /// Yol paftaları (road_sheets_window.rs, plan/road.md Y10.5).
    pub(crate) road_sheets_window: Option<Box<crate::road_sheets_window::Window>>,
    pub(crate) road_views_window: Option<Box<crate::road_views_window::Window>>,
    /// Çizgiden aks (road_alignment_fit_window.rs, plan/road.md Y10.6).
    pub(crate) road_alignment_fit_window: Option<Box<crate::road_alignment_fit_window::Window>>,
    /// Çizimden boykesit (road_profile_fit_window.rs, plan/road.md Y10.6b).
    pub(crate) road_profile_fit_window: Option<Box<crate::road_profile_fit_window::Window>>,
    /// Otokorkuluk (guardrail_window.rs, plan/road.md Y11.1).
    pub(crate) guardrail_window: Option<Box<crate::guardrail_window::Window>>,
    /// Menfez (culvert_window.rs, plan/road.md Y11.2).
    pub(crate) culvert_window: Option<Box<crate::culvert_window::Window>>,
    /// İstinat duvarı (retaining_wall_window.rs, plan/road.md Y11.3).
    pub(crate) retaining_wall_window: Option<Box<crate::retaining_wall_window::Window>>,
    /// Araç izi (swept_window.rs, plan/road.md Y17.3).
    pub(crate) swept_window: Option<Box<crate::swept_window::Window>>,
    /// Görüş mesafesi (sight_window.rs, plan/road.md Y20).
    pub(crate) sight_window: Option<Box<crate::sight_window::Window>>,
    /// Platform suyu (platform_water_window.rs, plan/road.md Y22).
    pub(crate) platform_water_window: Option<Box<crate::platform_water_window::Window>>,
    /// Taşıma planı (haul_plan_window.rs, plan/road.md Y23).
    pub(crate) haul_plan_window: Option<Box<crate::haul_plan_window::Window>>,
    /// Poz metrajı (pay_window.rs, plan/road.md Y11.4).
    pub(crate) pay_window: Option<Box<crate::pay_window::Window>>,
    /// Ön güzergâh (route_window.rs, plan/road.md Y12.1).
    pub(crate) route_window: Option<Box<crate::route_window::Window>>,
    /// Kavşak (junction_window.rs, plan/road.md Y12.2).
    pub(crate) junction_window: Option<Box<crate::junction_window::Window>>,
}

/// The road design (Yol) module's messages, `Message::Road`.
#[derive(Debug, Clone)]
pub enum Event {
    /// An event of Güzergâh (alignment_window.rs, plan/road.md Y3).
    Alignments(crate::alignment_window::Event),
    /// An event of Boykesit (profile_window.rs, plan/road.md Y4).
    Profiles(crate::profile_window::Event),
    /// An event of Enkesit (section_window.rs, plan/road.md Y5).
    Sections(crate::section_window::Event),
    /// An event of Dever (superelevation_window.rs, plan/road.md Y6).
    Superelevations(crate::superelevation_window::Event),
    /// An event of Koridor (corridor_window.rs, plan/road.md Y7).
    Corridors(crate::corridor_window::Event),
    /// An event of Kübaj (volume_window.rs, plan/road.md Y8).
    Volumes(crate::volume_window::Event),
    /// An event of Brükner (haul_window.rs, plan/road.md Y9).
    Hauls(crate::haul_window::Event),
    /// An event of Aplikasyon cetveli (stakeout_window.rs, plan/road.md Y10).
    Stakeout(crate::stakeout_window::Event),
    /// An event of Yol paftaları (road_sheets_window.rs, plan/road.md Y10.5).
    RoadSheets(crate::road_sheets_window::Event),
    RoadViews(crate::road_views_window::Event),
    /// An event of Çizgiden aks (road_alignment_fit_window.rs, plan/road.md Y10.6).
    RoadAlignmentFit(crate::road_alignment_fit_window::Event),
    /// An event of Çizimden boykesit (road_profile_fit_window.rs, plan/road.md Y10.6b).
    RoadProfileFit(crate::road_profile_fit_window::Event),
    /// An event of Otokorkuluk (guardrail_window.rs, plan/road.md Y11.1).
    Guardrails(crate::guardrail_window::Event),
    /// An event of Menfez (culvert_window.rs, plan/road.md Y11.2).
    Culverts(crate::culvert_window::Event),
    /// An event of İstinat duvarı (retaining_wall_window.rs, plan/road.md Y11.3).
    RetainingWalls(crate::retaining_wall_window::Event),
    /// An event of Araç izi (swept_window.rs, plan/road.md Y17.3).
    Swept(crate::swept_window::Event),
    /// An event of Görüş mesafesi (sight_window.rs, plan/road.md Y20).
    Sight(crate::sight_window::Event),
    /// An event of Platform suyu (platform_water_window.rs, plan/road.md Y22).
    PlatformWater(crate::platform_water_window::Event),
    /// An event of Taşıma planı (haul_plan_window.rs, plan/road.md Y23).
    HaulPlan(crate::haul_plan_window::Event),
    /// An event of Poz metrajı (pay_window.rs, plan/road.md Y11.4).
    Pay(crate::pay_window::Event),
    /// An event of Ön güzergâh (route_window.rs, plan/road.md Y12.1).
    Route(crate::route_window::Event),
    /// An event of Kavşak (junction_window.rs, plan/road.md Y12.2).
    Junctions(crate::junction_window::Event),
}

/// The road design (Yol) module's windows, `Dialog::Road`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadDialogKind {
    /// Güzergâh (alignment_window.rs, plan/road.md Y3); the window is `RoadUiState::alignment_window`.
    Alignments,
    /// Boykesit (profile_window.rs, plan/road.md Y4); the window is `RoadUiState::profile_window`.
    Profiles,
    /// Enkesit (section_window.rs, plan/road.md Y5); the window is `RoadUiState::section_window`.
    Sections,
    /// Dever (superelevation_window.rs, plan/road.md Y6); the window is `RoadUiState::superelevation_window`.
    Superelevations,
    /// Koridor (corridor_window.rs, plan/road.md Y7); the window is `RoadUiState::corridor_window`.
    Corridors,
    /// Kübaj (volume_window.rs, plan/road.md Y8); the window is `RoadUiState::volume_window`.
    Volumes,
    /// Brükner (haul_window.rs, plan/road.md Y9); the window is `RoadUiState::haul_window`.
    Hauls,
    /// Aplikasyon cetveli (stakeout_window.rs, plan/road.md Y10); the window is `RoadUiState::stakeout_window`.
    Stakeout,
    /// Yol paftaları (road_sheets_window.rs, plan/road.md Y10.5); the window is `RoadUiState::road_sheets_window`.
    RoadSheets,
    /// Görünüşler (road_views_window.rs, plan/road.md Y13.4); the window is `RoadUiState::road_views_window`.
    RoadViews,
    /// Çizgiden aks (road_alignment_fit_window.rs, plan/road.md Y10.6); the window is `RoadUiState::road_alignment_fit_window`.
    RoadAlignmentFit,
    /// Çizimden boykesit (road_profile_fit_window.rs, plan/road.md Y10.6b); the window is `RoadUiState::road_profile_fit_window`.
    RoadProfileFit,
    /// Otokorkuluk (guardrail_window.rs, plan/road.md Y11.1); the window is `RoadUiState::guardrail_window`.
    Guardrails,
    /// Menfez (culvert_window.rs, plan/road.md Y11.2); the window is `RoadUiState::culvert_window`.
    Culverts,
    /// İstinat duvarı (retaining_wall_window.rs, plan/road.md Y11.3); the window is `RoadUiState::retaining_wall_window`.
    RetainingWalls,
    /// Araç izi (swept_window.rs, plan/road.md Y17.3); the window is `RoadUiState::swept_window`.
    Swept,
    /// Görüş mesafesi (sight_window.rs, plan/road.md Y20); the window is `RoadUiState::sight_window`.
    Sight,
    /// Platform suyu (platform_water_window.rs, plan/road.md Y22); the window is `RoadUiState::platform_water_window`.
    PlatformWater,
    /// Taşıma planı (haul_plan_window.rs, plan/road.md Y23); the window is `RoadUiState::haul_plan_window`.
    HaulPlan,
    /// Poz metrajı (pay_window.rs, plan/road.md Y11.4); the window is `RoadUiState::pay_window`.
    Pay,
    /// Ön güzergâh (route_window.rs, plan/road.md Y12.1); the window is `RoadUiState::route_window`.
    Route,
    /// Kavşak (junction_window.rs, plan/road.md Y12.2); the window is `RoadUiState::junction_window`.
    Junctions,
}

impl App {
    /// A message of the module (`Message::Road`); a window's own handler is in its file.
    pub(crate) fn road_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Alignments(event) => self.alignment_event(event),
            Event::Profiles(event) => self.profile_event(event),
            Event::Sections(event) => self.section_event(event),
            Event::Superelevations(event) => self.superelevation_event(event),
            Event::Corridors(event) => self.corridor_event(event),
            Event::Volumes(event) => self.volume_event(event),
            Event::Hauls(event) => self.haul_event(event),
            Event::Stakeout(event) => self.stakeout_event(event),
            Event::RoadSheets(event) => self.road_sheets_event(event),
            Event::RoadViews(event) => self.road_views_event(event),
            Event::RoadAlignmentFit(event) => self.road_alignment_fit_event(event),
            Event::RoadProfileFit(event) => self.road_profile_fit_event(event),
            Event::Guardrails(event) => self.guardrail_event(event),
            Event::Culverts(event) => self.culvert_event(event),
            Event::RetainingWalls(event) => self.retaining_wall_event(event),
            Event::Swept(event) => self.swept_event(event),
            Event::Sight(event) => self.sight_event(event),
            Event::PlatformWater(event) => self.platform_water_event(event),
            Event::HaulPlan(event) => self.haul_plan_event(event),
            Event::Pay(event) => self.pay_event(event),
            Event::Route(event) => self.route_event(event),
            Event::Junctions(event) => self.junction_event(event),
        }
    }

    /// A window of the module (`Dialog::Road`).
    pub(crate) fn road_dialog_view(&self, kind: RoadDialogKind) -> Element<'_, Message> {
        match kind {
            RoadDialogKind::Alignments => self.alignment_view(),
            RoadDialogKind::Profiles => self.road_profile_view(),
            RoadDialogKind::Sections => self.road_section_view(),
            RoadDialogKind::Superelevations => self.road_superelevation_view(),
            RoadDialogKind::Corridors => self.road_corridor_view(),
            RoadDialogKind::Volumes => self.road_volume_view(),
            RoadDialogKind::Hauls => self.road_haul_view(),
            RoadDialogKind::Stakeout => self.road_stakeout_view(),
            RoadDialogKind::RoadSheets => self.road_sheets_view(),
            RoadDialogKind::RoadViews => self.road_views_view(),
            RoadDialogKind::RoadAlignmentFit => self.road_alignment_fit_view(),
            RoadDialogKind::RoadProfileFit => self.road_profile_fit_view(),
            RoadDialogKind::Guardrails => self.guardrail_view(),
            RoadDialogKind::Culverts => self.culvert_view(),
            RoadDialogKind::RetainingWalls => self.retaining_wall_view(),
            RoadDialogKind::Swept => self.swept_view(),
            RoadDialogKind::Sight => self.sight_view(),
            RoadDialogKind::PlatformWater => self.platform_water_view(),
            RoadDialogKind::HaulPlan => self.haul_plan_view(),
            RoadDialogKind::Pay => self.pay_view(),
            RoadDialogKind::Route => self.route_view(),
            RoadDialogKind::Junctions => self.junction_view(),
        }
    }
}
