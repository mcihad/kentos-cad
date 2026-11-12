//! Geçiş önerisi (transition_window.rs, plan/road.md Y34.3): opened from Dever on the schedule as typed, it reads
//! `road.superelevations.transitions` as the shared cases do, and Çizelgeye yaz puts the runoffs and shares into
//! Dever's cells, which Kaydet writes; and its pictures.

use crate::app::{App, Message};
use crate::road;
use crate::superelevation_window::Event as D;
use crate::transition_window::Event as T;

const CASES: &str = include_str!("../../../../fixtures/commands/v1/road.superelevations.transitions.json");

/// The drawing of the shared cases: the main alignment's d-1, the side's d-2 by rows, the winding alignment's d-5.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: T) {
    let _ = app.update(Message::Road(road::Event::Superelevations(D::Transition(
        e,
    ))));
}

/// The winding alignment's schedule with every limit of the shared case "Kıvrımlı aks: hepsi birlikte".
fn typed(app: &mut App) {
    let _ = app.update(Message::Run("superelevation.editor"));
    let at = app
        .road
        .superelevation_window
        .as_ref()
        .unwrap()
        .rows
        .iter()
        .position(|r| r.id == "d-5")
        .expect("d-5");
    let _ = app.update(Message::Road(road::Event::Superelevations(D::Choose(at))));
    let _ = app.update(Message::Road(road::Event::Superelevations(
        D::OpenTransition,
    )));
    for (k, v) in [
        (0, "250"),
        (1, "70"),
        (2, "0,6"),
        (3, "0,2"),
        (4, "1"),
        (5, "0,67"),
        (6, "5"),
    ] {
        send(app, T::Field(k, v.into()));
    }
}

fn shared(name: &str) -> serde_json::Value {
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("cases");
    cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).expect("the case")["steps"][0]["result"]["output"].clone()
}

#[test]
fn the_proposal_is_the_shared_cases_and_is_written_into_the_schedule() {
    let mut app = app();
    typed(&mut app);
    let x = app
        .road
        .superelevation_window
        .as_ref()
        .and_then(|w| w.transition.as_deref())
        .expect("open");
    let out = x.result.clone().expect("read").expect("reads");
    let want = shared("Kıvrımlı aks: hepsi birlikte");
    assert_eq!(out.rows.len(), 3);
    for (r, w) in out.rows.iter().zip(want["rows"].as_array().unwrap()) {
        assert_eq!(r.pi, w["pi"].as_str().unwrap());
        assert!((r.runoff.unwrap() - w["runoff"].as_f64().unwrap()).abs() < 1e-9, "{r:?}");
    }
    assert_eq!(
        out.findings.len(),
        want["findings"].as_array().unwrap().len()
    );
    send(&mut app, T::Apply);
    let w = app.road.superelevation_window.as_ref().unwrap();
    assert!(w.transition.is_none(), "Çizelgeye yaz goes back to Dever");
    let def = w.rows[w.chosen.unwrap()].def().expect("builds");
    let kentos_commands::SuperelevationMethod::Curves { curves } = def.method else {
        panic!("by curves")
    };
    assert_eq!((curves[0].runoff, curves[0].on_tangent), (55.0, 0.67));
    assert_eq!((curves[1].runoff, curves[1].on_tangent), (95.0, 0.0));
}

#[test]
fn a_schedule_by_rows_has_no_proposal_and_n_must_be_above_one() {
    let mut app = app();
    let _ = app.update(Message::Run("superelevation.editor"));
    let at = app
        .road
        .superelevation_window
        .as_ref()
        .unwrap()
        .rows
        .iter()
        .position(|r| r.id == "d-5")
        .unwrap();
    let _ = app.update(Message::Road(road::Event::Superelevations(D::Choose(at))));
    let _ = app.update(Message::Road(road::Event::Superelevations(
        D::OpenTransition,
    )));
    send(&mut app, T::Field(0, "1".into()));
    let x = app
        .road
        .superelevation_window
        .as_ref()
        .and_then(|w| w.transition.as_deref())
        .unwrap();
    let why = x.result.clone().unwrap().expect_err("1:1");
    assert!(why.contains("1'den büyük bir n"), "{why}");
    send(&mut app, T::Back);
    assert!(
        app.road
            .superelevation_window
            .as_ref()
            .unwrap()
            .transition
            .is_none()
    );
}

/// The window, wide and narrow, light and dark, and Dever after Çizelgeye yaz; `.run/gui-evidence/yol-y34/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib transition_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    use kentos_ui::widget::sizable::sizes;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y34");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name, full) in [
        ("light", Size::new(1440.0, 900.0), "acik", false),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar", false),
        ("light", Size::new(1920.0, 1080.0), "tam-ekran", true),
    ] {
        for step in ["oneri", "dever"] {
            for key in [crate::transition_window::TITLE, crate::superelevation_window::TITLE] {
                if full {
                    sizes::set(key, sizes::Kept { width: 800.0, height: 600.0, maximized: true });
                } else {
                    sizes::forget(key);
                }
            }
            let mut app = app();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::RibbonTab("alignment"));
            typed(&mut app);
            if step == "dever" {
                // Limits the alignment can take (the shared case "1:150 ve teğet payı 0, geçişler sığar"), written.
                for (k, v) in [(0, "150"), (1, ""), (2, ""), (3, ""), (4, ""), (5, "0"), (6, "")] {
                    send(&mut app, T::Field(k, v.into()));
                }
                send(&mut app, T::Apply);
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("y34-{step}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        }
    }
    for key in [crate::transition_window::TITLE, crate::superelevation_window::TITLE] {
        sizes::forget(key);
    }
}
