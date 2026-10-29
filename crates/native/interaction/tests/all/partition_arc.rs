//! Ölçülü bölme on an arc wall (docs/adr/0734): the arc edge is picked where it is drawn, not along its chord; the
//! partition is an arc about the same centre on the pointer's side, inside or outside; the preview's dashed source
//! face follows the arc.

use std::f64::consts::FRAC_PI_8;

use crate::common::{Bench, E, N};
use kentos_commands::Entity;
use kentos_geometry_core::geom::bulge::bulge_arc;
use kentos_interaction::Vec2;

/// A quarter ring wall of radius 8 about (E, N), counter-clockwise from east to north; Ölçülü bölme 1.5 m from it.
fn bench() -> Bench {
    let mut b = Bench::new("wall");
    let pts = [Vec2::new(E + 8.0, N), Vec2::new(E, N + 8.0)];
    assert!(b.run(|_, cx| kentos_interaction::wall::write(&pts, &[FRAC_PI_8.tan()], false, cx)));
    b.start("wallPartition");
    assert!(b.run(|s, cx| s.configure("U", "1.5m", cx)));
    b
}

/// The point on the arc at `deg` degrees, local.
fn on_arc(deg: f64) -> [f64; 2] {
    let t = deg.to_radians();
    [8.0 * t.cos(), 8.0 * t.sin()]
}

/// The newest wall's arc: its centre, local, and its radius.
fn newest_arc(b: &Bench) -> ([f64; 2], f64) {
    let Entity::Wall(w) = b.newest() else {
        panic!("a wall")
    };
    let (a, c) = (w.wall.pts[0], w.wall.pts[1]);
    let bulge = w
        .wall
        .bulges
        .as_ref()
        .and_then(|v| v.first().copied())
        .expect("an arc");
    let arc = bulge_arc(Vec2::new(a.x, a.y), Vec2::new(c.x, c.y), bulge).expect("an arc");
    ([arc.c.x - E, arc.c.y - N], arc.r)
}

#[test]
fn the_arc_is_picked_where_it_is_drawn_and_the_partition_goes_inside_or_outside() {
    let mut b = bench();
    b.click(on_arc(45.0)[0], on_arc(45.0)[1]);
    b.move_to(2.0, 2.0);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    let face = preview
        .strokes
        .iter()
        .find(|s| s.dash == Some([4.0, 4.0]))
        .expect("the dashed source face");
    assert!(face.pts.len() > 2, "an arc, not its chord: {:?}", face.pts);
    let radii: Vec<f64> = face.pts.iter().map(|p| (p.x - E).hypot(p.y - N)).collect();
    assert!(
        radii
            .iter()
            .all(|r| (r - radii[0]).abs() < 1e-6 && *r < 8.0),
        "the inner face, about the centre: {radii:?}"
    );
    b.confirm();
    let (c, r) = newest_arc(&b);
    assert!(
        c[0].abs() < 1e-6 && c[1].abs() < 1e-6,
        "the same centre: {c:?}"
    );
    assert!(r < 8.0 - 1.5, "inside, where the pointer was: {r}");

    b.start("wallPartition");
    assert!(b.run(|s, cx| s.configure("U", "1.5m", cx)));
    b.click(on_arc(30.0)[0], on_arc(30.0)[1]);
    b.move_to(9.0, 9.0);
    b.confirm();
    let (c, r) = newest_arc(&b);
    assert!(
        c[0].abs() < 1e-6 && c[1].abs() < 1e-6,
        "the same centre: {c:?}"
    );
    assert!(r > 8.0 + 1.5, "outside now: {r}");
}

#[test]
fn the_chord_of_an_arc_is_not_its_edge() {
    let mut b = bench();
    let walls = b.doc.entities().count();
    // The chord's middle, 2.34 m inside the arc.
    b.click(4.0, 4.0);
    assert_eq!(
        b.last_text(),
        Some("Bir duvarın düz ya da yaylı kenarına tıklayın.")
    );
    b.move_to(2.0, 2.0);
    b.confirm();
    assert_eq!(
        b.doc.entities().count(),
        walls,
        "nothing written without a source"
    );
}
