//! A data layer's table edits a listed field through the drawing's table's list editor (the CBS review's 4th item,
//! docs/adr/9323): the code chosen is written, the label shown; a value its field refuses keeps the cell open with
//! what was typed and the reason beside it.

use crate::bottom::BottomTab;
use crate::data_layers_tests::{answer_pages, municipal, node};
use crate::features::data::DataEvent;
use crate::files_testing::{app_with_drawing, drive};

fn send(app: &mut crate::app::App, e: DataEvent) {
    let task = app.data_table_event(e);
    drive(app, task);
}

#[test]
fn a_listed_fields_cell_writes_the_code_chosen_and_a_refused_value_stays_with_its_reason() {
    crate::data_layers::register();
    let path = municipal("tipli-hucre");
    let mut app = app_with_drawing();
    app.add_data_layers(vec![
        crate::data_layers::describe(&path.display().to_string(), "bakim").expect("described"),
    ])
    .expect("added");
    let layer = node(&app, "bakim").expect("bakim").id.clone();
    // mahalle given the neighbourhoods' list: a project rule of the layer's field.
    {
        let doc = app.document.as_mut().expect("open");
        let mut fields = doc
            .model
            .layers()
            .get(&layer)
            .expect("the layer")
            .fields
            .clone();
        let mahalle = fields
            .iter_mut()
            .find(|f| f.name == "mahalle")
            .expect("mahalle");
        mahalle.values = Some(vec![
            kentos_commands::FieldChoice::new("M1", "Çınar"),
            kentos_commands::FieldChoice::new("M2", "İstasyon"),
            kentos_commands::FieldChoice::new("M3", "Kale"),
        ]);
        doc.model
            .set_layer_fields(&layer, fields, &[])
            .expect("set");
    }
    let _ = app.data_edits_event(crate::data_edits::Event::Start(layer.clone()));
    app.gis.features.layer = Some(layer.clone());
    app.command_expanded = true;
    app.bottom_tab = BottomTab::Table;
    send(&mut app, DataEvent::Range(0, 12));
    answer_pages(&mut app);
    // A code chosen in the list: written to the row taken.
    send(&mut app, DataEvent::Edit(1, "mahalle".into()));
    send(&mut app, DataEvent::Choose("M3".into()));
    let doc = &app.document.as_ref().expect("open").model;
    let uid = doc.taken_uid(&layer, 1).expect("the row taken");
    let slot = doc.slot_of(uid).expect("its object");
    assert_eq!(
        doc.get(slot)
            .and_then(|e| e.base().attrs.get("mahalle").cloned())
            .as_deref(),
        Some("M3")
    );
    assert!(
        app.gis.features.data.cell_editing().is_none(),
        "the cell closes"
    );
    // A number refused: the cell stays with what was typed and why.
    send(&mut app, DataEvent::Edit(2, "bedel".into()));
    send(&mut app, DataEvent::Input("yüz".into()));
    send(&mut app, DataEvent::Enter);
    assert_eq!(
        app.gis.features.data.cell_editing(),
        Some((2, "bedel".to_owned(), "yüz".to_owned()))
    );
    let why = app
        .gis
        .features
        .data
        .cell_error()
        .expect("the reason beside the cell");
    assert!(why.contains("bedel"), "{why}");
    // Put right: written, the reason gone.
    send(&mut app, DataEvent::Input("120".into()));
    assert!(app.gis.features.data.cell_error().is_none());
    send(&mut app, DataEvent::Enter);
    assert!(app.gis.features.data.cell_editing().is_none());
    let _ = std::fs::remove_file(&path);
}

/// The picture (`.run/gui-evidence/kullanim-9323/`): a listed field's cell open as its list in the data table.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib typed_cell_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    let prepared = |tag: &str, refused: bool| {
        crate::data_layers::register();
        let path = municipal(tag);
        let mut app = app_with_drawing();
        app.add_data_layers(vec![
            crate::data_layers::describe(&path.display().to_string(), "bakim").expect("described"),
        ])
        .expect("added");
        let layer = node(&app, "bakim").expect("bakim").id.clone();
        {
            let doc = app.document.as_mut().expect("open");
            let mut fields = doc
                .model
                .layers()
                .get(&layer)
                .expect("the layer")
                .fields
                .clone();
            let mahalle = fields
                .iter_mut()
                .find(|f| f.name == "mahalle")
                .expect("mahalle");
            mahalle.values = Some(vec![
                kentos_commands::FieldChoice::new("M1", "Çınar"),
                kentos_commands::FieldChoice::new("M2", "İstasyon"),
                kentos_commands::FieldChoice::new("M3", "Kale"),
            ]);
            doc.model
                .set_layer_fields(&layer, fields, &[])
                .expect("set");
        }
        let _ = app.data_edits_event(crate::data_edits::Event::Start(layer.clone()));
        app.gis.features.layer = Some(layer);
        app.command_expanded = true;
        app.bottom_tab = BottomTab::Table;
        let _ = app.update(crate::app::Message::BottomResized(300.0));
        send(&mut app, DataEvent::Range(0, 12));
        answer_pages(&mut app);
        if refused {
            send(&mut app, DataEvent::Edit(1, "bedel".into()));
            send(&mut app, DataEvent::Input("yüz".into()));
            send(&mut app, DataEvent::Enter);
        } else {
            send(&mut app, DataEvent::Edit(1, "mahalle".into()));
        }
        app
    };
    crate::files_testing::usage_shots("9323", "01-listeli-hucre", || {
        prepared("resim-liste", false)
    });
    crate::files_testing::usage_shots("9323", "02-reddedilen-deger", || {
        prepared("resim-red", true)
    });
}
