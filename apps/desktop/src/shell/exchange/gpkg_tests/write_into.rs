//! GeoPackage dışa aktar into a file that is there (docs/adr/9191): the window asks how, “Yalnız bu tabloları yeniden
//! yaz” first; Tablolara ekle adds the new objects' rows to the file's table and says how many it had already; the
//! write runs on its own thread, and Durdur stops it with the file as it was. Pictures: `kullanim-9191`.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use kentos_commands::GpkgWrite;
use kentos_native_application::ExecutionContext;
use serde_json::json;

use super::super::gpkg_out::{Event as Out, Scope};
use super::super::tests::{run, said, send};
use super::super::{Event, Window};
use crate::app::{App, Message, Picker};
use crate::files_testing::{app_with_drawing, drive, scratch};

fn out(e: Out) -> Event {
    Event::GpkgOut(e)
}

/// A point layer of the drawing's, “Duraklar”: its id.
fn stops(app: &mut App) -> String {
    let doc = app.document.as_mut().expect("a drawing");
    doc.model.add_layer(kentos_domain::NewLayer::layer("Duraklar"), None, false).expect("a layer")
}

/// `n` points on `layer` from `from` on, by `cad.entities.create`.
fn points(app: &mut App, layer: &str, from: u32, n: u32) {
    let objects: Vec<_> = (from..from + n)
        .map(|i| json!({"geometry": {"kind": "point", "p": {"x": 1000.0 + f64::from(i % 400), "y": 2000.0 + f64::from(i / 400)}}}))
        .collect();
    let input = serde_json::from_value(json!({"layerId": layer, "objects": objects})).expect("an input");
    let doc = app.document.as_mut().expect("a drawing");
    let r = kentos_native_application::create::execute(&mut ExecutionContext::new(&mut doc.model), input);
    assert!(matches!(r, kentos_commands::CommandResult::Completed { .. }), "{r:?}");
}

/// The rows of the file's “Duraklar”.
fn rows(path: &Path) -> i64 {
    let gp = kentos_geopackage::GeoPackage::open(path, false).expect("opens");
    gp.connection().query_row("SELECT count(*) FROM \"Duraklar\"", [], |r| r.get(0)).expect("counted")
}

fn last(app: &App) -> Option<String> {
    app.log.lines().last().map(|l| l.text.clone())
}

/// The whole project written to `file`, a new file.
fn first_delivery(app: &mut App, file: &Path) {
    app.picker = Picker::File(file.to_path_buf());
    run(app, "file.export.gpkg");
    send(app, out(Out::Scope(Scope::All)));
    send(app, out(Out::Save));
    assert!(app.exchange.is_none(), "{:?}", last(app));
}

/// The window opened again on the file that is there: asked how to write it.
fn ask_again(app: &mut App) {
    run(app, "file.export.gpkg");
    send(app, out(Out::Scope(Scope::All)));
    send(app, out(Out::Save));
    let Some(Window::GpkgOut(state)) = &app.exchange else { panic!("the window") };
    assert!(state.confirm.is_some(), "asked before writing into it");
}

#[test]
fn rows_are_added_to_the_files_table_through_the_window() {
    let dir = scratch("gpkg-ekle");
    let file = dir.join("teslim.gpkg");
    let mut app = app_with_drawing();
    let layer = stops(&mut app);
    points(&mut app, &layer, 0, 4);
    first_delivery(&mut app, &file);
    assert_eq!(rows(&file), 4);
    points(&mut app, &layer, 4, 3);
    ask_again(&mut app);
    send(&mut app, out(Out::Write(GpkgWrite::Append)));
    let Some(Window::GpkgOut(state)) = &app.exchange else { panic!("the window") };
    let confirm = state.confirm.clone().expect("asked");
    let lines: Vec<String> = app.gpkg_out_confirm_lines(state, &confirm).into_iter().map(|(_, l)| l).collect();
    assert!(lines[0].contains("Ekle satırları dosyanın aynı adlı tablolarına ekler"), "{lines:#?}");
    assert!(
        lines.iter().any(|l| l.starts_with("Satır eklenen tablolar: ") && l.contains("Duraklar (dosyada 4 satır, en çok 7 eklenir)")),
        "{lines:#?}"
    );
    assert!(lines.iter().any(|l| l.starts_with("Tabloda sütunu olmayan alanlar yazılmaz")), "{lines:#?}");
    send(&mut app, out(Out::Confirm));
    assert!(app.exchange.is_none(), "{:?}", last(&app));
    assert_eq!(rows(&file), 7, "the three new points added");
    assert!(said(&app, "“teslim.gpkg” yazıldı: "), "{:?}", last(&app));
    let log: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(log.iter().any(|l| l.contains("“Duraklar” tablosuna 3 satır eklendi")), "{log:#?}");
    assert!(log.iter().any(|l| l.contains("“Duraklar” tablosunda 4 nesne zaten vardı")), "{log:#?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn durdur_stops_the_write_and_the_file_stays() {
    let dir = scratch("gpkg-durdur");
    let file = dir.join("teslim.gpkg");
    let mut app = app_with_drawing();
    let layer = stops(&mut app);
    points(&mut app, &layer, 0, 3);
    first_delivery(&mut app, &file);
    // More rows than four batches hold.
    points(&mut app, &layer, 3, 20_000);
    ask_again(&mut app);
    let before = std::fs::read(&file).expect("the file");
    let task = app.update(Message::Exchange(Box::new(out(Out::Confirm))));
    // The window waits for its write; Durdur asks it to stop.
    let Some(Window::GpkgOut(state)) = &app.exchange else { panic!("the window") };
    assert!(state.running.is_some(), "the write runs on its own thread");
    let _ = app.update(Message::Exchange(Box::new(out(Out::Stop))));
    let Some(Window::GpkgOut(state)) = &app.exchange else { panic!("the window") };
    assert!(state.running.as_ref().is_some_and(|r| r.stopping));
    drive(&mut app, task);
    let Some(Window::GpkgOut(state)) = &app.exchange else { panic!("the window stays") };
    assert!(state.running.is_none());
    assert_eq!(
        state.status,
        Some((false, "Dışa aktarma durduruldu; dosyaya hiçbir şey yazılmadı.".to_owned())),
        "{:?}",
        last(&app)
    );
    assert_eq!(std::fs::read(&file).expect("the file"), before, "the file as it was");
    // Geri, and the file written into as asked.
    send(&mut app, out(Out::Back));
    let Some(Window::GpkgOut(state)) = &app.exchange else { panic!("the window") };
    assert!(state.confirm.is_none() && state.status.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

/// The pictures (`.run/gui-evidence/kullanim-9191/`): the ways to write a file that is there, the rows added, the
/// foot while it writes with Durdur.
#[test]
#[ignore = "pictures for the owner"]
fn writing_into_screens() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let prepare = |way: GpkgWrite, running: bool| {
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = scratch(&format!("gpkg-yazim-resim-{n}"));
        let file = dir.join("teslim.gpkg");
        let mut app = app_with_drawing();
        let layer = stops(&mut app);
        points(&mut app, &layer, 0, 4);
        first_delivery(&mut app, &file);
        // The file's own table, kept by the first way.
        {
            use kentos_data::Driver as _;
            let source = kentos_data_gpkg::Gpkg.open(file.to_str().expect("utf-8")).expect("open");
            crate::data_layers_tests::point_table(
                &*source,
                "eski_tablo",
                &[("ad", kentos_data::FieldKind::Text { max: Some(8) })],
                &[vec![kentos_data::Value::Text("A".into())]],
            );
            drop(source);
            kentos_data::registry().forget(file.to_str().expect("utf-8"));
        }
        points(&mut app, &layer, 4, 3);
        ask_again(&mut app);
        send(&mut app, out(Out::Write(way)));
        if running && let Some(Window::GpkgOut(s)) = &mut app.exchange {
            s.running = Some(super::super::gpkg_out_run::Running {
                run: 0,
                path: file.clone(),
                done: 15_000,
                total: 20_007,
                stop: Arc::new(AtomicBool::new(false)),
                stopping: false,
            });
        }
        app
    };
    crate::files_testing::usage_shots("9191", "01-yalniz-bu-tablolar", || prepare(GpkgWrite::Tables, false));
    crate::files_testing::usage_shots("9191", "02-tablolara-ekle", || prepare(GpkgWrite::Append, false));
    crate::files_testing::usage_shots("9191", "03-dosyanin-yerine", || prepare(GpkgWrite::File, false));
    crate::files_testing::usage_shots("9191", "04-yaziliyor-durdur", || prepare(GpkgWrite::Append, true));
}
