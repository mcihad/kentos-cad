//! What a trace can do in an open window (the `dialog` step,
//! fixtures/interaction/README.md): read its title, and find the message a
//! control of it sends, by the words the window shows, as the web's player
//! finds the control in the page and uses it with the mouse and the keyboard.
//! A field is filled over what it held, a check box set, a button pressed; a
//! pressed button that is off sends nothing, as a click on it would. Only a
//! window that answers here can be played; another says so.

use crate::app::{App, Dialog, Message};

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
            Dialog::BlockDefine => crate::blocks::DEFINE_TITLE.to_owned(),
            Dialog::BlockAttributes => crate::block_attributes::ATTRIBUTES_TITLE.to_owned(),
            Dialog::AttributeValues => crate::attribute_values::VALUES_TITLE.to_owned(),
            Dialog::BlockCard => crate::block_card::CARD_TITLE.to_owned(),
            Dialog::BlockSchedule => crate::block_schedule::SCHEDULE_TITLE.to_owned(),
            Dialog::FindReplace => crate::find_replace::FIND_TITLE.to_owned(),
            Dialog::LayerMerge => crate::layer_merge::MERGE_TITLE.to_owned(),
            Dialog::DrawingStandard => crate::drawing_standard::STANDARD_TITLE.to_owned(),
            Dialog::DataProfile => crate::data_profile::PROFILE_TITLE.to_owned(),
            Dialog::Subnetworks => crate::subnetworks_window::SUBNETWORKS_TITLE.to_owned(),
            Dialog::SourceStatus => crate::source_status::STATUS_TITLE.to_owned(),
            Dialog::LineTypes => crate::line_types_window::LINE_TYPES_TITLE.to_owned(),
            Dialog::Parameters => crate::parameters_window::PARAMETERS_TITLE.to_owned(),
            Dialog::HatchPatterns => crate::hatch_patterns_window::HATCH_PATTERNS_TITLE.to_owned(),
            Dialog::StyleImpact => crate::style_impact::IMPACT_TITLE.to_owned(),
            Dialog::LayerFields => crate::layer_fields::FIELDS_TITLE.to_owned(),
            Dialog::TopologyRules => crate::topology::rules::RULES_TITLE.to_owned(),
            Dialog::Networks => crate::networks::window::TITLE.to_owned(),
            Dialog::AddressData => crate::address::data::TITLE.to_owned(),
            Dialog::AddressProviders => crate::address::providers::TITLE.to_owned(),
            Dialog::TimeLayer => crate::temporal::layer::TITLE.to_owned(),
            Dialog::LayerFilter => crate::layer_filters::TITLE.to_owned(),
            Dialog::FieldsWrite => crate::features::write::TITLE_CALC.to_owned(),
            Dialog::LayerInfo => crate::layer_info::TITLE.to_owned(),
            Dialog::Attachment => self.attachment_title().to_owned(),
            Dialog::PhotoViewer => crate::attachments::VIEWER_TITLE.to_owned(),
            Dialog::Labelling => crate::labelling::TITLE.to_owned(),
            Dialog::Scenario => self.scenario_title(),
            Dialog::LayerStates => crate::layer_states::STATES_TITLE.to_owned(),
            Dialog::Views => crate::views::VIEWS_TITLE.to_owned(),
            Dialog::Forms => crate::forms::designer::TITLE.to_owned(),
            Dialog::FormEntry => crate::forms::entry::TITLE.to_owned(),
            Dialog::Relations => crate::forms::relations::TITLE.to_owned(),
            Dialog::Joins => crate::joins_window::TITLE.to_owned(),
            Dialog::EventLinks => crate::event_links_window::TITLE.to_owned(),
            Dialog::TableStyles => crate::forms::table_styles::TITLE.to_owned(),
            Dialog::Subtypes => crate::forms::subtypes::TITLE.to_owned(),
            Dialog::Contingents => crate::forms::contingents::TITLE.to_owned(),
            Dialog::LayerRules => crate::forms::layer_rules::TITLE.to_owned(),
            Dialog::Domains => crate::forms::domains::TITLE.to_owned(),
            Dialog::FormAudit => crate::forms::audit::TITLE.to_owned(),
            Dialog::AnnotationStyles => self.annotation_styles_title().to_owned(),
            Dialog::WallStyles => crate::wall_styles::TITLE.to_owned(),
            Dialog::WallJoins => crate::wall_joins::TITLE.to_owned(),
            Dialog::TagsCheck => crate::tags_check::TITLE.to_owned(),
            Dialog::ViewRefs => crate::viewmark_refs::TITLE.to_owned(),
            Dialog::SheetSet => crate::sheet_set::TITLE.to_owned(),
            Dialog::Xrefs => crate::xrefs_window::TITLE.to_owned(),
            Dialog::XrefAttach => crate::xrefs_window::ATTACH_TITLE.to_owned(),
            Dialog::Quantities => crate::quantities::TITLE.to_owned(),
            Dialog::LayerPurge => crate::layer_purge::PURGE_TITLE.to_owned(),
            Dialog::TakeFrom => crate::drawing_exchange::TAKE_TITLE.to_owned(),
            Dialog::LayerList => crate::layer_list::LIST_TITLE.to_owned(),
            Dialog::DataCompare => crate::data_compare::COMPARE_TITLE.to_owned(),
            Dialog::TableInsert => crate::tables::insert::INSERT_TITLE.to_owned(),
            Dialog::TableEditor => crate::tables::editor::EDITOR_TITLE.to_owned(),
            Dialog::Cogo => crate::cogo::COGO_TITLE.to_owned(),
            Dialog::Project if self.asking_type() => crate::project::TYPE_TITLE.to_owned(),
            Dialog::LayerStyle => self.layer_style_title(),
            other => format!("{other:?}"),
        })
    }

    /// The message `control` of the open window sends; `None` when it sends
    /// nothing (a button that is off, a box already so).
    pub fn dialog_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        match self.dialog {
            Some(Dialog::BlockDefine) => self.block_define_control(control),
            Some(Dialog::BlockAttributes) => self.block_attributes_control(control),
            Some(Dialog::AttributeValues) => self.attribute_values_control(control),
            Some(Dialog::FindReplace) => self.find_replace_control(control),
            Some(Dialog::LayerMerge) => self.layer_merge_control(control),
            Some(Dialog::LayerTranslate) => self.layer_translate_control(control),
            Some(Dialog::DrawingStandard) => self.drawing_standard_control(control),
            Some(Dialog::DataProfile) => self.data_profile_control(control),
            Some(Dialog::Subnetworks) => self.subnetworks_control(control),
            Some(Dialog::SourceStatus) => self.source_status_control(control),
            Some(Dialog::LineTypes) => self.line_types_control(control),
            Some(Dialog::Parameters) => self.parameters_control(control),
            Some(Dialog::HatchPatterns) => self.hatch_patterns_control(control),
            Some(Dialog::StyleImpact) => self.style_impact_control(control),
            Some(Dialog::LayerFields) => self.layer_fields_control(control),
            Some(Dialog::TopologyRules) => self.topology_rules_control(control),
            Some(Dialog::Networks) => self.networks_control(control),
            Some(Dialog::AddressData) => self.address_data_control(control),
            Some(Dialog::AddressProviders) => self.address_providers_control(control),
            Some(Dialog::TimeLayer) => self.time_layer_control(control),
            Some(Dialog::LayerFilter) => self.layer_filter_control(control),
            Some(Dialog::LayerInfo) => self.layer_info_control(control),
            Some(Dialog::Labelling) => self.labelling_control(control),
            Some(Dialog::Scenario) => self.scenario_control(control),
            Some(Dialog::LayerStates) => self.layer_states_control(control),
            Some(Dialog::Views) => self.views_control(control),
            Some(Dialog::Forms) => self.forms_control(control),
            Some(Dialog::FormEntry) => self.form_entry_control(control),
            Some(Dialog::Relations) => self.relations_control(control),
            Some(Dialog::Joins) => self.joins_control(control),
            Some(Dialog::EventLinks) => self.event_links_control(control),
            Some(Dialog::TableStyles) => self.table_styles_control(control),
            Some(Dialog::Subtypes) => self.subtypes_control(control),
            Some(Dialog::Contingents) => self.contingents_control(control),
            Some(Dialog::LayerRules) => self.layer_rules_control(control),
            Some(Dialog::Domains) => self.domains_control(control),
            Some(Dialog::FormAudit) => self.form_audit_control(control),
            Some(Dialog::AnnotationStyles) => self.annotation_styles_control(control),
            Some(Dialog::LayerPurge) => self.layer_purge_control(control),
            Some(Dialog::TakeFrom) => self.take_from_control(control),
            Some(Dialog::LayerList) => self.layer_list_control(control),
            Some(Dialog::WallJoins) => self.wall_joins_control(control),
            Some(Dialog::TagsCheck) => self.tags_check_control(control),
            Some(Dialog::ViewRefs) => self.view_refs_control(control),
            Some(Dialog::SheetSet) => self.sheet_set_control(control),
            Some(Dialog::Xrefs | Dialog::XrefAttach) => self.xrefs_control(control),
            Some(Dialog::Quantities) => self.quantities_control(control),
            Some(Dialog::DataCompare) => self.data_compare_control(control),
            Some(Dialog::Cogo) => self.cogo_control(control),
            Some(Dialog::TableInsert) => self.table_insert_control(control),
            Some(Dialog::TableEditor) => self.table_editor_control(control),
            Some(Dialog::Project) if self.asking_type() => self.project_type_control(control),
            Some(Dialog::LayerStyle) => self.layer_style_control(control),
            Some(other) => Err(format!("{other:?} penceresi izden yanıtlanamıyor")),
            None => Err("açık pencere yok".to_owned()),
        }
    }
}
