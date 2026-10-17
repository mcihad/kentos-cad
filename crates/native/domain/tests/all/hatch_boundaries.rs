//! A hatch's exact boundary and a face's tie in the document (docs/adr/0328 §1, §3), worked out by hand on
//! axis-parallel line work: a boundary draws the ring, a ring changed by itself drops the boundary, a hatch tied to
//! the line work closing its face follows it when one line moves and when all of them move, and breaks when the
//! face opens.

use kentos_domain::contracts::{
    DocumentSnapshotV1, Entity, EntityBase, EntityId, HatchAssoc, HatchCurve, HatchEntity,
    HatchLoop, HatchPattern, HatchPatternType, LineEntity, PathEntity, Vec2,
};
use kentos_domain::{Document, Slot};
use std::f64::consts::TAU;

fn empty() -> Document {
    let text = r#"{"format":"kentos.document","version":1,"name":"Deneme","settings":{"srid":5256,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000},"origin":{"x":0,"y":0},
        "layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[],"styles":{"items":[],"categories":[]}}"#;
    Document::from_snapshot(DocumentSnapshotV1::from_json(text).expect("reads")).expect("opens")
}

fn base() -> EntityBase {
    EntityBase {
        look: Default::default(),
        id: 0,
        layer_id: "a".into(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
        attachments: Vec::new(),
    }
}

fn v(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

fn hatch(ring: Vec<Vec2>, boundary: Option<Vec<HatchLoop>>, assoc: Option<HatchAssoc>) -> Entity {
    Entity::Hatch(HatchEntity {
        base: base(),
        ring,
        holes: None,
        pattern: HatchPattern::user(HatchPatternType::Solid, 0.0, 1.0),
        assoc,
        boundary,
    })
}

fn line(a: Vec2, b: Vec2) -> Entity {
    Entity::Line(LineEntity {
        base: base(),
        a,
        b,
        za: None,
        zb: None,
        ma: None,
        mb: None,
    })
}

fn the_hatch(doc: &Document, slot: Slot) -> HatchEntity {
    match doc.get(slot) {
        Some(Entity::Hatch(h)) => h.clone(),
        other => panic!("{other:?}"),
    }
}

fn id(doc: &Document, slot: Slot) -> EntityId {
    EntityId(*doc.uid(slot).expect("an id").as_bytes())
}

/// The shoelace area of a ring.
fn area(r: &[Vec2]) -> f64 {
    let n = r.len();
    (0..n)
        .map(|i| r[i].x * r[(i + 1) % n].y - r[(i + 1) % n].x * r[i].y)
        .sum::<f64>()
        / 2.0
}

/// A square whose top edge is a half circle out: its exact boundary draws the ring, arcs as chords, the area
/// within a hundredth of the half disc's; the boundary stays.
#[test]
fn a_boundary_draws_the_ring() {
    let mut doc = empty();
    let boundary = vec![HatchLoop {
        pts: vec![v(0.0, 0.0), v(4.0, 0.0), v(4.0, 4.0), v(0.0, 4.0)],
        bulges: Some(vec![0.0, 0.0, 1.0, 0.0]),
        curves: None,
    }];
    let slot = doc
        .add(hatch(
            vec![v(0.0, 0.0), v(4.0, 0.0), v(4.0, 4.0)],
            Some(boundary.clone()),
            None,
        ))
        .expect("added");
    let h = the_hatch(&doc, slot);
    assert_eq!(h.boundary, Some(boundary));
    assert!(h.ring.len() > 20, "{}", h.ring.len());
    let want = 16.0 + std::f64::consts::PI * 2.0 * 2.0 / 2.0;
    assert!((area(&h.ring) - want).abs() < 0.01, "{}", area(&h.ring));
}

/// A boundary whose ellipse does not end at its loop's point is dropped; the ring stays as written.
#[test]
fn a_boundary_whose_curve_misses_its_points_is_dropped() {
    let mut doc = empty();
    let boundary = vec![HatchLoop {
        pts: vec![v(5.0, 0.0)],
        bulges: None,
        curves: Some(vec![Some(HatchCurve::Ellipse {
            c: v(0.0, 0.0),
            major: v(4.0, 0.0),
            ratio: 0.5,
            t0: 0.0,
            t1: TAU,
        })]),
    }];
    let ring = vec![v(4.0, 0.0), v(0.0, 2.0), v(-4.0, 0.0), v(0.0, -2.0)];
    let slot = doc
        .add(hatch(ring.clone(), Some(boundary), None))
        .expect("added");
    let h = the_hatch(&doc, slot);
    assert_eq!(h.boundary, None);
    assert_eq!(h.ring, ring);
}

/// A ring changed by itself (a grip, the vertex table) drops the boundary: the drawn ring is all it has now.
#[test]
fn a_ring_changed_by_itself_drops_the_boundary() {
    let mut doc = empty();
    let boundary = vec![HatchLoop {
        pts: vec![v(0.0, 0.0), v(4.0, 0.0), v(4.0, 4.0), v(0.0, 4.0)],
        bulges: Some(vec![0.0, 0.0, 1.0, 0.0]),
        curves: None,
    }];
    let slot = doc
        .add(hatch(Vec::new(), Some(boundary), None))
        .expect("added");
    let mut h = the_hatch(&doc, slot);
    h.ring[0] = v(-1.0, -1.0);
    let ring = h.ring.clone();
    doc.update(slot, Entity::Hatch(h));
    let h = the_hatch(&doc, slot);
    assert_eq!(h.boundary, None);
    assert_eq!(h.ring, ring);
}

/// Four long lines crossing at the corners of 0..40 × 0..30, a hatch tied to the face around (5, 5).
fn tied_face() -> (Document, [Slot; 4], Slot) {
    let mut doc = empty();
    let lines = [
        line(v(-10.0, 0.0), v(60.0, 0.0)),
        line(v(40.0, -10.0), v(40.0, 40.0)),
        line(v(60.0, 30.0), v(-10.0, 30.0)),
        line(v(0.0, 40.0), v(0.0, -10.0)),
    ];
    let slots = lines.map(|l| doc.add(l).expect("added"));
    let tie = HatchAssoc {
        outer: None,
        bounds: slots.iter().map(|s| id(&doc, *s)).collect(),
        islands: Vec::new(),
        cutouts: Vec::new(),
        seed: v(5.0, 5.0),
    };
    let ring = vec![v(0.0, 0.0), v(40.0, 0.0), v(40.0, 30.0), v(0.0, 30.0)];
    let hatch = doc.add(hatch(ring, None, Some(tie))).expect("added");
    (doc, slots, hatch)
}

/// The right line moved to x = 50: the face is 0..50 × 0..30, the tie and the seed stay.
#[test]
fn a_face_follows_its_line_work() {
    let (mut doc, lines, hatch) = tied_face();
    doc.update(lines[1], line(v(50.0, -10.0), v(50.0, 40.0)));
    let h = the_hatch(&doc, hatch);
    assert_eq!(area(&h.ring).abs(), 1500.0);
    let tie = h.assoc.expect("still tied");
    assert_eq!(tie.bounds.len(), 4);
    assert_eq!(tie.seed, v(5.0, 5.0));
    doc.undo();
    assert_eq!(area(&the_hatch(&doc, hatch).ring).abs(), 1200.0);
}

/// All four lines moved 100 to the east in one step: the seed would be outside; it moves as the line work did,
/// the face is 100..140 × 0..30.
#[test]
fn a_face_moved_whole_takes_its_seed_along() {
    let (mut doc, lines, hatch) = tied_face();
    let changes: Vec<(Slot, Entity)> = lines
        .iter()
        .map(|s| {
            let Some(Entity::Line(l)) = doc.get(*s) else {
                panic!("a line")
            };
            (*s, line(v(l.a.x + 100.0, l.a.y), v(l.b.x + 100.0, l.b.y)))
        })
        .collect();
    doc.update_many(changes, "Taşı");
    let h = the_hatch(&doc, hatch);
    let tie = h.assoc.expect("still tied");
    assert_eq!(tie.seed, v(105.0, 5.0));
    let xs: Vec<f64> = h.ring.iter().map(|p| p.x).collect();
    assert_eq!(xs.iter().cloned().fold(f64::INFINITY, f64::min), 100.0);
    assert_eq!(xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max), 140.0);
}

/// The top line moved off the face: nothing closes around the seed; the hatch stays where it was, untied.
#[test]
fn a_face_that_opens_breaks_the_tie() {
    let (mut doc, lines, hatch) = tied_face();
    let ring = the_hatch(&doc, hatch).ring;
    doc.update(lines[2], line(v(60.0, 70.0), v(50.0, 70.0)));
    let h = the_hatch(&doc, hatch);
    assert!(h.assoc.is_none());
    assert_eq!(h.ring, ring);
}

/// An area with an arc and an ellipse island inside, the hatch tied to both: moving the area 10 north re-cuts the
/// region and keeps the island's ellipse exact (one point, a whole ellipse run clockwise).
#[test]
fn a_tied_hatch_keeps_its_ellipse_island_exact() {
    let mut doc = empty();
    let room = Entity::Polygon(PathEntity {
        base: base(),
        pts: vec![v(0.0, 0.0), v(20.0, 0.0), v(20.0, 10.0), v(0.0, 10.0)],
        bulges: Some(vec![0.0, 0.0, 0.5, 0.0]),
        holes: None,
        zs: None,
        ms: None,
        parts: None,
        closed: false,
        widths: None,
    });
    let island = Entity::Ellipse(kentos_domain::contracts::EllipseEntity {
        base: base(),
        c: v(10.0, 5.0),
        major: v(3.0, 0.0),
        ratio: 0.5,
        t0: 0.0,
        t1: TAU,
    });
    let room = doc.add(room).expect("added");
    let island = doc.add(island).expect("added");
    let tie = HatchAssoc {
        outer: Some(id(&doc, room)),
        bounds: Vec::new(),
        islands: vec![id(&doc, island)],
        cutouts: Vec::new(),
        seed: v(2.0, 2.0),
    };
    let hatch = doc
        .add(hatch(
            vec![v(0.0, 0.0), v(20.0, 0.0), v(20.0, 10.0)],
            None,
            Some(tie),
        ))
        .expect("added");
    let mut moved = doc.get(room).expect("room").clone();
    if let Entity::Polygon(p) = &mut moved {
        for q in &mut p.pts {
            q.x += 0.0;
        }
        p.pts[2].y += 1.0;
        p.pts[3].y += 1.0;
    }
    doc.update(room, moved);
    let h = the_hatch(&doc, hatch);
    let b = h.boundary.expect("an exact boundary");
    assert_eq!(b.len(), 2, "{b:?}");
    // The room's arc a bulge, its top edge raised.
    assert!(
        b[0].bulges
            .iter()
            .flatten()
            .any(|x| (x.abs() - 0.5).abs() < 1e-12),
        "{b:?}"
    );
    assert!(b[0].pts.iter().any(|p| p.y == 11.0));
    // The island one whole ellipse.
    assert_eq!(b[1].pts.len(), 1);
    match b[1].curves.as_ref().and_then(|c| c[0].as_ref()) {
        Some(HatchCurve::Ellipse {
            c, ratio, t0, t1, ..
        }) => {
            assert_eq!(*c, v(10.0, 5.0));
            assert_eq!(*ratio, 0.5);
            assert!(((t1 - t0).abs() - TAU).abs() < 1e-12);
        }
        other => panic!("{other:?}"),
    }
}
