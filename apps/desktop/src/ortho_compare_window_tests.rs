//! Ortofoto karşılaştırması (ortho_compare_window.rs, docs/adr/1780) on the shared cases' drawing of
//! `cad.orthophoto.compare`: with nothing selected the drawing's two rasters in order, the layers guessed (GNSS, each
//! image's reading layer by the year in its file's name); the answer is the shared case's with the ground's points and
//! without, and Yer değiştir turns the shifts over as the case does; a selected raster comes first; one raster in the
//! drawing is not enough; the CSV; nothing written; Harita and GIS offer it. Dikiş ölçümü gives the shared seam case's
//! places (docs/adr/2460). And its pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::ortho_compare_window::Event as C;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.orthophoto.compare.json")).expect("cases")
}

/// A shared case by the start of its name.
fn case(name: &str) -> Value {
    cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(name))).cloned().expect(name)
}

fn open(setup: &Value, program: Option<Module>) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup.clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn opened(program: Option<Module>) -> App {
    open(&cases()["setup"], program)
}

/// The same JSON, numbers within 1e-9.
fn close(got: &Value, want: &Value, at: &str) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().expect("a number"), b.as_f64().expect("a number"));
            assert!((a - b).abs() <= 1e-9, "{at}: {a} ≠ {b}");
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}: length");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                close(x, y, &format!("{at}[{i}]"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                close(a.get(k).unwrap_or(&Value::Null), b.get(k).unwrap_or(&Value::Null), &format!("{at}.{k}"));
            }
        }
        _ => assert_eq!(got, want, "{at}"),
    }
}

fn answer(app: &App) -> Value {
    serde_json::to_value(app.ortho_compare.answer.clone().expect("an answer").expect("compared")).expect("json")
}

fn output(name: &str) -> Value {
    case(name)["steps"][0]["result"]["output"].clone()
}

#[test]
fn the_two_orthophotos_are_compared_as_the_shared_cases() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("raster.compare"));
    assert_eq!(app.dialog, Some(Dialog::OrthoCompare));
    let s = &app.ortho_compare;
    let uid = |n: u64| format!("0192f5c0-1780-7000-8000-{n:012x}");
    assert_eq!((s.first.clone(), s.second.clone()), (uid(1), uid(2)));
    assert_eq!((s.first_points.as_str(), s.second_points.as_str(), s.known.as_str()), ("okuma-2024", "okuma-2026", "gnss"));
    close(&answer(&app), &output("iki zaman, arazi noktalarıyla"), "with the ground");
    let _ = app.update(Message::OrthoCompare(C::Known("— yok —".into())));
    assert_eq!(app.ortho_compare.known, "");
    close(&answer(&app), &output("arazi noktaları olmadan"), "images only");
    let _ = app.update(Message::OrthoCompare(C::Swap));
    close(&answer(&app), &output("sıra değişince"), "swapped");
    let dir = std::env::temp_dir().join(format!("kentos-ortofoto-karsilastirma-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("karsilastirma.csv");
    app.ortho_compare_save(file.clone());
    let text = std::fs::read_to_string(&file).expect("written");
    assert_eq!(text.lines().filter(|l| l.ends_with(";Hayır")).count(), 10);
    let _ = std::fs::remove_dir_all(dir);
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

#[test]
fn a_selected_raster_comes_first_and_one_raster_is_not_enough() {
    let mut app = opened(Some(Module::Map));
    app.selection.set([Slot(2)]);
    let _ = app.update(Message::Run("raster.compare"));
    let s = &app.ortho_compare;
    assert_eq!((s.first_points.as_str(), s.second_points.as_str()), ("okuma-2026", "okuma-2024"), "the selected 2026 first");
    // A point over the limit is the shared case's.
    let blunder = case("kaba okunmuş nokta");
    let mut app = open(&blunder["setup"], Some(Module::Map));
    let _ = app.update(Message::Run("raster.compare"));
    let _ = app.update(Message::OrthoCompare(C::Known("— yok —".into())));
    close(&answer(&app), &blunder["steps"][0]["result"]["output"], "over");
    // The check's drawing has one raster.
    let check: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.orthophoto.check.json")).expect("cases");
    let mut app = open(&check["setup"], Some(Module::Map));
    let _ = app.update(Message::Run("raster.compare"));
    assert_ne!(app.dialog, Some(Dialog::OrthoCompare));
}

#[test]
fn harita_and_gis_offer_it() {
    for module in [Module::Map, Module::Gis] {
        assert!(opened(Some(module)).offers("raster.compare"), "{module:?}");
    }
}

/// `.run/gui-evidence/ortofoto-karsilastirma-1780/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ortho_compare_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/ortofoto-karsilastirma-1780");
    std::fs::create_dir_all(&out).expect("a folder");
    let blunder = case("kaba okunmuş nokta")["setup"].clone();
    for (setup, stem) in [(cases()["setup"].clone(), "1-iki-zaman"), (blunder, "2-siniri-asan")] {
        for (theme, size, name) in
            [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")]
        {
            let mut app = open(&setup, Some(Module::Map));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            app.selection.set([Slot(1), Slot(2)]);
            app.follow_selection_tab();
            let _ = app.update(Message::Run("raster.compare"));
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
    // The raster's context tab: where Karşılaştır sits, after Doğruluk.
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
    let file = out.join("3-serit-raster-sekmesi.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
}

use crate::ortho_compare_window::Show;

/// A case's setup with its `$fixture:` paths made absolute (the shared runners do it the same way).
fn with_fixtures(setup: &Value) -> Value {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/");
    let text = setup.to_string().replace("$fixture:", &root.display().to_string());
    serde_json::from_str(&text).expect("the setup")
}

/// docs/adr/2460: on the seam cases' two synthetic images, Dikiş ölçümü with the case's step, window, search and least
/// correlation gives the case's places, counts and shift; its table is the seam's; nothing written.
#[test]
fn dikis_olcumu_gives_the_shared_case_s_places() {
    let c = case("dikiş: örtüşmede ölçülen kayma");
    let mut app = open(&with_fixtures(&c["setup"]), Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("raster.compare"));
    assert_eq!(app.dialog, Some(Dialog::OrthoCompare));
    for e in [C::Seam(true), C::Step(1.0), C::Window(21.0), C::Search(5.0), C::Correlation("0,8".into())] {
        let _ = app.update(Message::OrthoCompare(e));
    }
    assert_eq!(app.ortho_compare.show, Show::Seam);
    let got = serde_json::to_value(app.ortho_compare.answer.clone().expect("an answer").expect("compared")).expect("json");
    close(&got["seam"], &c["steps"][0]["result"]["output"]["seam"], "seam");
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

/// `.run/gui-evidence/dikis-olcumu-2460/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ortho_compare_window_tests::seam_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn seam_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dikis-olcumu-2460");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name, case_name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis", "dikiş: örtüşmede ölçülen kayma"),
        ("dark", Size::new(1100.0, 760.0), "koyu-dar", "dikiş: 2,7 piksellik kayma"),
    ] {
        let c = case(case_name);
        let mut app = open(&with_fixtures(&c["setup"]), Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("raster.compare"));
        for e in [C::Seam(true), C::Step(1.0), C::Window(21.0), C::Search(5.0), C::Correlation("0,8".into())] {
            let _ = app.update(Message::OrthoCompare(e));
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-dikis-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// docs/adr/2860: the seam line chosen in the drawing (Seçili çizgiyi al) gives the shared case's places along it, the
/// distance along the line in the table; with nothing chosen it is said and the grid stays; Hattı kaldır goes back to
/// the overlap's grid.
#[test]
fn the_seam_is_measured_along_the_chosen_line() {
    let c = case("dikiş hattı boyunca");
    let mut app = open(&with_fixtures(&c["setup"]), Some(Module::Map));
    let _ = app.update(Message::Run("raster.compare"));
    for e in [C::Seam(true), C::Step(1.0), C::Window(21.0), C::Search(5.0), C::Correlation("0,8".into())] {
        let _ = app.update(Message::OrthoCompare(e));
    }
    let _ = app.update(Message::OrthoCompare(C::TakeLine));
    assert!(app.ortho_compare.seam_line.is_empty(), "nothing chosen: no line");
    app.selection.set(vec![Slot(3)]);
    let _ = app.update(Message::OrthoCompare(C::TakeLine));
    assert_eq!(app.ortho_compare.seam_line_name, "Çoklu çizgi");
    let got = serde_json::to_value(app.ortho_compare.answer.clone().expect("an answer").expect("compared")).expect("json");
    close(&got["seam"], &c["steps"][0]["result"]["output"]["seam"], "seam");
    let (_, rows) = app.ortho_compare_lines();
    assert_eq!(rows[0][0], "4.000", "the first place 4 m along the line");
    let _ = app.update(Message::OrthoCompare(C::DropLine));
    let got = serde_json::to_value(app.ortho_compare.answer.clone().expect("an answer").expect("compared")).expect("json");
    assert!(got["seam"].get("line").is_none(), "the overlap's grid again");
}

/// `.run/gui-evidence/dikis-hatti-2860/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ortho_compare_window_tests::line_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn line_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dikis-hatti-2860");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let c = case("dikiş hattı boyunca");
        let mut app = open(&with_fixtures(&c["setup"]), Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("raster.compare"));
        for e in [C::Seam(true), C::Step(1.0), C::Window(21.0), C::Search(5.0), C::Correlation("0,8".into())] {
            let _ = app.update(Message::OrthoCompare(e));
        }
        app.selection.set(vec![Slot(3)]);
        let _ = app.update(Message::OrthoCompare(C::TakeLine));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-dikis-hatti-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
