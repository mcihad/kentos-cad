//! Bilgi's card on the municipal report's 13th item (docs/adr/9145): the card names the object's layer and its key
//! under the title; a click over overlapping objects says how many are under it and goes from one to the next, round to
//! the first; a click on nothing closes the card and forgets them.

use crate::app::{App, Message};
use crate::files_testing::app_with_drawing;
use crate::tools_scenes::{E, N, click, run};

/// The sample drawing in a 1000 × 700 view fitted to it, with Bilgi on.
fn identifying() -> App {
    let mut app = app_with_drawing();
    let area = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(area)));
    let _ = app.update(Message::Run("view.zoomExtents"));
    run(&mut app, "tool.identify");
    app
}

/// Between the building's circle (r 3.25) and its arc (r 4) round the same centre, inside the parcel.
fn between() -> [f64; 2] {
    let r = 3.625_f64;
    [486_524.34 + r * 1f64.cos() - E, 4_420_199.52 + r * 1f64.sin() - N]
}

fn shown(app: &App) -> Option<kentos_domain::Slot> {
    app.info_card.map(|(s, _)| s)
}

#[test]
fn bilgis_card_names_the_layer_and_the_key_under_its_title() {
    let app = app_with_drawing();
    let doc = app.document.as_ref().expect("open");
    let slot = doc.model.by_layer("parsel").next().map(|e| kentos_domain::Slot(e.base().id)).expect("a parcel");
    let uid = doc.model.uid(slot).expect("a uid").to_string();
    let card = app.slot_card(slot).expect("its card");
    assert_eq!(card.source.map(|(l, _)| l), Some(format!("Parsel katmanı · kimlik …{}", &uid[uid.len() - 8..])));
}

#[test]
fn a_click_over_overlapping_objects_counts_them_and_goes_to_the_next() {
    let mut app = identifying();
    click(&mut app, between());
    let hits = app.gis.data_info.hits.clone();
    assert!(hits.len() >= 2, "the circle and the arc under the click: {hits:?}");
    assert_eq!(shown(&app), Some(hits[0]), "the card is the click's first");
    let doc = app.document.as_ref().expect("open");
    let layers: Vec<String> = hits.iter().filter_map(|s| doc.model.get(*s)).map(|e| e.base().layer_id.clone()).collect();
    assert!(layers.iter().filter(|l| *l == "bina").count() >= 2, "{layers:?}");
    assert_eq!(crate::info_card::hits_words(1, hits.len()), format!("Bu noktada {0} nesne · 1/{0}", hits.len()));
    // Sonraki: the next, then round to the first.
    let _ = app.update(Message::InfoCard(crate::info_card::Event::Next));
    assert_eq!(shown(&app), Some(hits[1]));
    for _ in 1..hits.len() {
        let _ = app.update(Message::InfoCard(crate::info_card::Event::Next));
    }
    assert_eq!(shown(&app), Some(hits[0]));
    // A click on nothing closes the card and forgets the hits.
    click(&mut app, [0.0, 0.0]);
    assert_eq!(shown(&app), None);
    assert!(app.gis.data_info.hits.is_empty());
}

/// Bilgi's card over the circle and the arc (`.run/gui-evidence/kullanim-9145/`): the layer and key line, how many
/// objects are under the click and Sonraki.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9145", "03-bilgi-ortusen", || {
        let mut app = identifying();
        click(&mut app, between());
        app
    });
}
