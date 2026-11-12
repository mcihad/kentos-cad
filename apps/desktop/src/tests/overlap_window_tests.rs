//! Tarama uyumu (overlap_window.rs, docs/adr/1500) on the shared cases' drawing of `cad.pointclouds.overlap`: the
//! drawing's first two scans, the GNSS layer guessed; strip B within the interswath limit with its 4 cm shift, strip C
//! out of it; the control points' KOH_Z; the CSV; nothing written; Harita and GIS offer it. And its pictures.

use crate::map;
use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::overlap_window::{Event as O, View};

fn opened(program: Option<Module>) -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pointclouds.overlap.json")).expect("cases");
    let mut text = cases["setup"].to_string();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    text = text.replace("$fixture:", &format!("{}/", root.display()));
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(&text).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

#[test]
fn two_scans_are_compared_and_the_shifted_one_fails() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("pointcloud.overlap"));
    assert_eq!(app.dialog, Some(Dialog::Map(map::MapDialogKind::Overlap)));
    assert_eq!(app.map.overlap.control, "gnss");
    let r = app
        .map
        .overlap
        .answer
        .clone()
        .expect("an answer")
        .expect("compared");
    assert_eq!((r.cells, r.passed), (100, Some(true)));
    assert!((r.mean - 0.04).abs() < 0.005, "{}", r.mean);
    assert_eq!(r.control_a.as_ref().and_then(|c| c.passed), Some(true));
    let c = app.map.overlap.clouds[2].0.clone();
    let _ = app.update(Message::Map(map::Event::Overlap(O::B(c))));
    let r = app
        .map
        .overlap
        .answer
        .clone()
        .expect("an answer")
        .expect("compared");
    assert_eq!(r.passed, Some(false), "25 cm out of place");
    let dir = std::env::temp_dir().join(format!("kentos-tarama-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("uyum.csv");
    app.overlap_save(file.clone());
    let text = std::fs::read_to_string(&file).expect("written");
    assert!(text.starts_with("Kural paketi;lidar-uyum-ornek;") && text.contains("\nK1;"), "{text}");
    let _ = std::fs::remove_dir_all(dir);
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

#[test]
fn harita_and_gis_offer_it() {
    for module in [Module::Map, Module::Gis] {
        assert!(opened(Some(module)).offers("pointcloud.overlap"), "{module:?}");
    }
}

/// `.run/gui-evidence/tarama-uyumu-1500/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib overlap_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tarama-uyumu-1500");
    std::fs::create_dir_all(&out).expect("a folder");
    for (second, view, stem) in [(1usize, View::Control, "1-b-kontrol"), (3, View::Worst, "2-d-arac-en-kotu")] {
        for (theme, size, name) in
            [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")]
        {
            let mut app = opened(Some(Module::Map));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("pointcloud.overlap"));
            let b = app.map.overlap.clouds[second].0.clone();
            let _ = app.update(Message::Map(map::Event::Overlap(O::B(b))));
            let _ = app.update(Message::Map(map::Event::Overlap(O::View(view))));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
    // Where it sits: the point cloud's context tab.
    let mut app = opened(Some(Module::Map));
    let size = Size::new(1920.0, 1000.0);
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::Run("view.zoomExtents"));
    app.selection.set([Slot(1), Slot(2)]);
    app.follow_selection_tab();
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("3-menu-bulut-sekmesi.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
}
