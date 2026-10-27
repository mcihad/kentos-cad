//! Ölçülü bölme's side follows the pointer (docs/adr/0657's 27 October addendum): the source edge picked, the partition
//! goes to the side the pointer is on, until Sol or Sağ is chosen; the preview's distance is a dimension with its
//! number.

use crate::common::Bench;
use kentos_commands::Entity;

/// One wall along y from (0, 0) to (0, 6), Ölçülü bölme 2 m from it, its source picked.
fn bench() -> Bench {
    let mut b = Bench::new("wall");
    b.click(0., 0.);
    b.click(0., 6.);
    b.confirm();
    b.confirm();
    b.start("wallPartition");
    assert!(b.run(|s, cx| s.configure("U", "2m", cx)));
    b.click(0., 3.);
    b
}

/// The newest wall's x, local.
fn x_of(b: &Bench) -> f64 {
    let Entity::Wall(w) = b.newest() else {
        panic!("a wall")
    };
    w.wall.pts[0].x - crate::common::E
}

#[test]
fn the_partition_goes_to_the_pointers_side() {
    let mut b = bench();
    b.move_to(-3., 3.);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert!(
        preview.texts.iter().any(|t| t.mask && !t.text.is_empty()),
        "the distance's number, masked: {:?}",
        preview.texts
    );
    b.confirm();
    assert!(x_of(&b) < 0.0, "west, where the pointer was: {}", x_of(&b));
    b.start("wallPartition");
    assert!(b.run(|s, cx| s.configure("U", "2m", cx)));
    b.click(0., 3.);
    b.move_to(3., 3.);
    b.confirm();
    assert!(x_of(&b) > 0.0, "east now: {}", x_of(&b));
}

#[test]
fn a_side_chosen_stays_where_the_pointer_goes() {
    let mut b = bench();
    // The wall runs north: its left is west.
    assert!(b.run(|s, cx| s.configure("Y", "left", cx)));
    b.move_to(3., 3.);
    b.confirm();
    assert!(x_of(&b) < 0.0, "Sol kept: {}", x_of(&b));
}
