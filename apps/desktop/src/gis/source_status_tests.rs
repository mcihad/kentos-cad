//! Kaynak durumu over data layers and the table's layer picker (docs/adr/9129): a project whose data are a
//! GeoPackage's tables is never said to have no source; each data layer's table is counted by its source, “hesaplanmadı”
//! until it answers; Öznitelikler says the data layers apart from the drawing's objects; two layers of the same name
//! from two sources are told apart in Katman ▾, a read-only one said so; the table says the layer drawn on when it
//! shows another.

use crate::app::{App, Message};
use crate::data_layers_tests::{node, parcels};
use crate::files_testing::{app_with_drawing, drive};

fn with_two_parcels(tag: &str) -> (App, String, String) {
    crate::data_layers::register();
    let (a, b) = (parcels(&format!("{tag}-a"), 4), parcels(&format!("{tag}-b"), 2));
    let mut app = app_with_drawing();
    app.add_data_layers(vec![
        crate::data_layers::describe(&a.display().to_string(), "parseller").expect("a"),
        crate::data_layers::describe(&b.display().to_string(), "parseller").expect("b"),
    ])
    .expect("added");
    let ids: Vec<String> = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .leaves()
        .iter()
        .filter(|n| n.data.is_some())
        .map(|n| n.id.clone())
        .collect();
    (app, ids[0].clone(), ids[1].clone())
}

#[test]
fn data_layers_are_listed_with_their_tables_counted_by_their_sources() {
    let (mut app, _, _) = with_two_parcels("kaynak-durumu");
    let task = app.update(Message::Run("source.status"));
    let w = app.gis.source_status.as_ref().expect("the window");
    let rows = app.source_data_rows();
    assert_eq!(rows.len(), 2);
    assert!(w.counts_for_test().is_empty(), "hesaplanmadı until the sources answer");
    drive(&mut app, task);
    let w = app.gis.source_status.as_ref().expect("the window");
    let mut counts: Vec<u64> = w.counts_for_test().values().filter_map(|r| r.as_ref().ok().copied()).collect();
    counts.sort_unstable();
    assert_eq!(counts, [2, 4], "each table's real count");
}

#[test]
fn the_inspector_says_the_data_layers_apart_from_the_drawings_objects() {
    let (app, _, _) = with_two_parcels("denetci");
    let doc = app.document.as_ref().expect("a drawing");
    let row = app
        .properties_panel(doc)
        .sections
        .into_iter()
        .filter(|s| s.title == "Çizim")
        .flat_map(|s| s.rows)
        .find(|r| r.label == "Veri katmanları")
        .expect("the row");
    assert!(row.value.starts_with("2 katman; satırları kaynaklarında"), "{}", row.value);
}

#[test]
fn katman_tells_same_named_layers_apart_and_the_table_names_the_target() {
    let (mut app, a, b) = with_two_parcels("ayni-ad");
    let names = app.table_layer_words_for_test();
    assert!(!names.is_empty());
    let ours: Vec<&String> = names.iter().filter(|n| n.starts_with("parseller (veri")).collect();
    assert_eq!(ours.len(), 2, "{names:?}");
    assert_ne!(ours[0], ours[1], "told apart by their sources: {names:?}");
    assert!(ours.iter().all(|n| n.contains(".gpkg › parseller")), "{names:?}");
    // The table on the first, the second drawn on: both said.
    let doc = app.document.as_mut().expect("a drawing");
    doc.model.set_active_layer(&b);
    app.gis.features.layer = Some(a);
    assert!(app.table_target_words_for_test().is_some_and(|w| w.starts_with("Çizim hedefi:")));
    let _ = node(&app, "parseller");
}
