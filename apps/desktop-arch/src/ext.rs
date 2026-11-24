//! The Mimari module's `App` methods, every file's extension trait (docs/adr/9199 §3.5): a file of the module, its parts
//! and its tests take them all with `use crate::arch_ext::*;`. A file's methods stay its own; this only names them.

pub(crate) use crate::arch::StateApp;
pub(crate) use crate::arch_audit::ArchAuditApp;
pub(crate) use crate::arch_audit::preview::ArchAuditPreviewApp;
pub(crate) use crate::arch_audit::standards::ArchAuditStandardsApp;
pub(crate) use crate::arch_audit::view::ArchAuditViewApp;
pub(crate) use crate::arch_audit::write::ArchAuditWriteApp;
pub(crate) use crate::arch_ifc::ArchIfcApp;
pub(crate) use crate::arch_project::ArchProjectApp;
pub(crate) use crate::arch_start::ArchStartApp;
pub(crate) use crate::architecture_template::ArchitectureTemplateApp;
pub(crate) use crate::bcf_window::BcfWindowApp;
pub(crate) use crate::deliveries_window::DeliveriesWindowApp;
pub(crate) use crate::design_options_window::DesignOptionsWindowApp;
pub(crate) use crate::digital_rules::DigitalRulesApp;
pub(crate) use crate::element_types_window::ElementTypesWindowApp;
pub(crate) use crate::ifc_compare_window::IfcCompareWindowApp;
pub(crate) use crate::material_library::MaterialLibraryApp;
pub(crate) use crate::materials_window::MaterialsWindowApp;
pub(crate) use crate::opening_types_window::OpeningTypesWindowApp;
pub(crate) use crate::package_review_window::PackageReviewWindowApp;
pub(crate) use crate::program_window::ProgramWindowApp;
pub(crate) use crate::program_window::view::ProgramViewApp;
pub(crate) use crate::quantities::QuantitiesApp;
pub(crate) use crate::room_modules_window::RoomModulesWindowApp;
pub(crate) use crate::schedule_window::ScheduleWindowApp;
pub(crate) use crate::tags_check::TagsCheckApp;
pub(crate) use crate::underlay_check_view::UnderlayCheckViewApp;
pub(crate) use crate::underlays::UnderlaysApp;
pub(crate) use crate::underlays_window::UnderlaysWindowApp;
pub(crate) use crate::viewmark_refs::ViewmarkRefsApp;
pub(crate) use crate::wall_joins::WallJoinsApp;
pub(crate) use crate::wall_styles::WallStylesApp;
