//! İzle in the path tools (docs/adr/0161 §1): off at first; a click near a
//! line goes onto it while it is on; a segment between points on connected
//! line work runs along it, and closing on the first corner goes back along
//! it; not for Sabit ilk nokta's rays nor İçine tıkla. Two parcels side by
//! side: (0, 0), (20, 0), (20, 10), (0, 12) and (20, 0)–(40, 10); the bench
//! shows 8 px a metre, so the 11 px snap aperture is 1.375 m. Expected
//! values are worked out by hand; the web's are
//! `apps/web/src/tools/pathTrace.test.ts`, the shared trace
//! `fixtures/interaction/v1/trace-draw.json`.

mod common;

use common::{Bench, rel};
use kentos_contracts::Entity;

fn bench() -> Bench {
    let mut b = Bench::on(include_str!(
        "../../../../fixtures/interaction/v1/empty.kcad"
    ));
    b.draft.snap = false;
    b.add_path(
        "cizim",
        &[[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 12.0]],
        true,
    );
    b.add_path(
        "cizim",
        &[[20.0, 0.0], [40.0, 0.0], [40.0, 10.0], [20.0, 10.0]],
        true,
    );
    b
}

fn newest_pts(b: &Bench) -> Vec<[f64; 2]> {
    match b.newest() {
        Entity::Polygon(p) | Entity::Polyline(p) => p.pts.iter().map(|q| rel(*q)).collect(),
        other => panic!("a path: {other:?}"),
    }
}

#[test]
fn off_at_first_a_click_near_a_line_stays_where_it_is() {
    let mut b = bench();
    b.start("polyline");
    b.click(10.0, 0.5);
    assert!(!b.session.prompt().text().contains("İzle (İ): açık"));
    b.click(10.0, 5.0);
    b.confirm();
    assert_eq!(newest_pts(&b), [[10.0, 0.5], [10.0, 5.0]]);
}

#[test]
fn a_new_parcel_closes_along_its_neighbours_boundary() {
    let mut b = bench();
    b.start("polygon");
    b.click(40.0, 10.0);
    assert!(b.type_text("İ"));
    assert_eq!(
        b.session.prompt().text(),
        "Kapalı alan: sonraki noktayı belirtin [Yay (Y) / Uzunluk (U) / İzle (İ): açık / Geri (G)]"
    );
    // Free corners: no line within the aperture, straight edges.
    b.click(40.0, 20.0);
    b.click(0.0, 25.0);
    b.click(0.0, 12.0);
    assert_eq!(b.points(), 4);
    // The first corner again: the last edge goes back along the parcels' northern boundary.
    b.click(40.0, 10.0);
    assert_eq!(
        newest_pts(&b),
        [
            [40.0, 10.0],
            [40.0, 20.0],
            [0.0, 25.0],
            [0.0, 12.0],
            [20.0, 10.0]
        ]
    );
}

#[test]
fn a_click_near_a_line_goes_onto_it_and_the_way_follows_the_boundary() {
    let mut b = bench();
    b.memory.trace = true;
    b.start("polyline");
    // 0.5 m off the southern edge: onto it.
    b.click(10.0, -0.5);
    // Near the shared edge, 0.75 m off: onto it, by the way the boundary goes.
    b.click(20.75, 6.0);
    b.confirm();
    assert_eq!(newest_pts(&b), [[10.0, 0.0], [20.0, 0.0], [20.0, 6.0]]);
}

#[test]
fn sabit_ilk_nokta_rays_follow_nothing() {
    let mut b = bench();
    b.memory.trace = true;
    b.start("measure");
    assert!(b.type_text("S"));
    b.click(10.0, 1.0);
    let before = b.log.len();
    b.click(10.0, 6.0);
    // The first point was not put on the edge 1 m below: 5 m, not 6 (the echo says where the click went).
    let said: Vec<&str> = b.said(before).into_iter().map(|(_, t)| t).collect();
    assert_eq!(
        said,
        ["  Y 487010.000  X 4420006.000", "1: 5.000 m, semt 0.0000 g"]
    );
}

#[test]
fn icine_tikla_measures_the_region_clicked_in() {
    let mut b = bench();
    b.memory.trace = true;
    b.memory.area_inside = true;
    b.start("area");
    let before = b.log.len();
    // 1 m inside the southern edge: the click stays inside, the parcel is measured.
    b.click(10.0, 1.0);
    let said: Vec<&str> = b.said(before).into_iter().map(|(_, t)| t).collect();
    assert_eq!(said[0], "  Y 487010.000  X 4420001.000");
    assert!(said[1].starts_with("Alan 220.00 m²"), "{said:?}");
}
