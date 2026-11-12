//! The window under what comes over it (view.rs): a menu, a dialog or a
//! panel coming over the window leaves the window's widgets as they were (an
//! open menu, a field, the drawing area's size), and a window that opens over
//! the drawing takes the keyboard from the command line under it (app.rs).

use iced::Size;

use crate::app::{App, Dialog, Message};
use crate::files_testing::{app_with_drawing, offscreen};
use crate::viewport;

/// Lays the app out until it rests; how many times the drawing area said its
/// size meanwhile (it says it when its own state is new).
fn settle(app: &mut App, snapshot: &mut kentos_ui::snapshot::Snapshot) -> usize {
    let mut said = 0;
    let mut update = |app: &mut App, message: Message| {
        if matches!(message, Message::Viewport(viewport::Event::Resized(_))) {
            said += 1;
        }
        let _ = app.update(message);
    };
    snapshot.settle(app, App::view, &mut update);
    said
}

/// The application menu over the window: the drawing area keeps its state
/// and says nothing again. Before, the window alone was the view and a layer
/// over it made the view a stack: iced built the window's widgets anew (an
/// open menu closed, the drawing area said its size again), and the rasters'
/// panel did so by itself, now and then (rasters/jobs.rs).
#[test]
fn a_menu_over_the_window_leaves_the_windows_widgets_as_they_were() {
    let size = Size::new(1440.0, 900.0);
    let mut app = app_with_drawing();
    let _ = app.update(Message::WindowResized(size));
    let mut snapshot = offscreen(size);
    assert!(
        settle(&mut app, &mut snapshot) > 0,
        "the drawing area says its size"
    );
    let _ = app.update(Message::AppMenu(crate::app_menu::Event::Toggle));
    assert!(app.app_menu.is_some());
    assert_eq!(settle(&mut app, &mut snapshot), 0, "nothing built anew");
    let _ = app.update(Message::AppMenu(crate::app_menu::Event::Toggle));
    assert!(app.app_menu.is_none());
    assert_eq!(settle(&mut app, &mut snapshot), 0, "nor when it goes");
}

/// A window opened from the command line has the keyboard: the line lets go
/// of it, so Enter and Space reach the window, not the line under it.
#[test]
fn a_window_opened_from_the_command_line_takes_its_keyboard() {
    let mut app = app_with_drawing();
    let _ = app.update(Message::CommandFocus(true));
    assert!(app.line_focused);
    let _ = app.update(Message::CommandRun("layer.list".into()));
    assert!(matches!(app.dialog, Some(Dialog::LayerList)));
    assert!(!app.line_focused, "the line lets go");
    // Back in the line while the window is open, the line keeps it.
    let _ = app.update(Message::CommandFocus(true));
    let _ = app.update(Message::CommandInput("çiz".into()));
    assert!(app.line_focused);
}
