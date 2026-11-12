//! A slab bound to its walls (docs/adr/0771 §4) moved and placed (docs/adr/8050 §2) on the shared cases' drawing
//! (scripts/fixtures/bound_slab_cases.py): its seed goes with it, as a space's does, so the outline found again round it
//! is the moved room's. Moved with its walls it stays whole; a module instance's slab binds the instance's walls, not
//! the source's, and grows when the instance's wall moves out.

use kentos_commands::{CallOp, DocumentSnapshotV1, Entity, SlabFields};
use kentos_domain::{Document, Slot, Uuid};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

/// The drawing: an 8 × 5 m room of 20 cm walls on zemin (slots 1–4), a 15 cm wall across it at x = 5 (slot 5).
fn drawing() -> Document {
    let f: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.slabs.bound.json"
    ))
    .unwrap();
    // The cases' drawing is of the file's first form, its objects' ids derived.
    let snapshot = DocumentSnapshotV1::from_json(&f["setup"].to_string()).unwrap();
    Document::from_snapshot(snapshot).unwrap()
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

fn uid(doc: &Document, slot: u32) -> String {
    doc.uid(Slot(slot)).unwrap().to_string()
}

fn slab(doc: &Document, uid: &str) -> SlabFields {
    let slot = doc.slot_of(Uuid::parse_str(uid).unwrap()).unwrap();
    match doc.get(slot) {
        Some(Entity::Slab(s)) => s.slab.clone(),
        other => panic!("{other:?}"),
    }
}

/// The outline's box: min x, min y, max x, max y.
fn outline(s: &SlabFields) -> [f64; 4] {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in s.parts.iter().flat_map(|p| &p.outer.pts) {
        b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
    }
    b
}

fn near(a: [f64; 4], b: [f64; 4]) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-9)
}

/// The room's slab bound to the right room's inner faces (seed (7, 2)): 5.075 to 7.9 across, 0.1 to 4.9 up.
fn with_slab(doc: &mut Document) -> String {
    let made = run(
        doc,
        "cad.slabs.create",
        json!({"layerId": "doseme", "slabs": [{"seed": {"x": 7.0, "y": 2.0}, "boundary": "room", "bound": true,
            "arch": {"level": "zemin"}}]}),
    );
    let slot = made["output"]["ids"][0].as_u64().unwrap() as u32;
    let u = uid(doc, slot);
    assert!(
        near(outline(&slab(doc, &u)), [5.075, 0.1, 7.9, 4.9]),
        "{:?}",
        outline(&slab(doc, &u))
    );
    u
}

#[test]
fn a_bound_slab_moved_with_its_walls_takes_its_seed_and_stays_whole() {
    let mut doc = drawing();
    let s = with_slab(&mut doc);
    let mut uids: Vec<String> = (1..=5).map(|i| uid(&doc, i)).collect();
    uids.push(s.clone());
    run(
        &mut doc,
        "cad.entities.transform",
        json!({"uids": uids, "transform": {"kind": "move", "dx": 20.0, "dy": 0.0}}),
    );
    let moved = slab(&doc, &s);
    let b = moved.bound.as_ref().unwrap();
    assert_eq!(b.seed.map(|p| (p.x, p.y)), Some((27.0, 2.0)));
    assert_eq!(b.broken, None, "{b:?}");
    assert!(
        near(outline(&moved), [25.075, 0.1, 27.9, 4.9]),
        "{:?}",
        outline(&moved)
    );
}

#[test]
fn a_module_instance_s_bound_slab_binds_its_own_walls() {
    let mut doc = drawing();
    let s = with_slab(&mut doc);
    let mut uids: Vec<String> = (1..=5).map(|i| uid(&doc, i)).collect();
    uids.push(s.clone());
    run(
        &mut doc,
        "cad.modules.edit",
        json!({"operation": {"kind": "define", "name": "Oda", "uids": uids, "base": {"x": 0.0, "y": 0.0}}}),
    );
    let placed = run(
        &mut doc,
        "cad.modules.edit",
        json!({"operation": {"kind": "place", "module": "Oda", "at": {"x": 20.0, "y": 0.0}, "level": "zemin"}}),
    );
    let created: Vec<String> = placed["output"]["created"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    // In the definition's order: the five walls, then the slab.
    let (walls, copy) = (&created[..5], &created[5]);
    let c = slab(&doc, copy);
    let b = c.bound.as_ref().unwrap();
    assert_eq!(b.seed.map(|p| (p.x, p.y)), Some((27.0, 2.0)));
    assert_eq!(b.broken, None);
    assert!(
        b.uids.iter().all(|u| walls.contains(u)),
        "{:?} {walls:?}",
        b.uids
    );
    assert!(
        near(outline(&c), [25.075, 0.1, 27.9, 4.9]),
        "{:?}",
        outline(&c)
    );
    // Placed as the source has it: nothing to take from it.
    let same = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.modules.edit",
        CallOp::Execute,
        json!({"operation": {"kind": "update", "module": "Oda", "from": "oda-2"}}),
    )
    .unwrap();
    assert_eq!(same["error"]["code"], "nothing_to_set", "{same}");
    // The instance's right wall (the source's second) 1 m out: its slab grows, the source's stays.
    run(
        &mut doc,
        "cad.walls.edit",
        json!({"uids": [walls[1]], "operation": {"kind": "shift", "by": -1.0}}),
    );
    assert!(
        near(outline(&slab(&doc, copy)), [25.075, 0.1, 28.9, 4.9]),
        "{:?}",
        outline(&slab(&doc, copy))
    );
    assert!(near(outline(&slab(&doc, &s)), [5.075, 0.1, 7.9, 4.9]));
}

/// The open plan of the module cases (docs/adr/8050 §4; ADR 0679's): a 10 × 8 m room of 20 cm walls (slots 1–4), a
/// room separator at x = 5 (5), the living room (6) and the kitchen (7) bounded by them.
fn open_plan() -> Document {
    let f: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.modules.edit.kinds.json"
    ))
    .unwrap();
    let setup = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|c| c.get("setup"))
        .expect("the open plan's case")
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).unwrap();
    Document::from_snapshot_v2(snapshot).unwrap()
}

/// A space's net area: its outer rings' less their holes'.
fn area(doc: &Document, uid: &str) -> f64 {
    let ring = |pts: &[kentos_commands::Vec2]| {
        let n = pts.len();
        (0..n)
            .map(|i| {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                a.x * b.y - b.x * a.y
            })
            .sum::<f64>()
            .abs()
            / 2.0
    };
    let slot = doc.slot_of(Uuid::parse_str(uid).unwrap()).unwrap();
    match doc.get(slot) {
        Some(Entity::Space(s)) => s
            .space
            .parts
            .iter()
            .map(|p| ring(&p.outer.pts) - p.holes.iter().map(|h| ring(&h.pts)).sum::<f64>())
            .sum(),
        other => panic!("{other:?}"),
    }
}

/// docs/adr/8050 §4: the open plan placed mirrored on 1. kat keeps its two rooms of 38.22 m² each, bounded by its own
/// walls and separator; the source's separator moved 1 m east, the instance's moves 1 m west and its rooms take
/// 46.02 and 30.42 m², as the source's do.
#[test]
fn a_module_s_room_separator_bounds_its_instance_s_rooms() {
    let mut doc = open_plan();
    let uids: Vec<String> = (1..=7).map(|i| uid(&doc, i)).collect();
    run(
        &mut doc,
        "cad.modules.edit",
        json!({"operation": {"kind": "define", "name": "Açık plan", "uids": uids, "base": {"x": 0.0, "y": 0.0}}}),
    );
    let placed = run(
        &mut doc,
        "cad.modules.edit",
        json!({"operation": {"kind": "place", "module": "Açık plan", "at": {"x": 20.0, "y": 0.0}, "mirror": true,
            "level": "kat1"}}),
    );
    let created: Vec<String> = placed["output"]["created"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    let (living, kitchen) = (&created[5], &created[6]);
    for u in [living, kitchen] {
        assert!((area(&doc, u) - 38.22).abs() < 1e-9, "{}", area(&doc, u));
    }
    let source = uid(&doc, 5);
    run(
        &mut doc,
        "cad.entities.transform",
        json!({"uids": [source], "transform": {"kind": "move", "dx": 1.0, "dy": 0.0}}),
    );
    let (s_living, s_kitchen) = (uid(&doc, 6), uid(&doc, 7));
    assert!(
        (area(&doc, &s_living) - 46.02).abs() < 1e-9,
        "{}",
        area(&doc, &s_living)
    );
    assert!((area(&doc, &s_kitchen) - 30.42).abs() < 1e-9);
    run(
        &mut doc,
        "cad.modules.edit",
        json!({"operation": {"kind": "update", "module": "Açık plan", "from": "acik-plan-1"}}),
    );
    assert!(
        (area(&doc, living) - 46.02).abs() < 1e-9,
        "{}",
        area(&doc, living)
    );
    assert!(
        (area(&doc, kitchen) - 30.42).abs() < 1e-9,
        "{}",
        area(&doc, kitchen)
    );
}
