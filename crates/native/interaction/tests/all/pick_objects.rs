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

/// The test drawing with Otomatik seçim off (a CBS project's own): the active layer is Çizim, the parcel is on Parsel.
fn auto_select_off() -> Bench {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    let mut snapshot = b.doc.to_snapshot_v2();
    snapshot.settings.auto_select = Some(false);
    b.doc = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
    b.spatial = kentos_interaction::Spatial::of(&b.doc);
    b
}

/// Kenar uzunluklarını yaz's field: areas, polylines and lines.
fn edge_lengths_field() -> PickObjects {
    PickObjects::new(
        "Nesneler",
        Some(vec!["polygon".into(), "polyline".into(), "line".into()]),
    )
}

/// A field's objects are read, not selected (docs/adr/0622 §5): with Otomatik seçim off the parcel on another layer
/// than the active one is taken by a click inside it, on its edge and by a window, as the field's own Görünen scope
/// would take it (the owner, 19 November 2026: the polygon on the drawing could not be picked).
#[test]
fn a_fields_pick_takes_another_layers_area_with_otomatik_secim_off() {
    let mut b = auto_select_off();
    b.run(|s, _| s.run(Box::new(edge_lengths_field())));
    // Inside, away from every edge.
    b.click(-18.0, 9.0);
    assert_eq!(b.selected(), [4]);
    b.click(-18.0, 9.0);
    assert!(b.selected().is_empty(), "turned over");
    // On its west edge.
    b.click(-24.0, 9.0);
    assert_eq!(b.selected(), [4]);
    b.click(-24.0, 9.0);
    // A window round it (the Çizim lines lie east of it).
    let before = b.log.len();
    b.drag([-30.0, 0.0], [-6.0, 20.0]);
    assert_eq!(b.selected(), [4]);
    assert!(
        b.said(before).is_empty(),
        "nothing left out to say: {:?}",
        b.said(before)
    );
}

/// The selection a window works on (Çizimden seç) is a selecting step: with Otomatik seçim off another layer's
/// parcel is not taken, by a click or a window, and the click says why and what to do.
#[test]
fn a_windows_selection_pick_keeps_to_the_active_layer_and_says_why() {
    let mut b = auto_select_off();
    b.run(|s, _| s.run(Box::new(PickObjects::selection("Seçim"))));
    let before = b.log.len();
    b.click(-18.0, 9.0);
    assert!(b.selected().is_empty());
    assert_eq!(
        b.said(before).last().map(|l| l.1),
        Some(
            "Tıklanan nesne başka katmanda. Otomatik seçim kapalıyken yalnız etkin katmanın (“Çizim”) nesneleri seçilir: katmanı etkin yapın ya da Otomatik seçim'i açın."
        )
    );
    assert!(
        matches!(b.views.last(), Some(kentos_interaction::ViewChange::LayerLeftOut)),
        "the host offers to turn Otomatik seçim on"
    );
    b.drag([-30.0, 0.0], [-6.0, 20.0]);
    assert!(b.selected().is_empty());
}

/// A click that takes nothing though an object lies under it says why: the field's kinds, the selection filter, a
/// form's layer. Away from every object nothing is said.
#[test]
fn a_click_that_takes_nothing_says_why() {
    // The spot height P5, east of the parcel: a point, and no area round it.
    let mut b = areas_only();
    let before = b.log.len();
    b.click(20.0, -8.0);
    assert!(b.selected().is_empty());
    assert_eq!(
        b.said(before).last().map(|l| l.1),
        Some("Alanlar: tıklanan nesne (Nokta) bu alana alınmaz.")
    );
    let before = b.log.len();
    b.click(30.0, 30.0);
    assert!(b.said(before).is_empty(), "nothing under the pointer");

    // The selection filter holds lines alone: the parcel is left out, and said.
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.draft.select_kinds = Some(kentos_interaction::selectable::bit("line"));
    b.run(|s, _| s.run(Box::new(edge_lengths_field())));
    let before = b.log.len();
    b.click(-18.0, 9.0);
    assert!(b.selected().is_empty());
    assert_eq!(
        b.said(before).last().map(|l| l.1),
        Some("Seçim süzgeci tıklanan nesneyi (Kapalı alan) dışarıda bıraktı.")
    );

    // A form's list from the Çizim layer: the parcel is of another.
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.run(|s, _| s.run(Box::new(PickObjects::on_layer("Parsel", "cizim"))));
    let before = b.log.len();
    b.click(-18.0, 9.0);
    assert!(b.selected().is_empty());
    assert_eq!(
        b.said(before).last().map(|l| l.1),
        Some("Parsel: yalnız “Çizim” katmanının nesneleri alınır; tıklanan nesne (Kapalı alan) başka katmanda.")
    );
}
