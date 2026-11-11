//! Bonds of arc edges' lines (docs/adr/8049 §4) on the shared cases' drawing (scripts/fixtures/arch_bond_cases.py):
//! the curved corridor's 1.20 m kept as a bond; an arc turned straight takes its bond with it, a straight edge turned
//! into an arc too.

use kentos_commands::{CallOp, DocumentSnapshotV2, Entity};
use kentos_domain::{Document, Uuid};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn uid(i: u64) -> String {
    format!("0192f5a1-7100-7000-8000-{i:012x}")
}

/// The drawing, its corridor's inner arc's outer face (wall 18) bonded to the outer arc's inner face (wall 19).
fn bonded() -> Document {
    let f: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.constraints.edit.arch.json"
    ))
    .unwrap();
    let snapshot: DocumentSnapshotV2 = serde_json::from_value(f["setup"].clone()).unwrap();
    let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.constraints.edit",
        CallOp::Execute,
        json!({"edit": {"kind": "add", "constraints": [{"rule": {
            "kind": "distance",
            "a": {"uid": uid(18), "element": {"kind": "wall", "edge": 0, "line": "rightFace"}},
            "b": {"uid": uid(19), "element": {"kind": "wall", "edge": 0, "line": "leftFace"}},
            "parameter": ""
        }}]}}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    assert_eq!(doc.constraints().parameters[0].expression, "1.2");
    doc
}

fn set_bulge(doc: &mut Document, i: u64, bulge: f64) {
    let slot = doc.slot_of(Uuid::parse_str(&uid(i)).unwrap()).unwrap();
    let mut e = doc.get(slot).unwrap().clone();
    let Entity::Wall(w) = &mut e else {
        unreachable!()
    };
    w.wall.bulges = Some(vec![bulge, 0.0]);
    assert!(doc.update(slot, e));
}

#[test]
fn an_arc_turned_straight_takes_its_bond() {
    let mut doc = bonded();
    set_bulge(&mut doc, 18, 0.0);
    assert!(doc.constraints().constraints.is_empty());
}

#[test]
fn a_straight_edge_turned_into_an_arc_takes_its_bond() {
    let mut doc = bonded();
    // Wall 1's line bonded to wall 6's (the corridor of the room), then wall 1 bent.
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.constraints.edit",
        CallOp::Execute,
        json!({"edit": {"kind": "add", "constraints": [{"rule": {
            "kind": "distance",
            "a": {"uid": uid(1), "element": {"kind": "wall", "edge": 0, "line": "leftFace"}},
            "b": {"uid": uid(6), "element": {"kind": "wall", "edge": 0, "line": "rightFace"}},
            "parameter": ""
        }}]}}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    assert_eq!(doc.constraints().constraints.len(), 2);
    set_bulge(&mut doc, 1, 0.1);
    assert_eq!(
        doc.constraints().constraints.len(),
        1,
        "the corridor's arc bond stays"
    );
}
