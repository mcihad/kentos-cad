//! Uygunluk analizi in the desktop's window (docs/adr/0237) over the valley
//! with its three criteria and the landslides (fixtures/interaction/v1/
//! suitability.kcad): the window's rows are the criteria by their layers'
//! names in the panel's order; Ağırlıklı çakıştırma writes its classes
//! right above the first criterion's layer, undone whole; İkili
//! karşılaştırma gives its weights' table without touching the drawing;
//! ROC ile doğrulama its curve and AUC.

use serde_json::{Value, json};

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::suitability_scenes::{overlay, pairwise, roc, with_criteria};

fn value(app: &mut App, name: &str, v: Value) {
    let _ = app.update(Message::Processing(Event::Value(name.into(), v)));
}

fn run(app: &mut App) -> RunStatus {
    super::surface_tests::run(app)
}

fn valley() -> App {
    let (mut app, _) = App::boot(None);
    with_criteria(&mut app);
    app
}

fn window(app: &mut App, tool: &'static str, values: &[(&str, Value)]) {
    let _ = app.update(Message::Run(tool));
    for (name, v) in values {
        value(app, name, v.clone());
    }
}

fn layer_at(app: &App, name: &str) -> (String, usize) {
    let layers = app.document.as_ref().expect("open").model.layers();
    let id = layers
        .leaves()
        .into_iter()
        .find(|l| l.name == name)
        .map(|l| l.id.clone())
        .unwrap_or_else(|| panic!("{name}"));
    let (_, at) = layers.place_of(&id).expect("placed");
    (id, at)
}

#[test]
fn the_rows_are_the_criteria_and_the_overlay_goes_above_the_first() {
    let mut app = valley();
    window(
        &mut app,
        "processing.run.suitability.weightedOverlay",
        &overlay(),
    );
    let rasters = app.processing.dialog.as_ref().expect("the window").inputs["input"]
        .rasters
        .clone();
    assert_eq!(rasters, ["Eğim", "Yola uzaklık", "Arazi örtüsü"]);
    let (text, undo) = match run(&mut app) {
        RunStatus::Ok { text, undo, .. } => (text, undo),
        s => panic!("{s:?}"),
    };
    assert!(undo);
    assert!(
        text.starts_with("480 × 324 hücrelik raster; “")
            && text.contains("3 raster birleşti. Kısıtlı "),
        "{text}"
    );
    let (_, at) = layer_at(&app, "Ağırlıklı çakıştırma");
    let (_, slope) = layer_at(&app, "Eğim");
    assert_eq!(at + 1, slope);
    let doc = &mut app.document.as_mut().expect("open").model;
    assert_eq!(doc.undo().as_deref(), Some("Ağırlıklı çakıştırma"));
    assert!(
        doc.layers()
            .leaves()
            .iter()
            .all(|l| l.name != "Ağırlıklı çakıştırma")
    );
}

#[test]
fn the_weights_come_in_a_table_and_the_drawing_stays() {
    let mut app = valley();
    let before = app
        .document
        .as_ref()
        .expect("open")
        .model
        .entities()
        .count();
    window(&mut app, "processing.run.suitability.pairwise", &pairwise());
    match run(&mut app) {
        RunStatus::Ok {
            text, undo, table, ..
        } => {
            assert!(!undo);
            assert!(
                text.starts_with("3 ölçütün ağırlıkları tabloda; λ ")
                    && text.ends_with("(tutarlı)."),
                "{text}"
            );
            let t = table.expect("the weights' table");
            assert_eq!(t.columns, ["Ölçüt", "Ağırlık", "Yüzde (%)"]);
            let names: Vec<&str> = t.rows.iter().map(|r| r[0].as_str()).collect();
            assert_eq!(names, ["Eğim", "Yola uzaklık", "Arazi örtüsü"]);
            let w: Vec<f64> = t
                .rows
                .iter()
                .map(|r| r[1].parse().expect("a weight"))
                .collect();
            assert!(w[0] > w[1] && w[1] > w[2], "{w:?}");
            assert!((w.iter().sum::<f64>() - 1.0).abs() < 2e-6, "{w:?}");
        }
        s => panic!("{s:?}"),
    }
    assert_eq!(
        app.document
            .as_ref()
            .expect("open")
            .model
            .entities()
            .count(),
        before
    );
}

#[test]
fn the_curve_says_how_well_the_slope_finds_the_landslides() {
    let mut app = valley();
    window(&mut app, "processing.run.suitability.roc", &roc());
    match run(&mut app) {
        RunStatus::Ok { text, table, .. } => {
            assert!(text.starts_with("AUC 0."), "{text}");
            assert!(
                text.contains("varlık hücresi") && text.contains("En iyi eşik"),
                "{text}"
            );
            let t = table.expect("the curve");
            assert_eq!(t.columns[4], "Alan oranı (%)");
            // The last row's threshold takes every landslide.
            assert_eq!(t.rows.last().expect("a row")[2], "100.00");
        }
        s => panic!("{s:?}"),
    }
    // The rows of a pair field follow the input too.
    window(
        &mut app,
        "processing.run.suitability.pairwise",
        &[("input", json!({ "scope": "selection" }))],
    );
    let w = app.processing.dialog.as_ref().expect("the window");
    let (rows, note) = kentos_processing::raster_rows::pairs_view(
        &w.values["comparisons"],
        Some(&w.inputs["input"].rasters),
    );
    assert_eq!(note, None);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].text, "Eşit");
}
