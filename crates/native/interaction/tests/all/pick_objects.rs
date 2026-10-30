//! Sahneden seç's objects (docs/adr/0088): what a click takes for a field
//! that wants areas only.

use crate::common;

use common::Bench;
use kentos_interaction::pick_objects::PickObjects;

const OBJECTS: &str = include_str!("../../../../../fixtures/interaction/v1/objects.kcad");

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

/// A form's list from a layer (docs/adr/0759 §1): one click takes one object of that layer, past
/// whatever else lies under the pointer, and ends the pick; another layer's object is not taken.
#[test]
fn a_pick_on_a_layer_takes_one_object_of_it() {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.start("point");
    b.click(-18.0, 9.0);
    b.confirm();
    let parcel = b
        .doc
        .get(kentos_domain::Slot(4))
        .expect("the parcel")
        .base()
        .layer_id
        .clone();
    b.run(|s, _| s.run(Box::new(PickObjects::on_layer("Parsel", parcel))));
    b.click(30.0, 30.0);
    assert!(b.selected().is_empty());
    assert!(b.views.is_empty(), "still picking");
    // On the spot height: it is of another layer, the parcel under it is taken.
    b.click(-18.0, 9.0);
    assert_eq!(b.selected(), [4]);
    assert!(matches!(
        b.views.last(),
        Some(kentos_interaction::ViewChange::PickedObjects(true))
    ));
}
