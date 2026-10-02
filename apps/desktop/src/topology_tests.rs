//! Topolojik düzenleme in the desktop's shell (docs/adr/0160): the status
//! bar's Topoloji cell turns the mode on and off, its right-click menu
//! Noktalar da. The grips' edits are the shared trace's
//! (`fixtures/interaction/v1/topology-edit.json`, traces/).

use iced::keyboard::key::Named;
use iced::{Point, Size, mouse};
use kentos_ui::snapshot::{Input, Snapshot};

use crate::app::{App, Message};
use crate::files_testing::find_text;

/// The app on the trace's drawing, `size` wide and high, in `theme`.
fn opened(size: Size, theme: &str) -> App {
    let (mut app, _) = App::boot(None);
    let _ = app
        .settings
        .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
    app.apply_settings();
    let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(include_str!(
        "../../../fixtures/interaction/v1/topology-edit.kcad"
    ))
    .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::WindowResized(size));
    app
}

fn press(snapshot: &mut Snapshot, app: &mut App, at: Point, button: mouse::Button) {
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    snapshot.step(
        app,
        App::view,
        &mut update,
        &[
            iced::Event::Mouse(mouse::Event::CursorMoved { position: at }),
            iced::Event::Mouse(mouse::Event::ButtonPressed(button)),
            iced::Event::Mouse(mouse::Event::ButtonReleased(button)),
        ],
    );
    snapshot.settle(app, App::view, &mut update);
}

/// A key through the widgets: the open menu takes it before anything else.
fn key(snapshot: &mut Snapshot, app: &mut App, named: Named) {
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    snapshot.input(app, App::view, &mut update, Input::Key(named));
}

/// The app laid out on the software renderer (`files_testing::offscreen`).
fn settled(app: &mut App, size: Size) -> Snapshot {
    settle(app, crate::files_testing::offscreen(size))
}

/// The app laid out on the GPU's renderer, for the pictures.
fn pictured(app: &mut App, size: Size) -> Snapshot {
    settle(app, Snapshot::new(size).expect("a renderer"))
}

fn settle(app: &mut App, mut snapshot: Snapshot) -> Snapshot {
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    snapshot.settle(app, App::view, &mut update);
    snapshot
}

/// Off as every session starts; a click on the cell turns it on, its
/// right-click menu turns Noktalar da on (its only item, taken with ↓ and
/// Enter: the menu's texts are drawn above the widgets, where `find_text`
/// does not look), another click turns the mode off.
#[test]
fn the_status_cell_turns_the_mode_and_its_menu_points() {
    let size = Size::new(1440.0, 900.0);
    let mut app = opened(size, "dark");
    let mut snapshot = settled(&mut app, size);
    assert!(!app.draft.topology && !app.draft.topology_points);
    let cell = find_text(&mut snapshot, &app, "Topoloji").expect("the Topoloji cell");
    press(&mut snapshot, &mut app, cell.center(), mouse::Button::Left);
    assert!(app.draft.topology, "the click turns it on");
    assert_eq!(app.checked("draft.topology"), Some(true));
    press(&mut snapshot, &mut app, cell.center(), mouse::Button::Right);
    key(&mut snapshot, &mut app, Named::ArrowDown);
    key(&mut snapshot, &mut app, Named::Enter);
    assert!(app.draft.topology_points, "Noktalar da from the menu");
    assert!(app.draft.topology, "the menu leaves the mode on");
    press(&mut snapshot, &mut app, cell.center(), mouse::Button::Left);
    assert!(!app.draft.topology, "a second click turns it off");
    assert!(app.draft.topology_points, "its option stays as it was");
}

/// The cell's menu open, for the owner, in `.run/shots/durum-topoloji-*`:
///
/// ```text
/// cargo test -p kentos-desktop topology_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let size = Size::new(width, height);
            let mut app = opened(size, mode);
            let _ = app.run("draft.topology");
            let mut snapshot = pictured(&mut app, size);
            let cell = find_text(&mut snapshot, &app, "Topoloji").expect("the Topoloji cell");
            press(&mut snapshot, &mut app, cell.center(), mouse::Button::Right);
            let file = out.join(format!("durum-topoloji-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
