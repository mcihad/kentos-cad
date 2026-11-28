//! Alanlar's layout (the owner's word of 27 November 2026, docs/adr/9349): the layer's fields listed on the left, the
//! chosen field's settings on the right in one form (Genel, Değerler, Nesne işlemleri, Kaynak, Kullanıldığı yerler);
//! the list's own buttons under it, the less used ones under Diğer ▾. The municipal `bakim` table as a data layer.

use super::{Cell, Event, msg};
use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, drive};

/// bakim's Alanlar, mahalle listed and aliased, `field` chosen.
fn opened(tag: &str, field: usize) -> App {
    crate::data_layers::register();
    let path = crate::data_layers_tests::municipal(tag);
    let mut app = app_with_drawing();
    app.add_data_layers(vec![
        crate::data_layers::describe(&path.display().to_string(), "bakim").expect("described"),
    ])
    .expect("added");
    let bakim = crate::data_layers_tests::node(&app, "bakim")
        .expect("bakim")
        .id
        .clone();
    {
        let doc = app.document.as_mut().expect("open");
        let mut fields = doc
            .model
            .layers()
            .get(&bakim)
            .expect("the layer")
            .fields
            .clone();
        if let Some(f) = fields.iter_mut().find(|f| f.name == "mahalle") {
            f.alias = Some("Mahalle".into());
            f.required = true;
            f.values = Some(vec![
                kentos_commands::FieldChoice::new("M1", "Çınar"),
                kentos_commands::FieldChoice::new("M2", "İstasyon"),
                kentos_commands::FieldChoice::new("M3", "Kale"),
            ]);
        }
        if let Some(f) = fields.iter_mut().find(|f| f.name == "kod") {
            f.alias = Some("Bakım kodu".into());
        }
        doc.model
            .set_layer_fields(&bakim, fields, &[])
            .expect("set");
    }
    let task = app.open_layer_fields(&bakim);
    drive(&mut app, task);
    let _ = app.update(msg(Event::Choose(field)));
    app
}

#[test]
fn the_chosen_fields_settings_are_its_own_and_editing_them_changes_it() {
    let mut app = opened("alanlar-duzen", 2);
    let w = app.gis.layer_fields.as_ref().expect("Alanlar is open");
    assert_eq!(w.chosen, Some(2));
    // The chosen field's alias typed in the form: that field's.
    let _ = app.update(msg(Event::Type(2, Cell::Alias, "Mahalle kodu".into())));
    let _ = app.update(msg(Event::Choose(0)));
    let _ = app.update(msg(Event::Save));
    let doc = &app.document.as_ref().expect("open").model;
    let bakim = crate::data_layers_tests::node(&app, "bakim")
        .expect("bakim")
        .id
        .clone();
    let fields = &doc.layers().get(&bakim).expect("the layer").fields;
    let aliases: Vec<(&str, Option<&str>)> = fields
        .iter()
        .map(|f| (f.name.as_str(), f.alias.as_deref()))
        .collect();
    assert_eq!(aliases[2], ("mahalle", Some("Mahalle kodu")));
    assert_eq!(aliases[0], ("kod", Some("Bakım kodu")));
}

/// The pictures (`.run/gui-evidence/kullanim-9349/`): mahalle chosen; its value list open.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib layer_fields_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9349", "01-alanlar", || opened("alanlar-resim", 2));
    crate::files_testing::usage_shots("9349", "02-deger-listesi", || {
        let mut app = opened("alanlar-resim-liste", 2);
        let _ = app.update(msg(Event::OpenList(2)));
        app
    });
    crate::files_testing::usage_shots("9349", "03-yeni-alan", || {
        let mut app = opened("alanlar-resim-yeni", 0);
        let _ = app.update(msg(Event::Add));
        app
    });
    let _ = Message::DialogClosed;
}
