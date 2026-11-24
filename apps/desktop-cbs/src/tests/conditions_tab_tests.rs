//! features › conditions › conditions_tests's tests that need the CBS module's surfaces (its project's bottom tabs and panels; docs/adr/9199
//! §4.14): moved from the core's gis/features/conditions_tests.rs with the module's package; the work they test is the core's.

use crate::features::Event;
use crate::app::{App, Message};
use crate::gis;

fn ev(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Features(e)));
    crate::files_testing::drive(app, task);
}

/// The table of the drawing's own layers: `fixtures/interaction/v1/feature-table.kcad`'s Parsel (Ada, Parsel, …) and
/// Yapı (its objects' Kat, Yapı and Not).
fn app_with_table() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!(
        "../../../../fixtures/interaction/v1/feature-table.kcad"
    ))
    .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.gis.features.reset(None);
    let _ = app.update(Message::Run("data.featureTable"));
    app
}

#[test]
fn each_drawing_layer_keeps_its_own_condition_and_order_and_a_field_it_lacks_is_said() {
    let mut app = app_with_table();
    ev(&mut app, Event::Layer("parsel".into()));
    ev(&mut app, Event::Filter("Ada = '101'".into()));
    ev(&mut app, Event::Sort(3));
    let (count, _, rows) = app.features_seen().expect("the tab is open");
    assert_eq!(count, "2 / 5");
    let parcels = rows.clone();
    // Yapı: none of Parsel's condition or order.
    ev(&mut app, Event::Layer("yapi".into()));
    assert_eq!((app.gis.features.filter.as_str(), app.gis.features.sort.as_deref()), ("", None));
    let (count, _, _) = app.features_seen().expect("the tab is open");
    assert_eq!(count, "2 / 2");
    // A condition naming a field Yapı's objects lack is said.
    ev(&mut app, Event::Filter("Ada = '101'".into()));
    let doc = app.document.as_ref().expect("a drawing");
    let why = app.feature_rows(doc).filter_error.clone().expect("said");
    assert!(why.contains("“Ada” alanı “Yapı” katmanında yok"), "{why}");
    assert!(why.contains("Kat") && why.contains("Not"), "{why}");
    // A field one object has is the layer's: no word, the condition asked.
    ev(&mut app, Event::Filter("[Not] = 'köşe'".into()));
    let doc = app.document.as_ref().expect("a drawing");
    let rows = app.feature_rows(doc);
    assert_eq!((rows.filter_error.as_deref(), rows.shown.len()), (None, 1));
    // Back to Parsel: its condition and order, the same rows.
    ev(&mut app, Event::Layer("parsel".into()));
    assert_eq!(app.gis.features.filter, "Ada = '101'");
    let (count, _, rows) = app.features_seen().expect("the tab is open");
    assert_eq!((count.as_str(), rows), ("2 / 5", parcels));
    // And Yapı's own when it comes again.
    ev(&mut app, Event::Layer("yapi".into()));
    assert_eq!(app.gis.features.filter, "[Not] = 'köşe'");
}
