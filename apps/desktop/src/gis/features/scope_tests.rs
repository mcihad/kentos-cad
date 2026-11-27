//! The second round's fourth tour (docs/adr/9348) on the municipal `bakim` table, its `mahalle` given the list M1 Çınar,
//! M2 İstasyon, M3 Kale:
//!
//! - T11: a field added to the layer while its table is open is asked again at once; its alias heads the column.
//! - T12: Ara finds a listed field's rows by the label shown (“çınar”), not only by the code (`M1`).
//! - T13: İfade oluşturucu on the layer's filter reads the source's rows: values as “Çınar (M1)”, `'M1'` inserted.
//! - T14: a Katman süzgeci `mahalle = 'M1'` keeps the table to its two rows; Kaynağın tümü shows the six.
//! - T15: Parsel alan denetimi is named so on the ribbon.

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::data_layers_tests::{answer_pages, municipal, node};
use crate::features::Event;
use crate::features::data::DataEvent;
use crate::files_testing::{app_with_drawing, drive};
use crate::gis;

fn ev(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Features(e)));
    drive(app, task);
}

/// bakim's table open, its mahalle listed.
fn opened(tag: &str) -> (App, String, std::path::PathBuf) {
    crate::data_layers::register();
    let path = municipal(tag);
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&path.display().to_string(), "bakim").expect("described")])
        .expect("added");
    let bakim = node(&app, "bakim").expect("bakim").id.clone();
    {
        let doc = app.document.as_mut().expect("open");
        let mut fields = doc.model.layers().get(&bakim).expect("the layer").fields.clone();
        let mahalle = fields.iter_mut().find(|f| f.name == "mahalle").expect("mahalle");
        mahalle.values = Some(vec![
            kentos_commands::FieldChoice::new("M1", "Çınar"),
            kentos_commands::FieldChoice::new("M2", "İstasyon"),
            kentos_commands::FieldChoice::new("M3", "Kale"),
        ]);
        doc.model.set_layer_fields(&bakim, fields, &[]).expect("set");
    }
    app.command_expanded = true;
    app.bottom_tab = BottomTab::Table;
    ev(&mut app, Event::Layer(bakim.clone()));
    let _ = app.data_table_event(DataEvent::Range(0, 12));
    answer_pages(&mut app);
    (app, bakim, path)
}

#[test]
fn a_field_added_is_asked_again_and_ara_finds_a_label() {
    let (mut app, bakim, path) = opened("kapsam-alan");
    assert_eq!(app.gis.features.data.total_rows(), Some(6));
    // T11: a field (with an alias) added to the layer: the table asks again without leaving the layer.
    {
        let doc = app.document.as_mut().expect("open");
        let mut fields = doc.model.layers().get(&bakim).expect("the layer").fields.clone();
        let mut added = kentos_commands::LayerField::new("qa_turu", kentos_commands::LayerFieldKind::Text);
        added.alias = Some("Saha türü".into());
        fields.push(added);
        doc.model.set_layer_fields(&bakim, fields, &[]).expect("set");
    }
    let _ = app.data_table_tasks();
    let asked = app.gis.features.data.pending();
    assert!(!asked.is_empty(), "asked again at once");
    for (generation, page, input) in asked {
        let node = app.data_table_layer().expect("bakim");
        let settings = app.document.as_ref().expect("open").model.settings().clone();
        let answer = kentos_native_application::layers_rows::read(&node, &settings, &input, &|| false)
            .map(Box::new)
            .map_err(|e| e.message);
        let _ = app.data_table_event(DataEvent::Answered(generation, page, answer));
    }
    answer_pages(&mut app);
    assert_eq!(app.gis.features.data.total_rows(), Some(6), "read again, the rows kept: {:?}", app.gis.features.data.error());
    // T12: Ara by the label shown, folded the Turkish way, and by the code.
    ev(&mut app, Event::Search("çınar".into()));
    ev(&mut app, Event::Data(DataEvent::Apply));
    answer_pages(&mut app);
    assert_eq!(app.gis.features.data.total_rows(), Some(2), "M1 rows by their label: {:?}", app.gis.features.data.error());
    ev(&mut app, Event::Search("M2".into()));
    ev(&mut app, Event::Data(DataEvent::Apply));
    answer_pages(&mut app);
    assert_eq!(app.gis.features.data.total_rows(), Some(2), "by the code too");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_layer_filter_keeps_the_table_to_its_rows_and_kaynagin_tumu_shows_all() {
    let (mut app, bakim, path) = opened("kapsam-suzgec");
    {
        let doc = app.document.as_mut().expect("open");
        let filter = kentos_commands::LayerFilter { expression: Some("mahalle = 'M1'".into()), ..Default::default() };
        doc.model.set_layer_filter(&bakim, Some(filter), "Katman süzgeci").expect("set");
    }
    answer_pages(&mut app);
    assert_eq!(app.gis.features.data.total_rows(), Some(2), "the filtered layer by default");
    ev(&mut app, Event::SourceAll(true));
    answer_pages(&mut app);
    assert_eq!(app.gis.features.data.total_rows(), Some(6), "Kaynağın tümü");
    ev(&mut app, Event::SourceAll(false));
    answer_pages(&mut app);
    assert_eq!(app.gis.features.data.total_rows(), Some(2));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_builder_reads_the_sources_values_with_their_labels() {
    let (mut app, bakim, path) = opened("kapsam-ifade");
    {
        let doc = app.document.as_mut().expect("open");
        let filter = kentos_commands::LayerFilter { expression: Some("mahalle = 'M1'".into()), ..Default::default() };
        doc.model.set_layer_filter(&bakim, Some(filter), "Katman süzgeci").expect("set");
    }
    let task = app.open_layer_filter(Some(bakim.clone()));
    drive(&mut app, task);
    let task = app.open_builder_for_layer_filter();
    drive(&mut app, task);
    let task = app.data_rows_tasks();
    drive(&mut app, task);
    let b = app.gis.builder.as_ref().expect("the builder");
    assert_eq!(b.objects.len(), 6, "the source's rows");
    assert!(matches!(b.preview, crate::expression::Preview::Value(_)), "a value on the first row");
    let task = app.update(Message::Gis(gis::Event::Builder(crate::expression::Event::Row("field:mahalle".into()))));
    drive(&mut app, task);
    let task = app.update(Message::Gis(gis::Event::Builder(crate::expression::Event::Values(true))));
    drive(&mut app, task);
    let listed = app.gis.builder.as_ref().and_then(|b| b.listed.as_ref()).expect("the values");
    let texts: Vec<(&str, &str)> = listed.items.iter().map(|v| (v.text.as_str(), v.insert.as_str())).collect();
    assert!(texts.contains(&("Çınar (M1)", "'M1'")), "{texts:?}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn parsel_alan_denetimi_is_named_so() {
    let c = crate::catalog::catalog().get("parcels.check").expect("the command");
    assert_eq!(c.short, "Parsel alan denetimi");
}

/// The pictures (`.run/gui-evidence/kullanim-9348/`): the table on bakim with its Katman süzgeci (Kapsam: Süzülen
/// katman, the alias in the head); İfade oluşturucu's values with their labels.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib scope_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    let prepare = |builder: bool| {
        let (mut app, bakim, _) = opened(if builder { "kapsam-resim-ifade" } else { "kapsam-resim" });
        {
            let doc = app.document.as_mut().expect("open");
            let mut fields = doc.model.layers().get(&bakim).expect("the layer").fields.clone();
            if let Some(f) = fields.iter_mut().find(|f| f.name == "mahalle") {
                f.alias = Some("Mahalle kodu".into());
            }
            doc.model.set_layer_fields(&bakim, fields, &[]).expect("set");
            let filter = kentos_commands::LayerFilter { expression: Some("mahalle = 'M1'".into()), ..Default::default() };
            doc.model.set_layer_filter(&bakim, Some(filter), "Katman süzgeci").expect("set");
        }
        let _ = app.update(Message::BottomResized(240.0));
        answer_pages(&mut app);
        if builder {
            let task = app.open_layer_filter(Some(bakim.clone()));
            drive(&mut app, task);
            let task = app.open_builder_for_layer_filter();
            drive(&mut app, task);
            let task = app.data_rows_tasks();
            drive(&mut app, task);
            for e in [crate::expression::Event::Row("field:mahalle".into()), crate::expression::Event::Values(true)] {
                let task = app.update(Message::Gis(gis::Event::Builder(e)));
                drive(&mut app, task);
            }
        }
        app
    };
    crate::files_testing::usage_shots("9348", "01-tablo-kapsami", || prepare(false));
    crate::files_testing::usage_shots("9348", "02-ifade-degerleri", || prepare(true));
}
