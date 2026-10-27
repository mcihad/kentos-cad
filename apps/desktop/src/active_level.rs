//! Etkin kat (docs/adr/0713): the level new walls, columns, stairs and ramps go on — Archicad's current story, a new
//! element's home story. It is the session's, not the drawing's: a drawing opens on its ground level (the one at
//! elevation 0, else its first). The status bar's Kat cell shows it while the project has levels; its menu chooses
//! another level, Katsız (no level) or Katları düzenle. The tools write it in their commands' input (CMD-07).

use kentos_commands::ProjectSettings;
use kentos_ui::widget::Menu;
use kentos_ui::widget::status_bar::Readout;

use crate::app::{App, Message};
use kentos_interaction::Level;

/// What was chosen.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum Chosen {
    /// Nothing yet: the drawing's ground level.
    #[default]
    Ground,
    /// Katsız: no level.
    Unassigned,
    /// A level, by its id.
    Level(String),
}

/// The level in force for a drawing of `settings`: the one chosen while the drawing has it, else its ground level;
/// none without levels or with Katsız.
pub(crate) fn effective<'a>(chosen: &'a Chosen, settings: &'a ProjectSettings) -> Option<&'a str> {
    let levels = settings.arch.as_ref().map_or(&[][..], |a| a.levels.as_slice());
    if levels.is_empty() {
        return None;
    }
    match chosen {
        Chosen::Unassigned => None,
        Chosen::Level(id) if levels.iter().any(|l| l.id == *id) => Some(id.as_str()),
        _ => levels
            .iter()
            .find(|l| l.elevation == 0.0)
            .or(levels.first())
            .map(|l| l.id.as_str()),
    }
}

impl App {
    /// Etkin kat for the open drawing, owned (the tools' context borrows the drawing mutably).
    pub(crate) fn active_level(&self) -> Option<String> {
        let doc = self.document.as_ref()?;
        effective(&self.active_level, doc.model.settings()).map(str::to_owned)
    }

    /// Kat chosen on the status bar's menu: a level by its id, or none (Katsız).
    pub(crate) fn choose_level(&mut self, level: Option<String>) {
        self.active_level = match level {
            Some(id) => Chosen::Level(id),
            None => Chosen::Unassigned,
        };
        let said = match (self.active_level(), &self.document) {
            (Some(id), Some(doc)) => {
                let name = doc
                    .model
                    .settings()
                    .arch
                    .as_ref()
                    .and_then(|a| a.levels.iter().find(|l| l.id == id))
                    .map_or(id.clone(), |l| l.name.clone());
                format!("Etkin kat: {name}. Yeni duvar, kolon, merdiven ve rampalar bu kata çizilir.")
            }
            _ => "Etkin kat yok (Katsız): yeni mimari nesneler kata atanmaz.".to_owned(),
        };
        self.say(Level::Info, said);
    }

    /// The tools that draw on Etkin kat: their settings show it first (docs/adr/0713).
    const ON_LEVEL: [&str; 8] = [
        "wall",
        "wallRectangle",
        "wallPartition",
        "column",
        "stair",
        "stairCurved",
        "ramp",
        "gridSystem",
    ];

    /// Etkin kat's row in a drawing tool's settings while the project has levels: its name and the status bar's
    /// menu.
    pub(crate) fn level_setting(&self, tool: &str) -> Option<iced::Element<'_, Message>> {
        if !Self::ON_LEVEL.contains(&tool) {
            return None;
        }
        let words = self.level_words()?;
        let shown = words.trim_start_matches("Kat: ").to_owned();
        let menu = self.level_menu();
        Some(kentos_ui::widget::property_grid::choice(shown, None, menu))
    }

    /// The Kat menu: the levels, Katsız, Katları düzenle.
    fn level_menu(&self) -> impl Fn() -> Menu<Message> + 'static {
        let levels: Vec<(String, String)> = self
            .document
            .as_ref()
            .and_then(|doc| doc.model.settings().arch.as_ref())
            .map(|a| a.levels.iter().map(|l| (l.id.clone(), l.name.clone())).collect())
            .unwrap_or_default();
        let now = self.active_level();
        move || {
            let mut menu = Menu::new().header("Etkin kat");
            for (id, name) in &levels {
                menu = menu.radio(
                    name.clone(),
                    now.as_deref() == Some(id.as_str()),
                    Some(Message::ActiveLevel(Some(id.clone()))),
                );
            }
            menu.radio("Katsız", now.is_none(), Some(Message::ActiveLevel(None)))
                .separator()
                .item("Katları düzenle…", Message::Run("arch.settings"))
                .icon(crate::icons::from_web(Some("archSettings")))
        }
    }

    /// The Kat cell's words while the project has levels: “Kat: Zemin kat”, “Kat: Katsız”.
    pub(crate) fn level_words(&self) -> Option<String> {
        let doc = self.document.as_ref()?;
        let levels = doc.model.settings().arch.as_ref().map(|a| &a.levels)?;
        if levels.is_empty() {
            return None;
        }
        let name = self.active_level().map_or("Katsız".to_owned(), |id| {
            levels
                .iter()
                .find(|l| l.id == id)
                .map_or(id.clone(), |l| l.name.clone())
        });
        Some(format!("Kat: {name}"))
    }

    /// The status bar's Kat cell (docs/adr/0713) while the project has levels: Etkin kat's name, its words while
    /// `words`, the icon alone once tight; its menu chooses another level, Katsız or Katları düzenle.
    pub(crate) fn level_cell(&self, words: bool) -> Option<iced::Element<'_, Message>> {
        let text = self.level_words()?;
        let menu = self.level_menu();
        Some(
            Readout::new(kentos_ui::label::body(if words { text } else { String::new() }))
            .icon(crate::icons::from_web(Some("archLevel")))
            .menu(menu)
            .tip("Etkin kat: yeni duvar, kolon, merdiven ve rampaların katı; mahal ve ölçü araçları o kattaki duvarlara bakar. Başka kat seçmek için tıklayın.")
            .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::{app_with_drawing, make_cad};
    use crate::tools_scenes::{click, typed};

    fn with_levels(app: &mut App) {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.model.settings().clone();
        settings.arch.get_or_insert_with(Default::default).levels = vec![
            kentos_commands::ArchLevel {
                id: "bodrum".into(),
                name: "Bodrum".into(),
                elevation: -3.0,
                height: None,
            },
            kentos_commands::ArchLevel {
                id: "zemin".into(),
                name: "Zemin kat".into(),
                elevation: 0.0,
                height: None,
            },
            kentos_commands::ArchLevel {
                id: "kat-1".into(),
                name: "1. kat".into(),
                elevation: 3.0,
                height: None,
            },
        ];
        doc.model.set_settings(settings);
    }

    /// The level of the wall written last.
    fn newest_level(app: &App) -> Option<String> {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.entities()
            .filter_map(|e| match e {
                kentos_commands::Entity::Wall(w) => Some(w),
                _ => None,
            })
            .max_by_key(|w| w.base.id)
            .expect("a wall")
            .wall
            .arch
            .level
            .clone()
    }

    fn wall(app: &mut App) {
        crate::tools_scenes::run(app, "tool.wall");
        click(app, [0.0, 0.0]);
        click(app, [5.0, 0.0]);
        typed(app, "");
        typed(app, "");
    }

    #[test]
    fn a_drawing_opens_on_its_ground_level_and_the_menu_moves_it() {
        // An empty CAD drawing at 1:100 in a 1000 × 700 view of it.
        let (mut app, _) = App::boot(None);
        crate::wall_scenes::opened(&mut app, crate::tools_scenes::Objects::new(), 100.0);
        let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0)),
        )));
        crate::wall_scenes::fit(&mut app, 8.0, 6.0);
        app.draft.snap = false;
        assert_eq!(app.active_level(), None, "no levels, no Kat");
        assert!(app.level_words().is_none(), "nor its cell");
        with_levels(&mut app);
        assert_eq!(app.active_level().as_deref(), Some("zemin"), "the level at 0, not the first");
        assert_eq!(app.level_words().as_deref(), Some("Kat: Zemin kat"));
        wall(&mut app);
        assert_eq!(newest_level(&app).as_deref(), Some("zemin"));
        let _ = app.update(Message::ActiveLevel(Some("kat-1".into())));
        assert_eq!(app.level_words().as_deref(), Some("Kat: 1. kat"));
        wall(&mut app);
        assert_eq!(newest_level(&app).as_deref(), Some("kat-1"));
        let _ = app.update(Message::ActiveLevel(None));
        assert_eq!(app.level_words().as_deref(), Some("Kat: Katsız"));
        wall(&mut app);
        assert_eq!(newest_level(&app), None, "Katsız: on none");
    }

    #[test]
    fn a_level_gone_from_the_project_gives_way_to_the_ground() {
        let mut app = app_with_drawing();
        make_cad(&mut app);
        with_levels(&mut app);
        let _ = app.update(Message::ActiveLevel(Some("kat-1".into())));
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.model.settings().clone();
        if let Some(arch) = settings.arch.as_mut() {
            arch.levels.retain(|l| l.id != "kat-1");
        }
        doc.model.set_settings(settings);
        assert_eq!(app.active_level().as_deref(), Some("zemin"));
    }
}
