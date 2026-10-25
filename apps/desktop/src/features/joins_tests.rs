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
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&cases["setup"].to_string())
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

/// The land registry's GeoPackage the source cases read (scripts/fixtures/join_source_cases.py).
fn tapu() -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/joins/v1/tapu.gpkg")
        .to_string_lossy()
        .into_owned()
}

/// Birleşik görünümler with a new view on the source's table and its summing columns, as a person makes it.
fn source_view_in_window(app: &mut App) {
    let _ = app.update(Message::Run("data.joinViews"));
    let _ = app.update(Message::Joins(W::New));
    let _ = app.update(Message::Joins(W::Name("Tapu özeti".into())));
    let _ = app.update(Message::Joins(W::Key("no".into())));
    let _ = app.update(Message::Joins(W::Joined(
        crate::joins_window::Joined::Source,
    )));
    let _ = app.update(Message::Joins(W::SourcePath(tapu())));
    // Tabloları oku runs off the UI thread; its answer is handed in as the thread would.
    let tables = crate::exchange::list_tables(tapu());
    let _ = app.update(Message::Joins(W::SourceTables(tapu(), tables)));
    let _ = app.update(Message::Joins(W::SourceTable("tapu".into())));
    let _ = app.update(Message::Joins(W::WithKey("parsel_no".into())));
    let _ = app.update(Message::Joins(W::SumAdd));
    let _ = app.update(Message::Joins(W::SumAdd));
    let _ = app.update(Message::Joins(W::SumName(1, "Pay toplamı".into())));
    let _ = app.update(Message::Joins(W::SumHow(1, kentos_commands::JoinHow::Sum)));
    let _ = app.update(Message::Joins(W::SumField(1, "pay".into())));
}

#[test]
fn a_view_on_a_source_table_sums_its_records() {
    let mut app = app_with_views();
    source_view_in_window(&mut app);
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
    let v = views.last().expect("the view").clone();
    assert!(v.with.is_empty() && v.source.is_some());
    ev(&mut app, Event::View(v.id.clone()));
    let r = read(&app);
    assert!(r.broken.is_none(), "{:?}", r.broken);
    // The parcel with three owners: three rows, each counting three and summing the shares once.
    assert_eq!(r.rows.len(), 7);
    let count = r
        .columns
        .iter()
        .position(|c| c.name == "Kayıt sayısı")
        .expect("the count");
    let sum = r
        .columns
        .iter()
        .position(|c| c.name == "Pay toplamı")
        .expect("the sum");
    assert_eq!(r.rows[0].cells[count].as_deref(), Some("3"));
    assert_eq!(r.rows[2].cells[sum].as_deref(), Some("0.75"));
    let total = r
        .totals
        .iter()
        .find(|t| t.column as usize == sum)
        .expect("a total");
    assert_eq!(total.sum.as_deref(), Some("2.75"), "each parcel once");
    // The source's cells are not written: the reason says so.
    let malik = r
        .columns
        .iter()
        .position(|c| c.name == "malik")
        .expect("malik");
    assert!(!r.columns[malik].editable);
    ev(&mut app, Event::JoinEdit(0, malik));
    assert!(app.features.join.editing_cell().is_none());
    // Dışa aktar: the rows as a sheet, the summing columns numbers.
    let sheet = kentos_native_application::joins_query::sheet(&r, &v.name);
    assert_eq!(sheet.rows.len(), 7);
    assert_eq!(sheet.columns[sum].kind, kentos_commands::CellKind::Number);
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
            for scene in [
                "tablo",
                "pencere",
                "kopuk",
                "kaynak-pencere",
                "kaynak-tablo",
            ] {
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
                    "kaynak-pencere" => source_view_in_window(&mut app),
                    "kaynak-tablo" => {
                        source_view_in_window(&mut app);
                        let _ = app.update(Message::Joins(W::Save));
                        let id = app
                            .document
                            .as_ref()
                            .expect("open")
                            .model
                            .settings()
                            .join_views
                            .last()
                            .expect("the view")
                            .id
                            .clone();
                        ev(&mut app, Event::View(id));
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
