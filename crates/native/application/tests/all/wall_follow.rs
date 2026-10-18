//! A wall's width changed with what follows it (docs/adr/0340 §5, plan/cad.md `CAD-ARCH-08`): a hatch made by a click
//! inside a room of one closed wall follows its inner face, and a dimension bound to the face's corner follows the
//! corner, in the same undo step as the new width; undo takes them back together.

use kentos_domain::contracts::{
    CommandResult, DocumentSnapshotV1, EntitiesHatch, Entity, WallChange, WallsEdit, WallsOperation,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, dimensions_create, entities_hatch, walls_edit};
use serde_json::json;

/// A 6 × 4 m room of one closed wall 0.2 m wide, centred on its path.
fn room() -> Document {
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Oda","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":50,"workspace":"cad"},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[
        {"kind":"wall","id":1,"layerId":"a","attrs":{},"closed":true,"width":0.2,"justify":"center",
         "pts":[{"x":0,"y":0},{"x":6,"y":0},{"x":6,"y":4},{"x":0,"y":4}]}],
        "styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn ring_area(ring: &[kentos_domain::contracts::Vec2]) -> f64 {
    let mut s = 0.0;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        s += a.x * b.y - a.y * b.x;
    }
    s.abs() / 2.0
}

fn hatch_area(doc: &Document) -> f64 {
    let hatch = doc
        .entities()
        .find_map(|e| match e {
            Entity::Hatch(h) => Some(h.clone()),
            _ => None,
        })
        .expect("a hatch");
    ring_area(&hatch.ring)
}

#[test]
fn a_room_hatch_follows_the_wall_getting_thicker() {
    let mut doc = room();
    let input: EntitiesHatch = serde_json::from_value(json!({
        "region": {"kind": "point", "at": {"x": 3, "y": 2}, "way": "lines"}, "pattern": {"name": "Dolu"}, "layerId": "a"
    }))
    .expect("an input");
    let made = entities_hatch::execute(&mut ExecutionContext::new(&mut doc), input);
    assert!(matches!(made, CommandResult::Completed { .. }), "{made:?}");
    // The inner face: 5.8 × 3.8.
    assert!(
        (hatch_area(&doc) - 5.8 * 3.8).abs() < 1e-9,
        "{}",
        hatch_area(&doc)
    );
    let uid = doc.uid(Slot(1)).expect("the wall").to_string();
    // The inner face's first edge measured from its corner to its corner: bound to the wall's line 0, vertices 0, 1.
    let corner = |index: u32| json!({"kind": "place", "uid": uid, "place": {"kind": "vertex", "part": 0, "ring": 0, "index": index}});
    let dim: kentos_domain::contracts::DimensionsCreate = serde_json::from_value(json!({
        "measure": {"kind": "aligned", "a": corner(0), "b": corner(1)}, "at": {"x": 3, "y": 1}, "layerId": "a"
    }))
    .expect("an input");
    let made = dimensions_create::execute(&mut ExecutionContext::new(&mut doc), dim);
    assert!(matches!(made, CommandResult::Completed { .. }), "{made:?}");
    let span = |doc: &Document| {
        doc.entities()
            .find_map(|e| match e {
                Entity::Dimension(d) => Some((d.b.x - d.a.x).hypot(d.b.y - d.a.y)),
                _ => None,
            })
            .expect("a dimension")
    };
    assert!((span(&doc) - 5.8).abs() < 1e-9, "{}", span(&doc));
    let edited = walls_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        WallsEdit {
            uids: vec![uid],
            operation: WallsOperation::Set(WallChange {
                width: Some(0.4),
                ..WallChange::default()
            }),
            expected_revision: None,
        },
    );
    assert!(
        matches!(edited, CommandResult::Completed { .. }),
        "{edited:?}"
    );
    // The same step: 5.6 × 3.6, and the face's edge 5.6 long.
    assert!(
        (hatch_area(&doc) - 5.6 * 3.6).abs() < 1e-9,
        "{}",
        hatch_area(&doc)
    );
    assert!((span(&doc) - 5.6).abs() < 1e-9, "{}", span(&doc));
    assert_eq!(doc.undo().as_deref(), Some("Duvarı düzenle"));
    assert!(
        (hatch_area(&doc) - 5.8 * 3.8).abs() < 1e-9,
        "{}",
        hatch_area(&doc)
    );
    assert!((span(&doc) - 5.8).abs() < 1e-9, "{}", span(&doc));
}
