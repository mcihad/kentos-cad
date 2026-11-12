//! Ortofoto doğruluğu (ortho_window.rs, docs/adr/1480) on the shared cases' drawing of `cad.orthophoto.check`: the
//! selected orthophoto, its layers guessed (GNSS, read on the image), KOH within BÖHHBÜY's limit at 1/1000 and out of
//! it at 1/500, the fit points apart, the CSV, nothing written; Harita and GIS offer it. And its pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::ortho_window::Event as O;

fn opened(program: Option<Module>) -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.orthophoto.check.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

#[test]
fn the_orthophoto_is_checked_against_the_gnss_points() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("raster.accuracy"));
    assert_ne!(app.dialog, Some(Dialog::Ortho), "no raster selected");
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("raster.accuracy"));
    assert_eq!(app.dialog, Some(Dialog::Ortho));
    assert_eq!((app.ortho.known.as_str(), app.ortho.observed.as_str()), ("gnss", "okuma"));
    let r = app.ortho.answer.clone().expect("an answer").expect("checked");
    assert_eq!((r.check.n, r.fit.as_ref().map(|f| f.n)), (8, Some(2)));
    assert_eq!(r.passed_xy, Some(true));
    let _ = app.update(Message::Ortho(O::Scale(500.0)));
    let r = app.ortho.answer.clone().expect("an answer").expect("checked");
    assert_eq!(r.passed_xy, Some(false), "7.14 cm at 1/500");
    let dir = std::env::temp_dir().join(format!("kentos-ortofoto-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("dogruluk.csv");
    app.ortho_save(file.clone());
    let text = std::fs::read_to_string(&file).expect("written");
    assert_eq!(text.lines().filter(|l| l.contains(";Denetim;")).count(), 8);
    assert_eq!(text.lines().filter(|l| l.contains(";Uyum;")).count(), 2);
    let _ = std::fs::remove_dir_all(dir);
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

#[test]
fn harita_and_gis_offer_it() {
    for module in [Module::Map, Module::Gis] {
        assert!(opened(Some(module)).offers("raster.accuracy"), "{module:?}");
    }
}

/// `.run/gui-evidence/ortofoto-dogruluk-1480/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ortho_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/ortofoto-dogruluk-1480");
    std::fs::create_dir_all(&out).expect("a folder");
    for (scale, stem) in [(1000.0, "1-binlik"), (500.0, "2-beslik-disinda")] {
        for (theme, size, name) in
            [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")]
        {
            let mut app = opened(Some(Module::Map));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            app.selection.set([Slot(1)]);
            app.follow_selection_tab();
            let _ = app.update(Message::Run("raster.accuracy"));
            let _ = app.update(Message::Ortho(O::Scale(scale)));
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
    // The raster's context tab: where Doğruluk sits.
    let mut app = opened(Some(Module::Map));
    let size = Size::new(1920.0, 1000.0);
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::Run("view.zoomExtents"));
    app.selection.set([Slot(1)]);
    app.follow_selection_tab();
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("3-menu-raster-sekmesi.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
}
