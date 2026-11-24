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
        // A window of its own answers while nothing is open over the drawing (shell/windows/surface.rs).
        if self.dialog.is_none()
            && let Some(title) = self.apart_trace_title()
        {
            return Some(title);
        }
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
            Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::Terrains) => crate::terrain_window::TITLE.to_owned(),
            Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::Platforms) => {
                crate::platform_window::TITLE.to_owned()
            }
            Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::TerrainCloud) => {
                crate::terrain_cloud_window::TITLE.to_owned()
            }
            Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::Drainage) => {
                crate::drainage_window::TITLE.to_owned()
            }
            Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::SurfaceAnalysis) => {
                crate::surface_analysis_window::TITLE.to_owned()
            }
            Dialog::Propose => crate::propose_window::TITLE.to_owned(),
            Dialog::Scene3d => crate::scene3d_window::TITLE.to_owned(),
            Dialog::IfcViewer => crate::ifc_viewer::TITLE.to_owned(),
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
            Dialog::Arch(arch::ArchDialogKind::DigitalRules) => crate::digital_rules::TITLE.to_owned(),
            Dialog::Arch(arch::ArchDialogKind::Deliveries) => crate::deliveries_window::TITLE.to_owned(),
            Dialog::Arch(arch::ArchDialogKind::Underlays) => crate::underlays_window::TITLE.to_owned(),
            Dialog::Arch(arch::ArchDialogKind::IfcCompare) => crate::ifc_compare_window::TITLE.to_owned(),
            Dialog::Arch(arch::ArchDialogKind::Bcf) => crate::bcf_window::TITLE.to_owned(),
            Dialog::Arch(arch::ArchDialogKind::ReviewPackage) => crate::package_review_window::TITLE.to_owned(),
            Dialog::Arch(arch::ArchDialogKind::ElementTypes) => crate::element_types_window::TITLE.to_owned(),
            Dialog::SheetSet => crate::sheet_set::TITLE.to_owned(),
            Dialog::Xrefs => crate::xrefs_window::TITLE.to_owned(),
            Dialog::Flatten => crate::flatten_window::TITLE.to_owned(),
            Dialog::Gis(gis::GisDialogKind::FeaturesCode) => {
                crate::features_window::TITLE.to_owned()
            }
            Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::Grade) => crate::grade_window::TITLE.to_owned(),
            Dialog::CleanReport => crate::clean_report_window::TITLE.to_owned(),
            Dialog::OpenReport => crate::open_report_window::TITLE.to_owned(),
            Dialog::ExchangeCheck => crate::exchange_check_window::TITLE.to_owned(),
            Dialog::OverlapLayers => crate::overlap_layers_window::TITLE.to_owned(),
            Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::ProjectGraph) => {
                crate::project_graph_window::TITLE.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::FeaturesReconcile) => {
                crate::features_reconcile_window::TITLE.to_owned()
            }
            Dialog::Gis(gis::GisDialogKind::Impact) => crate::impact_window::TITLE.to_owned(),
            Dialog::MapCrs => crate::map_crs::TITLE.to_owned(),
            Dialog::CatalogReport => crate::catalog_report_window::TITLE.to_owned(),
            Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::SheetContours) => {
                crate::sheet_contours_window::TITLE.to_owned()
            }
            Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::SectionCheck) => {
                crate::section_check_window::TITLE.to_owned()
            }
            Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::Stockpile) => {
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
            // A module's window: its title is the module's (docs/adr/9199 §3.3).
            Dialog::Module(d) => match crate::module_parts::parts(d.module) {
                Some(parts) => parts.dialog_title(d.kind),
                None => format!("{other:?}", other = Dialog::Module(d)),
            },
            Dialog::Project if self.asking_type() => crate::project::TYPE_TITLE.to_owned(),
            Dialog::LayerStyle => self.layer_style_title(),
            other => format!("{other:?}"),
        })
    }

    /// The message `control` of the open window sends; `None` when it sends
    /// nothing (a button that is off, a box already so).
    pub fn dialog_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        // The button that moves a window to its own or back is the windows', in every such window.
        if let Some(moved) = self.apart_control(control) {
            return Ok(Some(moved));
        }
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
            Some(Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::Terrains)) => self.terrain_control(control),
            Some(Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::Platforms)) => self.platform_control(control),
            Some(Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::TerrainCloud)) => {
                self.terrain_cloud_control(control)
            }
            Some(Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::Drainage)) => self.drainage_control(control),
            Some(Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::SurfaceAnalysis)) => {
                self.surface_analysis_control(control)
            }
            Some(Dialog::Propose) => self.propose_control(control),
            Some(Dialog::Scene3d) => self.scene3d_control(control),
            Some(Dialog::IfcViewer) => self.ifc_viewer_control(control),
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
            Some(Dialog::Arch(arch::ArchDialogKind::DigitalRules)) => self.digital_rules_control(control),
            Some(Dialog::Arch(arch::ArchDialogKind::Deliveries)) => self.deliveries_control(control),
            Some(Dialog::Arch(arch::ArchDialogKind::Underlays)) => self.underlays_control(control),
            Some(Dialog::Arch(arch::ArchDialogKind::IfcCompare)) => self.ifc_compare_control(control),
            Some(Dialog::Arch(arch::ArchDialogKind::Bcf)) => self.bcf_control(control),
            Some(Dialog::Arch(arch::ArchDialogKind::ReviewPackage)) => self.review_control(control),
            Some(Dialog::Arch(arch::ArchDialogKind::ElementTypes)) => self.element_types_control(control),
            Some(Dialog::SheetSet) => self.sheet_set_control(control),
            Some(Dialog::Xrefs | Dialog::XrefAttach) => self.xrefs_control(control),
            Some(Dialog::Flatten) => self.flatten_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::FeaturesCode)) => {
                self.features_code_control(control)
            }
            Some(Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::Grade)) => self.grade_control(control),
            Some(Dialog::CleanReport) => self.clean_report_control(control),
            Some(Dialog::OpenReport) => self.open_report_control(control),
            Some(Dialog::ExchangeCheck) => self.exchange_check_control(control),
            Some(Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::ProjectGraph)) => {
                self.project_graph_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::FeaturesReconcile)) => {
                self.features_reconcile_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::Impact)) => self.impact_control(control),
            Some(Dialog::Gis(gis::GisDialogKind::Reproject)) => self.reproject_control(control),
            Some(Dialog::CatalogReport) => self.catalog_report_control(control),
            Some(Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::SheetContours)) => {
                self.sheet_contours_control(control)
            }
            Some(Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::SectionCheck)) => {
                self.section_check_control(control)
            }
            Some(Dialog::TerrainWork(crate::terrain_work::TerrainDialogKind::Stockpile)) => self.stockpile_control(control),
            Some(Dialog::Arch(arch::ArchDialogKind::Quantities)) => {
                self.quantities_control(control)
            }
            Some(Dialog::Gis(gis::GisDialogKind::DataCompare)) => {
                self.data_compare_control(control)
            }
            Some(Dialog::TableInsert) => self.table_insert_control(control),
            Some(Dialog::TableEditor) => self.table_editor_control(control),
            Some(Dialog::Module(d)) => match crate::module_parts::parts(d.module) {
                Some(parts) => parts.dialog_control(self, d.kind, control),
                None => Err(format!("{:?} penceresi izden yanıtlanamıyor", Dialog::Module(d))),
            },
            Some(Dialog::Project) if self.asking_type() => self.project_type_control(control),
            Some(Dialog::LayerStyle) => self.layer_style_control(control),
            Some(other) => Err(format!("{other:?} penceresi izden yanıtlanamıyor")),
            None => self
                .apart_trace_control(control)
                .unwrap_or_else(|| Err("açık pencere yok".to_owned())),
        }
    }
}
