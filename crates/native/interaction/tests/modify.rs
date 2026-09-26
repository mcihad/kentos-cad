//! The move, copy, rotate, scale and mirror tools through the session, over
//! the native document: the behaviour of the web's `SelectionFirstTool` and
//! its tools that the interaction traces fix, one rule per test
//! (docs/adr/0037). The drawing is the traces' `objects.kcad`: lines 1–3,
//! the closed area 4, the point 5, a line on a locked layer (6) and one on a
//! hidden layer (7). Expected values are worked out by hand.

mod common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::Level;

const OBJECTS: &str = include_str!("../../../../fixtures/interaction/v1/objects.kcad");

/// The objects drawing with nothing running and snapping off; `selected` selected.
fn bench(selected: &[u32]) -> Bench {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.selection.set(selected.iter().map(|s| Slot(*s)));
    b
}

fn line(b: &Bench, slot: u32) -> [[f64; 2]; 2] {
    let Some(Entity::Line(l)) = b.doc.get(Slot(slot)) else {
        panic!("line {slot}");
    };
    [rel(l.a), rel(l.b)]
}

fn near(a: [[f64; 2]; 2], b: [[f64; 2]; 2]) -> bool {
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .all(|(x, y)| (x - y).abs() < 1e-9)
}

// ── Picking ─────────────────────────────────────────────────────────────────

#[test]
fn without_a_selection_the_tool_picks_first() {
    let mut b = bench(&[]);
    b.start("move");
    assert_eq!(
        b.session.prompt().text(),
        "Taşı: nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın (0 seçili)"
    );
    // A click turns one object over; a second click on it takes it out again.
    b.click(-16.0, -12.0);
    assert_eq!(b.selected(), [1]);
    assert!(b.session.prompt().text().ends_with("(1 seçili)"));
    b.click(-16.0, -12.0);
    assert!(b.selected().is_empty());
    // Hovering names what a click would pick.
    b.move_to(-16.0, -12.0);
    assert_eq!(b.selection.hover(), Some(Slot(1)));
    // A drag adds what its box holds (left to right: wholly inside), with no Shift.
    b.click(-16.0, -12.0);
    b.drag([-26.0, 2.0], [-10.0, 16.0]);
    assert_eq!(b.selected(), [1, 4]);
    assert!(
        b.session.select_box().is_none(),
        "the box is gone once released"
    );
    // Typed text means nothing while picking.
    assert!(!b.type_text("@1,1"));
    // Enter with something selected: on to the base point.
    b.confirm();
    assert_eq!(
        b.session.prompt().text(),
        "Taşı: 2 nesne için temel noktayı belirtin"
    );
    assert_eq!(b.selection.hover(), None);
}

#[test]
fn enter_with_nothing_selected_leaves() {
    let mut b = bench(&[]);
    b.start("rotate");
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn the_box_shows_while_it_is_drawn() {
    let mut b = bench(&[]);
    b.start("mirror");
    let p = b.snapped(10.0, 10.0);
    b.run(|s, cx| s.pointer_down(&p, cx));
    let p = b.snapped(-10.0, -10.0);
    b.run(|s, cx| s.pointer_move(&p, cx));
    let shown = b.session.select_box().expect("a box");
    assert!(shown.crossing(), "right to left");
}

// ── Taşı, Kopyala ───────────────────────────────────────────────────────────

#[test]
fn a_selection_moves_by_its_base_and_target_in_one_step() {
    let mut b = bench(&[1]);
    b.start("move");
    assert_eq!(
        b.session.prompt().text(),
        "Taşı: 1 nesne için temel noktayı belirtin"
    );
    b.click(-24.0, -12.0);
    assert_eq!(
        b.session.prompt().text(),
        "Taşı: hedef noktayı belirtin ya da @dY,dX yazın"
    );
    // The preview: the line's ghost where it would go, dashed, and the line from the base.
    b.move_to(-20.0, -10.0);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert!(preview.strokes.iter().any(|s| s.dash == Some([4.0, 3.0])));
    assert!(
        preview
            .strokes
            .iter()
            .any(|s| s.dash.is_none() && s.pts.len() == 2)
    );
    assert_eq!(
        preview.tag.as_ref().map(|t| t.lines.clone()),
        Some(vec!["4.472 m".to_owned()])
    );
    let before = b.log.len();
    assert!(b.type_text("@12.5,-7.25"));
    assert!(near(line(&b, 1), [[-11.5, -19.25], [4.5, -19.25]]));
    assert_eq!(
        b.said(before),
        [(Level::Success, "1 nesne taşındı: ΔY 12.500  ΔX -7.250")]
    );
    assert_eq!(b.session.tool_id(), "select", "the move leaves");
    assert_eq!(b.selected(), [1], "the selection stays");
    assert_eq!(b.doc.undo().as_deref(), Some("Taşı"));
    assert!(near(line(&b, 1), [[-24.0, -12.0], [-8.0, -12.0]]));
}

#[test]
fn copies_are_left_at_every_point_until_enter() {
    let mut b = bench(&[5]);
    b.start("copy");
    b.click(20.0, -8.0);
    assert_eq!(
        b.session.prompt().text(),
        "Kopyala: kopyanın yerini belirtin ya da @dY,dX yazın [Bitir (Enter)]"
    );
    assert_eq!(b.options(), ["Enter"]);
    assert!(b.type_text("@0,4"));
    assert!(b.type_text("@0,8"));
    assert_eq!(b.doc.len(), 9);
    let Entity::Point(p) = b.newest() else {
        panic!("a point");
    };
    assert_eq!(rel(p.p), [20.0, 0.0], "each copy from the base");
    assert_eq!(
        b.last_text(),
        Some("1 nesne kopyalandı: ΔY 0.000  ΔX 8.000")
    );
    assert_eq!(b.session.tool_id(), "copy", "still copying");
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.doc.undo().as_deref(), Some("Kopyala"));
    assert_eq!(b.doc.len(), 8, "one step per copy");
}

// ── Döndür ──────────────────────────────────────────────────────────────────

#[test]
fn rotate_by_a_typed_angle_or_as_a_copy() {
    let mut b = bench(&[1]);
    b.start("rotate");
    assert_eq!(
        b.session.prompt().text(),
        "Döndür: dönme merkezini belirtin"
    );
    b.click(-24.0, -12.0);
    assert_eq!(
        b.session.prompt().text(),
        "Döndür: açıyı yazın (derece, saat yönü tersine) ya da bir nokta gösterin [Referans (R) / Kopya (K): kapalı]"
    );
    assert!(b.type_text("k"), "Kopya turns on");
    assert!(b.session.prompt().text().ends_with("Kopya (K): açık]"));
    let before = b.log.len();
    assert!(b.type_text("90"));
    assert_eq!(
        b.said(before),
        [(Level::Success, "1 nesne 90.0000° döndürüldü (kopya).")]
    );
    assert_eq!(b.doc.len(), 8, "a copy");
    let Entity::Line(l) = b.newest() else {
        panic!("a line");
    };
    assert!(near([rel(l.a), rel(l.b)], [[-24.0, -12.0], [-24.0, 4.0]]));
    assert!(
        near(line(&b, 1), [[-24.0, -12.0], [-8.0, -12.0]]),
        "the original stays"
    );
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn rotate_by_a_reference_direction() {
    let mut b = bench(&[2]);
    b.start("rotate");
    b.click(0.0, 4.0);
    assert!(b.type_text("R"));
    assert_eq!(
        b.session.prompt().text(),
        "Döndür: referans doğrultunun ilk noktasını gösterin ya da referans açıyı yazın"
    );
    // The line's own direction as the reference, then north: it turns onto north.
    b.click(0.0, 4.0);
    b.click(24.0, 16.0);
    assert_eq!(
        b.session.prompt().text(),
        "Döndür: yeni doğrultuyu gösterin ya da yeni açıyı yazın [Kopya (K): kapalı]"
    );
    assert!(b.type_text("90"));
    let [a, e] = line(&b, 2);
    assert!((a[0] - 0.0).abs() < 1e-9 && (a[1] - 4.0).abs() < 1e-9);
    assert!(e[0].abs() < 1e-9, "north of the centre: {e:?}");
    assert!((e[1] - 4.0 - (24.0f64.hypot(12.0))).abs() < 1e-9);
}

// ── Ölçekle ─────────────────────────────────────────────────────────────────

#[test]
fn scale_by_a_factor_and_by_reference() {
    let mut b = bench(&[1]);
    b.start("scale");
    b.click(-24.0, -12.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçekle: ölçek faktörünü yazın ya da referans uzunluk için bir nokta gösterin [Referans (R) / Kopya (K): kapalı]"
    );
    let before = b.log.len();
    assert!(b.type_text("0"), "taken, with a warning");
    assert_eq!(
        b.said(before),
        [(Level::Warn, "Ölçek faktörü sıfırdan büyük olmalı.")]
    );
    assert!(b.type_text("2"));
    assert!(near(line(&b, 1), [[-24.0, -12.0], [8.0, -12.0]]));
    assert_eq!(b.last_text(), Some("1 nesne 2.0000 faktörüyle ölçeklendi."));

    // By reference: 32 m (the line as it is now) becomes 8 m.
    b.start("scale");
    b.click(-24.0, -12.0);
    assert!(b.type_text("r"));
    b.click(-24.0, -12.0);
    b.click(8.0, -12.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçekle: yeni uzunluğu temel noktadan gösterin ya da yazın [Kopya (K): kapalı]"
    );
    b.move_to(-12.0, -12.0);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert_eq!(
        preview.tag.map(|t| t.lines),
        Some(vec!["Faktör 0.3750".to_owned()])
    );
    assert!(b.type_text("8"));
    assert!(near(line(&b, 1), [[-24.0, -12.0], [-16.0, -12.0]]));
    assert_eq!(b.last_text(), Some("1 nesne 0.2500 faktörüyle ölçeklendi."));
}

// ── Aynala ──────────────────────────────────────────────────────────────────

#[test]
fn mirror_copies_or_turns_the_source_over() {
    let mut b = bench(&[1]);
    b.start("mirror");
    assert_eq!(
        b.session.prompt().text(),
        "Aynala: simetri ekseninin ilk noktasını belirtin [Kaynağı sil (S): hayır]"
    );
    b.click(0.0, -20.0);
    b.click(0.0, 0.0);
    assert_eq!(b.doc.len(), 8, "a mirrored copy");
    let Entity::Line(l) = b.newest() else {
        panic!("a line");
    };
    assert!(near([rel(l.a), rel(l.b)], [[24.0, -12.0], [8.0, -12.0]]));
    assert_eq!(
        b.last_text(),
        Some("1 nesnenin simetriği kopya olarak oluşturuldu.")
    );
    // Kaynağı sil: the source itself turns over, in place.
    b.start("mirror");
    assert!(b.type_text("s"));
    assert!(
        b.session
            .prompt()
            .text()
            .ends_with("[Kaynağı sil (S): evet]")
    );
    b.click(0.0, -20.0);
    b.click(0.0, 0.0);
    assert_eq!(b.doc.len(), 8, "no copy");
    assert!(near(line(&b, 1), [[24.0, -12.0], [8.0, -12.0]]));
    assert_eq!(b.doc.undo().as_deref(), Some("Aynala"));
}

// ── Kilitli katman ──────────────────────────────────────────────────────────

#[test]
fn locked_objects_stay_and_alone_are_refused() {
    let mut b = bench(&[1, 6]);
    b.start("copy");
    b.click(0.0, 0.0);
    let before = b.log.len();
    assert!(b.type_text("@0,4"));
    assert_eq!(
        b.said(before),
        [
            (
                Level::Warn,
                "1 nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."
            ),
            (Level::Success, "1 nesne kopyalandı: ΔY 0.000  ΔX 4.000"),
        ]
    );
    assert_eq!(b.doc.len(), 8, "only the unlocked one is copied");
    b.confirm();

    let mut b = bench(&[6]);
    b.start("move");
    b.click(0.0, 0.0);
    let before = b.log.len();
    let revision = b.doc.revision();
    assert!(b.type_text("@0,4"));
    assert_eq!(
        b.said(before),
        [(
            Level::Warn,
            "1 nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."
        )]
    );
    assert_eq!(b.doc.revision(), revision, "nothing written");
    assert_eq!(
        b.session.tool_id(),
        "select",
        "the move leaves all the same"
    );
}
