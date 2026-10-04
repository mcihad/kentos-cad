//! Topolojik düzenleme through the select tool's grips, the grip menu and
//! Esnet (docs/adr/0160) on the shared trace's drawing
//! (fixtures/interaction/v1/topology-edit.kcad): parcels 21 (slot 1) and 22
//! (slot 2) share the edge x = −20 from (−20, −20) to (−20, 10), its corners'
//! elevations 100 and 103 in both; at (−20, 10) the locked triangle 40 (3),
//! the road (4), P1 (5) and a line on a hidden layer (6) meet them. Expected
//! values are worked out by hand; the web's are
//! `apps/web/src/tools/selectGripTopology.test.ts`.

use crate::common;

use common::{Bench, E, N, rel};
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::grip_menu::{self, GripAction};
use kentos_interaction::{Format, Level, Vec2, View};

const SCENE: &str = include_str!("../../../../../fixtures/interaction/v1/topology-edit.kcad");

/// The drawing with `slots` selected, snapping off, the mode as given.
fn bench(slots: &[u32], topology: bool) -> Bench {
    let mut b = Bench::on(SCENE);
    b.draft.snap = false;
    b.draft.topology = topology;
    b.selection
        .set(slots.iter().map(|s| Slot(*s)).collect::<Vec<_>>());
    b
}

fn on_screen(de: f64, dn: f64) -> [f64; 2] {
    common::Camera.to_screen(Vec2::new(E + de, N + dn))
}

fn zs(b: &Bench, slot: u32) -> Vec<Option<f64>> {
    match b.doc.get(Slot(slot)) {
        Some(Entity::Polygon(p)) => p.zs.clone().unwrap_or_else(|| vec![None; p.pts.len()]),
        _ => panic!("an area"),
    }
}

fn bulges(b: &Bench, slot: u32) -> Option<Vec<f64>> {
    match b.doc.get(Slot(slot)) {
        Some(Entity::Polygon(p)) => p.bulges.clone(),
        _ => panic!("an area"),
    }
}

fn road(b: &Bench) -> [[f64; 2]; 2] {
    match b.doc.get(Slot(4)) {
        Some(Entity::Line(l)) => [rel(l.a), rel(l.b)],
        _ => panic!("the road"),
    }
}

fn point(b: &Bench) -> [f64; 2] {
    match b.doc.get(Slot(5)) {
        Some(Entity::Point(p)) => rel(p.p),
        _ => panic!("P1"),
    }
}

const A: [[f64; 2]; 4] = [[-40.0, -20.0], [-20.0, -20.0], [-20.0, 10.0], [-40.0, 10.0]];
const B: [[f64; 2]; 4] = [[-20.0, -20.0], [0.0, -20.0], [0.0, 10.0], [-20.0, 10.0]];
const LOCKED: &str = "Kilitli katmandaki 1 komşu nesne değişmedi; ortak sınır ayrıldı.";

#[test]
fn off_a_grip_moves_its_own_object_only() {
    let mut b = bench(&[1], false);
    b.click(-20.0, 10.0);
    assert!(b.type_text("@1,1"));
    assert_eq!(b.path_pts(Slot(1))[2], [-19.0, 11.0]);
    assert_eq!(b.path_pts(Slot(2)), B);
    assert_eq!(road(&b), [[-20.0, 10.0], [-5.0, 22.0]]);
}

#[test]
fn a_grip_moves_the_shared_corner_in_its_neighbours_in_one_step() {
    let mut b = bench(&[1], true);
    let before = b.log.len();
    b.click(-20.0, 10.0);
    assert!(b.type_text("@1,1"));
    assert_eq!(
        b.path_pts(Slot(1)),
        [[-40.0, -20.0], [-20.0, -20.0], [-19.0, 11.0], [-40.0, 10.0]]
    );
    // The neighbour's corner keeps its elevation as the object's does (docs/adr/0160 §4).
    assert_eq!(
        b.path_pts(Slot(2)),
        [[-20.0, -20.0], [0.0, -20.0], [0.0, 10.0], [-19.0, 11.0]]
    );
    assert_eq!(zs(&b, 2), [Some(100.0), None, None, Some(103.0)]);
    assert_eq!(road(&b), [[-19.0, 11.0], [-5.0, 22.0]]);
    // The locked triangle and the hidden line stay; without Noktalar da so does P1.
    assert_eq!(b.path_pts(Slot(3))[1], [-20.0, 10.0]);
    assert_eq!(point(&b), [-20.0, 10.0]);
    assert_eq!(
        b.said(before),
        [
            (
                Level::Info,
                "Topolojik düzenleme: 2 komşu nesne de değişti."
            ),
            (Level::Warn, LOCKED),
        ]
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Tutamaçla düzenle"));
    assert_eq!(b.path_pts(Slot(1)), A);
    assert_eq!(b.path_pts(Slot(2)), B);
    assert_eq!(road(&b), [[-20.0, 10.0], [-5.0, 22.0]]);
}

#[test]
fn with_noktalar_da_the_point_comes_along() {
    let mut b = bench(&[1], true);
    b.draft.topology_points = true;
    let before = b.log.len();
    b.click(-20.0, 10.0);
    assert!(b.type_text("@1,1"));
    assert_eq!(point(&b), [-19.0, 11.0]);
    assert_eq!(
        b.said(before)[0],
        (
            Level::Info,
            "Topolojik düzenleme: 3 komşu nesne de değişti."
        )
    );
}

/// The grip menu on the shared edge's middle (−20, −5): Yaya dönüştür bows
/// it out of parcel 21 (bulge 0.5 on its second edge), and the same arc
/// runs the other way in parcel 22 (−0.5 on its closing edge); Ortasına köşe
/// ekle puts the middle in both, its elevation halfway along the edge
/// (101.5); Köşeyi sil there takes it out of both again.
#[test]
fn the_grip_menu_puts_the_shared_edge_right() {
    let mut b = bench(&[1], true);
    let edge = b
        .run(|_, cx| grip_menu::at(on_screen(-20.0, -5.0), cx))
        .expect("the shared edge's middle");
    assert_eq!(edge.actions(), [GripAction::AddVertex, GripAction::ArcEdge]);
    let before = b.log.len();
    b.run(|_, cx| grip_menu::apply(edge, GripAction::ArcEdge, cx));
    assert_eq!(
        b.said(before),
        [
            (Level::Success, "Yaya dönüştür: tamam."),
            (
                Level::Info,
                "Topolojik düzenleme: 1 komşu nesne de değişti."
            ),
        ]
    );
    assert_eq!(bulges(&b, 1), Some(vec![0.0, 0.5, 0.0, 0.0]));
    assert_eq!(bulges(&b, 2), Some(vec![0.0, 0.0, 0.0, -0.5]));
    assert_eq!(b.doc.undo().as_deref(), Some("Yaya dönüştür"));
    assert_eq!(bulges(&b, 2), None);

    b.run(|_, cx| grip_menu::apply(edge, GripAction::AddVertex, cx));
    assert_eq!(
        b.path_pts(Slot(1)),
        [
            [-40.0, -20.0],
            [-20.0, -20.0],
            [-20.0, -5.0],
            [-20.0, 10.0],
            [-40.0, 10.0]
        ]
    );
    assert_eq!(
        b.path_pts(Slot(2)),
        [
            [-20.0, -20.0],
            [0.0, -20.0],
            [0.0, 10.0],
            [-20.0, 10.0],
            [-20.0, -5.0]
        ]
    );
    assert_eq!(zs(&b, 1)[2], Some(101.5));
    assert_eq!(zs(&b, 2)[4], Some(101.5));

    let middle = b
        .run(|_, cx| grip_menu::at(on_screen(-20.0, -5.0), cx))
        .expect("the new corner's grip");
    assert_eq!(middle.actions(), [GripAction::RemoveVertex]);
    let before = b.log.len();
    b.run(|_, cx| grip_menu::apply(middle, GripAction::RemoveVertex, cx));
    assert_eq!(
        b.said(before),
        [
            (Level::Success, "Köşe sil: tamam."),
            (
                Level::Info,
                "Topolojik düzenleme: 1 komşu nesne de değişti."
            ),
        ]
    );
    assert_eq!(b.path_pts(Slot(1)), A);
    assert_eq!(b.path_pts(Slot(2)), B);
}

/// Esnet with parcel 21 selected: only it is stretched, and the corner it
/// moves goes in the unselected parcel 22 and the road too, in the same
/// step; the preview draws them.
#[test]
fn esnet_moves_the_shared_corner_of_unselected_neighbours() {
    let mut b = bench(&[1], true);
    b.start("stretch");
    b.click(-22.0, 8.0);
    b.click(-18.0, 12.0);
    b.click(-20.0, 10.0);
    b.move_to(-19.0, 11.0);
    let ghosts = b
        .session
        .preview(&Format::default())
        .expect("the stretch's preview")
        .strokes;
    let drawn = |pts: &[[f64; 2]]| {
        ghosts.iter().any(|s| {
            pts.iter().all(|p| {
                s.pts
                    .iter()
                    .any(|q| (q.x - E - p[0]).abs() < 1e-9 && (q.y - N - p[1]).abs() < 1e-9)
            })
        })
    };
    assert!(drawn(&[[0.0, 10.0], [-19.0, 11.0]]), "parcel 22 follows");
    assert!(drawn(&[[-19.0, 11.0], [-5.0, 22.0]]), "the road follows");
    let before = b.log.len();
    assert!(b.type_text("@1,1"));
    assert_eq!(b.path_pts(Slot(1))[2], [-19.0, 11.0]);
    assert_eq!(b.path_pts(Slot(2))[3], [-19.0, 11.0]);
    assert_eq!(road(&b), [[-19.0, 11.0], [-5.0, 22.0]]);
    assert_eq!(
        b.said(before),
        [
            (Level::Success, "1 nesne esnetildi: ΔY 1.000  ΔX 1.000"),
            (
                Level::Info,
                "Topolojik düzenleme: 2 komşu nesne de değişti."
            ),
            (Level::Warn, LOCKED),
        ]
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Esnet"));
    assert_eq!(b.path_pts(Slot(2)), B);
}

/// Resting on the shared corner's grip: its elevation and how many objects
/// have a corner there (the parcels, the road and the locked triangle; P1
/// with Noktalar da); with the mode off, the elevation alone.
#[test]
fn the_tag_counts_the_objects_sharing_the_corner() {
    let tag = |b: &mut Bench| {
        b.move_to(-20.0, 10.0);
        b.session
            .preview(&Format::default())
            .and_then(|p| p.tag)
            .map(|t| t.lines)
    };
    let mut b = bench(&[1], true);
    assert_eq!(
        tag(&mut b).expect("a tag"),
        ["Kot 103.000 m", "4 nesnenin köşesi"]
    );
    b.draft.topology_points = true;
    assert_eq!(
        tag(&mut b).expect("a tag"),
        ["Kot 103.000 m", "5 nesnenin köşesi"]
    );
    b.draft.topology = false;
    assert_eq!(tag(&mut b).expect("a tag"), ["Kot 103.000 m"]);
    // A corner nobody shares has no count, nor, without an elevation, a tag.
    b.draft.topology = true;
    b.move_to(-40.0, -20.0);
    assert_eq!(
        b.session.preview(&Format::default()).and_then(|p| p.tag),
        None
    );
}
