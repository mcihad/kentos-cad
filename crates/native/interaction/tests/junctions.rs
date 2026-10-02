//! Corners shared with the neighbours (docs/adr/0162 §4): with Topoloji on,
//! a new area drawn by its outline is joined with its neighbours corner by
//! corner, in its own undo step. The scene is
//! `fixtures/interaction/v1/adjoin.kcad`: parcels 101 (−30..−10) and 103
//! (10..30) between y 0 and 24, a 4 m transformer lot (−2..2, 10..14)
//! between them on Parsel (the active layer), a road below them on Yol
//! (y −8..0). Expected values are worked out by hand; the web's are
//! `apps/web/src/tools/junctions.test.ts`, the shared trace
//! `fixtures/interaction/v1/junctions.json`.

mod common;

use common::{Bench, base, rel};
use kentos_contracts::{Entity, PathEntity, Vec2};
use kentos_domain::Slot;
use kentos_interaction::{Level, Overlap};

fn bench() -> Bench {
    let mut b = Bench::on(include_str!(
        "../../../../fixtures/interaction/v1/adjoin.kcad"
    ));
    b.draft.snap = false;
    b.draft.topology = true;
    b
}

fn corners(b: &Bench, slot: u32) -> Vec<[f64; 2]> {
    b.path_pts(Slot(slot))
}

fn newest_ring(b: &Bench) -> Vec<[f64; 2]> {
    match b.newest() {
        Entity::Polygon(p) => p.pts.iter().map(|q| rel(*q)).collect(),
        other => panic!("an area: {other:?}"),
    }
}

/// Draws with `tool` through `corners` and confirms; where the log stood
/// before the confirm (the clicks' coordinates are said before it).
fn draw(b: &mut Bench, tool: &str, corners: &[[f64; 2]]) -> usize {
    b.start(tool);
    for c in corners {
        b.click(c[0], c[1]);
    }
    let at = b.log.len();
    b.confirm();
    at
}

fn said(b: &Bench, before: usize) -> Vec<&str> {
    b.said(before).into_iter().map(|(_, t)| t).collect()
}

const PARCEL_101: [[f64; 2]; 4] = [[-30.0, 0.0], [-10.0, 0.0], [-10.0, 24.0], [-30.0, 24.0]];
const PARCEL_103: [[f64; 2]; 4] = [[10.0, 0.0], [30.0, 0.0], [30.0, 24.0], [10.0, 24.0]];
const GAP: [[f64; 2]; 4] = [[-10.0, 5.0], [10.0, 5.0], [10.0, 20.0], [-10.0, 20.0]];

#[test]
fn topology_off_leaves_the_neighbours_as_they_are() {
    let mut b = bench();
    b.draft.topology = false;
    draw(&mut b, "polygon", &GAP);
    assert_eq!(corners(&b, 1), PARCEL_101);
    assert_eq!(corners(&b, 2), PARCEL_103);
}

#[test]
fn the_new_areas_corners_go_to_the_parcels_on_either_side() {
    let mut b = bench();
    let before = draw(&mut b, "polygon", &GAP);
    assert_eq!(newest_ring(&b), GAP);
    assert_eq!(
        corners(&b, 1),
        [
            [-30.0, 0.0],
            [-10.0, 0.0],
            [-10.0, 5.0],
            [-10.0, 20.0],
            [-10.0, 24.0],
            [-30.0, 24.0]
        ]
    );
    assert_eq!(
        corners(&b, 2),
        [
            [10.0, 0.0],
            [30.0, 0.0],
            [30.0, 24.0],
            [10.0, 24.0],
            [10.0, 20.0],
            [10.0, 5.0]
        ]
    );
    assert_eq!(
        said(&b, before),
        [
            "Topolojik düzenleme: 2 komşu nesneye yeni alanın 4 köşesi eklendi.",
            "Kapalı alan eklendi: 300.00 m²"
        ]
    );
    // One undo step takes the area and the corners away.
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
    assert_eq!(b.doc.entities().count(), 4);
    assert_eq!(corners(&b, 1), PARCEL_101);
    assert_eq!(corners(&b, 2), PARCEL_103);
}

#[test]
fn the_new_area_takes_the_parcels_corners_on_its_edge() {
    let mut b = bench();
    // A strip over both parcels: their corners (±10, 24) lie on its bottom
    // edge, its own (±12, 24) on their tops.
    let before = draw(
        &mut b,
        "polygon",
        &[[-12.0, 24.0], [12.0, 24.0], [12.0, 30.0], [-12.0, 30.0]],
    );
    assert_eq!(
        newest_ring(&b),
        [
            [-12.0, 24.0],
            [-10.0, 24.0],
            [10.0, 24.0],
            [12.0, 24.0],
            [12.0, 30.0],
            [-12.0, 30.0]
        ]
    );
    assert_eq!(
        corners(&b, 1),
        [
            [-30.0, 0.0],
            [-10.0, 0.0],
            [-10.0, 24.0],
            [-12.0, 24.0],
            [-30.0, 24.0]
        ]
    );
    assert_eq!(
        corners(&b, 2),
        [
            [10.0, 0.0],
            [30.0, 0.0],
            [30.0, 24.0],
            [12.0, 24.0],
            [10.0, 24.0]
        ]
    );
    assert_eq!(
        said(&b, before),
        [
            "Topolojik düzenleme: 2 komşu nesneye yeni alanın 2 köşesi eklendi.",
            "Topolojik düzenleme: yeni alana komşulardan 2 köşe eklendi.",
            "Kapalı alan eklendi: 144.00 m²"
        ]
    );
}

#[test]
fn a_locked_neighbour_takes_no_corner_and_is_said() {
    let mut b = bench();
    b.doc.toggle_layer_locked("yol");
    let before = b.log.len();
    // Its corners (±5, 0) lie on the road's top edge.
    draw(
        &mut b,
        "polygon",
        &[[-5.0, 0.0], [5.0, 0.0], [5.0, 6.0], [-5.0, 6.0]],
    );
    assert_eq!(
        corners(&b, 4),
        [[-40.0, -8.0], [40.0, -8.0], [40.0, 0.0], [-40.0, 0.0]]
    );
    let lines = b.said(before);
    assert!(
        lines.contains(&(
            Level::Warn,
            "Kilitli katmandaki 1 komşu nesneye köşe eklenmedi."
        )),
        "{lines:?}"
    );
    assert_eq!(b.last_text(), Some("Kapalı alan eklendi: 60.00 m²"));
}

#[test]
fn bitişik_alan_gives_its_corners_to_the_parcels_and_the_road() {
    let mut b = bench();
    b.draft.overlap = Overlap::Layers;
    b.overlap_layers = vec!["parsel".to_owned(), "yol".to_owned()];
    let before = draw(&mut b, "adjoin", &[[-15.0, 20.0], [15.0, 20.0]]);
    assert_eq!(
        corners(&b, 1),
        [
            [-30.0, 0.0],
            [-10.0, 0.0],
            [-10.0, 20.0],
            [-10.0, 24.0],
            [-30.0, 24.0]
        ]
    );
    assert_eq!(
        corners(&b, 2),
        [
            [10.0, 0.0],
            [30.0, 0.0],
            [30.0, 24.0],
            [10.0, 24.0],
            [10.0, 20.0]
        ]
    );
    // The new area's corners (±10, 0) lie on the road's top edge.
    assert_eq!(
        corners(&b, 4),
        [
            [-40.0, -8.0],
            [40.0, -8.0],
            [40.0, 0.0],
            [10.0, 0.0],
            [-10.0, 0.0],
            [-40.0, 0.0]
        ]
    );
    assert_eq!(
        said(&b, before),
        [
            "Topolojik düzenleme: 3 komşu nesneye yeni alanın 4 köşesi eklendi.",
            "Bitişik alan eklendi: 384.00 m²"
        ]
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Bitişik alan"));
    assert_eq!(corners(&b, 4).len(), 4);
}

#[test]
fn a_rectangle_and_a_parcel_join_too() {
    let mut b = bench();
    b.start("rectangle");
    b.click(-10.0, 5.0);
    b.click(-4.0, 9.0);
    assert_eq!(
        corners(&b, 1),
        [
            [-30.0, 0.0],
            [-10.0, 0.0],
            [-10.0, 5.0],
            [-10.0, 9.0],
            [-10.0, 24.0],
            [-30.0, 24.0]
        ]
    );
    // Ctrl+Z inside the tool takes the rectangle and 101's new corners back
    // in one step; drawn again, they come back.
    assert!(b.undo_step());
    assert_eq!(corners(&b, 1), PARCEL_101);
    b.click(-10.0, 5.0);
    b.click(-4.0, 9.0);
    assert_eq!(corners(&b, 1).len(), 6);
    draw(
        &mut b,
        "parcel",
        &[[10.0, 10.0], [16.0, 10.0], [16.0, 14.0], [10.0, 14.0]],
    );
    let Entity::Polygon(parcel) = b.newest() else {
        panic!("a parcel");
    };
    assert_eq!(parcel.base.layer_id, "parsel");
    assert_eq!(
        corners(&b, 2),
        [
            [10.0, 0.0],
            [30.0, 0.0],
            [30.0, 24.0],
            [10.0, 24.0],
            [10.0, 14.0],
            [10.0, 10.0]
        ]
    );
}

#[test]
fn a_point_is_a_corner_with_noktalar_da() {
    let mut b = bench();
    let point = Entity::Point(kentos_contracts::PointEntity {
        base: base("cizim"),
        p: Vec2 {
            x: 487000.0,
            y: 4420005.0,
        },
        z: None,
    });
    b.doc.add(point).expect("a slot");
    draw(&mut b, "polygon", &GAP);
    assert_eq!(newest_ring(&b), GAP);
    b.draft.topology_points = true;
    draw(&mut b, "polygon", &GAP);
    assert_eq!(
        newest_ring(&b),
        [
            [-10.0, 5.0],
            [0.0, 5.0],
            [10.0, 5.0],
            [10.0, 20.0],
            [-10.0, 20.0]
        ]
    );
}

#[test]
fn a_neighbours_new_corner_takes_its_elevation_along_the_edge() {
    let mut b = bench();
    let at = |x: f64, y: f64| Vec2 {
        x: 487000.0 + x,
        y: 4420000.0 + y,
    };
    let lot = Entity::Polygon(PathEntity {
        base: base("parsel"),
        pts: vec![at(40.0, 0.0), at(50.0, 0.0), at(50.0, 10.0), at(40.0, 10.0)],
        bulges: None,
        holes: None,
        zs: Some(vec![Some(100.0), Some(110.0), Some(120.0), Some(130.0)]),
        parts: None,
    });
    let slot = b.doc.add(lot).expect("a slot");
    draw(
        &mut b,
        "polygon",
        &[[50.0, 4.0], [56.0, 4.0], [56.0, 8.0], [50.0, 8.0]],
    );
    let Some(Entity::Polygon(lot)) = b.doc.get(slot) else {
        panic!("the lot");
    };
    let zs: Vec<f64> = lot
        .zs
        .iter()
        .flatten()
        .map(|z| z.unwrap_or(f64::NAN))
        .collect();
    let want = [100.0, 110.0, 114.0, 118.0, 120.0, 130.0];
    assert_eq!(zs.len(), want.len(), "{zs:?}");
    assert!(
        zs.iter().zip(want).all(|(z, w)| (z - w).abs() < 1e-9),
        "{zs:?}"
    );
}
