//! The Kenet cell in the desktop's shell (docs/adr/0163 §5–§6): its
//! right-click menu ticks the snap kinds one by one and chooses Karelaj's
//! spacing, through the widgets; out of the snap's scale range the cell is
//! idle. The snapping itself is the shared trace's
//! (`fixtures/interaction/v1/snap-additions.json`, traces/).

use iced::keyboard::key::Named;
use iced::{Point, Size, mouse};
use kentos_interaction::SnapKind;
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
        "../../../fixtures/interaction/v1/snap-additions.kcad"
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

/// `n` steps down the open menu, then `then`.
fn down(n: usize, then: &[Named]) -> Vec<Named> {
    std::iter::repeat_n(Named::ArrowDown, n)
        .chain(then.iter().copied())
        .collect()
}

/// The menu's items taken with the arrows and Enter (its texts are drawn
/// above the widgets, where `find_text` does not look): the ninth kind,
/// Ağırlık merkezi, ticked; Karelaj aralığı ▸ (the fourteenth item), its
/// third spacing, 0.5 m, chosen, which turns Karelaj on.
#[test]
fn the_cells_menu_ticks_a_kind_and_chooses_a_spacing() {
    use Named::{ArrowRight, Enter};
    let size = Size::new(1440.0, 900.0);
    let mut app = opened(size, "dark");
    let mut snapshot = settled(&mut app, size);
    let cell = find_text(&mut snapshot, &app, "Kenet").expect("the Kenet cell");
    let on = |app: &App, kind: SnapKind| app.draft.snap_kinds & kind.bit() != 0;
    assert!(!on(&app, SnapKind::Centroid));
    press(&mut snapshot, &mut app, cell.center(), mouse::Button::Right);
    keys(&mut snapshot, &mut app, &down(9, &[Enter]));
    assert!(on(&app, SnapKind::Centroid), "Ağırlık merkezi ticked");
    assert!(
        app.draft.snap,
        "a kind's tick leaves Kenet itself as it was"
    );
    press(&mut snapshot, &mut app, cell.center(), mouse::Button::Right);
    keys(
        &mut snapshot,
        &mut app,
        &down(14, &[ArrowRight, Named::ArrowDown, Named::ArrowDown, Enter]),
    );
    assert_eq!(app.draft.snap_grid, [0.5, 0.5]);
    assert!(on(&app, SnapKind::Grid), "a spacing turns Karelaj on");
    // A left click still turns Kenet as a whole (F3).
    press(&mut snapshot, &mut app, cell.center(), mouse::Button::Left);
    assert!(!app.draft.snap);
}

/// The status bar's coordinates are where the tools take the cursor to be:
/// Karelaj's node, not the pointer's own place (the web's `cursorWorld`).
#[test]
fn the_coordinates_cell_shows_the_snapped_point() {
    let size = Size::new(1440.0, 900.0);
    let mut app = opened(size, "dark");
    let _ = app.update(Message::Run("draft.snap.grid"));
    let _ = app.update(Message::Run("tool.polygon"));
    let mut snapshot = settled(&mut app, size);
    // 5.3 m east and 7.8 m north of the scene's origin: the node (5, 8).
    let [x, y] = app
        .viewport
        .camera
        .world_to_screen(kentos_interaction::Vec2::new(487_005.3, 4_420_007.8));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Moved(
        Point::new(x as f32, y as f32),
    )));
    assert_eq!(app.snap.map(|s| s.kind), Some(SnapKind::Grid));
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    assert!(
        find_text(&mut snapshot, &app, "Y 487005.000   X 4420008.000").is_some(),
        "the node's coordinates"
    );
}

/// The cell's menu, its Karelaj aralığı, Kenetleme's settings and the idle
/// cell out of the scale range, for the owner, in `.run/shots/durum-kenet-*`
/// and `.run/shots/ayar-kenet-*`:
///
/// ```text
/// cargo test -p kentos-desktop snap_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use Named::ArrowRight;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let size = Size::new(width, height);
            let save = |snapshot: &mut Snapshot, app: &App, name: &str| {
                let file = out.join(format!("{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            };
            for (name, open) in [("durum-kenet-menu", false), ("durum-kenet-karelaj", true)] {
                let mut app = opened(size, mode);
                let _ = app.update(Message::Run("draft.snap.centroid"));
                let mut snapshot = pictured(&mut app, size);
                let cell = find_text(&mut snapshot, &app, "Kenet").expect("the Kenet cell");
                press(&mut snapshot, &mut app, cell.center(), mouse::Button::Right);
                if open {
                    keys(&mut snapshot, &mut app, &down(14, &[ArrowRight]));
                }
                save(&mut snapshot, &app, name);
            }
            // Kenetleme's settings: the kinds, Karelaj and Kenedin kapsamı (scrolled to).
            let mut app = opened(size, mode);
            let _ = app.update(Message::Snap(crate::snap_menu::Event::Settings));
            let mut snapshot = pictured(&mut app, size);
            save(&mut snapshot, &app, "ayar-kenet");
            if let Some(at) = find_text(&mut snapshot, &app, "Kenet türleri") {
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.step(
                    &mut app,
                    App::view,
                    &mut update,
                    &[
                        iced::Event::Mouse(mouse::Event::CursorMoved {
                            position: at.center(),
                        }),
                        iced::Event::Mouse(mouse::Event::WheelScrolled {
                            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -600.0 },
                        }),
                    ],
                );
                snapshot.settle(&mut app, App::view, &mut update);
                save(&mut snapshot, &app, "ayar-kenet-alt");
            }
            // Out of the scale range: the cell idle.
            let mut app = opened(size, mode);
            let _ = app
                .settings
                .choose(&[("snap.scaleMax", serde_json::Value::from(1))]);
            app.apply_settings();
            let mut snapshot = pictured(&mut app, size);
            save(&mut snapshot, &app, "durum-kenet-disinda");
        }
    }
}
