//! Proje türü (the web's `app/workspaces.ts`, DESIGN.md §7.3.2, docs/adr/0165):
//! CAD or CBS, one data model, each type its own presentation. The project's
//! type decides which ribbon tabs and panels show (the web builds each type's
//! ribbon with its filter; the inventory carries them); it never changes what
//! the data means, and a hidden command still runs from the command line and
//! its shortcut. The type is a project setting: choosing one is an edit of
//! the drawing, never an undo step. A project not asked its type yet (an old
//! Hibrit one) shows as CBS; announced types say “Yakında”.

use iced::Element;
use iced::widget::row;
use kentos_contracts::Workspace;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::widget::{Menu, MenuButton, Tip, tip};

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

impl App {
    /// The type the interface shows: the drawing's, CBS without one.
    pub(crate) fn work_mode(&self) -> Workspace {
        effective_mode(self.document.as_ref().and_then(|d| d.settings().workspace)).id
    }

    /// The ribbon's tabs in the drawing's mode.
    pub(crate) fn ribbon_tabs(&self) -> impl Iterator<Item = &'static Tab> {
        catalog().tabs_in(self.work_mode())
    }

    /// The contextual tabs of the drawing's mode (the web's Seçim, shown
    /// while something is selected).
    pub(crate) fn contextual_tabs(&self) -> impl Iterator<Item = &'static Tab> {
        catalog().contextual_in(self.work_mode())
    }

    /// The tab shown: the contextual Seçim while it is open and something is
    /// selected, else the chosen tab ([`App::regular_tab`]).
    pub(crate) fn shown_tab(&self) -> &'static str {
        if self.ribbon_context
            && !self.selection.is_empty()
            && let Some(tab) = self.contextual_tabs().next()
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
        let first = catalog()
            .tabs_in(self.work_mode())
            .nth(1)
            .map_or("home", |t| t.id);
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
        let mut settings = doc.settings().clone();
        // Choosing the type shown is an answer too when it was not asked yet (shown as CBS).
        if settings.workspace == Some(mode.id) {
            return;
        }
        settings.workspace = Some(mode.id);
        doc.model.set_settings(settings);
        // The tab stays open while the mode keeps it (DESIGN.md §7.3.2).
        self.tab = self.regular_tab();
        self.output(format!(
            "Proje türü: {}. Şeritte olmayan komutlar komut satırından ve kısayoluyla yine çalışır.",
            mode.label
        ));
    }

    /// The status bar's mode: its mark and name; a click lists the modes.
    /// The mode's cell; in a narrow window (`named` false) its icon only.
    pub(crate) fn mode_cell(&self, named: bool) -> Element<'static, Message> {
        let current = self.work_mode();
        let mode = effective_mode(Some(current));
        let set = self.document.as_ref().and_then(|d| d.settings().workspace);
        let note = match set {
            None | Some(Workspace::LegacyHybrid) => {
                " Projenin türü henüz seçilmedi; CBS gösteriliyor.".to_owned()
            }
            Some(saved) if saved != current => {
                let saved = catalog().modes().iter().find(|m| m.id == saved);
                format!(
                    " Proje “{}” türünde kaydedilmiş; bu tür yakında geliyor, şimdilik CBS gösteriliyor.",
                    saved.map_or("?", |m| m.label)
                )
            }
            _ => String::new(),
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
        let menu = move || {
            let modes = catalog().modes();
            let ready = modes.iter().filter(|m| m.ready).fold(
                Menu::new().header("Proje türü"),
                |menu, m| {
                    // A choice among several: its dot beside its icon (the web's `menuRowLook`).
                    menu.radio(
                        m.label.to_owned(),
                        m.id == current,
                        Message::Run(mode_command(m.id)),
                    )
                    .icon(mode_icon(m.id))
                },
            );
            modes
                .iter()
                .filter(|m| !m.ready)
                .fold(ready.separator(), |menu, m| {
                    menu.item(m.label, None)
                        .icon(mode_icon(m.id))
                        .shortcut("Yakında")
                })
        };
        tip(
            MenuButton::new(
                iced::widget::container(face).padding([0, 6]),
                menu,
            ),
            Tip::new(format!("Proje türü: {}", mode.label)).body(format!(
                "{}{note} Proje ayarıdır; değiştirmek için tıklayın. Şeritte olmayan komutlar komut satırından yine çalışır.",
                mode.description
            )),
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
    use kentos_contracts::Workspace;

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
    fn the_ribbon_follows_the_mode_and_a_hidden_tab_gives_way_to_giris() {
        let mut app = app_with_drawing();
        let gis: Vec<&str> = app.ribbon_tabs().map(|t| t.label).collect();
        assert!(gis.contains(&"Harita") && gis.contains(&"İşlemler"));
        run(&mut app, "workspace.cad");
        // The web's CAD ribbon (the inventory carries it): no İşlemler, Harita is Ölçme.
        let cad: Vec<&str> = app.ribbon_tabs().map(|t| t.label).collect();
        assert!(cad.contains(&"Ölçme"), "{cad:?}");
        assert!(!cad.contains(&"İşlemler"), "{cad:?}");
        // CBS hides the advanced drawing tools and shows Giriş's Harita panel.
        run(&mut app, "workspace.gis");
        let draw = app.ribbon_tabs().find(|t| t.id == "draw").expect("Çizim");
        let ids: Vec<&str> = draw
            .panels
            .iter()
            .flat_map(|p| p.items.iter())
            .filter_map(|i| match i {
                crate::catalog::Item::Command { id, .. } => Some(*id),
                _ => None,
            })
            .collect();
        assert!(!ids.contains(&"tool.ellipse"), "{ids:?}");
        let home = app.ribbon_tabs().find(|t| t.id == "home").expect("Giriş");
        assert!(home.panels.iter().any(|p| p.label == "Harita"));
        // A tab the type hides gives way to Giriş.
        app.tab = "processing";
        run(&mut app, "workspace.cad");
        assert_eq!(app.shown_tab(), "home");
        // A tab the mode keeps stays open.
        app.tab = "modify";
        run(&mut app, "workspace.gis");
        assert_eq!(app.shown_tab(), "modify");
    }
}
