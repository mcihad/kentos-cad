//! One record on every surface (the CBS review's 5th item, docs/adr/9326): a row chosen in a data layer's table is the
//! record Öznitelikler and Koordinat listesi show, by its layer and its key, read from its source and not taken into
//! the drawing. Two tables of one GeoPackage each have a row of key 1: each surface shows the row of the table chosen.
//!
//! The second round's T5 (docs/adr/9346): a cell edited in the edit session shows its value in Öznitelikler as waiting;
//! written to the source, the row is read again (said so until it comes) and shows the source's new value.

use crate::app::App;
use crate::bottom::BottomTab;
use crate::data_layers_tests::{answer_pages, municipal, node};
use crate::features::data::DataEvent;
use crate::files_testing::{app_with_drawing, drive};

/// The table on `layer` with its first page read, and its row `key` chosen; the chosen row read for the map.
fn choose(app: &mut App, layer: &str, key: i64) {
    app.gis.features.layer = Some(layer.to_owned());
    app.command_expanded = true;
    app.bottom_tab = BottomTab::Table;
    let task = app.data_table_event(DataEvent::Range(0, 12));
    drive(app, task);
    answer_pages(app);
    let task = app.data_table_event(DataEvent::Press(key, false));
    drive(app, task);
    let task = app.data_selection_tasks();
    drive(app, task);
}

fn line(app: &App, label: &str) -> Option<String> {
    app.source_row_lines()?.1.into_iter().find(|(l, _)| l == label).map(|(_, v)| v)
}

#[test]
fn the_tables_row_is_the_record_of_oznitelikler_and_the_coordinates_by_its_layer_and_key() {
    crate::data_layers::register();
    let path = municipal("tek-secim");
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    for table in ["bakim", "mahalleler"] {
        app.add_data_layers(vec![crate::data_layers::describe(&address, table).expect("described")]).expect("added");
    }
    let bakim = node(&app, "bakim").expect("bakim").id.clone();
    let mahalleler = node(&app, "mahalleler").expect("mahalleler").id.clone();
    assert!(app.selection.is_empty(), "nothing of the drawing selected");
    // bakim's row 1: its values, its layer and its key; its point listed read only.
    choose(&mut app, &bakim, 1);
    let (title, _, note) = app.source_row_lines().expect("the row shown");
    assert_eq!(title, "Veri satırı (1)");
    assert!(note.contains("çizime alınmadı"), "{note}");
    assert_eq!((line(&app, "Katman").as_deref(), line(&app, "fid").as_deref()), (Some("bakim"), Some("1")));
    assert_eq!(line(&app, "kod").as_deref(), Some("B01"));
    app.bottom_tab = BottomTab::Coords;
    assert_eq!(app.coordinate_points_for_test(), [("fid 1 (kaynakta)".to_owned(), false)]);
    // mahalleler's row 1, the same key of another table: that table's record.
    choose(&mut app, &mahalleler, 1);
    assert_eq!((line(&app, "Katman").as_deref(), line(&app, "fid").as_deref()), (Some("mahalleler"), Some("1")));
    assert_eq!((line(&app, "mahalle").as_deref(), line(&app, "ad").as_deref()), (Some("M1"), Some("Çınar")));
    assert_eq!(line(&app, "kod"), None, "not bakim's");
    assert!(app.document.as_ref().is_some_and(|d| d.model.taken_objects(&mahalleler).is_empty()), "nothing taken");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn an_edited_row_shows_its_waiting_value_and_after_the_write_the_sources() {
    crate::data_layers::register();
    let path = municipal("tek-secim-yaz");
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "bakim").expect("described")]).expect("added");
    let bakim = node(&app, "bakim").expect("bakim").id.clone();
    let _ = app.data_edits_event(crate::data_edits::Event::Start(bakim.clone()));
    choose(&mut app, &bakim, 1);
    assert_eq!(line(&app, "kod").as_deref(), Some("B01"));
    // The cell edited: the table's value is Öznitelikler's, said as waiting.
    for e in [DataEvent::Edit(1, "kod".into()), DataEvent::Input("B01-yeni".into()), DataEvent::Enter] {
        let task = app.data_table_event(e);
        drive(&mut app, task);
    }
    assert_eq!(line(&app, "kod").as_deref(), Some("B01-yeni"));
    let (_, _, note) = app.source_row_lines().expect("the row shown");
    assert!(note.starts_with("Bekleyen değişiklik: kaynağa yazılmadı."), "{note}");
    // Kaynağa yaz: the row read again, said so until the source answers, then the source's value.
    let doc = &app.document.as_ref().expect("a drawing").model;
    let writes = kentos_native_application::data_write::writes_of(doc, &bakim).expect("being edited");
    let outcome = kentos_native_application::data_write::write_group(std::slice::from_ref(&writes), doc.settings(), false)
        .map(|keys| (vec![writes], keys));
    assert!(outcome.is_ok(), "written");
    let _ = app.data_edits_event(crate::data_edits::Event::Saved(vec![bakim.clone()], Box::new(outcome)));
    let read = app.data_selection_tasks();
    let (_, _, note) = app.source_row_lines().expect("the row shown");
    assert!(note.contains("yeniden okunuyor"), "{note}");
    drive(&mut app, read);
    let (_, _, note) = app.source_row_lines().expect("the row shown");
    assert_eq!(note, "Kaynaktan okundu; çizime alınmadı.");
    assert_eq!(line(&app, "kod").as_deref(), Some("B01-yeni"));
    let _ = std::fs::remove_file(&path);
}

/// The picture (`.run/gui-evidence/kullanim-9326/`): bakim's second row chosen in its table, its record in Öznitelikler
/// with its layer and key, read from the source.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib source_selection_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9326", "01-veri-satiri-oznitelikler", || {
        crate::data_layers::register();
        let path = municipal("resim-tek-secim");
        let address = path.display().to_string();
        let mut app = app_with_drawing();
        for table in ["bakim", "mahalleler"] {
            app.add_data_layers(vec![crate::data_layers::describe(&address, table).expect("described")]).expect("added");
        }
        let bakim = node(&app, "bakim").expect("bakim").id.clone();
        let _ = app.update(crate::app::Message::BottomResized(240.0));
        choose(&mut app, &bakim, 2);
        app
    });
}
