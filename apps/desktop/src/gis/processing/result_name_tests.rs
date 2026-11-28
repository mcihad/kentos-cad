//! İşlemler's inputs and result name (docs/adr/9144, 9189), the municipal report's 11th item: Tampon opened while an
//! object is selected reads the selection, not the last run's layer; the last run's result name does not come back,
//! the new layer is named after what the run reads (“Parsel — tampon 10 m”) and follows it while no name is typed; a
//! name typed stays; an input that reads nothing leaves Çalıştır off.

use serde_json::json;

use super::Event;
use crate::app::{App, Message};
use crate::files_testing::app_with_drawing;
use crate::gis;

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Gis(gis::Event::Processing(e)));
}

fn result_name(app: &App) -> String {
    let w = app.gis.processing.dialog.as_ref().expect("the window");
    w.values
        .get("layer")
        .and_then(|v| v.get("newName"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn tampon_reads_the_selection_and_proposes_its_result_by_it() {
    let mut app = app_with_drawing();
    // A last run on another layer under another name.
    app.gis.processing.memory.remember(
        "geometry.buffer",
        &[
            (
                "input".to_owned(),
                json!({ "scope": "layer", "layerId": "cizim" }),
            ),
            ("distance".to_owned(), json!(10.0)),
            ("layer".to_owned(), json!({ "newName": "Park koruma 10 m" })),
        ]
        .into_iter()
        .collect(),
    );
    // The parcel selected.
    let parcel = {
        let doc = app.document.as_ref().expect("open");
        doc.model
            .by_layer("parsel")
            .next()
            .map(|e| kentos_domain::Slot(e.base().id))
            .expect("a parcel")
    };
    app.selection.set([parcel]);
    let _ = app.update(Message::Run("processing.run.geometry.buffer"));
    let w = app.gis.processing.dialog.as_ref().expect("the window");
    assert_eq!(
        w.values.get("input"),
        Some(&json!({ "scope": "selection" })),
        "the selection, not the last layer"
    );
    assert_eq!(
        result_name(&app),
        "Parsel — tampon 10 m",
        "named after what the run reads, not the last run's"
    );
    assert!(!w.input_empty());
    // The input's line names the selection's layer and its count (docs/adr/9157).
    let said = w
        .inputs
        .get("input")
        .map(|i| i.description.clone())
        .unwrap_or_default();
    assert!(
        said.ends_with("seçili, “Parsel” katmanından (katmanda 1 nesne)"),
        "{said}"
    );
    // A name typed is the person's: it stays.
    ev(
        &mut app,
        Event::LayerName("layer".into(), "Parsel etki alanı".into()),
    );
    ev(&mut app, Event::Number("distance".into(), "30".into()));
    assert_eq!(result_name(&app), "Parsel etki alanı");
}

#[test]
fn an_input_that_reads_nothing_leaves_calistir_off() {
    let mut app = app_with_drawing();
    // Only a text selected: Tampon takes no text, the selection reads nothing.
    let text = {
        let doc = app.document.as_ref().expect("open");
        doc.model
            .entities()
            .find(|e| matches!(e, kentos_commands::Entity::Text(_)))
            .map(|e| kentos_domain::Slot(e.base().id))
            .expect("a text")
    };
    app.selection.set([text]);
    let _ = app.update(Message::Run("processing.run.geometry.buffer"));
    let w = app.gis.processing.dialog.as_ref().expect("the window");
    assert_eq!(
        w.values.get("input"),
        Some(&json!({ "scope": "selection" }))
    );
    assert!(w.input_empty(), "the selection holds nothing Tampon takes");
}

/// The derived name follows the input and the measure while no name is typed (docs/adr/9189): another distance, another
/// layer; a selection of two layers' objects is “Seçim”; an input of no one layer (the view) leaves the tool's own
/// name; a name typed stops it; history's values keep theirs.
#[test]
fn the_derived_name_follows_the_input_and_the_measure_until_a_name_is_typed() {
    let mut app = app_with_drawing();
    let _ = app.update(Message::Run("processing.run.geometry.buffer"));
    // The active layer, Parsel, at the tool's distance.
    let first = result_name(&app);
    assert!(
        first.starts_with("Parsel — tampon ") && first.ends_with(" m"),
        "{first}"
    );
    ev(&mut app, Event::Number("distance".into(), "2.5".into()));
    assert_eq!(result_name(&app), "Parsel — tampon 2.5 m");
    ev(&mut app, Event::Scope("input".into(), "layer".into()));
    let w = app.gis.processing.dialog.as_ref().expect("the window");
    let mut input = w.values.get("input").cloned().expect("the input");
    input["layerId"] = json!("bina");
    ev(&mut app, Event::Value("input".into(), input));
    assert_eq!(result_name(&app), "Bina — tampon 2.5 m");
    // Objects of two layers: Seçim.
    let two: Vec<kentos_domain::Slot> = {
        let doc = app.document.as_ref().expect("open");
        ["parsel", "bina"]
            .iter()
            .filter_map(|l| {
                doc.model
                    .by_layer(l)
                    .next()
                    .map(|e| kentos_domain::Slot(e.base().id))
            })
            .collect()
    };
    app.selection.set(two);
    ev(&mut app, Event::Scope("input".into(), "selection".into()));
    assert_eq!(result_name(&app), "Seçim — tampon 2.5 m");
    // The view is no one layer: the name stays as it was.
    ev(&mut app, Event::Scope("input".into(), "view".into()));
    assert_eq!(result_name(&app), "Seçim — tampon 2.5 m");
    // A name typed stops it.
    ev(
        &mut app,
        Event::LayerName("layer".into(), "Koruma bandı".into()),
    );
    ev(&mut app, Event::Number("distance".into(), "5".into()));
    assert_eq!(result_name(&app), "Koruma bandı");
    // History's values keep their name: Yeniden aç is what was run.
    let values: kentos_processing::Values = [
        (
            "input".to_owned(),
            json!({ "scope": "layer", "layerId": "parsel" }),
        ),
        ("distance".to_owned(), json!(10.0)),
        ("layer".to_owned(), json!({ "newName": "Park koruma 10 m" })),
    ]
    .into_iter()
    .collect();
    app.open_processing("geometry.buffer", Some(values));
    assert_eq!(result_name(&app), "Park koruma 10 m");
}

/// The picture (`.run/gui-evidence/kullanim-9189/`): Tampon run over the selected parcel at 10 m; its new layer, named
/// after what it read, in Katmanlar.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9189", "1-turetilmis-ad", || {
        let mut app = app_with_drawing();
        let parcel = {
            let doc = app.document.as_ref().expect("open");
            doc.model
                .by_layer("parsel")
                .next()
                .map(|e| kentos_domain::Slot(e.base().id))
                .expect("a parcel")
        };
        app.selection.set([parcel]);
        let _ = app.update(Message::Run("processing.run.geometry.buffer"));
        ev(&mut app, Event::Number("distance".into(), "10".into()));
        let task = app.update(Message::Gis(gis::Event::Processing(Event::Run)));
        crate::files_testing::drive(&mut app, task);
        app.gis.processing.dialog = None;
        app.dialog = None;
        let _ = app.update(Message::Run("view.zoomExtents"));
        app
    });
}
