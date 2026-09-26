//! Esnet, Dizi, Kutupsal dizi and Hizala through the session, over the
//! native document: the behaviour of the web's `StretchTool`, `ArrayTool`,
//! `PolarArrayTool` and `AlignTool` beyond what the interaction traces fix,
//! one rule per test (docs/adr/0047, part 2). The drawing is the traces'
//! `objects.kcad`: lines 1–3, the closed area 4, the point 5, a line on a
//! locked layer (6) and one on a hidden layer (7). Expected values are
//! worked out by hand.

mod common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::Level;

const OBJECTS: &str = include_str!("../../../../fixtures/interaction/v1/objects.kcad");

/// The objects drawing with nothing running and snapping off; `selected` selected.
fn bench(selected: &[u32]) -> Bench {
    bench_on(OBJECTS, selected)
}

fn bench_on(drawing: &str, selected: &[u32]) -> Bench {
    let mut b = Bench::on(drawing);
    b.draft.snap = false;
    b.selection.set(selected.iter().map(|s| Slot(*s)));
    b
}

/// The objects drawing with more objects on its drawing layer (`cizim`),
/// from slot 8: each given as its kind and geometry fields, JSON.
fn with(objects: &[String]) -> String {
    let more: String = objects
        .iter()
        .enumerate()
        .map(|(i, o)| format!(r#",{{"id":{},"layerId":"cizim","attrs":{{}},{o}}}"#, 8 + i))
        .collect();
    assert!(OBJECTS.contains(r#"}],"styles""#), "the object list's end");
    OBJECTS.replacen(r#"}],"styles""#, &format!(r#"}}{more}],"styles""#), 1)
}

/// A point as JSON, east and north of (E, N).
fn p(de: f64, dn: f64) -> String {
    format!(r#"{{"x":{},"y":{}}}"#, 487000.0 + de, 4420000.0 + dn)
}

fn line(b: &Bench, slot: u32) -> [[f64; 2]; 2] {
    let Some(Entity::Line(l)) = b.doc.get(Slot(slot)) else {
        panic!("line {slot}: {:?}", b.doc.get(Slot(slot)));
    };
    [rel(l.a), rel(l.b)]
}

fn near(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
    a.len() == b.len()
        && a.iter()
            .flatten()
            .zip(b.iter().flatten())
            .all(|(x, y)| (x - y).abs() < 1e-9)
}

// ── Esnet ───────────────────────────────────────────────────────────────────

#[test]
fn stretch_asks_for_its_window_then_its_points() {
    let mut b = bench(&[]);
    b.start("stretch");
    let step = |b: &Bench| b.session.prompt().text();
    assert_eq!(
        step(&b),
        "Esnet: taşınacak köşeleri içine alan pencerenin ilk köşesini belirtin"
    );
    // Typed text means nothing before the window.
    assert!(!b.type_text("@1,1"));
    b.click(20.0, 12.0);
    assert_eq!(step(&b), "Esnet: pencerenin karşı köşesini belirtin");
    // The window being drawn shows as a crossing box whichever way it goes.
    b.move_to(28.0, 18.0);
    let window = b.session.select_box().expect("the window is drawn");
    assert!(window.crossing());
    b.click(28.0, 18.0);
    assert_eq!(step(&b), "Esnet: 1 nesne için temel noktayı belirtin");
    assert!(b.session.select_box().is_none());
    // The window and the object as it is (no displacement yet) are drawn.
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert_eq!(preview.strokes.len(), 2, "the window and the line");
    b.click(24.0, 16.0);
    assert_eq!(step(&b), "Esnet: hedef noktayı belirtin ya da @dY,dX yazın");
    b.move_to(26.0, 15.0);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert_eq!(
        preview.tag.as_ref().map(|t| t.lines.clone()),
        Some(vec!["2.236 m".to_owned()])
    );
    assert!(b.type_text("@2,-1"));
    assert_eq!(b.session.tool_id(), "select");
    assert!(near(&line(&b, 2), &[[0.0, 4.0], [26.0, 15.0]]));
    assert_eq!(
        b.last_text(),
        Some("1 nesne esnetildi: ΔY 2.000  ΔX -1.000")
    );
    assert_eq!(b.doc.undo(), Some("Esnet".into()));
}

#[test]
fn a_window_with_no_vertex_of_an_editable_object_is_asked_again() {
    let mut b = bench(&[]);
    b.start("stretch");
    // Crossing line 2 between its ends: it has no vertex in the window.
    b.click(10.0, 6.0);
    b.click(14.0, 12.0);
    assert_eq!(
        b.last_text(),
        Some("Pencerede köşesi olan düzenlenebilir nesne yok; yeniden deneyin.")
    );
    assert_eq!(
        b.session.prompt().text(),
        "Esnet: taşınacak köşeleri içine alan pencerenin ilk köşesini belirtin"
    );
    // The locked line's end is in this one: it is left out, said first.
    let before = b.log.len();
    b.drag([2.0, -18.0], [8.0, -14.0]);
    assert_eq!(
        b.said(before),
        [
            (Level::Warn, "1 nesne kilitli katmanda olduğu için atlandı."),
            (
                Level::Warn,
                "Pencerede köşesi olan düzenlenebilir nesne yok; yeniden deneyin."
            ),
        ]
    );
    // Esc leaves; nothing was written.
    b.run(|s, cx| s.cancel(cx));
    assert_eq!(b.session.tool_id(), "select");
    assert!(!b.doc.can_undo());
}

/// The core stretches every kind; dimensions and hatches are written through
/// `cad.entities.edit` too, their other fields kept; an arc keeps its bow.
#[test]
fn a_dimension_a_hatch_and_an_arc_stretch_in_one_step() {
    let drawing = with(&[
        format!(
            r#""kind":"hatch","ring":[{},{},{},{}],"pattern":{{"type":"lines","angle":45,"spacing":1.5}}"#,
            p(-28.0, -20.0),
            p(-20.0, -20.0),
            p(-20.0, -16.0),
            p(-28.0, -16.0)
        ),
        format!(
            r#""kind":"dimension","a":{},"b":{},"offset":2,"height":0.5,"style":"aligned""#,
            p(-28.0, 18.0),
            p(-20.0, 18.0)
        ),
        format!(
            r#""kind":"arc","c":{},"r":4,"a0":0,"a1":{}"#,
            p(-24.0, -24.0),
            std::f64::consts::FRAC_PI_2
        ),
    ]);
    let mut b = bench_on(&drawing, &[]);
    b.start("stretch");
    // A window over the east ends of the hatch and the dimension, and the arc's start.
    b.drag([-22.0, -30.0], [-18.0, 20.0]);
    assert_eq!(
        b.session.prompt().text(),
        "Esnet: 3 nesne için temel noktayı belirtin"
    );
    b.click(-20.0, 0.0);
    assert!(b.type_text("@2,0"));
    assert_eq!(b.last_text(), Some("3 nesne esnetildi: ΔY 2.000  ΔX 0.000"));
    let Some(Entity::Hatch(h)) = b.doc.get(Slot(8)) else {
        panic!("the hatch");
    };
    let ring: Vec<[f64; 2]> = h.ring.iter().map(|q| rel(*q)).collect();
    assert!(near(
        &ring,
        &[
            [-28.0, -20.0],
            [-18.0, -20.0],
            [-18.0, -16.0],
            [-28.0, -16.0]
        ]
    ));
    assert_eq!((h.pattern.angle, h.pattern.spacing), (45.0, 1.5));
    let Some(Entity::Dimension(d)) = b.doc.get(Slot(9)) else {
        panic!("the dimension");
    };
    assert!(near(&[rel(d.a), rel(d.b)], &[[-28.0, 18.0], [-18.0, 18.0]]));
    assert_eq!((d.offset, d.height), (2.0, 0.5));
    let Some(Entity::Arc(a)) = b.doc.get(Slot(10)) else {
        panic!("the arc");
    };
    // A new arc: its start (−20, −24) moved to (−18, −24), its end (−24, −20) stayed.
    let at = |angle: f64| {
        [
            a.c.x + a.r * angle.cos() - 487000.0,
            a.c.y + a.r * angle.sin() - 4420000.0,
        ]
    };
    let ends = [at(a.a0), at(a.a1)];
    assert!(
        ends.iter()
            .zip([[-18.0, -24.0], [-24.0, -20.0]])
            .all(|(e, w)| (e[0] - w[0]).abs() < 1e-6 && (e[1] - w[1]).abs() < 1e-6),
        "{ends:?}"
    );
    // One step takes all three back.
    assert_eq!(b.doc.undo(), Some("Esnet".into()));
    assert!(!b.doc.can_undo());
}

#[test]
fn with_a_selection_only_selected_objects_stretch() {
    let mut b = bench(&[2]);
    b.start("stretch");
    b.click(-2.0, 2.0);
    b.click(2.0, 18.0);
    assert_eq!(
        b.session.prompt().text(),
        "Esnet: 1 nesne için temel noktayı belirtin"
    );
    b.click(0.0, 4.0);
    b.click(0.0, 2.0);
    assert!(near(&line(&b, 2), &[[0.0, 2.0], [24.0, 16.0]]));
    assert!(near(&line(&b, 3), &[[0.0, 16.0], [16.0, 4.0]]));
}

// ── Dizi ────────────────────────────────────────────────────────────────────

#[test]
fn the_array_reads_its_counts_and_spacing_as_the_web_does() {
    let mut b = bench(&[1]);
    b.start("array");
    assert_eq!(
        b.session.prompt().text(),
        "Dizi: satır ve sütun sayısını yazın, ör. 2,3 (Enter: 2,3)"
    );
    // Clicks mean nothing before the spacing; a typed point is not read.
    b.click(-24.0, -12.0);
    assert!(!b.type_text("@1,1"));
    assert!(!b.type_text("3"));
    // Counts are rounded; out of range they are refused and asked again.
    assert!(b.type_text("0,5"));
    assert_eq!(
        b.last_text(),
        Some("Satır × sütun 2 ile 10 000 arasında olmalı.")
    );
    assert!(b.type_text("1.6;2.4"));
    assert_eq!(
        b.session.prompt().text(),
        "Dizi: sütun ve satır aralığını yazın dY,dX (Enter: 10,10) ya da iki nokta gösterin"
    );
    // A typed point is not a spacing either.
    assert!(!b.type_text("@4,4"));
    // Before the first point the ghosts are the last spacing's: 2 × 2 − 1 copies.
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert_eq!(preview.strokes.len(), 3);
    assert!(b.type_text("-4 6"));
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.last_text(), Some("2 × 2 dizi oluşturuldu: 3 yeni nesne."));
    assert!(near(&line(&b, 10), &[[-28.0, -6.0], [-12.0, -6.0]]));
    assert_eq!(
        (
            b.memory.array_rows,
            b.memory.array_cols,
            b.memory.array_dx,
            b.memory.array_dy
        ),
        (2, 2, -4.0, 6.0)
    );
    assert_eq!(b.doc.undo(), Some("Dizi".into()));
}

#[test]
fn a_refused_array_waits_for_another_spacing_and_is_not_remembered() {
    let mut b = bench(&[1]);
    b.start("array");
    assert!(b.type_text("1,3"));
    assert!(b.type_text("0,5"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some(
            "Sütunlar arasındaki aralık (dY) sıfır; kopyalar üst üste düşer. Sıfırdan farklı bir aralık verin."
        )
    );
    assert_eq!(b.session.tool_id(), "array");
    assert_eq!(b.memory.array_cols, 3, "nothing remembered yet");
    assert_eq!(b.memory.array_dx, 10.0);
    // Enter now: the last spacing (10, 10).
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.doc.entities().count(), 9);
}

// ── Kutupsal dizi ───────────────────────────────────────────────────────────

#[test]
fn the_polar_array_asks_its_options_and_checks_them() {
    let mut b = bench(&[3]);
    b.start("arrayPolar");
    assert_eq!(
        b.session.prompt().text(),
        "Kutupsal dizi: dizinin merkezini gösterin"
    );
    // Enter before the centre leaves.
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");

    b.start("arrayPolar");
    b.click(0.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Kutupsal dizi: uygulamak için sağ tıklayın [Adet (N): 6 / Açı (A): 360° / Nesneleri döndür (D): evet]"
    );
    // The copies show: five ghosts.
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert_eq!(preview.strokes.len(), 5);
    b.move_to(10.0, 10.0);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert_eq!(
        preview.tag.map(|t| t.lines),
        Some(vec!["6 adet · 360°".to_owned()])
    );
    // A number alone is the count; a count must be whole, 2 to 1000.
    assert!(b.type_text("2.5"));
    assert_eq!(
        b.last_text(),
        Some("Adet 2 ile 1000 arasında bir tam sayı olmalı.")
    );
    assert!(b.type_text("3"));
    assert_eq!(b.memory.polar_count, 3);
    // Açı (A): a fill of zero or past a full turn is refused.
    assert!(b.type_text("A"));
    assert_eq!(
        b.session.prompt().text(),
        "Kutupsal dizi: doldurma açısını derece olarak yazın (360 tam tur; eksi saat yönünde)"
    );
    assert!(b.type_text("-400"));
    assert_eq!(
        b.last_text(),
        Some("Doldurma açısı 0 ile ±360 derece arasında olmalı.")
    );
    // Enter leaves the question; text that is no number is not read.
    b.confirm();
    assert_eq!(b.session.tool_id(), "arrayPolar");
    assert!(!b.type_text("@1,1"));
    assert!(b.type_text("d"));
    assert!(!b.memory.polar_rotate);
    // Put the session's values back as they were.
    assert!(b.type_text("6"));
    assert!(b.type_text("D"));
    assert_eq!(
        (
            b.memory.polar_count,
            b.memory.polar_fill,
            b.memory.polar_rotate
        ),
        (6, 360.0, true)
    );
    assert!(!b.doc.can_undo(), "nothing written");
}

/// Copies that do not turn are placed by the middle of what is copied: the
/// locked line is not copied and does not move the middle (the command's
/// rule, and the preview's).
#[test]
fn a_locked_object_does_not_move_the_middle() {
    let mut b = bench(&[3, 6]);
    b.memory.polar_count = 2;
    b.memory.polar_fill = 180.0;
    b.memory.polar_rotate = false;
    b.start("arrayPolar");
    b.click(0.0, 0.0);
    let ghosts = b
        .session
        .preview(&Default::default())
        .expect("a preview")
        .strokes;
    let before = b.log.len();
    b.confirm();
    assert_eq!(
        b.said(before),
        [
            (
                Level::Warn,
                "1 nesne kilitli katmanda olduğu için atlandı. Kopyalamak için katmanın kilidini Katmanlar panelinden açın."
            ),
            (
                Level::Success,
                "Kutupsal dizi: 2 adet, 180° içinde, 1 yeni nesne."
            ),
        ]
    );
    // Line 3's box has its middle at (8, 10): half a turn moves it by (−16, −20).
    assert!(near(&line(&b, 8), &[[-16.0, -4.0], [0.0, -16.0]]));
    // The preview showed the copy where it went (and the locked line's ghost beside it).
    let copy: Vec<[f64; 2]> = ghosts[0]
        .pts
        .iter()
        .map(|q| [q.x - 487000.0, q.y - 4420000.0])
        .collect();
    assert!(near(&copy, &[[-16.0, -4.0], [0.0, -16.0]]), "{copy:?}");
}

// ── Hizala ──────────────────────────────────────────────────────────────────

#[test]
fn align_asks_its_pairs_and_toggles_its_scale() {
    let mut b = bench(&[1]);
    b.start("align");
    let step = |b: &Bench| b.session.prompt().text();
    assert_eq!(step(&b), "Hizala: birinci kaynak noktasını gösterin");
    b.click(-24.0, -12.0);
    assert_eq!(step(&b), "Hizala: birinci hedef noktasını gösterin");
    // The ghost follows the cursor from the source point.
    b.move_to(-20.0, -10.0);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert_eq!(
        preview.strokes.len(),
        2,
        "the ghost and the line from the source"
    );
    assert!(b.type_text("@0,8"));
    assert_eq!(
        step(&b),
        "Hizala: ikinci kaynak noktasını gösterin ya da yalnızca taşımak için sağ tıklayın [Ölçekle (Ö): hayır]"
    );
    // A source point on the target just given is let go.
    b.click(-24.0, -4.0);
    assert!(step(&b).starts_with("Hizala: ikinci kaynak"));
    assert!(b.type_text("ö"));
    assert!(b.memory.align_scale);
    b.click(-8.0, -12.0);
    assert_eq!(
        step(&b),
        "Hizala: ikinci hedef noktasını gösterin [Ölçekle (Ö): evet]"
    );
    assert!(b.type_text("O"));
    assert!(!b.memory.align_scale, "O is Ö too");
    b.click(-8.0, -4.0);
    assert_eq!(b.last_text(), Some("1 nesne hizalandı."));
    assert!(near(&line(&b, 1), &[[-24.0, -4.0], [-8.0, -4.0]]));
    assert_eq!(b.doc.undo(), Some("Hizala".into()));
}

#[test]
fn a_locked_object_is_not_aligned() {
    let mut b = bench(&[6]);
    b.start("align");
    b.click(4.0, -16.0);
    b.click(8.0, -16.0);
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(
        b.last_text(),
        Some(
            "1 nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."
        )
    );
    assert!(!b.doc.can_undo());
}
