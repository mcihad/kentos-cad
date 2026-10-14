//! Taşı, Kopyala, Döndür and Ölçekle with AutoCAD's MOVE, COPY, ROTATE and SCALE rules (docs/adr/0291):
//! Yer değiştirme (typed, or Enter at the base point, Enter again the last one), Kopyala's Kip (one copy) and Dizi
//! (a count along the displacement, Sığdır sharing it), a turn typed in the project's way (a CAD project's degrees
//! counter-clockwise, a CBS project's grads clockwise; Referans's directions a semt there), Referans's Noktalar for
//! the new direction and the new length. The drawing is the traces' `objects.kcad` (a CBS project, grads): line 1
//! from (−24, −12) to (−8, −12), the point 5. Expected values are worked out by hand.

use crate::common;

use common::{Bench, rel};
use kentos_contracts::{Entity, Workspace};
use kentos_domain::Slot;
use kentos_interaction::Level;

const OBJECTS: &str = include_str!("../../../../../fixtures/interaction/v1/objects.kcad");

fn bench(selected: &[u32]) -> Bench {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.selection.set(selected.iter().map(|s| Slot(*s)));
    b
}

/// The same drawing as a CAD project in degrees.
fn cad(selected: &[u32]) -> Bench {
    let mut b = bench(selected);
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.angle_unit = kentos_contracts::AngleUnit::Deg;
    b.doc.set_settings(settings);
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

fn points(b: &Bench) -> Vec<[f64; 2]> {
    b.doc
        .entities()
        .filter_map(|e| match e {
            Entity::Point(p) => Some(rel(p.p)),
            _ => None,
        })
        .collect()
}

// ── Yer değiştirme ──────────────────────────────────────────────────────────

#[test]
fn a_displacement_is_typed_or_taken_again_with_enter() {
    let mut b = bench(&[1]);
    b.start("move");
    assert!(b.type_text("Y"));
    assert_eq!(b.session.prompt().text(), "Taşı: yer değiştirmeyi yazın: dY,dX");
    let before = b.log.len();
    assert!(b.type_text("12.5,-7.25"));
    assert!(near(line(&b, 1), [[-11.5, -19.25], [4.5, -19.25]]));
    assert_eq!(b.said(before), [(Level::Success, "1 nesne taşındı: ΔY 12.500  ΔX -7.250")]);
    assert_eq!(b.session.tool_id(), "select", "a typed displacement leaves");

    // Enter at the base point: Yer değiştirme, the last one offered; Enter again takes it.
    b.start("move");
    b.confirm();
    assert_eq!(
        b.session.prompt().text(),
        "Taşı: yer değiştirmeyi yazın: dY,dX (Enter: 12.500,-7.250)"
    );
    b.confirm();
    assert!(near(line(&b, 1), [[1.0, -26.5], [17.0, -26.5]]));
    // A polar displacement in the project's way: a CBS project's semt (100g east), 4 m.
    b.start("move");
    assert!(b.type_text("Y"));
    assert!(b.type_text("@4<100"));
    assert!(near(line(&b, 1), [[5.0, -26.5], [21.0, -26.5]]));
}

#[test]
fn enter_with_no_displacement_leaves() {
    let mut b = bench(&[1]);
    b.start("move");
    b.confirm();
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
    assert!(near(line(&b, 1), [[-24.0, -12.0], [-8.0, -12.0]]), "nothing moved");
}

// ── Kopyala: Kip and Dizi ───────────────────────────────────────────────────

#[test]
fn single_mode_copies_once_and_stays_for_the_next_run() {
    let mut b = bench(&[5]);
    b.start("copy");
    assert_eq!(
        b.session.prompt().text(),
        "Kopyala: 1 nesne için temel noktayı belirtin [Yer değiştirme (Y) / Kip (P): çoklu]"
    );
    assert!(b.type_text("P"));
    assert!(b.session.prompt().text().ends_with("Kip (P): tek]"));
    let count = b.doc.len();
    b.click(20.0, -8.0);
    b.click(20.0, -4.0);
    assert_eq!(b.doc.len(), count + 1);
    assert_eq!(b.session.tool_id(), "select", "one copy, then the tool leaves");
    b.start("copy");
    assert!(b.session.prompt().text().ends_with("Kip (P): tek]"), "the session keeps it");
}

#[test]
fn an_array_copies_along_the_displacement_or_fits_it() {
    let mut b = bench(&[5]);
    let before: Vec<[f64; 2]> = points(&b);
    let p0 = before[0];
    b.start("copy");
    b.click(p0[0], p0[1]);
    assert!(b.type_text("A"));
    assert_eq!(b.session.prompt().text(), "Kopyala: dizideki öğe sayısını yazın (asıllar dahil)");
    assert!(b.type_text("1"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(b.type_text("4"));
    assert_eq!(
        b.session.prompt().text(),
        "Kopyala: dizinin ikinci noktasını belirtin (4 yer) [Sığdır (S): kapalı]"
    );
    b.click(p0[0] + 3.0, p0[1]);
    let got = points(&b);
    assert_eq!(got.len(), before.len() + 3, "three copies: {got:?}");
    for k in 1..=3 {
        let want = [p0[0] + 3.0 * f64::from(k), p0[1]];
        assert!(got.iter().any(|q| (q[0] - want[0]).abs() < 1e-9 && (q[1] - want[1]).abs() < 1e-9), "{want:?}");
    }
    assert_eq!(b.session.tool_id(), "copy", "back to the copies");
    assert_eq!(b.doc.undo().as_deref(), Some("Kopyala"), "one step");
    assert_eq!(points(&b).len(), before.len());

    // Sığdır: five places sharing the displacement, the last on the point.
    assert!(b.type_text("A"));
    assert!(b.type_text("5"));
    assert!(b.type_text("S"));
    b.click(p0[0], p0[1] + 8.0);
    let got = points(&b);
    assert_eq!(got.len(), before.len() + 4);
    for k in 1..=4 {
        let want = [p0[0], p0[1] + 2.0 * f64::from(k)];
        assert!(got.iter().any(|q| (q[0] - want[0]).abs() < 1e-9 && (q[1] - want[1]).abs() < 1e-9), "{want:?}");
    }
}

// ── Döndür ──────────────────────────────────────────────────────────────────

#[test]
fn a_cad_project_turns_by_degrees_counter_clockwise() {
    let mut b = cad(&[1]);
    b.start("rotate");
    b.click(-24.0, -12.0);
    assert_eq!(
        b.session.prompt().text(),
        "Döndür: açıyı yazın (derece, saat yönü tersine) ya da bir nokta gösterin [Referans (R) / Kopya (K): kapalı]"
    );
    let before = b.log.len();
    assert!(b.type_text("90"));
    assert_eq!(b.said(before), [(Level::Success, "1 nesne 90.0000° döndürüldü.")]);
    assert!(near(line(&b, 1), [[-24.0, -12.0], [-24.0, 4.0]]));
}

#[test]
fn a_cbs_project_turns_by_grads_clockwise_and_reads_semts() {
    let mut b = bench(&[1]);
    b.start("rotate");
    b.click(-24.0, -12.0);
    assert!(b.type_text("100"));
    assert!(near(line(&b, 1), [[-24.0, -12.0], [-24.0, -28.0]]), "a quarter clockwise");
    // Referans typed: the line runs east (semt 100g); the new direction north (0).
    b.start("rotate");
    b.click(-24.0, -12.0);
    assert!(b.type_text("R"));
    assert!(b.type_text("200"), "south: the line's direction now");
    assert!(b.type_text("100"), "east");
    assert!(near(line(&b, 1), [[-24.0, -12.0], [-8.0, -12.0]]), "back east");
}

#[test]
fn reference_takes_the_new_direction_from_two_points() {
    let mut b = cad(&[1]);
    b.start("rotate");
    b.click(-24.0, -12.0);
    assert!(b.type_text("R"));
    b.click(-24.0, -12.0);
    b.click(-8.0, -12.0);
    assert!(b.type_text("N"));
    assert_eq!(b.session.prompt().text(), "Döndür: yeni doğrultunun ilk noktasını gösterin");
    b.click(0.0, 0.0);
    assert_eq!(b.session.prompt().text(), "Döndür: yeni doğrultunun ikinci noktasını gösterin");
    b.click(3.0, 3.0);
    let [a, e] = line(&b, 1);
    assert!((e[0] - a[0] - e[1] + a[1]).abs() < 1e-9, "along north-east: {a:?} {e:?}");
    assert!((e[0] - a[0] - 16.0 / 2f64.sqrt()).abs() < 1e-9);
}

// ── Ölçekle ─────────────────────────────────────────────────────────────────

#[test]
fn reference_takes_the_new_length_from_two_points() {
    let mut b = bench(&[1]);
    b.start("scale");
    b.click(-24.0, -12.0);
    assert!(b.type_text("R"));
    assert!(b.type_text("16"), "the reference length typed");
    assert!(b.type_text("N"));
    assert_eq!(b.session.prompt().text(), "Ölçekle: yeni uzunluğun ilk ucunu gösterin");
    b.click(0.0, 0.0);
    b.click(3.0, 4.0);
    assert!(near(line(&b, 1), [[-24.0, -12.0], [-19.0, -12.0]]), "16 m became 5 m");
    assert_eq!(b.last_text(), Some("1 nesne 0.3125 faktörüyle ölçeklendi."));
}
