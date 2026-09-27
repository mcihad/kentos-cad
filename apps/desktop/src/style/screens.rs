//! Pictures of the styled drawing for the owner (docs/adr/0090): the web's
//! demo drawing (the sample project and its showcase of every system symbol,
//! written by `apps/web/scripts/style/demo-drawing.test.ts`) at the views the
//! web's pictures show, dark and light, 1440×900 and 1100×650. Not a test of
//! correctness and not run by default:
//!
//! ```text
//! KENTOS_DEMO_OUT=$PWD/.run/demo/demo.json pnpm -C apps/web exec vitest run scripts/style/demo-drawing.test.ts
//! cargo test -p kentos-desktop style::screens -- --ignored --nocapture
//! ```
//!
//! The pictures go to `.run/shots/stil-*.png`; the build and frame times to
//! standard output.

use std::path::PathBuf;
use std::time::Instant;

use iced::Size;
use kentos_contracts::DocumentSnapshotV1;
use kentos_render_wgpu::Bounds;
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::document::Document;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The web's demo drawing, when it has been written.
pub fn demo() -> Option<Document> {
    let path = std::env::var_os("KENTOS_DEMO_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join(".run/demo/demo.json"));
    let text = std::fs::read_to_string(&path).ok()?;
    let snapshot = DocumentSnapshotV1::from_json(&text).ok()?;
    Document::new(snapshot, None).ok()
}

/// The box of a layer's points (the showcase's areas, lines, points and texts).
fn extent(app: &App, layer: &str) -> Option<Bounds> {
    use kentos_contracts::Entity;
    let doc = app.document.as_ref()?;
    let mut b: Option<Bounds> = None;
    let mut grow = |x: f64, y: f64| {
        let a = b.get_or_insert(Bounds {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        });
        a.min_x = a.min_x.min(x);
        a.min_y = a.min_y.min(y);
        a.max_x = a.max_x.max(x);
        a.max_y = a.max_y.max(y);
    };
    for e in doc.model.by_layer(layer) {
        match e {
            Entity::Polygon(p) | Entity::Polyline(p) => p.pts.iter().for_each(|q| grow(q.x, q.y)),
            Entity::Point(p) => grow(p.p.x, p.p.y),
            Entity::Text(t) => grow(t.p.x, t.p.y),
            _ => {}
        }
    }
    b
}

/// Views of the showcase: its section layers, at 1:1000 on screen, their top left in view.
const VIEWS: [(&str, &str); 6] = [
    ("temel", "vitrin.temel-cizgi-tipleri"),
    (
        "uip-sinirlar",
        "vitrin.mpyy-uygulama-imar-plani-sinirlar-planlama-sinirlari",
    ),
    (
        "uip-konut",
        "vitrin.mpyy-uygulama-imar-plani-konut-alanlari",
    ),
    (
        "uip-yapi-duzeni",
        "vitrin.mpyy-uygulama-imar-plani-yapi-duzeni-ve-yogunluklari",
    ),
    (
        "uip-sosyal",
        "vitrin.mpyy-uygulama-imar-plani-sosyal-altyapi-alanlari-egitim-tesisleri-alani",
    ),
    (
        "uip-karayollari",
        "vitrin.mpyy-uygulama-imar-plani-teknik-altyapi-ulasim-karayollari",
    ),
];

/// Uygulama ayarları with its new Semboller ve çizgiler rows, at both sizes and in both themes.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn settings_screens() {
    let out = root().join(".run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let mut app = crate::files_testing::app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let _ = app.update(Message::Run("tools.options"));
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("stil-ayarlar-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let Some(doc) = demo() else {
        eprintln!("the web's demo drawing is not written: see the module's comment");
        return;
    };
    let out = root().join(".run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let (mut app, _) = App::boot(None);
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc.clone())))));
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            for (name, layer) in VIEWS {
                let Some(b) = extent(&app, layer) else {
                    eprintln!("{layer}: no objects");
                    continue;
                };
                let camera = &mut app.viewport.camera;
                // 1:1000 on a 96 dpi screen, the section's top left a little inside the view.
                camera.scale = 1.0 / (kentos_native_style::program::METRES_PER_PX * 1000.0);
                let (w, h) = (camera.width / camera.scale, camera.height / camera.scale);
                camera.center_on(kentos_render_wgpu::Vec2::new(
                    b.min_x - 4.0 + w / 2.0,
                    b.max_y + 16.0 - h / 2.0,
                ));
                let started = Instant::now();
                snapshot.settle(&mut app, App::view, &mut update);
                // More frames while the atlas leaves images for later (a frame's budget; the app asks
                // for the next frame itself, docs/adr/0090).
                for _ in 0..60 {
                    let _ = snapshot.render(app.view(), &app.theme());
                    if !app.viewport.status().images_pending {
                        break;
                    }
                }
                let file = out.join(format!("stil-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!(
                    "{} ({:?} with the build {:?})",
                    file.display(),
                    started.elapsed(),
                    app.viewport.styled_build()
                );
            }
        }
    }
}
