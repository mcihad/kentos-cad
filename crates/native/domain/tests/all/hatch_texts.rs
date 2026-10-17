//! The texts a hatch leaves open (docs/adr/0332 §2): a moved text takes its hole with it; a text linked to a hatch
//! follows the hatch cut again in the same step; a hatch's own label is not left open (its place is read from the
//! hatch: a cycle); Denetle finds a text that is gone and the cycle and repairs them.

use kentos_domain::audit::AuditKind;
use kentos_domain::contracts::{
    DocumentSnapshotV1, Entity, EntityId, HatchAssoc, HatchEntity, TextEntity, Vec2,
};
use kentos_domain::{Document, Slot};
use serde_json::json;

const GHOST: [u8; 16] = [
    0x01, 0x92, 0xf5, 0xa2, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0xff, 0xff,
];

/// A square area (1), a text inside it (2), a hatch of the square (3) and a text (4).
fn drawing() -> Document {
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Boş bırakılan yazılar","settings":{"srid":5256,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[
        {"kind":"polygon","id":1,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":20,"y":0},{"x":20,"y":20},{"x":0,"y":20}]},
        {"kind":"text","id":2,"layerId":"a","attrs":{},"p":{"x":4,"y":4},"text":"101","height":1,"rotation":0},
        {"kind":"hatch","id":3,"layerId":"a","attrs":{},"label":"Avlu","ring":[{"x":0,"y":0},{"x":20,"y":0},{"x":20,"y":20},{"x":0,"y":20}],
         "pattern":{"type":"lines","angle":0,"spacing":1}},
        {"kind":"text","id":4,"layerId":"a","attrs":{},"p":{"x":15,"y":15},"text":"Avlu","height":1,"rotation":0}],
        "styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn id(doc: &Document, s: u32) -> EntityId {
    EntityId(doc.uid(Slot(s)).expect("known").into_bytes())
}

fn hatch(doc: &Document) -> HatchEntity {
    match doc.get(Slot(3)).cloned() {
        Some(Entity::Hatch(h)) => h,
        _ => panic!("a hatch"),
    }
}

fn text(doc: &Document, s: u32) -> TextEntity {
    match doc.get(Slot(s)).cloned() {
        Some(Entity::Text(t)) => t,
        _ => panic!("a text"),
    }
}

/// The hatch tied to the square, leaving `cutouts` open; cut by the document's rule as the tie changes.
fn tie(doc: &mut Document, cutouts: Vec<EntityId>) {
    let tied = Entity::Hatch(HatchEntity {
        assoc: Some(HatchAssoc {
            gap: None,
            outer: Some(id(doc, 1)),
            bounds: Vec::new(),
            islands: Vec::new(),
            cutouts,
            seed: Vec2 { x: 10.0, y: 10.0 },
        }),
        ..hatch(doc)
    });
    assert!(doc.update(Slot(3), tied));
}

fn holes(h: &HatchEntity) -> Vec<Vec<Vec2>> {
    h.holes.clone().unwrap_or_default()
}

fn middle(ring: &[Vec2]) -> (f64, f64) {
    let n = ring.len() as f64;
    (
        ring.iter().map(|p| p.x).sum::<f64>() / n,
        ring.iter().map(|p| p.y).sum::<f64>() / n,
    )
}

#[test]
fn a_text_left_open_takes_its_hole_where_it_moves() {
    let mut doc = drawing();
    let cutouts = vec![id(&doc, 2)];
    tie(&mut doc, cutouts);
    let open = holes(&hatch(&doc));
    assert_eq!(open.len(), 1, "the text is left open");
    let (x0, y0) = middle(&open[0]);
    assert!(
        x0 > 3.0 && x0 < 8.0 && y0 > 3.0 && y0 < 6.0,
        "around the text: {x0}, {y0}"
    );
    let moved = Entity::Text(TextEntity {
        p: Vec2 { x: 10.0, y: 12.0 },
        ..text(&doc, 2)
    });
    assert!(doc.update(Slot(2), moved));
    let open = holes(&hatch(&doc));
    assert_eq!(open.len(), 1);
    let (x1, y1) = middle(&open[0]);
    assert!(
        (x1 - x0 - 6.0).abs() < 1e-6 && (y1 - y0 - 8.0).abs() < 1e-6,
        "the hole went with it: {x0}, {y0} → {x1}, {y1}"
    );
    // One undo takes the text and its hole back.
    assert!(doc.undo().is_some());
    let (x2, y2) = middle(&holes(&hatch(&doc))[0]);
    assert!((x2 - x0).abs() < 1e-9 && (y2 - y0).abs() < 1e-9);
}

#[test]
fn a_longer_text_widens_its_hole_and_a_moved_out_one_leaves_none() {
    let mut doc = drawing();
    let cutouts = vec![id(&doc, 2)];
    tie(&mut doc, cutouts);
    let width = |h: &HatchEntity| {
        let r = &holes(h)[0];
        r.iter().map(|p| p.x).fold(f64::MIN, f64::max)
            - r.iter().map(|p| p.x).fold(f64::MAX, f64::min)
    };
    let before = width(&hatch(&doc));
    let longer = Entity::Text(TextEntity {
        text: "101/5 parsel".into(),
        ..text(&doc, 2)
    });
    assert!(doc.update(Slot(2), longer));
    assert!(
        width(&hatch(&doc)) > 2.0 * before,
        "a longer text leaves more open"
    );
    let out = Entity::Text(TextEntity {
        p: Vec2 { x: 40.0, y: 4.0 },
        ..text(&doc, 2)
    });
    assert!(doc.update(Slot(2), out));
    let h = hatch(&doc);
    assert!(
        holes(&h).is_empty(),
        "outside the region it leaves nothing open"
    );
    assert_eq!(
        h.assoc.expect("tied").cutouts,
        vec![id(&doc, 2)],
        "and stays listed"
    );
}

#[test]
fn a_hatch_does_not_leave_its_own_label_open() {
    let mut doc = drawing();
    let cutouts = vec![id(&doc, 4)];
    tie(&mut doc, cutouts);
    assert_eq!(holes(&hatch(&doc)).len(), 1, "text 4 is left open");
    // Text 4 becomes the hatch's own label: its place is read from the hatch, which leaving it open would change.
    let label = Entity::Text(TextEntity {
        label_of: Some(id(&doc, 3)),
        label_scale: Some(100.0),
        ..text(&doc, 4)
    });
    assert!(doc.update(Slot(4), label));
    let h = hatch(&doc);
    assert!(holes(&h).is_empty(), "its own label is not left open");
    assert!(
        h.assoc.expect("tied").cutouts.is_empty(),
        "and leaves the list"
    );
    // One undo takes both back.
    assert!(doc.undo().is_some());
    assert_eq!(holes(&hatch(&doc)).len(), 1);
}

/// The drawing as a file from elsewhere holds it: the hatch tied to the square, leaving open `cutouts` (uids as
/// text), text 4 linked to `label_of` when given; read without the document's rules running.
fn from_file(cutouts: &[String], label_of: Option<String>) -> Document {
    let first = drawing();
    let uid = |s: u32| first.uid(Slot(s)).expect("known").to_string();
    let mut snapshot = serde_json::to_value(first.to_snapshot_v2()).expect("json");
    let entities = snapshot["entities"].as_array_mut().expect("entities");
    entities[2]["assoc"] = json!({"outer": uid(1), "cutouts": cutouts, "seed": {"x": 10, "y": 10}});
    if let Some(of) = label_of {
        entities[3]["labelOf"] = json!(of);
        entities[3]["labelScale"] = json!(100);
    }
    Document::from_snapshot_v2(serde_json::from_value(snapshot).expect("reads")).expect("opens")
}

#[test]
fn denetle_finds_a_text_gone_and_a_label_left_open() {
    let first = drawing();
    let ghost = kentos_domain::Uuid::from_bytes(GHOST).to_string();
    let text2 = first.uid(Slot(2)).expect("known").to_string();
    let text4 = first.uid(Slot(4)).expect("known").to_string();
    let hatch3 = first.uid(Slot(3)).expect("known").to_string();
    let doc = from_file(&[text2.clone(), ghost], None);
    assert_eq!(
        doc.uid(Slot(3)).map(|u| u.to_string()),
        Some(hatch3.clone()),
        "the file keeps its ids"
    );
    let faults = doc.audit();
    assert!(
        faults
            .iter()
            .any(|f| f.kind == AuditKind::MissingCutout && f.slot == Slot(3)),
        "{faults:?}"
    );
    assert!(
        !faults.iter().any(|f| f.kind == AuditKind::BrokenHatch),
        "its boundary is there: {faults:?}"
    );
    let Some(Entity::Hatch(r)) = doc.audit_repaired(Slot(3)) else {
        panic!("repaired")
    };
    assert_eq!(r.assoc.clone().expect("tied").cutouts, vec![id(&doc, 2)]);
    assert_eq!(
        holes(&r).len(),
        1,
        "cut again around the text that is there"
    );
    // Its own label left open: a cycle, found and taken off the list.
    let doc = from_file(&[text4], Some(hatch3));
    let faults = doc.audit();
    assert!(
        faults
            .iter()
            .any(|f| f.kind == AuditKind::HatchTextCycle && f.slot == Slot(3)),
        "{faults:?}"
    );
    let Some(Entity::Hatch(r)) = doc.audit_repaired(Slot(3)) else {
        panic!("repaired")
    };
    assert!(r.assoc.expect("tied").cutouts.is_empty());
}
