//! Tarama kaydı (register_window.rs, docs/adr/1800) on the shared cases' drawing of `cad.pointclouds.register`: the
//! drawing's first two scans, the GNSS layer guessed; the moved scan fitted as the shared case says, its control
//! points' KOH_Z down from 5 cm; the flat pair's plan shift loose; the CSV with the matrix; nothing written; Harita
//! and GIS offer it. And its pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::register_window::{Event as R, View};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.pointclouds.register.json")).expect("cases")
}

fn opened(program: Option<Module>) -> App {
    let mut text = cases()["setup"].to_string();
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
fn the_moved_scan_is_fitted_as_the_shared_case_says() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("pointcloud.register"));
    assert_eq!(app.dialog, Some(Dialog::Register));
    assert_eq!(app.register.control, "gnss");
    let r = app.register.answer.clone().expect("an answer").expect("registered");
    let want = &cases()["cases"][0]["steps"][0]["result"]["output"];
    assert_eq!((Value::from(r.iterations), Value::from(r.converged), Value::from(r.pairs)), (want["iterations"].clone(), want["converged"].clone(), want["pairs"].clone()));
    for (i, t) in r.translation.iter().enumerate() {
        assert!((t - want["translation"][i].as_f64().expect("a number")).abs() < 1e-7, "{i}: {t}");
    }
    let (before, after) = (r.control_before.clone().expect("before"), r.control_after.clone().expect("after"));
    assert!(before.koh_z > 0.04 && after.koh_z < 0.01, "{} → {}", before.koh_z, after.koh_z);
    // The flat pair: the height found, the plan shift loose.
    let (c, d) = (app.register.clouds[2].0.clone(), app.register.clouds[3].0.clone());
    let _ = app.update(Message::Register(R::Fixed(c)));
    let _ = app.update(Message::Register(R::Moving(d)));
    let flat = app.register.answer.clone().expect("an answer").expect("registered");
    assert!(flat.horizontal < 0.1 && (flat.translation[2] + 0.05).abs() < 0.01, "{} {:?}", flat.horizontal, flat.translation);
    let dir = std::env::temp_dir().join(format!("kentos-kayit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("kayit.csv");
    app.register_save(file.clone());
    let text = std::fs::read_to_string(&file).expect("written");
    assert!(text.starts_with("Kural paketi;lidar-uyum-ornek;") && text.contains("\nMatris (hareketli → sabit)\n"), "{text}");
    let _ = std::fs::remove_dir_all(dir);
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

#[test]
fn harita_and_gis_offer_it() {
    for module in [Module::Map, Module::Gis] {
        assert!(opened(Some(module)).offers("pointcloud.register"), "{module:?}");
    }
}

/// `.run/gui-evidence/tarama-kaydi-1800/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib register_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tarama-kaydi-1800");
    std::fs::create_dir_all(&out).expect("a folder");
    for (pair, view, stem) in [((0usize, 1usize), View::Control, "1-kaydirilmis-kontrol"), ((2, 3), View::Worst, "2-duz-zemin")] {
        for (theme, size, name) in
            [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")]
        {
            let mut app = opened(Some(Module::Map));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("pointcloud.register"));
            let (a, b) = (app.register.clouds[pair.0].0.clone(), app.register.clouds[pair.1].0.clone());
            let _ = app.update(Message::Register(R::Fixed(a)));
            let _ = app.update(Message::Register(R::Moving(b)));
            let _ = app.update(Message::Register(R::View(view)));
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
    // Where it sits: the point cloud's context tab, beside Tarama uyumu.
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
