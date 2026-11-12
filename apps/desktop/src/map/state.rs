//! The surveying, cadastre and zoning (Harita) module's part of the shell (plan/argon-tavsiye.md ADIM 06, docs/adr/8054 §6): its
//! windows' and panels' state in `App::map`, its messages under `Message::Map`, its windows under
//! `Dialog::Map`. A new window of the module is added here and in its own file; `app.rs` stays as it is.

use iced::{Element, Task};

use crate::app::{App, Message};

/// The surveying, cadastre and zoning (Harita) module's windows and panels.
pub(crate) struct MapUiState {
    pub(crate) map_work: crate::map_work_panel::PanelState,
    pub(crate) map_survey: crate::map_survey_panel::PanelState,
    pub(crate) baselines: crate::baselines_window::State,
    pub(crate) parcels_check: crate::parcels_check_window::State,
    pub(crate) parcel_boundaries: crate::parcel_boundaries_window::State,
    pub(crate) parcel_register: crate::parcel_register_window::State,
    /// Sınır grafiği left on the drawing by Çizimde göster (docs/adr/2220), until İşareti kaldır.
    pub(crate) parcel_graph: Option<crate::parcel_boundaries_marks::Shown>,
    /// İmar planı denetimi's road overlaps hatched on the drawing (docs/adr/3040).
    pub(crate) zoning_shown_marks: Option<crate::zoning_marks::Shown>,
    /// Boru hattı denetimi's crossings left on the drawing (pipes_marks.rs, docs/adr/3160).
    pub(crate) pipes_shown_marks: Option<crate::pipes_marks::Shown>,
    /// GNSS arşivi's baseline drawn on the map (docs/adr/3320).
    pub(crate) baseline_shown: Option<crate::survey_baseline_marks::Shown>,
    pub(crate) pipes_check: crate::pipes_check_window::State,
    pub(crate) asbuilt: crate::asbuilt_window::State,
    pub(crate) survey_network: crate::survey_network_window::State,
    pub(crate) zoning: crate::zoning_window::State,
    pub(crate) zoning_symbols: crate::zoning_symbols_window::State,
    pub(crate) zoning_draft: crate::zoning_draft_window::State,
    pub(crate) readjustment: crate::readjustment_window::State,
    pub(crate) ortho: crate::ortho_window::State,
    pub(crate) ortho_compare: crate::ortho_compare_window::State,
    pub(crate) ortho_seamlines: crate::ortho_seamlines_window::State,
    pub(crate) overlap: crate::overlap_window::State,
    pub(crate) register: crate::register_window::State,
    pub(crate) scan_network: crate::scan_network_window::State,
    pub(crate) cloud_place: crate::cloud_place_window::State,
    /// Kayıtlı ölçüleri denetle's window (cogo.rs, docs/adr/0180).
    pub(crate) cogo: Option<crate::cogo::Window>,
}

/// The surveying, cadastre and zoning (Harita) module's messages, `Message::Map`.
#[derive(Debug, Clone)]
pub(crate) enum Event {
    MapWork(crate::map_work_panel::Event),
    MapSurvey(crate::map_survey_panel::Event),
    Baselines(crate::baselines_window::Event),
    ParcelsCheck(crate::parcels_check_window::Event),
    ParcelBoundaries(crate::parcel_boundaries_window::Event),
    ParcelRegister(crate::parcel_register_window::Event),
    PipesCheck(crate::pipes_check_window::Event),
    Asbuilt(crate::asbuilt_window::Event),
    SurveyNetwork(crate::survey_network_window::Event),
    Zoning(crate::zoning_window::Event),
    ZoningSymbols(crate::zoning_symbols_window::Event),
    ZoningDraft(crate::zoning_draft_window::Event),
    Readjustment(crate::readjustment_window::Event),
    Ortho(crate::ortho_window::Event),
    OrthoCompare(crate::ortho_compare_window::Event),
    OrthoSeamlines(crate::ortho_seamlines_window::Event),
    Overlap(crate::overlap_window::Event),
    Register(crate::register_window::Event),
    ScanNetwork(crate::scan_network_window::Event),
    CloudPlace(crate::cloud_place_window::Event),
    /// Kayıtlı ölçüleri denetle's window (cogo.rs).
    Cogo(crate::cogo::Event),
    /// Nokta hesapla: a construction over the running command (docs/adr/0083).
    PointCalc(kentos_interaction::point_calc::CalcKind),
    /// Harita sistemi chosen (docs/adr/0583 §3).
    MapCrs(crate::map_crs::Choose),
}

/// The surveying, cadastre and zoning (Harita) module's windows, `Dialog::Map`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MapDialogKind {
    /// GNSS baz ağı (baselines_window.rs, docs/adr/1380); its state is `MapUiState::baselines`.
    Baselines,
    /// Parsel alan denetimi (parcels_check_window.rs, docs/adr/1400); its state is `MapUiState::parcels_check`.
    ParcelsCheck,
    /// Sınır grafiği (parcel_boundaries_window.rs, docs/adr/2000); its state is `MapUiState::parcel_boundaries`.
    ParcelBoundaries,
    /// Parsel kayıtları (parcel_register_window.rs, docs/adr/4220); its state is `MapUiState::parcel_register`.
    ParcelRegister,
    /// Boru hattı denetimi (pipes_check_window.rs, docs/adr/1640); its state is `MapUiState::pipes_check`.
    PipesCheck,
    /// Uygulama kontrolü (asbuilt_window.rs, docs/adr/1700); its state is `MapUiState::asbuilt`.
    Asbuilt,
    /// GNSS ağ dengelemesi (survey_network_window.rs, docs/adr/1740); its state is `MapUiState::survey_network`.
    SurveyNetwork,
    /// İmar planı denetimi (zoning_window.rs, docs/adr/1420); its state is `MapUiState::zoning`.
    Zoning,
    /// Sembolü karardan ver (zoning_symbols_window.rs, docs/adr/4240); its state is `MapUiState::zoning_symbols`.
    ZoningSymbols,
    /// Plan değişikliği taslağı (zoning_draft_window.rs, docs/adr/5040); its state is `MapUiState::zoning_draft`.
    ZoningDraft,
    /// Düzenleme dağıtım taslağı (readjustment_window.rs, docs/adr/1460); its state is `MapUiState::readjustment`.
    Readjustment,
    /// Ortofoto doğruluğu (ortho_window.rs, docs/adr/1480); its state is `MapUiState::ortho`.
    Ortho,
    /// Ortofoto karşılaştırması (ortho_compare_window.rs, docs/adr/1780); its state is `MapUiState::ortho_compare`.
    OrthoCompare,
    /// Dikiş çokgenleri (ortho_seamlines_window.rs, docs/adr/4700); its state is `MapUiState::ortho_seamlines`.
    OrthoSeamlines,
    /// Tarama uyumu (overlap_window.rs, docs/adr/1500); its state is `MapUiState::overlap`.
    Overlap,
    /// Tarama kaydı (register_window.rs, docs/adr/1800); its state is `MapUiState::register`.
    Register,
    /// Tarama ağı dengelemesi (scan_network_window.rs, docs/adr/4720); its state is `MapUiState::scan_network`.
    ScanNetwork,
    /// Bulutu konumla (cloud_place_window.rs, docs/adr/5100); its state is `MapUiState::cloud_place`.
    CloudPlace,
    /// Kayıtlı ölçüleri denetle (cogo.rs, docs/adr/0180); the window is `MapUiState::cogo`.
    Cogo,
}

impl App {
    /// A message of the module (`Message::Map`); a window's own handler is in its file.
    pub(crate) fn map_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::PointCalc(kind) => return self.start_point_calc(kind),
            Event::MapWork(e) => return self.map_work_event(e),
            Event::MapSurvey(e) => return self.map_survey_event(e),
            Event::Baselines(event) => return self.baselines_event(event),
            Event::ParcelsCheck(event) => return self.parcels_check_event(event),
            Event::ParcelBoundaries(event) => return self.parcel_boundaries_event(event),
            Event::ParcelRegister(event) => return self.parcel_register_event(event),
            Event::PipesCheck(event) => return self.pipes_check_event(event),
            Event::Asbuilt(event) => return self.asbuilt_event(event),
            Event::SurveyNetwork(event) => return self.survey_network_event(event),
            Event::Zoning(event) => return self.zoning_event(event),
            Event::ZoningSymbols(event) => return self.zoning_symbols_event(event),
            Event::ZoningDraft(event) => return self.zoning_draft_event(event),
            Event::Readjustment(event) => return self.readjustment_event(event),
            Event::Ortho(event) => return self.ortho_event(event),
            Event::OrthoCompare(event) => return self.ortho_compare_event(event),
            Event::OrthoSeamlines(event) => return self.ortho_seamlines_event(event),
            Event::Overlap(event) => return self.overlap_event(event),
            Event::Register(event) => return self.register_event(event),
            Event::ScanNetwork(event) => return self.scan_network_event(event),
            Event::CloudPlace(event) => return self.cloud_place_event(event),
            Event::Cogo(event) => return self.cogo_event(event),
            Event::MapCrs(choose) => self.choose_map(choose),
        }
        Task::none()
    }

    /// A window of the module (`Dialog::Map`).
    pub(crate) fn map_dialog_view(&self, kind: MapDialogKind) -> Element<'_, Message> {
        match kind {
            MapDialogKind::Baselines => self.baselines_view(),
            MapDialogKind::ParcelsCheck => self.parcels_check_view(),
            MapDialogKind::ParcelBoundaries => self.parcel_boundaries_view(),
            MapDialogKind::ParcelRegister => self.parcel_register_view(),
            MapDialogKind::PipesCheck => self.pipes_check_view(),
            MapDialogKind::Asbuilt => self.asbuilt_view(),
            MapDialogKind::SurveyNetwork => self.survey_network_view(),
            MapDialogKind::Zoning => self.zoning_view(),
            MapDialogKind::ZoningSymbols => self.zoning_symbols_view(),
            MapDialogKind::ZoningDraft => self.zoning_draft_view(),
            MapDialogKind::Readjustment => self.readjustment_view(),
            MapDialogKind::Ortho => self.ortho_view(),
            MapDialogKind::OrthoCompare => self.ortho_compare_view(),
            MapDialogKind::OrthoSeamlines => self.ortho_seamlines_view(),
            MapDialogKind::Overlap => self.overlap_view(),
            MapDialogKind::Register => self.register_view(),
            MapDialogKind::ScanNetwork => self.scan_network_view(),
            MapDialogKind::CloudPlace => self.cloud_place_view(),
            MapDialogKind::Cogo => self.cogo_view(),
        }
    }
}
