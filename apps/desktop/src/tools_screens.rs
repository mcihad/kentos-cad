//! Pictures of the drawing and editing tools of docs/adr/0140, for the owner
//! to look at: the ribbon tabs that hold them, then each tool at work, in
//! the dark and the light theme, at 1440×900 and 1100×650. Not a test of
//! correctness and not run by default; writes `.run/shots/arac-*`:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=serit-degistir cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! `KENTOS_SHOTS_ONLY` (comma separated) keeps only the named pictures.

use std::path::PathBuf;

use iced::{Point, Size};
use kentos_ui::snapshot::{Input, Snapshot};

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, find_text, find_texts};

/// One picture: its name and what brings the app to it.
pub(crate) type Scene = (&'static str, fn(&mut App));

/// What a click on the window opens over a scene (a menu).
pub(crate) type Pointing = fn(&mut Snapshot, &mut App);

/// A picture that needs the pointer at the window itself: what brings the app
/// to it, then what a click opens over it (a menu).
pub(crate) type Pointed = (&'static str, fn(&mut App), Pointing);

/// Any picture: its name, what brings the app to it, and the click over it, if any.
type Shot = (&'static str, fn(&mut App), Option<Pointing>);

/// Where a caption is drawn: its middle, or the point just right of it (a
/// small split button's arrow). A caption that is also the ribbon's (the layer
/// box says “Parsel”) is looked for in the docked panels first: the right of
/// the window, under the ribbon.
fn caption_at(snapshot: &mut Snapshot, app: &App, caption: &str, arrow: bool) -> Point {
    let docked = find_texts(snapshot, app, caption)
        .into_iter()
        .find(|at| at.x > 900.0 && at.y > 150.0);
    let at = docked
        .or_else(|| find_text(snapshot, app, caption))
        .unwrap_or_else(|| panic!("{caption} is on screen"));
    if arrow {
        Point::new(at.x + at.width + 10.0, at.center_y())
    } else {
        at.center()
    }
}

/// A split button's list opened, as its key tip opens it (ribbon_keys.rs): by the
/// button's own key, the tools' family or the tool with methods.
pub(crate) fn open_split(snapshot: &mut Snapshot, app: &mut App, key: &str) {
    use iced::futures::StreamExt as _;
    let task: iced::Task<Message> =
        kentos_ui::widget::context_menu::open_menu(crate::ribbon_keys::split_menu_id(key));
    if let Some(mut stream) = iced_runtime::task::into_stream(task) {
        while let Some(action) = iced::futures::executor::block_on(stream.next()) {
            if let iced_runtime::Action::Widget(operation) = action {
                snapshot.operate(app.view(), operation);
            }
        }
    }
}

/// A click (or a right click) on a caption of the window.
pub(crate) fn press_caption(
    snapshot: &mut Snapshot,
    app: &mut App,
    caption: &str,
    right: bool,
    arrow: bool,
) {
    let at = caption_at(snapshot, app, caption, arrow);
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    let input = if right {
        Input::RightClick(at)
    } else {
        Input::Click(at)
    };
    snapshot.input(app, App::view, &mut update, input);
}

/// Pictures of menus the new commands are in (docs/adr/0141): the layer tree's, and Seç ▾.
fn pointed_scenes() -> Vec<Pointed> {
    let mut all = crate::text_scenes::pointed();
    all.extend(crate::leader_scenes::pointed());
    all.extend(crate::dimension_scenes::pointed());
    all.extend(crate::topology_scenes::pointed());
    let menus: Vec<Pointed> = vec![
        (
            "katman-menu-katman",
            |_| {},
            |s, app| {
                press_caption(s, app, "Parsel", true, false);
            },
        ),
        (
            "katman-menu-grup",
            |_| {},
            |s, app| {
                press_caption(s, app, "Kadastro", true, false);
            },
        ),
        (
            "secim-ailesi",
            |app| app.tab = "home",
            |s, app| open_split(s, app, "select"),
        ),
        // docs/adr/0142: Kot ver's three ways, the list of its split button (Düzenle, a CBS project's).
        (
            "kot-ver-yontemler",
            |app| app.tab = "edit",
            |s, app| open_split(s, app, "setElevation"),
        ),
    ];
    all.extend(menus);
    all
}

/// The ribbon tabs the new tools sit in: a CAD project's Değiştir, a CBS
/// project's Düzenle and Harita.
fn ribbon_scenes() -> Vec<Scene> {
    vec![
        ("serit-degistir", |app| {
            crate::files_testing::make_cad(app);
            app.tab = "modify";
        }),
        ("serit-duzenle", |app| app.tab = "edit"),
        ("serit-harita", |app| app.tab = "map"),
        // docs/adr/0141: two zooms in and a step back, so that Önceki and Sonraki görünüm are both on.
        ("serit-gorunum", |app| {
            app.tab = "view";
            for id in ["view.zoomIn", "view.zoomIn", "view.previous"] {
                let _ = app.update(Message::Run(id));
            }
        }),
    ]
}

/// Every scene of the new tools: the ribbon's, then each tool's (the tool
/// modules add theirs here as they are built).
fn scenes() -> Vec<Scene> {
    let mut all = ribbon_scenes();
    all.extend(crate::tools_scenes::scenes());
    all.extend(crate::elevation_scenes::scenes());
    all.extend(crate::parts_scenes::scenes());
    all.extend(crate::text_scenes::scenes());
    all.extend(crate::leader_scenes::scenes());
    all.extend(crate::dimension_scenes::scenes());
    all.extend(crate::topology_scenes::scenes());
    all
}

fn wanted(name: &str) -> bool {
    std::env::var("KENTOS_SHOTS_ONLY")
        .map_or(true, |only| only.split(',').any(|o| o.trim() == name))
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let all: Vec<Shot> = scenes()
                .into_iter()
                .map(|(name, bring)| (name, bring, None))
                .chain(
                    pointed_scenes()
                        .into_iter()
                        .map(|(name, bring, point)| (name, bring, Some(point))),
                )
                .collect();
            for (name, bring, point) in all {
                if !wanted(name) {
                    continue;
                }
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                // The command history open, so the picture shows what the tool said; the
                // area settles at its size before the drawing opens and is fitted to it.
                app.command_expanded = !name.starts_with("serit");
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message: Message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                bring(&mut app);
                snapshot.settle(&mut app, App::view, &mut update);
                if let Some(point) = point {
                    point(&mut snapshot, &mut app);
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!("arac-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
