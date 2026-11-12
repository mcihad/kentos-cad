//! Senaryo ölçüleri's slope classes and aspect (impact_window.rs, docs/adr/4660) on the shared case's drawing: with the
//! surface, a threshold, the breaks “5; 15; 30” and Bakıya göre alanlar the window's comparison carries the case's class
//! and aspect measures; breaks that do not read are said, not guessed. And its pictures.

use crate::gis;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Message};
use crate::impact_window::{Event as I, View};

const CASE: &str = "eğim sınıfları ve bakı: iki üçgen ayrı sınıflarda, bakıları güney ve güneybatı";

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
    for e in [
        I::SlopeSurface("t-1".into()),
        I::SlopeThreshold("10".into()),
        I::SlopeClasses("5; 15; 30".into()),
        I::SlopeAspect(true),
    ] {
        let _ = app.update(Message::Gis(gis::Event::Impact(e)));
    }
    app
}

#[test]
fn the_classes_and_aspects_are_the_case_s() {
    let mut app = opened();
    let r = app
        .gis
        .impact
        .answer
        .clone()
        .expect("an answer")
        .expect("compares");
    let want = &case()["steps"][0]["result"]["output"];
    let ids: Vec<String> = want["measures"]
        .as_array()
        .expect("measures")
        .iter()
        .filter_map(|m| m["id"].as_str().filter(|id| id.starts_with("slopeClass") || id.starts_with("aspect")).map(str::to_owned))
        .collect();
    assert_eq!(ids.len(), 4 + 9);
    for id in &ids {
        let got = r.measures.iter().find(|m| &m.id == id).expect("the measure");
        let wanted = want["measures"].as_array().expect("measures").iter().find(|m| m["id"] == id.as_str()).expect("wanted");
        assert_eq!(got.label, wanted["label"].as_str().expect("a label"));
        for (g, w) in [(got.base, &wanted["base"]), (got.alternative, &wanted["alternative"])] {
            assert!((g - w.as_f64().expect("a number")).abs() < 1e-6, "{id}: {g} != {w}");
        }
    }
    // A break that does not read is said, not guessed.
    let _ = app.update(Message::Gis(gis::Event::Impact(I::SlopeClasses(
        "5; on".into(),
    ))));
    let why = app
        .gis
        .impact
        .answer
        .clone()
        .expect("an answer")
        .expect_err("refused");
    assert!(
        why.contains("Eğim sınıfı sınırı “on” bir sayı değil"),
        "{why}"
    );
}

/// `.run/gui-evidence/egim-sinif-baki-4660/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib impact_slope_classes_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/egim-sinif-baki-4660");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("light", Size::new(1440.0, 1700.0), "acik-uzun")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Gis(gis::Event::Impact(I::View(View::Measures))));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-sinif-baki-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
