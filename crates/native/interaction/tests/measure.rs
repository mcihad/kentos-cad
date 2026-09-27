//! Mesafe ölç, Alan hesapla and Parsel oluştur through the session, over the
//! native document: the web's `PathTool` with its `measureOnly` and
//! `parcelLayer` flags (docs/adr/0067). The drawing
//! (fixtures/interaction/v1/areas.kcad) holds parcels numbered 1, 2 and 3 on
//! Parseller (`parsel`) and nothing west of x = −17 m between y = 12 and 20.
//! Expected values are worked out by hand.

mod common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::Level;

const AREAS: &str = include_str!("../../../../fixtures/interaction/v1/areas.kcad");

fn bench(tool: &str) -> Bench {
    let mut b = Bench::on(AREAS);
    b.draft.snap = false;
    b.start(tool);
    b
}

/// The lines of the tag beside the cursor.
fn tag(b: &Bench) -> Vec<String> {
    let format = kentos_interaction::Format::default();
    let preview = b.session.preview(&format).expect("a tool runs");
    preview.tag.expect("a tag").lines
}

fn clicks(b: &mut Bench, pts: &[[f64; 2]]) {
    for &[de, dn] in pts {
        b.click(de, dn);
    }
}

#[test]
fn measure_says_the_total_length_and_writes_nothing() {
    let mut b = bench("measure");
    assert_eq!(b.session.prompt().text(), "Mesafe ölç: ilk noktayı belirtin");
    clicks(&mut b, &[[-14.0, 4.0], [-14.0, 10.0]]);
    // The tag adds the whole path's length, the cursor's segment included.
    b.move_to(-8.0, 10.0);
    let tag = tag(&b);
    assert_eq!(tag, ["6.000 m", "Semt 100.0000 g", "Toplam 12.000 m"]);
    b.click(-8.0, 10.0);
    // A measure never closes: the first point again is one more point.
    b.click(-14.0, 4.0);
    assert_eq!(b.points(), 4);
    let revision = b.doc.revision();
    b.confirm();
    // 6 + 6 + √(36 + 36).
    assert_eq!(b.last_text(), Some("Toplam uzunluk 20.485 m (3 kenar)"));
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.doc.revision(), revision, "nothing is written");
    assert!(!b.doc.can_undo());
    assert!(b.session.is_running(), "the tool waits for the next path");
    assert_eq!(b.points(), 0);
}

#[test]
fn area_says_the_area_and_perimeter_and_closes_on_its_first_corner() {
    let mut b = bench("area");
    assert_eq!(b.session.prompt().text(), "Alan hesapla: ilk noktayı belirtin");
    clicks(&mut b, &[[-14.0, 4.0], [-8.0, 4.0], [-8.0, 10.0], [-14.0, 10.0]]);
    b.move_to(-14.0, 4.0);
    assert_eq!(tag(&b).last().map(String::as_str), Some("Alan 36.00 m²"));
    // Clicking the first corner closes the ring and measures it.
    b.click(-14.0, 4.0);
    assert_eq!(b.last_text(), Some("Alan 36.00 m²   Çevre 24.000 m"));
    assert!(!b.doc.can_undo(), "nothing is written");
    // Too few corners: the web's words, and the draft is dropped.
    clicks(&mut b, &[[-14.0, 4.0], [-8.0, 4.0]]);
    b.confirm();
    assert_eq!(b.last_text(), Some("Alan hesapla için en az 3 nokta gerekir."));
    assert_eq!(b.points(), 0);
}

#[test]
fn a_parcel_is_numbered_on_the_parcel_layer_and_selected() {
    let mut b = bench("parcel");
    assert_eq!(b.session.prompt().text(), "Parsel: ilk noktayı belirtin");
    let count = b.doc.entities().count();
    clicks(
        &mut b,
        &[[-14.0, 12.0], [-8.0, 12.0], [-8.0, 18.0], [-14.0, 18.0], [-14.0, 12.0]],
    );
    assert_eq!(b.doc.entities().count(), count + 1);
    let Entity::Polygon(p) = b.newest().clone() else {
        panic!("a closed area");
    };
    assert_eq!(p.base.layer_id, "parsel", "whatever the active layer");
    assert_eq!(p.base.label.as_deref(), Some("4"));
    let attrs: Vec<(&str, &str)> = p
        .base
        .attrs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    // The deed area is the title deed's, left empty (CLAUDE.md §7, §23).
    assert_eq!(
        attrs,
        [
            ("Ada", ""),
            ("Mahalle", ""),
            ("Nitelik", "Arsa"),
            ("Pafta", ""),
            ("Parsel", "4"),
            ("Tapu alanı (m²)", ""),
        ]
    );
    let corners: Vec<[f64; 2]> = p.pts.iter().map(|&q| rel(q)).collect();
    assert_eq!(corners, [[-14.0, 12.0], [-8.0, 12.0], [-8.0, 18.0], [-14.0, 18.0]]);
    assert_eq!(b.selected(), [p.base.id]);
    assert_eq!(
        b.last_text(),
        Some(
            "Parsel 4 oluşturuldu; geometrik alanı 36.00 m². Ada, mahalle ve tapu alanı bilgisini Öznitelikler panelinden girin."
        )
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
}

#[test]
fn parcel_numbers_are_read_as_parseint_reads_them() {
    let mut b = bench("parcel");
    // “12/1” is 12; a text without leading digits counts as 0.
    let slots: Vec<Slot> = b
        .doc
        .entities()
        .filter(|e| e.base().layer_id == "parsel")
        .map(|e| Slot(e.base().id))
        .collect();
    for (slot, number) in slots.iter().zip(["12/1", " 7", "ada"]) {
        let mut e = b.doc.get(*slot).cloned().expect("a parcel");
        e.base_mut().attrs.insert("Parsel".into(), number.into());
        b.doc.update(*slot, e);
    }
    clicks(
        &mut b,
        &[[-14.0, 12.0], [-8.0, 12.0], [-8.0, 18.0], [-14.0, 12.0]],
    );
    assert_eq!(b.newest().base().label.as_deref(), Some("13"));
}

#[test]
fn a_locked_parcel_layer_is_said_in_the_tools_own_words() {
    let mut b = bench("parcel");
    b.doc.toggle_layer_locked("parsel");
    let count = b.doc.entities().count();
    clicks(
        &mut b,
        &[[-14.0, 12.0], [-8.0, 12.0], [-8.0, 18.0], [-14.0, 12.0]],
    );
    assert_eq!(b.doc.entities().count(), count);
    assert_eq!(
        b.last_text(),
        Some("“Parseller” katmanı kilitli; Parsel bu katmana yazar. Kilidi Katmanlar panelinden açın.")
    );
    assert_eq!(b.last_level(), Some(Level::Warn));
}

#[test]
fn g_while_a_length_is_waited_for_takes_the_point_back_and_ends_the_wait() {
    let mut b = bench("measure");
    clicks(&mut b, &[[-14.0, 4.0], [-14.0, 10.0]]);
    b.type_text("U");
    assert_eq!(
        b.session.prompt().text(),
        "Mesafe ölç: son doğrultuda devam edilecek uzunluğu yazın [Geri (G)]"
    );
    // G is the prompt's own option now, not Kapalı alan's shortcut (docs/adr/0069).
    assert_eq!(b.options(), ["G"]);
    b.type_text("G");
    assert_eq!(b.points(), 1);
    assert_eq!(b.options(), ["Y", "U", "G"]);
    // A number is a point again, not a length.
    assert!(b.type_text("@0,5"));
    assert_eq!(b.points(), 2);
}
