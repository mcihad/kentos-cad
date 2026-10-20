//! Adres (docs/adr/0229 §13) over `fixtures/interaction/v1/addresses.kcad`
//! (scripts/fixtures/address_scene.py): the tab finds an address in the
//! project's data on the address thread, a row brings the view and the mark,
//! Nokta ekle writes one step with the match's attributes and Taşı moves the
//! one point selected; Konumdan adres answers a place; the locator is built
//! again only when its layers change; Adres verisi checks and saves a
//! definition through `cad.addresses.define`; the sources list the project's
//! data before the device's providers. The traces address-search.json and
//! address-reverse.json play the same with the mouse and the keyboard.

use kentos_domain::Slot;
use kentos_interaction::Vec2;
use kentos_processing::address::AddressHost;

use super::data::Event as Data;
use super::providers::Event as Providers;
use super::{Event, Question, texts};
use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::files_testing::drive;

const DRAWING: &str = include_str!("../../../../fixtures/interaction/v1/addresses.kcad");
const CASES: &str = include_str!("../../../../fixtures/geocode/v1/cases.json");

/// The scene's origin (scripts/fixtures/geocode_cases.py's metres city).
const E: f64 = 500_000.0;
const N: f64 = 4_400_000.0;

fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// A message and the tasks it starts, to their end (the address thread's answers come back as messages).
fn send(app: &mut App, message: Message) {
    let task = app.update(message);
    drive(app, task);
}

fn ev(app: &mut App, e: Event) {
    send(app, Message::Address(e));
}

fn find(app: &mut App, text: &str) {
    ev(app, Event::Text(text.to_owned()));
    ev(app, Event::Find);
}

fn count(app: &App) -> usize {
    app.document.as_ref().expect("a drawing").model.len()
}

#[test]
fn an_address_is_found_on_the_address_thread_and_a_row_marks_it() {
    let mut app = app();
    send(&mut app, Message::Run("address.search"));
    assert_eq!(app.bottom_tab, BottomTab::Address);
    assert_eq!(app.address_seen(), Some((String::new(), Vec::new())));
    find(&mut app, "Kızılay Mah. Atatürk Blv. No:4 Çankaya/Ankara");
    let a = &app.address;
    assert_eq!(
        a.question,
        Some(Question::Address(
            "Kızılay Mah. Atatürk Blv. No:4 Çankaya/Ankara".into()
        ))
    );
    let best = a.rows.first().expect("a candidate");
    assert_eq!((best.kind.as_str(), best.score), ("Kapı", Some(100)));
    assert_eq!(best.fields[2], "4");
    let (line, rows) = app.address_seen().expect("the tab is open");
    assert!(
        line.starts_with(&format!("{} aday · En iyisi: Eşleşti (100)", rows.len())),
        "{line}"
    );
    // The door at (30, 105): the row's click brings the view and the mark.
    ev(&mut app, Event::Press(0));
    let (at, _) = app.address_mark_label().expect("marked");
    assert!(
        (at.x - (E + 30.0)).abs() < 1e-9 && (at.y - (N + 105.0)).abs() < 1e-9,
        "{at:?}"
    );
    // The panel closes when Adres ara runs on it again.
    send(&mut app, Message::Run("address.search"));
    assert!(!app.command_expanded);
}

#[test]
fn nokta_ekle_writes_one_step_and_tasi_moves_the_selected_point() {
    let mut app = app();
    send(&mut app, Message::Run("address.search"));
    let text = "Kızılay Mahallesi Atatürk Bulvarı No:6";
    find(&mut app, text);
    let before = count(&app);
    ev(&mut app, Event::AddPoint);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.len(), before + 1);
    let made = doc
        .by_layer("adres")
        .last()
        .expect("a point on the active layer");
    let attrs = &made.base().attrs;
    assert_eq!(attrs.get("Durum").map(String::as_str), Some("Eşleşti"));
    assert_eq!(
        attrs.get("Eşleşme türü").map(String::as_str),
        Some("Kapı (aradeğer)")
    );
    assert_eq!(attrs.get("Puan").map(String::as_str), Some("90"));
    assert_eq!(attrs.get("Kaynak").map(String::as_str), Some("Numarataj"));
    assert_eq!(attrs.get("Girdi adresi").map(String::as_str), Some(text));
    let slot = Slot(made.base().id);
    // Taşı: the point goes to another address, its attributes the new match's; one step.
    find(&mut app, "1000000002");
    app.selection.set([slot]);
    ev(&mut app, Event::MovePoint);
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let moved = doc.get(slot).expect("the point");
    let kentos_commands::Entity::Point(p) = moved else {
        panic!("a point");
    };
    assert_eq!((p.p.x, p.p.y), (E + 210.0, N + 105.0));
    assert_eq!(
        moved.base().attrs.get("Eşleşme türü").map(String::as_str),
        Some("Kod")
    );
    assert_eq!(doc.undo().as_deref(), Some(texts::MOVE));
    assert_eq!(doc.undo().as_deref(), Some(texts::ADD));
    assert_eq!(doc.len(), before);
}

#[test]
fn a_place_is_answered_and_is_not_written() {
    let mut app = app();
    let task = app.address_at(Vec2::new(E + 150.0, N + 250.0));
    drive(&mut app, task);
    assert_eq!(app.bottom_tab, BottomTab::Address);
    let row = app.address.rows.first().expect("an address");
    assert_eq!(row.kind, "Yol");
    assert_eq!(row.fields[1], "Gazi Mustafa Kemal Bulvarı");
    assert!((row.distance.expect("a distance") - 50.0).abs() < 1e-9);
    assert_eq!(
        app.address_seen().map(|s| s.0).as_deref(),
        Some(texts::PLACE)
    );
    ev(&mut app, Event::AddPoint);
    assert_eq!(
        app.address.said.as_ref().map(|s| s.1.as_str()),
        Some(texts::NOT_FOR_PLACE)
    );
}

#[test]
fn the_locator_is_built_again_only_when_its_layers_change() {
    let mut app = app();
    find(&mut app, "Atatürk Bulvarı No:2");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let def = doc.settings().address_locators[0].clone();
    assert!(app.address.fresh("adres-1", &def, doc));
    // A point on another layer: still fresh.
    ev(&mut app, Event::AddPoint);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert!(app.address.fresh("adres-1", &def, doc));
    // A door's change: built again.
    let door = doc
        .by_layer("kapi")
        .next()
        .map(|e| Slot(e.base().id))
        .expect("a door");
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let mut e = doc.get(door).cloned().expect("the door");
    e.base_mut().attrs.insert("KAPI_NO".into(), "2/B".into());
    assert!(doc.update(door, e), "the door changes");
    assert!(!app.address.fresh(
        "adres-1",
        &def,
        &app.document.as_ref().expect("a drawing").model
    ));
}

#[test]
fn adres_verisi_checks_the_definition_and_saves_it_as_a_project_setting() {
    let mut app = app();
    send(&mut app, Message::Run("address.data"));
    let w = app.address.data.as_ref().expect("the window is open");
    assert_eq!(w.names(), ["Numarataj"]);
    ev(&mut app, Event::Data(Data::Check));
    let checked = app
        .address
        .data
        .as_ref()
        .and_then(|w| w.checked())
        .expect("Denetle answered")
        .clone();
    // The reference's counts for the metres scene (fixtures/geocode/v1/cases.json).
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("the cases read");
    let c = &cases["scenes"][0]["counts"];
    assert_eq!(
        checked.counts.doors as u64,
        c["doors"].as_u64().unwrap_or(0)
    );
    assert_eq!(
        checked.counts.streets as u64,
        c["streets"].as_u64().unwrap_or(0)
    );
    assert_eq!(
        checked.empty_streets as u64,
        c["emptyStreets"].as_u64().unwrap_or(0)
    );
    assert_eq!(
        checked.unread_doors as u64,
        c["unreadDoors"].as_u64().unwrap_or(0)
    );
    // Doors 14 twice on Atatürk Bulvarı in Kızılay.
    assert_eq!(checked.repeated, 1);
    ev(&mut app, Event::Data(Data::Name("Numarataj 2026".into())));
    ev(&mut app, Event::Data(Data::Save));
    assert!(app.address.data.is_none());
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.settings().address_locators[0].name, "Numarataj 2026");
    assert!(!doc.can_undo(), "a project setting, not an undo step");
}

#[test]
fn the_sources_are_the_projects_data_then_the_devices_providers() {
    let app = app();
    let sources = app.address_sources();
    assert_eq!(sources[0].id, "locator:adres-1");
    assert_eq!(sources[0].why, None);
    assert!(sources[1..].iter().all(|s| s.id.starts_with("provider:")));
    assert!(sources.iter().any(|s| s.id == "provider:nominatim-osm"));
    assert_eq!(
        app.address_source().map(|s| s.id).as_deref(),
        Some("locator:adres-1")
    );
}

#[test]
fn the_providers_card_is_accepted_before_a_provider_is_used() {
    let mut app = app();
    let saved = super::online::online().settings();
    send(&mut app, Message::Run("address.providers"));
    let at = super::online::online()
        .providers()
        .iter()
        .position(|p| p.id == "photon-komoot")
        .expect("Photon");
    ev(&mut app, Event::Providers(Providers::Choose(at)));
    // Kullan waits for the card.
    ev(&mut app, Event::Providers(Providers::Use(true)));
    ev(&mut app, Event::Providers(Providers::Accept(true)));
    ev(&mut app, Event::Providers(Providers::Use(true)));
    ev(&mut app, Event::Providers(Providers::Save));
    assert!(app.address.providers.is_none());
    let photon = super::online::online()
        .providers()
        .into_iter()
        .find(|p| p.id == "photon-komoot")
        .expect("Photon");
    assert!(photon.usable());
    assert!(
        app.address_sources()
            .iter()
            .any(|s| s.id == "provider:photon-komoot" && s.why.is_none())
    );
    // The device's settings as they were (the tests share them).
    super::online::online().save(saved).expect("restored");
}
