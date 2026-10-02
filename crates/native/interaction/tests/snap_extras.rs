//! The snap additions as the session gives them to the tools (docs/adr/0163
//! §1, §3, §5): Ağırlık merkezi, Karelaj, the object being drawn and the
//! scale range, through `Session::snap` with the drafting settings. The
//! scene is `fixtures/interaction/v1/snap-additions.kcad`: parcel 7, an L
//! (−20..−4 × −10..−2 and −20..−12 × −2..6), its centroid (−40/3, −10/3).
//! The core's rules are held to the independent reference in
//! `crates/shared/geometry-core/tests/snap.rs`; the web's menu and range in
//! `apps/web/src/ui/statusbar/snapMenu.test.ts`, the shared trace
//! `fixtures/interaction/v1/snap-additions.json`.

mod common;

use common::{Bench, Camera, E, N};
use kentos_geometry_core::store::snap::NO_OBJECT;
use kentos_interaction::{SnapKind, View, default_snap_kinds, screen_scale};

fn bench() -> Bench {
    let mut b = Bench::on(include_str!(
        "../../../../fixtures/interaction/v1/snap-additions.kcad"
    ));
    b.draft.snap = true;
    b.start("polygon");
    b
}

/// The snap at a point, as the desktop takes it for the pointer.
fn snap_at(b: &mut Bench, de: f64, dn: f64) -> Option<(SnapKind, [f64; 2], f64)> {
    b.snapped(de, dn)
        .snap
        .map(|h| (h.kind, [h.point.x - E, h.point.y - N], h.id))
}

const CENTROID: [f64; 2] = [-40.0 / 3.0, -10.0 / 3.0];

fn near(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9
}

#[test]
fn the_additions_are_off_at_first() {
    let mut b = bench();
    assert_eq!(b.draft.snap_kinds, default_snap_kinds());
    assert_eq!(snap_at(&mut b, -13.25, -3.25), None);
    assert_eq!(snap_at(&mut b, 5.3, 7.8), None);
}

#[test]
fn the_centroid_of_a_closed_area() {
    let mut b = bench();
    b.draft.snap_kinds |= SnapKind::Centroid.bit();
    let (kind, at, id) = snap_at(&mut b, -13.25, -3.25).expect("a snap");
    assert_eq!((kind, id), (SnapKind::Centroid, 1.0));
    assert!(near(at, CENTROID), "{at:?}");
}

#[test]
fn karelaj_goes_to_the_nearest_node_of_its_spacings() {
    let mut b = bench();
    b.draft.snap_kinds |= SnapKind::Grid.bit();
    assert_eq!(
        snap_at(&mut b, 5.3, 7.8),
        Some((SnapKind::Grid, [5.0, 8.0], NO_OBJECT))
    );
    // Its own spacings east and north, nodes on their multiples in the project's coordinates.
    b.draft.snap_grid = [0.5, 2.0];
    assert_eq!(
        snap_at(&mut b, 5.3, 7.2),
        Some((SnapKind::Grid, [5.5, 8.0], NO_OBJECT))
    );
    // A point of an object near the cursor comes first: the parcel's corner (−4, −2).
    assert_eq!(
        snap_at(&mut b, -4.3, -2.2).map(|s| s.0),
        Some(SnapKind::Endpoint)
    );
}

#[test]
fn the_path_being_drawn_is_snapped_to_while_snap_self_is_on() {
    let mut b = bench();
    b.draft.snap_kinds |= SnapKind::Centroid.bit();
    b.click(CENTROID[0], CENTROID[1]);
    b.click(5.0, 8.0);
    b.click(12.0, -8.0);
    // The first edge's middle, the first corner over the centroid at the same place.
    let mid = [(CENTROID[0] + 5.0) / 2.0, (CENTROID[1] + 8.0) / 2.0];
    let (kind, at, id) = snap_at(&mut b, -4.0, 2.4).expect("the middle");
    assert_eq!((kind, id), (SnapKind::Midpoint, NO_OBJECT));
    assert!(near(at, mid), "{at:?}");
    assert_eq!(
        snap_at(&mut b, -13.25, -3.25).map(|s| (s.0, s.2)),
        Some((SnapKind::Endpoint, NO_OBJECT))
    );
    // Off: only the drawing's objects.
    b.draft.snap_self = false;
    assert_eq!(snap_at(&mut b, -4.0, 2.4), None);
    assert_eq!(
        snap_at(&mut b, -13.25, -3.25).map(|s| (s.0, s.2)),
        Some((SnapKind::Centroid, 1.0))
    );
}

#[test]
fn out_of_the_scale_range_nothing_snaps() {
    let mut b = bench();
    // The bench's view, 0.125 m a pixel: 1:472.
    assert_eq!(screen_scale(Camera.world_length(1.0)), 472.0);
    let corner = (-4.3, -2.2);
    assert!(snap_at(&mut b, corner.0, corner.1).is_some());
    for (range, snaps) in [
        ([0.0, 0.0], true),
        ([500.0, 0.0], false),
        ([0.0, 471.0], false),
        ([472.0, 472.0], true),
        ([100.0, 1000.0], true),
    ] {
        b.draft.snap_scale = range;
        assert_eq!(
            snap_at(&mut b, corner.0, corner.1).is_some(),
            snaps,
            "{range:?}"
        );
    }
    // A one-shot snap neither (the desktop gives it as the kinds of the draft).
    b.draft.snap_scale = [500.0, 0.0];
    b.draft.snap_kinds = SnapKind::Endpoint.bit();
    assert_eq!(snap_at(&mut b, corner.0, corner.1), None);
}
