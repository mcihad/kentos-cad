//! Bitişik alan (docs/adr/0162 §3): an open path whose ends lie in or on
//! the neighbouring areas; the region it closes with them is written as one
//! area. The scene is `fixtures/interaction/v1/adjoin.kcad`: parcels 101
//! (−30..−10) and 103 (10..30) between y 0 and 24 and a 4 m transformer lot
//! (−2..2, 10..14) between them on Parsel (the active layer), a road below
//! them on Yol (y −8..0). Expected values are worked out by hand; the web's
//! are `apps/web/src/tools/adjoinTool.test.ts`, the shared trace
//! `fixtures/interaction/v1/adjoin.json`.

mod common;

use common::{Bench, rel};
use kentos_contracts::{Entity, PathEntity};
use kentos_interaction::{Level, Overlap};

const NO_REGION: &str = "Yol komşu alanlarla kapalı bir bölge oluşturmuyor: ilk ve son noktayı komşu alanların içine ya da sınırına koyun.";

fn bench() -> Bench {
    let mut b = Bench::on(include_str!(
        "../../../../fixtures/interaction/v1/adjoin.kcad"
    ));
    b.draft.snap = false;
    b
}

/// Parcels and the road: what Seçili katmanlarda önle closes against here.
fn with_the_road(b: &mut Bench) {
    b.draft.overlap = Overlap::Layers;
    b.overlap_layers = vec!["parsel".to_owned(), "yol".to_owned()];
}

/// A ring turned to start at its lowest corner (x, then y), counter-clockwise:
/// the core starts and turns a ring where it likes.
fn lowest_first(mut ring: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    let twice: f64 = (0..ring.len())
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum();
    if twice < 0.0 {
        ring.reverse();
    }
    let k = (0..ring.len())
        .min_by(|&a, &b| ring[a].partial_cmp(&ring[b]).expect("numbers"))
        .unwrap_or(0);
    ring.rotate_left(k);
    ring
}

fn ring(pts: &[kentos_contracts::Vec2]) -> Vec<[f64; 2]> {
    lowest_first(pts.iter().map(|q| rel(*q)).collect())
}

fn newest_area(b: &Bench) -> &PathEntity {
    match b.newest() {
        Entity::Polygon(p) => p,
        other => panic!("an area: {other:?}"),
    }
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

fn draw(b: &mut Bench, corners: &[[f64; 2]]) {
    b.start("adjoin");
    for c in corners {
        b.click(c[0], c[1]);
    }
    b.confirm();
}

const ACROSS: [[f64; 2]; 2] = [[-15.0, 20.0], [15.0, 20.0]];

#[test]
fn serbest_closes_against_its_own_layer_only_and_the_path_stays() {
    let mut b = bench();
    b.start("adjoin");
    assert_eq!(
        b.session.prompt().text(),
        "Bitişik alan: ilk noktayı komşu alanın içinde ya da sınırında belirtin"
    );
    b.click(-15.0, 20.0);
    b.click(15.0, 20.0);
    b.confirm();
    // The road is on Yol: below the path the gap stays open.
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.last_text(), Some(NO_REGION));
    assert_eq!(b.doc.entities().count(), 4);
    assert_eq!(b.points(), 2);
}

#[test]
fn with_the_road_chosen_the_gap_closes_and_the_trafo_is_its_hole() {
    let mut b = bench();
    with_the_road(&mut b);
    draw(&mut b, &ACROSS);
    let made = newest_area(&b);
    assert_eq!(made.base.layer_id, "parsel");
    assert_eq!(ring(&made.pts), rect(-10.0, 0.0, 10.0, 20.0));
    let holes = made.holes.as_deref().unwrap_or_default();
    assert_eq!(holes.len(), 1);
    assert_eq!(ring(&holes[0].pts), rect(-2.0, 10.0, 2.0, 14.0));
    assert!(made.parts.is_none());
    assert_eq!(b.last_text(), Some("Bitişik alan eklendi: 384.00 m²"));
    assert_eq!(b.points(), 0);
    // One undo step, by the tool's name.
    assert_eq!(b.doc.undo().as_deref(), Some("Bitişik alan"));
    assert_eq!(b.doc.entities().count(), 4);
}

#[test]
fn geri_takes_back_an_end_that_closed_nothing() {
    let mut b = bench();
    with_the_road(&mut b);
    b.start("adjoin");
    b.click(-15.0, 20.0);
    // North of everything: the path hangs loose.
    b.click(0.0, 40.0);
    b.confirm();
    assert_eq!(b.last_text(), Some(NO_REGION));
    assert_eq!(b.points(), 2);
    assert!(b.type_text("G"));
    assert_eq!(b.points(), 1);
    b.click(15.0, 20.0);
    b.confirm();
    assert_eq!(b.last_text(), Some("Bitişik alan eklendi: 384.00 m²"));
    assert_eq!(b.doc.entities().count(), 5);
}

#[test]
fn a_gap_closed_all_round_fills_on_both_sides_of_the_path() {
    let mut b = bench();
    with_the_road(&mut b);
    b.add_path("parsel", &rect(-10.0, 24.0, 10.0, 34.0), true);
    draw(&mut b, &ACROSS);
    let made = newest_area(&b);
    assert_eq!(ring(&made.pts), rect(-10.0, 0.0, 10.0, 24.0));
    assert_eq!(b.last_text(), Some("Bitişik alan eklendi: 464.00 m²"));
}

#[test]
fn a_path_through_a_neighbour_makes_one_area_of_two_parts() {
    let mut b = bench();
    with_the_road(&mut b);
    // Parcel 102 between the two: the path crosses it, a gap on each side.
    b.add_path("parsel", &rect(-4.0, 0.0, 4.0, 24.0), true);
    draw(&mut b, &ACROSS);
    let made = newest_area(&b);
    let mut rings: Vec<Vec<[f64; 2]>> = std::iter::once(ring(&made.pts))
        .chain(made.parts.iter().flatten().map(|part| ring(&part.pts)))
        .collect();
    rings.sort_by(|a, b| a[0].partial_cmp(&b[0]).expect("numbers"));
    assert_eq!(
        rings,
        [rect(-10.0, 0.0, -4.0, 20.0), rect(4.0, 0.0, 10.0, 20.0)]
    );
    assert_eq!(
        b.last_text(),
        Some("Bitişik alan eklendi: 240.00 m² (2 parça)")
    );
}

#[test]
fn the_preview_fills_the_region_the_cursor_would_close() {
    let mut b = bench();
    with_the_road(&mut b);
    b.start("adjoin");
    b.click(-15.0, 20.0);
    b.move_to(15.0, 20.0);
    let preview = b.run(|s, cx| s.preview(&cx.format())).expect("a preview");
    // The region with its hole: two rings, filled.
    assert_eq!(preview.areas.len(), 1);
    assert_eq!(preview.areas[0].rings.len(), 2);
    let lines = preview.tag.map(|t| t.lines).expect("a tag");
    assert_eq!(&lines[2..], ["Yol 30.000 m", "Alan 384.00 m²"]);
    // The cursor leaves the parcels: nothing would close.
    b.move_to(15.0, 40.0);
    let preview = b.run(|s, cx| s.preview(&cx.format())).expect("a preview");
    assert!(preview.areas.is_empty());
}

#[test]
fn many_neighbour_edges_leave_the_region_to_the_clicks() {
    let mut b = bench();
    with_the_road(&mut b);
    // A ring of 2 400 corners in view, away from the gap: over the preview's budget.
    let round: Vec<[f64; 2]> = (0..2400)
        .map(|i| {
            let a = f64::from(i) * std::f64::consts::TAU / 2400.0;
            [-44.0 + 3.0 * a.cos(), -30.0 + 3.0 * a.sin()]
        })
        .collect();
    b.add_path("parsel", &round, true);
    b.start("adjoin");
    b.click(-15.0, 20.0);
    b.move_to(15.0, 20.0);
    let preview = b.run(|s, cx| s.preview(&cx.format())).expect("a preview");
    assert!(preview.areas.is_empty(), "no region at a move");
    b.click(15.0, 20.0);
    let preview = b.run(|s, cx| s.preview(&cx.format())).expect("a preview");
    assert_eq!(preview.areas.len(), 1, "the region at the click");
    b.confirm();
    assert_eq!(b.last_text(), Some("Bitişik alan eklendi: 384.00 m²"));
}

#[test]
fn a_hidden_layers_areas_are_no_neighbours() {
    let mut b = bench();
    with_the_road(&mut b);
    b.doc.set_layer_visible("yol", false);
    draw(&mut b, &ACROSS);
    assert_eq!(b.last_text(), Some(NO_REGION));
}
