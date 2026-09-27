//! Sahneden seç's objects (docs/adr/0088): what a click takes for a field
//! that wants areas only.

mod common;

use common::Bench;
use kentos_interaction::pick_objects::PickObjects;

const OBJECTS: &str = include_str!("../../../../fixtures/interaction/v1/objects.kcad");

/// The test drawing's parcel (4, from −24,4 to −12,14) with a spot height
/// at its middle, and the pick running for areas only.
fn areas_only() -> Bench {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.start("point");
    b.click(-18.0, 9.0);
    b.confirm();
    b.run(|s, _| {
        s.run(Box::new(PickObjects::new(
            "Alanlar",
            Some(vec!["polygon".to_owned()]),
        )));
    });
    b
}

#[test]
fn a_click_inside_an_area_away_from_its_edges_takes_the_area() {
    let mut b = areas_only();
    // On the spot height in the middle: the point is not taken, no edge is
    // within the aperture, so the parcel it stands in is.
    b.click(-18.0, 9.0);
    assert_eq!(b.selected(), [4]);
    // Again: taken out, as a click turns it over.
    b.click(-18.0, 9.0);
    assert!(b.selected().is_empty());
}

#[test]
fn a_click_outside_every_area_takes_nothing() {
    let mut b = areas_only();
    b.click(30.0, 30.0);
    assert!(b.selected().is_empty());
}
