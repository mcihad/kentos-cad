//! Bilgi's card on the municipal report's 13th item (docs/adr/9145): the card names the object's layer and its key
//! under the title; a click over overlapping objects says how many are under it and goes from one to the next, round to
//! the first; a click on nothing closes the card and forgets them. The title names the record by its layer's display
//! field (docs/adr/9160).

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

/// `slot` named “Kent Kitaplığı” with its storeys.
fn name(app: &mut App, slot: kentos_domain::Slot) {
    let doc = app.document.as_mut().expect("open");
    let mut e = doc.model.get(slot).cloned().expect("it");
    e.base_mut().attrs = [("Kat".to_owned(), "3".to_owned()), ("Ad".to_owned(), "Kent Kitaplığı".to_owned())].into();
    assert!(doc.model.update(slot, e));
}

/// The first building of the sample drawing, named.
fn named_building(app: &mut App) -> kentos_domain::Slot {
    let doc = app.document.as_ref().expect("open");
    let slot = doc.model.by_layer("bina").next().map(|e| kentos_domain::Slot(e.base().id)).expect("a building");
    name(app, slot);
    slot
}

/// The 13th item's display field (docs/adr/9160): the record's name follows its kind in the card's title, from the
/// layer's field with a name in it; a label saying the same is not said twice; a field the role does not read never
/// names it.
#[test]
fn the_cards_title_names_the_record_by_its_display_field() {
    let mut app = app_with_drawing();
    let slot = named_building(&mut app);
    let card = app.slot_card(slot).expect("its card");
    assert!(card.title.ends_with(" · Kent Kitaplığı"), "{}", card.title);
    // P1's label is its name already.
    let doc = app.document.as_ref().expect("open");
    let p1 = doc
        .model
        .by_layer("cizim")
        .find(|e| e.base().label.as_deref() == Some("P1"))
        .map(|e| kentos_domain::Slot(e.base().id))
        .expect("P1");
    let title = app.slot_card(p1).expect("its card").title;
    assert!(title.ends_with("P1") && !title.contains(" · P1"), "{title}");
    // A field the role does not read never names it.
    let e = doc.model.get(slot).expect("it");
    assert_eq!(app.display_value(e, &["Ad".to_owned()]), None);
    assert_eq!(app.display_value(e, &[]), Some(("Ad".to_owned(), "Kent Kitaplığı".to_owned())));
}

/// A data row's card without a title of its own (docs/adr/9160): the layer's name and the row's, from the table's
/// field with a name in it (the neighbourhoods' “ad”).
#[test]
fn a_data_rows_card_is_titled_with_its_layers_name_and_its_own() {
    crate::data_layers::register();
    let address = crate::data_layers_tests::municipal("kart-gorunen-ad").display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "mahalleler").expect("described")])
        .expect("added");
    let id = crate::data_layers_tests::node(&app, "mahalleler").expect("the layer").id.clone();
    let doc = app.document.as_ref().expect("open");
    let mut object = doc.model.by_layer("cizim").next().cloned().expect("a point");
    object.base_mut().layer_id = id.clone();
    object.base_mut().label = None;
    object.base_mut().attrs = [("mahalle".to_owned(), "M1".to_owned()), ("ad".to_owned(), "Çınar".to_owned())].into();
    app.gis.data_info.card =
        Some(crate::data_info::Found { layer: id, key: 1, object, at: kentos_interaction::Vec2 { x: 0.0, y: 0.0 } });
    let (card, _, _) = app.data_card().expect("the row's card");
    assert_eq!(card.title, "mahalleler · Çınar");
}

/// The sample drawing with Bilgi on and the parcels' table bound as a data layer; a click inside the drawing's parcel
/// and the table's first row (x 486 500–486 525, y 4 420 200–4 420 225) with what lies beside it read.
fn clicked_over_both(tag: &str) -> App {
    crate::data_layers::register();
    let address = crate::data_layers_tests::parcels(tag, 40).display().to_string();
    let mut app = identifying();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
        .expect("added");
    click(&mut app, [486_520.0 - E, 4_420_210.0 - N]);
    // The click's work beside the object, as the update after it starts it (the helper drops its tasks).
    let at = app.info_card.map(|(_, at)| at).expect("the object's card");
    app.gis.data_info.beside.push((at, app.gis.data_info.reach));
    let task = app.data_info_tasks();
    crate::files_testing::drive(&mut app, task);
    app
}

/// The 13th item's last count (docs/adr/9167): a click over a drawing's object and a data layer's row counts them
/// together; Sonraki goes from the object to the row's card and round to the object.
#[test]
fn a_click_counts_the_objects_and_the_data_rows_under_it_together() {
    let mut app = clicked_over_both("bilgi-birlikte");
    let parcel = shown(&app).expect("the drawing's parcel first");
    assert_eq!(app.gis.data_info.hits, [parcel]);
    let rows: Vec<i64> = app.gis.data_info.rows.iter().map(|r| r.key).collect();
    assert_eq!(rows, [1], "the table's first row under the same click");
    assert_eq!(app.info_place(), Some((0, 2)));
    assert_eq!(crate::info_card::place_words(1, 1, 1), "Bu noktada 1 nesne, 1 veri satırı · 1/2");
    // Sonraki: the row's card.
    let _ = app.update(Message::InfoCard(crate::info_card::Event::Next));
    assert_eq!(shown(&app), None);
    assert_eq!(app.gis.data_info.card.as_ref().map(|f| f.key), Some(1));
    assert_eq!(app.info_place(), Some((1, 2)));
    let (card, _, _) = app.data_card().expect("the row's card");
    assert_eq!(card.title, "parseller");
    // Round to the object.
    let _ = app.update(Message::InfoCard(crate::info_card::Event::Next));
    assert_eq!(shown(&app), Some(parcel));
    assert!(app.gis.data_info.card.is_none());
    // Closed: nothing kept.
    let _ = app.update(Message::InfoCard(crate::info_card::Event::Close));
    assert!(app.gis.data_info.rows.is_empty() && app.gis.data_info.hits.is_empty());
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

/// Bilgi's card on a named building object (`.run/gui-evidence/kullanim-9160/`): its name after its kind in the title.
#[test]
#[ignore = "pictures for the owner"]
fn named_screens() {
    crate::files_testing::usage_shots("9160", "01-bilgi-gorunen-ad", || {
        let mut app = identifying();
        click(&mut app, between());
        let first = shown(&app).expect("the click's first");
        name(&mut app, first);
        app
    });
}

/// Bilgi kartı's window on the buildings (`.run/gui-evidence/kullanim-9160/`): under Başlık, which field an empty title
/// names the record by.
#[test]
#[ignore = "pictures for the owner"]
fn layer_info_screens() {
    crate::files_testing::usage_shots("9160", "02-kart-ayari-gorunen-alan", || {
        let mut app = app_with_drawing();
        named_building(&mut app);
        let _ = app.open_layer_info(Some("bina".to_owned()));
        app
    });
}

/// Bilgi's card over the drawing's parcel and the table's first row (`.run/gui-evidence/kullanim-9167/`): one object
/// and one data row counted together, then the row's card.
#[test]
#[ignore = "pictures for the owner"]
fn together_screens() {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    crate::files_testing::usage_shots("9167", "01-nesne-ve-satir", || {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        clicked_over_both(&format!("bilgi-birlikte-resim-{n}"))
    });
    crate::files_testing::usage_shots("9167", "02-satirin-karti", || {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut app = clicked_over_both(&format!("bilgi-birlikte-resim-{n}"));
        let _ = app.update(Message::InfoCard(crate::info_card::Event::Next));
        app
    });
}
