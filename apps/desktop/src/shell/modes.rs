//! Proje türü (the web's `app/workspaces.ts`, DESIGN.md §7.3.2, docs/adr/0165):
//! CAD or CBS, one data model, each type its own presentation. The project's
//! type decides which ribbon tabs and panels show (the web builds each type's
//! ribbon with its filter; the inventory carries them); it never changes what
//! the data means, and a hidden command still runs from the command line and
//! its shortcut. The type is a project setting: choosing one is an edit of
//! the drawing, never an undo step. A project not asked its type yet (an old
//! Hibrit one) shows as CBS; announced types say “Yakında”.
//!
//! A project keeps the type it started with (the owner's decision, 23 October
//! 2026; docs/adr/0622 §1): a CAD project stays CAD and a CBS project CBS, and
//! nothing in the interface switches between them. Only a project not asked
//! its type yet takes one, once; going from one type to the other will be a
//! conversion of the project, not a switch.

use iced::Element;
use iced::widget::row;
use kentos_commands::Workspace;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::widget::{Tip, tip};

use crate::app::{App, Message};
use crate::catalog::{Tab, catalog, effective_mode, mode_command};

/// A mode's mark in the status bar and its menu (the web's `workspace.*` commands' icons).
pub(crate) fn mode_icon(mode: Workspace) -> Icon {
    crate::icons::from_web(Some(match mode {
        Workspace::Cad => "modeCad",
        Workspace::Gis | Workspace::LegacyHybrid => "modeGis",
        Workspace::Plan3d => "modePlan3d",
        Workspace::Disaster => "modeDisaster",
    }))
}

/// The contextual tab a selection opened, by what it was worked out from: the
/// selection's change count, the drawing's and the project type (docs/adr/0622 §4).
pub(crate) type SelectionTab = (u64, u64, u64, u64, Option<&'static str>);

impl App {
    /// The type the interface shows: the drawing's; without one (or without a
    /// drawing), a module's program's own (docs/adr/0801), kentos-cad's CBS.
    pub(crate) fn work_mode(&self) -> Workspace {
        let own = self
            .document
            .as_ref()
            .and_then(|d| d.settings().project_type());
        effective_mode(own.or(self.program_type())).id
    }

    /// The ribbon's tabs in the drawing's mode, a CAD ribbon cut to its work (docs/adr/0801).
    pub(crate) fn ribbon_tabs(&self) -> impl Iterator<Item = &'static Tab> {
        catalog().tabs_for(self.work_mode(), self.ribbon_work())
    }

    /// The contextual tabs of the drawing's mode (the web's Seçim, shown
    /// while something is selected), a CAD project's cut to its work (docs/adr/0802).
    pub(crate) fn contextual_tabs(&self) -> impl Iterator<Item = &'static Tab> {
        catalog().contextual_for_work(self.work_mode(), self.ribbon_work())
    }

    /// The contextual tab the selection opens (docs/adr/0622 §4): its
    /// objects' kind's (Çizgi, Alan, Raster …), Seçim for a mixed selection;
    /// none without a selection. Kept until the selection or the drawing changes.
    pub(crate) fn selection_tab(&self) -> Option<&'static Tab> {
        let doc = self.document.as_ref()?;
        if self.selection.is_empty() {
            return None;
        }
        let key = (
            doc.session,
            self.selection.version(),
            doc.model.generation(),
            self.work_mode() as u64,
        );
        let id = match self.selection_tab_cache.get() {
            Some((s, v, g, m, id)) if (s, v, g, m) == key => id,
            _ => {
                let mut kinds: Vec<&'static str> = Vec::new();
                for slot in self.selection.ids() {
                    if let Some(e) = doc.model.get(*slot) {
                        let kind = e.kind();
                        if !kinds.contains(&kind) {
                            kinds.push(kind);
                        }
                    }
                }
                let id = catalog()
                    .contextual_for(self.work_mode(), self.ribbon_work(), &kinds)
                    .map(|t| t.id);
                self.selection_tab_cache
                    .set(Some((key.0, key.1, key.2, key.3, id)));
                id
            }
        };
        let id = id?;
        self.contextual_tabs().find(|t| t.id == id)
    }

    /// After a message: a selection made or changed brings its tab to the
    /// front (AutoCAD's contextual tabs; docs/adr/0622 §4), one cleared
    /// takes it away and the chosen tab shows again.
    pub(crate) fn follow_selection_tab(&mut self) {
        let stamp = self.document.as_ref().map(|d| (d.session, self.selection.version()));
        if stamp == self.ribbon_follows {
            return;
        }
        self.ribbon_follows = stamp;
        let follow = self.settings.effective("drafting.contextualTabs") != serde_json::Value::Bool(false);
        if self.selection.is_empty() {
            self.ribbon_context = false;
        } else if follow {
            self.ribbon_context = true;
        }
    }

    /// The tab shown: the selection's contextual tab while it is open and
    /// something is selected, else the chosen tab ([`App::regular_tab`]).
    pub(crate) fn shown_tab(&self) -> &'static str {
        if self.ribbon_context
            && let Some(tab) = self.selection_tab()
        {
            return tab.id;
        }
        self.regular_tab()
    }

    /// A click on a tab (the web's `select`): the contextual tab opens over
    /// the chosen one, which stays for when the selection is gone; another
    /// becomes the chosen one.
    pub(crate) fn choose_tab(&mut self, id: &'static str) {
        if self.contextual_tabs().any(|t| t.id == id) {
            self.ribbon_context = !self.selection.is_empty();
        } else {
            self.tab = id;
            self.ribbon_context = false;
        }
    }

    /// The chosen tab: the chosen one while the mode keeps it, else Giriş.
    pub(crate) fn regular_tab(&self) -> &'static str {
        let mut tabs = self.ribbon_tabs();
        let first = self.ribbon_tabs().nth(1).map_or("home", |t| t.id);
        tabs.find(|t| t.id == self.tab).map_or(first, |t| t.id)
    }

    /// `workspace.*`: the drawing's mode becomes this one.
    pub(crate) fn choose_mode(&mut self, id: &str) {
        let Some(mode) = catalog().modes().iter().find(|m| mode_command(m.id) == id) else {
            return;
        };
        if !mode.ready {
            self.output(format!(
                "“{}” proje türü yakında geliyor. Şimdilik CAD ya da CBS türünü kullanın.",
                mode.label
            ));
            return;
        }
        let Some(doc) = &mut self.document else {
            self.output(
                "Proje türü projenin ayarıdır; önce bir çizim açın ya da yeni proje oluşturun.",
            );
            return;
        };
        let settings = doc.settings().clone();
        // Choosing the type shown is an answer too when it was not asked yet (shown as CBS).
        if settings.workspace == Some(mode.id) {
            return;
        }
        // A project keeps the type it has (docs/adr/0622 §1).
        if let Some(kept) = settings.project_type() {
            let kept = catalog().modes().iter().find(|m| m.id == kept);
            self.output(format!(
                "Bu proje {} projesi; proje türü değiştirilmez. CAD ile CBS arasında geçiş ileride projeyi dönüştürerek olacak.",
                kept.map_or("?", |m| m.label)
            ));
            return;
        }
        let result = kentos_native_application::settings_set::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_commands::SettingsSet { project_type: Some(mode.id), ..Default::default() },
        );
        if self.answered(result).is_none() {
            return;
        }
        // The tab stays open while the mode keeps it (DESIGN.md §7.3.2).
        self.tab = self.regular_tab();
        self.output(format!(
            "Proje türü: {}. Şeritte olmayan komutlar komut satırından ve kısayoluyla yine çalışır.",
            mode.label
        ));
    }

    /// The status bar's type: its mark and name. A project keeps its type
    /// (docs/adr/0622 §1), so the cell only says it; a project not asked its
    /// type yet opens the question when pressed.
    /// The mode's cell; in a narrow window (`named` false) its icon only.
    pub(crate) fn mode_cell(&self, named: bool) -> Element<'static, Message> {
        let current = self.work_mode();
        let mode = effective_mode(Some(current));
        let set = self.document.as_ref().and_then(|d| d.settings().workspace);
        let unasked = matches!(set, None | Some(Workspace::LegacyHybrid));
        // A CAD project not asked its work is asked it from here too (docs/adr/0801).
        let workless = set == Some(Workspace::Cad)
            && self
                .document
                .as_ref()
                .is_some_and(|d| d.settings().cad_work().is_none());
        let note = match set {
            None | Some(Workspace::LegacyHybrid) => format!(
                " Projenin türü henüz seçilmedi; {} gösteriliyor. Seçmek için tıklayın: tür bir kez seçilir.",
                mode.label
            ),
            Some(Workspace::Cad) if workless => {
                " Bu CAD çiziminin işi (Mimari proje ya da Harita / imar planı) henüz seçilmedi. Seçmek için tıklayın: iş bir kez seçilir."
                    .to_owned()
            }
            Some(saved) if saved != current => {
                let saved = catalog().modes().iter().find(|m| m.id == saved);
                format!(
                    " Proje “{}” türünde kaydedilmiş; bu tür yakında geliyor, şimdilik CBS gösteriliyor.",
                    saved.map_or("?", |m| m.label)
                )
            }
            _ => " Proje türü projeyle başlar ve değişmez; CAD ile CBS arasında geçiş ileride dönüştürmeyle olacak."
                .to_owned(),
        };
        let mut face = row![
            icon(mode_icon(current))
                .size(13.0)
                .tone(kentos_ui::icon::Tone::Accent)
        ]
        .spacing(5)
        .align_y(iced::Center);
        if named {
            face = face.push(kentos_ui::label::strong(mode.label));
        }
        let cell = iced::widget::container(face).padding([0, 6]);
        let ask = if unasked {
            Some(Message::AskProjectType)
        } else {
            workless.then_some(Message::AskCadWork)
        };
        let body: Element<'static, Message> = match ask.filter(|_| self.document.is_some()) {
            Some(ask) => iced::widget::button(cell)
                .padding(0)
                .style(kentos_ui::style::button::ghost)
                .on_press(ask)
                .into(),
            None => cell.into(),
        };
        // The work a CAD drawing is drawn for goes with its type (docs/adr/0801).
        let title = match self.document.as_ref().and_then(|d| d.settings().cad_work()) {
            Some(work) => format!("Proje türü: {} · {}", mode.label, work.name()),
            None => format!("Proje türü: {}", mode.label),
        };
        tip(
            body,
            Tip::new(title).body(format!("{}{note}", mode.description)),
            iced::widget::tooltip::Position::Top,
        )
    }
}

/// The label under a mode's cell, for tests.
#[cfg(test)]
fn shown(app: &App) -> &'static str {
    effective_mode(Some(app.work_mode())).label
}

#[cfg(test)]
mod tests {
    use kentos_commands::Workspace;

    use super::shown;
    use crate::app::{App, Message};
    use crate::files_testing::{app_with_drawing, last_said};

    fn run(app: &mut App, id: &'static str) {
        let _ = app.update(Message::Run(id));
    }

    #[test]
    fn a_mode_is_the_projects_setting_and_an_edit_not_an_undo_step() {
        let mut app = app_with_drawing();
        assert_eq!(app.work_mode(), Workspace::Gis);
        let steps = app.document.as_ref().map(|d| d.model.can_undo());
        run(&mut app, "workspace.cad");
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(doc.settings().workspace, Some(Workspace::Cad));
        assert!(doc.dirty(), "a project setting changed");
        assert_eq!(Some(doc.model.can_undo()), steps, "not an undo step");
        assert_eq!(shown(&app), "CAD");
        assert_eq!(app.checked("workspace.cad"), Some(true));
        assert_eq!(app.checked("workspace.gis"), Some(false));
        assert!(last_said(&app).contains("komut satırından"));
    }

    #[test]
    fn contextual_cache_is_scoped_to_the_open_document() {
        let mut app = app_with_drawing();
        app.selection.set([kentos_domain::Slot(4)]);
        let tab = app.selection_tab().expect("selected area").id;
        let cached = app.selection_tab_cache.get().expect("cached");
        app.follow_selection_tab();
        app.choose_tab("file");
        assert_eq!(app.shown_tab(), "file");
        // A parked drawing may have the same selection/change counters. Its session must invalidate the cache.
        app.document.as_mut().expect("open").session += 1;
        assert_eq!(app.selection_tab().expect("selected area").id, tab);
        assert_ne!(app.selection_tab_cache.get().expect("refreshed").0, cached.0);
        app.follow_selection_tab();
        assert_eq!(app.shown_tab(), tab, "another drawing brings its selection's tab forward");
    }

    #[test]
    fn announced_modes_say_soon_and_change_nothing() {
        let mut app = app_with_drawing();
        run(&mut app, "workspace.plan3d");
        assert_eq!(app.work_mode(), Workspace::Gis);
        assert!(!app.document.as_ref().expect("a drawing").dirty());
        // Without a drawing there is no project to set.
        let (mut empty, _) = App::boot(None);
        run(&mut empty, "workspace.gis");
        assert!(last_said(&empty).contains("önce bir çizim açın"));
    }

    #[test]
    fn a_project_keeps_its_type_once_it_has_one() {
        let mut app = app_with_drawing();
        // Not asked its type yet: shown as CBS, and it takes a type, once (docs/adr/0622 §1).
        assert_eq!(app.work_mode(), Workspace::Gis);
        assert!(app.available("workspace.cad"));
        run(&mut app, "workspace.cad");
        assert_eq!(app.work_mode(), Workspace::Cad);
        // Then it keeps it: the commands are off, say why, and change nothing.
        assert!(!app.available("workspace.gis"));
        assert!(app.why_disabled("workspace.gis").is_some());
        run(&mut app, "workspace.gis");
        assert_eq!(app.work_mode(), Workspace::Cad);
        assert!(last_said(&app).contains("proje türü değiştirilmez"), "{}", last_said(&app));
        // A tab the type lacks gives way to Giriş; one it has stays open.
        app.tab = "analysis";
        assert_eq!(app.shown_tab(), "home");
        app.tab = "output";
        assert_eq!(app.shown_tab(), "output");
    }
}
