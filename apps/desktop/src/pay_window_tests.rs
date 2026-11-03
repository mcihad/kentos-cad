//! Poz metrajı (pay_window.rs, plan/road.md Y11.4): the window reads the quantities as the book is typed and saves it,
//! one undo step; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::pay_window::Event as W;
use kentos_commands::PayMeasure;

/// A road project with `road.pay.query`'s drawing: guardrails, culverts, walls, volumes and a book.
fn app(with_book: bool) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.pay.query.json")).expect("cases");
    let mut setup = cases["setup"].clone();
    if !with_book {
        setup["settings"]["road"].as_object_mut().expect("road").remove("pay");
    }
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Pay(e));
}

#[test]
fn the_window_reads_and_saves_the_book() {
    let mut app = app(true);
    let _ = app.update(Message::Run("payItems.editor"));
    assert_eq!(app.dialog, Some(Dialog::Pay));
    let w = app.pay_window.as_ref().expect("open");
    let a = w.preview.answer.clone().unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    assert_eq!(a.rows.len(), 6);
    assert_eq!(a.unpriced, 2);
    // A price for the guardrail makes it count.
    send(&mut app, W::Item(2, 3, "40".into()));
    let priced = app.pay_window.as_ref().and_then(|w| w.preview.answer.clone()).expect("reads");
    assert_eq!(priced.unpriced, 1);
    assert!((priced.total - a.total - 40.0 * a.rows[2].quantity).abs() < 1e-6);
    send(&mut app, W::Field(0, " ".into()));
    let why = app.pay_window.as_ref().and_then(|w| w.preview.why.clone()).expect("refused");
    assert!(why.contains("kaynağı"), "{why}");
    send(&mut app, W::Field(0, "Yeni liste".into()));
    send(&mut app, W::Save);
    let saved = app.document.as_ref().expect("a drawing").model.road_pay().cloned().expect("a book");
    assert_eq!(saved.source, "Yeni liste");
    assert_eq!(saved.items[2].price, Some(40.0));
    assert_eq!(saved.revision, 2);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(app.document.as_ref().expect("a drawing").model.road_pay().map(|b| b.revision), Some(1));
}

#[test]
fn a_new_book_gets_its_items_and_rules() {
    let mut app = app(false);
    let _ = app.update(Message::Run("payItems.editor"));
    let why = app.pay_window.as_ref().and_then(|w| w.preview.why.clone()).expect("no source yet");
    assert!(why.contains("kaynağı"), "{why}");
    for (k, v) in [(0, "İdarenin listesi"), (1, "2026")] {
        send(&mut app, W::Field(k, v.into()));
    }
    for (k, v) in [(0, "A-1"), (1, "Duvar betonu"), (2, "m³"), (3, "120")] {
        send(&mut app, W::Item(0, k, v.into()));
    }
    send(&mut app, W::AddRule);
    send(&mut app, W::Measure(0, PayMeasure::WallConcrete));
    let a = app.pay_window.as_ref().and_then(|w| w.preview.answer.clone()).expect("reads");
    assert_eq!(a.rows[0].lines.len(), 2);
    assert!(a.total > 0.0);
    send(&mut app, W::Save);
    assert!(app.document.as_ref().expect("a drawing").model.road_pay().is_some());
}

/// The window, wide and narrow, light and dark; `.run/gui-evidence/yol-y11-poz-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib pay_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = app(true);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y11-poz-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("payItems.editor"));
        shot(&mut app, "defter");
    }
}
