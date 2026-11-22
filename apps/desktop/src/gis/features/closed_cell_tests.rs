//! The data table on a layer whose edit session is closed (docs/adr/9142), the municipal report's 9th item: a cell's
//! double click says why in a notice read whole and offers Düzenlemeyi başlat, which opens the session; then the cell is
//! edited. The table's line says the rows read so far beside the table's count while they are fewer.

use crate::features::data::DataEvent;
use crate::bottom::BottomTab;
use crate::data_layers_tests::{answer_pages, municipal, node, parcels};
use crate::files_testing::{app_with_drawing, drive};

#[test]
fn a_closed_layers_cell_offers_its_session_and_then_is_edited() {
    crate::data_layers::register();
    let path = municipal("kapali-hucre");
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&path.display().to_string(), "bakim").expect("described")])
        .expect("added");
    let layer = node(&app, "bakim").expect("bakim").id.clone();
    app.gis.features.layer = Some(layer.clone());
    app.command_expanded = true;
    app.bottom_tab = BottomTab::Table;
    let _ = app.data_table_event(DataEvent::Range(0, 12));
    answer_pages(&mut app);
    // The session closed: no cell edited, a notice with the way to open it.
    let _ = app.data_table_event(DataEvent::Edit(1, "kod".into()));
    assert!(app.gis.features.data.cell_editing().is_none());
    let offered = app.toasts.iter().find_map(|(_, t, _)| t.action_of().filter(|(label, _)| *label == "Düzenlemeyi başlat").map(|(_, m)| m.clone()));
    let start = offered.expect("Düzenlemeyi başlat offered");
    // Pressed: the session open, the same double click edits the cell.
    let task = app.update(start);
    drive(&mut app, task);
    assert!(app.document.as_ref().expect("open").model.editing(&layer), "the session is open");
    let _ = app.data_table_event(DataEvent::Edit(1, "kod".into()));
    assert_eq!(app.gis.features.data.cell_editing().map(|(k, f, _)| (k, f)), Some((1, "kod".to_owned())));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_tables_line_says_the_rows_read_beside_its_count() {
    crate::data_layers::register();
    let path = parcels("yuklenen", 450);
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&path.display().to_string(), "parseller").expect("described")])
        .expect("added");
    let layer = node(&app, "parseller").expect("the layer").id.clone();
    app.gis.features.layer = Some(layer);
    app.command_expanded = true;
    app.bottom_tab = BottomTab::Table;
    let _ = app.data_table_event(DataEvent::Range(0, 12));
    answer_pages(&mut app);
    assert_eq!(app.gis.features.data.total_rows(), Some(450));
    assert_eq!(app.data_table_line().as_deref(), Some("450 satır (200 yüklendi) · süzgeç ve sıra kaynakta"));
    let _ = std::fs::remove_file(&path);
}
