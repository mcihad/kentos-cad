//! Sonuç and Bütün tablo in the tool's window (docs/adr/0592 §3): a layer bound to sorgu.gpkg's `bina` by a part (4
//! of its 12 squares), Nesneden nokta on it; the window offers Bütün tablo for the bound input only when the result
//! goes to a file or a database, the run asks `cad.processing.run` with the input read from its source and the sink,
//! the file takes 12 points, the drawing does not change; a refusal is said in the command's words; and the pictures:
//!
//! ```text
//! cargo test -p kentos-desktop processing::sink_tests::screens -- --ignored --nocapture
//! ```
//!
//! They go to `.run/shots/islem-sonuc-*.png`. Toplu çalıştır has the same Sonucun yeri (docs/adr/0593 §5): the rows
//! read the bound input whole and each row's layer is a table of the file; its pictures are `islem-sonuc-toplu*`.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use kentos_commands::{CallOp, SinkMode};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::json;

use super::batch::{self, Line};
use super::sink::{Place, SinkEvent, run_to_sink};
use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, scratch};

const LAYER: &str = "import-bina";

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::Processing(e));
}

fn sink(app: &mut App, e: SinkEvent) {
    event(app, Event::Sink(e));
}

/// The drawing with `bina` bound by its first four squares, Nesneden nokta's window open on the bound layer.
fn scene(name: &str) -> (App, PathBuf) {
    let dir = scratch(name);
    let file = dir.join("sorgu.gpkg");
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sources/v1/sorgu.gpkg"),
        &file,
    )
    .expect("copied");
    // The desktop registers İşlemler's tools for the commands when it starts (main.rs).
    kentos_processing::command::register_as("desktop");
    let mut app = app_with_drawing();
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let answer = registry::run(
        &mut ExecutionContext::new(doc),
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": file.display().to_string(), "table": "bina",
                          "extent": [486995.0, 4420595.0, 487105.0, 4420625.0] } }),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let _ = app.update(Message::Run("processing.run.convert.toPoints"));
    event(
        &mut app,
        Event::Value(
            "input".into(),
            json!({ "scope": "layer", "layerId": LAYER }),
        ),
    );
    (app, dir)
}

fn window(app: &App) -> &super::dialog::ToolDialog {
    app.processing.dialog.as_ref().expect("a window")
}

/// What the thread does, done here and handed to the window as the thread hands it.
fn finish(app: &mut App) {
    let input = app.processing_sink_input().expect("an input");
    let mut copy = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .reading_copy();
    let end = run_to_sink(&mut copy, &input, Arc::new(AtomicBool::new(false)));
    sink(app, SinkEvent::Ran(end));
}

#[test]
fn a_bound_input_is_read_whole_into_a_file_and_the_drawing_stays() {
    let (mut app, dir) = scene("sonuc-dosya");
    let bound = &window(&app).sink.bound;
    assert_eq!(bound.len(), 1, "{bound:?}");
    assert!(bound[0].part, "the drawing holds a part of the table");
    assert_eq!(bound[0].layer, LAYER);
    // To the drawing, the input cannot be read whole: the switch does nothing to the run.
    sink(&mut app, SinkEvent::Whole("input".into(), true));
    assert_eq!(window(&app).sink.place, Place::Drawing);
    // To a file: the run asks for the source and the sink.
    let out = dir.join("sonuc.gpkg");
    sink(&mut app, SinkEvent::Place(Place::File));
    sink(&mut app, SinkEvent::Path(out.display().to_string()));
    let input = app.processing_sink_input().expect("an input");
    assert_eq!(
        input.parameters["input"],
        json!({ "scope": "source", "layer": LAYER })
    );
    let s = input.sink.clone().expect("a sink");
    assert_eq!(
        (s.path.as_str(), s.mode, s.schema),
        (out.to_str().expect("a path"), SinkMode::New, None)
    );
    let before = app.document.as_ref().expect("a drawing").model.revision();
    event(&mut app, Event::Run);
    assert!(
        matches!(window(&app).status, RunStatus::Running { .. }),
        "{:?}",
        window(&app).status
    );
    finish(&mut app);
    let RunStatus::Ok { text, undo, .. } = window(&app).status.clone() else {
        panic!("{:?}", window(&app).status);
    };
    assert!(!undo);
    assert!(text.starts_with("12 nesneden 12 nokta yazıldı."), "{text}");
    assert!(
        text.contains("Sonuç sonuc.gpkg dosyasına yazıldı: Merkezler (12 satır).")
            && text.contains("Kaynaktan okunan: 12 kayıt (sorgu.gpkg › bina)."),
        "{text}"
    );
    assert!(out.exists(), "the file is written");
    assert_eq!(
        app.document.as_ref().expect("a drawing").model.revision(),
        before
    );
    // The part only (Bütün tablo off): 4 points, said as a part.
    sink(&mut app, SinkEvent::Whole("input".into(), false));
    sink(&mut app, SinkEvent::Mode(SinkMode::ReplaceTables));
    finish(&mut app);
    let RunStatus::Ok { text, .. } = window(&app).status.clone() else {
        panic!("{:?}", window(&app).status);
    };
    assert!(text.starts_with("4 nesneden 4 nokta yazıldı."), "{text}");
    assert!(text.contains("yalnız bir parçası"), "{text}");
}

fn batch_event(app: &mut App, e: batch::Event) {
    event(app, Event::Batch(e));
}

/// Toplu çalıştır over the window: two rows, each its own output layer (the layer column's cells).
fn open_batch(app: &mut App) {
    batch_event(app, batch::Event::Open);
    batch_event(app, batch::Event::RemoveColumn(0));
    batch_event(app, batch::Event::AddColumn("layer".into()));
    batch_event(app, batch::Event::Cell(0, 0, "Bir".into()));
    batch_event(app, batch::Event::Cell(1, 0, "İki".into()));
}

/// Looks at the running batch until it is over.
fn finish_batch(app: &mut App) {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while app.processing.batch.as_ref().is_some_and(|b| b.running()) {
        assert!(std::time::Instant::now() < until, "the batch did not end");
        std::thread::sleep(std::time::Duration::from_millis(15));
        batch_event(app, batch::Event::Tick);
    }
}

#[test]
fn a_batch_reads_whole_and_writes_each_row_to_a_table_of_the_file() {
    let (mut app, dir) = scene("toplu-dosya");
    let out = dir.join("toplu.gpkg");
    sink(&mut app, SinkEvent::Place(Place::File));
    sink(&mut app, SinkEvent::Path(out.display().to_string()));
    sink(&mut app, SinkEvent::Whole("input".into(), true));
    open_batch(&mut app);
    let before = app.document.as_ref().expect("a drawing").model.revision();
    batch_event(&mut app, batch::Event::Run);
    finish_batch(&mut app);
    let b = app.processing.batch.as_ref().expect("the table");
    assert!(
        matches!(&b.line, Line::Done(s) if s == "2 satır çalıştı: 24 nesne toplu.gpkg dosyasına 2 tabloya yazıldı."),
        "{:?}",
        b.line
    );
    let record = b.record.as_ref().expect("its record");
    let tables: Vec<(&str, u64)> = record
        .sink
        .as_ref()
        .expect("where it went")
        .tables
        .iter()
        .map(|t| (t.table.as_str(), t.rows))
        .collect();
    assert_eq!(tables, [("Bir", 12), ("İki", 12)]);
    assert_eq!(
        record.inputs[0].rows,
        Some(12),
        "the whole table, not the part"
    );
    assert!(out.exists());
    assert_eq!(
        app.document.as_ref().expect("a drawing").model.revision(),
        before,
        "the drawing does not change"
    );
    // An empty path is said before anything runs.
    sink(&mut app, SinkEvent::Path(String::new()));
    batch_event(&mut app, batch::Event::Run);
    let b = app.processing.batch.as_ref().expect("the table");
    assert!(
        matches!(&b.line, Line::Failed(m) if m.contains("GeoPackage dosyasını seçin")),
        "{:?}",
        b.line
    );
}

#[test]
fn a_database_without_an_address_is_asked_for_and_a_refusal_is_said() {
    let (mut app, _) = scene("sonuc-vt");
    sink(&mut app, SinkEvent::Place(Place::Database));
    sink(&mut app, SinkEvent::Address(String::new()));
    event(&mut app, Event::Run);
    let RunStatus::Invalid(why) = window(&app).status.clone() else {
        panic!("{:?}", window(&app).status);
    };
    assert!(why.contains("veritabanının adresini"), "{why}");
    sink(
        &mut app,
        SinkEvent::Address("postgresql://kentos:gizli@localhost/vt".into()),
    );
    finish(&mut app);
    let RunStatus::Error(why) = window(&app).status.clone() else {
        panic!("{:?}", window(&app).status);
    };
    assert!(why.contains("adreste parola var"), "{why}");
}

#[test]
#[ignore = "pictures: cargo test -p kentos-desktop processing::sink_tests::screens -- --ignored --nocapture"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let views = [
        "cizim",
        "dosya",
        "veritabani",
        "yazildi",
        "toplu",
        "toplu-yazildi",
    ];
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in views {
                if only
                    .as_deref()
                    .is_some_and(|o| !o.split(',').any(|x| x == name))
                {
                    continue;
                }
                let (mut app, dir) = scene(&format!("sonuc-resim-{name}"));
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
                    "toplu" | "toplu-yazildi" => {
                        sink(&mut app, SinkEvent::Place(Place::File));
                        sink(
                            &mut app,
                            SinkEvent::Path(dir.join("toplu.gpkg").display().to_string()),
                        );
                        sink(&mut app, SinkEvent::Whole("input".into(), true));
                        open_batch(&mut app);
                        if name == "toplu-yazildi" {
                            batch_event(&mut app, batch::Event::Run);
                            finish_batch(&mut app);
                        }
                    }
                    "dosya" | "yazildi" => {
                        sink(&mut app, SinkEvent::Place(Place::File));
                        sink(
                            &mut app,
                            SinkEvent::Path(dir.join("sonuc.gpkg").display().to_string()),
                        );
                        sink(&mut app, SinkEvent::Whole("input".into(), true));
                        if name == "yazildi" {
                            finish(&mut app);
                        }
                    }
                    "veritabani" => {
                        sink(&mut app, SinkEvent::Place(Place::Database));
                        sink(
                            &mut app,
                            SinkEvent::Address(
                                "postgresql://kentos@vt.belediye.local/kadastro".into(),
                            ),
                        );
                        sink(&mut app, SinkEvent::Schema("sonuc".into()));
                        sink(&mut app, SinkEvent::Whole("input".into(), true));
                    }
                    _ => {}
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if matches!(name, "dosya" | "veritabani" | "yazildi") {
                    // The form at its end: Sonucun yeri under Çıktı.
                    snapshot.operate(app.view(), Box::new(crate::files_testing::SnapDown));
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!("islem-sonuc-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
