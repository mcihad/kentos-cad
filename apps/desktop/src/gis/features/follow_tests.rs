//! Tablo and the active layer (the second round's T34; docs/adr/9355), on the municipal `bakim` and `mahalleler` data
//! layers:
//!
//! - Tablo (the ribbon's) opens the active layer's table, also after Katman ▾ pinned the table to another layer.
//! - Pinned to a layer other than the active one, the bar says “Sabit” with Etkin katmana geç, which brings the table to
//!   the active layer and lets it follow.
//! - Göster is the layer's: Seçililer chosen on one layer is not carried to a layer come to for the first time, and
//!   comes back on its layer.
//! - T18 (docs/adr/9356): the asked İfade süzgeci is marked under the table and taken off there.

use crate::app::{App, Message};
use crate::data_layers_tests::{municipal, node};
use crate::features::{Event, Show};
use crate::files_testing::{app_with_drawing, drive};
use crate::gis;

fn ev(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Features(e)));
    drive(app, task);
}

fn run(app: &mut App, command: &'static str) {
    let task = app.update(Message::Run(command));
    drive(app, task);
}

/// The layer the table shows now.
fn table_layer(app: &App) -> Option<String> {
    let doc = app.document.as_ref()?;
    app.feature_layer(doc)
}

fn pinned(app: &App) -> Option<(String, String)> {
    let doc = app.document.as_ref()?;
    app.table_pinned(&app.feature_rows(doc))
}

/// The two tables as data layers, `bakim` the active one.
pub(super) fn opened(tag: &str) -> (App, String, String) {
    crate::data_layers::register();
    let path = municipal(tag).display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(
        ["bakim", "mahalleler"].into_iter().map(|t| crate::data_layers::describe(&path, t).expect("described")).collect(),
    )
    .expect("added");
    let bakim = node(&app, "bakim").expect("bakim").id.clone();
    let mahalleler = node(&app, "mahalleler").expect("mahalleler").id.clone();
    app.document.as_mut().expect("open").model.set_active_layer(&bakim);
    (app, bakim, mahalleler)
}

#[test]
fn tablo_opens_the_active_layer_and_seciler_stays_with_its_layer() {
    let (mut app, bakim, mahalleler) = opened("tablo-etkin");
    run(&mut app, "data.featureTable");
    assert_eq!(table_layer(&app).as_deref(), Some(bakim.as_str()));
    assert_eq!(pinned(&app), None, "the table follows the active layer");
    // Katman ▾ pins it to mahalleler: said, with the active layer named.
    ev(&mut app, Event::Layer(mahalleler.clone()));
    assert_eq!(pinned(&app), Some(("mahalleler".to_owned(), "bakim".to_owned())));
    // Seçililer on mahalleler: not carried to bakim, come to for the first time; back on mahalleler.
    ev(&mut app, Event::Show(Show::Selected));
    ev(&mut app, Event::Layer(bakim.clone()));
    assert_eq!(app.gis.features.show, Show::All, "bakim shows all its rows");
    ev(&mut app, Event::Layer(mahalleler.clone()));
    assert_eq!(app.gis.features.show, Show::Selected, "mahalleler's own choice comes back");
    // Etkin katmana geç: the table on bakim, following it.
    ev(&mut app, Event::FollowActive);
    assert_eq!(app.gis.features.layer, None);
    assert_eq!(table_layer(&app).as_deref(), Some(bakim.as_str()));
    assert_eq!(app.gis.features.show, Show::All);
    // Pinned again, then Tablo on the ribbon: the active layer's table.
    ev(&mut app, Event::Layer(mahalleler.clone()));
    run(&mut app, "data.featureTable");
    assert_eq!(table_layer(&app).as_deref(), Some(bakim.as_str()));
    assert_eq!(pinned(&app), None);
    // The active layer made mahalleler: the table follows it.
    app.document.as_mut().expect("open").model.set_active_layer(&mahalleler);
    assert_eq!(table_layer(&app).as_deref(), Some(mahalleler.as_str()));
}

/// The asked İfade süzgeci under the table (the second round's T18): marked beside the count, × takes it off.
#[test]
fn the_asked_filter_is_marked_under_the_table_and_taken_off_there() {
    let (mut app, bakim, _) = opened("tablo-suzgec");
    run(&mut app, "data.featureTable");
    assert_eq!(table_layer(&app).as_deref(), Some(bakim.as_str()));
    ev(&mut app, Event::Filter("fid > 2".into()));
    ev(&mut app, Event::Data(crate::features::data::DataEvent::Apply));
    assert_eq!(app.gis.features.data.asked_filter(), "fid > 2");
    ev(&mut app, Event::Data(crate::features::data::DataEvent::ClearFilter));
    assert_eq!(app.gis.features.data.asked_filter(), "");
    assert_eq!(app.gis.features.filter, "");
}

/// The picture of T18 (`.run/gui-evidence/kullanim-9356/`): the bar wrapped whole, the asked filter marked under the
/// table.
#[test]
#[ignore = "pictures"]
fn filter_picture() {
    crate::files_testing::usage_shots("9356", "02-tablo-suzgec", || {
        let (mut app, _, mahalleler) = opened("tablo-suzgec-resim");
        run(&mut app, "data.featureTable");
        ev(&mut app, Event::Layer(mahalleler.clone()));
        ev(&mut app, Event::Filter("mahalle = 'M2'".into()));
        ev(&mut app, Event::Data(crate::features::data::DataEvent::Apply));
        crate::data_layers_tests::answer_pages(&mut app);
        app
    });
}

/// The picture (`.run/gui-evidence/kullanim-9355/`): the table pinned to mahalleler while bakim is active.
#[test]
#[ignore = "pictures"]
fn pinned_picture() {
    crate::files_testing::usage_shots("9355", "03-tablo-sabit", || {
        let (mut app, _, mahalleler) = opened("tablo-sabit-resim");
        run(&mut app, "data.featureTable");
        ev(&mut app, Event::Layer(mahalleler.clone()));
        crate::data_layers_tests::answer_pages(&mut app);
        app
    });
}
