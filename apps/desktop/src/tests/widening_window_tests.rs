//! Genişletme önerisi (widening_window.rs, plan/road.md Y28.3): opened from Dever on the schedule as typed, it reads
//! `road.superelevations.widen`, and Çizelgeye yaz puts the suggestion into Dever's Genişleme cell, which Kaydet
//! writes; and its pictures.

use crate::app::{App, Message};
use crate::road;
use crate::superelevation_window::Event as D;
use crate::widening_window::{Event as W, Method};

/// The drawing of `road.superelevations.query`'s cases: the main alignment with p1 (R 150 m, 40 m clothoids), d-1 on
/// it (6 %, widening 0,5) and d-2 from rows on the side alignment.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.superelevations.query.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Road(road::Event::Superelevations(D::Widen(e))));
}

/// A tractor and semitrailer of WB-19's sizes, 0,9 m clearance, 80 km/h, rounded up to 0,1 m.
fn typed(app: &mut App) {
    let _ = app.update(Message::Run("superelevation.editor"));
    let _ = app.update(Message::Road(road::Event::Superelevations(D::Choose(0))));
    let _ = app.update(Message::Road(road::Event::Superelevations(D::OpenWiden)));
    for (k, v) in [(0, "6,6"), (1, "1,2"), (2, "2,59"), (6, "0,9"), (7, "80"), (8, "0,1")] {
        send(app, W::Field(k, v.into()));
    }
    send(app, W::Trailer(true));
    send(app, W::Field(4, "-0,8".into()));
    send(app, W::Field(5, "12,5".into()));
}

fn widen(app: &App) -> &crate::widening_window::Widen {
    app.road
        .superelevation_window
        .as_ref()
        .and_then(|w| w.widen.as_deref())
        .expect("open")
}

#[test]
fn the_suggestion_is_read_and_written_into_the_schedule() {
    let mut app = app();
    typed(&mut app);
    let out = widen(&app).result.clone().expect("read").expect("reads");
    assert_eq!((out.lanes, out.tangent), (2, 7.0), "Tip 1's two 3,5 m lanes");
    let p1 = &out.curves[0];
    assert_eq!(p1.pi, "p1");
    assert!((p1.offtracking - 0.6654).abs() < 1e-3, "{p1:?}");
    assert!((p1.suggested - 2.1).abs() < 1e-9, "2,0215 rounded up: {p1:?}");
    assert!((p1.current - 0.5).abs() < 1e-9);
    // Basit drops F_A and Z; Sür drives the curve, under the steady state.
    send(&mut app, W::Method(Method::Simple));
    let simple = widen(&app).result.clone().unwrap().unwrap().curves[0].widening;
    assert!(simple < p1.widening - 0.6, "{simple}");
    send(&mut app, W::Method(Method::Standard));
    send(&mut app, W::Drive(true));
    let driven = widen(&app).result.clone().unwrap().unwrap().curves[0].driven.expect("driven");
    assert!(driven <= p1.offtracking + 0.01 && driven > 0.9 * p1.offtracking, "{driven}");
    // Çizelgeye yaz: back to Dever with 2,1 in p1's cell; Kaydet writes it.
    send(&mut app, W::Apply);
    assert!(
        app.road
            .superelevation_window
            .as_ref()
            .unwrap()
            .widen
            .is_none(),
        "back to Dever"
    );
    let _ = app.update(Message::Road(road::Event::Superelevations(D::Save)));
    let doc = &app.document.as_ref().unwrap().model;
    match &doc.road_superelevations()[0].method {
        kentos_commands::SuperelevationMethod::Curves { curves } => assert!((curves[0].widening - 2.1).abs() < 1e-9),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_missing_size_says_what_to_type_and_rows_are_not_written() {
    let mut app = app();
    let _ = app.update(Message::Run("superelevation.editor"));
    let _ = app.update(Message::Road(road::Event::Superelevations(D::Choose(0))));
    let _ = app.update(Message::Road(road::Event::Superelevations(D::OpenWiden)));
    let why = widen(&app).result.clone().expect("read").expect_err("not yet");
    assert!(why.contains("Dingil aralığı"), "{why}");
    send(&mut app, W::Back);
    // d-2 is from rows: the suggestion reads, Çizelgeye yaz is not offered.
    let _ = app.update(Message::Road(road::Event::Superelevations(D::Choose(1))));
    typed(&mut app);
    let _ = app.update(Message::Road(road::Event::Superelevations(D::Choose(1))));
    send(&mut app, W::Field(7, "60".into()));
    let out = widen(&app).result.clone().expect("read").expect("reads");
    assert!(out.curves.is_empty(), "the side alignment has no curve");
}

/// The window, wide and narrow, light and dark; `.run/gui-evidence/yol-y26/y28-3-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib widening_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y26");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("y28-3-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("superelevation.editor"));
        let _ = app.update(Message::Road(road::Event::Superelevations(D::Choose(0))));
        let _ = app.update(Message::Road(road::Event::Superelevations(D::OpenWiden)));
        shot(&mut app, "bos");
        typed(&mut app);
        send(&mut app, W::Field(9, "0,6".into()));
        shot(&mut app, "oneri");
        send(&mut app, W::Drive(true));
        shot(&mut app, "surus");
        send(&mut app, W::Apply);
        shot(&mut app, "dever");
    }
}

/// Full screen: the road windows' buttons sit at the bottom of the box (`.run/gui-evidence/yol-y26/tam-ekran-*`).
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_full_screen() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    use kentos_ui::widget::sizable::sizes;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y26");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut app = app();
    let mut snapshot = Snapshot::new(Size::new(1920.0, 1080.0)).expect("a renderer");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    let mut shot = |app: &mut App, file: &str| {
        snapshot.settle(app, App::view, &mut update);
        let file = out.join(format!("tam-ekran-{file}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    };
    for key in ["Dever", "Genişletme önerisi", "Enkesit", "Kübaj"] {
        sizes::set(key, sizes::Kept { width: 800.0, height: 600.0, maximized: true });
    }
    let _ = app.update(Message::Run("superelevation.editor"));
    let _ = app.update(Message::Road(road::Event::Superelevations(D::Choose(1))));
    shot(&mut app, "dever");
    let _ = app.update(Message::Road(road::Event::Superelevations(D::Choose(0))));
    let _ = app.update(Message::Road(road::Event::Superelevations(D::OpenWiden)));
    shot(&mut app, "genisletme");
    let _ = app.update(Message::Road(road::Event::Superelevations(D::Widen(
        W::Back,
    ))));
    let _ = app.update(Message::Road(road::Event::Superelevations(D::Cancel)));
    let _ = app.update(Message::Run("section.editor"));
    shot(&mut app, "enkesit");
    let _ = app.update(Message::Road(road::Event::Sections(
        crate::section_window::Event::Cancel,
    )));
    // The other road windows whose buttons moved to the footer.
    for (key, run, file) in [("Boykesit", "profile.new", "boykesit"), ("Güzergâh", "alignment.new", "guzergah"), ("Otokorkuluk", "guardrail.editor", "otokorkuluk")] {
        sizes::set(key, sizes::Kept { width: 800.0, height: 600.0, maximized: true });
        let _ = app.update(Message::Run(run));
        shot(&mut app, file);
        let _ = app.update(Message::CapturedEscape);
    }
}
