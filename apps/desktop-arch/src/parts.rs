//! The architecture (Mimari) module's parts the shell reaches (docs/adr/9199 §3.3, §4.9): its messages, its 21 windows
//! with their usage-trace answers and how Esc and Tamam close them, its commands (the style, type and check windows,
//! Tadilat görünümü, IFC dışa aktar), Denetim's tab and preview, the underlay picked on the plan, what 3B görünüm shows
//! beside the drawing's building, Öznitelikler's type and schedule actions, and the program's first tools.

use crate::arch_ext::*;
use std::any::Any;
use std::collections::BTreeSet;
use std::sync::Arc;

use iced::{Element, Task};
use kentos_launch::Module;

use crate::app::{App, Message, Panel};
use crate::arch::{ArchDialogKind as K, ArchState, ArchUiState, Event, dialog};
use crate::bottom::BottomTab;
use crate::module_parts::{Building3dParts, ModuleEvent, ModuleMarks, ModuleParts, PropertiesAsk};
use crate::traces::Control;
use crate::unsaved_close::Answer;

/// The Mimari module's parts.
pub struct Architecture;

impl ModuleParts for Architecture {
    fn module(&self) -> Module {
        Module::Architecture
    }

    fn profile(&self) -> &'static dyn crate::module_profile::ModuleProfile {
        &crate::arch_profile::Architecture
    }

    fn state(&self) -> Box<dyn Any> {
        Box::new(ArchUiState::default())
    }

    fn event(&self, app: &mut App, event: ModuleEvent) -> Task<Message> {
        match event.take::<Event>() {
            Some(event) => app.arch_event(event),
            None => Task::none(),
        }
    }

    fn command(&self, app: &mut App, id: &str) -> Option<Task<Message>> {
        // Tadilat görünümü's phases (arch_project.rs, docs/adr/0721).
        if let Some(phase) = id.strip_prefix("arch.phase.") {
            app.set_phase_view(phase);
            return Some(Task::none());
        }
        match id {
            // Mimari denetim (arch_audit.rs, docs/adr/0703).
            "arch.audit" => return Some(app.open_arch_audit()),
            // IFC dışa aktar (bina) (arch_ifc.rs, docs/adr/8047).
            "arch.ifcExport" => return Some(app.export_ifc_building(kentos_commands::IfcView::DesignTransfer)),
            "arch.ifcExportCut" => return Some(app.export_ifc_building(kentos_commands::IfcView::Reference)),
            "arch.elementTypeFrom" => {
                return Some(app.element_types_event(crate::element_types_window::Event::FromSelection));
            }
            _ => {}
        }
        match id {
            // Duvar stilleri (docs/adr/0339 §6).
            "arch.wallStyles" => app.open_wall_styles(),
            "arch.wallJoinCheck" => app.open_wall_joins(),
            // Kapı ve pencere tipleri (docs/adr/0772).
            "arch.openingTypes" => app.open_opening_types(),
            // Eleman tipleri (docs/adr/8106).
            "arch.elementTypes" => app.open_element_types(),
            "arch.elementSameType" => app.select_same_element_type(),
            // Malzemeler (docs/adr/0785).
            "arch.materials" => app.open_materials(),
            // Oda modülleri (docs/adr/0775).
            "arch.modules" => app.open_room_modules(),
            // Tasarım seçenekleri (docs/adr/0788).
            "arch.options" => app.open_design_options(),
            "arch.moduleExcept" => app.room_module_selection_action(false),
            "arch.moduleRestore" => app.room_module_selection_action(true),
            "arch.moduleAddMembers" => app.room_module_add_selection(None),
            "arch.moduleRemoveMembers" => app.room_module_remove_selection(None),
            "arch.openingSameType" => app.select_same_opening_type(),
            "arch.openingDetach" => app.opening_type_action(kentos_commands::OpeningsOperation::Detach),
            "arch.openingPush" => {
                app.opening_type_action(kentos_commands::OpeningsOperation::Push { fields: Vec::new() });
            }
            "arch.openingReset" => {
                app.opening_type_action(kentos_commands::OpeningsOperation::Reset { fields: Vec::new() });
            }
            "arch.tagsCheck" => app.open_tags_check(),
            "arch.viewRefs" => app.open_view_refs(),
            "arch.template" => app.apply_arch_template(),
            // Metraj and Mahalleri birleştir (docs/adr/0342 §5).
            "arch.quantities" => app.open_quantities(),
            "arch.spaceMerge" => app.space_merge(),
            // Dijital proje kuralları (digital_rules.rs, docs/adr/8062).
            "arch.digitalRules" => app.open_digital_rules(),
            // Teslim kaydı (deliveries_window.rs, docs/adr/8099).
            "arch.deliveries" => app.open_deliveries(),
            // Altlık modeller (underlays_window.rs, docs/adr/8101).
            "arch.underlays" => app.open_underlays(),
            // IFC revizyonlarını karşılaştır and BCF sorunları (docs/adr/8103).
            "arch.ifcCompare" => app.open_ifc_compare(),
            "arch.bcfIssues" => app.open_bcf_issues(),
            // Gelen paketi incele (docs/adr/8104).
            "arch.reviewPackage" => app.open_review_package(),
            // Bina programı (program_window.rs, docs/adr/0773).
            "arch.program" => app.open_program(),
            _ => return None,
        }
        Some(Task::none())
    }

    fn enabled(&self, _app: &App, _id: &str) -> Option<bool> {
        None
    }

    fn dialog_view<'a>(&self, app: &'a App, kind: u16) -> Element<'a, Message> {
        match K::of(kind) {
            Some(kind) => app.arch_dialog_view(kind),
            None => iced::widget::Column::new().into(),
        }
    }

    fn dialog_title(&self, kind: u16) -> String {
        let Some(kind) = K::of(kind) else {
            return format!("Mimari penceresi {kind}");
        };
        match kind {
            K::ArchTemplate => crate::architecture_template::TITLE,
            K::Program => crate::program_window::TITLE,
            K::SpaceMerge => crate::quantities::MERGE_TITLE,
            K::WallStyles => crate::wall_styles::TITLE,
            K::WallJoins => crate::wall_joins::TITLE,
            K::OpeningTypes => crate::opening_types_window::TITLE,
            K::Materials => crate::materials_window::TITLE,
            K::RoomModules => crate::room_modules_window::TITLE,
            K::DesignOptions => crate::design_options_window::TITLE,
            K::Schedule => crate::schedule_window::TITLE,
            K::ScheduleExport => crate::schedule_window::EXPORT_TITLE,
            K::TagsCheck => crate::tags_check::TITLE,
            K::ViewRefs => crate::viewmark_refs::TITLE,
            K::Quantities => crate::quantities::TITLE,
            K::DigitalRules => crate::digital_rules::TITLE,
            K::Deliveries => crate::deliveries_window::TITLE,
            K::Underlays => crate::underlays_window::TITLE,
            K::IfcCompare => crate::ifc_compare_window::TITLE,
            K::Bcf => crate::bcf_window::TITLE,
            K::ReviewPackage => crate::package_review_window::TITLE,
            K::ElementTypes => crate::element_types_window::TITLE,
        }
        .to_owned()
    }

    fn dialog_control(&self, app: &App, kind: u16, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(kind) = K::of(kind) else {
            return Err(format!("Mimari penceresi {kind} izden yanıtlanamıyor"));
        };
        match kind {
            K::Program => app.program_control(control),
            K::WallJoins => app.wall_joins_control(control),
            K::OpeningTypes => app.opening_types_control(control),
            K::Materials => app.materials_control(control),
            K::RoomModules => app.room_modules_control(control),
            K::DesignOptions => app.design_options_control(control),
            K::TagsCheck => app.tags_check_control(control),
            K::ViewRefs => app.view_refs_control(control),
            K::DigitalRules => app.digital_rules_control(control),
            K::Deliveries => app.deliveries_control(control),
            K::Underlays => app.underlays_control(control),
            K::IfcCompare => app.ifc_compare_control(control),
            K::Bcf => app.bcf_control(control),
            K::ReviewPackage => app.review_control(control),
            K::ElementTypes => app.element_types_control(control),
            K::Quantities => app.quantities_control(control),
            K::ArchTemplate | K::SpaceMerge | K::WallStyles | K::Schedule | K::ScheduleExport => {
                Err(format!("{} penceresi izden yanıtlanamıyor", self.dialog_title(kind.number())))
            }
        }
    }

    fn dialog_closed(&self, app: &mut App, kind: u16) {
        let Some(kind) = K::of(kind) else { return };
        let arch = app.modules.arch_mut();
        match kind {
            K::ArchTemplate => arch.arch_template = None,
            // Sil's question first, then the window.
            K::WallStyles if arch.wall_styles.as_ref().is_some_and(|w| w.asking()) => {
                app.dialog = Some(dialog(K::WallStyles));
                let _ = app.wall_styles_event(crate::wall_styles::Event::RemoveDropped);
            }
            // An editor with its own save: Esc is its Kapat, which asks when changes would go; over the question, Esc
            // is Vazgeç (docs/adr/8006).
            K::WallStyles => {
                let closing = arch.wall_styles.as_ref().is_some_and(|w| w.closing());
                app.dialog = Some(dialog(K::WallStyles));
                let _ = app.wall_styles_event(if closing {
                    crate::wall_styles::Event::Answer(Answer::Stay)
                } else {
                    crate::wall_styles::Event::Close
                });
            }
            K::Program => {
                let closing = arch.program_window.as_ref().is_some_and(|w| w.closing());
                app.dialog = Some(dialog(K::Program));
                let _ = app.program_event(if closing {
                    crate::program_window::Event::Answer(Answer::Stay)
                } else {
                    crate::program_window::Event::Close
                });
            }
            K::OpeningTypes => {
                let closing = arch.opening_types.as_ref().is_some_and(|w| w.closing);
                app.dialog = Some(dialog(K::OpeningTypes));
                let _ = app.opening_types_event(if closing {
                    crate::opening_types_window::Event::Answer(Answer::Stay)
                } else {
                    crate::opening_types_window::Event::Close
                });
            }
            K::Materials => {
                let closing = arch.materials.as_ref().is_some_and(|w| w.closing);
                app.dialog = Some(dialog(K::Materials));
                let _ = app.materials_event(if closing {
                    crate::materials_window::Event::Answer(Answer::Stay)
                } else {
                    crate::materials_window::Event::Close
                });
            }
            K::WallJoins => arch.wall_joins = None,
            K::RoomModules => arch.room_modules = None,
            K::DesignOptions => arch.design_options = None,
            K::TagsCheck => arch.tags_check = None,
            K::ViewRefs => arch.view_refs = None,
            K::DigitalRules => arch.digital_rules = None,
            K::Deliveries => arch.deliveries = None,
            K::Underlays => arch.underlays_window = None,
            K::IfcCompare => arch.ifc_compare_window = None,
            K::Bcf => arch.bcf_window = None,
            K::ReviewPackage => arch.review_window = None,
            K::ElementTypes => arch.element_types = None,
            K::Quantities => arch.quantities = None,
            // Its question and the schedule's windows keep what they hold until asked again.
            K::SpaceMerge | K::Schedule | K::ScheduleExport => {}
        }
    }

    fn document_changed(&self, _app: &mut App) {}

    fn unmark(&self, _app: &mut App) {}

    fn marks(&self, _app: &App) -> Vec<Arc<dyn ModuleMarks>> {
        Vec::new()
    }

    fn area_layers<'a>(&self, _app: &'a App) -> Vec<Element<'a, Message>> {
        Vec::new()
    }

    fn panel_view<'a>(&self, _app: &'a App, _panel: Panel) -> Option<Element<'a, Message>> {
        None
    }

    fn started(&self, app: &mut App) {
        // KentOS Mimari's wall tools start on a hatched style (arch_start.rs, docs/adr/8007).
        app.start_architecture_tools();
    }

    fn device(&self, app: &mut App, history: &std::path::Path) {
        // Malzemeler's library folders, beside the program's history (docs/adr/0791).
        app.modules.arch_mut().material_libraries = crate::material_library::Libraries::open(history);
    }

    fn follow(&self, app: &mut App) -> Task<Message> {
        // The underlay models' files read and their plan at the active level drawn (underlays.rs).
        app.follow_underlays()
    }

    fn escape_closes(&self, kind: u16) -> bool {
        // The editors with their own save ask first (docs/adr/8006).
        matches!(K::of(kind), Some(K::WallStyles | K::Program | K::OpeningTypes | K::Materials))
    }

    fn dialog_confirmed(&self, app: &mut App, kind: u16) {
        // Mahalleri birleştir's decision (quantities.rs, docs/adr/0773 §4).
        if K::of(kind) == Some(K::SpaceMerge) {
            app.space_merge_confirmed();
        }
    }

    fn checked(&self, app: &App, id: &str) -> Option<bool> {
        let phase = id.strip_prefix("arch.phase.")?;
        Some(app.phase_view().name() == phase)
    }

    fn picked(&self, app: &mut App, at: Option<kentos_interaction::Vec2>) -> bool {
        // Altlık modeller's Planda seç (underlays_window.rs).
        app.underlays_picked(at)
    }

    fn preview(&self, app: &App) -> Option<kentos_interaction::Preview> {
        // Denetim's chosen findings' fixes and use zones (arch_audit/preview.rs, docs/adr/0732 §5).
        app.arch_audit_preview()
    }

    fn bottom_tab_view<'a>(&self, app: &'a App, tab: BottomTab) -> Option<Element<'a, Message>> {
        (tab == BottomTab::Audit).then(|| app.arch_audit_tab())
    }

    fn properties_ask(&self, app: &mut App, ask: PropertiesAsk<'_>) -> bool {
        use crate::element_types_window::Event as Types;
        use crate::module_parts::{ElementTypeAsk, ScheduleAsk};
        match ask {
            // Tipe uygula from Öznitelikler: its preview asks first (opening_types_window.rs, docs/adr/0772).
            PropertiesAsk::OpeningPush(slot, op) => app.preview_opening_push_at(slot, op.clone()),
            PropertiesAsk::ElementType(ask) => {
                let _ = app.element_types_event(match ask {
                    ElementTypeAsk::Assign(id) => Types::Assign(id.clone()),
                    ElementTypeAsk::Detach => Types::Detach,
                    ElementTypeAsk::Push => Types::Push,
                    ElementTypeAsk::Reset => Types::Reset,
                    ElementTypeAsk::Refresh => Types::Refresh,
                });
            }
            PropertiesAsk::Schedule(ScheduleAsk::Settings(uid)) => {
                let _ = app.schedule_event(crate::schedule_window::Event::Open(uid.clone()));
            }
            PropertiesAsk::Schedule(ScheduleAsk::Export(uid)) => {
                let _ = app.schedule_event(crate::schedule_window::Event::Export(uid.clone()));
            }
        }
        true
    }

    fn building3d(&self) -> Option<&dyn Building3dParts> {
        Some(self)
    }
}

impl Building3dParts for Architecture {
    fn beside(&self, app: &App) -> Vec<(String, String, Arc<crate::underlay_model::Loaded>)> {
        // The underlay models shown (underlays.rs, docs/adr/8101).
        app.underlays_shown().into_iter().map(|(u, l)| (u.id, u.name, l)).collect()
    }

    fn choose_beside(&self, app: &mut App, id: &str, object: usize) -> bool {
        let known = app.document.as_ref().is_some_and(|d| d.model.underlays().iter().any(|u| u.id == id));
        if !known {
            return false;
        }
        // Shown in Altlık modeller, the plan's selection kept.
        if app.modules.arch().underlays_window.is_none() {
            app.open_underlays();
        }
        if let Some(w) = app.modules.arch_mut().underlays_window.as_mut() {
            w.chosen = Some(id.to_owned());
            w.object = Some((id.to_owned(), object));
        }
        true
    }

    fn marked_key(&self, app: &App) -> u64 {
        // The check (its drawing's state and how many it found), the rows chosen, the group shown and whether the
        // drawing changed since.
        use std::hash::{Hash, Hasher};
        let a = &app.modules.arch().arch_audit;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        a.audited.as_ref().map(|x| (x.generation, x.revision, x.findings.len())).hash(&mut h);
        a.selected.hash(&mut h);
        a.chosen.hash(&mut h);
        a.group.hash(&mut h);
        app.arch_audit_stale().hash(&mut h);
        h.finish()
    }

    fn marked(&self, app: &App) -> BTreeSet<String> {
        // The objects of the findings chosen in Denetim, else of all it lists (its last check, while the drawing is
        // as it saw it): a drawing's findings can name nearly every object, so the rows chosen say what to look at.
        if app.arch_audit_stale() {
            return BTreeSet::new();
        }
        let chosen = app.arch_audit_chosen();
        if !chosen.is_empty() {
            return chosen.into_iter().flat_map(|f| f.uids.iter().cloned()).collect();
        }
        let Some(a) = app.modules.arch().arch_audit.audited.as_ref() else { return BTreeSet::new() };
        app.arch_audit_shown().into_iter().filter_map(|i| a.findings.get(i)).flat_map(|f| f.uids.iter().cloned()).collect()
    }
}
