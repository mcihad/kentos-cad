//! Up to four views (docs/adr/0637 §1, §2): each stands at a place of its own and keeps it; a press makes a view
//! active without moving anything on the screen; × closes one; Bağla moves every one of them; Kaydırmalı karşılaştır
//! is of two; a revision's view has a time of its own.

use iced::{Point, Rectangle, Size};
use kentos_geometry_core::time::{self, Read, Window};
use kentos_geometry_core::tools::view_link::Link;

use crate::app::{App, Message};
use crate::split_view::{Event, MOST_VIEWS, Source, TimeAsk};
use crate::viewport::Event as ViewEvent;

fn sized(app: &mut App) {
    let _ = app.update(Message::Viewport(ViewEvent::Resized(Rectangle::new(
        Point::ORIGIN,
        Size::new(500.0, 400.0),
    ))));
}

/// The views' places: the active one's, then each other's by index.
fn places(app: &App) -> (usize, Vec<usize>) {
    let s = app.split.as_ref().expect("split");
    (s.active_at, s.others.iter().map(|o| o.place).collect())
}

#[test]
fn up_to_four_views_stand_in_place_and_a_press_swaps_without_moving() {
    let mut app = crate::files_testing::app_with_drawing();
    sized(&mut app);
    let _ = app.update(Message::Run("view.splitAdd"));
    assert_eq!(places(&app), (0, vec![1]), "the area splits in two first");
    let _ = app.update(Message::Run("view.splitAdd"));
    let _ = app.update(Message::Run("view.splitAdd"));
    assert_eq!(places(&app), (0, vec![1, 2, 3]));
    // A fifth is refused and said; four stay.
    let _ = app.update(Message::Run("view.splitAdd"));
    assert_eq!(
        app.split.as_ref().expect("split").others.len(),
        MOST_VIEWS - 1
    );
    assert!(crate::files_testing::last_said(&app).contains("en çok 4"));
    // Each view tells its index.
    let others = &app.split.as_ref().expect("split").others;
    assert_eq!(
        others.iter().map(|o| o.viewport.other).collect::<Vec<_>>(),
        [Some(0), Some(1), Some(2)]
    );
    // The view at place 2 zoomed alone, then pressed: it is the active one, at its place, with its camera.
    let _ = app.update(Message::OtherViewport(
        1,
        ViewEvent::Resized(Rectangle::new(
            Point::new(0.0, 402.0),
            Size::new(500.0, 400.0),
        )),
    ));
    let before = app.viewport.camera;
    let _ = app.update(Message::OtherViewport(
        1,
        ViewEvent::Zoomed {
            factor: 2.0,
            at: Point::new(250.0, 200.0),
        },
    ));
    assert_eq!(app.viewport.camera, before, "the active view stays");
    let zoomed = app.split.as_ref().expect("split").others[1].viewport.camera;
    let _ = app.update(Message::OtherViewport(
        1,
        ViewEvent::Pressed(Point::new(10.0, 10.0)),
    ));
    let _ = app.update(Message::Viewport(ViewEvent::Released(Point::new(
        10.0, 10.0,
    ))));
    assert_eq!(
        app.viewport.camera, zoomed,
        "the pressed view is the active one"
    );
    assert_eq!(
        places(&app),
        (2, vec![1, 0, 3]),
        "nothing moved on the screen"
    );
    assert!(app.viewport.other.is_none());
    assert_eq!(
        app.split.as_ref().expect("split").others[1].viewport.other,
        Some(1)
    );
    // × on the view at place 1 (index 0): the places after it close up, the indices follow.
    let _ = app.update(Message::Split(Event::Close(0)));
    assert_eq!(places(&app), (1, vec![0, 2]));
    let others = &app.split.as_ref().expect("split").others;
    assert_eq!(
        others.iter().map(|o| o.viewport.other).collect::<Vec<_>>(),
        [Some(0), Some(1)]
    );
    // The last ones closed: one view.
    let _ = app.update(Message::Split(Event::Close(1)));
    let _ = app.update(Message::Split(Event::Close(0)));
    assert!(app.split.is_none());
    assert!(app.viewport.other.is_none());
}

#[test]
fn bagla_moves_every_view_and_swipe_is_of_two() {
    let mut app = crate::files_testing::app_with_drawing();
    sized(&mut app);
    let _ = app.update(Message::Run("view.splitAdd"));
    let _ = app.update(Message::Run("view.splitAdd"));
    let _ = app.update(Message::Split(Event::Link(Link::CentreScale)));
    app.viewport.camera.zoom_at(3.0, 50.0, 50.0);
    let _ = app.update(Message::DialogClosed);
    let a = app.viewport.camera;
    for o in &app.split.as_ref().expect("split").others {
        assert_eq!(
            (o.viewport.camera.center, o.viewport.camera.scale),
            (a.center, a.scale)
        );
    }
    // Three views: Kaydırmalı karşılaştır is refused and said.
    let _ = app.update(Message::Run("view.swipe"));
    assert!(app.split.as_ref().is_some_and(|s| s.swipe.is_none()));
    assert!(crate::files_testing::last_said(&app).contains("iki görünümle"));
    // Two views compared; a view added goes back to side by side.
    let _ = app.update(Message::Split(Event::Close(1)));
    let _ = app.update(Message::Run("view.swipe"));
    assert!(app.split.as_ref().is_some_and(|s| s.swipe.is_some()));
    let _ = app.update(Message::Run("view.splitAdd"));
    let s = app.split.as_ref().expect("split");
    assert!(s.swipe.is_none() && s.others.iter().all(|o| !o.viewport.passive));
}

/// A revision's view has a time of its own (docs/adr/0637 §2): the open drawing one step back shown at 2015 beside the
/// drawing itself and the slider at 2024; the revision's own store shows parcel 101 then.
#[test]
fn a_revision_s_view_has_a_time_of_its_own() {
    let (mut app, _) = App::boot(None);
    crate::temporal_scenes::opened(&mut app);
    sized(&mut app);
    let task = app.run("time.slider");
    crate::files_testing::drive(&mut app, task);
    // A step to go back: a point drawn.
    let doc = app.document.as_mut().expect("open");
    let layer = doc.model.layers().leaves()[0].id.clone();
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.point.create",
        kentos_commands::CallOp::Execute,
        serde_json::json!({ "layerId": layer, "p": { "x": 0.0, "y": 0.0 } }),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let _ = app.update(Message::Run("view.splitAdd"));
    let _ = app.update(Message::Run("view.splitAdd"));
    let _ = app.update(Message::Split(Event::Source(1, Source::History(1))));
    let view = Some(1);
    let _ = app.update(Message::Split(Event::Time {
        view,
        ask: TimeAsk::Own,
    }));
    for _ in 0..9 {
        let _ = app.update(Message::Split(Event::Time {
            view,
            ask: TimeAsk::Step(-1),
        }));
    }
    let Read::Moment(y2015) = time::read("2015-01-01") else {
        panic!("a date");
    };
    let s = app.split.as_ref().expect("split");
    let o = &s.others[1];
    let window = app.view_window(o.viewport.time);
    assert_eq!(window, Some(Window::Instant(y2015)));
    let r = o.revision.as_ref().expect("a revision");
    let store = r.spatial.store();
    assert!(
        store.with_window(window, || store.view_shown(1.0)),
        "101 at 2015"
    );
    assert!(
        !store.with_window(app.time.slider.window(), || store.view_shown(1.0)),
        "not at 2024"
    );
    assert!(
        s.others[0].revision.is_none(),
        "the other view shows the drawing"
    );
}

/// The open drawing's selection in a revision's view (docs/adr/0637 §3): the same records, by their lasting ids, in
/// place of the marks while something is selected.
#[test]
fn a_selection_shows_the_same_records_in_a_revision() {
    let mut app = crate::files_testing::app_with_drawing();
    sized(&mut app);
    let doc = app.document.as_mut().expect("open");
    let first = doc
        .model
        .entities()
        .next()
        .map(|e| e.base().id)
        .expect("an object");
    let slot = kentos_domain::Slot(first);
    let uid = doc.model.uid(slot).expect("a lasting id");
    // A step to go back to: the first object moved.
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.entities.transform",
        kentos_commands::CallOp::Execute,
        serde_json::json!({ "uids": [uid.to_string()], "transform": { "kind": "move", "dx": 3.0, "dy": 0.0 } }),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let _ = app.update(Message::Run("view.split"));
    let _ = app.update(Message::Split(Event::Source(0, Source::History(1))));
    let chosen = |app: &App| {
        app.split.as_ref().expect("split").others[0]
            .revision
            .as_ref()
            .expect("a revision")
            .chosen
            .ids()
            .to_vec()
    };
    assert!(chosen(&app).is_empty(), "nothing selected: the marks");
    app.selection.set([slot]);
    let _ = app.update(Message::DialogClosed);
    let r = app.split.as_ref().expect("split").others[0]
        .revision
        .as_ref()
        .expect("a revision");
    let theirs = r
        .doc
        .model
        .slot_of(uid)
        .expect("the record is in the revision");
    assert_eq!(chosen(&app), [theirs]);
    app.selection.set([]);
    let _ = app.update(Message::DialogClosed);
    assert!(chosen(&app).is_empty());
}

/// The revision's marks follow the open drawing's edits by the objects they touched (docs/adr/0637 §4) and come out
/// as a whole comparison would: moves, a removal and its undo, an attribute, a new object and its removal.
#[test]
fn marks_follow_edits_as_the_whole_comparison_would() {
    let mut app = crate::files_testing::app_with_drawing();
    sized(&mut app);
    let uids: Vec<String> = {
        let doc = app.document.as_ref().expect("open");
        doc.model
            .entities_with_uids()
            .map(|(u, _)| u.to_string())
            .take(4)
            .collect()
    };
    assert_eq!(uids.len(), 4, "the drawing has four objects to edit");
    let run = |app: &mut App, name: &str, input: serde_json::Value| {
        let doc = app.document.as_mut().expect("open");
        let answer = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            name,
            kentos_commands::CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
        let _ = app.update(Message::DialogClosed);
    };
    let moved = |u: &str, dx: f64| serde_json::json!({ "uids": [u], "transform": { "kind": "move", "dx": dx, "dy": 0.0 } });
    run(&mut app, "cad.entities.transform", moved(&uids[0], 2.0));
    let _ = app.update(Message::Run("view.split"));
    let _ = app.update(Message::Split(Event::Source(0, Source::History(1))));
    let check = |app: &App, step: &str| {
        let r = app.split.as_ref().expect("split").others[0]
            .revision
            .as_ref()
            .expect("a revision");
        let (counts, marked) = r.whole(app.document.as_ref().expect("open"));
        assert_eq!(r.counts, counts, "{step}: the counts");
        assert_eq!(r.marked.ids(), marked.as_slice(), "{step}: the marks");
    };
    check(&app, "opened");
    run(&mut app, "cad.entities.transform", moved(&uids[1], 5.0));
    check(&app, "a second move");
    run(&mut app, "cad.entities.transform", moved(&uids[0], -2.0));
    check(&app, "the first moved back");
    run(
        &mut app,
        "cad.entities.delete",
        serde_json::json!({ "uids": [uids[2]] }),
    );
    check(&app, "a removal");
    app.document.as_mut().expect("open").model.undo();
    let _ = app.update(Message::DialogClosed);
    check(&app, "its undo");
    run(
        &mut app,
        "cad.entities.set",
        serde_json::json!({ "uids": [uids[3]], "attrs": { "Ada": "12" }, "operation": "attributes" }),
    );
    check(&app, "an attribute");
    let layer = app
        .document
        .as_ref()
        .expect("open")
        .model
        .entities()
        .next()
        .map(|e| e.base().layer_id.clone())
        .expect("a layer");
    run(
        &mut app,
        "cad.line.create",
        serde_json::json!({ "layerId": layer, "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 9.0, "y": 9.0 } }),
    );
    check(&app, "a new line");
    app.document.as_mut().expect("open").model.undo();
    let _ = app.update(Message::DialogClosed);
    check(&app, "the new line undone");
}

/// A view zoomed with the wheel waits to settle; a new size still reaches it at once, so its scene and its text
/// are of one size (docs/adr/0637 §1).
#[test]
fn a_view_waiting_to_settle_takes_a_new_size() {
    use kentos_ui::snapshot::Snapshot;
    let mut app = crate::files_testing::app_with_drawing();
    app.window_size = Size::new(1100.0, 650.0);
    let Ok(mut snapshot) = Snapshot::new(Size::new(1100.0, 650.0)) else {
        return;
    };
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let _ = app.update(Message::Run("view.split"));
    let _ = app.update(Message::OtherViewport(
        0,
        ViewEvent::Zoomed {
            factor: 0.6,
            at: Point::new(150.0, 150.0),
        },
    ));
    snapshot.settle(&mut app, App::view, &mut update);
    let other = &app.split.as_ref().expect("split").others[0].viewport;
    assert!(other.bounds.width > 0.0, "the view was told its size");
    assert_eq!(other.camera.width, f64::from(other.bounds.width));
    assert_eq!(other.camera.width, app.viewport.camera.width);
}
