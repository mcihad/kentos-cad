//! Bonds to a grid's line (docs/adr/8049 §2) on the shared cases' drawing (scripts/fixtures/arch_bond_cases.py) with a
//! column where grids A and 1 cross: the wall bonded to grid A moves, the grid follows and the column on it with the
//! grid, all one undo step; a grid turned into an arc or deleted takes its bond with it.

use kentos_commands::{CallOp, DocumentSnapshotV2, Entity};
use kentos_domain::{Document, Uuid};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn uid(i: u64) -> String {
    format!("0192f5a1-7100-7000-8000-{i:012x}")
}

/// The drawing; its column 17 (40 × 40 cm) is tied to grids A (13) and 1 (14) where they cross, at (−1, −1.5).
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

/// Grid `i`'s line's height (a horizontal grid).
fn grid_y(doc: &Document, i: u64) -> f64 {
    match entity(doc, i) {
        Entity::Grid(g) => {
            assert!((g.grid.pts[0].y - g.grid.pts[1].y).abs() < 1e-12);
            g.grid.pts[0].y
        }
        other => panic!("{other:?}"),
    }
}

/// The south wall's axis height.
fn south_y(doc: &Document) -> f64 {
    match entity(doc, 1) {
        Entity::Wall(w) => w.wall.pts[0].y,
        other => panic!("{other:?}"),
    }
}

fn column_at(doc: &Document) -> (f64, f64) {
    match entity(doc, 17) {
        Entity::Column(c) => (c.column.at.x, c.column.at.y),
        other => panic!("{other:?}"),
    }
}

/// The south wall's axis kept 1.50 m from grid A.
fn bonded() -> Document {
    let mut doc = drawing();
    run(
        &mut doc,
        "cad.constraints.edit",
        json!({"edit": {"kind": "add", "constraints": [{"rule": {
            "kind": "distance",
            "a": {"uid": uid(13), "element": {"kind": "grid"}},
            "b": {"uid": uid(1), "element": {"kind": "wall", "edge": 0, "line": "axis"}},
            "parameter": ""
        }}]}}),
    );
    let set = doc.constraints();
    assert_eq!(set.parameters[0].expression, "1.5");
    assert!(matches!(
        &set.constraints[0].rule,
        kentos_commands::ConstraintRule::Distance { left: true, .. }
    ));
    doc
}

#[test]
fn the_south_wall_moved_takes_its_grid_and_the_column_on_it_in_one_step() {
    let mut doc = bonded();
    assert_eq!(column_at(&doc), (-1.0, -1.5));
    run(
        &mut doc,
        "cad.walls.edit",
        json!({"uids": [uid(1)], "operation": {"kind": "shift", "by": -0.5}}),
    );
    assert!((south_y(&doc) + 0.5).abs() < 1e-9, "{}", south_y(&doc));
    assert!(
        (grid_y(&doc, 13) + 2.0).abs() < 1e-9,
        "{}",
        grid_y(&doc, 13)
    );
    let (x, y) = column_at(&doc);
    assert!((x + 1.0).abs() < 1e-9 && (y + 2.0).abs() < 1e-9, "{x}, {y}");
    // One undo takes the wall, the grid and the column back.
    assert_eq!(doc.undo().as_deref(), Some("Duvarı düzenle"));
    assert!(south_y(&doc).abs() < 1e-12);
    assert!((grid_y(&doc, 13) + 1.5).abs() < 1e-12);
    assert_eq!(column_at(&doc), (-1.0, -1.5));
}

#[test]
fn the_grid_moved_takes_the_wall_and_its_column() {
    let mut doc = bonded();
    run(
        &mut doc,
        "cad.entities.transform",
        json!({"uids": [uid(13)], "transform": {"kind": "move", "dx": 0.0, "dy": -0.5}}),
    );
    assert!((grid_y(&doc, 13) + 2.0).abs() < 1e-9);
    assert!((south_y(&doc) + 0.5).abs() < 1e-9, "{}", south_y(&doc));
    let (_, y) = column_at(&doc);
    assert!((y + 2.0).abs() < 1e-9, "{y}");
    // The bond still measures its 1.50 m.
    let listed = run(
        &mut doc,
        "cad.constraints.query",
        json!({"architectural": true}),
    );
    let c = &listed["output"]["constraints"][0];
    assert_eq!(c["holds"], true, "{listed}");
    assert!((c["measured"].as_f64().unwrap() - 1.5).abs() < 1e-9, "{c}");
}

#[test]
fn a_grid_deleted_or_bent_into_an_arc_takes_its_bond() {
    let mut doc = bonded();
    run(&mut doc, "cad.entities.delete", json!({"uids": [uid(13)]}));
    assert!(doc.constraints().constraints.is_empty());
    assert!(doc.undo().is_some());
    assert_eq!(doc.constraints().constraints.len(), 1);
    // Bent into an arc: the grid is no longer a line, its bond goes in the same step.
    let mut g = entity(&doc, 13);
    let Entity::Grid(grid) = &mut g else {
        unreachable!()
    };
    grid.grid.bulges = Some(vec![0.2, 0.0]);
    let slot = doc.slot_of(Uuid::parse_str(&uid(13)).unwrap()).unwrap();
    assert!(doc.update(slot, g));
    assert!(doc.constraints().constraints.is_empty());
}
