//! The overlap control (docs/adr/0162 §1–§2) in the tools that draw a new
//! area by its outline: Serbest writes it as drawn; Kendi katmanında önle
//! cuts what overlaps the visible areas of its own layer, Seçili katmanlarda
//! önle those of the chosen layers; covered, nothing is written; cut apart,
//! one multi-part area. The scene is `fixtures/interaction/v1/overlap.kcad`:
//! parcels 21 (−25..−5) and 22 (−5..15) between y −10 and 10 on Parsel (the
//! active layer), a road below them on Yol (y −16..−10). Expected values are
//! worked out by hand; the web's are `apps/web/src/tools/overlap.test.ts`,
//! the shared trace `fixtures/interaction/v1/overlap.json`.

mod common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_interaction::{Level, Overlap};

fn bench() -> Bench {
    let mut b = Bench::on(include_str!(
        "../../../../fixtures/interaction/v1/overlap.kcad"
    ));
    b.draft.snap = false;
    b
}

/// The newest area's corners, its first part, turned to start at its lowest
/// corner (x, then y): the overlay starts a ring where it likes.
fn newest_ring(b: &Bench) -> Vec<[f64; 2]> {
    match b.newest() {
        Entity::Polygon(p) => lowest_first(p.pts.iter().map(|q| rel(*q)).collect()),
        other => panic!("an area: {other:?}"),
    }
}

fn lowest_first(mut ring: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    let k = (0..ring.len())
        .min_by(|&a, &b| ring[a].partial_cmp(&ring[b]).expect("numbers"))
        .unwrap_or(0);
    ring.rotate_left(k);
    ring
}

fn draw(b: &mut Bench, tool: &str, corners: &[[f64; 2]]) {
    b.start(tool);
    for c in corners {
        b.click(c[0], c[1]);
    }
    b.confirm();
}

const SIDE: [[f64; 2]; 4] = [[10.0, 0.0], [25.0, 0.0], [25.0, 8.0], [10.0, 8.0]];

#[test]
fn serbest_writes_the_new_area_as_drawn() {
    let mut b = bench();
    draw(&mut b, "polygon", &SIDE);
    assert_eq!(newest_ring(&b), SIDE);
    assert_eq!(b.last_text(), Some("Kapalı alan eklendi: 120.00 m²"));
}

#[test]
fn kendi_katmaninda_onle_cuts_it_back_to_the_neighbour() {
    let mut b = bench();
    b.draft.overlap = Overlap::Layer;
    let before = b.log.len();
    draw(&mut b, "polygon", &SIDE);
    assert_eq!(
        newest_ring(&b),
        [[15.0, 0.0], [25.0, 0.0], [25.0, 8.0], [15.0, 8.0]]
    );
    let said: Vec<&str> = b.said(before).into_iter().map(|(_, t)| t).collect();
    assert!(
        said.ends_with(&[
            "Çakışma önlendi: 1 komşu alanla örtüşen kısım çıkarıldı.",
            "Kapalı alan eklendi: 80.00 m²"
        ]),
        "{said:?}"
    );
    // One undo step takes the new area away.
    assert_eq!(b.doc.entities().count(), 4);
    assert!(b.doc.undo().is_some());
    assert_eq!(b.doc.entities().count(), 3);
}

#[test]
fn a_neighbour_of_another_layer_cuts_only_when_its_layer_is_chosen() {
    let mut b = bench();
    let across = [[10.0, -14.0], [20.0, -14.0], [20.0, -4.0], [10.0, -4.0]];
    // Kendi katmanında: parcel 22 cuts, the road (Yol) does not.
    b.draft.overlap = Overlap::Layer;
    draw(&mut b, "polygon", &across);
    assert_eq!(
        newest_ring(&b),
        [
            [10.0, -14.0],
            [20.0, -14.0],
            [20.0, -4.0],
            [15.0, -4.0],
            [15.0, -10.0],
            [10.0, -10.0]
        ]
    );
    // Seçili katmanlarda, Yol chosen: the road cuts, the parcel does not.
    b.draft.overlap = Overlap::Layers;
    b.overlap_layers = vec!["yol".to_owned()];
    draw(&mut b, "polygon", &across);
    assert_eq!(
        newest_ring(&b),
        [[10.0, -10.0], [20.0, -10.0], [20.0, -4.0], [10.0, -4.0]]
    );
}

#[test]
fn nothing_is_written_when_the_neighbours_cover_the_new_area() {
    let mut b = bench();
    b.draft.overlap = Overlap::Layer;
    draw(
        &mut b,
        "polygon",
        &[[-20.0, -5.0], [-10.0, -5.0], [-10.0, 5.0], [-20.0, 5.0]],
    );
    assert_eq!(b.doc.entities().count(), 3);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some("Yeni alan komşu alanların içinde kalıyor; alan eklenmedi.")
    );
}

#[test]
fn seçili_katmanlarda_with_no_layer_chosen_says_so_and_writes_the_area() {
    let mut b = bench();
    b.draft.overlap = Overlap::Layers;
    let before = b.log.len();
    draw(&mut b, "polygon", &SIDE);
    assert_eq!(newest_ring(&b), SIDE);
    let said: Vec<&str> = b.said(before).into_iter().map(|(_, t)| t).collect();
    assert!(said.contains(&"Seçili katmanlarda önle kipinde seçili katman yok: alan olduğu gibi yazıldı. Katmanları Çakışma hücresinin menüsünden seçin."), "{said:?}");
}

#[test]
fn a_hidden_layers_areas_do_not_count() {
    let mut b = bench();
    b.draft.overlap = Overlap::Layers;
    b.overlap_layers = vec!["yol".to_owned()];
    b.doc.set_layer_visible("yol", false);
    let across = [[10.0, -14.0], [20.0, -14.0], [20.0, -4.0], [10.0, -4.0]];
    draw(&mut b, "polygon", &across);
    assert_eq!(newest_ring(&b), across);
}

#[test]
fn a_cut_in_two_is_one_area_of_two_parts() {
    let mut b = bench();
    b.add_path(
        "cizim",
        &[[40.0, -5.0], [42.0, -5.0], [42.0, 15.0], [40.0, 15.0]],
        true,
    );
    b.draft.overlap = Overlap::Layers;
    b.overlap_layers = vec!["cizim".to_owned()];
    draw(
        &mut b,
        "polygon",
        &[[35.0, 0.0], [50.0, 0.0], [50.0, 10.0], [35.0, 10.0]],
    );
    let Entity::Polygon(p) = b.newest() else {
        panic!("an area");
    };
    let mut rings: Vec<Vec<[f64; 2]>> = std::iter::once(&p.pts)
        .chain(p.parts.iter().flatten().map(|part| &part.pts))
        .map(|pts| lowest_first(pts.iter().map(|q| rel(*q)).collect()))
        .collect();
    rings.sort_by(|a, b| a[0].partial_cmp(&b[0]).expect("numbers"));
    assert_eq!(
        rings,
        [
            vec![[35.0, 0.0], [40.0, 0.0], [40.0, 10.0], [35.0, 10.0]],
            vec![[42.0, 0.0], [50.0, 0.0], [50.0, 10.0], [42.0, 10.0]],
        ]
    );
}

#[test]
fn a_rectangle_is_cut_as_a_drawn_area_is() {
    let mut b = bench();
    b.draft.overlap = Overlap::Layer;
    b.start("rectangle");
    b.click(10.0, 5.0);
    b.click(20.0, 15.0);
    assert_eq!(
        newest_ring(&b),
        [
            [10.0, 10.0],
            [15.0, 10.0],
            [15.0, 5.0],
            [20.0, 5.0],
            [20.0, 15.0],
            [10.0, 15.0]
        ]
    );
}

#[test]
fn a_parcel_is_cut_against_the_parcel_layer() {
    let mut b = bench();
    b.draft.overlap = Overlap::Layer;
    // The road is no parcel's neighbour: only parcel 22 is cut away.
    draw(
        &mut b,
        "parcel",
        &[[10.0, -14.0], [20.0, -14.0], [20.0, -4.0], [10.0, -4.0]],
    );
    let Entity::Polygon(p) = b.newest() else {
        panic!("a parcel");
    };
    assert_eq!(p.base.layer_id, "parsel");
    assert_eq!(p.base.attrs.get("Parsel").map(String::as_str), Some("23"));
    assert_eq!(
        newest_ring(&b),
        [
            [10.0, -14.0],
            [20.0, -14.0],
            [20.0, -4.0],
            [15.0, -4.0],
            [15.0, -10.0],
            [10.0, -10.0]
        ]
    );
    let text = b.last_text().expect("said");
    assert!(
        text.starts_with("Parsel 23 oluşturuldu; geometrik alanı 70.00 m²"),
        "{text}"
    );
}

#[test]
fn alan_olarak_ciz_is_cut_in_its_one_step() {
    let mut b = bench();
    draw(&mut b, "area", &SIDE);
    b.draft.overlap = Overlap::Layer;
    assert!(b.type_text("A"));
    assert_eq!(
        newest_ring(&b),
        [[15.0, 0.0], [25.0, 0.0], [25.0, 8.0], [15.0, 8.0]]
    );
    assert_eq!(b.last_text(), Some("Alan olarak çizildi: 80.00 m²."));
    assert_eq!(b.doc.undo().as_deref(), Some("Alan olarak çiz"));
    assert_eq!(b.doc.entities().count(), 3);
}
