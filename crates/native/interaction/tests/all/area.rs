//! Alan işlemleri through the session, over the native document: the web's
//! words, what each tool writes and the one undo step it names (the web's
//! area tools, docs/adr/0065). The drawing (fixtures/interaction/v1/areas.kcad):
//! parcels 1 and 2 of 10 × 10 m overlapping 4 × 6 m; a circle of radius 3;
//! four lines closing a 10 × 10 m square; parcel 8 of 10 × 10 m with a
//! 4 × 4 m hole; a square on the locked layer; a line across parcel 1 at
//! y = 2. Expected values are worked out by hand.

use crate::common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::{Level, measures};

const AREAS: &str = include_str!("../../../../../fixtures/interaction/v1/areas.kcad");

fn bench() -> Bench {
    let mut b = Bench::on(AREAS);
    b.draft.snap = false;
    b
}

fn select(b: &mut Bench, slots: &[u32]) {
    b.selection
        .set(slots.iter().map(|s| Slot(*s)).collect::<Vec<_>>());
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
        "Alan kesiştir: nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın (0 seçili) [Kaynakları sil (S): hayır / Tek nesne (T): hayır]"
    );
    assert!(b.type_text("s"));
    assert!(b.session.prompt().text().contains("(S): evet"));
    select(&mut b, &[1, 2]);
    b.confirm();
    assert!(b.doc.get(Slot(1)).is_none() && b.doc.get(Slot(2)).is_none());
    let made = b.selected();
    let e = b.doc.get(Slot(made[0])).expect("the part");
    assert_eq!(e.base().label.as_deref(), Some("1"));
    assert_eq!(
        b.last_text(),
        Some("Ortak alan: 24.00 m²; kaynaklar silindi.")
    );
}

#[test]
fn subtract_takes_the_second_selection_from_the_first() {
    let mut b = bench();
    select(&mut b, &[1]);
    b.start("areaSubtract");
    assert!(b.session.is_running());
    assert_eq!(
        b.session.prompt().text(),
        "Alan çıkar: çıkarılacak alanları seçin, bitince sağ tıklayın (0 seçili) [Çıkarılanları sil (S): hayır / Tek nesne (T): hayır]"
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
    assert_eq!(
        other.base().attrs.get("Parsel").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        b.last_text(),
        Some(
            "1 alan 2 parçaya bölündü: 50.00 m², 50.00 m². Parçalar özgün alanın özniteliklerini taşır; parsel numaralarını güncelleyin."
        )
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
    assert_eq!(
        outer.base.attrs.get("Parsel").map(String::as_str),
        Some("3")
    );
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
        Some(
            "Birleştirmek için en az iki alan seçin (kapalı alan, daire, elips ya da kapalı eğri)."
        )
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
        Some(
            "Tıklanan yer kapalı bir bölgenin içinde değil. Bölgeyi saran çizgiler birleşmeli ya da kesişmeli; görünüm dışındaki çizgiler sayılmaz."
        )
    );
    assert_eq!(undo(&mut b).as_deref(), Some("Alan oluştur"));
    // The boundary layer is picked by an object of it; K again, every layer.
    assert!(b.type_text("K"));
    assert_eq!(
        b.session.prompt().text(),
        "İçine tıklayarak alan: sınır olacak katmandan bir nesneye tıklayın [Tüm katmanlar (K)]"
    );
    b.click(1.0, 5.0);
    assert!(
        b.session
            .prompt()
            .text()
            .ends_with("Sınır katmanı (K): Parseller]")
    );
    assert!(b.type_text("a"));
    assert!(
        b.session
            .prompt()
            .text()
            .contains("Adalar (A): yok sayılır")
    );
}

// ── Çok parçalı alan (docs/adr/0143) ────────────────────────────────────────

/// The parts of the area at `slot`: each part's area, the first its own.
fn parts(b: &Bench, slot: u32) -> Vec<f64> {
    let Some(Entity::Polygon(p)) = b.doc.get(Slot(slot)) else {
        panic!("an area at {slot}");
    };
    let one = |pts: &[kentos_contracts::Vec2],
               holes: &Option<Vec<kentos_contracts::RingGeometry>>| {
        let mut q = p.clone();
        (q.pts, q.holes, q.zs, q.parts) = (pts.to_vec(), holes.clone(), None, None);
        measures(&Entity::Polygon(q)).0.expect("an area")
    };
    std::iter::once(one(&p.pts, &p.holes))
        .chain(
            p.parts
                .iter()
                .flatten()
                .map(|part| one(&part.pts, &part.holes)),
        )
        .collect()
}

#[test]
fn union_with_one_object_makes_one_multi_part_area() {
    // Parcels 1 and 8 do not touch: two areas, or one of two parts with Tek nesne.
    let mut b = bench();
    select(&mut b, &[1, 8]);
    b.start("areaUnion");
    assert_eq!(b.selected().len(), 2);
    assert_eq!(
        b.last_text(),
        Some(
            "2 alan birleştirildi: 2 ayrı alan (birbirine değmeyenler ayrı kalır), toplam 184.00 m²."
        )
    );
    undo(&mut b);
    select(&mut b, &[]);
    b.start("areaUnion");
    assert_eq!(
        b.session.prompt().text(),
        "Alan birleştir: nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın (0 seçili) [Tek nesne (T): hayır]"
    );
    assert!(b.type_text("t"));
    assert!(b.memory.area_one_object);
    select(&mut b, &[1, 8]);
    b.confirm();
    let made = b.selected();
    assert_eq!(made.len(), 1);
    // The largest part first: parcel 1's 100 m², then parcel 8's 100 − 16.
    let sizes = parts(&b, made[0]);
    assert!(near(sizes[0], 100.0) && near(sizes[1], 84.0), "{sizes:?}");
    let e = b.doc.get(Slot(made[0])).expect("the union");
    assert_eq!(e.base().label.as_deref(), Some("1"));
    assert_eq!(
        b.last_text(),
        Some("2 alan birleştirildi: 2 parçalı tek alan, toplam 184.00 m².")
    );
    assert_eq!(undo(&mut b).as_deref(), Some("Alan birleştir"));
}

#[test]
fn a_multi_part_area_is_taken_whole() {
    // Parcels 1 and 8 as one area of two parts; parcel 2 overlaps only parcel 1.
    let mut b = bench();
    b.memory.area_one_object = true;
    select(&mut b, &[1, 8]);
    b.start("areaUnion");
    let whole = b.selected()[0];
    b.memory.area_one_object = false;
    select(&mut b, &[whole, 2]);
    b.start("areaIntersect");
    let made = b.selected();
    assert!(near(area(&b, made[0]), 24.0));
    // Parcel 2 cut from the two-part area: it stays whole with Tek nesne.
    undo(&mut b);
    b.memory.area_one_object = true;
    select(&mut b, &[whole]);
    b.start("areaSubtract");
    select(&mut b, &[2]);
    b.confirm();
    let made = b.selected();
    assert_eq!(made.len(), 1);
    let sizes = parts(&b, made[0]);
    assert!(near(sizes[0], 84.0) && near(sizes[1], 76.0), "{sizes:?}");
    assert_eq!(b.last_text(), Some("1 alandan çıkarıldı; kalan 160.00 m²."));
}

#[test]
fn parts_join_in_the_first_ones_place_and_split_back() {
    let mut b = bench();
    let count = b.doc.entities().count();
    select(&mut b, &[1, 8]);
    b.start("partsJoin");
    assert!(!b.session.is_running(), "it acts and leaves");
    assert_eq!(b.selected(), vec![1]);
    assert!(b.doc.get(Slot(8)).is_none());
    assert_eq!(b.doc.entities().count(), count - 1);
    let sizes = parts(&b, 1);
    assert!(near(sizes[0], 100.0) && near(sizes[1], 84.0), "{sizes:?}");
    assert_eq!(
        b.last_text(),
        Some("2 alan tek alanda birleşti: 2 parça, toplam 184.00 m².")
    );
    assert_eq!(undo(&mut b).as_deref(), Some("Parçaları birleştir"));
    // Overlapping parcels merge into one part.
    select(&mut b, &[1, 2]);
    b.start("partsJoin");
    assert_eq!(parts(&b, 1).len(), 1);
    assert_eq!(
        b.last_text(),
        Some("2 alan tek alanda birleşti: örtüşenler birleşti, 1 parça, toplam 176.00 m².")
    );
    undo(&mut b);
    // Joined, then split: the first part keeps slot 1, the other is new with its data.
    select(&mut b, &[1, 8]);
    b.start("partsJoin");
    select(&mut b, &[1]);
    b.start("partsSplit");
    let made = b.selected();
    assert_eq!((made.len(), made[0]), (2, 1));
    assert!(near(area(&b, 1), 100.0) && near(area(&b, made[1]), 84.0));
    let e = b.doc.get(Slot(made[1])).expect("the new area");
    assert_eq!(e.base().label.as_deref(), Some("1"));
    assert_eq!(b.last_text(), Some("1 alan parçalarına ayrıldı (2 alan)."));
    assert_eq!(undo(&mut b).as_deref(), Some("Parçalara ayır"));
    // An area of one part has nothing to split.
    select(&mut b, &[2]);
    b.start("partsSplit");
    assert_eq!(
        b.last_text(),
        Some(
            "Parçalarına ayrılacak çok parçalı bir nesne seçin: alan, çoklu çizgi ya da çok noktalı nesne."
        )
    );
    select(&mut b, &[2]);
    b.start("partsJoin");
    assert_eq!(
        b.last_text(),
        Some(
            "Parçaları birleştirmek için en az iki alan seçin (kapalı alan, daire, elips ya da kapalı eğri)."
        )
    );
}

/// Parçaları birleştir and Parçalara ayır on lines and points (docs/adr/0174
/// §4), as the web's: lines 4 and 10 one polyline in line 4's place, a
/// polyline's part of two points one again when split; points one object,
/// each with its elevation; kinds mixed refused.
#[test]
fn lines_and_points_join_and_come_apart() {
    let mut b = bench();
    let count = b.doc.entities().count();
    select(&mut b, &[4, 10]);
    b.start("partsJoin");
    assert!(!b.session.is_running(), "it acts and leaves");
    assert_eq!(b.selected(), vec![4]);
    assert!(b.doc.get(Slot(10)).is_none());
    assert_eq!(b.doc.entities().count(), count - 1);
    let Some(Entity::Polyline(joined)) = b.doc.get(Slot(4)) else {
        panic!("a polyline")
    };
    assert_eq!(joined.pts.len(), 2);
    assert_eq!(joined.parts.as_ref().map(Vec::len), Some(1));
    // 10 m and 14 m.
    assert_eq!(
        b.last_text(),
        Some("2 çizgi tek çoklu çizgide birleşti: 2 parça, toplam 24.000 m.")
    );
    select(&mut b, &[4]);
    b.start("partsSplit");
    let made = b.selected();
    assert_eq!((made.len(), made[0]), (2, 4));
    assert!(matches!(b.doc.get(Slot(made[1])), Some(Entity::Polyline(p)) if p.pts.len() == 2));
    assert_eq!(
        b.last_text(),
        Some("1 nesne parçalarına ayrıldı (2 nesne).")
    );
    assert_eq!(undo(&mut b).as_deref(), Some("Parçalara ayır"));
    assert_eq!(undo(&mut b).as_deref(), Some("Parçaları birleştir"));
    assert!(matches!(b.doc.get(Slot(4)), Some(Entity::Line(_))));
    // Points, each with its elevation.
    let p = b.add_point_z("cizim", [0.0, 50.0], 50.0);
    let q = b.add_point_z("cizim", [5.0, 50.0], 0.0);
    select(&mut b, &[p.0, q.0]);
    b.start("partsJoin");
    let Some(Entity::Point(joined)) = b.doc.get(p) else {
        panic!("a point")
    };
    assert_eq!(joined.z, Some(50.0));
    assert_eq!(
        joined
            .parts
            .as_ref()
            .map(|ps| ps.iter().map(|q| q.z).collect::<Vec<_>>()),
        Some(vec![Some(0.0)])
    );
    assert_eq!(
        b.last_text(),
        Some("2 nokta tek nesnede birleşti: 2 nokta.")
    );
    // Kinds mixed are refused; one of a kind is not enough.
    select(&mut b, &[1, 5]);
    b.start("partsJoin");
    assert_eq!(
        b.last_text(),
        Some(
            "Parçaları birleştir aynı türden nesneleri birleştirir: alanları, çizgileri ya da noktaları."
        )
    );
    select(&mut b, &[5]);
    b.start("partsJoin");
    assert_eq!(
        b.last_text(),
        Some("Parçaları birleştirmek için en az iki çizgi ya da çoklu çizgi seçin.")
    );
}
