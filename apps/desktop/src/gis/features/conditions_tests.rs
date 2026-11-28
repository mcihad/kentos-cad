//! The table's conditions by layer (docs/adr/9132), the municipal report's case: `bedel >= 100` on `bakim` keeps B02,
//! B03 and B06; going to `mahalleler` asks nothing of it and shows M1, M2 and M3; coming back brings the condition and
//! its three rows. A condition naming a field the layer lacks says so instead of “no row”, on a data layer and on a
//! layer of the drawing's own.

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::data_layers_tests::{answer_pages, municipal, node};
use crate::features::Event;
use crate::features::data::DataEvent;
use crate::files_testing::app_with_drawing;
use crate::gis;

fn ev(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Features(e)));
    crate::files_testing::drive(app, task);
}

/// The keys of the rows the table read, in its order.
fn keys(app: &App) -> Vec<i64> {
    let data = &app.gis.features.data;
    (0..data.total_rows().unwrap_or(0) as usize)
        .filter_map(|i| data.row(i).map(|r| r.key))
        .collect()
}

fn values(app: &App, field: &str) -> Vec<String> {
    let data = &app.gis.features.data;
    (0..data.total_rows().unwrap_or(0) as usize)
        .filter_map(|i| data.row(i).and_then(|r| r.attrs.get(field).cloned()))
        .collect()
}

#[test]
fn each_data_layer_keeps_its_own_condition_and_a_field_it_lacks_is_said() {
    crate::data_layers::register();
    let path = municipal("kosul-katman");
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![
        crate::data_layers::describe(&address, "bakim").expect("described"),
        crate::data_layers::describe(&address, "mahalleler").expect("described"),
    ])
    .expect("added");
    let bakim = node(&app, "bakim").expect("bakim").id.clone();
    let mahalleler = node(&app, "mahalleler").expect("mahalleler").id.clone();
    app.command_expanded = true;
    app.bottom_tab = BottomTab::Table;
    ev(&mut app, Event::Layer(bakim.clone()));
    let _ = app.data_table_event(DataEvent::Range(0, 12));
    answer_pages(&mut app);
    assert_eq!(app.gis.features.data.total_rows(), Some(6));
    // bedel >= 100, asked with Enter: B02, B03 and B06.
    ev(&mut app, Event::Filter("bedel >= 100".into()));
    ev(&mut app, Event::Data(DataEvent::Apply));
    answer_pages(&mut app);
    assert_eq!(keys(&app), [2, 3, 6]);
    // Mahalleler: the condition stays with bakim; its three rows.
    ev(&mut app, Event::Layer(mahalleler.clone()));
    assert_eq!(
        app.gis.features.filter, "",
        "bakim's condition is not mahalleler's"
    );
    answer_pages(&mut app);
    assert_eq!(app.gis.features.data.error(), None);
    assert_eq!(values(&app, "ad"), ["Çınar", "İstasyon", "Kale"]);
    // Back to bakim: the reader's condition and its three rows.
    ev(&mut app, Event::Layer(bakim.clone()));
    assert_eq!(app.gis.features.filter, "bedel >= 100");
    answer_pages(&mut app);
    assert_eq!(keys(&app), [2, 3, 6]);
    // Written on mahalleler, the condition names a field it lacks: said, not answered as no row.
    ev(&mut app, Event::Layer(mahalleler.clone()));
    ev(&mut app, Event::Filter("bedel >= 100".into()));
    ev(&mut app, Event::Data(DataEvent::Apply));
    answer_pages(&mut app);
    let why = app.gis.features.data.error().expect("refused").to_owned();
    assert!(
        why.contains("“bedel” alanı “mahalleler” katmanında yok"),
        "{why}"
    );
    assert!(why.contains("Katmanın alanları: mahalle, ad."), "{why}");
    assert_eq!(
        app.gis.features.data.total_rows(),
        None,
        "no count of rows a refused question has not got"
    );
    // Each keeps its own still: bakim's three rows come back with it.
    ev(&mut app, Event::Layer(bakim));
    answer_pages(&mut app);
    assert_eq!(keys(&app), [2, 3, 6]);
    assert_eq!(app.gis.features.data.error(), None);
    let _ = std::fs::remove_file(&path);
}

/// Pictures (docs/adr/9132), light at 1440×900 and dark at 1100×760: Mahalleler's three rows after Bakım's condition,
/// Bakım's condition and its three rows back, and the condition written on Mahalleler said.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib conditions_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::data_layers::register();
    let path = municipal("kosul-resim");
    let prepare = |step: usize| {
        let path = path.clone();
        move || {
            let address = path.display().to_string();
            let mut app = app_with_drawing();
            app.add_data_layers(vec![
                crate::data_layers::describe(&address, "bakim").expect("described"),
                crate::data_layers::describe(&address, "mahalleler").expect("described"),
            ])
            .expect("added");
            let bakim = node(&app, "bakim").expect("bakim").id.clone();
            let mahalleler = node(&app, "mahalleler").expect("mahalleler").id.clone();
            app.command_expanded = true;
            app.bottom_tab = BottomTab::Table;
            ev(&mut app, Event::Layer(bakim.clone()));
            let _ = app.data_table_event(DataEvent::Range(0, 12));
            ev(&mut app, Event::Filter("bedel >= 100".into()));
            ev(&mut app, Event::Data(DataEvent::Apply));
            answer_pages(&mut app);
            ev(&mut app, Event::Layer(mahalleler.clone()));
            answer_pages(&mut app);
            if step >= 2 {
                ev(&mut app, Event::Layer(bakim));
                answer_pages(&mut app);
            }
            if step >= 3 {
                ev(&mut app, Event::Layer(mahalleler));
                ev(&mut app, Event::Filter("bedel >= 100".into()));
                ev(&mut app, Event::Data(DataEvent::Apply));
                answer_pages(&mut app);
            }
            app
        }
    };
    crate::files_testing::usage_shots("9132", "01-mahalleler-kosulsuz", prepare(1));
    crate::files_testing::usage_shots("9132", "02-bakim-kosulu-geri", prepare(2));
    crate::files_testing::usage_shots("9132", "03-alan-yok", prepare(3));
}
