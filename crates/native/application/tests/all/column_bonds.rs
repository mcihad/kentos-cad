//! Bonds to a column's side (docs/adr/8049 §3) on the shared cases' drawing (scripts/fixtures/arch_bond_cases.py): a
//! free column kept 0.50 m from the south wall by Mesafe's Bu ölçüyü koru, in one undo step, following the wall; a
//! column tied to grids bonded to the west wall, the grid moved, the column and the wall following in the same step.

use kentos_commands::{CallOp, DocumentSnapshotV2, Entity};
use kentos_domain::{Document, Uuid};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn uid(i: u64) -> String {
    format!("0192f5a1-7100-7000-8000-{i:012x}")
}

fn drawing() -> Document {
    let f: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.constraints.edit.arch.json"
    ))
    .unwrap();
    let snapshot: DocumentSnapshotV2 = serde_json::from_value(f["setup"].clone()).unwrap();
    Document::from_snapshot_v2(snapshot).unwrap()
}

fn run(doc: &mut Document, command: &str, input: Value) -> Value {
    let answer = registry::run(
        &mut ExecutionContext::new(doc),
        command,
        CallOp::Execute,
        input,
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{command}: {answer}");
    answer
}

fn entity(doc: &Document, i: u64) -> Entity {
    let slot = doc.slot_of(Uuid::parse_str(&uid(i)).unwrap()).unwrap();
    doc.get(slot).unwrap().clone()
}

fn column_at(doc: &Document, i: u64) -> (f64, f64) {
    match entity(doc, i) {
        Entity::Column(c) => (c.column.at.x, c.column.at.y),
        other => panic!("{other:?}"),
    }
}

/// Wall `i`'s first point.
fn wall_start(doc: &Document, i: u64) -> (f64, f64) {
    match entity(doc, i) {
        Entity::Wall(w) => (w.wall.pts[0].x, w.wall.pts[0].y),
        other => panic!("{other:?}"),
    }
}

fn close(a: (f64, f64), x: f64, y: f64) -> bool {
    (a.0 - x).abs() < 1e-9 && (a.1 - y).abs() < 1e-9
}

#[test]
fn a_kept_column_distance_is_one_step_and_the_column_follows_its_wall() {
    let mut doc = drawing();
    run(
        &mut doc,
        "cad.arch.edit",
        json!({"uids": [uid(16)], "operation": {"kind": "distance", "side": 0, "distance": 0.5, "keep": true}}),
    );
    // The column's lower side 0.50 m from the south wall's inner face (0.10): its middle at 0.80.
    assert!(
        close(column_at(&doc, 16), 6.0, 0.8),
        "{:?}",
        column_at(&doc, 16)
    );
    let set = doc.constraints();
    assert_eq!(set.constraints.len(), 1);
    assert_eq!(set.parameters[0].expression, "0.5");
    // One undo takes the move and the bond.
    assert_eq!(doc.undo().as_deref(), Some("Mimari nesneyi düzenle"));
    assert!(doc.constraints().constraints.is_empty());
    assert!(close(column_at(&doc, 16), 6.0, 0.5));
    assert!(doc.redo().is_some());
    // The south wall 0.5 m out: the column follows.
    run(
        &mut doc,
        "cad.walls.edit",
        json!({"uids": [uid(1)], "operation": {"kind": "shift", "by": -0.5}}),
    );
    assert!(
        close(column_at(&doc, 16), 6.0, 0.3),
        "{:?}",
        column_at(&doc, 16)
    );
}

#[test]
fn a_tied_column_s_grid_moved_takes_the_column_and_its_bonded_wall_in_one_step() {
    let mut doc = drawing();
    run(
        &mut doc,
        "cad.constraints.edit",
        json!({"edit": {"kind": "add", "constraints": [{"rule": {
            "kind": "distance",
            "a": {"uid": uid(4), "element": {"kind": "wall", "edge": 0, "line": "rightFace"}},
            "b": {"uid": uid(17), "element": {"kind": "columnSide", "side": 1}},
            "parameter": ""
        }}]}}),
    );
    assert_eq!(doc.constraints().parameters[0].expression, "0.7");
    run(
        &mut doc,
        "cad.entities.transform",
        json!({"uids": [uid(14)], "transform": {"kind": "move", "dx": -0.3, "dy": 0.0}}),
    );
    assert!(
        close(column_at(&doc, 17), -1.3, -1.5),
        "{:?}",
        column_at(&doc, 17)
    );
    assert!(
        close(wall_start(&doc, 4), -0.3, 5.0),
        "{:?}",
        wall_start(&doc, 4)
    );
    assert_eq!(doc.undo().as_deref(), Some("Taşı"));
    assert!(close(column_at(&doc, 17), -1.0, -1.5));
    assert!(close(wall_start(&doc, 4), 0.0, 5.0));
}
