//! address › tests's tests that need the CBS module's surfaces (its project's bottom tabs and panels; docs/adr/9199
//! §4.14): moved from the core's gis/address/tests.rs with the module's package; the work they test is the core's.

use crate::address::{Event, Question, texts};
use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::files_testing::drive;
use crate::gis;
use kentos_interaction::Vec2;

const DRAWING: &str = include_str!("../../../../fixtures/interaction/v1/addresses.kcad");

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
    send(app, Message::Gis(gis::Event::Address(e)));
}

fn find(app: &mut App, text: &str) {
    ev(app, Event::Text(text.to_owned()));
    ev(app, Event::Find);
}

#[test]
fn an_address_is_found_on_the_address_thread_and_a_row_marks_it() {
    let mut app = app();
    send(&mut app, Message::Run("address.search"));
    assert_eq!(app.bottom_tab, BottomTab::Address);
    assert_eq!(app.address_seen(), Some((String::new(), Vec::new())));
    find(&mut app, "Kızılay Mah. Atatürk Blv. No:4 Çankaya/Ankara");
    let a = &app.gis.address;
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
fn a_place_is_answered_and_is_not_written() {
    let mut app = app();
    let task = app.address_at(Vec2::new(E + 150.0, N + 250.0));
    drive(&mut app, task);
    assert_eq!(app.bottom_tab, BottomTab::Address);
    let row = app.gis.address.rows.first().expect("an address");
    assert_eq!(row.kind, "Yol");
    assert_eq!(row.fields[1], "Gazi Mustafa Kemal Bulvarı");
    assert!((row.distance.expect("a distance") - 50.0).abs() < 1e-9);
    assert_eq!(
        app.address_seen().map(|s| s.0).as_deref(),
        Some(texts::PLACE)
    );
    ev(&mut app, Event::AddPoint);
    assert_eq!(
        app.gis.address.said.as_ref().map(|s| s.1.as_str()),
        Some(texts::NOT_FOR_PLACE)
    );
}

/// The municipal report's address row (docs/adr/9154): the same street in two neighbourhoods is tied and named by
/// them, the neighbourhood added matches one, and no candidate is said apart from a tie.
#[test]
fn a_tie_names_its_neighbourhoods_and_no_candidate_is_said_apart() {
    let mut app = app();
    send(&mut app, Message::Run("address.search"));
    find(&mut app, "Atatürk Bulvarı No:2");
    let (line, _) = app.address_seen().expect("the tab is open");
    assert!(line.contains("aday aynı puanda (100): "), "{line}");
    assert!(
        line.contains("Cumhuriyet")
            && line.contains("Kızılay")
            && line.contains("mahalleyi ekleyerek ayırın"),
        "{line}"
    );
    find(&mut app, "Kızılay Atatürk Bulvarı No:4");
    let (line, _) = app.address_seen().expect("the tab is open");
    assert!(line.contains("En iyisi: Eşleşti (100)"), "{line}");
    find(&mut app, "1203/2 Sokak No:1");
    let (line, _) = app.address_seen().expect("the tab is open");
    assert_eq!(line, texts::NOT_FOUND, "no candidate is no tie");
}
