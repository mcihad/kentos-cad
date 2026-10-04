//! Karne editörü's flow on the sample book and a text book, and its pictures.

use std::path::PathBuf;

use kentos_contracts::SurveySettings;

use super::{Event, NAME, USE};
use crate::app::{App, Message};
use crate::calc::grid::Table;
use crate::calc::{Event as CalcEvent, Window};

fn sample() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/field/v1/sample.gsi")
}

fn send(app: &mut App, e: Event) {
    let _ = app.update(Message::Calc(CalcEvent::FieldBook(e)));
}

/// An app with a drawing whose project checks the two faces' slope
/// distances against 5 mm, Karne editörü open on the sample book.
fn opened() -> App {
    let mut app = crate::files_testing::app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.settings().clone();
    settings.survey = Some(SurveySettings {
        face_slope: Some(0.005),
        ..SurveySettings::default()
    });
    doc.model.set_settings(settings);
    let _ = app.update(Message::Run("calc.fieldbook"));
    send(&mut app, Event::Picked(Some(sample())));
    app
}

/// The sample GSI book (docs/adr/0169 §6): read by its content, its
/// first station reduced, faces paired, 103's slope difference above the
/// project's 5 mm; an observation left out and one renamed reduce again.
#[test]
fn a_gsi_book_is_read_reduced_and_edited() {
    let mut app = opened();
    assert_eq!(app.calc.open, Some(Window::FieldBook));
    let form = &app.calc.fieldbook;
    let book = form.book.as_ref().expect("read");
    assert_eq!((book.format.as_str(), book.stations.len()), ("gsi", 2));
    let r = form.reduction.as_ref().expect("reduced");
    let rows: Vec<(&str, usize)> = r
        .rows
        .iter()
        .map(|x| (x.target.as_str(), x.faces))
        .collect();
    assert_eq!(
        rows,
        [("P2", 2), ("101", 2), ("102", 2), ("103", 2), ("104", 1)]
    );
    let over: Vec<&str> = r
        .rows
        .iter()
        .filter(|x| !x.over.is_empty())
        .map(|x| x.target.as_str())
        .collect();
    assert_eq!(over, ["103"]);
    // P2's face I left out: its face II reads alone.
    let _ = app.update(Message::Calc(CalcEvent::Cell(0, USE, "0".to_owned())));
    let r = app.calc.fieldbook.reduction.as_ref().expect("reduced");
    assert_eq!((r.rows[4].target.as_str(), r.rows[4].faces), ("P2", 1));
    // 104 renamed.
    let _ = app.update(Message::Calc(CalcEvent::Cell(4, NAME, "105".to_owned())));
    let r = app.calc.fieldbook.reduction.as_ref().expect("reduced");
    assert!(r.rows.iter().any(|x| x.target == "105"));
    assert_eq!(Table::get(&app.calc.fieldbook, 4, NAME), "105");
    // The second station.
    send(&mut app, Event::Station(1));
    let r = app.calc.fieldbook.reduction.as_ref().expect("reduced");
    assert_eq!(r.rows.len(), 4);
}

/// A text book is read only to its first line until its point and
/// horizontal reading are mapped; the mapping is remembered.
#[test]
fn a_text_book_is_mapped_then_read() {
    let dir = std::env::temp_dir().join(format!("kentos-karne-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("karne.csv");
    std::fs::write(
        &file,
        "İstasyon;Nokta;Hz;V;SD\nS1;A;10;99;100\nS1;A;210,001;301;100,002\n",
    )
    .expect("written");
    let mut app = crate::files_testing::app_with_drawing();
    let _ = app.update(Message::Run("calc.fieldbook"));
    send(&mut app, Event::Picked(Some(file.clone())));
    let book = app.calc.fieldbook.book.as_ref().expect("read");
    assert_eq!((book.format.as_str(), book.stations.len()), ("csv", 0));
    assert_eq!(book.first_line, ["İstasyon", "Nokta", "Hz", "V", "SD"]);
    for (field, col) in [(0, 0), (2, 1), (3, 2), (4, 3), (5, 4)] {
        send(&mut app, Event::Map(field, Some(col)));
    }
    let r = app.calc.fieldbook.reduction.as_ref().expect("reduced");
    assert_eq!((r.rows.len(), r.rows[0].faces), (1, 2));
    // Opened again, the same mapping reads it at once.
    send(&mut app, Event::Picked(Some(file)));
    assert_eq!(
        app.calc.fieldbook.book.as_ref().map(|b| b.stations.len()),
        Some(1)
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// Karne editörü's pictures: the sample book in the light theme at
/// 1440×900 and the dark at 1100×650, and a text book's mapping.
#[test]
#[ignore = "writes pictures: cargo test -p kentos-desktop calc::fieldbook::tests::screens -- --ignored --nocapture"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let dir = std::env::temp_dir().join(format!("kentos-karne-shots-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let csv = dir.join("arazi-karnesi.csv");
    std::fs::write(
        &csv,
        "İstasyon;Alet;Nokta;Hz;V;SD;Prizma;Kod\nS1;1,55;A;10;99;100;1,6;POL\n",
    )
    .expect("written");
    for (theme, w, h) in [("light", 1440.0, 900.0), ("dark", 1100.0, 650.0)] {
        for name in ["gsi", "csv"] {
            let mut app = opened();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            if name == "csv" {
                app.calc.fieldbook.mapping = super::Mapping::default();
                send(&mut app, Event::Picked(Some(csv.clone())));
            }
            app.follow.flash = None;
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("karne-{name}-{w}x{h}-{theme}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}
