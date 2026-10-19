//! The Tablo tab on a join view and Birleşik görünümler (docs/adr/0527) over the shared cases' drawing
//! (`fixtures/commands/v1/cad.joins.query.json`'s setup): the rows, a cell written to its record, a key's cell
//! left as it is, the window's new view saved through `cad.joins.edit`.

use super::Event;
use crate::app::{App, Message};
use crate::joins_window::Event as W;

fn app_with_views() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.joins.query.json"
    ))
    .expect("the cases read");
    let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(&cases["setup"].to_string())
        .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.features.reset(None);
    let _ = app.update(Message::Run("data.featureTable"));
    let _ = app.update(Message::Features(Event::View("parsel-malik".into())));
    app
}

fn read(app: &App) -> std::rc::Rc<kentos_native_application::joins_query::JoinRead> {
    let doc = app.document.as_ref().expect("open");
    let v = app.join_view(doc).expect("the view");
    app.join_read(doc, &v)
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Features(e));
}

#[test]
fn a_join_view_shows_its_rows_and_writes_a_cell_to_its_record() {
    let mut app = app_with_views();
    let r = read(&app);
    assert_eq!(r.rows.len(), 6);
    assert_eq!(r.report.cardinality, "1:N");
    let pay = r.columns.iter().position(|c| c.name == "pay").expect("pay");
    let key = r
        .columns
        .iter()
        .position(|c| c.name == "Parsel no")
        .expect("the key");
    // The first row's owner's share, written to the owner (the second row's owner keeps hers).
    ev(&mut app, Event::JoinEdit(0, pay));
    ev(&mut app, Event::JoinInput("0.4".into()));
    ev(&mut app, Event::JoinEnter);
    let r = read(&app);
    assert_eq!(r.rows[0].cells[pay].as_deref(), Some("0.4"));
    assert_eq!(r.rows[1].cells[pay].as_deref(), Some("0.5"));
    // The key's cell is not edited: the row would join another record.
    ev(&mut app, Event::JoinEdit(0, key));
    assert!(app.features.join.editing_cell().is_none());
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(
        read(&app).rows[0].cells[pay].as_deref(),
        Some("0.5"),
        "one undo step"
    );
}

#[test]
fn the_window_saves_a_new_view_through_the_command() {
    let mut app = app_with_views();
    let _ = app.update(Message::Run("data.joinViews"));
    assert_eq!(app.dialog, Some(crate::app::Dialog::Joins));
    let _ = app.update(Message::Joins(W::New));
    let _ = app.update(Message::Joins(W::Name("Hisseler".into())));
    let _ = app.update(Message::Joins(W::With("hisse".into())));
    let _ = app.update(Message::Joins(W::Key("no".into())));
    let _ = app.update(Message::Joins(W::WithKey("parseller".into())));
    let _ = app.update(Message::Joins(W::Save));
    assert!(app.joins_window.is_none(), "Kaydet closes the window");
    let views = app
        .document
        .as_ref()
        .expect("open")
        .model
        .settings()
        .join_views
        .clone();
    assert_eq!(views.len(), 2);
    assert_eq!(
        (views[1].layer.as_str(), views[1].with.as_str()),
        ("parsel", "hisse")
    );
    ev(&mut app, Event::View(views[1].id.clone()));
    assert_eq!(read(&app).report.cardinality, "N:N");
}

/// The tab on a join view and the window, dark and light (`.run/shots/birlesik-*`):
///
/// ```text
/// cargo test -p kentos-desktop features::joins_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for scene in ["tablo", "pencere", "kopuk"] {
                let mut app = app_with_views();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::WindowResized(Size::new(width, height)));
                let _ = app.update(Message::BottomResized(320.0));
                match scene {
                    "pencere" => {
                        let _ = app.update(Message::Run("data.joinViews"));
                    }
                    "kopuk" => {
                        // The joined layer deleted: the view says so, no rows of a former reading.
                        let doc = app.document.as_mut().expect("open");
                        let mut s = doc.model.settings().clone();
                        s.join_views[0].with = "eski".into();
                        doc.model.set_settings(s);
                        app.features.changed();
                    }
                    _ => {}
                }
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("birlesik-{scene}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
