//! docs/adr/0780 §3's budget in release: a shift's preview (`cad.walls.edit`'s plan) on a plan of 200 walls and 50
//! architectural bonds, the bonds following, refused with its bonds in the way, and written.
//!
//! The plan: 50 floors' walls (`H`, 20 m, 3 m apart) with their two ends' walls (`V`) and a partition in each room
//! (`P`); 49 distance bonds chain each `H`'s upper face to the next one's lower face (a corridor kept as each wall
//! moves), one bond holds the first partition from its room's west wall. Moving the lowest `H` drags all 49. The same
//! at ADR 0770 §8's size: 100 floors, 400 walls, 100 bonds.
//!
//! ```text
//! cargo test --release -p kentos-native-application --test all arch_bond_timing -- --ignored --nocapture
//! ```

use std::time::Instant;

use kentos_commands::*;
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, constraints_edit, walls_create, walls_edit};
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../../../fixtures/commands/v1/cad.walls.edit.bonds.json");

fn done<T: std::fmt::Debug>(r: CommandResult<T>) -> T {
    match r {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

fn line(uid: &str, line: &str) -> Value {
    json!({"uid": uid, "element": {"kind": "wall", "edge": 0, "line": line}})
}

/// The plan of the module's comment; `fixed`: the last floor's wall fixed too (the shift then refused).
fn drawing(floors: usize, total: usize, fixed: bool) -> (Document, Vec<String>) {
    let file: Value = serde_json::from_str(CASES).unwrap();
    let mut setup = file["setup"].clone();
    setup["entities"] = json!([]);
    setup["uids"] = json!([]);
    setup["constraints"] = json!({});
    let snapshot: DocumentSnapshotV2 = serde_json::from_value(setup).unwrap();
    let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
    let mut walls = Vec::new();
    for i in 0..floors {
        let y = 3.0 * i as f64;
        walls.push(json!({"pts":[{"x":0,"y":y},{"x":20,"y":y}],"width":0.2}));
    }
    for i in 0..floors - 1 {
        let y = 3.0 * i as f64;
        walls.push(json!({"pts":[{"x":0,"y":y},{"x":0,"y":y + 3.0}],"width":0.2}));
        walls.push(json!({"pts":[{"x":20,"y":y},{"x":20,"y":y + 3.0}],"width":0.2}));
    }
    let mut k = 0;
    while walls.len() < total {
        // Neighbouring rooms' partitions stand apart, not end to end through the floor between them.
        let room = k % (floors - 1);
        let y = 3.0 * room as f64;
        let x = 7.0 + 6.0 * (room % 2) as f64 + 3.0 * (k / (floors - 1)) as f64;
        walls.push(json!({"pts":[{"x":x,"y":y},{"x":x,"y":y + 3.0}],"width":0.1}));
        k += 1;
    }
    let made = done(walls_create::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(json!({"layerId": "duvar", "walls": walls})).unwrap(),
    ));
    let uids = made.uids;
    let mut bonds: Vec<Value> = (0..floors - 1)
        .map(|i| {
            json!({"rule": {"kind": "distance", "a": line(&uids[i], "leftFace"),
                            "b": line(&uids[i + 1], "rightFace"), "parameter": ""}})
        })
        .collect();
    // The first partition from its room's west wall (the partitions come after the floors and their ends).
    let first_partition = &uids[floors + 2 * (floors - 1)];
    bonds.push(json!({"rule": {"kind": "distance", "a": line(&uids[floors], "leftFace"),
                               "b": line(first_partition, "rightFace"), "parameter": ""}}));
    if fixed {
        bonds.push(json!({"rule": {"kind": "fix", "a": line(&uids[floors - 1], "axis")}}));
    }
    done(constraints_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(json!({"edit": {"kind": "add", "constraints": bonds}})).unwrap(),
    ));
    (doc, uids)
}

fn median(n: usize, mut f: impl FnMut()) -> f64 {
    let mut times: Vec<f64> = (0..n)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    times.sort_by(f64::total_cmp);
    times[n / 2]
}

#[test]
#[ignore]
fn arch_bond_timing() {
    measure(50, 200);
    measure(100, 400);
}

fn measure(floors: usize, total: usize) {
    let shift = |uid: &str, by: f64| -> WallsEdit {
        serde_json::from_value(json!({"uids": [uid], "operation": {"kind": "shift", "by": by}})).unwrap()
    };
    let (mut doc, uids) = drawing(floors, total, false);
    assert_eq!(doc.entities().count(), total);
    assert_eq!(doc.constraints().constraints.len(), floors);
    let input = shift(&uids[0], -0.2);
    let followed = match walls_edit::plan(&ExecutionContext::new(&mut doc), &input) {
        CommandResult::Completed { output, .. } => output.effects.map_or(0, |e| e.bonded.len()),
        other => panic!("{other:?}"),
    };
    let plan = median(21, || {
        std::hint::black_box(walls_edit::plan(&ExecutionContext::new(&mut doc), &input));
    });
    let write = median(11, || {
        let mut copy = doc.clone();
        std::hint::black_box(walls_edit::execute(
            &mut ExecutionContext::new(&mut copy),
            input.clone(),
        ));
    });
    // A partition's shift: one bond near it, none following.
    let partition = shift(&uids[floors + 2 * (floors - 1) + 10], 0.3);
    assert!(
        matches!(
            walls_edit::plan(&ExecutionContext::new(&mut doc), &partition),
            CommandResult::Completed { .. }
        ),
        "{:?}",
        walls_edit::plan(&ExecutionContext::new(&mut doc), &partition)
    );
    let near = median(21, || {
        std::hint::black_box(walls_edit::plan(&ExecutionContext::new(&mut doc), &partition));
    });
    let (mut fixed, fixed_uids) = drawing(floors, total, true);
    let refused_input = shift(&fixed_uids[0], -0.2);
    let bonds = match walls_edit::plan(&ExecutionContext::new(&mut fixed), &refused_input) {
        CommandResult::Failed { error } => {
            assert_eq!(error.code, "constraint_conflict");
            error.bonds.len()
        }
        other => panic!("{other:?}"),
    };
    let refused = median(11, || {
        std::hint::black_box(walls_edit::plan(&ExecutionContext::new(&mut fixed), &refused_input));
    });
    println!(
        "{total} duvar, {floors} bağ: kaydırmanın önizlemesi {plan:.2} ms ({followed} duvar bağla izler), yazma {write:.2} ms; \
         tek bağlı bölmenin önizlemesi {near:.2} ms; reddedilen önizleme {refused:.2} ms ({bonds} bağ yolda)"
    );
}
