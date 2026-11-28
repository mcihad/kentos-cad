//! Tabs and leaving (the second round's T10 and the owner's word of 27 November 2026, docs/adr/9347): drawing A with
//! an unsaved change; B opened in a new tab without a question, A kept in its tab with its change. Aç asks about the
//! drawing on screen only once a file is chosen, and Vazgeç leaves it as it was. Closing the window with an unsaved
//! drawing set aside asks a question naming it, not a line in the history while the window stays.

use crate::app::{Dialog, Message, Then};
use crate::files_testing::{app_with_drawing, drive, saved, scratch};

/// A line written on the drawing on screen: an unsaved change.
fn change(app: &mut crate::app::App) {
    let layer = app
        .document
        .as_ref()
        .map(|d| d.model.layers().active().to_owned())
        .expect("a drawing");
    let out = crate::files_testing::command(
        app,
        "cad.entities.create",
        serde_json::json!({ "layerId": layer, "objects": [{ "geometry": { "kind": "line",
            "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 5.0, "y": 0.0 } }, "attrs": {} }] }),
    );
    assert_eq!(out["status"], "completed", "{out}");
}

#[test]
fn a_new_tab_asks_nothing_aç_asks_after_the_file_and_closing_names_the_unsaved_tabs() {
    let dir = scratch("sekmeler");
    let b = saved(&dir, "b", 0);
    let c = saved(&dir, "c", 0);
    let mut app = app_with_drawing();
    change(&mut app);
    let a = app.document.as_ref().map(|d| d.session).expect("A");
    assert!(app.document.as_ref().is_some_and(|d| d.dirty()));
    // Yeni sekmede aç: B opens beside A, no question.
    let task = app.update(Message::Opening(crate::opening::Event::PickedTab(Some(
        b.clone(),
    ))));
    drive(&mut app, task);
    // No question about A (a project type to choose for the bare file may open; not this test's).
    assert!(
        !matches!(app.dialog, Some(Dialog::Unsaved(_))),
        "{:?}",
        app.dialog
    );
    app.dialog = None;
    assert_ne!(
        app.document.as_ref().map(|d| d.session),
        Some(a),
        "B in front"
    );
    let kept = app.doc_tabs.find_mut(a).expect("A in its tab");
    assert!(kept.dirty(), "A keeps its change");
    // Aç on a changed B: the question comes after C is chosen, and Vazgeç leaves B as it was.
    change(&mut app);
    let front = app.document.as_ref().map(|d| d.session);
    let _ = app.run("file.open");
    assert!(
        app.dialog.is_none(),
        "nothing asked before a file is chosen"
    );
    let _ = app.update(Message::Opening(crate::opening::Event::Picked(Some(
        c.clone(),
    ))));
    assert_eq!(app.dialog, Some(Dialog::Unsaved(Then::Open)));
    let _ = app.update(Message::DialogClosed);
    assert!(app.open_after_leave.is_none());
    assert_eq!(
        app.document.as_ref().map(|d| d.session),
        front,
        "the drawing stays"
    );
    // Closing the window: one question naming both unsaved drawings.
    let _ = app.update(Message::CloseRequested(iced::window::Id::unique()));
    let Some(Dialog::Unsaved(then @ Then::Close(_))) = app.dialog.clone() else {
        panic!("{:?}", app.dialog);
    };
    let q = app.unsaved_question(then);
    assert!(
        q.message
            .starts_with("2 çizimde kaydedilmemiş değişiklik var:"),
        "{}",
        q.message
    );
    assert_eq!(q.confirm, "Kaydetmeden çık");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The picture (`.run/gui-evidence/kullanim-9347/02-kapatma-sorusu`): the window closed with two unsaved drawings.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib doc_tabs_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    let dir = scratch("sekmeler-resim");
    let b = saved(&dir, "ikinci.kcad", 0);
    crate::files_testing::usage_shots("9347", "02-kapatma-sorusu", || {
        let mut app = app_with_drawing();
        change(&mut app);
        let task = app.update(Message::Opening(crate::opening::Event::PickedTab(Some(
            b.clone(),
        ))));
        drive(&mut app, task);
        app.dialog = None;
        change(&mut app);
        let _ = app.update(Message::CloseRequested(iced::window::Id::unique()));
        app
    });
}
