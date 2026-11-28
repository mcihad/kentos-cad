//! search › tests's tests that need the CBS module's surfaces (its project's bottom tabs and panels; docs/adr/9199
//! §4.14): moved from the core's cad/search/tests.rs with the module's package; the work they test is the core's.

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::search::{Event, Field, PANEL_HEIGHT};
use kentos_domain::Slot;

const DRAWING: &str = include_str!("../../../../fixtures/interaction/v1/data-search.kcad");

fn app_on(drawing: &str) -> App {
    let (mut app, _) = App::boot(None);
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(drawing).expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn app() -> App {
    app_on(DRAWING)
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Search(e));
}

fn typed(app: &mut App, words: &str) {
    ev(app, Event::Text(words.to_owned()));
}

/// The count line and rows the tab shows.
fn seen(app: &App) -> (String, Vec<String>) {
    app.data_seen().expect("the Arama tab is open")
}

fn opened() -> App {
    let mut app = app();
    let _ = app.run("data.search");
    app
}

#[test]
fn veride_ara_opens_the_tab_grows_the_panel_and_leaves_a_taller_one() {
    let mut app = app();
    assert!(!app.command_expanded);
    let _ = app.run("data.search");
    assert!(app.command_expanded);
    assert_eq!(app.bottom_tab, BottomTab::Search);
    let bar = f64::from(kentos_ui::widget::tabs::height());
    let height = f64::from(app.bottom_log()) + bar;
    assert!(height >= PANEL_HEIGHT - 1.0, "{height}");
    // A taller panel stays as it is.
    app.bottom_dragged(Some(400.0), std::time::Instant::now());
    let tall = app.bottom_log();
    let _ = app.run("data.search");
    assert_eq!(app.bottom_log(), tall);
}

#[test]
fn the_rows_are_the_objects_that_answer_in_the_drawings_order() {
    let mut app = opened();
    assert_eq!(seen(&app), (String::new(), vec![]));
    typed(&mut app, "101");
    let (count, rows) = seen(&app);
    assert_eq!(count, "3 sonuç");
    assert_eq!(
        rows,
        [
            "Kadastro / Parsel | Kapalı alan | Ad / etiket +1 | 101",
            "Yazılar | Yazı | Yazı | Parsel 101 kuzey sınırı",
            "Nokta | Nokta | Ad / etiket | P101",
        ]
    );
    ev(&mut app, Event::WholeWord(true));
    assert_eq!(seen(&app).0, "2 sonuç");
    ev(&mut app, Event::WholeWord(false));
    // A field off: only Parsel's attribute is read.
    for f in [Field::Label, Field::Text, Field::Block] {
        ev(&mut app, Event::Field(f));
    }
    ev(&mut app, Event::AttrName(Some("Parsel".to_owned())));
    assert_eq!(
        seen(&app),
        (
            "1 sonuç".to_owned(),
            vec!["Kadastro / Parsel | Kapalı alan | Parsel | 101".to_owned()]
        )
    );
    // Nothing asked for: no rows, and the table says so.
    ev(&mut app, Event::Field(Field::Attrs));
    assert_eq!(seen(&app), ("0 sonuç".to_owned(), vec![]));
}

#[test]
fn hepsini_sec_selects_every_object_found_a_hidden_layers_too() {
    let mut app = opened();
    typed(&mut app, "ST");
    assert_eq!(seen(&app).0, "3 sonuç");
    ev(&mut app, Event::SelectAll);
    let ids: Vec<u32> = app.selection.ids().iter().map(|s| s.0).collect();
    assert_eq!(ids, [7, 8, 12]);
    // Yalnız seçimde looks only at them.
    ev(&mut app, Event::OnlySelected(true));
    typed(&mut app, "P1");
    assert_eq!(seen(&app).1.len(), 2);
    assert!(app.selection.contains(Slot(12)));
}
