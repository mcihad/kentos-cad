//! GNSS ağ dengelemesi's confidence regions (survey_network_window.rs, plan/map-cad.md MAP-05, docs/adr/2560): on
//! the shared case whose baselines' deviations are oblong and turned 30°, every new mark's error ellipse is the case's
//! (semi-axes, direction, height's interval at 0.95), the summary names the largest and Noktalar's column shows each
//! with its direction as a semt in the project's angle unit; on the base case the round ellipses have no direction.
//! And the pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::survey_network_window::{Event as N, View};

const OBLONG: &str = "Bazların sapmaları 1,5 ve 4 mm, eksenleri 30° dönük: noktaların hata elipsi o yöne uzanır, yönü \
                      verilir";

fn case(name: &str) -> Value {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.survey.network.json")).expect("cases");
    cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case").clone()
}

fn opened(setup: &Value) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup.clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("map.survey.adjustment"));
    assert_eq!(app.dialog, Some(Dialog::SurveyNetwork));
    app
}

fn answer(app: &App) -> kentos_commands::MapSurveyNetworked {
    app.survey_network.answer.clone().expect("an answer").expect("adjusted")
}

#[test]
fn every_new_mark_has_the_case_s_confidence_region() {
    let c = case(OBLONG);
    let mut app = opened(&c["setup"]);
    let a = answer(&app);
    let want = c["steps"][0]["result"]["output"]["stations"].as_array().expect("stations").clone();
    for (st, w) in a.stations.iter().zip(&want) {
        match (&st.ellipse, w.get("ellipse")) {
            (None, None) => assert!(st.held, "{}", st.mark),
            (Some(e), Some(w)) => {
                for (got, key) in [(e.major, "major"), (e.minor, "minor"), (e.up, "up"), (e.confidence, "confidence")] {
                    assert!((got - w[key].as_f64().expect(key)).abs() < 1e-9, "{} {key}", st.mark);
                }
                let az = e.azimuth.expect("an oblong ellipse has a direction");
                assert!((az - w["azimuth"].as_f64().expect("azimuth")).abs() < 1e-6, "{}", st.mark);
                assert!((az - 30.0).abs() < 0.1, "{}: {az}", st.mark);
            }
            other => panic!("{}: {other:?}", st.mark),
        }
    }
    let line = crate::survey_network_window::region_line(&a).expect("a region line");
    assert!(line.starts_with("Güven bölgesi (%95): en büyük hata elipsi N5 noktasında"), "{line}");
    // Noktalar's column: the semi-axes, the direction as a semt in grads (the case's project), the height's interval.
    let _ = app.update(Message::SurveyNetwork(N::View(View::Marks)));
    let n5 = a.stations.iter().find(|s| s.mark == "N5").and_then(|s| s.ellipse.clone()).expect("N5's ellipse");
    let mm = |v: f64| format!("{:.1}", v * 1000.0);
    let cell = format!(
        "{}/{} · {} · ±{}",
        mm(n5.major),
        mm(n5.minor),
        app.format().bearing(n5.azimuth.expect("a direction") * 10.0 / 9.0),
        mm(n5.up)
    );
    let mut shot = crate::files_testing::offscreen(iced::Size::new(1440.0, 900.0));
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    shot.settle(&mut app, App::view, &mut update);
    assert!(crate::files_testing::find_text(&mut shot, &app, &cell).is_some(), "{cell}");
    // The base case's round ellipses: no direction.
    let base = opened(&case("İki bilinen nokta, üç işaret, sekiz baz: serbest ve bilinen noktalara dayalı dengeleme; N1'in kabul edilen noktasından farkı")["setup"]);
    assert!(answer(&base).stations.iter().filter_map(|s| s.ellipse.as_ref()).all(|e| e.azimuth.is_none()));
}

/// The oblong case's Noktalar with its confidence regions, light wide and dark narrow,
/// `.run/gui-evidence/guven-bolgesi-2560/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib survey_region_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/guven-bolgesi-2560");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(&case(OBLONG)["setup"]);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::SurveyNetwork(N::View(View::Marks)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-noktalar-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
