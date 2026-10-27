//! The pictures of docs/adr/0720 (the level a drawing opens on, a level in use, a tool following Etkin kat): the
//! two-floor apartment of the walkthrough saved on 1. kat with Zemin kat faint and opened again; 1. kat taken away in
//! Proje ayarları while its walls and rooms name it (refused, said, they selected); Merdiven running while Üst kat
//! (Ctrl+↑) moves Etkin kat, its height the new level's at once; the Kat menu with Üst kat and Alt kat. `tools_screens`
//! takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kat-dosya-acilis,kat-silme-pencere,kat-silme,kat-merdiven-izler,kat-dosya-menu,kat-gorunumu-serit cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_interaction::level_view::LevelView;

use crate::app::{App, Message};
use crate::project::Event as Project;
use crate::project::arch::Edit;
use crate::project::settings::Event as Settings;
use crate::tools_scenes::{click, hover, run};
use crate::tools_screens::{Pointed, Scene};
use crate::walkthrough_scenes::{upper_floor, view};

/// The two floors saved with 1. kat active and Zemin kat faint under it, then opened again from the file: it opens on
/// them and says so.
fn reopened(app: &mut App) {
    upper_floor(app);
    let _ = app.update(Message::LevelView(LevelView::Below));
    let path = crate::files_testing::scratch("kat-dosya").join("daire.kcad");
    let task = app.start_saving(path.clone());
    crate::files_testing::drive(app, task);
    let task = app.start_opening(path, crate::opening::Purpose::File);
    crate::files_testing::drive(app, task);
    view(app);
    app.tab = "arch";
    app.command_expanded = true;
    hover(app, [14.0, 11.0]);
}

fn settings(app: &mut App, e: Settings) {
    let _ = app.update(Message::Project(Box::new(Project::Settings(e))));
}

/// Proje ayarları › Mimari: 1. kat's row taken away while its walls and rooms name it: said under the levels with
/// Nesneleri seç, Kaydet waiting.
fn kept_level_window(app: &mut App) {
    upper_floor(app);
    let _ = app.update(Message::Run("arch.settings"));
    settings(app, Settings::Level(1, Edit::Delete));
}

/// Nesneleri seç: what names 1. kat selected (Öznitelikler's Kat ready), the window closed, said.
fn kept_level(app: &mut App) {
    kept_level_window(app);
    settings(app, Settings::SelectLevelUsers);
    hover(app, [14.0, 11.0]);
}

/// Merdiven started on Zemin kat (3.06 m floor to floor), its first point given; Üst kat (Ctrl+↑): 1. kat (2.80 m),
/// its Yükseklik and the preview's risers the new level's at once, the pointer resting.
fn stair_follows(app: &mut App) {
    upper_floor(app);
    if let Some(doc) = app.document.as_mut() {
        let mut s = doc.model.settings().clone();
        if let Some(level) = s.arch.as_mut().and_then(|a| a.levels.iter_mut().find(|l| l.id == "kat-1")) {
            level.height = Some(2.8);
        }
        doc.model.set_settings(s);
    }
    let _ = app.update(Message::ActiveLevel(Some("zemin".into())));
    run(app, "tool.stair");
    click(app, [1.0, 5.5]);
    hover(app, [6.5, 5.5]);
    let _ = app.update(Message::Run("view.levels.up"));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kat-dosya-acilis", reopened),
        ("kat-silme-pencere", kept_level_window),
        ("kat-silme", kept_level),
        ("kat-merdiven-izler", stair_follows),
    ]
}

/// The status bar's Kat menu after the file opened again: the levels, Katsız, Üst kat and Alt kat with their keys,
/// Kat görünümü.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![("kat-dosya-menu", reopened, |s, app| {
        // The Kat cell right of the scale's (its words fold away first in a full bar): its icon.
        let bar = crate::files_testing::find_text(s, app, "Yardımcılar").expect("the status bar");
        let scale = crate::files_testing::find_texts(s, app, "1:100")
            .into_iter()
            .find(|r| (r.center_y() - bar.center_y()).abs() < 6.0)
            .expect("the scale cell");
        let at = iced::Point::new(scale.x + scale.width + 50.0, scale.center_y());
        let mut update = |app: &mut App, message: Message| {
            let _ = app.update(message);
        };
        s.input(app, App::view, &mut update, kentos_ui::snapshot::Input::Click(at));
    })]
}
