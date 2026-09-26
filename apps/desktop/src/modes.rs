//! Çalışma modu (the web's `app/workspaces.ts`, DESIGN.md §7.3.2): one
//! project, one data model, several presentations. The project's mode
//! decides which ribbon tabs and panels show (the web builds each mode's
//! ribbon with its filter; the inventory carries them); it never changes what
//! the data means, and a hidden command still runs from the command line and
//! its shortcut. The mode is a project setting: choosing one is an edit of
//! the drawing, never an undo step. Announced modes say “Yakında”.

use iced::Element;
use iced::widget::row;
use kentos_contracts::Workspace;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::widget::{Menu, MenuButton, Tip, tip};

use crate::app::{App, Message};
use crate::catalog::{Tab, catalog, effective_mode, mode_command};

/// A mode's mark in the status bar.
fn mode_icon(mode: Workspace) -> Icon {
    match mode {
        Workspace::Cad => Icon::Polyline,
        Workspace::Gis => Icon::Legend,
        _ => Icon::Layers,
    }
}

impl App {
    /// The mode the interface shows: the drawing's, Hibrit without one.
    pub(crate) fn work_mode(&self) -> Workspace {
        effective_mode(self.document.as_ref().and_then(|d| d.settings().workspace)).id
    }

    /// The ribbon's tabs in the drawing's mode.
    pub(crate) fn ribbon_tabs(&self) -> impl Iterator<Item = &'static Tab> {
        catalog().tabs_in(self.work_mode())
    }

    /// The tab shown: the chosen one while the mode keeps it, else Giriş.
    pub(crate) fn shown_tab(&self) -> &'static str {
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
                "“{}” çalışma modu yakında geliyor. Şimdilik Hibrit, CAD ya da CBS modunu kullanın.",
                mode.label
            ));
            return;
        }
        let Some(doc) = &mut self.document else {
            self.output(
                "Çalışma modu projenin ayarıdır; önce bir çizim açın ya da yeni proje oluşturun.",
            );
            return;
        };
        let mut settings = doc.settings().clone();
        if effective_mode(settings.workspace).id == mode.id {
            return;
        }
        settings.workspace = Some(mode.id);
        doc.model.set_settings(settings);
        // The tab stays open while the mode keeps it (DESIGN.md §7.3.2).
        self.tab = self.shown_tab();
        self.output(format!(
            "Çalışma modu: {}. Gizlenen komutlar komut satırından ve kısayoluyla yine çalışır.",
            mode.label
        ));
    }

    /// The status bar's mode: its mark and name; a click lists the modes.
    pub(crate) fn mode_cell(&self) -> Element<'static, Message> {
        let current = self.work_mode();
        let mode = effective_mode(Some(current));
        let set = self.document.as_ref().and_then(|d| d.settings().workspace);
        let note = match set {
            Some(saved) if saved != current => {
                let saved = catalog().modes().iter().find(|m| m.id == saved);
                format!(
                    " Proje “{}” modunda kaydedilmiş; bu mod yakında geliyor, şimdilik Hibrit gösteriliyor.",
                    saved.map_or("?", |m| m.label)
                )
            }
            _ => String::new(),
        };
        let face = row![
            icon(mode_icon(current))
                .size(13.0)
                .tone(kentos_ui::icon::Tone::Accent),
            kentos_ui::label::strong(mode.label),
        ]
        .spacing(5)
        .align_y(iced::Center);
        let menu = move || {
            let modes = catalog().modes();
            let ready = modes.iter().filter(|m| m.ready).fold(
                Menu::new().header("Çalışma modu"),
                |menu, m| {
                    menu.check(
                        if m.id == Workspace::Hybrid {
                            format!("{} ({})", m.label, m.title)
                        } else {
                            m.label.to_owned()
                        },
                        m.id == current,
                        Message::Run(mode_command(m.id)),
                    )
                },
            );
            modes
                .iter()
                .filter(|m| !m.ready)
                .fold(ready.separator(), |menu, m| {
                    menu.item(m.label, None).shortcut("Yakında")
                })
        };
        tip(
            MenuButton::new(
                iced::widget::container(face).padding([0, 6]),
                menu,
            ),
            Tip::new(format!("Çalışma modu: {}", mode.label)).body(format!(
                "{}{note} Proje ayarıdır; değiştirmek için tıklayın. Gizlenen komutlar komut satırından yine çalışır.",
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
        assert_eq!(app.work_mode(), Workspace::Hybrid);
        let steps = app.document.as_ref().map(|d| d.model.can_undo());
        run(&mut app, "workspace.cad");
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(doc.settings().workspace, Some(Workspace::Cad));
        assert!(doc.dirty(), "a project setting changed");
        assert_eq!(Some(doc.model.can_undo()), steps, "not an undo step");
        assert_eq!(shown(&app), "CAD");
        assert_eq!(app.checked("workspace.cad"), Some(true));
        assert_eq!(app.checked("workspace.hybrid"), Some(false));
        assert!(last_said(&app).contains("komut satırından"));
    }

    #[test]
    fn announced_modes_say_soon_and_change_nothing() {
        let mut app = app_with_drawing();
        run(&mut app, "workspace.plan3d");
        assert_eq!(app.work_mode(), Workspace::Hybrid);
        assert!(!app.document.as_ref().expect("a drawing").dirty());
        // Without a drawing there is no project to set.
        let (mut empty, _) = App::boot(None);
        run(&mut empty, "workspace.gis");
        assert!(last_said(&empty).contains("önce bir çizim açın"));
    }

    #[test]
    fn the_ribbon_follows_the_mode_and_a_hidden_tab_gives_way_to_giris() {
        let mut app = app_with_drawing();
        let hybrid: Vec<&str> = app.ribbon_tabs().map(|t| t.label).collect();
        assert!(hybrid.contains(&"Harita") && hybrid.contains(&"İşlemler"));
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
        // A tab the mode hides gives way to Giriş.
        run(&mut app, "workspace.hybrid");
        app.tab = "processing";
        run(&mut app, "workspace.cad");
        assert_eq!(app.shown_tab(), "home");
        // A tab the mode keeps stays open.
        app.tab = "modify";
        run(&mut app, "workspace.gis");
        assert_eq!(app.shown_tab(), "modify");
    }
}
