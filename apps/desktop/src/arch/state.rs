//! The architecture (Mimari) module's part of the shell (plan/argon-tavsiye.md ADIM 06, docs/adr/8054 §6): its
//! windows' and panels' state in `App::arch`, its messages under `Message::Arch`, its windows under
//! `Dialog::Arch`. A new window of the module is added here and in its own file; `app.rs` stays as it is.

use iced::{Element, Task};

use crate::app::{App, Message};

/// The architecture (Mimari) module's windows and panels.
pub(crate) struct ArchUiState {
    /// The Şablonlar panel (templates_panel.rs), and the templates drawn with
    /// in this session, the newest first (templates.rs).
    pub(crate) templates_panel: crate::templates_panel::PanelState,
    /// Şablon düzenleyici while it is open (template_editor.rs).
    pub(crate) template_editor: Option<crate::template_editor::Editor>,
    /// The bottom panel's Denetim tab: the last architectural audit, the chosen finding, the group and the
    /// thresholds (arch_audit.rs, docs/adr/0703).
    pub(crate) arch_audit: crate::arch_audit::ArchAuditPanel,
    /// The device's NTv2 grid library and the grids looked for (grids.rs, docs/adr/0168 §4).
    pub(crate) grids: crate::grids::Grids,
    /// The object template being drawn with (templates.rs, docs/adr/0176 §3):
    /// its stamp is in the tools' context while its tool runs.
    pub(crate) template: Option<crate::templates::TemplateRun>,
    /// Etkin kat (active_level.rs, docs/adr/0713): the level new architectural objects go on, the session's.
    pub(crate) active_level: crate::active_level::Chosen,
    pub(crate) arch_template: Option<crate::architecture_template::Window>,
    /// Tasarım denetimi (design_check_window.rs, plan/road.md Y21).
    pub(crate) design_check_window: Option<Box<crate::design_check_window::Window>>,
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
}

/// The architecture (Mimari) module's messages, `Message::Arch`.
#[derive(Debug, Clone)]
pub(crate) enum Event {
    /// The Denetim tab (arch_audit.rs, docs/adr/0703).
    ArchAudit(crate::arch_audit::Event),
    /// The NTv2 grids the project names, looked for in the device's library (grids.rs).
    GridsLooked(Vec<crate::grids::Looked>),
    ToolSetting(crate::architecture_settings::Commit),
    ArchTemplate(crate::architecture_template::Event),
    /// Şablonlar panel (templates_panel.rs).
    TemplatesPanel(crate::templates_panel::Event),
    /// Şablon düzenleyici (template_editor.rs).
    TemplateEditor(crate::template_editor::Event),
    /// An event of Tasarım denetimi (design_check_window.rs, plan/road.md Y21).
    DesignCheck(crate::design_check_window::Event),
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
}

/// The architecture (Mimari) module's windows, `Dialog::Arch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArchDialogKind {
    /// Read-only architectural package preview, followed by an explicit semantic apply.
    ArchTemplate,
    /// Şablonlar → Sil (templates_panel.rs); the template is the panel's `deleting`.
    RemoveTemplate,
    /// Şablon düzenleyici (template_editor.rs); its form is `ArchUiState::template_editor`.
    TemplateEditor,
    /// Tasarım denetimi (design_check_window.rs, plan/road.md Y21); the window is `ArchUiState::design_check_window`.
    DesignCheck,
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
}

impl App {
    /// A message of the module (`Message::Arch`); a window's own handler is in its file.
    pub(crate) fn arch_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::ArchAudit(event) => return self.arch_audit_event(event),
            Event::GridsLooked(looked) => self.grids_looked(looked),
            Event::ToolSetting(event) => return self.tool_setting(event),
            Event::TemplatesPanel(event) => return self.templates_panel_event(event),
            Event::TemplateEditor(event) => return self.template_editor_event(event),
            Event::DesignCheck(event) => return self.design_check_event(event),
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
            Event::ArchTemplate(event) => self.architecture_template_event(event),
        }
        Task::none()
    }

    /// A window of the module (`Dialog::Arch`).
    pub(crate) fn arch_dialog_view(&self, kind: ArchDialogKind) -> Element<'_, Message> {
        match kind {
            ArchDialogKind::RemoveTemplate => self.remove_template_question(),
            ArchDialogKind::TemplateEditor => self.template_editor_view(),
            ArchDialogKind::DesignCheck => self.design_check_view(),
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
        }
    }
}
