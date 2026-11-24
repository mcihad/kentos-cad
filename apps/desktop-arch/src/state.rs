//! The architecture (Mimari) module's hub (plan/argon-tavsiye.md ADIM 06, docs/adr/8054 §6, 9199 §4.9): its windows'
//! and panels' state among the modules' (`App::modules`, [`ArchState`]), its messages as the shell's
//! (`Message::Module`), its windows numbered as the shell's (`Dialog::Module`, [`dialog`]). A new window of the module
//! is added here and in its own file; the shell stays as it is.

use crate::arch_ext::*;
use iced::{Element, Task};

use crate::app::{App, Message};

/// The architecture (Mimari) module's windows and panels.
#[derive(Default)]
pub struct ArchUiState {
    /// The bottom panel's Denetim tab: the last architectural audit, the chosen finding, the group and the
    /// thresholds (arch_audit.rs, docs/adr/0703).
    pub(crate) arch_audit: crate::arch_audit::ArchAuditPanel,
    pub(crate) arch_template: Option<crate::architecture_template::Window>,
    /// Bina programı (program_window.rs, docs/adr/0773).
    pub(crate) program_window: Option<crate::program_window::Window>,
    /// Duvar stilleri (wall_styles.rs, docs/adr/0339 §6).
    pub(crate) wall_styles: Option<crate::wall_styles::Window>,
    /// Birleşimleri denetle's window (wall_joins.rs, docs/adr/0340 §6).
    pub(crate) wall_joins: Option<crate::wall_joins::Window>,
    /// Kapı ve pencere tipleri's window (opening_types_window.rs, docs/adr/0772).
    pub(crate) opening_types: Option<crate::opening_types_window::Window>,
    /// Malzemeler's window (materials_window.rs, docs/adr/0785).
    pub(crate) materials: Option<crate::materials_window::Window>,
    /// The office's material library folders (material_library.rs, docs/adr/0791).
    pub(crate) material_libraries: crate::material_library::Libraries,
    /// Oda modülleri's window (room_modules_window.rs, docs/adr/0775).
    pub(crate) room_modules: Option<crate::room_modules_window::Window>,
    /// Tasarım seçenekleri's window (design_options_window.rs, docs/adr/0788).
    pub(crate) design_options: Option<crate::design_options_window::Window>,
    /// Etiketleri denetle's window (tags_check.rs, docs/adr/0344 §2).
    pub(crate) tags_check: Option<crate::tags_check::Window>,
    /// Çizelge ayarları's and Dosyaya yaz's window (schedule_window.rs, docs/adr/0733 §5).
    pub(crate) schedule: Option<crate::schedule_window::Window>,
    /// Başvurular' window (viewmark_refs.rs, docs/adr/0345 §3).
    pub(crate) view_refs: Option<crate::viewmark_refs::Window>,
    /// Metraj's window (quantities.rs, docs/adr/0342 §5).
    pub(crate) quantities: Option<crate::quantities::Window>,
    /// Dijital proje kuralları' window (digital_rules.rs, docs/adr/8062).
    pub(crate) digital_rules: Option<crate::digital_rules::Window>,
    /// Teslim kaydı's window (deliveries_window.rs, docs/adr/8099).
    pub(crate) deliveries: Option<crate::deliveries_window::Window>,
    /// The underlay models' files as read and their plan (underlays.rs, docs/adr/8101).
    pub(crate) underlays: crate::underlays::Underlays,
    /// Altlık modeller's window (underlays_window.rs, docs/adr/8101).
    pub(crate) underlays_window: Option<crate::underlays_window::Window>,
    /// IFC revizyonlarını karşılaştır's window (ifc_compare_window.rs, docs/adr/8103).
    pub(crate) ifc_compare_window: Option<crate::ifc_compare_window::Window>,
    /// BCF sorunları's window (bcf_window.rs, docs/adr/8103).
    pub(crate) bcf_window: Option<crate::bcf_window::Window>,
    /// Gelen paketi incele's window (package_review_window.rs, docs/adr/8104).
    pub(crate) review_window: Option<crate::package_review_window::Window>,
    /// Eleman tipleri's window (element_types_window.rs, docs/adr/8106).
    pub(crate) element_types: Option<crate::element_types_window::Window>,
    /// Planda seç waits for a point on an underlay's plan.
    pub(crate) underlay_picking: bool,
}

/// The architecture (Mimari) module's messages (`Message::Module`).
#[derive(Debug, Clone)]
pub enum Event {
    /// The Denetim tab (arch_audit.rs, docs/adr/0703).
    ArchAudit(crate::arch_audit::Event),
    ArchTemplate(crate::architecture_template::Event),
    /// An event of Bina programı (program_window.rs, docs/adr/0773).
    Program(crate::program_window::Event),
    WallStyles(crate::wall_styles::Event),
    WallJoins(crate::wall_joins::Event),
    OpeningTypes(crate::opening_types_window::Event),
    Materials(crate::materials_window::Event),
    RoomModules(crate::room_modules_window::Event),
    DesignOptions(crate::design_options_window::Event),
    TagsCheck(crate::tags_check::Event),
    Schedule(crate::schedule_window::Event),
    ViewRefs(crate::viewmark_refs::Event),
    Quantities(crate::quantities::Event),
    /// Dijital proje kuralları (digital_rules.rs, docs/adr/8062).
    DigitalRules(crate::digital_rules::Event),
    /// Teslim kaydı (deliveries_window.rs, docs/adr/8099).
    Deliveries(crate::deliveries_window::Event),
    /// An underlay model's file read off the window's thread, for what it was read for (underlays.rs).
    UnderlayRead(
        String,
        Box<crate::underlays::Key>,
        Result<std::sync::Arc<crate::underlays::Loaded>, String>,
    ),
    /// Altlık modeller (underlays_window.rs, docs/adr/8101).
    Underlays(crate::underlays_window::Event),
    /// IFC revizyonlarını karşılaştır (ifc_compare_window.rs, docs/adr/8103).
    IfcCompare(crate::ifc_compare_window::Event),
    /// BCF sorunları (bcf_window.rs, docs/adr/8103).
    Bcf(crate::bcf_window::Event),
    /// Gelen paketi incele (package_review_window.rs, docs/adr/8104).
    ReviewPackage(crate::package_review_window::Event),
    /// Eleman tipleri and the selection's type actions (element_types_window.rs, docs/adr/8106).
    ElementTypes(crate::element_types_window::Event),
    /// IFC dışa aktar (bina): the file chosen (arch_ifc.rs, docs/adr/8047).
    IfcBuildingExport(Option<std::path::PathBuf>),
    /// IFC dışa aktar (bina, kesilmiş gövde): the file chosen (arch_ifc.rs, docs/adr/8071 §4).
    IfcBuildingExportCut(Option<std::path::PathBuf>),
}

/// The architecture (Mimari) module's windows (`Dialog::Module`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchDialogKind {
    /// Read-only architectural package preview, followed by an explicit semantic apply.
    ArchTemplate,
    /// Bina programı (program_window.rs, docs/adr/0773); the window is `ArchUiState::program_window`.
    Program,
    /// Mahalleri birleştir's decision, asked before writing (quantities.rs, docs/adr/0773 §4).
    SpaceMerge,
    /// Duvar stilleri (wall_styles.rs, docs/adr/0339 §6); the window is `ArchUiState::wall_styles`.
    WallStyles,
    /// Birleşimleri denetle (wall_joins.rs, docs/adr/0340 §6); the window is `ArchUiState::wall_joins`.
    WallJoins,
    /// Kapı ve pencere tipleri (opening_types_window.rs, docs/adr/0772); the window is `ArchUiState::opening_types`.
    OpeningTypes,
    /// Malzemeler (materials_window.rs, docs/adr/0785); the window is `ArchUiState::materials`.
    Materials,
    /// Oda modülleri (room_modules_window.rs, docs/adr/0775); the window is `ArchUiState::room_modules`.
    RoomModules,
    /// Tasarım seçenekleri (design_options_window.rs, docs/adr/0788); the window is `ArchUiState::design_options`.
    DesignOptions,
    /// Çizelge ayarları and Çizelgeyi dosyaya yaz (schedule_window.rs, docs/adr/0733 §5, §7); the window is
    /// `ArchUiState::schedule`.
    Schedule,
    ScheduleExport,
    /// Etiketleri denetle (tags_check.rs, docs/adr/0344 §2); the window is `ArchUiState::tags_check`.
    TagsCheck,
    /// Başvurular (viewmark_refs.rs, docs/adr/0345 §3); the window is `ArchUiState::view_refs`.
    ViewRefs,
    /// Metraj (quantities.rs, docs/adr/0342 §5); the window is `ArchUiState::quantities`.
    Quantities,
    /// Dijital proje kuralları (digital_rules.rs, docs/adr/8062); the window is `ArchUiState::digital_rules`.
    DigitalRules,
    /// Teslim kaydı (deliveries_window.rs, docs/adr/8099); the window is `ArchUiState::deliveries`.
    Deliveries,
    /// Altlık modeller (underlays_window.rs, docs/adr/8101); the window is `ArchUiState::underlays_window`.
    Underlays,
    /// IFC revizyonlarını karşılaştır (ifc_compare_window.rs, docs/adr/8103).
    IfcCompare,
    /// BCF sorunları (bcf_window.rs, docs/adr/8103).
    Bcf,
    /// Gelen paketi incele (package_review_window.rs, docs/adr/8104).
    ReviewPackage,
    /// Eleman tipleri (element_types_window.rs, docs/adr/8106); the window is `ArchUiState::element_types`.
    ElementTypes,
}

impl ArchDialogKind {
    /// Every window, in the order their numbers go (`ModuleDialog::kind`).
    pub const ALL: [ArchDialogKind; 21] = [
        ArchDialogKind::ArchTemplate,
        ArchDialogKind::Program,
        ArchDialogKind::SpaceMerge,
        ArchDialogKind::WallStyles,
        ArchDialogKind::WallJoins,
        ArchDialogKind::OpeningTypes,
        ArchDialogKind::Materials,
        ArchDialogKind::RoomModules,
        ArchDialogKind::DesignOptions,
        ArchDialogKind::Schedule,
        ArchDialogKind::ScheduleExport,
        ArchDialogKind::TagsCheck,
        ArchDialogKind::ViewRefs,
        ArchDialogKind::Quantities,
        ArchDialogKind::DigitalRules,
        ArchDialogKind::Deliveries,
        ArchDialogKind::Underlays,
        ArchDialogKind::IfcCompare,
        ArchDialogKind::Bcf,
        ArchDialogKind::ReviewPackage,
        ArchDialogKind::ElementTypes,
    ];

    /// The window's number (`ModuleDialog::kind`), its place in [`Self::ALL`]; a match with no arm for the rest, so a
    /// new window does not build without its number.
    pub const fn number(self) -> u16 {
        match self {
            ArchDialogKind::ArchTemplate => 0,
            ArchDialogKind::Program => 1,
            ArchDialogKind::SpaceMerge => 2,
            ArchDialogKind::WallStyles => 3,
            ArchDialogKind::WallJoins => 4,
            ArchDialogKind::OpeningTypes => 5,
            ArchDialogKind::Materials => 6,
            ArchDialogKind::RoomModules => 7,
            ArchDialogKind::DesignOptions => 8,
            ArchDialogKind::Schedule => 9,
            ArchDialogKind::ScheduleExport => 10,
            ArchDialogKind::TagsCheck => 11,
            ArchDialogKind::ViewRefs => 12,
            ArchDialogKind::Quantities => 13,
            ArchDialogKind::DigitalRules => 14,
            ArchDialogKind::Deliveries => 15,
            ArchDialogKind::Underlays => 16,
            ArchDialogKind::IfcCompare => 17,
            ArchDialogKind::Bcf => 18,
            ArchDialogKind::ReviewPackage => 19,
            ArchDialogKind::ElementTypes => 20,
        }
    }

    /// The window of number `kind`; none for a number the module has not.
    pub fn of(kind: u16) -> Option<Self> {
        Self::ALL.get(usize::from(kind)).copied()
    }
}

/// The module's window `kind` as the shell's.
pub fn dialog(kind: ArchDialogKind) -> crate::app::Dialog {
    crate::app::Dialog::Module(crate::module_parts::ModuleDialog { module: kentos_launch::Module::Architecture, kind: kind.number() })
}

impl From<Event> for Message {
    fn from(event: Event) -> Self {
        Message::Module(crate::module_parts::ModuleEvent::new(kentos_launch::Module::Architecture, event))
    }
}

/// The module's state among the modules' (`App::modules`).
pub trait ArchState {
    fn arch(&self) -> &ArchUiState;
    fn arch_mut(&mut self) -> &mut ArchUiState;
}

impl ArchState for crate::module_parts::ModuleStates {
    fn arch(&self) -> &ArchUiState {
        // Every program with the module's parts starts its state (ModuleStates::start).
        self.get::<ArchUiState>().expect("Mimari modülünün durumu başlatılmamış")
    }

    fn arch_mut(&mut self) -> &mut ArchUiState {
        self.get_mut::<ArchUiState>().expect("Mimari modülünün durumu başlatılmamış")
    }
}

/// The module's `App` methods of this file, as an extension trait (docs/adr/9199 §3.5).
pub(crate) trait StateApp {
    fn arch_event(&mut self, event: Event) -> Task<Message>;
    fn arch_dialog_view(&self, kind: ArchDialogKind) -> Element<'_, Message>;
}

impl StateApp for App {
    /// A message of the module; a window's own handler is in its file.
    fn arch_event(&mut self, event: Event) -> Task<Message>{
        match event {
            Event::ArchAudit(event) => return self.arch_audit_event(event),
            Event::Program(event) => return self.program_event(event),
            Event::WallStyles(event) => return self.wall_styles_event(event),
            Event::WallJoins(event) => return self.wall_joins_event(event),
            Event::OpeningTypes(event) => return self.opening_types_event(event),
            Event::Materials(event) => return self.materials_event(event),
            Event::RoomModules(event) => return self.room_modules_event(event),
            Event::DesignOptions(event) => return self.design_options_event(event),
            Event::TagsCheck(event) => return self.tags_check_event(event),
            Event::Schedule(event) => return self.schedule_event(event),
            Event::ViewRefs(event) => return self.view_refs_event(event),
            Event::Quantities(event) => return self.quantities_event(event),
            Event::DigitalRules(event) => return self.digital_rules_event(event),
            Event::Deliveries(event) => return self.deliveries_event(event),
            Event::UnderlayRead(id, key, read) => self.underlay_read(id, *key, read),
            Event::Underlays(event) => return self.underlays_event(event),
            Event::IfcCompare(event) => return self.ifc_compare_event(event),
            Event::Bcf(event) => return self.bcf_event(event),
            Event::ReviewPackage(event) => return self.review_event(event),
            Event::ElementTypes(event) => return self.element_types_event(event),
            Event::ArchTemplate(event) => self.architecture_template_event(event),
            Event::IfcBuildingExport(path) => {
                return self.ifc_building_chosen(path, kentos_commands::IfcView::DesignTransfer);
            }
            Event::IfcBuildingExportCut(path) => {
                return self.ifc_building_chosen(path, kentos_commands::IfcView::Reference);
            }
        }
        Task::none()
    }

    /// A window of the module.
    fn arch_dialog_view(&self, kind: ArchDialogKind) -> Element<'_, Message>{
        match kind {
            ArchDialogKind::Program => self.program_view(),
            ArchDialogKind::SpaceMerge => self.space_merge_question(),
            ArchDialogKind::WallStyles => self.wall_styles_view(),
            ArchDialogKind::ArchTemplate => self.architecture_template_view(),
            ArchDialogKind::WallJoins => self.wall_joins_view(),
            ArchDialogKind::OpeningTypes => self.opening_types_view(),
            ArchDialogKind::Materials => self.materials_view(),
            ArchDialogKind::RoomModules => self.room_modules_view(),
            ArchDialogKind::DesignOptions => self.design_options_view(),
            ArchDialogKind::TagsCheck => self.tags_check_view(),
            ArchDialogKind::Schedule | ArchDialogKind::ScheduleExport => self.schedule_view(),
            ArchDialogKind::ViewRefs => self.view_refs_view(),
            ArchDialogKind::Quantities => self.quantities_view(),
            ArchDialogKind::DigitalRules => self.digital_rules_view(),
            ArchDialogKind::Deliveries => self.deliveries_view(),
            ArchDialogKind::Underlays => self.underlays_view(),
            ArchDialogKind::IfcCompare => self.ifc_compare_view(),
            ArchDialogKind::Bcf => self.bcf_view(),
            ArchDialogKind::ReviewPackage => self.review_view(),
            ArchDialogKind::ElementTypes => self.element_types_view(),
        }
    }
}
