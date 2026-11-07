//! Uygulama kontrolü (asbuilt_window.rs, docs/adr/1700) on the shared cases' drawing of `cad.asbuilt.check`: the
//! window opens on the measured layer, the design points, the kerb and the design surface guessed by their names, no
//! limit; the cases' limits typed in the form give the shared case's counts; a limit that is no number is said;
//! Dışındakileri seç selects the points not within; CSV kaydet writes the table; nothing is written to the drawing.
//! Harita, Yol and GIS offer it, Mimari does not. And its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::asbuilt_window::{Event as A, Show};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.asbuilt.check.json")).expect("cases")
}

fn opened(program: Option<Module>) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn limits(app: &mut App) {
    for e in [
        A::Search("0,5".into()),
        A::Horizontal("0,02".into()),
        A::Vertical("0,03".into()),
        A::QualityField("Hassasiyet".into()),
        A::Quality("0,05".into()),
    ] {
        let _ = app.update(Message::Asbuilt(e));
    }
}

#[test]
fn the_measured_points_against_the_design() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("asbuilt.check"));
    assert_eq!(app.dialog, Some(Dialog::Asbuilt));
    assert_eq!(
        (app.asbuilt.measured.as_str(), app.asbuilt.design.as_str(), app.asbuilt.lines.as_str(), app.asbuilt.surface.as_str()),
        ("olcum", "tasarim", "bordur", "t-1")
    );
    limits(&mut app);
    let r = app.asbuilt.answer.clone().expect("an answer").expect("checked");
    let want = &cases()["cases"][0]["steps"][0]["result"]["output"];
    for key in ["within", "outside", "unmatched", "ambiguous", "lowQuality"] {
        let got = match key {
            "within" => r.within,
            "outside" => r.outside,
            "unmatched" => r.unmatched,
            "ambiguous" => r.ambiguous,
            _ => r.low_quality,
        };
        assert_eq!(Value::from(got), want[key], "{key}");
    }
    let _ = app.update(Message::Asbuilt(A::Show(Show::Faults)));
    let _ = app.update(Message::Asbuilt(A::Select));
    assert_eq!(app.dialog, None);
    let faults = (r.outside + r.unmatched + r.ambiguous) as usize;
    assert_eq!(app.selection.ids().len(), faults);
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

#[test]
fn a_limit_that_is_no_number_is_said() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("asbuilt.check"));
    let _ = app.update(Message::Asbuilt(A::Horizontal("iki".into())));
    assert_eq!(app.asbuilt.answer, Some(Err("Yatay tolerans “iki” bir sayı değil.".into())));
}

#[test]
fn csv_kaydet_writes_the_table() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("asbuilt.check"));
    limits(&mut app);
    let dir = std::env::temp_dir().join(format!("kentos-uygulama-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("kontrol.csv");
    app.asbuilt_save(file.clone());
    let text = std::fs::read_to_string(&file).expect("written");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 11, "a head and ten points");
    assert!(lines[2].starts_with("P2;") && lines[2].contains(";Dışında;yatay dışında"), "{}", lines[2]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn harita_yol_and_gis_offer_it_mimari_does_not() {
    for (module, yes) in [(Module::Map, true), (Module::Road, true), (Module::Gis, true), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module)).offers("asbuilt.check"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/uygulama-kontrolu-1700/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib asbuilt_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/uygulama-kontrolu-1700");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("asbuilt.check"));
        limits(&mut app);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-kontrol-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        // The ribbon's place: Ölçme › Saha.
        let _ = app.update(Message::Asbuilt(A::Close));
        let _ = app.update(Message::RibbonTab("survey"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-serit-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
