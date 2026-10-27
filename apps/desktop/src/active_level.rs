//! Etkin kat (docs/adr/0713): the level new walls, columns, stairs and ramps go on — Archicad's current story, a new
//! element's home story. It is the session's, not the drawing's: a drawing opens on its ground level (the one at
//! elevation 0, else its first). The status bar's Kat cell shows it while the project has levels; its menu chooses
//! another level, Katsız (no level) or Katları düzenle. The tools write it in their commands' input (CMD-07).
//!
//! Kat görünümü (docs/adr/0715), the session's too: Bütün katlar, Yalnız etkin kat, or Etkin kat ve alt kat (the
//! level under it faint, snapped to but not picked). The same menu chooses it, and Mimari › Proje's.

use kentos_commands::ProjectSettings;
use kentos_interaction::level_view::LevelView;
use kentos_ui::widget::Menu;
use kentos_ui::widget::status_bar::Readout;

use crate::app::{App, Message};
use kentos_interaction::Level;

/// Kat görünümü by the end of its command's id (`view.levels.all` …).
pub(crate) fn level_view_named(name: &str) -> Option<LevelView> {
    match name {
        "all" => Some(LevelView::All),
        "active" => Some(LevelView::Active),
        "below" => Some(LevelView::Below),
        _ => None,
    }
}

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
        self.sync_level_view();
        self.say(Level::Info, said);
    }

    /// The name of the level with id `id` in the open drawing, else the id.
    fn level_name(&self, id: &str) -> String {
        self.document
            .as_ref()
            .and_then(|doc| doc.model.settings().arch.as_ref())
            .and_then(|a| a.levels.iter().find(|l| l.id == id))
            .map_or_else(|| id.to_owned(), |l| l.name.clone())
    }

    /// Kat görünümü chosen (docs/adr/0715): the plan shows it at once; what it hides or draws faint leaves the
    /// selection.
    pub(crate) fn choose_level_view(&mut self, view: LevelView) {
        self.level_view = view;
        self.sync_level_view();
        let active = self.active_level();
        let said = match (view, active) {
            (LevelView::All, _) => "Kat görünümü: Bütün katlar.".to_owned(),
            (LevelView::Active, Some(id)) => format!(
                "Kat görünümü: yalnız {}. Öbür katların nesneleri gizli: seçilmez, kenetlenmez.",
                self.level_name(&id)
            ),
            (LevelView::Below, Some(id)) => {
                let under = self.document.as_ref().and_then(|doc| {
                    kentos_interaction::level_view::below(doc.model.settings(), &id).map(|l| l.name.clone())
                });
                match under {
                    Some(under) => format!(
                        "Kat görünümü: {} ve altında {} soluk: soluk kat kenetlenir, seçilmez.",
                        self.level_name(&id),
                        under
                    ),
                    None => format!(
                        "Kat görünümü: {}; altında kat yok, yalnız o çizilir.",
                        self.level_name(&id)
                    ),
                }
            }
            (_, None) => "Kat görünümü: Katsız; yalnız katı olmayan nesneler çizilir.".to_owned(),
        };
        self.say(Level::Info, said);
    }

    /// Kat görünümü with Etkin kat into the store (docs/adr/0715 §2): what it hides is neither picked nor snapped to,
    /// what it draws faint is snapped to only; the selection keeps what can still be picked.
    pub(crate) fn sync_level_view(&mut self) {
        let active = self.active_level();
        let Some(doc) = &self.document else {
            return;
        };
        self.spatial.set_level_view(&doc.model, self.level_view, active);
        if self.level_view != LevelView::All {
            let store = self.spatial.store();
            self.selection.retain(|slot| {
                store
                    .get(f64::from(slot.0))
                    .is_some_and(|it| store.pickable(it))
            });
        }
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
        let view = self.level_view;
        move || {
            let mut menu = Menu::new().header("Etkin kat");
            for (id, name) in &levels {
                menu = menu.radio(
                    name.clone(),
                    now.as_deref() == Some(id.as_str()),
                    Some(Message::ActiveLevel(Some(id.clone()))),
                );
            }
            menu = menu
                .radio("Katsız", now.is_none(), Some(Message::ActiveLevel(None)))
                .separator()
                .header("Kat görünümü");
            for v in [LevelView::All, LevelView::Active, LevelView::Below] {
                menu = menu.radio(v.name(), view == v, Some(Message::LevelView(v)));
            }
            menu.separator()
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

    /// The Kat cell's icon: Kat görünümü's (docs/adr/0715 §3), so the view shows without taking the bar's room.
    fn level_icon(&self) -> &'static str {
        match self.level_view {
            LevelView::All => "levelsAll",
            LevelView::Active => "levelsActive",
            LevelView::Below => "levelsBelow",
        }
    }

    /// The status bar's Kat cell (docs/adr/0713) while the project has levels: Etkin kat's name, its words while
    /// `words`, the icon alone once tight; its menu chooses another level, Katsız or Katları düzenle.
    pub(crate) fn level_cell(&self, words: bool) -> Option<iced::Element<'_, Message>> {
        let text = self.level_words()?;
        let menu = self.level_menu();
        Some(
            Readout::new(kentos_ui::label::body(if words { text } else { String::new() }))
            .icon(crate::icons::from_web(Some(self.level_icon())))
            .menu(menu)
            .tip(match self.level_view {
                LevelView::All => "Etkin kat: yeni duvar, kolon, merdiven ve rampaların katı; mahal ve ölçü araçları o kattaki duvarlara bakar. Kat görünümü: Bütün katlar. Başka kat ya da görünüm için tıklayın.",
                LevelView::Active => "Etkin kat: yeni duvar, kolon, merdiven ve rampaların katı. Kat görünümü: yalnız etkin kat; öbür katlar gizli. Başka kat ya da görünüm için tıklayın.",
                LevelView::Below => "Etkin kat: yeni duvar, kolon, merdiven ve rampaların katı. Kat görünümü: etkin kat ve alt kat soluk (kenetlenir, seçilmez). Başka kat ya da görünüm için tıklayın.",
            })
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

    /// The slot of the wall written last.
    fn newest_wall(app: &App) -> kentos_domain::Slot {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.entities()
            .filter(|e| matches!(e, kentos_commands::Entity::Wall(_)))
            .map(|e| kentos_domain::Slot(e.base().id))
            .max()
            .expect("a wall")
    }

    /// Kat görünümü (docs/adr/0715): two walls stacked on Zemin kat and 1. kat; with 1. kat active, Etkin kat ve alt
    /// kat leaves the ground's out of picking (drawn faint), Yalnız etkin kat out of snapping too, Bütün katlar
    /// brings it back. The Kat cell says the view; a wall the view leaves out leaves the selection.
    #[test]
    fn the_level_view_shows_and_picks_the_active_level() {
        let (mut app, _) = App::boot(None);
        crate::wall_scenes::opened(&mut app, crate::tools_scenes::Objects::new(), 100.0);
        let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0)),
        )));
        crate::wall_scenes::fit(&mut app, 8.0, 6.0);
        app.draft.snap = false;
        with_levels(&mut app);
        wall(&mut app);
        let ground = newest_wall(&app);
        let _ = app.update(Message::ActiveLevel(Some("kat-1".into())));
        wall(&mut app);
        let first = newest_wall(&app);
        assert_ne!(ground, first);
        let at = kentos_interaction::Vec2::new(crate::tools_scenes::E + 2.5, crate::tools_scenes::N);
        let hits = |app: &App| app.spatial.hits(at, 0.2);
        assert!(hits(&app).contains(&ground) && hits(&app).contains(&first), "Bütün katlar: both");
        app.selection.set([ground, first]);
        let _ = app.update(Message::Run("view.levels.below"));
        assert_eq!(app.level_view, LevelView::Below);
        assert_eq!(app.level_words().as_deref(), Some("Kat: 1. kat"));
        assert_eq!(app.level_icon(), "levelsBelow");
        assert_eq!(hits(&app), vec![first], "the ground's is faint: not picked");
        assert_eq!(app.spatial.level_looks().1.faded, vec![ground.0]);
        assert_eq!(app.selection.ids(), &[first], "the faint wall left the selection");
        let kinds = kentos_interaction::default_snap_kinds();
        let end = kentos_interaction::Vec2::new(crate::tools_scenes::E + 5.0, crate::tools_scenes::N);
        let snapped = |app: &App| app.spatial.snap(end, 0.3, kinds, None).map(|h| h.id as u32);
        assert!(snapped(&app).is_some(), "an end snapped to");
        let _ = app.update(Message::LevelView(LevelView::Active));
        assert_eq!(app.level_icon(), "levelsActive");
        assert_eq!(app.spatial.level_looks().1.hidden, vec![ground.0]);
        assert_eq!(hits(&app), vec![first]);
        // Zemin kat active: 1. kat's wall is the one left out now.
        let _ = app.update(Message::ActiveLevel(Some("zemin".into())));
        assert_eq!(app.spatial.level_looks().1.hidden, vec![first.0]);
        assert_eq!(hits(&app), vec![ground]);
        let _ = app.update(Message::Run("view.levels.all"));
        assert!(app.spatial.level_looks().1.is_empty());
        assert_eq!(hits(&app).len(), 2, "Bütün katlar: both again");
        assert_eq!(app.level_words().as_deref(), Some("Kat: Zemin kat"));
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

    /// docs/adr/0718: Çıktı › Kat planları from the drawing: a sheet per level, each map drawing its level, in front;
    /// the sheet mode's Geri al takes them all back.
    #[test]
    fn kat_planlari_make_a_sheet_per_level_from_the_drawing() {
        let mut app = app_with_drawing();
        make_cad(&mut app);
        with_levels(&mut app);
        wall(&mut app);
        let task = app.run("sheet.levels");
        crate::files_testing::drive(&mut app, task);
        if app.sheets.questions().is_some() {
            let task = app.update(Message::Sheet(kentos_sheet_ui::Message::Questions(
                kentos_sheet_ui::questions::QuestionsMessage::Make,
            )));
            crate::files_testing::drive(&mut app, task);
        }
        let names: Vec<String> = app.sheets.book().sheets.iter().map(|s| s.name.clone()).collect();
        assert_eq!(names, ["A-101 Bodrum planı", "A-102 Zemin kat planı", "A-103 1. kat planı"]);
        assert!(app.sheets.is_active(), "the first sheet in front");
        let task = app.update(Message::Sheet(kentos_sheet_ui::Message::Undo));
        crate::files_testing::drive(&mut app, task);
        assert!(app.sheets.book().sheets.is_empty(), "one step");
    }
}
