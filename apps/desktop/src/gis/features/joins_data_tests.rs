//! A join view of two data layers (docs/adr/9134), the municipal report's case: `mahalleler` joined with `bakim` by
//! `mahalle`. Its rows are the sources' (read off the interface's thread; the view says it waits for them): M1/B01,
//! M1/B02, M2/B03, M2/B04 and M3 without a match, five; four with only the matched kept. A source that cannot be read,
//! a layer without a record and no match at all are each said.

use super::Event;
use crate::app::{App, Message};
use crate::data_layers_tests::{municipal, node};
use crate::files_testing::{app_with_drawing, drive};
use crate::gis;
use crate::joins_window::Event as W;
use kentos_native_application::joins_query::{SourceRows, read_with};

fn window(app: &mut App, e: W) {
    let task = app.update(Message::Gis(gis::Event::Joins(e)));
    drive(app, task);
}

/// A new view of `mahalleler` with `bakim` by `mahalle`, saved through Birleşik görünümler; its id.
fn view(app: &mut App, name: &str, matched: bool) -> String {
    let (left, right) = (
        node(app, "mahalleler").expect("mahalleler").id.clone(),
        node(app, "bakim").expect("bakim").id.clone(),
    );
    let task = app.update(Message::Run("data.joinViews"));
    drive(app, task);
    window(app, W::New);
    window(app, W::Name(name.into()));
    window(app, W::Layer(left));
    window(app, W::Key("mahalle".into()));
    window(app, W::With(right));
    window(app, W::WithKey("mahalle".into()));
    if matched {
        window(app, W::Matched);
    }
    window(app, W::Save);
    let views = &app.document.as_ref().expect("open").model.settings().join_views;
    views.iter().find(|v| v.name == name).expect("saved").id.clone()
}

/// The view read until its sources' rows are in: each reading asks for them, read as a frame of the program would.
fn settle(app: &mut App) {
    for _ in 0..3 {
        let _ = read(app);
        let task = app.data_rows_tasks();
        drive(app, task);
    }
}

fn read(app: &App) -> std::rc::Rc<kentos_native_application::joins_query::JoinRead> {
    let doc = app.document.as_ref().expect("open");
    let v = app.join_view(doc).expect("the view");
    app.join_read(doc, &v)
}

/// Each row's neighbourhood name and its maintenance code (empty without a match).
fn pairs(app: &App) -> Vec<(String, String)> {
    let r = read(app);
    let at = |name: &str| r.columns.iter().position(|c| c.name == name).unwrap_or_else(|| panic!("no {name}"));
    let (ad, kod) = (at("ad"), at("kod"));
    r.rows
        .iter()
        .map(|row| (row.cells[ad].clone().unwrap_or_default(), row.cells[kod].clone().unwrap_or_default()))
        .collect()
}

fn opened(tag: &str) -> (App, std::path::PathBuf) {
    crate::data_layers::register();
    let path = municipal(tag);
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![
        crate::data_layers::describe(&address, "bakim").expect("described"),
        crate::data_layers::describe(&address, "mahalleler").expect("described"),
    ])
    .expect("added");
    let task = app.update(Message::Run("data.featureTable"));
    drive(&mut app, task);
    (app, path)
}

#[test]
fn a_view_of_two_data_layers_reads_their_sources_rows() {
    let (mut app, path) = opened("birlesim-veri");
    let all = view(&mut app, "Mahalle bakımları", false);
    // Chosen: its rows asked of the sources, the view waiting for them (the message's tasks not run yet).
    let task = app.update(Message::Gis(gis::Event::Features(Event::View(all))));
    let r = read(&app);
    assert!(r.waiting.as_deref().is_some_and(|w| w.ends_with("kaynaktan okunuyor…")), "{:?}", r.waiting);
    drive(&mut app, task);
    let task = app.data_rows_tasks();
    drive(&mut app, task);
    // Both tables asked at once: read together.
    assert_eq!(read(&app).waiting, None);
    let pair = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    assert_eq!(
        pairs(&app),
        [pair("Çınar", "B01"), pair("Çınar", "B02"), pair("İstasyon", "B03"), pair("İstasyon", "B04"), pair("Kale", "")]
    );
    let report = read(&app).report.clone();
    assert_eq!((report.records, report.matched, report.unmatched), (3, 2, 1));
    assert_eq!((report.with_records, report.with_used), (6, 4));
    // A data layer's cells are read only from the view, said why.
    let r = read(&app);
    let kod = r.columns.iter().find(|c| c.name == "kod").expect("kod");
    assert!(!kod.editable && kod.reason.as_deref().is_some_and(|w| w.contains("veri katmanının satırı")), "{kod:?}");
    // Only the matched: four.
    let matched = view(&mut app, "Eşleşen bakımlar", true);
    let task = app.update(Message::Gis(gis::Event::Features(Event::View(matched))));
    drive(&mut app, task);
    settle(&mut app);
    assert_eq!(pairs(&app).len(), 4);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_source_that_cannot_be_read_and_no_match_are_said_apart() {
    let (mut app, path) = opened("birlesim-hata");
    let id = view(&mut app, "Mahalle bakımları", true);
    let task = app.update(Message::Gis(gis::Event::Features(Event::View(id))));
    drive(&mut app, task);
    settle(&mut app);
    assert_eq!(pairs(&app).len(), 4);
    // No neighbourhood's code is any maintenance's: no match, said so, not as an empty layer.
    let doc = app.document.as_ref().expect("open");
    let mut v = app.join_view(doc).expect("the view");
    v.with_key = "kod".into();
    let r = kentos_native_application::joins_query::read(&doc.model, &v);
    assert_eq!((r.report.records, r.report.matched, r.rows.len()), (3, 0, 0));
    let said = App::join_empty(&r, &v, doc, "bakim");
    assert!(said.starts_with("Hiçbir kayıt eşleşmedi"), "{said}");
    // A layer without a record says that instead.
    let v = app.join_view(doc).expect("the view");
    let none = |_: &kentos_commands::LayerNode| SourceRows::Ready(std::sync::Arc::new(Vec::new()));
    let r = read_with(&doc.model, &v, &none);
    assert_eq!(r.report.records, 0);
    let said = App::join_empty(&r, &v, doc, "bakim");
    assert_eq!(said, "“mahalleler” katmanında kayıt yok; görünümde satır yok.");
    // A source that cannot be read breaks the view, said why, with no former reading.
    let lost = |_: &kentos_commands::LayerNode| SourceRows::Failed("bağlantı kesildi".into());
    let r = read_with(&doc.model, &v, &lost);
    let why = r.broken.clone().expect("broken");
    assert!(why.contains("veri katmanının kaynağı okunamadı: bağlantı kesildi"), "{why}");
    assert!(r.rows.is_empty());
    let _ = std::fs::remove_file(&path);
}

/// Pictures (docs/adr/9134), light at 1440×900 and dark at 1100×760: the view of `mahalleler` with `bakim`, its five
/// rows from the sources; and the same with only the matched, four.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib joins_data_tests::screens -- --ignored --nocapture"]
fn screens() {
    let prepare = |matched: bool| {
        move || {
            let (mut app, _) = opened(&format!("birlesim-resim-{matched}"));
            let id = view(&mut app, if matched { "Eşleşen bakımlar" } else { "Mahalle bakımları" }, matched);
            let task = app.update(Message::Gis(gis::Event::Features(Event::View(id))));
            drive(&mut app, task);
            settle(&mut app);
            app
        }
    };
    crate::files_testing::usage_shots("9134", "01-bes-satir", prepare(false));
    crate::files_testing::usage_shots("9134", "02-yalniz-eslesen", prepare(true));
}
