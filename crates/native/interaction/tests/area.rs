//! Alan işlemleri through the session, over the native document: the web's
//! words, what each tool writes and the one undo step it names (the web's
//! area tools, docs/adr/0065). The drawing (fixtures/interaction/v1/areas.kcad):
//! parcels 1 and 2 of 10 × 10 m overlapping 4 × 6 m; a circle of radius 3;
//! four lines closing a 10 × 10 m square; parcel 8 of 10 × 10 m with a
//! 4 × 4 m hole; a square on the locked layer; a line across parcel 1 at
//! y = 2. Expected values are worked out by hand.

mod common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::{Level, measures};

const AREAS: &str = include_str!("../../../../fixtures/interaction/v1/areas.kcad");

fn bench() -> Bench {
    let mut b = Bench::on(AREAS);
    b.draft.snap = false;
    b
}

fn select(b: &mut Bench, slots: &[u32]) {
    b.selection.set(slots.iter().map(|s| Slot(*s)).collect::<Vec<_>>());
}

fn area(b: &Bench, slot: u32) -> f64 {
    let e = b.doc.get(Slot(slot)).expect("the object");
    measures(e).0.expect("an area")
}

fn near(have: f64, want: f64) -> bool {
    (have - want).abs() < 1e-6
}

fn undo(b: &mut Bench) -> Option<String> {
    b.doc.undo()
}

#[test]
fn union_makes_one_area_with_the_first_ones_data() {
    let mut b = bench();
    select(&mut b, &[1, 2]);
    let count = b.doc.entities().count();
    b.start("areaUnion");
    assert!(!b.session.is_running(), "it acts and leaves");
    assert_eq!(b.doc.entities().count(), count - 1);
    assert!(b.doc.get(Slot(1)).is_none() && b.doc.get(Slot(2)).is_none());
    let made = b.selected();
    assert_eq!(made.len(), 1);
    // 100 + 100 − 24.
    assert!(near(area(&b, made[0]), 176.0));
    let e = b.doc.get(Slot(made[0])).expect("the union");
    assert_eq!(e.base().attrs.get("Parsel").map(String::as_str), Some("1"));
    assert_eq!(
        (e.base().label.as_deref(), e.base().layer_id.as_str()),
        (Some("1"), "parsel")
    );
    assert_eq!(
        b.last_text(),
        Some("2 alan birleştirildi: tek alan, toplam 176.00 m².")
    );
    assert_eq!(undo(&mut b).as_deref(), Some("Alan birleştir"));
    assert_eq!(b.doc.entities().count(), count);
}

#[test]
fn intersect_adds_the_common_part_and_can_erase_the_sources() {
    let mut b = bench();
    select(&mut b, &[1, 2]);
    b.start("areaIntersect");
    let made = b.selected();
    assert!(near(area(&b, made[0]), 24.0));
    assert!(b.doc.get(Slot(1)).is_some(), "the sources stay");
    let e = b.doc.get(Slot(made[0])).expect("the part");
    assert!(e.base().attrs.is_empty() && e.base().label.is_none());
    assert_eq!(b.last_text(), Some("Ortak alan: 24.00 m²."));
    assert_eq!(undo(&mut b).as_deref(), Some("Alan kesiştir"));
    // Kaynakları sil while picking: the part keeps the first one's data.
    select(&mut b, &[]);
    b.start("areaIntersect");
    assert_eq!(
        b.session.prompt().text(),
        "Alan kesiştir: nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın (0 seçili) [Kaynakları sil (S): hayır]"
    );
    assert!(b.type_text("s"));
    assert!(b.session.prompt().text().ends_with("(S): evet]"));
    select(&mut b, &[1, 2]);
    b.confirm();
    assert!(b.doc.get(Slot(1)).is_none() && b.doc.get(Slot(2)).is_none());
    let made = b.selected();
    let e = b.doc.get(Slot(made[0])).expect("the part");
    assert_eq!(e.base().label.as_deref(), Some("1"));
    assert_eq!(b.last_text(), Some("Ortak alan: 24.00 m²; kaynaklar silindi."));
}

#[test]
fn subtract_takes_the_second_selection_from_the_first() {
    let mut b = bench();
    select(&mut b, &[1]);
    b.start("areaSubtract");
    assert!(b.session.is_running());
    assert_eq!(
        b.session.prompt().text(),
        "Alan çıkar: çıkarılacak alanları seçin, bitince sağ tıklayın (0 seçili) [Çıkarılanları sil (S): hayır]"
    );
    select(&mut b, &[2]);
    b.confirm();
    assert!(!b.session.is_running());
    assert!(b.doc.get(Slot(1)).is_none(), "cut: a new area");
    assert!(b.doc.get(Slot(2)).is_some(), "the cutter stays");
    let made = b.selected();
    assert!(near(area(&b, made[0]), 76.0));
    assert_eq!(b.last_text(), Some("1 alandan çıkarıldı; kalan 76.00 m²."));
    assert_eq!(undo(&mut b).as_deref(), Some("Alan çıkar"));
}

#[test]
fn subtract_without_overlap_changes_nothing() {
    let mut b = bench();
    select(&mut b, &[1]);
    b.start("areaSubtract");
    select(&mut b, &[8]);
    let count = b.doc.entities().count();
    b.confirm();
    assert_eq!(b.doc.entities().count(), count);
    assert_eq!(
        b.last_text(),
        Some("Çıkarılan alanlar kesilecek alanlarla örtüşmüyor; hiçbir alan değişmedi.")
    );
}

#[test]
fn split_by_clicked_points_keeps_the_area_as_its_first_piece() {
    let mut b = bench();
    select(&mut b, &[1]);
    b.start("areaSplit");
    assert_eq!(
        b.session.prompt().text(),
        "Alan böl: kesme çizgisinin ilk noktasını gösterin [Çizgiyle kes (N)]"
    );
    b.click(5.0, -3.0);
    assert_eq!(
        b.session.prompt().text(),
        "Alan böl: sonraki noktayı gösterin [Geri (G) / Çizgiyle kes (N)]"
    );
    // One point is not a line.
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Kesme çizgisi için en az iki nokta gösterin.")
    );
    b.click(5.0, 13.0);
    b.confirm();
    assert!(!b.session.is_running());
    let made = b.selected();
    assert_eq!(made.len(), 2);
    assert_eq!(made[0], 1, "the first piece keeps the slot");
    assert!(near(area(&b, 1), 50.0) && near(area(&b, made[1]), 50.0));
    let other = b.doc.get(Slot(made[1])).expect("the second piece");
    assert_eq!(other.base().attrs.get("Parsel").map(String::as_str), Some("1"));
    assert_eq!(
        b.last_text(),
        Some("1 alan 2 parçaya bölündü: 50.00 m², 50.00 m². Parçalar özgün alanın özniteliklerini taşır; parsel numaralarını güncelleyin.")
    );
    assert_eq!(undo(&mut b).as_deref(), Some("Alan böl"));
}

#[test]
fn split_by_an_existing_line() {
    let mut b = bench();
    select(&mut b, &[1]);
    b.start("areaSplit");
    assert!(b.type_text("n"));
    assert_eq!(
        b.session.prompt().text(),
        "Alan böl: kesici çizgiye tıklayın: çizgi, çoklu çizgi, yay ya da daire [Noktalarla kes (N)]"
    );
    // Nothing there.
    b.click(20.0, 18.0);
    assert_eq!(
        b.last_text(),
        Some("Kesici olarak bir çizgiye, çoklu çizgiye, yaya ya da daireye tıklayın.")
    );
    // The line at y = 2: 20 and 80 m².
    b.click(-1.0, 2.0);
    assert!(!b.session.is_running());
    let made = b.selected();
    let mut sizes: Vec<f64> = made.iter().map(|&s| area(&b, s)).collect();
    sizes.sort_by(f64::total_cmp);
    assert!(near(sizes[0], 20.0) && near(sizes[1], 80.0), "{sizes:?}");
    assert!(b.doc.get(Slot(10)).is_some(), "the cutter stays");
}

#[test]
fn to_area_turns_closed_objects_and_closed_line_work_into_areas() {
    let mut b = bench();
    select(&mut b, &[3, 4, 5, 6, 7]);
    b.start("toArea");
    // The circle is an area now, in its own slot.
    assert!(matches!(b.doc.get(Slot(3)), Some(Entity::Polygon(_))));
    assert!(near(area(&b, 3), std::f64::consts::PI * 9.0));
    // The lines stay; their square is a new area on their layer.
    assert!(b.doc.get(Slot(4)).is_some());
    let made = b.selected();
    assert_eq!(made.len(), 2);
    assert!(near(area(&b, made[1]), 100.0));
    assert_eq!(
        b.last_text(),
        Some("1 nesne alana çevrildi; çizgilerden 1 alan oluştu (100.00 m²).")
    );
    assert_eq!(undo(&mut b).as_deref(), Some("Alana çevir"));
    // An area is an area already.
    select(&mut b, &[1]);
    b.start("toArea");
    assert_eq!(b.last_text(), Some("Seçili nesneler zaten alan."));
}

#[test]
fn to_polyline_gives_a_ring_each_the_outer_one_keeping_the_data() {
    let mut b = bench();
    select(&mut b, &[8]);
    b.start("toPolyline");
    let made = b.selected();
    assert_eq!(made.len(), 2);
    let Some(Entity::Polyline(outer)) = b.doc.get(Slot(8)) else {
        panic!("the outer ring, in its slot");
    };
    assert_eq!(outer.pts.len(), 5, "closed: the first point again");
    assert_eq!(rel(outer.pts[0]), [18.0, -15.0]);
    assert_eq!(outer.base.attrs.get("Parsel").map(String::as_str), Some("3"));
    assert_eq!(
        b.last_text(),
        Some("1 alan kapalı çoklu çizgiye çevrildi (2 çizgi).")
    );
    assert_eq!(undo(&mut b).as_deref(), Some("Çizgiye çevir"));
}

#[test]
fn objects_on_a_locked_layer_are_left_out() {
    let mut b = bench();
    select(&mut b, &[1, 9]);
    b.start("areaUnion");
    let said = b.said(0);
    assert!(said.contains(&(Level::Warn, "1 nesne kilitli katmanda olduğu için atlandı.")));
    assert_eq!(
        b.last_text(),
        Some("Birleştirmek için en az iki alan seçin (kapalı alan, daire, elips ya da kapalı eğri).")
    );
}

#[test]
fn a_click_inside_line_work_makes_an_area_on_the_active_layer() {
    let mut b = bench();
    b.start("boundary");
    assert_eq!(
        b.session.prompt().text(),
        "İçine tıklayarak alan: alanı oluşturulacak bölgenin içine tıklayın [Adalar (A): delik olur / Sınır katmanı (K): tümü]"
    );
    b.click(-23.0, -10.0);
    let made = b.selected();
    assert!(near(area(&b, made[0]), 100.0));
    assert_eq!(
        b.doc.get(Slot(made[0])).expect("the area").base().layer_id,
        "cizim"
    );
    assert_eq!(b.last_text(), Some("Alan oluşturuldu: 100.00 m²."));
    assert!(b.session.is_running(), "the tool waits for the next one");
    // Outside every region.
    b.click(-23.0, 18.0);
    assert_eq!(
        b.last_text(),
        Some("Tıklanan yer kapalı bir bölgenin içinde değil. Bölgeyi saran çizgiler birleşmeli ya da kesişmeli; görünüm dışındaki çizgiler sayılmaz.")
    );
    assert_eq!(undo(&mut b).as_deref(), Some("Alan oluştur"));
    // The boundary layer is picked by an object of it; K again, every layer.
    assert!(b.type_text("K"));
    assert_eq!(
        b.session.prompt().text(),
        "İçine tıklayarak alan: sınır olacak katmandan bir nesneye tıklayın [Tüm katmanlar (K)]"
    );
    b.click(1.0, 5.0);
    assert!(b.session.prompt().text().ends_with("Sınır katmanı (K): Parseller]"));
    assert!(b.type_text("a"));
    assert!(b.session.prompt().text().contains("Adalar (A): yok sayılır"));
}
