//! Senaryo ölçüleri's access along a network, its slope map on the drawing and its table written in the drawing
//! (impact_window.rs, impact_reach_marks.rs, docs/adr/5060) on the shared cases' drawings: the window's access along
//! “Yollar” carries the case's routes and Erişimi çizimde göster draws them; Eğimi çizimde göster colours the case's
//! parts with a key of the classes; Ölçüleri çizime yaz writes the table on “Senaryo ölçüleri” in one undo step and the
//! window reads it back as current, as stale once the base changes. And their pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Message};
use crate::impact_window::{Event as I, View};

fn case(name: &str) -> Value {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.scenarios.impact.json")).expect("cases");
    cases["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(name))).expect("the case").clone()
}

fn opened(setup: &Value) -> App {
    // The network is built by İşlemler's tools, as the app registers them when it starts.
    kentos_processing::command::register_as("desktop");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup.clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("scenario.impact"));
    app
}

fn ask(app: &mut App, events: Vec<I>) {
    for e in events {
        let _ = app.update(Message::Impact(e));
    }
}

const NET: &str = "ağ üzerinden erişim: konuttan";
const MAP: &str = "eğim haritası: alanların";

fn network(app: &mut App) {
    ask(
        app,
        vec![
            I::From("konut".into()),
            I::To("park, yesil".into()),
            I::Distance("100".into()),
            I::Network("yollar".into()),
            I::Connect("40".into()),
        ],
    );
}

fn slope(app: &mut App) {
    ask(app, vec![I::SlopeSurface("t-1".into()), I::SlopeThreshold("10".into()), I::SlopeClasses("5; 15; 30".into())]);
}

#[test]
fn the_access_along_the_network_is_the_case_s_and_drawn() {
    let c = case(NET);
    let mut app = opened(&c["setup"]);
    network(&mut app);
    let r = app.impact.answer.clone().expect("an answer").expect("compares");
    let want = &c["steps"][0]["result"]["output"]["reach"]["alternativeRoutes"];
    let reach = r.reach.expect("the access");
    assert_eq!(reach.alternative_routes.len(), want.as_array().expect("routes").len());
    for (g, w) in reach.alternative_routes.iter().zip(want.as_array().expect("routes")) {
        assert!((g.distance - w["distance"].as_f64().expect("a number")).abs() < 1e-9);
    }
    // A leg that does not read is said, not guessed.
    ask(&mut app, vec![I::Connect("kırk".into())]);
    let why = app.impact.answer.clone().expect("an answer").expect_err("refused");
    assert!(why.contains("Ağa bağlanma uzaklığı “kırk” bir sayı değil"), "{why}");
    ask(&mut app, vec![I::Connect("40".into()), I::DrawAccess]);
    let m = app.impact_reach_marks().expect("the marks");
    assert_eq!(m.routes.len(), 3);
}

#[test]
fn the_slope_map_is_the_case_s_parts() {
    let c = case(MAP);
    let mut app = opened(&c["setup"]);
    slope(&mut app);
    ask(&mut app, vec![I::DrawSlope]);
    let m = app.impact_reach_marks().expect("the marks");
    let want = c["steps"][0]["result"]["output"]["slopeMap"]["alternative"].as_array().expect("parts").clone();
    assert_eq!(m.pieces.len(), want.len());
    for ((_, class), w) in m.pieces.iter().zip(&want) {
        assert_eq!(*class as u64, w["class"].as_u64().expect("a class"));
    }
    assert_eq!(m.key.len(), 4, "four classes in the key");
    let _ = app.update(Message::Run("data.unmark"));
    assert!(app.impact_reach_marks().is_none());
}

#[test]
fn the_table_is_written_in_one_step_and_read_back() {
    let c = case(NET);
    let mut app = opened(&c["setup"]);
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    ask(&mut app, vec![I::Record]);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.entities().count(), before + 1);
    assert!(doc.layers().leaves().iter().any(|l| l.name == crate::impact_window::RECORD_LAYER));
    let r = app.impact.answer.clone().expect("an answer").expect("compares");
    assert_eq!(r.records.len(), 1);
    assert!(!r.records[0].base_changed && !r.records[0].scenario_changed);
    let _ = app.update(Message::Run("edit.undo"));
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.entities().count(), before, "one undo step takes the table");
    assert!(!doc.layers().leaves().iter().any(|l| l.name == crate::impact_window::RECORD_LAYER), "and its layer");
}

/// `.run/gui-evidence/map-10-10/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib impact_network_record_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/map-10-10");
    std::fs::create_dir_all(&out).expect("a folder");
    let size = Size::new(1440.0, 900.0);
    let shot = |app: &mut App, stem: &str, theme: &str, size: Size| {
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(app, App::view, &mut update);
        let file = out.join(format!("{stem}-{theme}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    };
    for theme in ["light", "dark"] {
        // 1: the window with the network and the table written in the drawing.
        let c = case(NET);
        let mut app = opened(&c["setup"]);
        let _ = app.update(Message::WindowResized(Size::new(1440.0, 1100.0)));
        network(&mut app);
        ask(&mut app, vec![I::Record, I::View(View::Measures)]);
        shot(&mut app, "1-pencere-ag-tablo", theme, Size::new(1440.0, 1400.0));
        // 2: the routes along the network on the drawing.
        ask(&mut app, vec![I::DrawAccess]);
        shot(&mut app, "2-ag-erisim-cizimde", theme, size);
        // 3: the slope map with its key.
        let c = case(MAP);
        let mut app = opened(&c["setup"]);
        slope(&mut app);
        ask(&mut app, vec![I::DrawSlope]);
        shot(&mut app, "3-egim-haritasi", theme, size);
        // 4: the table in the drawing, the comparison's numbers.
        let c = case(NET);
        let mut app = opened(&c["setup"]);
        network(&mut app);
        ask(&mut app, vec![I::Record, I::Close]);
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut app, "4-tablo-cizimde", theme, size);
    }
}
