//! docs/adr/0780 §3's budget in release: a shift's preview (`cad.walls.edit`'s plan) on a plan of 200 walls and 50
//! architectural bonds, the bonds following, refused with its bonds in the way, and written. docs/adr/0790's: a grip's
//! drag (`cad.entities.edit`'s plan, the select tool's preview on every move) taken and refused, and a refusal no bond
//! alone frees (the lowest floor fixed twice and shifted: the pairs tried).
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
use kentos_native_application::{ExecutionContext, constraints_edit, edit, walls_create, walls_edit};
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

/// The plan of the module's comment; `fixed`: the last floor's wall fixed too (the shift then refused); `twice`: the
/// lowest floor's wall fixed by its axis and its left face (its shift frees with neither alone).
fn drawing(floors: usize, total: usize, fixed: bool) -> (Document, Vec<String>) {
    drawing_with(floors, total, fixed, false)
}

fn drawing_with(floors: usize, total: usize, fixed: bool, twice: bool) -> (Document, Vec<String>) {
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
    if twice {
        bonds.push(json!({"rule": {"kind": "fix", "a": line(&uids[0], "axis")}}));
        bonds.push(json!({"rule": {"kind": "fix", "a": line(&uids[0], "leftFace")}}));
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
    late(50, 200);
    late(100, 400);
}

/// docs/adr/0790's open worst case (closed by docs/adr/0797's measurement): a conflict that comes while the shifts are
/// written, every try a whole solve. The top room's end walls stand on a locked layer (the solve does not hold them: Kaydır
/// drags their ends when the top wall moves); besides the chain, nine bonds hold the top wall straight from the moved
/// one (its faces and axis to the top's), so the shift is refused whatever single bond, pair or triple goes: every bond
/// in the way alone, then the 48 sets, none freeing.
fn late(floors: usize, total: usize) {
    let (doc, uids) = drawing(floors, total, false);
    let top = &uids[floors - 1];
    let lines = ["leftFace", "rightFace", "axis"];
    let extra: Vec<Value> = lines
        .iter()
        .flat_map(|a| lines.iter().map(move |b| (a, b)))
        .map(|(a, b)| {
            json!({"rule": {"kind": "distance", "a": line(&uids[0], a), "b": line(top, b), "parameter": ""}})
        })
        .collect();
    let mut doc = doc;
    done(constraints_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(json!({"edit": {"kind": "add", "constraints": extra}})).unwrap(),
    ));
    // The top room's two end walls onto the locked layer: the bonds hold the top wall, its shift drags their ends.
    let mut snapshot = serde_json::to_value(doc.to_snapshot_v2()).unwrap();
    for end in [&uids[floors + 2 * (floors - 2)], &uids[floors + 2 * (floors - 2) + 1]] {
        let at = snapshot["uids"].as_array().unwrap().iter().position(|u| u == end).unwrap();
        snapshot["entities"][at]["layerId"] = json!("kilitli");
    }
    let mut doc = Document::from_snapshot_v2(serde_json::from_value(snapshot).unwrap()).unwrap();
    let input: WallsEdit =
        serde_json::from_value(json!({"uids": [uids[0]], "operation": {"kind": "shift", "by": -0.2}})).unwrap();
    let bonds = match walls_edit::plan(&ExecutionContext::new(&mut doc), &input) {
        CommandResult::Failed { error } => {
            assert_eq!(error.code, "constraint_conflict", "{error:?}");
            assert!(error.bonds.iter().all(|b| !b.frees && b.frees_with.is_empty()), "{:?}", error.bonds);
            error.bonds.len()
        }
        other => panic!("{other:?}"),
    };
    assert!(bonds >= 10, "{bonds} bonds in the way");
    let worst = median(5, || {
        std::hint::black_box(walls_edit::plan(&ExecutionContext::new(&mut doc), &input));
    });
    println!("{total} duvar: geç çelişkide en kötü ret {worst:.2} ms ({bonds} bağ yolda, tek tek ve 48 küme denendi)");
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
    // A grip's drag: the lowest wall's two ends moved together, as Esnet writes it (docs/adr/0790 §1).
    let grip = |doc: &Document, uid: &str| -> EntitiesEdit {
        let Some(Entity::Wall(w)) = doc.slot_of(uid.parse().unwrap()).and_then(|s| doc.get(s)) else {
            panic!("no wall {uid}")
        };
        let mut fields = w.wall.clone();
        for p in &mut fields.pts {
            p.y -= 0.2;
        }
        EntitiesEdit {
            operation: EditOperation::Grip,
            changes: vec![EntityEdit::Update {
                uid: uid.to_owned(),
                geometry: EntityGeometry::Wall(fields),
            }],
            expected_revision: None,
        }
    };
    let dragged = grip(&doc, &uids[0]);
    assert!(matches!(edit::plan_in(&doc, &dragged), CommandResult::Completed { .. }));
    let drag = median(21, || {
        std::hint::black_box(edit::plan_in(&doc, &dragged));
    });
    let dragged_fixed = grip(&fixed, &fixed_uids[0]);
    assert!(matches!(edit::plan_in(&fixed, &dragged_fixed), CommandResult::Failed { .. }));
    let drag_refused = median(21, || {
        std::hint::black_box(edit::plan_in(&fixed, &dragged_fixed));
    });
    // No bond alone frees it: the pairs are tried (docs/adr/0790 §2).
    let (twice, twice_uids) = drawing_with(floors, total, false, true);
    let twice_input = shift(&twice_uids[0], -0.2);
    let together = match walls_edit::plan(&ExecutionContext::new(&mut twice.clone()), &twice_input) {
        CommandResult::Failed { error } => {
            assert!(error.bonds.iter().all(|b| !b.frees));
            error.bonds.iter().filter(|b| !b.frees_with.is_empty()).count()
        }
        other => panic!("{other:?}"),
    };
    let mut twice = twice;
    let pairs = median(11, || {
        std::hint::black_box(walls_edit::plan(&ExecutionContext::new(&mut twice), &twice_input));
    });
    println!(
        "{total} duvar, {floors} bağ: kaydırmanın önizlemesi {plan:.2} ms ({followed} duvar bağla izler), yazma {write:.2} ms; \
         tek bağlı bölmenin önizlemesi {near:.2} ms; reddedilen önizleme {refused:.2} ms ({bonds} bağ yolda); \
         tutamaçla sürüklemenin önizlemesi {drag:.2} ms, reddedileni {drag_refused:.2} ms; \
         hiçbiri tek başına geçirmeyen ret {pairs:.2} ms ({together} bağ birlikte)"
    );
}
