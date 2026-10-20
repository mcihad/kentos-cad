//! The snap is taken again where a point is committed (docs/adr/0273): a press with no move before it, a drawing
//! changed between the last move and the press, Tab's candidates of a drawing since changed, and a large drawing
//! all commit the snap of the place and the drawing of the press, never the last move's.

use kentos_commands::Entity;
use kentos_interaction::SnapKind;

use crate::app::{App, Message};
use crate::tools_scenes::{E, N, Objects, at, hover, open, run};
use crate::viewport::Event;

fn press(app: &mut App, p: [f64; 2]) {
    let point = at(app, p);
    let _ = app.update(Message::Viewport(Event::Pressed(point)));
    let _ = app.update(Message::Viewport(Event::Released(point)));
}

/// The newest line's first point, east and north of the origin.
fn newest_start(app: &App) -> [f64; 2] {
    let doc = app.document.as_ref().expect("a drawing");
    let line = doc
        .model
        .entities()
        .filter_map(|e| match e {
            Entity::Line(l) => Some(l),
            _ => None,
        })
        .max_by_key(|l| l.base.id)
        .expect("a line");
    [line.a.x - E, line.a.y - N]
}

fn near(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9
}

/// A view 1100 × 600 px over the two lines at 30 px a metre: the aperture is a fraction of a metre (the test app
/// has no window to fit to).
fn close(app: &mut App) {
    let c = &mut app.viewport.camera;
    (c.width, c.height, c.scale) = (1100.0, 600.0, 30.0);
    c.center = kentos_interaction::Vec2::new(E + 15.0, N + 4.0);
}

fn two_lines() -> Objects {
    let mut o = Objects::new();
    o.line("cizim", [0.0, 0.0], [10.0, 0.0]);
    o.line("cizim", [20.0, 0.0], [30.0, 0.0]);
    o
}

#[test]
fn a_press_with_no_move_before_it_takes_the_snap_of_its_own_place() {
    let mut app = crate::files_testing::app_with_drawing();
    open(&mut app, two_lines());
    close(&mut app);
    run(&mut app, "tool.line");
    // The last move snaps to the first line's start …
    hover(&mut app, [0.1, 0.1]);
    assert_eq!(app.snap.map(|s| s.kind), Some(SnapKind::Endpoint));
    // … the press, far from it with no move between, takes the second line's start.
    press(&mut app, [20.1, 0.1]);
    press(&mut app, [25.0, 10.0]);
    assert!(
        near(newest_start(&app), [20.0, 0.0]),
        "{:?}",
        newest_start(&app)
    );
}

#[test]
fn a_snap_point_gone_since_the_last_move_is_not_committed() {
    let mut app = crate::files_testing::app_with_drawing();
    open(&mut app, two_lines());
    close(&mut app);
    run(&mut app, "tool.line");
    hover(&mut app, [20.1, 0.1]);
    assert_eq!(app.snap.map(|s| s.kind), Some(SnapKind::Endpoint));
    // The second line goes (another's edit, a script) with the pointer still.
    let doc = app.document.as_mut().expect("a drawing");
    let slot = doc
        .model
        .entities()
        .find(|e| matches!(e, Entity::Line(l) if l.a.x - E == 20.0))
        .map(|e| kentos_domain::Slot(e.base().id))
        .expect("the second line");
    assert_eq!(doc.model.remove(&[slot]), 1);
    press(&mut app, [20.1, 0.1]);
    press(&mut app, [25.0, 10.0]);
    let start = newest_start(&app);
    assert!(
        !near(start, [20.0, 0.0]),
        "the removed line's end was committed"
    );
    assert!(
        (start[0] - 20.1).abs() < 0.2 && (start[1] - 0.1).abs() < 0.2,
        "{start:?}"
    );
}

#[test]
fn tab_s_candidates_of_a_drawing_since_changed_are_let_go() {
    let mut app = crate::files_testing::app_with_drawing();
    let mut o = Objects::new();
    o.line("cizim", [0.0, 0.0], [10.0, 0.0]);
    o.line("cizim", [0.6, -3.0], [0.6, 3.0]);
    open(&mut app, o);
    run(&mut app, "tool.line");
    hover(&mut app, [0.2, 0.1]);
    assert!(app.cycle_snap(false), "two candidates: a step");
    let cycled = app.snap.expect("the cycled snap");
    // An edit with the pointer still: the cycle's candidates are of the drawing before it.
    let doc = app.document.as_mut().expect("a drawing");
    let mut copy = doc.model.entities().next().cloned().expect("a line");
    if let Entity::Line(l) = &mut copy {
        l.a.x += 50.0;
        l.b.x += 50.0;
    }
    doc.model.add(copy).expect("a slot");
    press(&mut app, [0.2, 0.1]);
    assert!(app.snap_cycle.is_none(), "the cycle went with the change");
    assert_ne!(app.snap, Some(cycled), "the press took the snap anew");
}

#[test]
fn in_a_large_drawing_a_far_press_takes_its_own_snap() {
    let mut app = crate::files_testing::app_with_drawing();
    let mut o = Objects::new();
    // 20 000 short lines on a 141 × 142 grid, one metre apart.
    for i in 0..20_000 {
        let (x, y) = (f64::from(i % 141), f64::from(i / 141));
        o.line("cizim", [x, y], [x + 0.5, y]);
    }
    open(&mut app, o);
    // The whole grid on a 1100 × 600 px view: an aperture of about 1.4 m.
    let c = &mut app.viewport.camera;
    (c.width, c.height, c.scale) = (1100.0, 600.0, 8.0);
    c.center = kentos_interaction::Vec2::new(E + 70.0, N + 70.0);
    run(&mut app, "tool.line");
    hover(&mut app, [3.02, 4.02]);
    assert_eq!(
        app.snap.map(|s| s.point),
        Some(kentos_interaction::Vec2::new(E + 3.0, N + 4.0))
    );
    // A fast move: the next event is the press, 100 m away.
    press(&mut app, [103.02, 104.02]);
    press(&mut app, [120.0, 130.0]);
    assert!(
        near(newest_start(&app), [103.0, 104.0]),
        "{:?}",
        newest_start(&app)
    );
}
