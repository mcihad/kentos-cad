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

use iced::Size;
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::files_testing::app_with_drawing;

/// One picture: its name and what brings the app to it.
pub(crate) type Scene = (&'static str, fn(&mut App));

/// The ribbon tabs the new tools sit in.
fn ribbon_scenes() -> Vec<Scene> {
    vec![
        ("serit-cizim", |app| app.tab = "draw"),
        ("serit-degistir", |app| app.tab = "modify"),
        ("serit-harita", |app| app.tab = "map"),
    ]
}

/// Every scene of the new tools: the ribbon's, then each tool's (the tool
/// modules add theirs here as they are built).
fn scenes() -> Vec<Scene> {
    ribbon_scenes()
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
            for (name, bring) in scenes() {
                if !wanted(name) {
                    continue;
                }
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message: Message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                bring(&mut app);
                snapshot.settle(&mut app, App::view, &mut update);
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
