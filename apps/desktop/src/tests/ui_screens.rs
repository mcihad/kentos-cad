//! Pictures of the interface's finish for the owner (docs/adr/0127): the
//! right-click menu in the three corners and shadows, a long layer name in
//! the ribbon's select (closed and open), the layer search box focused and
//! Uygulama ayarları → Görünüm's Biçim; dark and light,
//! `.run/shots/arayuz-*`. Not a correctness test.
//!
//! `cargo test -p kentos-desktop ui_screens -- --ignored --nocapture`

use std::path::{Path, PathBuf};

use iced::{Point, Size};
use kentos_domain::NewLayer;
use kentos_ui::snapshot::{Input, Snapshot};
use serde_json::Value;

use crate::app::App;
use crate::files_testing::{app_with_drawing, find_texts};

/// A layer's name longer than any box it is shown in.
const LONG: &str = "Kadastro sınırları (2026 güncellemesi, düzeltilmiş ve onaylı)";

fn booted(mode: &str, corners: &str, shadows: &str) -> App {
    let mut app = app_with_drawing();
    let _ = app.settings.choose(&[
        ("appearance.theme", Value::from(mode)),
        ("appearance.corners", Value::from(corners)),
        ("appearance.shadows", Value::from(shadows)),
    ]);
    app.apply_settings();
    app
}

fn update(app: &mut App, message: crate::app::Message) {
    let _ = app.update(message);
}

fn save(snapshot: &mut Snapshot, app: &App, out: &Path, name: &str) {
    let file = out.join(format!("{name}.png"));
    snapshot
        .render(app.view(), &app.theme())
        .save(&file)
        .expect("writes the picture");
    println!("{}", file.display());
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let size = Size::new(1440.0, 900.0);
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        // The right-click menu in each corner and shadow.
        for (corners, shadows) in [("sharp", "off"), ("soft", "soft"), ("round", "strong")] {
            let mut app = booted(mode, corners, shadows);
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            snapshot.settle(&mut app, App::view, &mut update);
            let area = app.viewport.bounds;
            let at = Point::new(area.x + area.width * 0.45, area.y + area.height * 0.35);
            snapshot.input(&mut app, App::view, &mut update, Input::RightClick(at));
            save(
                &mut snapshot,
                &app,
                &out,
                &format!("arayuz-sag-tik-{corners}{suffix}"),
            );
        }

        // A long layer's name, active: the ribbon's select shortens it; open, the list too.
        let mut app = booted(mode, "soft", "soft");
        if let Some(doc) = app.document.as_mut() {
            doc.model
                .add_layer(NewLayer::layer(LONG.to_owned()), None, true)
                .expect("a layer");
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        save(
            &mut snapshot,
            &app,
            &out,
            &format!("arayuz-uzun-ad{suffix}"),
        );
        // The tree and the attributes report the whole name however short they show it.
        assert!(find_texts(&mut snapshot, &app, LONG).len() >= 2);
        // Giriş → Katmanlar's field, where the ribbon lays it out at this width.
        snapshot.input(
            &mut app,
            App::view,
            &mut update,
            Input::Click(Point::new(998.0, 57.0)),
        );
        save(
            &mut snapshot,
            &app,
            &out,
            &format!("arayuz-uzun-ad-liste{suffix}"),
        );

        // The layer search box focused: its edge in the accent line.
        let mut app = booted(mode, "soft", "soft");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let search = Point::new(size.width - 150.0, 196.0);
        snapshot.input(&mut app, App::view, &mut update, Input::Click(search));
        snapshot.input(
            &mut app,
            App::view,
            &mut update,
            Input::Type("kad".to_owned()),
        );
        save(&mut snapshot, &app, &out, &format!("arayuz-odak{suffix}"));

        // Uygulama ayarları → Görünüm, scrolled down to Biçim.
        let mut app = booted(mode, "soft", "soft");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.run("tools.options");
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.input(
            &mut app,
            App::view,
            &mut update,
            Input::Scroll(Point::new(800.0, 500.0), -6.0),
        );
        save(
            &mut snapshot,
            &app,
            &out,
            &format!("arayuz-ayarlar-bicim{suffix}"),
        );
    }
}
