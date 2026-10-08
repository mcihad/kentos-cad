//! Grips through the session, while no command runs: the web's `SelectTool`
//! (`apps/web/src/tools/SelectTool.ts`, docs/adr/0068) over the selection
//! traces' drawing (fixtures/interaction/v1/objects.kcad): line 1 from
//! (−24, −12) to (−8, −12), the 12 × 10 m area 4 from (−24, 4), point 5 at
//! (20, −8), line 6 on the locked layer from (4, −16) to (16, −16). Expected
//! values are worked out by hand.

use crate::common;

use std::collections::BTreeMap;

use common::{Bench, E, N, rel};
use kentos_contracts::{CircleEntity, Entity, EntityBase, Vec2 as Wire};
use kentos_domain::Slot;
use kentos_interaction::{Format, GripSet, Level, Vec2};

const OBJECTS: &str = include_str!("../../../../../fixtures/interaction/v1/objects.kcad");

const PROMPT: &str = "Tutamaç: yeni konumu belirtin ya da koordinat yazın (Esc: vazgeç)";

/// The drawing with `slots` selected, snapping off.
fn bench(slots: &[u32]) -> Bench {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.selection
        .set(slots.iter().map(|s| Slot(*s)).collect::<Vec<_>>());
    b
}

fn ends(b: &Bench, slot: u32) -> Vec<[f64; 2]> {
    match b.doc.get(Slot(slot)) {
        Some(Entity::Line(l)) => vec![rel(l.a), rel(l.b)],
        Some(Entity::Polygon(p)) => p.pts.iter().map(|&q| rel(q)).collect(),
        _ => panic!("a line or an area"),
    }
}

#[test]
fn a_dragged_grip_moves_the_end_in_one_step() {
    let mut b = bench(&[1]);
    b.drag([-8.0, -12.0], [-8.0, -6.0]);
    assert_eq!(ends(&b, 1), [[-24.0, -12.0], [-8.0, -6.0]]);
    assert_eq!(b.selected(), [1], "the selection stays");
    assert!(!b.session.grip_active());
    assert_eq!(b.doc.undo().as_deref(), Some("Tutamaçla düzenle"));
    assert_eq!(ends(&b, 1), [[-24.0, -12.0], [-8.0, -12.0]]);
}

#[test]
fn a_clicked_grip_is_hot_until_the_next_click_places_it() {
    let mut b = bench(&[1]);
    b.click(-24.0, -12.0);
    assert!(b.session.grip_active());
    assert_eq!(b.session.prompt().text(), PROMPT);
    assert_eq!(b.session.active_grip(), Some((Slot(1), 0)));
    // The preview: the line as it would be, dashed; the distance beside the pointer.
    b.move_to(-24.0, -4.0);
    let preview = b
        .session
        .preview(&Format::default())
        .expect("a grip's preview");
    assert_eq!(preview.tag.expect("a tag").lines, ["8.000 m"]);
    assert!(preview.strokes.iter().any(|s| s.dash == Some([4.0, 3.0])));
    b.click(-24.0, -4.0);
    assert_eq!(ends(&b, 1), [[-24.0, -4.0], [-8.0, -12.0]]);
    assert!(!b.session.grip_active());
    assert_eq!(b.session.prompt().text(), "Komut");
}

#[test]
fn a_typed_point_places_the_grip_from_where_it_was() {
    let mut b = bench(&[1]);
    b.click(-8.0, -12.0);
    assert!(b.type_text("@0,4"));
    assert_eq!(ends(&b, 1), [[-24.0, -12.0], [-8.0, -8.0]]);
    // With no grip, typed text is not the select tool's.
    assert!(!b.type_text("@0,4"));
}

#[test]
fn enter_places_the_grip_at_the_pointer_and_esc_leaves_it() {
    let mut b = bench(&[1]);
    b.click(-8.0, -12.0);
    b.move_to(0.0, 0.0);
    assert!(b.run(|s, cx| s.cancel(cx)), "Esc is the grip's");
    assert_eq!(ends(&b, 1), [[-24.0, -12.0], [-8.0, -12.0]]);
    assert_eq!(b.selected(), [1]);
    b.click(-8.0, -12.0);
    b.move_to(-8.0, -8.0);
    assert!(b.session.confirms());
    b.confirm();
    assert_eq!(ends(&b, 1), [[-24.0, -12.0], [-8.0, -8.0]]);
    // With no grip, Enter is not the select tool's: the last command repeats.
    assert!(!b.session.confirms());
}

#[test]
fn a_mid_grip_adds_a_corner_and_a_short_segment_hides_it() {
    let mut b = bench(&[4]);
    b.drag([-18.0, 4.0], [-18.0, 0.0]);
    assert_eq!(
        ends(&b, 4),
        [
            [-24.0, 4.0],
            [-18.0, 0.0],
            [-12.0, 4.0],
            [-12.0, 14.0],
            [-24.0, 14.0]
        ]
    );
    b.spatial.sync(&b.doc);
    let sets = b.spatial.grips(&[Slot(4)]);
    let set = &sets[0];
    assert_eq!(set.vertices, 5);
    let first = set
        .segments
        .iter()
        .position(|s| *s == Some(0))
        .expect("the first segment's mid grip");
    // 7.2 m long at 8 px per metre: 58 px, shown.
    assert!(set.shown(first, &common::Camera));
    // A segment shorter than 28 px on the area offers no mid grip: 2 m is 16 px.
    let (a, c) = (Vec2::new(E, N), Vec2::new(E + 2.0, N));
    let short = GripSet::new(
        Slot(9),
        vec![a, c, Vec2::new(E + 1.0, N), Vec2::new(E + 1.0, N)],
        vec![None, None, Some(0), Some(1)],
        2,
    );
    assert!(short.shown(0, &common::Camera), "a vertex always shows");
    assert!(!short.shown(2, &common::Camera));
    // A multi-part area (docs/adr/0143): a 2 m square, then a 10 m one. Each mid grip is
    // measured on its own part's edge: the second part's show, the first part's do not.
    let square = |x: f64, side: f64| {
        [(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)]
            .map(|(dx, dy)| Vec2::new(E + x + dx, N + dy))
            .to_vec()
    };
    let mids = |ring: &[Vec2]| -> Vec<Vec2> {
        (0..ring.len())
            .map(|i| {
                let (p, q) = (ring[i], ring[(i + 1) % ring.len()]);
                Vec2::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0)
            })
            .collect()
    };
    let (small, big) = (square(0.0, 2.0), square(20.0, 10.0));
    let points: Vec<Vec2> = [small.clone(), mids(&small), big.clone(), mids(&big)].concat();
    let segments: Vec<Option<usize>> = [
        vec![None; 4],
        (0..4).map(Some).collect(),
        vec![None; 4],
        (0..4).map(Some).collect(),
    ]
    .concat();
    let two = GripSet::new(Slot(10), points, segments, 4);
    assert!((4..8).all(|i| !two.shown(i, &common::Camera)), "2 m: 16 px");
    assert!(
        (12..16).all(|i| two.shown(i, &common::Camera)),
        "10 m: 80 px"
    );
    assert_eq!(two.rings[12], Some((8, 4)));
}

#[test]
fn a_locked_objects_grips_are_not_taken() {
    let mut b = bench(&[6]);
    b.drag([4.0, -16.0], [7.0, -10.0]);
    assert_eq!(ends(&b, 6), [[4.0, -16.0], [16.0, -16.0]]);
    // The drag was a selection box, with nothing inside it.
    assert_eq!(b.selected(), Vec::<u32>::new());
}

#[test]
fn a_grip_that_would_break_the_shape_is_left_where_it_was() {
    let mut b = bench(&[]);
    let circle = b
        .doc
        .add(Entity::Circle(CircleEntity {
            base: EntityBase {
                id: 0,
                layer_id: "cizim".to_owned(),
                color: None,
                attrs: BTreeMap::new(),
                label: None,
                symbol: None,
                line_weight: None,
            },
            c: Wire { x: E, y: N + 20.0 },
            r: 4.0,
        }))
        .expect("a slot");
    let undo_before = b.doc.can_undo();
    b.selection.set([circle]);
    // A quadrant onto the centre: no circle is left.
    b.drag([4.0, 20.0], [0.0, 20.0]);
    let Some(Entity::Circle(c)) = b.doc.get(circle) else {
        panic!("the circle");
    };
    assert_eq!(c.r, 4.0);
    assert_eq!(
        b.last_text(),
        Some("Bu konum geçersiz bir şekil oluşturuyor; tutamaç yerinde bırakıldı.")
    );
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.doc.can_undo(), undo_before, "no step");
}

#[test]
fn a_grip_snaps_from_where_it_was() {
    let mut b = bench(&[1]);
    b.draft.snap = true;
    b.click(-8.0, -12.0);
    // Line 2's end at (0, 4) is within the snap aperture of the pointer.
    b.move_to(0.3, 4.2);
    b.click(0.3, 4.2);
    assert_eq!(ends(&b, 1), [[-24.0, -12.0], [0.0, 4.0]]);
}

/// A point on the area as the traces' view shows it.
fn on_screen(de: f64, dn: f64) -> [f64; 2] {
    use kentos_interaction::View;
    common::Camera.to_screen(Vec2::new(E + de, N + dn))
}

fn corners(b: &Bench) -> (Vec<[f64; 2]>, Option<Vec<f64>>) {
    match b.doc.get(Slot(4)) {
        Some(Entity::Polygon(p)) => (p.pts.iter().map(|&q| rel(q)).collect(), p.bulges.clone()),
        _ => panic!("the area"),
    }
}

/// A grip left waiting while its layer is locked from the panel is refused
/// in the command's words; one whose object is deleted meanwhile is said
/// (the web's dd39864: the grip writes through `cad.entities.edit`).
#[test]
fn a_waiting_grip_meets_a_lock_or_a_deletion() {
    let mut b = bench(&[1]);
    b.click(-8.0, -12.0);
    assert!(b.session.grip_active());
    b.doc.toggle_layer_locked("cizim");
    b.move_to(-8.0, -6.0);
    b.click(-8.0, -6.0);
    assert_eq!(
        ends(&b, 1),
        [[-24.0, -12.0], [-8.0, -12.0]],
        "nothing written"
    );
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some(
            "“Çizim” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."
        )
    );
    b.doc.toggle_layer_locked("cizim");
    b.click(-8.0, -12.0);
    assert!(b.session.grip_active());
    b.doc.remove(&[Slot(1)]);
    b.move_to(-8.0, -6.0);
    b.click(-8.0, -6.0);
    assert_eq!(
        b.last_text(),
        Some(
            "Tutamacın nesnesi artık çizimde yok (silinmiş ya da geri alınmış); tutamaç bırakıldı."
        )
    );
}

/// The grip menu (docs/adr/0074, the web's `gripItems`) on the 12 × 10 m
/// area 4, a counter-clockwise ring from (−24, 4): a corner offers Köşeyi
/// sil; the bottom edge's middle Ortasına köşe ekle and Yaya dönüştür, which
/// bows it out of the ring (bulge 0.5, sagitta 3 m, the mid grip at
/// (−18, 1)); there Düz kenar yap makes it straight again. Each is one step
/// through `cad.entities.edit`, named after its operation.
#[test]
fn the_grip_menu_edits_a_vertex_or_an_edge_through_the_command() {
    use kentos_interaction::grip_menu::{self, GripAction};
    let mut b = bench(&[4]);
    let corner = b
        .run(|_, cx| grip_menu::at(on_screen(-24.0, 4.0), cx))
        .expect("a corner's grip");
    assert_eq!(corner.header(), "Köşe 1");
    assert_eq!(corner.actions(), [GripAction::RemoveVertex]);
    let edge = b
        .run(|_, cx| grip_menu::at(on_screen(-18.0, 4.0), cx))
        .expect("the bottom edge's middle");
    assert_eq!(edge.header(), "Kenar 1");
    assert_eq!(edge.actions(), [GripAction::AddVertex, GripAction::ArcEdge]);
    b.run(|_, cx| grip_menu::apply(edge, GripAction::ArcEdge, cx));
    assert_eq!(b.last_text(), Some("Yaya dönüştür: tamam."));
    assert_eq!(corners(&b).1, Some(vec![0.5, 0.0, 0.0, 0.0]));
    let arc = b
        .run(|_, cx| grip_menu::at(on_screen(-18.0, 1.0), cx))
        .expect("the arc's middle");
    assert_eq!(
        arc.actions(),
        [GripAction::AddVertex, GripAction::StraightEdge]
    );
    b.run(|_, cx| grip_menu::apply(arc, GripAction::StraightEdge, cx));
    assert_eq!(b.last_text(), Some("Düz kenar yap: tamam."));
    assert_eq!(corners(&b).1, None, "no arc left: no bulges");
    assert_eq!(b.doc.undo().as_deref(), Some("Düz kenar yap"));
    assert_eq!(b.doc.undo().as_deref(), Some("Yaya dönüştür"));

    b.run(|_, cx| grip_menu::apply(edge, GripAction::AddVertex, cx));
    assert_eq!(b.last_text(), Some("Köşe ekle: tamam."));
    assert_eq!(
        corners(&b).0,
        [
            [-24.0, 4.0],
            [-18.0, 4.0],
            [-12.0, 4.0],
            [-12.0, 14.0],
            [-24.0, 14.0]
        ]
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Köşe ekle"));

    b.run(|_, cx| grip_menu::apply(corner, GripAction::RemoveVertex, cx));
    assert_eq!(b.last_text(), Some("Köşe sil: tamam."));
    assert_eq!(corners(&b).0, [[-12.0, 4.0], [-12.0, 14.0], [-24.0, 14.0]]);
    // A triangle keeps its corners: the core's refusal, nothing written.
    let revision = b.doc.revision();
    let triangle = b
        .run(|_, cx| grip_menu::at(on_screen(-12.0, 4.0), cx))
        .expect("a corner's grip");
    b.run(|_, cx| grip_menu::apply(triangle, GripAction::RemoveVertex, cx));
    assert_eq!(b.last_text(), Some("Kapalı alanda en az üç köşe kalmalı."));
    assert_eq!(b.doc.revision(), revision);
    // A line's grips have no menu.
    let mut line = bench(&[1]);
    assert_eq!(
        line.run(|_, cx| grip_menu::at(on_screen(-8.0, -12.0), cx)),
        None
    );
}

/// A multi-part area's grip menu acts on the grip's own part and keeps the
/// others (docs/adr/0143): a second part, a 6 m square from (0, 20), is
/// added to area 4; its corner and its edge's middle offer what any area's
/// do, and the headings count in that part.
#[test]
fn the_grip_menu_edits_the_part_its_grip_is_on() {
    use kentos_contracts::AreaPart;
    use kentos_interaction::grip_menu::{self, GripAction};
    let mut b = bench(&[4]);
    let Some(Entity::Polygon(mut area)) = b.doc.get(Slot(4)).cloned() else {
        panic!("the area");
    };
    let at = |de: f64, dn: f64| Wire {
        x: E + de,
        y: N + dn,
    };
    area.parts = Some(vec![AreaPart {
        pts: vec![at(0.0, 20.0), at(6.0, 20.0), at(6.0, 26.0), at(0.0, 26.0)],
        bulges: None,
        holes: None,
        zs: None,
    }]);
    let first = (area.pts.clone(), area.bulges.clone());
    assert!(b.doc.update(Slot(4), Entity::Polygon(area)));
    let part = |b: &Bench| match b.doc.get(Slot(4)) {
        Some(Entity::Polygon(p)) => {
            assert_eq!(
                (p.pts.clone(), p.bulges.clone()),
                first,
                "the first part stays"
            );
            let q = &p.parts.as_ref().expect("the second part")[0];
            (
                q.pts.iter().map(|&w| rel(w)).collect::<Vec<_>>(),
                q.bulges.clone(),
            )
        }
        _ => panic!("the area"),
    };
    let corner = b
        .run(|_, cx| grip_menu::at(on_screen(6.0, 26.0), cx))
        .expect("the part's corner");
    assert_eq!((corner.part, corner.header()), (1, "Köşe 3".to_owned()));
    let edge = b
        .run(|_, cx| grip_menu::at(on_screen(3.0, 20.0), cx))
        .expect("the part's bottom edge's middle");
    assert_eq!((edge.part, edge.header()), (1, "Kenar 1".to_owned()));
    b.run(|_, cx| grip_menu::apply(edge, GripAction::ArcEdge, cx));
    assert_eq!(b.last_text(), Some("Yaya dönüştür: tamam."));
    assert_eq!(part(&b).1, Some(vec![0.5, 0.0, 0.0, 0.0]));
    assert_eq!(b.doc.undo().as_deref(), Some("Yaya dönüştür"));
    b.run(|_, cx| grip_menu::apply(corner, GripAction::RemoveVertex, cx));
    assert_eq!(b.last_text(), Some("Köşe sil: tamam."));
    assert_eq!(part(&b).0, [[0.0, 20.0], [6.0, 20.0], [0.0, 26.0]]);
}
