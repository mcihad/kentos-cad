//! A wall given a style keeps what is bound to its faces (docs/adr/0721 §6): an İç ölçü zinciri across a room of one
//! closed wall and a partition stays bound when the walls take KentOS's Dış duvar 25 and İç duvar 15 (their faces where
//! they were; the lines between their components added), and its values do not change.

use kentos_commands::{DocumentSnapshotV1, Entity};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

/// A 12 × 9 m room of one closed wall 0.25 m wide, its inner face on the path, and a partition 0.15 m wide across it.
fn room() -> Document {
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Oda","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100,"workspace":"cad"},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[
        {"kind":"wall","id":1,"layerId":"a","attrs":{},"closed":true,"width":0.25,"justify":"right",
         "pts":[{"x":0,"y":0},{"x":12,"y":0},{"x":12,"y":9},{"x":0,"y":9}]},
        {"kind":"wall","id":2,"layerId":"a","attrs":{},"width":0.15,"justify":"center",
         "pts":[{"x":4.075,"y":0},{"x":4.075,"y":9}]}],
        "styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads")).expect("opens")
}

fn done(doc: &mut Document, name: &str, input: Value) -> Value {
    let result = registry::run(&mut ExecutionContext::new(doc), name, kentos_commands::CallOp::Execute, input).unwrap();
    assert_eq!(result["status"], "completed", "{name}: {result}");
    result["output"].clone()
}

/// Each dimension's span and its references' breaks.
fn dimensions(doc: &Document) -> Vec<(f64, Vec<Option<String>>)> {
    doc.entities()
        .filter_map(|e| match e {
            Entity::Dimension(d) => Some((
                (d.b.x - d.a.x).hypot(d.b.y - d.a.y),
                d.refs
                    .as_ref()
                    .map(|r| [&r.a, &r.b].iter().map(|s| s.as_ref().and_then(|s| s.broken.clone())).collect())
                    .unwrap_or_default(),
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn an_interior_chain_stays_bound_when_its_walls_take_a_style() {
    let mut doc = room();
    done(&mut doc, "cad.dimensions.interior", json!({
        "from": {"x": -1.0, "y": 4.5}, "to": {"x": 13.0, "y": 4.5}, "layerId": "a"
    }));
    let before = dimensions(&doc);
    // The shell, the west room, the partition, the east room, the shell.
    assert_eq!(before.len(), 5, "{before:?}");
    let shell = doc.uid(Slot(1)).expect("the shell").to_string();
    let partition = doc.uid(Slot(2)).expect("the partition").to_string();
    done(&mut doc, "cad.walls.edit", json!({"uids": [shell], "operation": {"kind": "set", "style": "Dış duvar 25"}}));
    done(&mut doc, "cad.walls.edit", json!({"uids": [partition], "operation": {"kind": "set", "style": "İç duvar 15"}}));
    let after = dimensions(&doc);
    assert_eq!(after.len(), before.len());
    for ((was, _), (now, broken)) in before.iter().zip(&after) {
        assert!(broken.iter().all(Option::is_none), "a tie broke: {after:?}");
        assert!((was - now).abs() < 1e-9, "{before:?} → {after:?}");
    }
}
