//! Uygulama kontrolü › İzleme, docs/adr/4420 (plan/map-cad.md MAP-19): Önceki dönemler names the earlier epochs'
//! layers, oldest first; each epoch's day is its layer's Veri kümesi bilgisi (Güncelleme günü); the answer's tracks are
//! the shared case's, and the window's series table shows them. A name no layer has is said. And its pictures.

use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::asbuilt_window::Event as A;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.asbuilt.check.series.json")).expect("cases")
}

/// The series cases' drawing, each epoch's layer given its day as its Veri kümesi bilgisi.
fn opened() -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    for (layer, day) in [("donem1", "2026-01-15"), ("donem2", "2026-04-15"), ("donem3", "2026-07-15")] {
        crate::program_scenes::command(&mut app, "cad.datasets.edit", json!({"layer": layer, "dataset": {"updated": day}}));
    }
    app
}

fn series(app: &mut App, epochs: &str) {
    for e in [
        A::Kind(2),
        A::Measured("donem3".into()),
        A::Design("donem2".into()),
        A::Horizontal("0,008".into()),
        A::Vertical("0,01".into()),
        A::Control("K1, K2".into()),
        A::Epochs(epochs.into()),
    ] {
        let _ = app.update(Message::Asbuilt(e));
    }
}

#[test]
fn onceki_donemler_track_each_mark_as_the_shared_case() {
    let mut app = opened();
    let _ = app.update(Message::Run("asbuilt.check"));
    series(&mut app, "Birinci dönem, İkinci dönem");
    let r = app.asbuilt.answer.clone().expect("an answer").expect("checked");
    let all = cases();
    let want = &all["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("dizi: üç dönem"))).expect("the case")["steps"][0]["result"]["output"]["tracks"];
    let got = serde_json::to_value(&r.tracks).expect("json");
    assert_eq!(got.as_array().map(Vec::len), want.as_array().map(Vec::len));
    for (g, w) in got.as_array().expect("tracks").iter().zip(want.as_array().expect("tracks")) {
        assert_eq!((&g["name"], &g["from"]), (&w["name"], &w["from"]));
        for key in ["rateHorizontal", "rateVertical"] {
            assert!((g[key].as_f64().expect("a rate") - w[key].as_f64().expect("a rate")).abs() < 1e-9, "{key} {g}");
        }
    }
    assert_eq!(app.asbuilt.epoch_titles, ["1. dönem", "2. dönem", "Bu dönem"]);
    // A name no layer has is said.
    series(&mut app, "Birinci dönem, Yok");
    let said = app.asbuilt.answer.clone().expect("an answer").expect_err("refused");
    assert!(said.contains("“Yok” adlı katman yok"), "{said}");
}

/// `.run/gui-evidence/donem-dizisi-4420/`: the monitoring form with Önceki dönemler and the series table.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib asbuilt_series_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/donem-dizisi-4420");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 1000.0), "acik-genis"), ("dark", Size::new(1100.0, 800.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("asbuilt.check"));
        series(&mut app, "Birinci dönem, İkinci dönem");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("izleme-dizisi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
