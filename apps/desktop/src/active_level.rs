//! Etkin kat (docs/adr/0713): the level new walls, columns, stairs and ramps go on — Archicad's current story, a new
//! element's home story. Each open drawing has its own (a tab set aside keeps it); a drawing opens on the level it
//! was saved on, else its ground level (the one at elevation 0, else its first) (docs/adr/0720 §1–§2, Revit's last
//! viewed view). Saving writes it into the file, no edit of the drawing. The status bar's Kat cell shows it while the
//! project has levels; its menu chooses another level, Üst kat / Alt kat (Ctrl+↑ / Ctrl+↓), Katsız (no level) or
//! Katları düzenle. The tools write it in their commands' input (CMD-07) and see a change at once.
//!
//! Kat görünümü (docs/adr/0715), kept and saved the same way: Bütün katlar, Yalnız etkin kat, or Etkin kat ve alt
//! kat (the level under it faint, snapped to but not picked). The same menu chooses it, and Mimari › Proje's.

use kentos_commands::{ArchLevel, ProjectSettings};
use kentos_interaction::level_view::LevelView;
use kentos_ui::widget::Menu;
use kentos_ui::widget::status_bar::Readout;

use crate::app::{App, Message};
use kentos_interaction::Level;

/// Kat görünümü by the end of its command's id (`view.levels.all` …).
pub(crate) fn level_view_named(name: &str) -> Option<LevelView> {
    LevelView::ALL.into_iter().find(|v| v.name() == name)
}

/// What a drawing's settings say it opens with (docs/adr/0720 §1): its saved active level (a level, Katsız, else its
/// ground level) and its saved view (else Bütün katlar).
pub(crate) fn opened_with(settings: &ProjectSettings) -> (Chosen, LevelView) {
    let arch = settings.arch.as_ref();
    let chosen = match arch.and_then(|a| a.active_level.as_ref()) {
        None => Chosen::Ground,
        Some(None) => Chosen::Unassigned,
        Some(Some(id)) => Chosen::Level(id.clone()),
    };
    (chosen, arch.and_then(|a| a.level_view).unwrap_or_default())
}

/// The drawing being saved says the level and view it opens with (docs/adr/0720 §1): those on screen, written into
/// the file's settings, not into the drawing (no undo step, no unsaved work). Its ground level and Bütün katlar are
/// said by saying nothing, so a drawing left on them keeps its bytes; a project without levels says neither.
pub(crate) fn with_level_state(settings: &mut ProjectSettings, chosen: &Chosen, view: LevelView) {
    let has_levels = settings.arch.as_ref().is_some_and(|a| !a.levels.is_empty());
    let active = match chosen {
        Chosen::Unassigned if has_levels => Some(None),
        _ => {
            let now = effective(chosen, settings).map(str::to_owned);
            let ground = effective(&Chosen::Ground, settings).map(str::to_owned);
            (now != ground).then_some(now)
        }
    };
    let view = (has_levels && view != LevelView::All).then_some(view);
    if let Some(arch) = settings.arch.as_mut() {
        (arch.active_level, arch.level_view) = (active, view);
    }
}

/// The level above (`up`) or under the active one by its floor's elevation, the project's order among equals; from
/// Katsız (none), the ground level. None at the top or the bottom.
pub(crate) fn stepped<'a>(settings: &'a ProjectSettings, active: Option<&str>, up: bool) -> Option<&'a ArchLevel> {
    let levels = settings.arch.as_ref().map_or(&[][..], |a| a.levels.as_slice());
    let mut order: Vec<&ArchLevel> = levels.iter().collect();
    order.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
    let Some(at) = active.and_then(|id| order.iter().position(|l| l.id == id)) else {
        let ground = effective(&Chosen::Ground, settings)?;
        return order.into_iter().find(|l| l.id == ground);
    };
    if up { order.get(at + 1).copied() } else { at.checked_sub(1).and_then(|k| order.get(k).copied()) }
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
        self.tool_sees_level();
        self.say(Level::Info, said);
    }

    /// Üst kat / Alt kat (docs/adr/0720 §5; Archicad's Go Up/Down a Story, Ctrl+↑/↓): the level above or under the
    /// active one becomes Etkin kat; at the top or the bottom it stays and that is said.
    pub(crate) fn step_level(&mut self, up: bool) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let settings = doc.model.settings();
        if settings.arch.as_ref().is_none_or(|a| a.levels.is_empty()) {
            self.say(Level::Info, "Projede kat yok: Mimari › Proje › Katlar'dan kat ekleyin.");
            return;
        }
        let active = self.active_level();
        match stepped(settings, active.as_deref(), up).map(|l| l.id.clone()) {
            Some(id) => self.choose_level(Some(id)),
            None if up => self.say(Level::Info, "Etkin kat en üst kat; üstünde kat yok."),
            None => self.say(Level::Info, "Etkin kat en alt kat; altında kat yok."),
        }
    }

    /// The running tool sees Etkin kat or Kat görünümü changed at once (docs/adr/0720 §4): its settings and preview
    /// follow, and where the pointer rests on the drawing it is seen there again.
    fn tool_sees_level(&mut self) {
        if !self.session.is_running() {
            return;
        }
        self.with_tool(|s, cx| s.level_changed(cx));
        self.repoint();
    }

    /// The level and view the drawing on screen opens with, as its file says (docs/adr/0720 §1); said after an open.
    pub(crate) fn take_level_state(&mut self) {
        if let Some(doc) = &self.document {
            (self.active_level, self.level_view) = opened_with(doc.model.settings());
        }
        self.sync_level_view();
    }

    /// A tab brought in front: its own level and view (docs/adr/0720 §2).
    pub(crate) fn give_level_state(&mut self, (chosen, view): (Chosen, LevelView)) {
        (self.active_level, self.level_view) = (chosen, view);
        self.sync_level_view();
    }

    /// After an open: the level and view the file kept, said when they are not its ground level and all levels.
    pub(crate) fn say_level_state(&mut self) {
        if self.active_level == Chosen::Ground && self.level_view == LevelView::All {
            return;
        }
        let Some(words) = self.level_words() else {
            return;
        };
        let name = words.trim_start_matches("Kat: ").to_owned();
        self.say(
            Level::Info,
            format!("Çizim kaydedildiği katta açıldı: etkin kat {name}, kat görünümü {}.", self.level_view.label()),
        );
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
        self.tool_sees_level();
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
    const ON_LEVEL: [&str; 9] = [
        // A fixture goes on Etkin kat too (docs/adr/0733 §1).
        "blockInsert",
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
            menu = menu.radio("Katsız", now.is_none(), Some(Message::ActiveLevel(None))).separator();
            // Üst kat and Alt kat (docs/adr/0720 §5), their keys shown.
            for (id, title) in [("view.levels.up", "Üst kat"), ("view.levels.down", "Alt kat")] {
                menu = menu.item(title, Message::Run(id)).icon(crate::icons::from_web(Some(if id.ends_with("up") {
                    "levelUp"
                } else {
                    "levelDown"
                })));
                if let Some(keys) = crate::catalog::catalog().get(id).and_then(|c| c.shortcuts.first()) {
                    menu = menu.shortcut(*keys);
                }
            }
            menu = menu.separator().header("Kat görünümü");
            for v in LevelView::ALL {
                menu = menu.radio(v.label(), view == v, Some(Message::LevelView(v)));
            }
            menu.separator()
                .item("Katları düzenle…", Message::Run("arch.settings"))
                .icon(crate::icons::from_web(Some("archSettings")))
        }
    }

    /// The Kat cell's words while the project has levels: “Kat: Zemin kat”, “Kat: Katsız”.
    pub(crate) fn level_words(&self) -> Option<String> {
        // A building's (module_surfaces.rs): the Kat cell and Etkin aracın ayarları' Kat row go with it.
        if !self.shows_cell(crate::module_surfaces::Cell::Level) {
            return None;
        }
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

    /// The app with a CAD drawing at 1:100 in a 1000 × 700 view, its three levels, snapping off.
    fn levelled() -> App {
        let (mut app, _) = App::boot(None);
        crate::wall_scenes::opened(&mut app, crate::tools_scenes::Objects::new(), 100.0);
        let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0)),
        )));
        crate::wall_scenes::fit(&mut app, 8.0, 6.0);
        app.draft.snap = false;
        with_levels(&mut app);
        app
    }

    fn said(app: &App, words: &str) -> bool {
        app.log.lines().any(|l| l.text.contains(words))
    }

    /// docs/adr/0720 §1–§2: the level and view on screen go into the file when it is saved, not into the drawing (it
    /// stays saved, nothing to undo); the file opens on them and says so; Katsız too. The ground level with all levels
    /// writes nothing. A drawing set aside in its tab keeps its own.
    #[test]
    fn a_drawing_opens_on_the_level_and_view_it_was_saved_with() {
        use crate::files_testing::{drive, scratch};
        let dir = scratch("active-level");
        let path = dir.join("katlar.kcad");
        let mut app = levelled();
        wall(&mut app);
        let save = |app: &mut App| {
            let task = app.start_saving(path.clone());
            drive(app, task);
        };
        let kept = |path: &std::path::Path| {
            let arch = kentos_kcad::decode(&std::fs::read(path).expect("file")).expect("reads").settings.arch.expect("arch");
            (arch.active_level, arch.level_view)
        };
        save(&mut app);
        assert_eq!(kept(&path), (None, None), "the ground level with all levels: nothing written");
        let _ = app.update(Message::ActiveLevel(Some("kat-1".into())));
        let _ = app.update(Message::LevelView(LevelView::Below));
        assert!(!app.document.as_ref().expect("a drawing").dirty(), "choosing a level leaves the drawing saved");
        save(&mut app);
        assert_eq!(kept(&path), (Some(Some("kat-1".into())), Some(LevelView::Below)));
        // Opened again: on 1. kat with the level under it faint, said.
        let (mut again, _) = App::boot(None);
        let task = again.start_opening(path.clone(), crate::opening::Purpose::File);
        drive(&mut again, task);
        assert_eq!(again.active_level().as_deref(), Some("kat-1"));
        assert_eq!(again.level_view, LevelView::Below);
        assert_eq!(again.level_words().as_deref(), Some("Kat: 1. kat"));
        assert!(said(&again, "kaydedildiği katta açıldı: etkin kat 1. kat, kat görünümü Etkin kat ve alt kat"));
        assert_eq!(again.spatial.level_looks().1.faded.len(), 1, "the ground's wall faint under 1. kat");
        // Katsız, Yalnız etkin kat: written and read back.
        let _ = again.update(Message::ActiveLevel(None));
        let _ = again.update(Message::LevelView(LevelView::Active));
        let task = again.start_saving(path.clone());
        drive(&mut again, task);
        assert_eq!(kept(&path), (Some(None), Some(LevelView::Active)));
        let (mut third, _) = App::boot(None);
        let task = third.start_opening(path.clone(), crate::opening::Purpose::File);
        drive(&mut third, task);
        assert_eq!((third.active_level(), third.level_view), (None, LevelView::Active));
        assert_eq!(third.level_words().as_deref(), Some("Kat: Katsız"));
        // A tab: the drawing on screen keeps its level while another is in front.
        let task = third.start_opening(path.clone(), crate::opening::Purpose::Tab);
        drive(&mut third, task);
        assert_eq!(third.doc_tabs.count(true), 2);
        let _ = third.update(Message::ActiveLevel(Some("bodrum".into())));
        let _ = third.update(Message::LevelView(LevelView::All));
        let _ = third.update(Message::DocTabs(crate::doc_tabs::Event::Show(0)));
        assert_eq!((third.active_level(), third.level_view), (None, LevelView::Active), "the first tab's own");
        let _ = third.update(Message::DocTabs(crate::doc_tabs::Event::Show(1)));
        assert_eq!((third.active_level().as_deref(), third.level_view), (Some("bodrum"), LevelView::All));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// docs/adr/0720 §3: a level a wall names is kept when Proje ayarları takes it away: said by its name with the
    /// count, the wall selected to be given another level; once it is, the level goes, and Etkin kat on it gives way
    /// to the ground. A level renamed keeps its objects and its new name shows.
    #[test]
    fn a_level_in_use_stays_its_objects_selected_and_a_renamed_one_keeps_them() {
        use crate::project::Event as Project;
        use crate::project::arch::Edit;
        use crate::project::settings::Event as Settings;
        let mut app = levelled();
        let _ = app.update(Message::ActiveLevel(Some("kat-1".into())));
        wall(&mut app);
        let upper = newest_wall(&app);
        let send = |app: &mut App, e: Settings| {
            let _ = app.update(Message::Project(Box::new(Project::Settings(e))));
        };
        // Renamed: “1. kat” → “Birinci kat”; the wall keeps the level, the cell says the new name.
        let _ = app.update(Message::Run("arch.settings"));
        send(&mut app, Settings::Level(2, Edit::Name("Birinci kat".into())));
        send(&mut app, Settings::Save);
        assert_eq!(app.level_words().as_deref(), Some("Kat: Birinci kat"));
        assert_eq!(newest_level(&app).as_deref(), Some("kat-1"));
        // Taken away while the wall names it: kept, said, the wall selected.
        app.selection.set(Vec::<kentos_domain::Slot>::new());
        let _ = app.update(Message::Run("arch.settings"));
        send(&mut app, Settings::Level(2, Edit::Delete));
        send(&mut app, Settings::Save);
        let levels = |app: &App| {
            app.document.as_ref().expect("a drawing").model.settings().arch.as_ref().map_or(0, |a| a.levels.len())
        };
        assert_eq!(levels(&app), 3, "kept");
        assert!(said(&app, "“Birinci kat” katını 1 duvar anıyor; kaldırılmadı."), "{:?}", app.log.last());
        assert!(said(&app, "Andığı 1 nesne seçildi."));
        assert_eq!(app.selection.ids(), &[upper]);
        // The window's Nesneleri seç (Kaydet waits there): the wall selected, the window closed as Vazgeç would.
        app.selection.set(Vec::<kentos_domain::Slot>::new());
        send(&mut app, Settings::SelectLevelUsers);
        assert!(app.project.is_none(), "closed");
        assert_eq!(app.selection.ids(), &[upper]);
        assert!(said(&app, "Katı anan 1 nesne seçildi"));
        assert_eq!(levels(&app), 3);
        // The wall on Zemin kat (Öznitelikler's Kat): then the level goes, Etkin kat on it gives way to the ground.
        let doc = app.document.as_mut().expect("a drawing");
        let uid = doc.model.uid(upper).expect("uid").to_string();
        let result = kentos_native_application::levels_assign::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_commands::LevelsAssign { level: "zemin".into(), uids: Some(vec![uid]), expected_revision: None },
        );
        assert!(matches!(result, kentos_commands::CommandResult::Completed { .. }), "{result:?}");
        let _ = app.update(Message::Run("arch.settings"));
        send(&mut app, Settings::Level(2, Edit::Delete));
        send(&mut app, Settings::Save);
        assert_eq!(levels(&app), 2, "taken away");
        assert_eq!(app.active_level().as_deref(), Some("zemin"));
        assert_eq!(app.level_words().as_deref(), Some("Kat: Zemin kat"));
    }

    /// docs/adr/0720 §5: Üst kat and Alt kat (Ctrl+↑, Ctrl+↓) step by the floors' elevations, stop at the top and the
    /// bottom and say so; from Katsız they start at the ground level.
    #[test]
    fn ust_kat_and_alt_kat_step_through_the_levels() {
        let mut app = levelled();
        assert_eq!(app.shortcut("Ctrl+Up"), Some("view.levels.up"));
        assert_eq!(app.shortcut("Ctrl+Down"), Some("view.levels.down"));
        let _ = app.update(Message::Run("view.levels.up"));
        assert_eq!(app.active_level().as_deref(), Some("kat-1"));
        let _ = app.update(Message::Run("view.levels.up"));
        assert_eq!(app.active_level().as_deref(), Some("kat-1"));
        assert!(said(&app, "Etkin kat en üst kat; üstünde kat yok."));
        for _ in 0..2 {
            let _ = app.update(Message::Run("view.levels.down"));
        }
        assert_eq!(app.active_level().as_deref(), Some("bodrum"));
        let _ = app.update(Message::Run("view.levels.down"));
        assert!(said(&app, "Etkin kat en alt kat; altında kat yok."));
        let _ = app.update(Message::ActiveLevel(None));
        let _ = app.update(Message::Run("view.levels.up"));
        assert_eq!(app.active_level().as_deref(), Some("zemin"), "from Katsız, the ground");
    }

    /// docs/adr/0720 §4: Etkin kat changed on the status bar while Merdiven runs: its height row is the new level's at
    /// once, the pointer away from the drawing.
    #[test]
    fn a_running_stair_follows_the_level_chosen_on_the_status_bar() {
        let mut app = levelled();
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.model.settings().clone();
        for (l, h) in settings.arch.as_mut().expect("arch").levels.iter_mut().zip([2.6, 3.06, 2.8]) {
            l.height = Some(h);
        }
        doc.model.set_settings(settings);
        crate::tools_scenes::run(&mut app, "tool.stair");
        let height = |app: &App| {
            app.session.settings(&app.format()).into_iter().find(|r| r.key == "Y").map(|r| r.value).expect("Yükseklik")
        };
        assert!(height(&app).ends_with("· katın") && height(&app).starts_with("3.06"), "{}", height(&app));
        let _ = app.update(Message::Viewport(crate::viewport::Event::Left));
        let _ = app.update(Message::ActiveLevel(Some("kat-1".into())));
        assert!(height(&app).starts_with("2.80"), "{}", height(&app));
        let _ = app.update(Message::Run("view.levels.down"));
        assert!(height(&app).starts_with("3.06"), "{}", height(&app));
    }
}
