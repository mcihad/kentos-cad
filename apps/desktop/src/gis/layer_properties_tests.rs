//! Katman özellikleri (docs/adr/9350) on the municipal `bakim` table as a CBS data layer: the menu's last entry opens
//! the window on Genel; Kaynak names the GeoPackage, the table, its key and geometry, and the rows the source counts;
//! Koordinat sistemi the table's system beside the project's; Alanlar and Formlar list what the layer has.

use super::{Event, Section, msg};
use crate::app::{App, Dialog, Message};
use crate::files_testing::{app_with_drawing, drive};

fn opened(tag: &str) -> (App, String) {
    crate::data_layers::register();
    let path = crate::data_layers_tests::municipal(tag);
    let mut app = app_with_drawing();
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        let mut settings = model.settings().clone();
        settings.workspace = Some(kentos_commands::Workspace::Gis);
        model.set_settings(settings);
    }
    app.add_data_layers(vec![crate::data_layers::describe(&path.display().to_string(), "bakim").expect("described")])
        .expect("added");
    let bakim = crate::data_layers_tests::node(&app, "bakim").expect("bakim").id.clone();
    {
        let doc = app.document.as_mut().expect("open");
        let mut fields = doc.model.layers().get(&bakim).expect("the layer").fields.clone();
        if let Some(f) = fields.iter_mut().find(|f| f.name == "mahalle") {
            f.alias = Some("Mahalle".into());
            f.required = true;
            f.values = Some(vec![
                kentos_commands::FieldChoice::new("M1", "Çınar"),
                kentos_commands::FieldChoice::new("M2", "İstasyon"),
            ]);
        }
        doc.model.set_layer_fields(&bakim, fields, &[]).expect("set");
    }
    (app, bakim)
}

#[test]
fn the_menus_last_entry_opens_the_layers_summary() {
    let (mut app, bakim) = opened("ozellikler");
    // The menu entry is placed by the CBS profile: kentos-cad's layer_menu_tests read it in every module and kind.
    let task = app.update(super::open(bakim.clone()));
    drive(&mut app, task);
    assert_eq!(app.dialog, Some(Dialog::Gis(crate::gis::GisDialogKind::LayerProperties)));
    assert_eq!(app.gis.layer_properties.as_ref().map(|w| w.section), Some(Section::General));
    // The source's rows counted for Kaynak.
    let _ = app.update(msg(Event::Show(Section::Source)));
    // Asked as Kaynak asks it while drawn, then counted.
    let _ = app.source_count_of(&bakim);
    let task = app.source_counts_tasks();
    drive(&mut app, task);
    assert_eq!(app.source_count_of(&bakim), crate::source_counts::Counted::Ready(6));
    let _ = app.update(msg(Event::Close));
    assert!(app.dialog.is_none() && app.gis.layer_properties.is_none());
}

/// The pictures (`.run/gui-evidence/kullanim-9350/`): Genel, Kaynak and Alanlar.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib layer_properties_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    for (n, section, name) in [
        ("01", Section::Source, "kaynak"),
        ("02", Section::Fields, "alanlar"),
        ("03", Section::Crs, "koordinat-sistemi"),
    ] {
        crate::files_testing::usage_shots("9350", &format!("{n}-{name}"), || {
            let (mut app, bakim) = opened(&format!("ozellikler-resim-{name}"));
            let task = app.update(super::open(bakim.clone()));
            drive(&mut app, task);
            let _ = app.update(msg(Event::Show(section)));
            let _ = app.source_count_of(&bakim);
            let task = app.source_counts_tasks();
            drive(&mut app, task);
            app
        });
    }
    let _ = Message::DialogClosed;
}
