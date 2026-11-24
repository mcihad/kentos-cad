//! Arama's rules and the tab over `fixtures/interaction/v1/data-search.kcad`
//! (docs/adr/0178): the words of the rows (the core decides which objects
//! answer; fixtures/search/v1 holds it), the choices, the place typed and
//! its mark, as the app runs them. The trace `data-search.json` plays the
//! whole flow with the mouse and the keyboard.


use super::{COLUMNS, Event, LIMIT, count_text, field_cell, next_sort, texts};
use crate::app::{App, Message};

const DRAWING: &str = include_str!("../../../../../fixtures/interaction/v1/data-search.kcad");

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

fn opened() -> App {
    let mut app = app();
    let _ = app.run("data.search");
    app
}

#[test]
fn the_columns_are_the_adrs_sira_the_drawings_order() {
    assert_eq!(
        COLUMNS,
        [
            ("Sıra", None),
            ("Katman", Some("layer")),
            ("Tür", Some("kind")),
            ("Alan", Some("field")),
            ("Değer", Some("value")),
        ]
    );
}

#[test]
fn a_header_sorts_ascending_then_descending_then_in_the_drawings_order() {
    let (mut sort, mut desc) = (None, false);
    for want in [
        (Some("value"), false),
        (Some("value"), true),
        (None, false),
        (Some("layer"), false),
    ] {
        let column = if want.0 == Some("layer") {
            Some("layer")
        } else {
            Some("value")
        };
        (sort, desc) = next_sort(sort, desc, column);
        assert_eq!((sort, desc), want);
    }
    // Sıra is the drawing's order.
    assert_eq!(next_sort(Some("layer"), true, None), (None, false));
}

#[test]
fn the_count_says_how_many_answer_and_how_many_are_listed() {
    assert_eq!(count_text(3, 3), "3 sonuç");
    assert_eq!(count_text(5000, 12000), "5000 / 12000 sonuç");
    assert_eq!(LIMIT, 5000);
}

#[test]
fn a_rows_field_is_named_with_the_others_that_answer_counted() {
    use kentos_geometry_core::ops::data_search::Row;
    let row = |field, name: Option<&str>, more| Row {
        record: 0,
        field,
        name: name.map(str::to_owned),
        value: String::new(),
        more,
    };
    assert_eq!(field_cell(&row("label", None, 0)), "Ad / etiket");
    assert_eq!(field_cell(&row("label", None, 2)), "Ad / etiket +2");
    assert_eq!(field_cell(&row("text", None, 0)), "Yazı");
    assert_eq!(field_cell(&row("block", None, 1)), "Blok adı +1");
    assert_eq!(field_cell(&row("attr", Some("Parsel"), 0)), "Parsel");
}

#[test]
fn a_row_selects_its_object_and_the_run_goes_with_ctrl_and_shift() {
    let mut app = opened();
    typed(&mut app, "101");
    let at = |app: &App| app.selection.ids().iter().map(|s| s.0).collect::<Vec<_>>();
    ev(&mut app, Event::Press(0));
    assert_eq!(at(&app), [1]);
    // Shift takes the run from the last click without Shift.
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::SHIFT));
    ev(&mut app, Event::Press(2));
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::empty()));
    assert_eq!(at(&app), [1, 6, 8]);
    // Ctrl turns one over.
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::CTRL));
    ev(&mut app, Event::Press(1));
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::empty()));
    assert_eq!(at(&app), [1, 8]);
    // Enter takes the first row.
    ev(&mut app, Event::Submit);
    assert_eq!(at(&app), [1]);
}

#[test]
fn a_typed_coordinate_goes_marks_and_is_removed() {
    let mut app = opened();
    assert!(app.data_place().is_none() && app.data_mark().is_none());
    assert!(!app.available("data.unmark"));
    typed(&mut app, "487010,4420005");
    let at = app.data_place().expect("a place");
    assert_eq!((at.x, at.y), (487010.0, 4420005.0));
    ev(&mut app, Event::Go);
    assert_eq!(
        app.data_mark().map(|p| (p.x, p.y)),
        Some((487010.0, 4420005.0))
    );
    assert_eq!(app.viewport.camera.center.x, 487010.0);
    assert!(app.available("data.unmark"));
    // A word that is no place goes nowhere, and the mark stays.
    typed(&mut app, "101");
    assert!(app.data_place().is_none());
    ev(&mut app, Event::Go);
    assert!(app.data_mark().is_some());
    let _ = app.run("data.unmark");
    assert!(app.data_mark().is_none());
}

#[test]
fn the_mark_is_the_open_drawings_only() {
    let mut app = opened();
    typed(&mut app, "487010 4420005");
    ev(&mut app, Event::Submit);
    assert!(app.data_mark().is_some());
    // Another drawing: the place is not on it.
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!(
        "../../../../../fixtures/interaction/v1/empty.kcad"
    ))
    .expect("reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    assert!(app.data_mark().is_none());
}

#[test]
fn a_cad_project_words_and_reads_the_coordinate_as_x_then_y() {
    let cad = DRAWING.replace("\"workspace\": \"gis\"", "\"workspace\": \"cad\"");
    assert_ne!(cad, DRAWING, "the drawing is a CBS one");
    let mut app = app_on(&cad);
    let _ = app.run("data.search");
    let f = app.format();
    assert!(f.axes_text(texts::PLACEHOLDER).ends_with("ya da X,Y"));
    assert!(f.axes_text(texts::SEARCH_HINT).contains("(X,Y)"));
    assert!(f.axes_text(texts::NO_QUERY).contains("X,Y yazarsanız"));
    // East first in both types: only its name changes.
    typed(&mut app, "487010,4420005");
    let at = app.data_place().expect("a place");
    assert_eq!((at.x, at.y), (487010.0, 4420005.0));
    // A CBS project keeps the surveyor's Y,X.
    let gis = app_on(DRAWING).format();
    assert!(gis.axes_text(texts::PLACEHOLDER).ends_with("ya da Y,X"));
}
