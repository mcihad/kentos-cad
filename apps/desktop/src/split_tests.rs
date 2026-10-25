//! Görünüm başına zaman, bağ ve revizyon (docs/adr/0576) in the split area: each view's own time and what the
//! active one's is for picking, the link's three modes against the screen scale's rule, and the other view as a
//! read-only revision (history, file, a downloaded file) with what changed marked.

use iced::{Point, Rectangle, Size};
use kentos_geometry_core::time::{self, Read, Window};
use kentos_geometry_core::tools::view_link::{self, Link};
use kentos_native_application::{ExecutionContext, registry};

use crate::app::{App, Message};
use crate::document::Document;
use crate::split_view::{Event, Source, TimeAsk, ViewTime};
use crate::viewport::Event as ViewEvent;

fn sized(app: &mut App) {
    let _ = app.update(Message::Viewport(ViewEvent::Resized(Rectangle::new(
        Point::ORIGIN,
        Size::new(500.0, 800.0),
    ))));
}

/// The temporal drawing with its slider open at its last position (2024) and the area split.
fn temporal() -> App {
    let (mut app, _) = App::boot(None);
    crate::temporal_scenes::opened(&mut app);
    sized(&mut app);
    let task = app.run("time.slider");
    crate::files_testing::drive(&mut app, task);
    assert!(app.time.slider.open);
    let _ = app.update(Message::Run("view.split"));
    app
}

fn back(app: &mut App, active: bool, years: usize) {
    let view = (!active).then_some(0);
    let _ = app.update(Message::Split(Event::Time {
        view,
        ask: TimeAsk::Own,
    }));
    for _ in 0..years {
        let _ = app.update(Message::Split(Event::Time {
            view,
            ask: TimeAsk::Step(-1),
        }));
    }
}

/// Each view with its own moment (docs/adr/0576 §1): the other at 2015 while the slider and the active view stand at
/// 2024; the active view's own time is what is picked and snapped to, and without the slider every view follows.
#[test]
fn each_view_shows_its_own_time_and_the_active_ones_is_what_is_picked() {
    let mut app = temporal();
    let latest = app.time.slider.window();
    back(&mut app, false, 9);
    let ViewTime::Own(m) = app.split.as_ref().expect("split").others[0].viewport.time else {
        panic!("the other view has a time of its own");
    };
    let Read::Moment(y2015) = time::read("2015-01-01") else {
        panic!("a date");
    };
    assert_eq!(m, y2015, "nine yearly steps back from 2024");
    let other = app.view_window(ViewTime::Own(m));
    assert_eq!(other, Some(Window::Instant(y2015)));
    assert_eq!(
        app.view_window(app.viewport.time),
        latest,
        "the active view follows the slider"
    );
    // Parcel 101 is valid 2010 to 2018: shown at 2015, not at 2024.
    let store = app.spatial.store();
    assert!(store.with_window(other, || store.view_shown(1.0)));
    assert!(
        !store.view_shown(1.0),
        "the store stands at the active view's time"
    );
    assert_eq!(store.time_window(), latest);
    // The active view takes a time of its own: what is picked follows it, the slider stays.
    back(&mut app, true, 9);
    assert_eq!(
        app.spatial.store().time_window(),
        Some(Window::Instant(y2015))
    );
    assert!(app.spatial.store().view_shown(1.0));
    assert_eq!(
        app.time.slider.window(),
        latest,
        "the slider is the slider's"
    );
    // The slider closed: every view follows again (no window at all).
    let task = app.update(Message::Run("time.slider"));
    crate::files_testing::drive(&mut app, task);
    assert!(!app.time.slider.open);
    assert_eq!(app.viewport.time, ViewTime::Follow);
    assert_eq!(
        app.split.as_ref().expect("split").others[0].viewport.time,
        ViewTime::Follow
    );
    assert_eq!(app.spatial.store().time_window(), None);
}

/// A view's own time is a view setting like the theme (docs/adr/0523): the drawing, its revision and its undo steps
/// do not move.
#[test]
fn a_time_of_its_own_is_no_edit() {
    let mut app = temporal();
    let model = &app.document.as_ref().expect("open").model;
    let (revision, undoable) = (model.revision(), model.can_undo());
    back(&mut app, false, 9);
    back(&mut app, true, 3);
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!((model.revision(), model.can_undo()), (revision, undoable));
}

fn split_app() -> App {
    let mut app = crate::files_testing::app_with_drawing();
    sized(&mut app);
    let _ = app.update(Message::Run("view.split"));
    let _ = app.update(Message::OtherViewport(0, ViewEvent::Resized(Rectangle::new(
        Point::new(502.0, 0.0),
        Size::new(500.0, 800.0),
    ))));
    app
}

/// Bağla's three modes (docs/adr/0576 §2): the centre alone, the centre with the scale and the twist, nothing; and
/// the scale a view shows is the camera's own 96 dpi scale.
#[test]
fn the_link_follows_the_centre_or_the_centre_and_the_scale() {
    let mut app = split_app();
    // The other view at three times the scale.
    let _ = app.update(Message::OtherViewport(0, ViewEvent::Zoomed {
        factor: 3.0,
        at: Point::new(250.0, 400.0),
    }));
    let own_scale = app.split.as_ref().expect("split").others[0].viewport.camera.scale;
    let _ = app.update(Message::Split(Event::Link(Link::Centre)));
    app.viewport.camera.zoom_at(2.0, 100.0, 100.0);
    let _ = app.update(Message::DialogClosed);
    let (a, b) = (
        app.viewport.camera,
        app.split.as_ref().expect("split").others[0].viewport.camera,
    );
    assert_eq!(b.center, a.center, "the centre followed");
    assert_eq!(b.scale, own_scale, "the scale stayed its own");
    assert_ne!(b.scale, a.scale);
    let _ = app.update(Message::Split(Event::Link(Link::CentreScale)));
    let (a, b) = (
        app.viewport.camera,
        app.split.as_ref().expect("split").others[0].viewport.camera,
    );
    assert_eq!((b.center, b.scale, b.twist), (a.center, a.scale, a.twist));
    // Nothing: the other view stays where it is.
    let _ = app.update(Message::Split(Event::Link(Link::Off)));
    let before = app.split.as_ref().expect("split").others[0].viewport.camera;
    app.viewport.camera.zoom_at(1.5, 50.0, 50.0);
    let _ = app.update(Message::DialogClosed);
    assert_eq!(app.split.as_ref().expect("split").others[0].viewport.camera, before);
    // The scale each view says is the camera's.
    for c in [app.viewport.camera, before] {
        assert_eq!(view_link::screen_scale(c.scale), c.screen_scale());
    }
}

fn model_of(app: &App) -> &kentos_domain::Document {
    &app.document.as_ref().expect("open").model
}

fn run(app: &mut App, command: &str, input: serde_json::Value) {
    let doc = app.document.as_mut().expect("open");
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc.model),
        command,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
}

/// The open drawing some undo steps back in the other view (docs/adr/0576 §3): a copy that changes nothing of the
/// drawing or its history, what the drawing changed since marked, and never the edit target.
#[test]
fn the_other_view_shows_the_history_read_only_with_what_changed_marked() {
    use crate::tools_scenes::{Objects, open as open_drawing};
    let mut app = crate::files_testing::app_with_drawing();
    sized(&mut app);
    let mut o = Objects::new();
    o.path(
        "parsel",
        &[[0.0, 0.0], [20.0, 0.0], [20.0, 16.0], [0.0, 16.0]],
        true,
    );
    o.line("yol", [-8.0, -6.0], [48.0, -6.0]);
    open_drawing(&mut app, o);
    let uid = |app: &App, slot: u32| {
        model_of(app)
            .uid(kentos_domain::Slot(slot))
            .expect("an object")
            .to_string()
    };
    let (first, second) = (uid(&app, 1), uid(&app, 2));
    run(
        &mut app,
        "cad.entities.transform",
        serde_json::json!({ "uids": [second], "transform": { "kind": "move", "dx": 5.0, "dy": 0.0 } }),
    );
    run(
        &mut app,
        "cad.entities.delete",
        serde_json::json!({ "uids": [first] }),
    );
    let (revision, labels) = (model_of(&app).revision(), model_of(&app).undo_labels());
    assert_eq!(labels, ["Sil", "Taşı"]);
    // Two steps back: the parcel is there, the line not yet moved: the parcel is removed since, the line changed.
    let _ = app.update(Message::Split(Event::Source(0, Source::History(2))));
    let revision_view = app
        .split
        .as_ref()
        .and_then(|s| s.others[0].revision.as_ref())
        .expect("a revision");
    assert_eq!(revision_view.name, "2 adım geri, “Taşı” öncesi");
    let c = revision_view.counts;
    assert_eq!((c.changed, c.added, c.removed, c.same), (1, 0, 1, 0));
    assert_eq!(
        revision_view.spatial.len(),
        2,
        "the revision's store holds its objects"
    );
    assert_eq!(
        revision_view.marked.ids().len(),
        2,
        "the changed and the removed"
    );
    // The drawing and its history are as they were.
    assert_eq!(model_of(&app).revision(), revision);
    assert_eq!(model_of(&app).undo_labels(), labels);
    // One step back: the parcel is there, the line already moved: only the parcel differs.
    let _ = app.update(Message::Split(Event::Source(0, Source::History(1))));
    let c = app
        .split
        .as_ref()
        .and_then(|s| s.others[0].revision.as_ref())
        .expect("a revision")
        .counts;
    assert_eq!((c.changed, c.removed, c.same), (0, 1, 1));
    // Read only: a press on the revision does not make it the active view.
    let left = app.split.as_ref().expect("split").active_at == 1;
    let at = Point::new(300.0, 300.0);
    let _ = app.update(Message::OtherViewport(0, ViewEvent::Pressed(at)));
    let _ = app.update(Message::Viewport(ViewEvent::Released(at)));
    assert_eq!(app.split.as_ref().expect("split").active_at == 1, left);
    assert!(app.viewport.other.is_none());
    // More steps than the history holds: said, the revision shown stays.
    let _ = app.update(Message::Split(Event::Source(0, Source::History(9))));
    assert!(app.split.as_ref().is_some_and(|s| s.others[0].revision.is_some()));
    // The open drawing again.
    let _ = app.update(Message::Split(Event::Source(0, Source::Drawing)));
    assert!(app.split.as_ref().is_some_and(|s| s.others[0].revision.is_none()));
}

fn revisions(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/compare/v1/revisions")
        .join(name)
}

/// A drawing file, and a downloaded revision, in the other view (docs/adr/0576 §3): the same counts the shared cases
/// of `cad.drawing.compare` give; the downloaded file is read and removed, and the catalog goes.
#[test]
fn a_file_and_a_downloaded_revision_are_shown_and_compared() {
    let mut app = crate::files_testing::app_with_drawing();
    sized(&mut app);
    let open = Document::read(&revisions("ayni.kcad")).expect("reads");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(open)))));
    app.revision_from_file(0, &revisions("tasindi.kcad"));
    let r = app
        .split
        .as_ref()
        .and_then(|s| s.others[0].revision.as_ref())
        .expect("a revision");
    assert_eq!(r.name, "tasindi.kcad");
    // A parcel moved, a parcel the file lacks, a line the drawing lacks: 1 changed, 1 added, 1 removed, 5 equal.
    let c = r.counts;
    assert_eq!((c.changed, c.added, c.removed, c.same), (1, 1, 1, 5));
    assert_eq!(r.marked.ids().len(), 2);
    assert_eq!(
        r.spatial.len(),
        7,
        "a file's store holds all its objects (reloaded, not synced)"
    );
    // A revision downloaded to a file of the app's own.
    let _ = app.update(Message::Split(Event::Source(0, Source::Drawing)));
    let temp = std::env::temp_dir().join(format!("kentos-ikinci-{}.kcad", std::process::id()));
    std::fs::copy(revisions("oznitelik.kcad"), &temp).expect("copies");
    app.revision_from_download("Revizyon 3", &temp);
    assert!(!temp.exists(), "read, then removed");
    let r = app
        .split
        .as_ref()
        .and_then(|s| s.others[0].revision.as_ref())
        .expect("a revision");
    assert_eq!(r.name, "Revizyon 3");
    assert_eq!(r.counts.changed, 3);
    // The drawing's edit marks follow the drawing: an edit that moves a parcel changes the counts.
    let before = r.counts;
    let uid = model_of(&app)
        .uid(kentos_domain::Slot(3))
        .expect("a parcel")
        .to_string();
    run(
        &mut app,
        "cad.entities.transform",
        serde_json::json!({ "uids": [uid], "transform": { "kind": "move", "dx": 1.0, "dy": 0.0 } }),
    );
    let _ = app.update(Message::DialogClosed);
    let after = app
        .split
        .as_ref()
        .and_then(|s| s.others[0].revision.as_ref())
        .expect("a revision")
        .counts;
    assert_eq!(
        after.changed, before.changed,
        "that parcel was changed already (its layer)"
    );
    let _ = app.update(Message::Split(Event::Source(0, Source::Drawing)));
}
