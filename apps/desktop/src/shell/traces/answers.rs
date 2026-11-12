//! What a trace can do in an open window (the `dialog` step,
//! fixtures/interaction/README.md): read its title, and find the message a
//! control of it sends, by the words the window shows, as the web's player
//! finds the control in the page and uses it with the mouse and the keyboard.
//! A field is filled over what it held, a check box set, a button pressed; a
//! pressed button that is off sends nothing, as a click on it would. Only a
//! window that answers here can be played; another says so.

use crate::app::{App, Dialog, Message};
use crate::arch;
use crate::gis;
use crate::map;
use crate::road;

/// A control of the open window, by its words.
#[derive(Clone, Copy, Debug)]
pub enum Control<'a> {
    /// The field labelled so, its text replaced by this one.
    Fill(&'a str, &'a str),
    /// The check box with these words, set on or off.
    Check(&'a str, bool),
    /// The button with these words.
    Press(&'a str),
    /// The item with these words of the list named so (the `panel` step's `pick`).
    Pick(&'a str, &'a str),
    /// The table's header with these words, pressed.
    Sort(&'a str),
    /// The n-th row of the table, 1 first, pressed.
    Row(usize),
    /// A key of the box that has the keyboard (`Enter`, `Esc`).
    Key(&'a str),
}

impl std::fmt::Display for Control<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Control::Fill(label, _) => write!(f, "“{label}” alanı"),
            Control::Check(words, _) => write!(f, "“{words}” kutusu"),
            Control::Press(words) => write!(f, "“{words}” düğmesi"),
            Control::Pick(list, item) => write!(f, "“{item}” öğesi (“{list}” listesi)"),
            Control::Sort(header) => write!(f, "“{header}” başlığı"),
            Control::Row(n) => write!(f, "{n}. sonuç satırı"),
            Control::Key(key) => write!(f, "{key} tuşu"),
        }
    }
}

impl App {
    /// The open window's title, as it shows it; a window no trace answers
    /// yet goes by its name in the code.
    pub fn dialog_title(&self) -> Option<String> {
        self.dialog.map(|d| match d {
            Dialog::Exchange => self.exchange_trace_title(),
            Dialog::BlockDefine => crate::blocks::DEFINE_TITLE.to_owned(),
            Dialog::BlockAttributes => crate::block_attributes::ATTRIBUTES_TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::AttributeValues) => {
                crate::attribute_values::VALUES_TITLE.to_owned()
            }
            Dialog::BlockCard => crate::block_card::CARD_TITLE.to_owned(),
            Dialog::BlockParameterGroups => crate::block_parameter_groups::TITLE.to_owned(),
            Dialog::BlockTypeValues => crate::block_type_values::TITLE.to_owned(),
            Dialog::BlockSchedule => crate::block_schedule::SCHEDULE_TITLE.to_owned(),
            Dialog::FindReplace => crate::find_replace::FIND_TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::LayerMerge) => {
                crate::layer_merge::MERGE_TITLE.to_owned()
            }
            Dialog::DrawingStandard => crate::drawing_standard::STANDARD_TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::DataProfile) => {
                crate::data_profile::PROFILE_TITLE.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::Subnetworks) => {
                crate::subnetworks_window::SUBNETWORKS_TITLE.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::SourceStatus) => {
                crate::source_status::STATUS_TITLE.to_owned()
            }
            Dialog::LineTypes => crate::line_types_window::LINE_TYPES_TITLE.to_owned(),
            Dialog::Parameters => crate::parameters_window::PARAMETERS_TITLE.to_owned(),
            Dialog::HatchPatterns => crate::hatch_patterns_window::HATCH_PATTERNS_TITLE.to_owned(),
            Dialog::StyleImpact => crate::style_impact::IMPACT_TITLE.to_owned(),
            Dialog::BlockReview => crate::block_editor::REVIEW_TITLE.to_owned(),
            Dialog::BlockReplace => crate::blocks_panel::REPLACE_TITLE.to_owned(),
            Dialog::BlockFind => crate::blocks_panel::FIND_TITLE.to_owned(),
            Dialog::ArrayFind => self.array_find_title().to_owned(),
            Dialog::Gis(gis::GisDialogKind::LayerFields) => {
                crate::layer_fields::FIELDS_TITLE.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::TopologyRules) => {
                crate::topology::rules::RULES_TITLE.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::Networks) => crate::networks::window::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::AddressData) => crate::address::data::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::AddressProviders) => {
                crate::address::providers::TITLE.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::TimeLayer) => crate::temporal::layer::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::Sensors) => crate::temporal::sensors::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::LayerFilter) => crate::layer_filters::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::FieldsWrite) => {
                crate::features::write::TITLE_CALC.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::LayerInfo) => crate::layer_info::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::DatasetInfo) => crate::dataset_window::TITLE.to_owned(),
            Dialog::Attachment => self.attachment_title().to_owned(),
            Dialog::PhotoViewer => crate::attachments::VIEWER_TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::Labelling) => crate::labelling::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::Scenario) => self.scenario_title(),
            Dialog::LayerStates => crate::layer_states::STATES_TITLE.to_owned(),
            Dialog::Views => crate::views::VIEWS_TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::Forms) => crate::forms::designer::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::FormEntry) => crate::forms::entry::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::Relations) => crate::forms::relations::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::Joins) => crate::joins_window::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::EventLinks) => {
                crate::event_links_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::Terrains) => crate::terrain_window::TITLE.to_owned(),
            Dialog::Road(road::RoadDialogKind::Alignments) => {
                crate::alignment_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::Profiles) => crate::profile_window::TITLE.to_owned(),
            Dialog::Road(road::RoadDialogKind::Sections) => crate::section_window::TITLE.to_owned(),
            Dialog::Road(road::RoadDialogKind::Superelevations) => {
                crate::superelevation_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::Corridors) => {
                crate::corridor_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::Volumes) => crate::volume_window::TITLE.to_owned(),
            Dialog::Road(road::RoadDialogKind::Hauls) => crate::haul_window::TITLE.to_owned(),
            Dialog::Road(road::RoadDialogKind::Stakeout) => {
                crate::stakeout_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::RoadSheets) => {
                crate::road_sheets_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::RoadViews) => {
                crate::road_views_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::RoadAlignmentFit) => {
                crate::road_alignment_fit_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::RoadProfileFit) => {
                crate::road_profile_fit_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::Guardrails) => {
                crate::guardrail_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::Culverts) => crate::culvert_window::TITLE.to_owned(),
            Dialog::Road(road::RoadDialogKind::RetainingWalls) => {
                crate::retaining_wall_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::Platforms) => {
                crate::platform_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::TerrainCloud) => {
                crate::terrain_cloud_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::Drainage) => {
                crate::drainage_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::Swept) => crate::swept_window::TITLE.to_owned(),
            Dialog::Road(road::RoadDialogKind::SurfaceAnalysis) => {
                crate::surface_analysis_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::Sight) => crate::sight_window::TITLE.to_owned(),
            Dialog::Propose => crate::propose_window::TITLE.to_owned(),
            Dialog::Arch(arch::ArchDialogKind::DesignCheck) => {
                crate::design_check_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::PlatformWater) => {
                crate::platform_water_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::HaulPlan) => {
                crate::haul_plan_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::Pay) => crate::pay_window::TITLE.to_owned(),
            Dialog::Road(road::RoadDialogKind::Route) => crate::route_window::TITLE.to_owned(),
            Dialog::Road(road::RoadDialogKind::Junctions) => {
                crate::junction_window::TITLE.to_owned()
            }
            Dialog::Scene3d => crate::scene3d_window::TITLE.to_owned(),
            Dialog::Arch(arch::ArchDialogKind::Program) => crate::program_window::TITLE.to_owned(),
            Dialog::Arch(arch::ArchDialogKind::SpaceMerge) => {
                crate::quantities::MERGE_TITLE.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::TableStyles) => {
                crate::forms::table_styles::TITLE.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::Subtypes) => crate::forms::subtypes::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::Contingents) => {
                crate::forms::contingents::TITLE.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::LayerRules) => {
                crate::forms::layer_rules::TITLE.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::LayerIndexes) => {
                crate::forms::layer_indexes::TITLE.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::Domains) => crate::forms::domains::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::FormAudit) => crate::forms::audit::TITLE.to_owned(),
            Dialog::AnnotationStyles => self.annotation_styles_title().to_owned(),
            Dialog::Arch(arch::ArchDialogKind::WallStyles) => crate::wall_styles::TITLE.to_owned(),
            Dialog::TableStyleDefs => crate::table_style_window::TITLE.to_owned(),
            Dialog::Arch(arch::ArchDialogKind::WallJoins) => crate::wall_joins::TITLE.to_owned(),
            Dialog::Arch(arch::ArchDialogKind::OpeningTypes) => {
                crate::opening_types_window::TITLE.to_owned()
            }
            Dialog::Arch(arch::ArchDialogKind::Materials) => {
                crate::materials_window::TITLE.to_owned()
            }
            Dialog::Arch(arch::ArchDialogKind::RoomModules) => {
                crate::room_modules_window::TITLE.to_owned()
            }
            Dialog::Arch(arch::ArchDialogKind::DesignOptions) => {
                crate::design_options_window::TITLE.to_owned()
            }
            Dialog::Arch(arch::ArchDialogKind::TagsCheck) => crate::tags_check::TITLE.to_owned(),
            Dialog::Arch(arch::ArchDialogKind::Schedule) => {
                crate::schedule_window::TITLE.to_owned()
            }
            Dialog::Arch(arch::ArchDialogKind::ScheduleExport) => {
                crate::schedule_window::EXPORT_TITLE.to_owned()
            }
            Dialog::Arch(arch::ArchDialogKind::ViewRefs) => crate::viewmark_refs::TITLE.to_owned(),
            Dialog::SheetSet => crate::sheet_set::TITLE.to_owned(),
            Dialog::Xrefs => crate::xrefs_window::TITLE.to_owned(),
            Dialog::Flatten => crate::flatten_window::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::FeaturesCode) => {
                crate::features_window::TITLE.to_owned()
            }
            Dialog::Map(map::MapDialogKind::Baselines) => crate::baselines_window::TITLE.to_owned(),
            Dialog::Road(road::RoadDialogKind::Grade) => crate::grade_window::TITLE.to_owned(),
            Dialog::CleanReport => crate::clean_report_window::TITLE.to_owned(),
            Dialog::Map(map::MapDialogKind::ParcelsCheck) => {
                crate::parcels_check_window::TITLE.to_owned()
            }
            Dialog::Map(map::MapDialogKind::ParcelBoundaries) => {
                crate::parcel_boundaries_window::TITLE.to_owned()
            }
            Dialog::Map(map::MapDialogKind::ParcelRegister) => {
                crate::parcel_register_window::TITLE.to_owned()
            }
            Dialog::Map(map::MapDialogKind::PipesCheck) => {
                crate::pipes_check_window::TITLE.to_owned()
            }
            Dialog::ExchangeCheck => crate::exchange_check_window::TITLE.to_owned(),
            Dialog::Map(map::MapDialogKind::Asbuilt) => crate::asbuilt_window::TITLE.to_owned(),
            Dialog::Road(road::RoadDialogKind::ProjectGraph) => {
                crate::project_graph_window::TITLE.to_owned()
            }
            Dialog::Map(map::MapDialogKind::SurveyNetwork) => {
                crate::survey_network_window::TITLE.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::FeaturesReconcile) => {
                crate::features_reconcile_window::TITLE.to_owned()
            }
            Dialog::Map(map::MapDialogKind::Zoning) => crate::zoning_window::TITLE.to_owned(),
            Dialog::Map(map::MapDialogKind::ZoningSymbols) => {
                crate::zoning_symbols_window::TITLE.to_owned()
            }
            Dialog::Map(map::MapDialogKind::ZoningDraft) => {
                crate::zoning_draft_window::TITLE.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::Impact) => crate::impact_window::TITLE.to_owned(),
            Dialog::Map(map::MapDialogKind::Readjustment) => {
                crate::readjustment_window::TITLE.to_owned()
            }
            Dialog::Map(map::MapDialogKind::Ortho) => crate::ortho_window::TITLE.to_owned(),
            Dialog::Map(map::MapDialogKind::OrthoCompare) => {
                crate::ortho_compare_window::TITLE.to_owned()
            }
            Dialog::Map(map::MapDialogKind::OrthoSeamlines) => {
                crate::ortho_seamlines_window::TITLE.to_owned()
            }
            Dialog::Map(map::MapDialogKind::Overlap) => crate::overlap_window::TITLE.to_owned(),
            Dialog::Map(map::MapDialogKind::Register) => crate::register_window::TITLE.to_owned(),
            Dialog::Map(map::MapDialogKind::ScanNetwork) => {
                crate::scan_network_window::TITLE.to_owned()
            }
            Dialog::Map(map::MapDialogKind::CloudPlace) => {
                crate::cloud_place_window::TITLE.to_owned()
            }
            Dialog::CatalogReport => crate::catalog_report_window::TITLE.to_owned(),
            Dialog::Road(road::RoadDialogKind::SheetContours) => {
                crate::sheet_contours_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::SectionCheck) => {
                crate::section_check_window::TITLE.to_owned()
            }
            Dialog::Road(road::RoadDialogKind::Stockpile) => {
                crate::stockpile_window::TITLE.to_owned()
            }
            Dialog::XrefAttach => crate::xrefs_window::ATTACH_TITLE.to_owned(),
            Dialog::Arch(arch::ArchDialogKind::Quantities) => crate::quantities::TITLE.to_owned(),
            Dialog::LayerPurge => crate::layer_purge::PURGE_TITLE.to_owned(),
            Dialog::TakeFrom => crate::drawing_exchange::TAKE_TITLE.to_owned(),
            Dialog::LayerList => crate::layer_list::LIST_TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::DataCompare) => {
                crate::data_compare::COMPARE_TITLE.to_owned()
            }
            Dialog::TableInsert => crate::tables::insert::INSERT_TITLE.to_owned(),
            Dialog::TableEditor => crate::tables::editor::EDITOR_TITLE.to_owned(),
            Dialog::Extract => crate::tables::extract::TITLE.to_owned(),
            Dialog::SheetDetails => crate::sheet_details_window::TITLE.to_owned(),
            Dialog::PlotStyles => crate::plot_styles_window::TITLE.to_owned(),
            Dialog::Map(map::MapDialogKind::Cogo) => crate::cogo::COGO_TITLE.to_owned(),
            Dialog::Project if self.asking_type() => crate::project::TYPE_TITLE.to_owned(),
            Dialog::LayerStyle => self.layer_style_title(),
            other => format!("{other:?}"),
        })
    }

    /// The message `control` of the open window sends; `None` when it sends
    /// nothing (a button that is off, a box already so).
    pub fn dialog_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        match self.dialog {
            Some(Dialog::Exchange) => self.exchange_trace_control(control),
            Some(Dialog::BlockDefine) => self.block_define_control(control),
            Some(Dialog::BlockAttributes) => self.block_attributes_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::AttributeValues)) => {
                self.attribute_values_control(control)
            }
            Some(Dialog::FindReplace) => self.find_replace_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::LayerMerge)) => self.layer_merge_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::LayerTranslate)) => {
                self.layer_translate_control(control)
            }
            Some(Dialog::DrawingStandard) => self.drawing_standard_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::DataProfile)) => {
                self.data_profile_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::Subnetworks)) => self.subnetworks_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::SourceStatus)) => {
                self.source_status_control(control)
            }
            Some(Dialog::LineTypes) => self.line_types_control(control),
            Some(Dialog::Parameters) => self.parameters_control(control),
            Some(Dialog::HatchPatterns) => self.hatch_patterns_control(control),
            Some(Dialog::StyleImpact) => self.style_impact_control(control),
            Some(Dialog::BlockReview) => match control {
                Control::Press("Kapat") => Ok(Some(Message::BlockEditor(crate::block_editor::Event::CloseReview))),
                other => Err(format!("Blok etkisi penceresinde {other} yok")),
            },
            Some(Dialog::BlockFind) => self.block_find_control(control),
            Some(Dialog::ArrayFind) => self.array_find_control(control),
            Some(Dialog::BlockReplace) => match control {
                Control::Check("Yalnız seçili yerleştirmeler", on) => Ok(Some(Message::BlocksPanel(crate::blocks_panel::Event::Replace(crate::blocks_panel::ReplaceEvent::Scope(on))))),
                Control::Press("Değiştir") => Ok(Some(Message::BlocksPanel(crate::blocks_panel::Event::Replace(crate::blocks_panel::ReplaceEvent::Apply)))),
                Control::Press("Vazgeç") => Ok(Some(Message::BlocksPanel(crate::blocks_panel::Event::Replace(crate::blocks_panel::ReplaceEvent::Close)))),
                other => Err(format!("Blok değiştirme penceresinde {other} yok")),
            },
            Some(Dialog::Gis(gis::GisDialogKind::LayerFields)) => {
                self.layer_fields_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::TopologyRules)) => {
                self.topology_rules_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::Networks)) => self.networks_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::AddressData)) => {
                self.address_data_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::AddressProviders)) => {
                self.address_providers_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::TimeLayer)) => self.time_layer_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::LayerFilter)) => {
                self.layer_filter_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::LayerInfo)) => self.layer_info_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::DatasetInfo)) => {
                self.dataset_info_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::Labelling)) => self.labelling_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::Scenario)) => self.scenario_control(control),
            Some(Dialog::LayerStates) => self.layer_states_control(control),
            Some(Dialog::Views) => self.views_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::Forms)) => self.forms_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::FormEntry)) => self.form_entry_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::Relations)) => self.relations_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::Joins)) => self.joins_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::EventLinks)) => self.event_links_control(control),
            Some(Dialog::Road(road::RoadDialogKind::Terrains)) => self.terrain_control(control),
            Some(Dialog::Road(road::RoadDialogKind::Alignments)) => self.alignment_control(control),
            Some(Dialog::Road(road::RoadDialogKind::Profiles)) => self.profile_control(control),
            Some(Dialog::Road(road::RoadDialogKind::Sections)) => self.section_control(control),
            Some(Dialog::Road(road::RoadDialogKind::Superelevations)) => {
                self.superelevation_control(control)
            }
            Some(Dialog::Road(road::RoadDialogKind::Corridors)) => self.corridor_control(control),
            Some(Dialog::Road(road::RoadDialogKind::Volumes)) => self.volume_control(control),
            Some(Dialog::Road(road::RoadDialogKind::Hauls)) => self.haul_control(control),
            Some(Dialog::Road(road::RoadDialogKind::Stakeout)) => self.stakeout_control(control),
            Some(Dialog::Road(road::RoadDialogKind::RoadSheets)) => {
                self.road_sheets_control(control)
            }
            Some(Dialog::Road(road::RoadDialogKind::RoadViews)) => self.road_views_control(control),
            Some(Dialog::Road(road::RoadDialogKind::RoadAlignmentFit)) => {
                self.road_alignment_fit_control(control)
            }
            Some(Dialog::Road(road::RoadDialogKind::RoadProfileFit)) => {
                self.road_profile_fit_control(control)
            }
            Some(Dialog::Road(road::RoadDialogKind::Guardrails)) => self.guardrail_control(control),
            Some(Dialog::Road(road::RoadDialogKind::Culverts)) => self.culvert_control(control),
            Some(Dialog::Road(road::RoadDialogKind::RetainingWalls)) => {
                self.retaining_wall_control(control)
            }
            Some(Dialog::Road(road::RoadDialogKind::Platforms)) => self.platform_control(control),
            Some(Dialog::Road(road::RoadDialogKind::TerrainCloud)) => {
                self.terrain_cloud_control(control)
            }
            Some(Dialog::Road(road::RoadDialogKind::Drainage)) => self.drainage_control(control),
            Some(Dialog::Road(road::RoadDialogKind::Swept)) => self.swept_control(control),
            Some(Dialog::Road(road::RoadDialogKind::SurfaceAnalysis)) => {
                self.surface_analysis_control(control)
            }
            Some(Dialog::Road(road::RoadDialogKind::Sight)) => self.sight_control(control),
            Some(Dialog::Propose) => self.propose_control(control),
            Some(Dialog::Arch(arch::ArchDialogKind::DesignCheck)) => {
                self.design_check_control(control)
            }
            Some(Dialog::Road(road::RoadDialogKind::PlatformWater)) => {
                self.platform_water_control(control)
            }
            Some(Dialog::Road(road::RoadDialogKind::HaulPlan)) => self.haul_plan_control(control),
            Some(Dialog::Road(road::RoadDialogKind::Pay)) => self.pay_control(control),
            Some(Dialog::Road(road::RoadDialogKind::Route)) => self.route_control(control),
            Some(Dialog::Road(road::RoadDialogKind::Junctions)) => self.junction_control(control),
            Some(Dialog::Scene3d) => self.scene3d_control(control),
            Some(Dialog::Arch(arch::ArchDialogKind::Program)) => self.program_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::TableStyles)) => {
                self.table_styles_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::Subtypes)) => self.subtypes_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::Contingents)) => self.contingents_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::LayerRules)) => self.layer_rules_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::LayerIndexes)) => {
                self.layer_indexes_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::Domains)) => self.domains_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::FormAudit)) => self.form_audit_control(control),
            Some(Dialog::AnnotationStyles) => self.annotation_styles_control(control),
            Some(Dialog::LayerPurge) => self.layer_purge_control(control),
            Some(Dialog::TakeFrom) => self.take_from_control(control),
            Some(Dialog::LayerList) => self.layer_list_control(control),
            Some(Dialog::Arch(arch::ArchDialogKind::WallJoins)) => self.wall_joins_control(control),
            Some(Dialog::Arch(arch::ArchDialogKind::OpeningTypes)) => {
                self.opening_types_control(control)
            }
            Some(Dialog::Arch(arch::ArchDialogKind::Materials)) => self.materials_control(control),
            Some(Dialog::Arch(arch::ArchDialogKind::RoomModules)) => {
                self.room_modules_control(control)
            }
            Some(Dialog::Arch(arch::ArchDialogKind::DesignOptions)) => {
                self.design_options_control(control)
            }
            Some(Dialog::Arch(arch::ArchDialogKind::TagsCheck)) => self.tags_check_control(control),
            Some(Dialog::Arch(arch::ArchDialogKind::ViewRefs)) => self.view_refs_control(control),
            Some(Dialog::SheetSet) => self.sheet_set_control(control),
            Some(Dialog::Xrefs | Dialog::XrefAttach) => self.xrefs_control(control),
            Some(Dialog::Flatten) => self.flatten_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::FeaturesCode)) => {
                self.features_code_control(control)
            }
            Some(Dialog::Map(map::MapDialogKind::Baselines)) => self.baselines_control(control),
            Some(Dialog::Road(road::RoadDialogKind::Grade)) => self.grade_control(control),
            Some(Dialog::CleanReport) => self.clean_report_control(control),
            Some(Dialog::Map(map::MapDialogKind::ParcelsCheck)) => {
                self.parcels_check_control(control)
            }
            Some(Dialog::Map(map::MapDialogKind::ParcelBoundaries)) => {
                self.parcel_boundaries_control(control)
            }
            Some(Dialog::Map(map::MapDialogKind::ParcelRegister)) => {
                self.parcel_register_control(control)
            }
            Some(Dialog::Map(map::MapDialogKind::PipesCheck)) => self.pipes_check_control(control),
            Some(Dialog::ExchangeCheck) => self.exchange_check_control(control),
            Some(Dialog::Map(map::MapDialogKind::Asbuilt)) => self.asbuilt_control(control),
            Some(Dialog::Road(road::RoadDialogKind::ProjectGraph)) => {
                self.project_graph_control(control)
            }
            Some(Dialog::Map(map::MapDialogKind::SurveyNetwork)) => {
                self.survey_network_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::FeaturesReconcile)) => {
                self.features_reconcile_control(control)
            }
            Some(Dialog::Map(map::MapDialogKind::Zoning)) => self.zoning_control(control),
            Some(Dialog::Map(map::MapDialogKind::ZoningSymbols)) => {
                self.zoning_symbols_control(control)
            }
            Some(Dialog::Map(map::MapDialogKind::ZoningDraft)) => {
                self.zoning_draft_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::Impact)) => self.impact_control(control),
            Some(Dialog::Map(map::MapDialogKind::Readjustment)) => {
                self.readjustment_control(control)
            }
            Some(Dialog::Map(map::MapDialogKind::Ortho)) => self.ortho_control(control),
            Some(Dialog::Map(map::MapDialogKind::OrthoCompare)) => {
                self.ortho_compare_control(control)
            }
            Some(Dialog::Map(map::MapDialogKind::OrthoSeamlines)) => {
                self.ortho_seamlines_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::Reproject)) => self.reproject_control(control),
            Some(Dialog::Map(map::MapDialogKind::Overlap)) => self.overlap_control(control),
            Some(Dialog::Map(map::MapDialogKind::Register)) => self.register_control(control),
            Some(Dialog::Map(map::MapDialogKind::ScanNetwork)) => {
                self.scan_network_control(control)
            }
            Some(Dialog::Map(map::MapDialogKind::CloudPlace)) => self.cloud_place_control(control),
            Some(Dialog::CatalogReport) => self.catalog_report_control(control),
            Some(Dialog::Road(road::RoadDialogKind::SheetContours)) => {
                self.sheet_contours_control(control)
            }
            Some(Dialog::Road(road::RoadDialogKind::SectionCheck)) => {
                self.section_check_control(control)
            }
            Some(Dialog::Road(road::RoadDialogKind::Stockpile)) => self.stockpile_control(control),
            Some(Dialog::Arch(arch::ArchDialogKind::Quantities)) => {
                self.quantities_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::DataCompare)) => {
                self.data_compare_control(control)
            }
            Some(Dialog::Map(map::MapDialogKind::Cogo)) => self.cogo_control(control),
            Some(Dialog::TableInsert) => self.table_insert_control(control),
            Some(Dialog::TableEditor) => self.table_editor_control(control),
            Some(Dialog::Project) if self.asking_type() => self.project_type_control(control),
            Some(Dialog::LayerStyle) => self.layer_style_control(control),
            Some(other) => Err(format!("{other:?} penceresi izden yanıtlanamıyor")),
            None => Err("açık pencere yok".to_owned()),
        }
    }
}
