//! Tüm köşeleri yuvarla, Tüm köşelere pah, Yönü çevir and Sadeleştir through
//! the session, over the native document (docs/adr/0140): selected before or
//! after, drawn live, a typed value, one undo step named after the tool,
//! objects on a locked layer refused, nothing to do said plainly. The drawing
//! is the traces' `edits.kcad` (a locked layer `kilitli`, its line 13) with the
//! objects each test adds north of y = 22 m. Expected values are worked out by hand.

mod common;

use common::Bench;
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::{Level, MarkerShape, Tone};

const EDITS: &str = include_str!("../../../../fixtures/interaction/v1/edits.kcad");

fn bench() -> Bench {
    let mut b = Bench::on(EDITS);
    b.draft.snap = false;
    b
}

/// An open polyline with two square corners: 10 m east, 10 m north, 10 m west.
const U: [[f64; 2]; 4] = [[0.0, 22.0], [10.0, 22.0], [10.0, 32.0], [0.0, 32.0]];

fn near(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
    a.len() == b.len()
        && a.iter()
            .flatten()
            .zip(b.iter().flatten())
            .all(|(x, y)| (x - y).abs() < 1e-9)
}

fn preview(b: &Bench) -> kentos_interaction::Preview {
    b.session.preview(&Default::default()).expect("a preview")
}

// ── Tüm köşeleri yuvarla ────────────────────────────────────────────────────

#[test]
fn fillet_all_rounds_every_corner_at_the_typed_radius() {
    let mut b = bench();
    let slot = b.add_path("cizim", &U, false);
    let rev = b.doc.revision();
    b.selection.set([slot]);
    b.start("filletAll");
    assert_eq!(
        b.session.prompt().text(),
        "Tüm köşeleri yuvarla: yarıçapı yazın [yarıçap 1.000 m; 2 köşe yuvarlanacak; Uygula (Enter)]"
    );
    assert_eq!(b.options(), ["Enter"]);
    // The rounded outline is drawn live, the tag beside the cursor counts.
    b.move_to(20.0, 27.0);
    let p = preview(&b);
    assert!(
        p.strokes
            .iter()
            .any(|s| s.width == 2.5 && s.tone == Tone::Accent)
    );
    assert_eq!(p.tag.expect("a tag").lines, ["2 köşe yuvarlanacak"]);
    // A typed radius is the new preview and stays in the session's memory.
    assert!(b.type_text("2"));
    assert_eq!(b.memory.fillet_radius, Some(2.0));
    assert!(b.session.prompt().text().contains("yarıçap 2.000 m"));
    assert_eq!(
        b.doc.revision(),
        rev,
        "nothing is written before the confirm"
    );
    b.confirm();
    assert_eq!(b.last_text(), Some("2 köşe yuvarlandı."));
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.session.tool_id(), "select", "the tool leaves");
    // Each corner is two vertices and an arc: the first at 2 m from the corner.
    let pts = b.path_pts(slot);
    assert!(near(
        &pts,
        &[
            [0.0, 22.0],
            [8.0, 22.0],
            [10.0, 24.0],
            [10.0, 30.0],
            [8.0, 32.0],
            [0.0, 32.0]
        ]
    ));
    let Some(Entity::Polyline(p)) = b.doc.get(slot) else {
        panic!("a polyline");
    };
    let bulges = p.bulges.as_ref().expect("arcs");
    assert_eq!(bulges.iter().filter(|v| **v != 0.0).count(), 2);
    assert_eq!(b.doc.undo().as_deref(), Some("Köşe yuvarla"));
    assert!(near(&b.path_pts(slot), &U), "one step brings it back");
}

#[test]
fn fillet_all_picks_first_and_says_what_does_not_fit() {
    let mut b = bench();
    // A 10 m square: a radius of 6 m fits no corner.
    let square = b.add_path(
        "cizim",
        &[[0.0, 22.0], [10.0, 22.0], [10.0, 32.0], [0.0, 32.0]],
        true,
    );
    let rev = b.doc.revision();
    b.start("filletAll");
    assert!(b.session.prompt().text().starts_with(
        "Tüm köşeleri yuvarla: nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın (0 seçili) [yarıçap 1.000 m; değiştirmek için değer yazın]"
    ));
    // A number typed while picking is the radius.
    assert!(b.type_text("6"));
    assert_eq!(b.memory.fillet_radius, Some(6.0));
    b.click(5.0, 22.0);
    assert_eq!(b.selected(), [square.0]);
    b.confirm();
    assert!(
        b.session.is_running(),
        "the selection is confirmed, the tool goes on"
    );
    let p = preview(&b);
    assert!(
        p.strokes
            .iter()
            .any(|s| s.tone == Tone::Danger && s.dash.is_some()),
        "what does not fit is drawn as refused"
    );
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some(
            "Hiçbir köşe yuvarlanamadı: 4 köşe sığmadı ya da yaya komşu. Daha küçük bir değer yazın."
        )
    );
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.doc.revision(), rev);
    assert_eq!(
        b.session.tool_id(),
        "filletAll",
        "the tool stays for another value"
    );
    // A radius that fits rounds the four corners.
    assert!(b.type_text("1"));
    b.confirm();
    assert_eq!(b.last_text(), Some("4 köşe yuvarlandı."));
    assert_eq!(b.path_pts(square).len(), 8);
    assert_eq!(b.doc.undo().as_deref(), Some("Köşe yuvarla"));
}

#[test]
fn a_bad_value_is_said_and_the_old_one_stays() {
    let mut b = bench();
    let slot = b.add_path("cizim", &U, false);
    b.selection.set([slot]);
    b.start("filletAll");
    assert!(b.type_text("3"));
    assert!(b.type_text("0"));
    assert_eq!(b.last_text(), Some("Yarıçap sıfırdan büyük olmalı."));
    assert_eq!(b.memory.fillet_radius, Some(3.0));
    assert!(!b.type_text("abc"), "text that is not a value is not taken");
}

#[test]
fn a_selection_with_no_corners_to_round_is_said_and_left() {
    let mut b = bench();
    // A line has no corner.
    let rev = b.doc.revision();
    b.selection.set([Slot(1)]);
    b.start("filletAll");
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(
        b.last_text(),
        Some(
            "Seçimde çoklu çizgi ya da kapalı alan yok; Tüm köşeleri yuvarla yalnız onların köşelerini yuvarlar."
        )
    );
    assert_eq!(b.doc.revision(), rev);
}

#[test]
fn a_locked_layer_is_refused_and_esc_leaves_writing_nothing() {
    let mut b = bench();
    let locked = b.add_path("kilitli", &U, false);
    let rev = b.doc.revision();
    b.selection.set([locked]);
    b.start("filletAll");
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(
        b.last_text(),
        Some("1 nesne kilitli katmanda olduğu için atlandı.")
    );
    assert_eq!(b.doc.revision(), rev);
    // A locked and an open object: the open one is rounded, the locked one is said.
    let open = b.add_path("cizim", &U, false);
    let rev = b.doc.revision();
    b.selection.set([locked, open]);
    b.start("filletAll");
    assert!(b.session.is_running());
    // Esc leaves; the drawing is as it was.
    assert!(!b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.doc.revision(), rev);
    assert!(near(&b.path_pts(open), &U));
    // The locked object is left out of what is written; one undo step brings the open one back.
    b.selection.set([locked, open]);
    b.start("filletAll");
    b.confirm();
    assert!(
        near(&b.path_pts(locked), &U),
        "the locked object is untouched"
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Köşe yuvarla"));
    assert!(near(&b.path_pts(open), &U), "one step undone");
}

// ── Tüm köşelere pah ────────────────────────────────────────────────────────

#[test]
fn chamfer_all_cuts_every_corner_with_one_or_two_distances() {
    let mut b = bench();
    let slot = b.add_path("cizim", &U, false);
    b.selection.set([slot]);
    b.start("chamferAll");
    assert_eq!(
        b.session.prompt().text(),
        "Tüm köşelere pah: mesafeyi yazın: d ya da d1,d2 [pah 1.000 m; 2 köşe pahlanacak; Uygula (Enter)]"
    );
    assert!(b.type_text("1,2"));
    assert_eq!(b.memory.chamfer, Some((1.0, 2.0)));
    assert!(b.session.prompt().text().contains("pah 1.000 ile 2.000 m"));
    assert!(b.type_text("2"));
    assert_eq!(b.memory.chamfer, Some((2.0, 2.0)));
    b.confirm();
    assert_eq!(b.last_text(), Some("2 köşeye pah kırıldı."));
    assert!(near(
        &b.path_pts(slot),
        &[
            [0.0, 22.0],
            [8.0, 22.0],
            [10.0, 24.0],
            [10.0, 30.0],
            [8.0, 32.0],
            [0.0, 32.0]
        ]
    ));
    assert_eq!(b.doc.undo().as_deref(), Some("Pah"));
    assert!(near(&b.path_pts(slot), &U));
}

// ── Yönü çevir ──────────────────────────────────────────────────────────────

#[test]
fn reverse_shows_the_new_direction_and_writes_one_step() {
    let mut b = bench();
    let path = b.add_path("cizim", &U, false);
    let line = b.add_line("cizim", [0.0, 36.0], [10.0, 36.0]);
    // And the traces' L-shaped polyline, slot 4.
    b.selection.set([path, line, Slot(4)]);
    b.start("reverse");
    assert_eq!(
        b.session.prompt().text(),
        "Yönü çevir: yeni yönü görün [3 nesnenin yönü çevrilecek; Uygula (Enter)]"
    );
    let p = preview(&b);
    // A ring at each new start, and an arrow along each.
    assert_eq!(
        p.markers
            .iter()
            .filter(|m| matches!(m.shape, MarkerShape::Ring(_)))
            .count(),
        3
    );
    assert!(p.strokes.iter().any(|s| s.pts.len() == 3 && s.width == 2.0));
    b.confirm();
    assert_eq!(b.last_text(), Some("3 nesnenin yönü çevrildi."));
    assert!(near(
        &b.path_pts(path),
        &[[0.0, 32.0], [10.0, 32.0], [10.0, 22.0], [0.0, 22.0]]
    ));
    let Some(Entity::Line(l)) = b.doc.get(line) else {
        panic!("a line");
    };
    assert_eq!(
        (l.a.x - common::E, l.b.x - common::E),
        (10.0, 0.0),
        "the ends swapped"
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Yönü çevir"));
    assert!(near(&b.path_pts(path), &U));
}

#[test]
fn reverse_leaves_what_has_no_direction_and_says_so() {
    let mut b = bench();
    let circle = b.doc.add(circle()).expect("a slot");
    b.selection.set([circle]);
    b.start("reverse");
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(
        b.last_text(),
        Some("Seçimde yönü çevrilebilecek çizgi, çoklu çizgi, eğri ya da kapalı alan yok.")
    );
    // Among others, it says how many it left.
    let line = b.add_line("cizim", [0.0, 36.0], [10.0, 36.0]);
    b.selection.set([line, circle]);
    b.start("reverse");
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("1 nesnenin yönü çevrildi. 1 nesne yönü olmadığı için atlandı.")
    );
}

fn circle() -> Entity {
    Entity::Circle(kentos_contracts::CircleEntity {
        base: common::base("cizim"),
        c: kentos_contracts::Vec2 {
            x: common::E + 30.0,
            y: common::N + 30.0,
        },
        r: 3.0,
    })
}

// ── Sadeleştir ──────────────────────────────────────────────────────────────

#[test]
fn simplify_drops_the_vertices_within_the_tolerance_and_says_how_far() {
    let mut b = bench();
    // The second vertex is 4 mm off the line between its neighbours.
    let slot = b.add_path(
        "cizim",
        &[[0.0, 22.0], [5.0, 22.004], [10.0, 22.0], [10.0, 32.0]],
        false,
    );
    let rev = b.doc.revision();
    b.selection.set([slot]);
    b.start("simplify");
    assert_eq!(
        b.session.prompt().text(),
        "Sadeleştir: toleransı yazın (m) [tolerans 0.010 m; 1 köşe atılacak, en büyük sapma 0.004 m; Uygula (Enter)]"
    );
    let p = preview(&b);
    assert_eq!(
        p.markers
            .iter()
            .filter(|m| matches!(m.shape, MarkerShape::Cross(_)) && m.tone == Tone::Danger)
            .count(),
        1,
        "the vertex that goes is crossed out"
    );
    // A tolerance below the deviation keeps it.
    assert!(b.type_text("0.001"));
    assert_eq!(b.memory.simplify_tolerance, 0.001);
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Bu toleransla atılacak köşe yok; daha büyük bir tolerans yazın.")
    );
    assert_eq!(b.doc.revision(), rev);
    assert!(b.type_text("0.01"));
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Sadeleştir: 1 köşe atıldı; en büyük sapma 0.004 m.")
    );
    assert_eq!(b.path_pts(slot).len(), 3);
    assert_eq!(b.doc.undo().as_deref(), Some("Sadeleştir"));
    assert_eq!(b.path_pts(slot).len(), 4);
}
