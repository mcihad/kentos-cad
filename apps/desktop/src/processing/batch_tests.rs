//! Toplu çalıştır in İşlemler's window (docs/adr/0529 §4): the table run as `cad.processing.batch` from the window as
//! a user fills it, one undo step, a row that stops shown and nothing written, the record shown in the window and in
//! the history, and the pictures for the owner.

use std::time::{Duration, Instant};

use kentos_commands::DocumentSnapshotV1;
use kentos_domain::Slot;
use serde_json::json;

use super::batch::{self, Line, RowState};
use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const PARCELS: &str = include_str!("../../../../fixtures/processing/v1/parcels.kcad");
const QUERIES: &str = include_str!("../../../../fixtures/processing/v1/queries.kcad");

fn app_with(drawing: &str) -> App {
    // The desktop registers İşlemler's tools for the commands when it starts (main.rs).
    kentos_processing::command::register_as("desktop");
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(drawing).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("the drawing opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::Processing(e));
}

fn batch_event(app: &mut App, e: batch::Event) {
    event(app, Event::Batch(e));
}

fn state(app: &App) -> &batch::BatchState {
    app.processing.batch.as_ref().expect("the table is open")
}

/// Öznitelik hesapla opened on the first three parcels, its expression and the first text cell as a user types them.
fn open_calculate(app: &mut App) {
    app.selection.set([Slot(1), Slot(2), Slot(3)]);
    let _ = app.update(Message::Run("processing.run.attributes.calculate"));
    event(
        app,
        Event::Value("value".into(), json!("$genişlik || ' x ' || $yükseklik")),
    );
    batch_event(app, batch::Event::Open);
}

/// Looks at the running job until it is over.
fn finish(app: &mut App) {
    let until = Instant::now() + Duration::from_secs(20);
    while state(app).running() {
        assert!(Instant::now() < until, "the batch did not end");
        std::thread::sleep(Duration::from_millis(15));
        batch_event(app, batch::Event::Tick);
    }
}

fn type_cell(app: &mut App, row: usize, column: usize, text: &str) {
    batch_event(app, batch::Event::Cell(row, column, text.to_owned()));
}

#[test]
fn the_table_opens_with_a_column_and_two_rows() {
    let mut app = app_with(PARCELS);
    open_calculate(&mut app);
    let b = state(&app);
    // Öznitelik hesapla has no number: its first offered parameter is the column (the objects).
    assert_eq!(b.table.rows.len(), 2);
    assert_eq!(b.table.columns.len(), 1);
    assert!(!b.running());
    // Forma dön closes the table, the window stays.
    batch_event(&mut app, batch::Event::Back);
    assert!(app.processing.batch.is_none() && app.processing.dialog.is_some());
}

#[test]
fn a_batch_runs_from_the_window_as_one_undo_step_with_its_record() {
    let mut app = app_with(PARCELS);
    open_calculate(&mut app);
    batch_event(&mut app, batch::Event::RemoveColumn(0));
    batch_event(&mut app, batch::Event::AddColumn("field".into()));
    type_cell(&mut app, 0, 0, "Boyut 1");
    type_cell(&mut app, 1, 0, "Boyut 2");
    batch_event(&mut app, batch::Event::Run);
    finish(&mut app);
    let b = state(&app);
    assert!(
        matches!(&b.line, Line::Done(s) if s.starts_with("2 satır çalıştı")),
        "{:?}",
        b.line
    );
    assert!(
        b.rows.iter().all(|r| matches!(
            r,
            RowState::Ran {
                records_in: 3,
                records_out: 3,
                ..
            }
        )),
        "{:?}",
        b.rows
    );
    let record = b.record.as_ref().expect("its record");
    assert_eq!(record.kind, "batch");
    assert_eq!(record.steps.len(), 2);
    assert!(b.rerun.is_some());
    // Both rows wrote, one undo step takes them both back.
    let doc = &mut app.document.as_mut().expect("open").model;
    let attrs = doc.get(Slot(1)).expect("a parcel").base().attrs.clone();
    assert!(attrs.contains_key("Boyut 1") && attrs.contains_key("Boyut 2"));
    assert_eq!(doc.undo().as_deref(), Some("Toplu: Öznitelik hesapla"));
    assert!(!doc.can_undo());
}

#[test]
fn a_cell_that_does_not_fit_stops_before_anything_runs() {
    let mut app = app_with(PARCELS);
    let _ = app.update(Message::Run("processing.run.geometry.buffer"));
    batch_event(&mut app, batch::Event::Open);
    // The first column is the tool's first number (the distance): a word in it is refused with its row.
    type_cell(&mut app, 1, 0, "çok");
    batch_event(&mut app, batch::Event::Run);
    let b = state(&app);
    assert!(!b.running());
    assert!(
        matches!(&b.line, Line::Failed(m) if m.starts_with("Satır 2:")),
        "{:?}",
        b.line
    );
    assert!(matches!(b.rows[1], RowState::Failed(_)));
    assert!(!app.document.as_ref().expect("open").model.can_undo());
}

#[test]
fn a_row_that_stops_is_shown_and_the_rows_before_it_are_taken_back() {
    let mut app = app_with(QUERIES);
    let _ = app.update(Message::Run("processing.run.attributes.calculate"));
    event(
        &mut app,
        Event::Value(
            "input".into(),
            json!({ "scope": "layer", "layerId": "parsel" }),
        ),
    );
    event(&mut app, Event::Value("field".into(), json!("Ağaç sayısı")));
    batch_event(&mut app, batch::Event::Open);
    batch_event(&mut app, batch::Event::RemoveColumn(0));
    batch_event(&mut app, batch::Event::AddColumn("value".into()));
    type_cell(&mut app, 0, 0, "5");
    type_cell(&mut app, 1, 0, "'çok'");
    batch_event(&mut app, batch::Event::Run);
    finish(&mut app);
    let b = state(&app);
    assert!(
        matches!(&b.line, Line::Failed(m) if m.contains("Toplu işin hiçbir sonucu yazılmadı")),
        "{:?}",
        b.line
    );
    assert_eq!(b.rows[0], RowState::BackOut);
    assert!(
        matches!(&b.rows[1], RowState::Failed(m) if m.contains("tam sayı ister")),
        "{:?}",
        b.rows
    );
    assert!(!app.document.as_ref().expect("open").model.can_undo());
}

/// Hata olursa: Devam et: the row that stops is shown with its reason, taken back alone; the others are written.
#[test]
fn with_continue_the_stopped_row_is_shown_with_its_reason_and_the_others_are_written() {
    let mut app = app_with(QUERIES);
    let _ = app.update(Message::Run("processing.run.attributes.calculate"));
    event(
        &mut app,
        Event::Value(
            "input".into(),
            json!({ "scope": "layer", "layerId": "parsel" }),
        ),
    );
    event(&mut app, Event::Value("field".into(), json!("Ağaç sayısı")));
    batch_event(&mut app, batch::Event::Open);
    assert_eq!(state(&app).table.on_error, kentos_commands::BatchOnError::Stop);
    batch_event(&mut app, batch::Event::RemoveColumn(0));
    batch_event(&mut app, batch::Event::AddColumn("value".into()));
    batch_event(&mut app, batch::Event::AddRow);
    type_cell(&mut app, 0, 0, "5");
    type_cell(&mut app, 1, 0, "'çok'");
    type_cell(&mut app, 2, 0, "7");
    batch_event(&mut app, batch::Event::OnError(batch::OnError::Continue));
    batch_event(&mut app, batch::Event::Run);
    finish(&mut app);
    let b = state(&app);
    assert!(
        matches!(&b.line, Line::Partial(s) if s.contains("1 satır çalışmadı")),
        "{:?}",
        b.line
    );
    assert!(matches!(b.rows[0], RowState::Ran { .. }), "{:?}", b.rows);
    assert!(
        matches!(&b.rows[1], RowState::Failed(m) if m.contains("tam sayı ister")),
        "{:?}",
        b.rows
    );
    assert!(matches!(b.rows[2], RowState::Ran { .. }));
    assert_eq!(b.failed.len(), 1);
    assert_eq!(b.failed[0].0, "Satır 2");
    assert_eq!(b.record.as_ref().map(|r| r.failed.len()), Some(1));
    // The two rows that ran stand as one step; the last value is the one left.
    let doc = &mut app.document.as_mut().expect("open").model;
    let value = |doc: &kentos_domain::Document| {
        doc.get(Slot(1))
            .and_then(|e| e.base().attrs.get("Ağaç sayısı").cloned())
    };
    assert_eq!(value(doc).as_deref(), Some("7"));
    assert_eq!(doc.undo().as_deref(), Some("Toplu: Öznitelik hesapla"));
    assert!(!doc.can_undo());
    assert_eq!(value(doc), None);
}

/// A model run in the window puts its steps, their counts and times in the history.
#[test]
fn the_history_keeps_a_models_steps_with_their_counts() {
    let mut app = app_with(PARCELS);
    app.selection.set([Slot(1), Slot(2)]);
    let _ = app.update(Message::Run("processing.model.builtin.parcelSheet"));
    event(&mut app, Event::Run);
    assert!(matches!(
        app.processing.dialog.as_ref().map(|w| &w.status),
        Some(RunStatus::Ok { .. })
    ));
    let last = &app.processing.runner.history()[0];
    assert_eq!(last.steps.len(), 3);
    assert_eq!(
        last.steps
            .iter()
            .map(|s| (s.records_in, s.records_out))
            .collect::<Vec<_>>(),
        [(2, 6), (2, 7), (2, 2)]
    );
    assert_eq!((last.records_in, last.records_out), (6, 15));
}

/// The window for the owner: the tool's footer with Toplu çalıştır, the table before and after a run with its record,
/// a row that stopped, and the history with a model's steps. Not run by default:
/// `cargo test -p kentos-desktop processing::batch_tests::screens -- --ignored --nocapture --test-threads=1`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let views = ["form", "tablo", "sonuc", "hata", "devam", "gecmis"];
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in views {
                if only
                    .as_deref()
                    .is_some_and(|o| !o.split(',').any(|x| x == name))
                {
                    continue;
                }
                let mut app = app_with(if matches!(name, "hata" | "devam") {
                    QUERIES
                } else {
                    PARCELS
                });
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                match name {
                    "form" => {
                        open_calculate(&mut app);
                        batch_event(&mut app, batch::Event::Back);
                    }
                    "tablo" => {
                        let _ = app.update(Message::Run("processing.run.geometry.buffer"));
                        batch_event(&mut app, batch::Event::Open);
                        batch_event(&mut app, batch::Event::AddRow);
                        for (i, d) in ["5", "10", "20"].iter().enumerate() {
                            type_cell(&mut app, i, 0, d);
                        }
                        batch_event(&mut app, batch::Event::AddColumn("dissolve".into()));
                        type_cell(&mut app, 2, 1, "evet");
                        app.processing.batch.as_mut().expect("table").table.rows[0].label =
                            "yakın".into();
                    }
                    "sonuc" => {
                        open_calculate(&mut app);
                        batch_event(&mut app, batch::Event::RemoveColumn(0));
                        batch_event(&mut app, batch::Event::AddColumn("field".into()));
                        type_cell(&mut app, 0, 0, "Boyut 1");
                        type_cell(&mut app, 1, 0, "Boyut 2");
                        batch_event(&mut app, batch::Event::Run);
                        finish(&mut app);
                    }
                    "hata" | "devam" => {
                        let _ = app.update(Message::Run("processing.run.attributes.calculate"));
                        event(
                            &mut app,
                            Event::Value(
                                "input".into(),
                                json!({ "scope": "layer", "layerId": "parsel" }),
                            ),
                        );
                        event(&mut app, Event::Value("field".into(), json!("Ağaç sayısı")));
                        batch_event(&mut app, batch::Event::Open);
                        batch_event(&mut app, batch::Event::RemoveColumn(0));
                        batch_event(&mut app, batch::Event::AddColumn("value".into()));
                        batch_event(&mut app, batch::Event::AddRow);
                        type_cell(&mut app, 0, 0, "5");
                        type_cell(&mut app, 1, 0, "12");
                        type_cell(&mut app, 2, 0, "'çok'");
                        if name == "devam" {
                            batch_event(&mut app, batch::Event::AddRow);
                            type_cell(&mut app, 3, 0, "9");
                            batch_event(
                                &mut app,
                                batch::Event::OnError(batch::OnError::Continue),
                            );
                        }
                        batch_event(&mut app, batch::Event::Run);
                        finish(&mut app);
                    }
                    _ => {
                        app.selection.set([Slot(1), Slot(2)]);
                        let _ = app.update(Message::Run("processing.model.builtin.parcelSheet"));
                        event(&mut app, Event::Run);
                        event(&mut app, Event::Close);
                        let _ = app.update(Message::Run("processing.history"));
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("islem-toplu-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
