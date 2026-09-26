//! Kaydır and Pencere yakınlaştır through the session: the web's `PanTool`
//! and `ZoomWindowTool`, one rule per test (docs/adr/0056). The tools ask for
//! view changes; the bench collects them as the desktop's camera takes them.
//! The bench's view is 0.125 m per pixel (8 px per metre).

mod common;

use common::{Bench, E, N};

const OBJECTS: &str = include_str!("../../../../fixtures/interaction/v1/objects.kcad");
use kentos_geometry_core::geometry::Bounds;
use kentos_interaction::{Cursor, ViewChange};

#[test]
fn a_drag_moves_the_view_with_the_pointer() {
    let mut b = Bench::new("pan");
    assert_eq!(
        b.session.prompt().text(),
        "Kaydır: sürükleyerek görünümü taşıyın. Çıkmak için Esc"
    );
    assert!(b.options().is_empty());
    assert_eq!(b.session.cursor(), Cursor::Grab);
    // Moving with no button down moves nothing.
    b.move_to(2.0, 2.0);
    assert!(b.views.is_empty());
    // Down at (0, 0), through (4, 2) to (8, 4): 64 px right and 32 up in two steps.
    b.drag([0.0, 0.0], [8.0, 4.0]);
    assert_eq!(
        b.views,
        [
            ViewChange::Pan {
                dx: 32.0,
                dy: -16.0
            },
            ViewChange::Pan {
                dx: 32.0,
                dy: -16.0
            },
        ]
    );
    // Released: moving again moves nothing; the tool stays until Esc.
    b.move_to(0.0, 0.0);
    assert_eq!(b.views.len(), 2);
    assert_eq!(b.session.tool_id(), "pan");
    assert!(!b.run(|s, cx| s.cancel(cx)));
    assert!(!b.session.is_running());
}

#[test]
fn pan_takes_no_confirm_and_is_not_repeated() {
    // Snapping on, over the traces' objects: line 1 ends at (−8, −12).
    let mut b = Bench::on(OBJECTS);
    b.start("line");
    assert!(b.snapped(-8.1, -12.1).snap.is_some(), "the line tool snaps");
    b.run(|s, _| s.exit());
    b.start("pan");
    assert!(
        !b.session.confirms(),
        "Enter repeats the last command instead"
    );
    assert_eq!(b.session.last(), Some("line"), "Kaydır is not remembered");
    assert!(b.snapped(-8.1, -12.1).snap.is_none(), "it does not snap");
    b.start("zoomWindow");
    assert!(!b.session.confirms());
    assert!(b.snapped(-8.1, -12.1).snap.is_none());
    assert_eq!(
        b.session.last(),
        Some("zoomWindow"),
        "Pencere yakınlaştır is"
    );
    assert_eq!(b.session.cursor(), Cursor::Cross);
}

#[test]
fn two_clicks_fit_the_box_they_give() {
    let mut b = Bench::new("zoomWindow");
    assert_eq!(
        b.session.prompt().text(),
        "Pencere yakınlaştır: ilk köşeyi belirtin"
    );
    b.click(-24.0, -14.0);
    assert_eq!(
        b.session.prompt().text(),
        "Pencere yakınlaştır: karşı köşeyi belirtin"
    );
    assert_eq!(b.points(), 0, "the web's tool counts no points");
    // The box follows the pointer, dashed, from the first corner.
    b.move_to(-4.0, -2.0);
    let preview = b
        .session
        .preview(&kentos_interaction::Format::default())
        .expect("a preview");
    let [stroke] = preview.strokes.as_slice() else {
        panic!("one box: {preview:?}");
    };
    assert!(stroke.closed && stroke.dash == Some([5.0, 4.0]));
    assert_eq!(stroke.pts.len(), 4);
    b.click(-4.0, -2.0);
    assert_eq!(
        b.views,
        [ViewChange::Fit {
            bounds: Bounds {
                min_x: E - 24.0,
                min_y: N - 14.0,
                max_x: E - 4.0,
                max_y: N - 2.0,
            },
            padding: 0.0,
        }]
    );
    assert!(!b.session.is_running(), "it leaves");
}

#[test]
fn a_drag_past_four_pixels_is_a_box_too() {
    let mut b = Bench::new("zoomWindow");
    // Half a metre is 4 px: not past the threshold; the release waits for the other corner.
    b.drag([0.0, 0.0], [0.5, 0.0]);
    assert!(b.views.is_empty());
    assert!(b.session.is_running());
    b.run(|s, _| s.exit());
    b.start("zoomWindow");
    b.drag([2.0, 1.0], [-6.0, 7.0]);
    assert_eq!(
        b.views,
        [ViewChange::Fit {
            bounds: Bounds {
                min_x: E - 6.0,
                min_y: N + 1.0,
                max_x: E + 2.0,
                max_y: N + 7.0,
            },
            padding: 0.0,
        }]
    );
    assert!(!b.session.is_running());
}

#[test]
fn a_box_with_no_width_or_height_changes_nothing() {
    let mut b = Bench::new("zoomWindow");
    b.click(1.0, 1.0);
    b.click(1.0, 9.0);
    assert!(b.views.is_empty(), "no width");
    assert!(!b.session.is_running(), "the tool leaves all the same");
    b.start("zoomWindow");
    b.click(1.0, 1.0);
    b.click(1.0, 1.0);
    assert!(b.views.is_empty());
    // Typed text and Ctrl+Z are not the tool's.
    b.start("zoomWindow");
    assert!(!b.type_text("10,10"));
    assert!(!b.undo_step());
}
