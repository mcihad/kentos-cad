//! The Harita module's `App` methods, every file's extension trait (docs/adr/9199 §3.5): a file of the module, its
//! parts and its tests take them all with `use crate::map_ext::*;`. A file's methods stay its own; this only names
//! them.

pub(crate) use crate::asbuilt_chart::AsbuiltChartApp;
pub(crate) use crate::asbuilt_review_form::AsbuiltReviewFormApp;
pub(crate) use crate::asbuilt_window::AsbuiltWindowApp;
pub(crate) use crate::baselines_window::BaselinesWindowApp;
pub(crate) use crate::cloud_place_window::CloudPlaceWindowApp;
pub(crate) use crate::cogo::CogoApp;
pub(crate) use crate::map::StateApp;
pub(crate) use crate::map_survey_panel::MapSurveyPanelApp;
pub(crate) use crate::map_work_panel::MapWorkPanelApp;
pub(crate) use crate::ortho_compare_window::OrthoCompareWindowApp;
pub(crate) use crate::ortho_network_section::OrthoNetworkSectionApp;
pub(crate) use crate::ortho_seamlines_window::OrthoSeamlinesWindowApp;
pub(crate) use crate::ortho_window::OrthoWindowApp;
pub(crate) use crate::overlap_window::OverlapWindowApp;
pub(crate) use crate::parcel_boundaries_marks::ParcelBoundariesMarksApp;
pub(crate) use crate::parcel_boundaries_window::ParcelBoundariesWindowApp;
pub(crate) use crate::parcel_register_window::ParcelRegisterWindowApp;
pub(crate) use crate::parcels_check_window::ParcelsCheckWindowApp;
pub(crate) use crate::pipes_check_window::PipesCheckWindowApp;
pub(crate) use crate::pipes_marks::PipesMarksApp;
pub(crate) use crate::readjustment_sheet::ReadjustmentSheetApp;
pub(crate) use crate::readjustment_window::ReadjustmentWindowApp;
pub(crate) use crate::register_window::RegisterWindowApp;
pub(crate) use crate::scan_network_window::ScanNetworkWindowApp;
pub(crate) use crate::survey_baseline_marks::SurveyBaselineMarksApp;
pub(crate) use crate::survey_network_window::SurveyNetworkWindowApp;
pub(crate) use crate::zoning_draft_window::ZoningDraftWindowApp;
pub(crate) use crate::zoning_marks::ZoningMarksApp;
pub(crate) use crate::zoning_symbols_window::ZoningSymbolsWindowApp;
pub(crate) use crate::zoning_window::ZoningWindowApp;
