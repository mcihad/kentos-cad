//! Closing Mimari's editors (docs/adr/3006): a click beside them, × and Esc ask when they hold unsaved changes and
//! close at once when they do not; F1 closes on Esc over its search box; Çizimden seç steps a window aside and brings
//! it back.

use kentos_commands::{CadWork, Workspace};
use kentos_domain::Slot;

use crate::app::{App, Dialog, Message};
use crate::files_testing::app_with_drawing;
use crate::unsaved_close::Answer;

/// A building's drawing open in KentOS Mimari.
fn app() -> App {
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let mut s = doc.settings().clone();
    s.workspace = Some(Workspace::Cad);
    s.cad_work = Some(CadWork::Architecture);
    doc.model.set_settings(s);
    app.program = Some(kentos_launch::Module::Architecture);
    app
}

#[test]
fn an_untouched_editor_closes_at_once() {
    let mut app = app();
    app.open_wall_styles();
    let _ = app.update(Message::WallStyles(crate::wall_styles::Event::Close));
    assert!(app.wall_styles.is_none() && app.dialog.is_none(), "nothing changed: no question");
    // Bina programı compares with what it would write untouched: its numbers as text do not count as a change.
    app.open_program();
    assert_eq!(app.dialog, Some(Dialog::Program));
    let _ = app.update(Message::Program(crate::program_window::Event::Close));
    assert!(app.program_window.is_none() && app.dialog.is_none(), "an untouched program closes");
}

#[test]
fn a_changed_editor_asks_and_vazgec_keeps_it() {
    let mut app = app();
    app.open_wall_styles();
    let _ = app.update(Message::WallStyles(crate::wall_styles::Event::New));
    let _ = app.update(Message::WallStyles(crate::wall_styles::Event::Close));
    let w = app.wall_styles.as_ref().expect("still open");
    assert!(w.closing(), "a new style unsaved: the question");
    let _ = app.update(Message::WallStyles(crate::wall_styles::Event::Answer(Answer::Stay)));
    let w = app.wall_styles.as_ref().expect("Vazgeç keeps the window");
    assert!(!w.closing());
    // Esc is Kapat: it asks again; over the question, Esc is Vazgeç.
    app.close_dialog();
    assert!(app.wall_styles.as_ref().is_some_and(|w| w.closing()), "Esc asks");
    assert_eq!(app.dialog, Some(Dialog::WallStyles));
    app.close_dialog();
    assert!(app.wall_styles.as_ref().is_some_and(|w| !w.closing()), "Esc over the question stays");
    let _ = app.update(Message::WallStyles(crate::wall_styles::Event::Close));
    let _ = app.update(Message::WallStyles(crate::wall_styles::Event::Answer(Answer::Discard)));
    assert!(app.wall_styles.is_none() && app.dialog.is_none(), "Kaydetmeden kapat");
}

#[test]
fn kaydet_ve_kapat_writes_then_closes() {
    let mut app = app();
    app.open_program();
    let _ = app.update(Message::Program(crate::program_window::Event::AddUnit));
    let _ = app.update(Message::Program(crate::program_window::Event::Close));
    assert!(app.program_window.as_ref().is_some_and(|w| w.closing()), "a unit added: the question");
    let _ = app.update(Message::Program(crate::program_window::Event::Answer(Answer::Save)));
    assert!(app.program_window.is_none() && app.dialog.is_none(), "saved and closed");
    let program = app.document.as_ref().unwrap().model.settings().arch.as_ref().map(|a| a.program.clone());
    assert_eq!(program.map(|p| p.units.len()), Some(1), "the unit was written");
}

#[test]
fn esc_closes_f1_over_its_search_box() {
    let mut app = app();
    let _ = app.open_shortcuts();
    assert_eq!(app.dialog, Some(Dialog::Shortcuts));
    let _ = app.update(Message::CapturedEscape);
    assert_eq!(app.dialog, None, "Esc in the search box closes F1 (plan G11)");
}

#[test]
fn cizimden_sec_steps_the_window_aside_and_brings_it_back() {
    let mut app = app();
    app.open_room_modules();
    assert_eq!(app.dialog, Some(Dialog::RoomModules));
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::RoomModules(crate::room_modules_window::Event::PickFromDrawing));
    assert_eq!(app.dialog, None, "the window steps aside");
    assert_eq!(app.session.tool_id(), kentos_interaction::pick_objects::ID);
    assert!(app.room_modules.is_some(), "it keeps what it held");
    // Esc: the selection before comes back with the window.
    app.selection.set([Slot(2)]);
    assert!(app.window_picked(false));
    assert_eq!(app.dialog, Some(Dialog::RoomModules));
    assert_eq!(app.selection.ids(), &[Slot(1)]);
    // Enter keeps what was picked.
    let _ = app.update(Message::RoomModules(crate::room_modules_window::Event::PickFromDrawing));
    app.selection.set([Slot(2)]);
    assert!(app.window_picked(true));
    assert_eq!(app.selection.ids(), &[Slot(2)]);
    assert!(!app.window_picked(true), "no window waits now");
}
