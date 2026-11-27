//! The network analyses in İşlemler's history (the CBS review's 18th item, docs/adr/9334), on the review's roads: A
//! (500000, 4320000), C (500030, 4320000), B (500030, 4320040). Hizmet alanı, En kısa yol and Şebeke izleme each leave a
//! record beside İşlemler's runs: the network by its id, the cost with its unit, the breaks, the stops, the layer it
//! wrote or the objects it chose. Yeniden aç starts the tool with them again. The layers they open carry what made them;
//! the project written and opened again keeps it: the tool, the time, the values, the network's layer with its digest,
//! still current.

use serde_json::json;

use super::prerequisite_tests::{click, prompt, send, settings};
use super::service_area_tests::{A, B, at_35_m, drawn_back, service_area, typed};
use crate::app::{App, Message};
use crate::processing::Event;
use crate::processing::panel::Event as PanelEvent;

fn history(app: &App) -> Vec<kentos_processing::RunRecord> {
    app.gis.processing.runner.history().to_vec()
}

fn reopen(app: &mut App, seq: u64) {
    send(app, Message::Gis(crate::gis::Event::Processing(Event::Panel(PanelEvent::Reopen(seq)))));
}

/// The layer named `name`.
fn layer(app: &App, name: &str) -> String {
    let model = &app.document.as_ref().expect("open").model;
    model.layers().leaves().into_iter().find(|n| n.name == name).map(|n| n.id.clone()).expect(name)
}

fn network(app: &App) -> String {
    app.document.as_ref().expect("open").model.settings().networks[0].id.clone()
}

/// Hizmet alanı at 35 m from A, the lines written too: its run.
fn served() -> App {
    let mut app = service_area();
    at_35_m(&mut app);
    typed(&mut app, "");
    app
}

/// En kısa yol from A to B written, then Şebeke izleme from A selecting what is connected.
fn routed_and_traced(app: &mut App) {
    send(app, Message::Run("tool.netRoute"));
    click(app, A.0, A.1);
    click(app, B.0, B.1);
    typed(app, "");
    send(app, Message::Run("tool.netTrace"));
    click(app, A.0, A.1);
    typed(app, "");
}

#[test]
fn a_service_area_is_a_run_reopened_with_its_values_and_its_layer_keeps_what_made_it() {
    let mut app = served();
    let net = network(&app);
    let runs = history(&app);
    let [r] = &runs[..] else { panic!("one run: {runs:?}") };
    assert_eq!((r.tool_id.as_str(), r.label.as_str(), r.status), ("tool.netServiceArea", "Hizmet alanı", kentos_processing::Status::Ok));
    assert_eq!(r.values["network"], json!(net));
    assert_eq!(r.values["cost"], json!("Uzunluk"));
    assert_eq!(r.values["breaks"], json!([35.0]));
    assert_eq!(r.values["trim"], json!(50.0));
    assert_eq!(r.values["lines"], json!(true));
    assert_eq!(r.values["stops"], json!([[A.0, A.1]]));
    let f = app.format();
    for w in [
        format!("Ağ: qa_yollar ağı ({net})"),
        "Maliyet: Uzunluk, metre".to_owned(),
        "Tesisler: 1".to_owned(),
        "Aralıklar: 35 m".to_owned(),
        format!("Kenar payı: {}", f.length(50.0)),
        "Çıktı katmanı: “Yaklaşık hizmet alanı” (yeni katman)".to_owned(),
        "Ulaşılan yollar: “Ulaşılan yollar” (yeni katman)".to_owned(),
    ] {
        assert!(r.words.contains(&w), "{w} in {:?}", r.words);
    }
    // The three roads read; the area and the three reached lines written.
    assert_eq!((r.added.len(), r.records_in, r.records_out), (4, 3, 4));
    // The layers it opened say what made them.
    let area = layer(&app, "Yaklaşık hizmet alanı");
    let model = &app.document.as_ref().expect("open").model;
    let d = model.layers().info_of(&area).and_then(|i| i.derivation).expect("a derivation");
    assert_eq!((d.tool.as_str(), d.label.as_str(), d.tool_version), ("tool.netServiceArea", "Hizmet alanı", 1));
    assert_eq!(d.parameters["breaks"], json!([35.0]));
    assert_eq!(d.parameters["network"], json!(net));
    assert_eq!(d.inputs.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["qa_yollar"]);
    assert_eq!(kentos_commands::derivation_problem(&d), None, "a record that holds");
    let lines = layer(&app, "Ulaşılan yollar");
    assert!(model.layers().info_of(&lines).and_then(|i| i.derivation).is_some());
    // Written and opened again: the record is there and its input is still what it read.
    let path = crate::files_testing::scratch("analiz-kaydi").join("analiz.kcad");
    crate::document::write(&model.to_snapshot_v2(), &path).expect("written");
    let again = crate::document::Document::read(&path).expect("read again");
    assert_eq!(again.model.layers().info_of(&area).and_then(|i| i.derivation), Some(d));
    let mut opened = crate::files_testing::app_with_drawing();
    let _ = opened.update(Message::Opened(Some(Ok(Box::new(again)))));
    let described = crate::files_testing::command(&mut opened, "cad.datasets.describe", json!({ "layers": [area] }));
    let set = &described["output"]["datasets"][0];
    assert_eq!(set["derivation"]["tool"], json!("tool.netServiceArea"), "{described}");
    assert_eq!(set["inputs"][0]["state"], json!("current"), "{described}");
    // A stale derivation says so; a current one leaves the word out.
    assert_ne!(set["stale"], json!(true), "{described}");
    // Yeniden aç: the tool again with its breaks, margin, lines and the facility at A; the areas as they were.
    reopen(&mut app, r.seq);
    assert_eq!(app.session.tool_id(), "netServiceArea");
    assert!(prompt(&app).contains("1 tesis; 3 çizgi, 1 alan"), "{}", prompt(&app));
    let rows = settings(&app);
    assert_eq!((rows[4].1.as_str(), rows[7].1.as_str()), ("35", "Açık"));
}

#[test]
fn a_route_and_a_trace_are_runs_too_and_reopen_with_their_points() {
    let mut app = served();
    routed_and_traced(&mut app);
    let runs = history(&app);
    let labels: Vec<&str> = runs.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels, ["Şebeke izleme", "En kısa yol", "Hizmet alanı"], "newest first");
    // En kısa yol: two stops, the route on Rota, which says what made it.
    let route = &runs[1];
    assert_eq!(route.tool_id, "tool.netRoute");
    assert_eq!(route.values["stops"], json!([[A.0, A.1], [B.0, B.1]]));
    assert!(route.words.contains(&"Duraklar: 2".to_owned()), "{:?}", route.words);
    assert!(route.words.contains(&"Çıktı katmanı: “Rota” (yeni katman)".to_owned()), "{:?}", route.words);
    assert_eq!(route.added.len(), 1);
    let rota = layer(&app, "Rota");
    let model = &app.document.as_ref().expect("open").model;
    let d = model.layers().info_of(&rota).and_then(|i| i.derivation).expect("Rota's derivation");
    assert_eq!(d.tool, "tool.netRoute");
    // Şebeke izleme: what it chose, no layer.
    let trace = &runs[0];
    assert_eq!(trace.tool_id, "tool.netTrace");
    assert!(trace.words.contains(&"Tür: bağlı".to_owned()), "{:?}", trace.words);
    assert!(trace.words.contains(&"Başlangıç noktaları: 1".to_owned()), "{:?}", trace.words);
    assert!(trace.words.contains(&"Seçilen nesneler: 3".to_owned()), "{:?}", trace.words);
    assert_eq!((trace.added.len(), trace.touched.len()), (0, 3));
    // Yeniden aç: En kısa yol with A and B, the direct 50 m road; Şebeke izleme with A.
    reopen(&mut app, route.seq);
    assert_eq!(app.session.tool_id(), "netRoute");
    assert!(prompt(&app).contains("2 durak") && prompt(&app).contains("Uzunluk 50.000 m"), "{}", prompt(&app));
    reopen(&mut app, trace.seq);
    assert_eq!(app.session.tool_id(), "netTrace");
    assert!(prompt(&app).contains("120.000 m"), "{}", prompt(&app));
}

/// The pictures (`.run/gui-evidence/kullanim-9334/`): İşlemler › Geçmiş with the three analyses beside each other;
/// Hizmet alanı reopened from its record; the area's layer's Soy after the project was written and opened again.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop-cbs analysis_history_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9334", "01-gecmiste-analizler", || {
        let mut app = served();
        routed_and_traced(&mut app);
        send(&mut app, Message::Run("processing.history"));
        app
    });
    crate::files_testing::usage_shots("9334", "02-yeniden-ac", || {
        let mut app = served();
        let seq = history(&app)[0].seq;
        reopen(&mut app, seq);
        drawn_back(&mut app);
        send(&mut app, Message::Run("processing.history"));
        app
    });
    crate::files_testing::usage_shots("9334", "03-soy", || {
        let app = served();
        let area = layer(&app, "Yaklaşık hizmet alanı");
        let model = &app.document.as_ref().expect("open").model;
        let path = crate::files_testing::scratch("analiz-soy").join("analiz.kcad");
        crate::document::write(&model.to_snapshot_v2(), &path).expect("written");
        let mut again = crate::document::Document::read(&path).expect("read again");
        again.model.set_active_layer(&area);
        let mut opened = crate::files_testing::app_with_drawing();
        let _ = opened.update(Message::Opened(Some(Ok(Box::new(again)))));
        send(&mut opened, Message::Run("layer.dataset"));
        send(&mut opened, Message::Gis(crate::gis::Event::DatasetInfo(crate::dataset_window::Event::Tab(2))));
        opened
    });
}
