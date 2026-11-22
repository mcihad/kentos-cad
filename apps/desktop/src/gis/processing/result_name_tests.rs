//! İşlemler's inputs and result name (docs/adr/9144), the municipal report's 11th item: Tampon opened while an object
//! is selected reads the selection, not the last run's layer; the last run's result name does not come back (the
//! tool's own, “Tampon”, does), a name typed stays; an input that reads nothing leaves Çalıştır off.

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
    w.values.get("layer").and_then(|v| v.get("newName")).and_then(|v| v.as_str()).unwrap_or_default().to_owned()
}

#[test]
fn tampon_reads_the_selection_and_proposes_its_result_by_it() {
    let mut app = app_with_drawing();
    // A last run on another layer under another name.
    app.gis.processing.memory.remember(
        "geometry.buffer",
        &[
            ("input".to_owned(), json!({ "scope": "layer", "layerId": "cizim" })),
            ("distance".to_owned(), json!(10.0)),
            ("layer".to_owned(), json!({ "newName": "Park koruma 10 m" })),
        ]
        .into_iter()
        .collect(),
    );
    // The parcel selected.
    let parcel = {
        let doc = app.document.as_ref().expect("open");
        doc.model.by_layer("parsel").next().map(|e| kentos_domain::Slot(e.base().id)).expect("a parcel")
    };
    app.selection.set([parcel]);
    let _ = app.update(Message::Run("processing.run.geometry.buffer"));
    let w = app.gis.processing.dialog.as_ref().expect("the window");
    assert_eq!(w.values.get("input"), Some(&json!({ "scope": "selection" })), "the selection, not the last layer");
    assert_eq!(result_name(&app), "Tampon", "the tool's own name, not the last run's");
    assert!(!w.input_empty());
    // A name typed is the person's: it stays.
    ev(&mut app, Event::LayerName("layer".into(), "Parsel etki alanı".into()));
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
    assert_eq!(w.values.get("input"), Some(&json!({ "scope": "selection" })));
    assert!(w.input_empty(), "the selection holds nothing Tampon takes");
}
