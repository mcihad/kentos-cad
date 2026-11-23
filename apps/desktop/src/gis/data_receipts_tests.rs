//! A write whose commit's answer was lost (docs/adr/9126), on the desktop, through the lossy test source over a
//! GeoPackage: the status bar says the result is not known, neither success nor failure; the next Kaynağa yaz asks the
//! source's transaction records (the interface's “işlem kaydı”, docs/adr/9175) by the write's receipt and takes the row
//! it finds out of the drawing, so one drawn row stays one row, with the program closed and the project opened again
//! too; a source without them is not written again by itself, only by Yeniden gönder; İşlem kaydı tablosunu oluştur
//! makes their table; two objects drawn on the same place on purpose are two rows.

use kentos_data::lossy::{self, Loss};
use kentos_native_application::ExecutionContext;

use crate::app::{App, Message, Picker};
use crate::data_journal::Journals;
use crate::data_layers_tests::{node, parcels};
use crate::data_pending::{Event as Pending, SET_UP};
use crate::files_testing::{app_with_drawing, drive, scratch};

/// The rows of `parseller` as a reader of its own counts them.
fn rows(path: &std::path::Path) -> i64 {
    let gp = kentos_geopackage::GeoPackage::open(path, false).expect("the file");
    gp.connection().query_row("SELECT count(*) FROM parseller", [], |r| r.get(0)).expect("count")
}

/// A parcel drawn on `layer` at (x, y): a new row.
fn draw(app: &mut App, layer: &str, x: f64, y: f64) {
    let p = |dx: f64, dy: f64| serde_json::json!({ "x": x + dx, "y": y + dy });
    let input = serde_json::from_value(serde_json::json!({
        "layerId": layer,
        "objects": [{
            "geometry": { "kind": "polygon", "pts": [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)] },
            "attrs": { "ada": "300", "kullanim": "Yeni" }
        }]
    }))
    .expect("an input");
    let doc = app.document.as_mut().expect("a drawing");
    let done = kentos_native_application::create::execute(&mut ExecutionContext::new(&mut doc.model), input);
    assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
}

/// A drawing with the lossy source over a GeoPackage of three parcels as a data layer being edited; its receipts set
/// up when `receipts`.
fn app_over(tag: &str, receipts: bool) -> (App, std::path::PathBuf, String, Journals, std::path::PathBuf) {
    crate::data_layers::register();
    lossy::register();
    let path = parcels(tag, 3);
    let dir = scratch(tag);
    let journals = Journals::at(&dir.join("veri-oturumlari"));
    let mut app = app_with_drawing();
    app.gis.data_journal = journals.clone();
    let address = lossy::address_of(&path.display().to_string());
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")]).expect("added");
    let layer = node(&app, "parseller").expect("the layer").id.clone();
    let _ = app.data_edits_event(crate::data_edits::Event::Start(layer.clone()));
    if receipts {
        let data = node(&app, "parseller").and_then(|n| n.data.clone()).expect("a data layer");
        kentos_native_application::data_write::set_up_receipts(&data).expect("set up");
    }
    (app, path, layer, journals, dir.join("proje.kcad"))
}

fn kaynaga_yaz(app: &mut App) {
    let task = app.data_pending_event(Pending::Save(Vec::new()));
    drive(app, task);
}

fn said(app: &App) -> Vec<String> {
    app.log.lines().map(|l| l.text.clone()).collect()
}

#[test]
fn a_lost_answer_is_asked_by_its_receipt_and_the_row_is_not_written_twice() {
    let (mut app, path, layer, _, _) = app_over("kayit-bulunur", true);
    draw(&mut app, &layer, 486_400.0, 4_420_100.0);
    lossy::lose(&path.display().to_string(), Loss::AfterCommit);
    kaynaga_yaz(&mut app);
    assert_eq!(rows(&path), 4, "the commit was made");
    assert!(app.gis.data_edits.uncertain.get(&layer).is_some_and(|u| u.receipt.is_some()));
    assert_eq!(
        app.data_pending_words(0).map(|w| w.0).as_deref(),
        Some("Yazımın sonucu bilinmiyor: 1 katmanda 1 satır"),
        "neither success nor failure"
    );
    assert!(!said(&app).iter().any(|t| t.contains("kaynağa yazıldı")), "no success said: {:#?}", said(&app));
    // Kaynağa yaz again: the receipt found, the row taken out of the drawing, nothing written a second time.
    kaynaga_yaz(&mut app);
    assert_eq!(rows(&path), 4, "one row");
    assert!(app.data_pending_words(0).is_none(), "nothing waits");
    assert!(app.gis.data_edits.uncertain.is_empty());
    assert!(said(&app).iter().any(|t| t.starts_with("Önceki yazım kaynağa ulaşmıştı")), "{:#?}", said(&app));
}

#[test]
fn a_write_that_never_reached_the_source_is_written_on_the_next_try() {
    let (mut app, path, layer, _, _) = app_over("kayit-yok", true);
    draw(&mut app, &layer, 486_400.0, 4_420_100.0);
    lossy::lose(&path.display().to_string(), Loss::BeforeCommit);
    kaynaga_yaz(&mut app);
    assert_eq!(rows(&path), 3, "nothing written");
    kaynaga_yaz(&mut app);
    assert_eq!(rows(&path), 4, "written once, on the next try");
    assert!(app.data_pending_words(0).is_none());
}

#[test]
fn the_question_survives_the_program_closing_and_the_project_opened_again() {
    let (mut app, path, layer, journals, file) = app_over("kayit-yeniden-acilis", true);
    draw(&mut app, &layer, 486_400.0, 4_420_100.0);
    lossy::lose(&path.display().to_string(), Loss::AfterCommit);
    kaynaga_yaz(&mut app);
    assert_eq!(rows(&path), 4);
    // The project saved (its journal with the uncertain write), the program closed.
    app.picker = Picker::File(file.clone());
    let task = app.update(Message::Run("file.save"));
    drive(&mut app, task);
    drop(app);
    // Opened again: the write's question comes back; Kaynağa yaz asks, writes nothing again.
    let mut again = app_with_drawing();
    again.gis.data_journal = journals;
    let doc = crate::document::Document::read(&file).expect("the file reads");
    let _ = again.update(Message::Opened(Some(Ok(Box::new(doc)))));
    assert!(!again.gis.data_edits.uncertain.is_empty(), "{:#?}", said(&again));
    kaynaga_yaz(&mut again);
    assert_eq!(rows(&path), 4, "still one row");
    assert!(again.data_pending_words(0).is_none(), "{:#?}", said(&again));
}

#[test]
fn without_receipts_nothing_is_sent_again_until_yeniden_gonder() {
    let (mut app, path, layer, _, _) = app_over("kayitsiz-masaustu", false);
    draw(&mut app, &layer, 486_400.0, 4_420_100.0);
    lossy::lose(&path.display().to_string(), Loss::AfterCommit);
    kaynaga_yaz(&mut app);
    assert_eq!(rows(&path), 4);
    assert!(app.gis.data_edits.uncertain.get(&layer).is_some_and(|u| u.receipt.is_none()));
    // Kaynağa yaz does not send it again by itself.
    kaynaga_yaz(&mut app);
    assert_eq!(rows(&path), 4, "not sent again");
    assert!(said(&app).iter().any(|t| t.contains("kendiliğinden yeniden gönderilmedi")), "{:#?}", said(&app));
    // The owner looked at the source and sends it again at Yeniden gönder's second press.
    let _ = app.data_pending_event(Pending::Resend(layer.clone()));
    assert_eq!(rows(&path), 4, "the first press only asks");
    let task = app.data_pending_event(Pending::ResendConfirmed(layer));
    drive(&mut app, task);
    assert_eq!(rows(&path), 5, "sent again as the owner chose");
}

#[test]
fn two_objects_drawn_on_the_same_place_are_two_rows() {
    let (mut app, path, layer, _, _) = app_over("ayni-yer", true);
    draw(&mut app, &layer, 486_400.0, 4_420_100.0);
    draw(&mut app, &layer, 486_400.0, 4_420_100.0);
    kaynaga_yaz(&mut app);
    assert_eq!(rows(&path), 5);
}

/// A database that keeps no transaction records shows İşlem kaydı tablosunu oluştur in Katman değişiklikleri; the press
/// makes the table in the source and says so, and the button goes (docs/adr/9126, 9175).
#[test]
fn the_transaction_records_table_is_made_from_katman_degisiklikleri() {
    let (mut app, _path, layer, _, _) = app_over("kayit-tablosu", false);
    draw(&mut app, &layer, 486_400.0, 4_420_100.0);
    as_a_database_without_records(&mut app, &layer);
    let mut shot = crate::files_testing::pictured(iced::Size::new(1440., 900.));
    shot.settle(&mut app, App::view, &mut |app: &mut App, m| drop(app.update(m)));
    assert!(crate::files_testing::find_text(&mut shot, &app, SET_UP).is_some(), "the button is shown");
    let task = app.data_pending_event(Pending::SetUpReceipts(layer.clone()));
    drive(&mut app, task);
    let data = node(&app, "parseller").and_then(|n| n.data.clone()).expect("a data layer");
    assert_eq!(kentos_native_application::data_write::receipts_ready(&data), Ok(true), "the table is in the source");
    assert!(said(&app).iter().any(|t| t.contains("işlem kaydı tablosu oluşturuldu")), "{:#?}", said(&app));
    shot.settle(&mut app, App::view, &mut |app: &mut App, m| drop(app.update(m)));
    assert!(crate::files_testing::find_text(&mut shot, &app, SET_UP).is_none(), "made: the button goes");
}

/// Katman değişiklikleri open on `layer` as for a database that keeps no transaction records: the lossy file stands for
/// one here, since only a database's source is asked whether it keeps them (a file's commit has a sure answer).
fn as_a_database_without_records(app: &mut App, layer: &str) {
    let _ = app.data_pending_event(Pending::Open(Some(layer.to_owned())));
    let data = node(app, "parseller").and_then(|n| n.data.clone()).expect("a data layer");
    let known = kentos_native_application::data_write::receipts_ready(&data);
    assert_eq!(known, Ok(false), "no table yet");
    let _ = app.data_pending_event(Pending::Receipts(layer.to_owned(), known));
}

/// Pictures (docs/adr/9126, 9175): Katman değişiklikleri after a lost answer, with transaction records (the next
/// Kaynağa yaz asks) and without (Yeniden gönder…), and a database that keeps none with İşlem kaydı tablosunu oluştur;
/// light at 1440×900 and dark at 1100×760.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib data_receipts_tests::screens -- --ignored --nocapture"]
fn screens() {
    for (name, receipts) in [("01-islem-kayitli", true), ("02-islem-kayitsiz", false)] {
        crate::files_testing::usage_shots("9175", name, || {
            let (mut app, path, layer, _, _) = app_over(&format!("kayit-resim-{name}"), receipts);
            draw(&mut app, &layer, 486_400.0, 4_420_100.0);
            lossy::lose(&path.display().to_string(), Loss::AfterCommit);
            kaynaga_yaz(&mut app);
            let _ = app.data_pending_event(Pending::Open(Some(layer)));
            app
        });
    }
    crate::files_testing::usage_shots("9175", "03-islem-kaydi-tablosu", || {
        let (mut app, _path, layer, _, _) = app_over("kayit-resim-03", false);
        draw(&mut app, &layer, 486_400.0, 4_420_100.0);
        as_a_database_without_records(&mut app, &layer);
        app
    });
}
