//! Çakışma denetimi in the desktop's shell (docs/adr/0162 §1): the status
//! bar's Çakışma cell turns the overlap control on and off, its right-click
//! menu chooses the mode and Seçili katmanlarda önle's layers. The cutting
//! is the shared trace's (`fixtures/interaction/v1/overlap.json`, traces/).

use iced::keyboard::key::Named;
use iced::{Point, Size, mouse};
use kentos_interaction::Overlap;
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
        "../../../fixtures/interaction/v1/overlap.kcad"
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

/// Keys through the widgets: the open menu takes them before anything else.
fn keys(snapshot: &mut Snapshot, app: &mut App, named: &[Named]) {
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    for k in named {
        snapshot.input(app, App::view, &mut update, Input::Key(*k));
    }
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

/// Serbest as every session starts; a click avoids overlap on its own layer
/// (the mode it starts with); the menu (its items taken with the arrows and
/// Enter: the menu's texts are drawn above the widgets, where `find_text`
/// does not look) chooses Seçili katmanlarda önle, then ticks the first
/// layer, Parsel; a click goes back to Serbest, the next to the last mode.
#[test]
fn the_status_cell_turns_the_control_and_its_menu_chooses_mode_and_layers() {
    use Named::{ArrowDown, ArrowRight, Enter, Escape};
    let size = Size::new(1440.0, 900.0);
    let mut app = opened(size, "dark");
    let mut snapshot = settled(&mut app, size);
    assert_eq!(app.draft.overlap, Overlap::Allow);
    let cell = find_text(&mut snapshot, &app, "Çakışma").expect("the Çakışma cell");
    press(&mut snapshot, &mut app, cell.center(), mouse::Button::Left);
    assert_eq!(
        app.draft.overlap,
        Overlap::Layer,
        "the click avoids on its own layer"
    );
    assert_eq!(app.checked("draft.overlap"), Some(true));
    assert_eq!(app.checked("draft.overlap.layer"), Some(true));
    // Serbest, Kendi katmanında önle, Seçili katmanlarda önle.
    press(&mut snapshot, &mut app, cell.center(), mouse::Button::Right);
    keys(
        &mut snapshot,
        &mut app,
        &[ArrowDown, ArrowDown, ArrowDown, Enter],
    );
    assert_eq!(app.draft.overlap, Overlap::Layers);
    assert!(app.overlap_layers.is_empty());
    // Katmanlar ▸, its first layer.
    press(&mut snapshot, &mut app, cell.center(), mouse::Button::Right);
    keys(
        &mut snapshot,
        &mut app,
        &[
            ArrowDown, ArrowDown, ArrowDown, ArrowDown, ArrowRight, Enter,
        ],
    );
    assert_eq!(app.overlap_layers, ["parsel"]);
    assert_eq!(app.draft.overlap, Overlap::Layers);
    // The layers are ticked one after another (docs/adr/0187 §5, 8 Ekim):
    // the menu is still open, the next layer one key away; Esc closes the
    // list, then the menu (a click outside would only close it).
    keys(&mut snapshot, &mut app, &[ArrowDown, Enter]);
    assert_eq!(
        app.overlap_layers.len(),
        2,
        "a second layer from the same opening"
    );
    keys(&mut snapshot, &mut app, &[Enter]);
    assert_eq!(
        app.overlap_layers,
        ["parsel"],
        "and off again, the row still lit"
    );
    keys(&mut snapshot, &mut app, &[Escape, Escape]);
    press(&mut snapshot, &mut app, cell.center(), mouse::Button::Left);
    assert_eq!(
        app.draft.overlap,
        Overlap::Allow,
        "a click goes back to Serbest"
    );
    press(&mut snapshot, &mut app, cell.center(), mouse::Button::Left);
    assert_eq!(
        app.draft.overlap,
        Overlap::Layers,
        "and the next to the last mode"
    );
    assert_eq!(app.overlap_layers, ["parsel"], "the layers stay chosen");
}

/// The cell's menu open, and its Katmanlar open, for the owner, in
/// `.run/shots/durum-cakisma-*`:
///
/// ```text
/// cargo test -p kentos-desktop overlap_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use Named::{ArrowDown, ArrowRight};
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for (name, layers) in [("menu", false), ("katmanlar", true)] {
                let size = Size::new(width, height);
                let mut app = opened(size, mode);
                let _ = app.update(Message::OverlapLayer("yol".to_owned()));
                let mut snapshot = pictured(&mut app, size);
                let cell = find_text(&mut snapshot, &app, "Çakışma").expect("the Çakışma cell");
                press(&mut snapshot, &mut app, cell.center(), mouse::Button::Right);
                if layers {
                    keys(
                        &mut snapshot,
                        &mut app,
                        &[ArrowDown, ArrowDown, ArrowDown, ArrowDown, ArrowRight],
                    );
                }
                let file = out.join(format!("durum-cakisma-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
