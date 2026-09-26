//! The edge, corner and object modify tools through the session, over the
//! native document: the behaviour of the web's `EdgePickTool` family and of
//! `JoinTool` and `ExplodeTool` that the interaction traces fix, one rule per
//! test (docs/adr/0047). The drawing is the traces' `edits.kcad`: the lines
//! 1 and 2 crossing at (−20, 12), the boundary 3 at x = −4, the L-shaped
//! polyline 4 with its corner at (0, 4), the lines 5 and 6 meeting at
//! (14, 4), the line 7 to break, the lines 8 and 9 meeting at (8, −4), the
//! closed area 10, the 10 m line 11, the line 12 and the line 13 on a locked
//! layer. Expected values are worked out by hand.

mod common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::{Level, MarkerShape, Tone};

const EDITS: &str = include_str!("../../../../fixtures/interaction/v1/edits.kcad");

/// The drawing with nothing running and snapping off; `selected` selected.
fn bench(selected: &[u32]) -> Bench {
    let mut b = Bench::on(EDITS);
    b.draft.snap = false;
    b.selection.set(selected.iter().map(|s| Slot(*s)));
    b
}

fn line(b: &Bench, slot: u32) -> [[f64; 2]; 2] {
    let Some(Entity::Line(l)) = b.doc.get(Slot(slot)) else {
        panic!("line {slot}: {:?}", b.doc.get(Slot(slot)));
    };
    [rel(l.a), rel(l.b)]
}

fn path(b: &Bench, slot: u32) -> Vec<[f64; 2]> {
    let Some(Entity::Polyline(p) | Entity::Polygon(p)) = b.doc.get(Slot(slot)) else {
        panic!("path {slot}: {:?}", b.doc.get(Slot(slot)));
    };
    p.pts.iter().map(|q| rel(*q)).collect()
}

fn near(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
    a.len() == b.len()
        && a.iter()
            .flatten()
            .zip(b.iter().flatten())
            .all(|(x, y)| (x - y).abs() < 1e-9)
}

// ── Ötele ───────────────────────────────────────────────────────────────────

#[test]
fn offset_copies_at_the_distance_or_through_a_point() {
    let mut b = bench(&[]);
    b.start("offset");
    assert_eq!(
        b.session.prompt().text(),
        "Ötele: ötelenecek nesneye tıklayın [mesafe 1.000 m; mesafe için sayı yazın; Noktadan geç (N): kapalı]"
    );
    assert_eq!(b.options(), ["N"]);
    // A typed number is the distance.
    assert!(b.type_text("2"));
    assert_eq!(b.memory.offset_distance, 2.0);
    // Hovering names the object by its edge; a click picks it.
    b.move_to(-20.0, 10.0);
    assert_eq!(b.selection.hover(), Some(Slot(2)));
    b.click(-20.0, 10.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ötele: kopyanın gideceği tarafa tıklayın [Noktadan geç (N): kapalı]"
    );
    b.move_to(-23.0, 10.0);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert_eq!(preview.tag.expect("a tag").lines, ["Mesafe 2.000 m"]);
    assert!(preview.strokes.iter().any(|s| s.dash == Some([4.0, 3.0])));
    b.click(-23.0, 10.0);
    assert_eq!(b.last_text(), Some("2.000 m ötelenmiş kopya eklendi."));
    assert!(near(&line(&b, 14), &[[-22.0, 6.0], [-22.0, 18.0]]));
    assert_eq!(b.doc.len(), 14);
    // Noktadan geç: the copy passes through the point, its distance remembered.
    assert!(b.type_text("n"));
    assert!(
        b.session
            .prompt()
            .text()
            .ends_with("[Noktadan geç (N): açık]")
    );
    b.click(-20.0, 10.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ötele: kopyanın geçeceği noktaya tıklayın [Noktadan geç (N): açık]"
    );
    b.click(-17.0, 9.0);
    assert!(near(&line(&b, 15), &[[-17.0, 6.0], [-17.0, 18.0]]));
    assert_eq!(b.memory.offset_distance, 3.0);
    // Esc drops the picked object and the tool stays; with none, Esc leaves.
    b.click(-20.0, 10.0);
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.session.tool_id(), "offset");
    assert!(!b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.session.tool_id(), "select");
    // One undo step each, the tool's name.
    assert_eq!(b.doc.undo().as_deref(), Some("Ötele"));
    assert_eq!(b.doc.undo().as_deref(), Some("Ötele"));
    assert_eq!(b.doc.len(), 13);
}

// ── Buda, Uzat ──────────────────────────────────────────────────────────────

#[test]
fn trim_cuts_between_the_visible_edges_and_shift_extends() {
    let mut b = bench(&[]);
    b.start("trim");
    assert_eq!(
        b.session.prompt().text(),
        "Buda: silinecek parçaya tıklayın [sınır: görünen tüm kenarlar; Sınır seç (S); Shift+tık: uzat]"
    );
    assert_eq!(b.options(), ["S"]);
    // The preview: the whole line dashed red, the part that stays solid over it.
    b.move_to(-26.0, 12.0);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert!(preview.strokes.iter().any(|s| s.tone == Tone::Danger));
    b.click(-26.0, 12.0);
    assert_eq!(b.last_text(), Some("Budandı: 1 parça kaldı."));
    assert!(near(&line(&b, 1), &[[-20.0, 12.0], [-8.0, 12.0]]));
    // Shift: the other one, extend to the next edge.
    b.shift = true;
    b.click(-9.0, 12.0);
    b.shift = false;
    assert_eq!(b.last_text(), Some("Uzatıldı."));
    assert!(near(&line(&b, 1), &[[-20.0, 12.0], [-4.0, 12.0]]));
    assert_eq!(b.doc.undo().as_deref(), Some("Uzat"));
    assert_eq!(b.doc.undo().as_deref(), Some("Buda"));
    assert!(near(&line(&b, 1), &[[-28.0, 12.0], [-8.0, 12.0]]));
    // A locked object is not an edge to edit.
    let before = b.doc.revision();
    b.click(8.0, -15.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some("Buda için düzenlenebilir bir kenara tıklayın.")
    );
    assert_eq!(b.doc.revision(), before);
}

#[test]
fn extend_grows_to_the_chosen_boundaries() {
    let mut b = bench(&[]);
    b.start("extend");
    // Sınır seç: clicks pick the boundaries, a confirm takes them.
    assert!(b.type_text("S"));
    assert_eq!(
        b.session.prompt().text(),
        "Uzat: sınır olacak nesnelere tıklayın, bitince sağ tıklayın (0 seçili) [Tüm kenarlar (T)]"
    );
    b.click(-4.0, 10.0);
    assert_eq!(b.selected(), [3]);
    b.confirm();
    assert_eq!(b.session.tool_id(), "extend");
    assert_eq!(
        b.session.prompt().text(),
        "Uzat: uzatılacak ucun yakınına tıklayın [sınır: seçilen 1 nesne; Tüm kenarlar (T) / Sınır seç (S); Shift+tık: buda]"
    );
    assert_eq!(b.options(), ["T", "S"]);
    b.click(-9.0, 12.0);
    assert!(near(&line(&b, 1), &[[-28.0, 12.0], [-4.0, 12.0]]));
    // Tüm kenarlar: every visible edge again; the selection goes.
    assert!(b.type_text("T"));
    assert!(b.selected().is_empty());
    assert!(b.session.prompt().text().contains("görünen tüm kenarlar"));
    // Esc while picking goes back to the boundaries taken before.
    assert!(b.type_text("S"));
    b.click(-20.0, 10.0);
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert!(b.selected().is_empty());
    assert_eq!(b.session.tool_id(), "extend");
}

// ── Köşe yuvarla, Pah ───────────────────────────────────────────────────────

#[test]
fn a_fillet_rounds_a_polyline_corner_by_a_typed_radius() {
    let mut b = bench(&[]);
    b.start("fillet");
    assert_eq!(
        b.session.prompt().text(),
        "Köşe yuvarla: yuvarlanacak köşeye tıklayın ya da sırayla iki çizgi seçin [Kırp (K): evet]"
    );
    // The corner under the cursor is ringed.
    b.move_to(0.2, 4.2);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert!(matches!(
        preview.markers.first().map(|m| m.shape),
        Some(MarkerShape::Ring(r)) if r == 7.0
    ));
    b.click(0.2, 4.2);
    assert_eq!(
        b.session.prompt().text(),
        "Köşe yuvarla: fareyi kenar boyunca kaydırıp tıklayın ya da yarıçap yazın [Kırp (K): evet]"
    );
    assert!(b.type_text("2"));
    assert_eq!(b.last_text(), Some("Köşe 2.000 m yarıçapla yuvarlandı."));
    assert!(near(
        &path(&b, 4),
        &[[0.0, 14.0], [0.0, 6.0], [2.0, 4.0], [10.0, 4.0]]
    ));
    let Some(Entity::Polyline(p)) = b.doc.get(Slot(4)) else {
        panic!("the polyline");
    };
    let arcs: Vec<f64> = p
        .bulges
        .iter()
        .flatten()
        .copied()
        .filter(|b| *b != 0.0)
        .collect();
    assert_eq!(arcs.len(), 1);
    assert!((arcs[0] - (std::f64::consts::PI / 8.0).tan()).abs() < 1e-12);
    assert_eq!(
        p.base.attrs.get("Ad").map(String::as_str),
        Some("Yol"),
        "the object keeps its data"
    );
    assert_eq!(b.memory.fillet_radius, Some(2.0));
    // The last radius is offered now.
    b.click(14.2, 4.2);
    assert_eq!(b.options(), ["Enter", "K"]);
    b.confirm();
    assert_eq!(b.last_text(), Some("Köşe 2.000 m yarıçapla yuvarlandı."));
    assert_eq!(b.doc.len(), 14, "the arc is a new object");
    assert_eq!(b.doc.undo().as_deref(), Some("Köşe yuvarla"));
    assert_eq!(b.doc.undo().as_deref(), Some("Köşe yuvarla"));
}

#[test]
fn a_chamfer_cuts_where_two_lines_meet() {
    let mut b = bench(&[]);
    b.start("chamfer");
    b.click(14.2, 4.2);
    assert_eq!(
        b.session.prompt().text(),
        "Pah: fareyi kenar boyunca kaydırıp tıklayın ya da mesafe yazın (d ya da d1,d2) [Kırp (K): evet]"
    );
    assert!(b.type_text("2"));
    assert_eq!(b.last_text(), Some("Pah kırıldı: 2.000 m."));
    // Each line keeps the side away from the corner, cut back 2 m; the cut is new.
    assert!(near(&line(&b, 5), &[[26.0, 4.0], [16.0, 4.0]]));
    assert!(near(&line(&b, 6), &[[14.0, 16.0], [14.0, 6.0]]));
    assert!(near(&line(&b, 14), &[[16.0, 4.0], [14.0, 6.0]]));
    assert_eq!(b.memory.chamfer, Some((2.0, 2.0)));
    assert_eq!(b.doc.undo().as_deref(), Some("Pah"));
    // Kırp: hayır adds only the cut; two distances, d1,d2.
    assert!(b.type_text("K"));
    assert!(!b.memory.corner_trim);
    b.click(14.2, 4.2);
    assert!(b.type_text("1,2"));
    assert_eq!(
        b.last_text(),
        Some("Pah kırıldı: 1.000 ile 2.000 m. Kenarlar kırpılmadı.")
    );
    assert!(near(&line(&b, 5), &[[14.0, 4.0], [26.0, 4.0]]));
    // Slots are not given twice: the undone cut had 14.
    assert!(near(&line(&b, 15), &[[15.0, 4.0], [14.0, 6.0]]));
    assert!(b.type_text("K"));
    // Two lines picked one after the other: a line and a polyline do not make a corner.
    b.click(20.0, 4.0);
    assert_eq!(
        b.session.prompt().text(),
        "Pah: ikinci çizgiyi ya da aynı çoklu çizginin komşu kenarını seçin"
    );
    b.click(5.0, 4.0);
    assert_eq!(
        b.last_text(),
        Some(
            "Çoklu çizgide köşe için köşenin kendisine ya da aynı nesnenin iki komşu kenarına tıklayın."
        )
    );
    // Esc steps back to finding a corner.
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Pah: kesilecek köşeye tıklayın")
    );
}

// ── Kır ─────────────────────────────────────────────────────────────────────

#[test]
fn break_removes_between_two_points_or_splits_at_one() {
    let mut b = bench(&[]);
    b.start("break");
    assert_eq!(
        b.session.prompt().text(),
        "Kır: nesneyi ilk kırılma noktasından seçin (kesişim ve uç kenetleri çalışır)"
    );
    b.click(-24.0, -4.0);
    assert_eq!(
        b.session.prompt().text(),
        "Kır: ikinci kırılma noktasını belirtin [Aynı noktadan böl (Enter)]"
    );
    b.move_to(-16.0, -4.0);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert!(preview.strokes.iter().any(|s| s.tone == Tone::Danger));
    b.click(-16.0, -4.0);
    assert_eq!(b.last_text(), Some("Aradaki kısım silindi."));
    assert!(near(&line(&b, 7), &[[-28.0, -4.0], [-24.0, -4.0]]));
    assert!(near(&line(&b, 14), &[[-16.0, -4.0], [-8.0, -4.0]]));
    // Enter: split at the first point.
    b.click(-12.0, -4.0);
    b.confirm();
    assert_eq!(b.last_text(), Some("Nesne bölündü: 2 parça."));
    assert!(near(&line(&b, 14), &[[-16.0, -4.0], [-12.0, -4.0]]));
    assert!(near(&line(&b, 15), &[[-12.0, -4.0], [-8.0, -4.0]]));
    assert_eq!(b.session.tool_id(), "break");
    assert_eq!(b.doc.undo().as_deref(), Some("Kır"));
    assert_eq!(b.doc.undo().as_deref(), Some("Kır"));
    assert!(near(&line(&b, 7), &[[-28.0, -4.0], [-8.0, -4.0]]));
}

// ── Birleştir, Patlat ───────────────────────────────────────────────────────

#[test]
fn join_with_a_selection_acts_at_once_and_leaves_the_locked_out() {
    let mut b = bench(&[8, 9, 13]);
    let before = b.log.len();
    b.start("join");
    assert_eq!(b.session.tool_id(), "select", "it acted and left");
    assert_eq!(
        b.said(before),
        [
            (Level::Warn, "1 nesne kilitli katmanda olduğu için atlandı."),
            (Level::Success, "2 nesne birleştirildi: çoklu çizgi.")
        ]
    );
    assert!(near(
        &path(&b, 8),
        &[[0.0, -4.0], [8.0, -4.0], [8.0, -12.0]]
    ));
    assert!(b.doc.get(Slot(9)).is_none());
    assert_eq!(b.selected(), [8]);
    assert_eq!(b.doc.undo().as_deref(), Some("Birleştir"));
    assert!(near(&line(&b, 9), &[[8.0, -4.0], [8.0, -12.0]]));
}

#[test]
fn join_without_a_selection_picks_first_and_takes_a_typed_tolerance() {
    let mut b = bench(&[]);
    b.start("join");
    assert_eq!(
        b.session.prompt().text(),
        "Birleştir: nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın (0 seçili) [uç boşluğu toleransı 0.001 m; değiştirmek için sayı yazın]"
    );
    assert!(b.type_text("0.5"));
    assert_eq!(b.memory.join_tolerance, 0.5);
    assert!(b.session.prompt().text().contains("toleransı 0.500 m"));
    b.click(4.0, -4.0);
    b.click(8.0, -8.0);
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.last_text(), Some("2 nesne birleştirildi: çoklu çizgi."));
}

#[test]
fn explode_takes_a_closed_area_apart_into_lines() {
    let mut b = bench(&[10]);
    b.start("explode");
    assert_eq!(b.last_text(), Some("1 nesne patlatıldı: 4 parça."));
    assert!(b.doc.get(Slot(10)).is_none());
    assert_eq!(b.selected(), [14, 15, 16, 17]);
    let Some(Entity::Line(l)) = b.doc.get(Slot(14)) else {
        panic!("a line");
    };
    assert_eq!(l.base.layer_id, "parsel", "the pieces keep the layer");
    assert!(l.base.attrs.is_empty(), "and not the parcel's data");
    assert_eq!(b.doc.undo().as_deref(), Some("Patlat"));
    assert_eq!(b.doc.len(), 13);
}

// ── Uzat-kısalt ─────────────────────────────────────────────────────────────

#[test]
fn lengthen_follows_the_mouse_or_takes_a_typed_total() {
    let mut b = bench(&[]);
    b.start("lengthen");
    assert_eq!(
        b.session.prompt().text(),
        "Uzat-kısalt: değiştirilecek ucun yakınına tıklayın [kip: dinamik; Dinamik (D) / Fark (F): 1.000 m / Yüzde (Y): 100 / Toplam (T): 10.000 m]"
    );
    assert_eq!(b.options(), ["D", "F", "Y", "T"]);
    // Dinamik: the end follows the mouse, a typed number is the total length.
    b.click(-19.0, -12.0);
    b.move_to(-15.0, -12.0);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert_eq!(preview.tag.expect("a tag").lines, ["10.000 m → 13.000 m"]);
    assert!(b.type_text("12"));
    assert_eq!(b.last_text(), Some("Uzunluk 10.000 m → 12.000 m."));
    assert!(near(&line(&b, 11), &[[-28.0, -12.0], [-16.0, -12.0]]));
    // Toplam: the value asked for, then every end clicked takes it.
    assert!(b.type_text("T"));
    assert_eq!(
        b.session.prompt().text(),
        "Uzat-kısalt: yeni toplam uzunluğu yazın"
    );
    assert!(b.type_text("15"));
    assert!(b.session.prompt().text().contains("kip: toplam"));
    b.click(-27.0, -12.0);
    assert!(near(&line(&b, 11), &[[-31.0, -12.0], [-16.0, -12.0]]));
    // A value asked for must be above zero, except a difference.
    assert!(b.type_text("Y"));
    assert!(b.type_text("0"));
    assert_eq!(b.last_text(), Some("Değer sıfırdan büyük olmalı."));
    assert_eq!(b.doc.undo().as_deref(), Some("Uzat-kısalt"));
}

// ── Köşe ekle/sil ───────────────────────────────────────────────────────────

#[test]
fn a_vertex_is_added_on_an_edge_and_removed_at_a_vertex() {
    let mut b = bench(&[]);
    b.start("vertex");
    b.move_to(-18.0, -18.0);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert!(matches!(preview.markers[0].shape, MarkerShape::Plus(_)));
    b.click(-18.0, -18.0);
    assert_eq!(b.last_text(), Some("Köşe eklendi."));
    assert!(near(
        &path(&b, 12),
        &[[-28.0, -18.0], [-18.0, -18.0], [-8.0, -18.0]]
    ));
    b.move_to(-18.0, -18.0);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert!(matches!(preview.markers[0].shape, MarkerShape::Cross(_)));
    assert_eq!(preview.tag_tone, Tone::Danger);
    b.click(-18.0, -18.0);
    assert_eq!(b.last_text(), Some("Köşe silindi."));
    // The polyline stays one, of two points (the core's `remove_vertex`).
    assert!(near(&path(&b, 12), &[[-28.0, -18.0], [-8.0, -18.0]]));
    assert_eq!(b.doc.undo().as_deref(), Some("Köşe sil"));
    assert_eq!(b.doc.undo().as_deref(), Some("Köşe ekle"));
    // Enter: the tool stays (the web starts it again).
    b.confirm();
    assert_eq!(b.session.tool_id(), "vertex");
}
