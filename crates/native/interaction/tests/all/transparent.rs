//! Transparent tools through the session (docs/adr/0283): Kaydır, Pencere
//! yakınlaştır, Mesafe ölç, Açı ölç and Koordinat oku run over a command
//! without ending it, as AutoCAD's `'pan`, `'zoom` and `'dist`; the point
//! calculator does too (docs/adr/0083). When they end, the command is back
//! with its points, options, prompt, locks and preview as they were, and
//! its preview follows the pointer again. A tool that writes is never one.

use crate::common;

use common::{Bench, E, N};
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::point_calc::{CalcKind, PointCalc};
use kentos_interaction::{Preview, Session, ViewChange};

const OBJECTS: &str = include_str!("../../../../../fixtures/interaction/v1/objects.kcad");

/// What a command shows at one moment: its prompt, its points, its options and its preview.
#[derive(Debug, PartialEq)]
struct Shown {
    tool: &'static str,
    prompt: String,
    points: usize,
    options: Vec<&'static str>,
    preview: Option<Preview>,
}

fn shown(b: &Bench) -> Shown {
    let format = kentos_interaction::Format::default();
    Shown {
        tool: b.session.tool_id(),
        prompt: b.session.prompt().text(),
        points: b.session.point_count(),
        options: b.options(),
        preview: b.session.preview(&format),
    }
}

/// Starts `id` over what runs, as the desktop does with a transparent tool.
fn over(b: &mut Bench, id: &str) {
    assert!(
        b.session.can_start_over(id),
        "{id} runs over {}",
        b.session.tool_id()
    );
    assert!(b.run(|s, cx| s.start_over(id, cx)), "{id} started over");
    assert!(b.session.nested());
    assert_eq!(b.session.top_id(), Some(id));
}

/// Esc ends the transparent tool: the command is back, and the host is told.
fn back(b: &mut Bench) {
    assert!(
        b.run(|s, cx| s.cancel(cx)),
        "Esc leaves the transparent tool only"
    );
    assert!(!b.session.nested());
    assert!(b.session.take_resumed(), "the host gives the pointer again");
}

/// Çizgi with two points placed (snapping off), the pointer at (6, 2).
fn line_with_points() -> Bench {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.click(4.0, 0.0);
    b.move_to(6.0, 2.0);
    b
}

/// Each transparent tool, run over Çoklu çizgi with two points placed and
/// ended with Esc, gives it back as it was.
#[test]
fn every_transparent_tool_gives_the_command_back_as_it_was() {
    for id in Session::tools()
        .iter()
        .filter(|id| Session::is_transparent(id))
    {
        let mut b = line_with_points();
        let before = shown(&b);
        assert!(before.preview.as_ref().is_some_and(|p| !p.path.is_empty()));
        let (runs, last) = (b.session.runs(), b.session.last());
        over(&mut b, id);
        assert_eq!(
            b.session.tool_id(),
            "polyline",
            "{id}: the command is still the polyline"
        );
        assert_eq!(b.session.runs(), runs, "{id} is not a run of its own");
        assert_ne!(
            b.session.prompt().text(),
            before.prompt,
            "{id} speaks for itself"
        );
        back(&mut b);
        assert_eq!(b.session.last(), last, "{id} is not remembered for repeat");
        assert_eq!(b.session.prompt().text(), before.prompt, "{id}");
        assert_eq!(b.session.point_count(), 2, "{id}");
        // The host gives the pointer again where it is: the preview is the one before.
        b.move_to(6.0, 2.0);
        assert_eq!(shown(&b), before, "{id}");
    }
}

/// Kaydır over Çoklu çizgi: the drag moves the view and places no corner;
/// after Esc the preview follows the pointer from the corners placed before.
#[test]
fn a_pan_over_a_polyline_moves_the_view_and_places_no_corner() {
    let mut b = line_with_points();
    let draft = shown(&b).preview.expect("the polyline's preview").path;
    over(&mut b, "pan");
    assert!(b.session.prompt().text().starts_with("Kaydır:"));
    // The polyline's draft waits under the pan, drawn as it was, with no tag of its own.
    let waiting = b
        .session
        .preview(&kentos_interaction::Format::default())
        .expect("a preview");
    assert!(waiting.strokes.iter().any(|s| s.pts == draft && !s.closed));
    assert!(waiting.tag.is_none() && waiting.path.is_empty());
    b.drag([0.0, 0.0], [8.0, 4.0]);
    assert_eq!(b.views.len(), 2);
    assert!(b.views.iter().all(|v| matches!(v, ViewChange::Pan { .. })));
    back(&mut b);
    assert_eq!(b.session.point_count(), 2);
    b.move_to(10.0, -3.0);
    let preview = b
        .session
        .preview(&kentos_interaction::Format::default())
        .expect("a preview");
    let end = *preview.path.last().expect("the cursor's segment");
    assert!((end.x - (E + 10.0)).abs() < 1e-9 && (end.y - (N - 3.0)).abs() < 1e-9);
    // The polyline goes on where it was.
    b.click(10.0, -3.0);
    assert_eq!(b.session.point_count(), 3);
    b.confirm();
    assert!(!b.session.is_running() || b.session.point_count() == 0);
}

/// Pencere yakınlaştır over Daire after its centre: its box is the view,
/// it ends by itself, and the second corner's release is not the circle's.
#[test]
fn a_zoom_window_ends_by_itself_and_its_release_is_its_own() {
    let mut b = Bench::new("circle");
    b.click(0.0, 0.0);
    b.move_to(3.0, 4.0);
    let before = shown(&b);
    over(&mut b, "zoomWindow");
    b.click(-10.0, -10.0);
    b.click(10.0, 10.0);
    assert!(!b.session.nested(), "the box ends it");
    assert!(b.session.take_resumed());
    assert!(matches!(b.views.as_slice(), [ViewChange::Fit { .. }]));
    assert!(
        b.doc.entities().next().is_none(),
        "the circle took no radius"
    );
    b.move_to(3.0, 4.0);
    assert_eq!(shown(&b), before);
}

/// Mesafe ölç over Taşı after its base point: it measures and says so,
/// writes nothing, and Taşı goes on to move the selection.
#[test]
fn a_measure_over_move_says_its_length_and_move_goes_on() {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.selection.set([Slot(1)]);
    let from = line_a(&b);
    b.start("move");
    b.click(0.0, 0.0);
    b.move_to(2.0, 0.0);
    let before = shown(&b);
    let count = b.doc.entities().count();
    over(&mut b, "measure");
    b.click(0.0, 0.0);
    b.click(3.0, 4.0);
    b.confirm();
    assert!(
        b.log.iter().any(|l| l.text.starts_with("Toplam uzunluk 5")),
        "{:?}",
        b.log.iter().map(|l| &l.text).collect::<Vec<_>>()
    );
    // Enter with nothing measured ends it.
    b.confirm();
    assert!(!b.session.nested());
    assert!(b.session.take_resumed());
    assert_eq!(b.doc.entities().count(), count, "measuring writes nothing");
    b.move_to(2.0, 0.0);
    assert_eq!(shown(&b), before);
    b.click(2.0, 0.0);
    assert!(!b.session.is_running(), "Taşı moved and left");
    let to = line_a(&b);
    assert_eq!(
        [to[0] - from[0], to[1] - from[1]],
        [2.0, 0.0],
        "moved 2 m east"
    );
}

/// Line 1's first end, from (E, N).
fn line_a(b: &Bench) -> [f64; 2] {
    let Some(Entity::Line(l)) = b.doc.get(Slot(1)) else {
        panic!("line 1");
    };
    common::rel(l.a)
}

/// Döndür with Kopya on and Referans's first point given: a pan keeps both.
#[test]
fn rotate_keeps_its_copy_option_and_its_reference_point() {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.selection.set([Slot(1)]);
    b.start("rotate");
    b.click(0.0, 0.0);
    assert!(b.type_text("K"));
    assert!(b.type_text("R"));
    b.click(5.0, 0.0);
    b.move_to(5.0, 5.0);
    let before = shown(&b);
    over(&mut b, "pan");
    b.drag([1.0, 1.0], [2.0, 2.0]);
    back(&mut b);
    b.move_to(5.0, 5.0);
    assert_eq!(shown(&b), before);
}

/// The command's locks wait with it: the transparent tool starts with none,
/// and Esc gives them back.
#[test]
fn the_commands_locks_wait_with_it() {
    let mut b = line_with_points();
    assert!(b.run(|s, cx| s.lock_length(3.0, cx)));
    assert_eq!(b.locks.length, Some(3.0));
    b.move_to(6.0, 2.0);
    let before = shown(&b);
    over(&mut b, "measure");
    assert_eq!(
        b.locks.length, None,
        "the measure is not held by the line's lock"
    );
    back(&mut b);
    assert_eq!(b.locks.length, Some(3.0));
    b.move_to(6.0, 2.0);
    assert_eq!(shown(&b), before);
}

/// A transparent tool over another takes its place; with no command, or
/// over itself, it starts as any tool; a tool that writes never runs over.
#[test]
fn what_runs_over_what() {
    let mut b = line_with_points();
    assert!(
        !b.session.can_start_over("line"),
        "a drawing tool ends the command"
    );
    assert!(!b.session.can_start_over("erase"));
    assert!(
        !b.session.can_start_over("area"),
        "Alan hesapla may draw its area"
    );
    over(&mut b, "pan");
    over(&mut b, "zoomWindow");
    assert_eq!(b.session.tool_id(), "polyline");
    back(&mut b);
    assert_eq!(b.session.point_count(), 2, "one Esc: the polyline is back");
    // With nothing running, Kaydır is a tool of its own.
    let mut b = Bench::new("pan");
    assert!(!b.session.nested());
    assert!(!b.session.can_start_over("pan"));
    assert!(
        !b.session.can_start_over("zoomWindow"),
        "Kaydır alone is replaced"
    );
    b.run(|s, _| s.exit());
    assert!(!b.session.can_start_over("pan"), "nothing to run over");
}

/// Run over a command, Kaydır and Pencere yakınlaştır take Enter: they end
/// and the command is back (alone, Enter repeats the last command instead).
#[test]
fn enter_ends_a_pan_or_a_zoom_window_run_over_a_command() {
    for id in ["pan", "zoomWindow"] {
        let mut b = line_with_points();
        let before = shown(&b);
        over(&mut b, id);
        assert!(b.session.confirms(), "{id} takes Enter over a command");
        b.confirm();
        assert!(!b.session.nested(), "{id}");
        assert!(b.session.take_resumed());
        b.move_to(6.0, 2.0);
        assert_eq!(shown(&b), before, "{id}");
    }
}

/// A command started while a transparent tool runs ends both, as any start.
#[test]
fn starting_a_command_ends_the_transparent_tool_and_the_command() {
    let mut b = line_with_points();
    over(&mut b, "pan");
    b.start("circle");
    assert!(!b.session.nested());
    assert_eq!(b.session.tool_id(), "circle");
    assert_eq!(b.session.point_count(), 0);
}

/// The point calculator runs over a transparent Mesafe ölç: its point is
/// the measure's; the command under both waits.
#[test]
fn the_calculator_over_a_transparent_measure_gives_it_its_point() {
    let mut b = line_with_points();
    let before = shown(&b);
    over(&mut b, "measure");
    assert!(b.session.can_calc_point());
    b.run(|s, cx| {
        s.nest(
            Box::new(PointCalc::new(CalcKind::Mid)),
            "Nokta hesabı".into(),
            cx,
        )
    });
    assert!(!b.session.can_calc_point(), "not over itself");
    assert!(!b.session.can_start_over("measure") || b.session.top_id() != Some("measure"));
    b.click(0.0, 0.0);
    b.click(2.0, 0.0);
    assert_eq!(
        b.session.top_id(),
        Some("measure"),
        "the middle went to the measure"
    );
    assert_eq!(b.session.point_count(), 1);
    back(&mut b);
    b.move_to(6.0, 2.0);
    assert_eq!(shown(&b), before);
}

/// The point calculator, ended with Esc over a command, tells the host too.
#[test]
fn the_calculator_tells_the_host_when_the_command_is_back() {
    let mut b = line_with_points();
    let before = shown(&b);
    b.run(|s, cx| {
        s.nest(
            Box::new(PointCalc::new(CalcKind::Mid)),
            "Nokta hesabı".into(),
            cx,
        )
    });
    back(&mut b);
    b.move_to(6.0, 2.0);
    assert_eq!(shown(&b), before);
}

/// A hot grip waits under Kaydır as a command does: after Esc it is still
/// hot, and the next click places it.
#[test]
fn a_hot_grip_waits_under_a_pan() {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.selection.set([Slot(1)]);
    b.click(-24.0, -12.0);
    assert!(b.session.grip_active());
    b.move_to(-24.0, -4.0);
    let before = shown(&b);
    over(&mut b, "pan");
    assert!(!b.session.grip_active(), "the pan has the pointer");
    assert_eq!(b.session.tool_id(), "select");
    b.drag([0.0, 0.0], [4.0, 4.0]);
    back(&mut b);
    assert!(b.session.grip_active());
    b.move_to(-24.0, -4.0);
    assert_eq!(shown(&b), before);
    b.click(-24.0, -4.0);
    let Some(Entity::Line(l)) = b.doc.get(Slot(1)) else {
        panic!("line 1");
    };
    assert_eq!(common::rel(l.a), [-24.0, -4.0]);
}
