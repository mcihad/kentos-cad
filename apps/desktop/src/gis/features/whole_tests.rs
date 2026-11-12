//! Kaynağın tümü (docs/adr/0591 §4): a layer bound by a part shows its source's every row, ordered by the source or
//! here, a page at a time; Sonraki sayfa goes on where the last page ended; a row the part holds selects its object.
//! A page the source does not give leaves the pages read and is asked again from its cursor (docs/adr/0740 §2).

use std::path::PathBuf;

use kentos_commands::CallOp;
use kentos_native_application::sources::trouble::Trouble;
use kentos_native_application::{ExecutionContext, registry};

use super::Event;
use super::whole::{PAGE, WholeEvent};
use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, scratch};
use crate::source_trouble::Refused;

const LAYER: &str = "import-olay";

fn ev(app: &mut App, e: WholeEvent) {
    let _ = app.update(Message::Features(Event::Whole(e)));
}

/// A drawing with `sira.gpkg`'s `olay` bound by a part (its first 120 points), the table open on it.
fn scene(name: &str) -> App {
    let file = scratch(name).join("sira.gpkg");
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sources/v1/sira.gpkg"),
        &file,
    )
    .expect("copied");
    let mut app = app_with_drawing();
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let answer = registry::run(
        &mut ExecutionContext::new(doc),
        "cad.sources.edit",
        CallOp::Execute,
        serde_json::json!({ "edit": { "kind": "link", "path": file.display().to_string(), "table": "olay",
                                      "extent": [486995.0, 4419995.0, 487595.0, 4420015.0] } }),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let _ = app.update(Message::Run("data.featureTable"));
    let _ = app.update(Message::Features(Event::Layer(LAYER.into())));
    app
}

/// What the thread asks, asked here and handed to the tab as the thread hands it.
pub(crate) fn answer(app: &mut App) {
    let (ticket, more, input) = app.features.whole.asked.clone().expect("a question");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let link = doc.settings().sources[0].clone();
    let told = Refused::of(
        kentos_native_application::sources_query::ask_source(
            doc.settings(),
            link.kind,
            &link.path,
            &link.table,
            LAYER,
            &input,
            doc.revision().to_string(),
            &|| false,
        ),
        "durduruldu",
    );
    ev(app, WholeEvent::Answered(ticket, more, Box::new(told)));
}

/// The bound file's path.
fn file_of(app: &App) -> PathBuf {
    PathBuf::from(
        &app.document
            .as_ref()
            .expect("a drawing")
            .model
            .settings()
            .sources[0]
            .path,
    )
}

fn keys(app: &App) -> Vec<i64> {
    app.features
        .whole
        .rows
        .iter()
        .map(|r| r.key.expect("a key"))
        .collect()
}

#[test]
fn the_whole_source_is_read_in_pages_in_the_order_asked() {
    let mut app = scene("tablo-tumu");
    let held = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .by_layer_with_uids(LAYER)
        .count();
    assert_eq!(held, 120);
    ev(&mut app, WholeEvent::Toggle(true));
    assert!(app.features.whole.running.is_some());
    answer(&mut app);
    assert_eq!(app.features.whole.rows.len(), PAGE as usize);
    assert_eq!(keys(&app)[..3], [1, 2, 3]);
    let last = app.features.whole.last.clone().expect("a page");
    assert_eq!(last.total, Some(2400));
    assert_eq!(last.plan.order, "key");
    // Sonraki sayfa goes on from the key.
    ev(&mut app, WholeEvent::More);
    answer(&mut app);
    assert_eq!(keys(&app)[PAGE as usize], i64::from(PAGE) + 1);
    // A header orders by the source: Sıra, then the columns as the layer's fields.
    let columns = app.whole_columns(LAYER);
    let puan = columns.iter().position(|c| c == "puan").expect("puan") + 1;
    ev(&mut app, WholeEvent::Sort(puan));
    answer(&mut app);
    let last = app.features.whole.last.clone().expect("a page");
    assert_eq!(last.plan.order, "column");
    assert!(last.next.as_deref().is_some_and(|n| n.starts_with("o:[")));
    let values = |app: &App| -> Vec<f64> {
        app.features
            .whole
            .rows
            .iter()
            .filter_map(|r| r.attrs.get("puan").and_then(|v| v.parse().ok()))
            .collect()
    };
    ev(&mut app, WholeEvent::More);
    answer(&mut app);
    let v = values(&app);
    assert_eq!(v.len(), 2 * PAGE as usize);
    assert!(v.windows(2).all(|w| w[0] <= w[1]), "artan değil");
    // Pressed again: descending.
    ev(&mut app, WholeEvent::Sort(puan));
    answer(&mut app);
    let v = values(&app);
    assert!(v.windows(2).all(|w| w[0] >= w[1]), "azalan değil");
    // A text column is ordered here.
    let ad = columns.iter().position(|c| c == "ad").expect("ad") + 1;
    ev(&mut app, WholeEvent::Sort(ad));
    answer(&mut app);
    assert_eq!(
        app.features.whole.last.as_ref().expect("a page").plan.order,
        "here"
    );
    // The filter goes with the question.
    let _ = app.update(Message::Features(Event::Filter("sayi < 0".into())));
    answer(&mut app);
    let last = app.features.whole.last.clone().expect("a page");
    assert!(last.plan.pushed.is_some());
    assert!(
        app.features
            .whole
            .rows
            .iter()
            .all(|r| r.attrs["sayi"].starts_with('-'))
    );
    // A row the part holds selects its object; one it does not hold selects nothing.
    let first_held = app
        .features
        .whole
        .rows
        .iter()
        .position(|r| r.key.is_some_and(|k| k <= 120))
        .expect("a held row");
    ev(&mut app, WholeEvent::Press(first_held));
    assert_eq!(app.selection.ids().len(), 1);
    // Çalışma kopyası leaves the source.
    ev(&mut app, WholeEvent::Toggle(false));
    assert!(app.features.whole.layer.is_none() && app.features.whole.rows.is_empty());
}

#[test]
fn an_older_answer_is_dropped_and_a_bad_filter_is_said() {
    let mut app = scene("tablo-tumu-eski");
    ev(&mut app, WholeEvent::Toggle(true));
    let (old, ..) = app.features.whole.asked.clone().expect("a question");
    let _ = app.update(Message::Features(Event::Filter("puan >".into())));
    ev(
        &mut app,
        WholeEvent::Answered(
            old,
            false,
            Box::new(Err(Refused {
                code: "x".into(),
                why: "eski".into(),
            })),
        ),
    );
    assert!(app.features.whole.missed.is_none(), "eski cevap alındı");
    answer(&mut app);
    let (page, r) = app.features.whole.missed.clone().expect("a problem");
    assert_eq!(page, 1);
    assert!(r.why.starts_with("Süzgeç:"), "{}", r.why);
    // A filter that does not compile is helped by another filter, not by asking again.
    assert_eq!(r.way(), None);
}

/// A page the source does not give (docs/adr/0740 §2): the pages read stay, the page is said with its way out, and
/// the same question asked again once the file is back goes on from its cursor: the rows are the table's in key
/// order, none twice, none skipped. The first page refused says so in place of the rows; Durdur's page is asked again
/// with Sürdür.
#[test]
fn a_page_not_read_stays_said_and_is_asked_again() {
    let mut app = scene("tablo-tumu-sorun");
    let file = file_of(&app);
    let away = file.with_file_name("uzakta.gpkg");
    ev(&mut app, WholeEvent::Toggle(true));
    answer(&mut app);
    assert_eq!(app.features.whole.pages, 1);
    std::fs::rename(&file, &away).expect("moved away");
    ev(&mut app, WholeEvent::More);
    let (_, _, asked) = app.features.whole.asked.clone().expect("a question");
    answer(&mut app);
    let (page, r) = app.features.whole.missed.clone().expect("not read");
    assert_eq!((page, r.code.as_str()), (2, "file_not_found"));
    assert_eq!(r.way(), Some(Trouble::Relink));
    assert_eq!(
        app.features.whole.rows.len(),
        PAGE as usize,
        "the first page stays"
    );
    assert!(
        app.features
            .whole
            .last
            .as_ref()
            .is_some_and(|l| l.next.is_some())
    );
    // Back: the same question, the same cursor and stamp.
    std::fs::rename(&away, &file).expect("moved back");
    ev(&mut app, WholeEvent::Retry);
    let (_, more, again) = app.features.whole.asked.clone().expect("asked again");
    assert!(more);
    assert_eq!(again, asked);
    assert!(app.features.whole.missed.is_none());
    answer(&mut app);
    assert_eq!(keys(&app), (1..=2 * i64::from(PAGE)).collect::<Vec<_>>());
    assert_eq!(app.features.whole.pages, 2);
    // The first page refused: nothing read, said in place of the rows.
    ev(&mut app, WholeEvent::Toggle(false));
    std::fs::rename(&file, &away).expect("moved away");
    ev(&mut app, WholeEvent::Toggle(true));
    answer(&mut app);
    assert!(app.features.whole.rows.is_empty());
    assert_eq!(
        app.features
            .whole
            .missed
            .as_ref()
            .map(|(p, r)| (*p, r.code.clone())),
        Some((1, "file_not_found".into()))
    );
    std::fs::rename(&away, &file).expect("moved back");
    ev(&mut app, WholeEvent::Retry);
    answer(&mut app);
    assert_eq!(app.features.whole.rows.len(), PAGE as usize);
    // Durdur: the page is not read and Sürdür asks it again.
    ev(&mut app, WholeEvent::More);
    let (_, _, stopped) = app.features.whole.asked.clone().expect("a question");
    ev(&mut app, WholeEvent::Stop);
    assert!(app.features.whole.stopped && app.features.whole.running.is_none());
    ev(&mut app, WholeEvent::Retry);
    let (_, _, again) = app.features.whole.asked.clone().expect("asked again");
    assert_eq!(again, stopped);
    assert!(!app.features.whole.stopped);
    answer(&mut app);
    assert_eq!(keys(&app), (1..=2 * i64::from(PAGE)).collect::<Vec<_>>());
}

/// A bound layer whose file moved (docs/adr/0740 §2): its page is refused; Yeniden bağla… links the layer to the
/// file's new place and asks the page again, which goes on from its cursor there.
#[test]
fn a_relinked_layer_asks_again_what_its_file_did_not_give() {
    let mut app = scene("tablo-tumu-yeniden-bagla");
    let file = file_of(&app);
    let moved = file.with_file_name("tasindi.gpkg");
    ev(&mut app, WholeEvent::Toggle(true));
    answer(&mut app);
    std::fs::rename(&file, &moved).expect("moved");
    ev(&mut app, WholeEvent::More);
    answer(&mut app);
    assert!(app.features.whole.missed.is_some());
    let id = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .settings()
        .sources[0]
        .id
        .clone();
    let _ = app.update(Message::Bound(crate::bound::Event::Relinked(
        id,
        Some(moved.clone()),
    )));
    assert_eq!(file_of(&app), moved, "relinked");
    assert!(app.features.whole.running.is_some(), "asked again");
    answer(&mut app);
    assert!(app.features.whole.missed.is_none());
    assert_eq!(keys(&app), (1..=2 * i64::from(PAGE)).collect::<Vec<_>>());
}

#[test]
#[ignore = "pictures, run by hand"]
fn whole_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height, mode) in [
        (1440.0, 900.0, "dark"),
        (1100.0, 650.0, "dark"),
        (1440.0, 900.0, "light"),
    ] {
        let mut app = scene(&format!("tablo-tumu-{mode}-{width}"));
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(Size::new(width, height)));
        let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
        let mut picture = |app: &mut App, label: &str| {
            snapshot.settle(app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            let path = out.join(format!("tablo-tumu-{mode}-{label}-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        };
        picture(&mut app, "1-kopya");
        ev(&mut app, WholeEvent::Toggle(true));
        answer(&mut app);
        picture(&mut app, "2-tumu");
        let columns = app.whole_columns(LAYER);
        let puan = columns.iter().position(|c| c == "puan").expect("puan") + 1;
        ev(&mut app, WholeEvent::Sort(puan));
        answer(&mut app);
        ev(&mut app, WholeEvent::Sort(puan));
        answer(&mut app);
        picture(&mut app, "3-azalan");
        let ad = columns.iter().position(|c| c == "ad").expect("ad") + 1;
        ev(&mut app, WholeEvent::Sort(ad));
        answer(&mut app);
        let _ = app.update(Message::Features(Event::Filter("sayi < 0".into())));
        answer(&mut app);
        ev(&mut app, WholeEvent::More);
        answer(&mut app);
        picture(&mut app, "4-burada-suzgec");
        // A page the moved file did not give, the first page refused, a page stopped (docs/adr/0740 §2).
        let _ = app.update(Message::Features(Event::Filter(String::new())));
        ev(&mut app, WholeEvent::Sort(0));
        answer(&mut app);
        let file = file_of(&app);
        let away = file.with_file_name("uzakta.gpkg");
        std::fs::rename(&file, &away).expect("moved away");
        ev(&mut app, WholeEvent::More);
        answer(&mut app);
        picture(&mut app, "6-sayfa-okunamadi");
        ev(&mut app, WholeEvent::Toggle(false));
        ev(&mut app, WholeEvent::Toggle(true));
        answer(&mut app);
        picture(&mut app, "7-ilk-sayfa");
        std::fs::rename(&away, &file).expect("moved back");
        ev(&mut app, WholeEvent::Retry);
        answer(&mut app);
        ev(&mut app, WholeEvent::More);
        ev(&mut app, WholeEvent::Stop);
        picture(&mut app, "8-durduruldu");
        // The slider's time on the layer (docs/adr/0595 §4): the rows of its month, asked of the source.
        let _ = app.update(Message::Features(Event::Filter(String::new())));
        let doc = &mut app.document.as_mut().expect("a drawing").model;
        let _ = registry::run(
            &mut ExecutionContext::new(doc),
            "cad.layers.time",
            CallOp::Execute,
            serde_json::json!({ "layer": LAYER, "time": { "start": "tarih" } }),
        );
        let _ = app.update(Message::Run("time.slider"));
        ev(&mut app, WholeEvent::Toggle(false));
        ev(&mut app, WholeEvent::Toggle(true));
        answer(&mut app);
        picture(&mut app, "5-zaman");
    }
}

/// The slider's time on a temporal bound layer (docs/adr/0595 §4): Kaynağın tümü asks the source for the rows of the
/// slider's moment, every row it shows is of it, and moving the slider asks again.
#[test]
fn the_whole_source_is_read_at_the_sliders_time() {
    let mut app = scene("tablo-tumu-zaman");
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let set = registry::run(
        &mut ExecutionContext::new(doc),
        "cad.layers.time",
        CallOp::Execute,
        serde_json::json!({ "layer": LAYER, "time": { "start": "tarih" } }),
    )
    .expect("runs");
    assert_eq!(set["status"], "completed", "{set}");
    let _ = app.update(Message::Run("time.slider"));
    assert!(app.time.slider.open, "the slider opens");
    ev(&mut app, WholeEvent::Toggle(true));
    let (_, _, first) = app.features.whole.asked.clone().expect("a question");
    let time = first.time.clone().expect("the slider's time");
    assert_eq!(
        time.fields.as_ref().map(|f| f.start.as_str()),
        Some("tarih")
    );
    answer(&mut app);
    let last = app.features.whole.last.clone().expect("a page");
    assert!(
        last.plan.local.iter().any(|l| l == "time"),
        "{:?}",
        last.plan
    );
    let window = app.time.slider.window().expect("a window");
    for r in &app.features.whole.rows {
        let at = r
            .attrs
            .get("tarih")
            .map_or(kentos_geometry_core::time::Read::Empty, |t| {
                kentos_geometry_core::time::read(t)
            });
        if let kentos_geometry_core::time::Read::Moment(t) = at {
            let shown = kentos_geometry_core::time::Time {
                s: t,
                e: t,
                mode: kentos_geometry_core::time::Mode::Instant,
            };
            assert!(kentos_geometry_core::time::shows(&shown, &window), "{r:?}");
        }
    }
    // The slider moves: the question is asked again at its new time.
    let step = if app.time.slider.position > 0 {
        crate::temporal::Event::First
    } else {
        crate::temporal::Event::Next
    };
    let _ = app.update(Message::Time(step));
    let (_, _, again) = app.features.whole.asked.clone().expect("a question");
    assert_ne!(again.time, first.time);
}
