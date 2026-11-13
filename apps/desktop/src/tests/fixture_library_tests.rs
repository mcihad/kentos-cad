//! Donatı › Kitaplık (usage review UR-024): looking at KentOS's block library changes no drawing; a block placed from
//! it takes in its own definition alone; the whole library comes in only by its own button, one undo step.

use crate::app::{App, Message};
use crate::blocks_library::Event;
use crate::files_testing::{app_with_drawing, make_cad};

fn library(app: &mut App, e: Event) {
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Library(e)));
}

fn blocks(app: &App) -> usize {
    app.document.as_ref().expect("a drawing").model.blocks().len()
}

#[test]
fn looking_at_the_library_changes_nothing_and_a_block_comes_in_alone() {
    let mut app = app_with_drawing();
    make_cad(&mut app);
    let before = blocks(&app);
    let revision = app.document.as_ref().expect("a drawing").model.revision();
    let _ = app.update(Message::Run("block.library"));
    assert_eq!(blocks(&app), before, "no definition taken by looking");
    assert_eq!(app.document.as_ref().expect("a drawing").model.revision(), revision, "nothing written");
    assert!(app.blocks_panel.library_shown && app.blocks_panel.library.builtin, "the library shown");

    let first = crate::fixture_search::fixtures().first().expect("a fixture").name.clone();
    library(&mut app, Event::PlaceBuiltin(first.clone()));
    assert_eq!(blocks(&app), before + 1, "its definition alone");
    assert!(app.document.as_ref().expect("a drawing").model.block_named(&first).is_some());
    let _ = app.update(Message::Run("tool.cancel"));

    library(&mut app, Event::TakeBuiltin);
    let all = blocks(&app);
    assert!(all > before + 1, "the whole library by its button");
    assert!(app.document.as_mut().expect("a drawing").model.undo().is_some());
    assert_eq!(blocks(&app), before + 1, "one undo step takes the library back");
}

/// UR-024's pictures (`.run/gui-evidence/kullanim-UR-024/`): Donatı › Kitaplık pressed, KentOS's blocks listed.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn fixture_library_screens() {
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-UR-024");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = app_with_drawing();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        make_cad(&mut app);
        app.tab = "arch";
        let _ = app.update(Message::Run("block.library"));
        let mut snapshot = kentos_ui::snapshot::Snapshot::new(iced::Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("donati-kitaplik-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
