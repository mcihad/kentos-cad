//! Komut ara, the tab row's search box (the web's `ui/ribbon/search.ts` and
//! `Ribbon.where` / `reveal`; docs/adr/0077): commands found by name or
//! command-line alias, Turkish letters folded, as the web's registry
//! `search` ranks them (an alias equal first, then an alias beginning with
//! the text, then a title beginning with it, then a title holding it; nine
//! at most). Each row says where the command is on the ribbon (“Değiştir ›
//! Değiştir”), or why it does not run here. Enter or a click runs one; Alt+Enter
//! or the row's pin shows where it lives: its tab opens and its button (or
//! its panel's ▾, or its folded panel) is outlined for a moment. Alt+Q comes
//! here (`view.commandSearch`).

use std::time::Duration;

use iced::Task;
use iced::widget::operation;
use kentos_ui::widget::search_box::Found;

use crate::app::{App, Message};
use crate::catalog::{Command, Item, Panel, Standing, catalog};
use crate::exchange::apply::fold_turkish;
use crate::input::release_keyboard;

/// The search box's field (`view.commandSearch` focuses it).
pub const SEARCH_INPUT: &str = "ribbon-search";

/// How many commands the list shows (the web's `LIMIT`).
const LIMIT: usize = 9;

/// How long a revealed button stays outlined (the web's 1600 ms).
const FLASH: Duration = Duration::from_millis(1600);

/// What Komut ara found: a command of the catalog, or one of the user's
/// models (the web registers each as `processing.model.<id>`, “<label>…”).
pub enum Hit {
    Command(&'static Command),
    Model { id: String, title: String },
}

/// The commands `query` finds, best first (the web's `CommandRegistry.search`;
/// the app ranks the user's models with them, `App::hits`).
#[cfg(test)]
pub fn search(query: &str) -> Vec<&'static Command> {
    scored_commands(&fold_turkish(query))
        .into_iter()
        .take(LIMIT)
        .map(|(_, c)| c)
        .collect()
}

/// Every command the folded text finds, with its rank (lower is better).
fn scored_commands(q: &str) -> Vec<(f64, &'static Command)> {
    if q.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(f64, &'static Command)> = catalog()
        .commands()
        .iter()
        .filter_map(|command| {
            let mut score = f64::INFINITY;
            for alias in command.aliases {
                let alias = fold_turkish(alias);
                if alias == q {
                    score = score.min(0.0);
                } else if alias.starts_with(q) {
                    score = score.min(1.0 + alias.chars().count() as f64 / 100.0);
                }
            }
            let title = fold_turkish(command.title);
            if title.starts_with(q) {
                score = score.min(2.0);
            } else if title.contains(q) {
                score = score.min(3.0);
            }
            score.is_finite().then_some((score, command))
        })
        .collect();
    // Stable: ties keep the catalog's order.
    scored.sort_by(|a, b| a.0.total_cmp(&b.0));
    scored
}

/// Every command a panel offers: its buttons, a family's tools, a menu's
/// items and its seldom used ones (the web's `panelCommands`).
fn panel_commands(panel: &Panel) -> Vec<&'static str> {
    let mut ids = Vec::new();
    for item in &panel.items {
        match item {
            Item::Command { id, .. } => ids.push(*id),
            Item::Split { entries, .. } => ids.extend(entries.iter().map(|e| e.id)),
            Item::Menu { ids: menu, .. } => ids.extend(menu.iter().copied()),
            Item::Builtin(_) => {}
        }
    }
    ids.extend(panel.overflow.iter().copied());
    ids
}

impl App {
    /// The tab and panel a command lives in: the tabs other than Giriş
    /// first, then Giriş (the web's `where` and `reveal`).
    fn home_of(&self, id: &str) -> Option<(&'static str, &'static str, &Panel)> {
        let tabs: Vec<_> = self.ribbon_tabs().collect();
        let order = tabs
            .iter()
            .filter(|tab| tab.id != "home")
            .chain(tabs.iter().filter(|tab| tab.id == "home"));
        for tab in order {
            if let Some(panel) = tab
                .panels
                .iter()
                .find(|panel| panel_commands(panel).contains(&id))
            {
                return Some((tab.id, tab.label, panel));
            }
        }
        None
    }

    /// What the text typed finds: the catalog's commands, then the user's
    /// models by their names, ranked together (models have no aliases;
    /// ties keep the web's order, the models registered after).
    pub(crate) fn hits(&self) -> Vec<Hit> {
        let q = fold_turkish(&self.ribbon_search);
        let mut scored: Vec<(f64, Hit)> = scored_commands(&q)
            .into_iter()
            .map(|(score, c)| (score, Hit::Command(c)))
            .collect();
        if !q.is_empty() {
            for m in self.user_models() {
                let title = format!("{}…", m.label);
                let folded = fold_turkish(&title);
                let score = if folded.starts_with(&q) {
                    2.0
                } else if folded.contains(&q) {
                    3.0
                } else {
                    continue;
                };
                scored.push((
                    score,
                    Hit::Model {
                        id: m.id.clone(),
                        title,
                    },
                ));
            }
        }
        scored.sort_by(|a, b| a.0.total_cmp(&b.0));
        scored.into_iter().take(LIMIT).map(|(_, h)| h).collect()
    }

    /// The rows for the text typed: where each command lives, or why it
    /// does not run here; a command that does not run here is dimmed.
    pub(crate) fn search_rows(&self) -> Vec<Found<'static>> {
        self.hits()
            .into_iter()
            .map(|hit| {
                let command = match hit {
                    Hit::Command(command) => command,
                    Hit::Model { title, .. } => {
                        let home = self.home_of("processing.newModel");
                        return Found {
                            icon: crate::icons::from_web(Some("processing")),
                            title: title.into(),
                            detail: home
                                .map_or_else(
                                    || "İşlemler".to_owned(),
                                    |(_, tab, panel)| format!("{tab} › {}", panel.label),
                                )
                                .into(),
                            shortcut: None,
                            enabled: true,
                            revealable: home.is_some(),
                        };
                    }
                };
                let home = self.home_of(command.id);
                let runs = command.standing == Standing::Ported && self.available(command.id);
                let detail = match (command.standing, &home) {
                    (Standing::Pending, _) => command
                        .pending_note
                        .unwrap_or("Geliştirme aşamasında")
                        .to_owned(),
                    (Standing::OnTheWeb, _) => "Web'de var; masaüstüne henüz taşınmadı".to_owned(),
                    (Standing::Ported, Some((_, tab, panel))) => format!("{tab} › {}", panel.label),
                    (Standing::Ported, None) => command.category.to_owned(),
                };
                Found {
                    icon: command.icon,
                    title: command.title.into(),
                    detail: detail.into(),
                    shortcut: command.shortcuts.first().map(|s| (*s).into()),
                    enabled: runs,
                    revealable: home.is_some(),
                }
            })
            .collect()
    }

    /// The text typed into Komut ara.
    pub(crate) fn search_typed(&mut self, text: String) {
        self.ribbon_search = text;
    }

    /// Enter or a click on a row: the command runs; the box empties and
    /// gives the keyboard back (a tool takes it next, the web's `view.focus`).
    pub(crate) fn search_run(&mut self, index: usize) -> Task<Message> {
        let command = match self.hits().into_iter().nth(index) {
            Some(Hit::Command(command)) => command,
            Some(Hit::Model { id, .. }) => {
                self.ribbon_search.clear();
                let open = self.processing_command(&format!("processing.model.{id}"));
                return Task::batch([release_keyboard(), open]);
            }
            None => return Task::none(),
        };
        if command.standing != Standing::Ported || !self.available(command.id) {
            return Task::none();
        }
        self.ribbon_search.clear();
        Task::batch([release_keyboard(), self.run(command.id)])
    }

    /// Alt+Enter or the row's pin: the command's tab opens and its place is
    /// outlined for a moment (the web's `reveal`).
    pub(crate) fn search_reveal(&mut self, index: usize) -> Task<Message> {
        let command = match self.hits().into_iter().nth(index) {
            Some(Hit::Command(command)) => command,
            // A model's button in the model library's panel.
            Some(Hit::Model { id, .. }) => {
                let Some((tab, ..)) = self.home_of("processing.newModel") else {
                    return Task::none();
                };
                self.ribbon_search.clear();
                self.tab = tab;
                self.ribbon_context = false;
                self.ribbon_collapsed = false;
                self.ribbon_flash_model = Some(id.clone());
                return Task::batch([
                    release_keyboard(),
                    crate::hover_card::after(FLASH, Message::RibbonModelFlashEnd(id)),
                ]);
            }
            None => return Task::none(),
        };
        let Some((tab, ..)) = self.home_of(command.id) else {
            return Task::none();
        };
        self.ribbon_search.clear();
        // Its own tab, never the contextual one (the web's `homeOf`).
        self.tab = tab;
        self.ribbon_context = false;
        // The desktop's folded ribbon has no peek over the drawing: it opens.
        self.ribbon_collapsed = false;
        self.ribbon_flash = Some(command.id);
        Task::batch([
            release_keyboard(),
            crate::hover_card::after(FLASH, Message::RibbonFlashEnd(command.id)),
        ])
    }

    /// The outline goes, unless another reveal took its place.
    pub(crate) fn search_flash_end(&mut self, id: &'static str) {
        if self.ribbon_flash == Some(id) {
            self.ribbon_flash = None;
        }
    }

    /// A model's outline goes, unless another reveal took its place.
    pub(crate) fn search_model_flash_end(&mut self, id: &str) {
        if self.ribbon_flash_model.as_deref() == Some(id) {
            self.ribbon_flash_model = None;
        }
    }

    /// Alt+Q (`view.commandSearch`): the box takes the keyboard, what it
    /// holds selected.
    pub(crate) fn search_focus(&mut self) -> Task<Message> {
        Task::batch([
            operation::focus(SEARCH_INPUT),
            operation::select_all(SEARCH_INPUT),
        ])
    }

    /// Whether the panel holds the command being outlined, and whether it
    /// is under the panel's ▾.
    pub(crate) fn flash_in(&self, panel: &Panel) -> (bool, bool) {
        match self.ribbon_flash {
            Some(id) => (
                panel_commands(panel).contains(&id),
                panel.overflow.contains(&id),
            ),
            // A model outlined is in the model library's panel, never under its ▾.
            None if self.ribbon_flash_model.is_some() => (
                panel_commands(panel).contains(&"processing.newModel"),
                false,
            ),
            None => (false, false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::app_with_drawing;

    fn ids(query: &str) -> Vec<&'static str> {
        search(query).iter().map(|c| c.id).collect()
    }

    /// The web's ranking: an alias equal first, then one beginning with the
    /// text, then titles beginning with it, then holding it; Turkish letters
    /// fold (CIZGI finds Çizgi); nine at most; nothing for blank text.
    #[test]
    fn commands_are_found_by_alias_and_title_as_on_the_web() {
        assert_eq!(ids("L").first(), Some(&"tool.line"));
        assert_eq!(ids("cizgi").first(), Some(&"tool.line"));
        assert_eq!(ids("ÇİZGİ").first(), Some(&"tool.line"));
        assert!(ids("a").len() <= 9);
        assert!(ids("  ").is_empty());
        let found = ids("taşı");
        assert!(found.contains(&"tool.move"), "{found:?}");
    }

    /// The user's models are commands as on the web (`processing.model.<id>`):
    /// a button in the model library's panel, a name on the command line and
    /// a row of Komut ara, each opening the model's window (UX-13).
    #[test]
    fn the_user_s_models_are_on_the_ribbon_the_command_line_and_komut_ara() {
        use crate::app::Dialog;
        let mut app = app_with_drawing();
        let mut model = app
            .processing
            .registry
            .model("builtin.parcelSheet")
            .cloned()
            .expect("the built-in model");
        model.id = "user.olculer".into();
        model.label = "Ölçülerim".into();
        app.processing
            .registry
            .save_model(model)
            .expect("a user's model");
        assert_eq!(app.user_model_buttons().len(), 1, "on the ribbon");

        // Typed on the command line, by its name.
        let _ = app.update(Message::CommandRun("ölçülerim".into()));
        assert_eq!(app.dialog, Some(Dialog::Processing));
        let _ = app.update(Message::DialogClosed);

        // Found by Komut ara, where it lives said; Enter opens it.
        let _ = app.update(Message::RibbonSearch("ölçüler".into()));
        let rows = app.search_rows();
        let at = rows
            .iter()
            .position(|r| r.title == "Ölçülerim…")
            .expect("found");
        assert!(rows[at].detail.contains("Modeller"), "{}", rows[at].detail);
        let _ = app.update(Message::RibbonSearchReveal(at));
        assert_eq!(app.ribbon_flash_model.as_deref(), Some("user.olculer"));
        let _ = app.update(Message::RibbonSearch("ölçülerim".into()));
        let at = app
            .search_rows()
            .iter()
            .position(|r| r.title == "Ölçülerim…")
            .expect("found");
        let _ = app.update(Message::RibbonSearchRun(at));
        assert_eq!(app.dialog, Some(Dialog::Processing));
    }

    /// The İşlemler tab with a model of the user's for the owner;
    /// `.run/shots/serit-kullanici-modeli-*`.
    /// `cargo test -p kentos-desktop ribbon_search::tests::user_model_screens -- --ignored --nocapture`
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn user_model_screens() {
        use iced::Size;
        use kentos_ui::snapshot::Snapshot;

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
            for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::WindowResized(Size::new(width, height)));
                let mut model = app
                    .processing
                    .registry
                    .model("builtin.parcelSheet")
                    .cloned()
                    .expect("the built-in model");
                model.id = "user.olculer".into();
                model.label = "Ölçülerim".into();
                app.processing
                    .registry
                    .save_model(model)
                    .expect("a user's model");
                app.choose_tab("analysis");
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!(
                    "serit-kullanici-modeli-{width}x{height}{suffix}.png"
                ));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }

    /// A row says where its command lives; Alt+Enter opens its tab and
    /// outlines it until the moment passes; Enter runs it and empties the box.
    #[test]
    fn a_found_command_runs_or_shows_where_it_lives() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::RibbonSearch("döndür".into()));
        let rows = app.search_rows();
        let first = rows.first().expect("found");
        assert_eq!(first.title, "Döndür");
        assert!(first.enabled && first.revealable);
        assert!(first.detail.contains(" › "), "{}", first.detail);
        let _ = app.update(Message::RibbonSearchReveal(0));
        assert_ne!(app.tab, "file");
        assert_eq!(app.ribbon_flash, Some("tool.rotate"));
        assert!(app.ribbon_search.is_empty());
        let _ = app.update(Message::RibbonFlashEnd("tool.rotate"));
        assert_eq!(app.ribbon_flash, None);

        let _ = app.update(Message::RibbonSearch("çizgi".into()));
        let _ = app.update(Message::RibbonSearchRun(0));
        assert!(app.session.is_running(), "Çizgi runs");
        assert!(app.ribbon_search.is_empty());
    }
}

/// Pictures for the owner: Komut ara with “çiz” typed (the list open), one
/// that finds nothing, and a command's place shown (Döndür outlined);
/// `.run/shots/komut-ara-*`.
/// `cargo test -p kentos-desktop ribbon_search::screens -- --ignored --nocapture`
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use iced::futures::StreamExt as _;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["komut-ara", "komut-ara-yok", "komut-ara-yeri"] {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                if name == "komut-ara-yeri" {
                    let _ = app.update(Message::RibbonSearch("döndür".into()));
                    let _ = app.update(Message::RibbonSearchReveal(0));
                } else {
                    // Alt+Q focuses the box by a widget operation, as the runtime runs it.
                    let task = app.update(Message::Run("view.commandSearch"));
                    if let Some(mut stream) = iced_runtime::task::into_stream(task) {
                        while let Some(action) = iced::futures::executor::block_on(stream.next()) {
                            if let iced_runtime::Action::Widget(operation) = action {
                                snapshot.operate(app.view(), operation);
                            }
                        }
                    }
                    let typed = if name == "komut-ara" { "çiz" } else { "qwx" };
                    let _ = app.update(Message::RibbonSearch(typed.into()));
                    // As typed: the cursor after the text, nothing selected.
                    snapshot.operate(
                        app.view(),
                        Box::new(
                            iced::advanced::widget::operation::text_input::move_cursor_to_end(
                                iced::widget::Id::new(SEARCH_INPUT),
                            ),
                        ),
                    );
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
