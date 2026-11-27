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

use crate::gis;
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
    let _ = app.update(Message::Gis(gis::Event::Processing(e)));
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
    app.keep_tool_names();
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
    app.gis.processing.dialog.as_ref().expect("a window")
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
    // Çalıştır runs it on another thread; its end comes back to the window (driven here, not raced by a second run).
    let task = app.update(Message::Gis(gis::Event::Processing(Event::Run)));
    assert!(
        matches!(window(&app).status, RunStatus::Running { .. }),
        "{:?}",
        window(&app).status
    );
    crate::files_testing::drive(&mut app, task);
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
    while app
        .gis
        .processing
        .batch
        .as_ref()
        .is_some_and(|b| b.running())
    {
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
    let b = app.gis.processing.batch.as_ref().expect("the table");
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
    let b = app.gis.processing.batch.as_ref().expect("the table");
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

/// Asked as Çalıştır asks it (its record kept), then finished here as the thread would.
fn finish_asked(app: &mut App) {
    let w = app.gis.processing.dialog.as_mut().expect("a window");
    w.sink.asked = Some(super::sink::asked_of(w));
    finish(app);
}

fn history(app: &App) -> Vec<kentos_processing::RunRecord> {
    app.gis.processing.runner.history().to_vec()
}

/// The second round's T30 and T31 (docs/adr/9354): to a file the output is a table's name; a file there and a table
/// there are said in the window's words, the first file kept; a written result adds as a data layer; written,
/// refused and drawn runs all go into İşlem geçmişi, the drawn one marked as taken back after Geri al; opened again
/// with fewer objects than it read, the window says so.
#[test]
fn a_file_there_is_said_in_the_windows_words_and_each_run_goes_into_the_history() {
    let (mut app, dir) = scene("sonuc-gecmis");
    crate::data_layers::register();
    let out = dir.join("sonuc.gpkg");
    sink(&mut app, SinkEvent::Place(Place::File));
    sink(&mut app, SinkEvent::Path(out.display().to_string()));
    finish_asked(&mut app);
    assert!(matches!(window(&app).status, RunStatus::Ok { .. }), "{:?}", window(&app).status);
    let written = window(&app).sink.written.clone().expect("what it wrote");
    assert!(written.file);
    assert_eq!(written.tables, [("Merkezler".to_owned(), 4)]);
    let kept = std::fs::metadata(&out).and_then(|m| m.modified()).expect("the file");
    // Again with Varsa üzerine yaz off: the file is there, said in the window's words; the first file is kept.
    finish_asked(&mut app);
    let RunStatus::Error(why) = window(&app).status.clone() else {
        panic!("{:?}", window(&app).status);
    };
    assert_eq!(
        why,
        "Bu GeoPackage zaten var: sonuc.gpkg. Başka dosya seçin ya da Varsa üzerine yaz'ı açın. Dosyaya dokunulmadı."
    );
    assert_eq!(std::fs::metadata(&out).and_then(|m| m.modified()).expect("the file"), kept);
    assert!(window(&app).sink.written.is_none(), "no card for a refusal");
    // Tablo ekle into it: the table is there, told from the file by the provider's code.
    sink(&mut app, SinkEvent::Mode(SinkMode::AddTables));
    finish_asked(&mut app);
    let RunStatus::Error(why) = window(&app).status.clone() else {
        panic!("{:?}", window(&app).status);
    };
    assert!(
        why.starts_with("“Merkezler” tablosu sonuc.gpkg dosyasında var. Tablolar'da Tabloyu değiştir'i seçin"),
        "{why}"
    );
    // Written once more over it, then Katman olarak ekle: the table is a data layer of the drawing.
    sink(&mut app, SinkEvent::Mode(SinkMode::ReplaceTables));
    finish_asked(&mut app);
    let task = app.update(Message::Gis(gis::Event::Processing(Event::Sink(SinkEvent::AddLayer))));
    crate::files_testing::drive(&mut app, task);
    assert!(window(&app).sink.written.as_ref().is_some_and(|w| w.added));
    let layers = app.document.as_ref().expect("a drawing").model.layers();
    assert!(
        layers.leaves().iter().any(|l| l.name == "Merkezler" && l.data.is_some()),
        "the written table added as a data layer"
    );
    // Dosya konumunu aç: the file's folder.
    sink(&mut app, SinkEvent::Reveal);
    let opened = crate::sheet_pdf::OPENED.with(|o| o.borrow().clone());
    assert_eq!(opened.last(), Some(&dir), "{opened:?}");
    // To the drawing: a layer of 4 points.
    sink(&mut app, SinkEvent::Place(Place::Drawing));
    event(&mut app, Event::LayerName("layer".into(), "Çizimdeki merkezler".into()));
    event(&mut app, Event::Run);
    assert!(matches!(window(&app).status, RunStatus::Ok { .. }), "{:?}", window(&app).status);
    let runs = history(&app);
    let states: Vec<kentos_processing::Status> = runs.iter().map(|r| r.status).collect();
    use kentos_processing::Status::{Error, Ok};
    assert_eq!(states, [Ok, Ok, Error, Error, Ok], "newest first");
    let first = runs.last().expect("the first run");
    assert!(first.words.iter().any(|w| w == "Sonucun yeri: sonuc.gpkg (GeoPackage dosyası)"), "{:?}", first.words);
    assert!(first.words.iter().any(|w| w == "Yazılan tablolar: Merkezler (4 satır)"), "{:?}", first.words);
    assert!(runs[3].summary.starts_with("Yazılmadı: Bu GeoPackage zaten var"), "{}", runs[3].summary);
    // Geri al: the drawn run's points are gone, its record says so with its layer.
    assert_eq!(app.run_gone(&runs[0], app.alive(&runs[0]).len()), None);
    let _ = app.update(Message::Run("edit.undo"));
    let gone = app.run_gone(&runs[0], app.alive(&runs[0]).len()).expect("taken back");
    assert_eq!(gone, "Geri alındı: eklediği nesneler çizimde yok, “Çizimdeki merkezler” katmanı kaldırıldı.");
    // One of the four squares deleted, the first run opened again: the window says it read 4 and reads 3 now.
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        let square = model
            .entities()
            .find(|e| e.base().layer_id == LAYER)
            .map(|e| kentos_domain::Slot(e.base().id))
            .expect("a square");
        model.remove(&[square]);
    }
    event(&mut app, Event::Panel(super::panel::Event::Reopen(first.seq)));
    let note = window(&app).reopen_note.clone().expect("a note");
    assert!(note.contains("önceki çalıştırma 4 nesne okudu, girdi şimdi 3 nesne"), "{note}");
    // Its target came back with it: the same file, Yeni.
    let s = &window(&app).sink;
    assert_eq!((s.place, s.path.as_str(), s.mode), (Place::File, out.to_str().expect("a path"), SinkMode::New));
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

/// Seen in the real window (27 November 2026): a run's result moved to a source's table (Veri kaynağına taşı) is not
/// taken back: its record stays Başarılı and says where its rows are; objects deleted from a layer still there are
/// said deleted; Geri al alone marks it Geri alındı.
#[test]
fn a_result_moved_to_a_source_or_deleted_is_not_taken_for_undone() {
    let (mut app, dir) = scene("sonuc-tasindi");
    crate::data_layers::register();
    event(&mut app, Event::LayerName("layer".into(), "Merkezler".into()));
    event(&mut app, Event::Run);
    let r = history(&app)[0].clone();
    let layer = {
        let model = &app.document.as_ref().expect("a drawing").model;
        model.layers().leaves().into_iter().find(|l| l.name == "Merkezler").map(|l| l.id.clone()).expect("its layer")
    };
    // Deleted from its layer: not taken back.
    let snapshot = app.document.as_ref().expect("a drawing").model.clone();
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        model.remove(&r.added);
    }
    assert!(!app.run_undone(&r));
    assert_eq!(app.run_gone(&r, 0).as_deref(), Some("Eklediği nesneler çizimde yok: silindi."));
    // Moved to a GeoPackage's table: said so.
    app.document.as_mut().expect("a drawing").model = snapshot;
    let input = kentos_commands::LayersMigrate {
        layer: layer.clone(),
        target: kentos_commands::CreateTarget::Gpkg,
        path: Some(dir.join("tasinan.gpkg").display().to_string()),
        address: None,
        schema: None,
        table: Some("merkezler".into()),
        existing: None,
    };
    let model = &mut app.document.as_mut().expect("a drawing").model;
    let done = kentos_native_application::layers_migrate::execute(&mut ExecutionContext::new(model), input);
    assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
    assert_eq!(app.alive(&r).len(), 0, "its objects left the drawing");
    assert!(!app.run_undone(&r));
    assert_eq!(
        app.run_gone(&r, 0).as_deref(),
        Some("Sonucu veri kaynağına taşındı: “Merkezler” katmanının satırları kaynağında.")
    );
}

/// The pictures of the second round's T30 and T31 (docs/adr/9354), into `.run/gui-evidence/kullanim-9354/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib processing::sink_tests::usage_pictures -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures"]
fn usage_pictures() {
    let to_file = |tag: &str| {
        let (mut app, dir) = scene(tag);
        sink(&mut app, SinkEvent::Place(Place::File));
        sink(&mut app, SinkEvent::Path(dir.join("sonuc.gpkg").display().to_string()));
        app
    };
    // The form scrolled to its end: Sonucun yeri, then Çıktı's Tablo adı.
    for (theme, size) in [("light", iced::Size::new(1440.0, 900.0)), ("dark", iced::Size::new(1100.0, 760.0))] {
        let mut app = to_file("resim-9354-hedef");
        crate::files_testing::themed(&mut app, theme);
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = kentos_ui::snapshot::Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.operate(app.view(), Box::new(crate::files_testing::SnapDown));
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = snapshot.render(app.view(), &app.theme());
        let file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../.run/gui-evidence/kullanim-9354/01-dosya-hedefi-{theme}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
    crate::files_testing::usage_shots("9354", "02-yazildi", || {
        let mut app = to_file("resim-9354-yazildi");
        finish_asked(&mut app);
        app
    });
    crate::files_testing::usage_shots("9354", "03-cakisma", || {
        let mut app = to_file("resim-9354-cakisma");
        finish_asked(&mut app);
        finish_asked(&mut app);
        app
    });
    let ran = |tag: &str| {
        let mut app = to_file(tag);
        finish_asked(&mut app);
        finish_asked(&mut app);
        sink(&mut app, SinkEvent::Place(Place::Drawing));
        event(&mut app, Event::LayerName("layer".into(), "Çizimdeki merkezler".into()));
        event(&mut app, Event::Run);
        let _ = app.update(Message::Run("edit.undo"));
        app
    };
    crate::files_testing::usage_shots("9354", "04-gecmis", || {
        let mut app = ran("resim-9354-gecmis");
        event(&mut app, Event::Close);
        let _ = app.update(Message::Run("processing.history"));
        app
    });
    crate::files_testing::usage_shots("9354", "05-yeniden-ac", || {
        let mut app = ran("resim-9354-yeniden");
        let first = history(&app).last().map(|r| r.seq).expect("the first run");
        {
            let model = &mut app.document.as_mut().expect("a drawing").model;
            let square = model
                .entities()
                .find(|e| e.base().layer_id == LAYER)
                .map(|e| kentos_domain::Slot(e.base().id))
                .expect("a square");
            model.remove(&[square]);
        }
        event(&mut app, Event::Panel(super::panel::Event::Reopen(first)));
        app
    });
}
