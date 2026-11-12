//! Transparent tools on the desktop (docs/adr/0283): Kaydır, Pencere
//! yakınlaştır and Mesafe ölç started from the ribbon while a command runs
//! run over it; the wheel, Tümünü göster and Önceki görünüm move the view
//! under it. Each time, the command is back with its points and prompt, and
//! its preview is where the pointer is in the view it came back to.

use iced::{Point, Rectangle, Size};
use kentos_interaction::{Level, Vec2};

use crate::app::{App, Message};
use crate::files_testing::app_with_drawing;
use crate::viewport::Event;

fn run(app: &mut App, id: &'static str) {
    let _ = app.update(Message::Run(id));
}

fn viewport(app: &mut App, event: Event) {
    let _ = app.update(Message::Viewport(event));
}

fn click(app: &mut App, at: Point) {
    viewport(app, Event::Moved(at));
    viewport(app, Event::Pressed(at));
    viewport(app, Event::Released(at));
}

/// The sample on a 1000 × 800 area, snapping off, Çoklu çizgi with two
/// corners placed and the pointer at (700, 300).
fn polyline() -> App {
    let mut app = app_with_drawing();
    viewport(
        &mut app,
        Event::Resized(Rectangle::new(Point::ORIGIN, Size::new(1000.0, 800.0))),
    );
    app.draft.snap = false;
    app.draft.ortho = false;
    app.draft.polar = None;
    run(&mut app, "tool.polyline");
    click(&mut app, Point::new(300.0, 400.0));
    click(&mut app, Point::new(500.0, 400.0));
    viewport(&mut app, Event::Moved(Point::new(700.0, 300.0)));
    assert_eq!(app.session.point_count(), 2);
    app
}

/// The preview's cursor end: the last point of the open path.
fn preview_end(app: &App) -> Vec2 {
    let preview = app.session.preview(&app.format()).expect("a preview");
    *preview.path.last().expect("the cursor's segment")
}

/// The world point under `at` in the view as it is now.
fn under(app: &App, at: Point) -> Vec2 {
    app.viewport.world(at)
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6
}

#[test]
fn a_pan_from_the_ribbon_runs_over_the_command_and_gives_it_back() {
    let mut app = polyline();
    let prompt = app.session.prompt().text();
    let start = app.viewpoint();
    run(&mut app, "tool.pan");
    assert!(app.session.nested(), "Kaydır runs over the polyline");
    assert_eq!(app.session.tool_id(), "polyline");
    assert_eq!(
        app.log.lines().last().map(|l| (l.level, l.text.as_str())),
        Some((Level::Command, "Kaydır"))
    );
    // A drag with the left button moves the view, and places no corner.
    let from = Point::new(400.0, 300.0);
    viewport(&mut app, Event::Moved(from));
    viewport(&mut app, Event::Pressed(from));
    viewport(&mut app, Event::Moved(Point::new(450.0, 320.0)));
    viewport(&mut app, Event::Moved(Point::new(500.0, 340.0)));
    viewport(&mut app, Event::Released(Point::new(500.0, 340.0)));
    assert_ne!(app.viewpoint(), start);
    run(&mut app, "tool.cancel");
    assert!(!app.session.nested());
    assert_eq!(app.session.point_count(), 2);
    assert_eq!(app.session.prompt().text(), prompt);
    // The preview is where the pointer is, in the new view.
    let at = Point::new(500.0, 340.0);
    assert!(close(preview_end(&app), under(&app, at)));
    // The polyline goes on.
    click(&mut app, Point::new(600.0, 300.0));
    assert_eq!(app.session.point_count(), 3);
}

#[test]
fn a_zoom_window_gives_the_command_back_with_its_preview_in_the_new_view() {
    let mut app = polyline();
    run(&mut app, "tool.zoomWindow");
    assert!(app.session.nested());
    click(&mut app, Point::new(200.0, 200.0));
    let at = Point::new(800.0, 600.0);
    click(&mut app, at);
    assert!(!app.session.nested(), "the box ends it");
    assert_eq!(app.session.tool_id(), "polyline");
    assert_eq!(app.session.point_count(), 2, "its clicks were not corners");
    assert!(close(preview_end(&app), under(&app, at)));
    assert_eq!(app.view_history.len(), 1, "a navigation");
}

#[test]
fn the_wheel_and_the_view_commands_keep_the_preview_under_the_pointer() {
    let mut app = polyline();
    // The wheel at another place than the last move.
    let at = Point::new(600.0, 500.0);
    viewport(&mut app, Event::Zoomed { factor: 1.25, at });
    assert!(close(preview_end(&app), under(&app, at)));
    // Tümünü göster and Önceki görünüm: the pointer stays where it is on the area.
    run(&mut app, "view.zoomExtents");
    assert!(close(preview_end(&app), under(&app, at)));
    run(&mut app, "view.previous");
    assert!(close(preview_end(&app), under(&app, at)));
    assert_eq!(app.session.point_count(), 2);
}

#[test]
fn a_measure_over_the_command_says_its_length_and_writes_nothing() {
    let mut app = polyline();
    let count = app.document.as_ref().map(|d| d.model.entities().count());
    let prompt = app.session.prompt().text();
    run(&mut app, "tool.measure");
    assert!(app.session.nested());
    click(&mut app, Point::new(100.0, 100.0));
    click(&mut app, Point::new(200.0, 100.0));
    run(&mut app, "tool.confirm");
    assert!(
        app.log
            .lines()
            .any(|l| l.text.starts_with("Toplam uzunluk")),
        "it measured"
    );
    run(&mut app, "tool.cancel");
    assert!(!app.session.nested());
    assert_eq!(app.session.prompt().text(), prompt);
    assert_eq!(
        app.document.as_ref().map(|d| d.model.entities().count()),
        count
    );
    // Esc then ends the polyline: the command to repeat is it, not the measure.
    run(&mut app, "tool.cancel");
    assert!(!app.session.is_running());
    assert_eq!(app.session.last(), Some("polyline"));
}

/// Enter over Kaydır ends it and gives the polyline back; it does not start the polyline again.
#[test]
fn enter_ends_the_pan_and_keeps_the_polyline() {
    let mut app = polyline();
    run(&mut app, "tool.pan");
    run(&mut app, "tool.confirm");
    assert!(!app.session.nested());
    assert_eq!(app.session.tool_id(), "polyline");
    assert_eq!(app.session.point_count(), 2);
    assert!(close(
        preview_end(&app),
        under(&app, Point::new(700.0, 300.0))
    ));
}

#[test]
fn a_drawing_command_ends_the_running_one_as_before() {
    let mut app = polyline();
    run(&mut app, "tool.circle");
    assert!(!app.session.nested());
    assert_eq!(app.session.tool_id(), "circle");
}
