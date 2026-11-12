//! Senaryo ölçüleri's terrain slope (impact_window.rs, docs/adr/4260) on the shared case's drawing: the surface chosen
//! and a threshold written, the window's comparison carries the case's four slope measures and its gap; without a
//! threshold the slope is not asked. And its pictures.

use crate::gis;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Message};
use crate::impact_window::{Event as I, View};

const CASE: &str = "eğim: Zemin yüzeyinde, dik eşik 10 %; senaryonun doğudaki konutu yüzeyden taşar";

fn case() -> Value {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.scenarios.impact.json")).expect("cases");
    cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == CASE).expect("the case").clone()
}

fn opened() -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("scenario.impact"));
    app
}

#[test]
fn the_slope_is_measured_once_a_surface_and_a_threshold_are_given() {
    let mut app = opened();
    assert_eq!(
        app.gis
            .impact
            .surfaces
            .iter()
            .map(|(_, n)| n.as_str())
            .collect::<Vec<_>>(),
        ["—", "Zemin"]
    );
    let _ = app.update(Message::Gis(gis::Event::Impact(I::SlopeSurface(
        "t-1".into(),
    ))));
    let r = app
        .gis
        .impact
        .answer
        .clone()
        .expect("an answer")
        .expect("compares");
    assert!(
        r.measures.iter().all(|m| !m.id.starts_with("slope")),
        "no threshold: not asked"
    );
    let _ = app.update(Message::Gis(gis::Event::Impact(I::SlopeThreshold(
        "10".into(),
    ))));
    let r = app
        .gis
        .impact
        .answer
        .clone()
        .expect("an answer")
        .expect("compares");
    let want = &case()["steps"][0]["result"]["output"];
    for id in ["slopeMean", "slopeSteep", "slopeSteepShare", "slopeOff"] {
        let got = r.measures.iter().find(|m| m.id == id).expect(id);
        let wanted = want["measures"].as_array().expect("measures").iter().find(|m| m["id"] == id).expect(id);
        assert_eq!(got.label, wanted["label"].as_str().expect("a label"));
        for (g, w) in [(got.base, &wanted["base"]), (got.alternative, &wanted["alternative"])] {
            assert!((g - w.as_f64().expect("a number")).abs() < 1e-6, "{id}: {g} != {w}");
        }
    }
    assert!(
        r.gaps
            .iter()
            .any(|g| g.contains("senaryoda Zemin yüzeyinin dışında")),
        "{:?}",
        r.gaps
    );
    // A threshold that does not read is said, not guessed.
    let _ = app.update(Message::Gis(gis::Event::Impact(I::SlopeThreshold(
        "on".into(),
    ))));
    let why = app
        .gis
        .impact
        .answer
        .clone()
        .expect("an answer")
        .expect_err("refused");
    assert!(why.contains("Dik eğim eşiği “on” bir sayı değil"), "{why}");
}

/// `.run/gui-evidence/senaryo-egim-4260/`: the measures with the slope on Zemin, 10 %.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib impact_slope_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/senaryo-egim-4260");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 760.0), "koyu-dar"),
        // Tall enough for the whole table, the slope rows at its end.
        ("light", Size::new(1440.0, 1500.0), "acik-uzun"),
    ] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("scenario.impact"));
        for e in [
            I::SlopeSurface("t-1".into()),
            I::SlopeThreshold("10".into()),
            I::View(View::Measures),
        ] {
            let _ = app.update(Message::Gis(gis::Event::Impact(e)));
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("olculer-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
