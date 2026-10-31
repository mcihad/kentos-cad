//! Modules of several storeys and a level's loose objects repeated (docs/adr/0796): the shared cases' duplex (two
//! walls, a door and a room on the ground floor, two walls, a door, a room and a bed on the first) defined as one
//! module of two storeys, placed in the basement (its upper storey on the ground floor, the levels taken by their
//! elevations, not their list's order) and refused on the top level; the ground floor repeated on the first with its
//! loose objects (the hall's walls, door, room, tag, dimensions and fixture follow their copies; the room's number takes
//! the first floor's prefix; a column, a room bound to another level's wall and its tag are left and said); one undo
//! step takes the repeat back whole; the drawing written as KCAD (schema 152) and read back keeps the storeys. And the
//! timings of a ten-storey block of duplexes with loose halls (ignored, release).

use std::time::Instant;

use kentos_commands::{
    CommandResult, DocumentSnapshotV2, Entity, ModuleLeftWhy, ModulesEdit, ModulesEdited,
};
use kentos_domain::{Document, Uuid};
use kentos_native_application::{ExecutionContext, modules_edit};
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../../../fixtures/commands/v1/cad.modules.edit.storeys.json");

fn drawing() -> Document {
    let file: Value = serde_json::from_str(CASES).unwrap();
    let snapshot: DocumentSnapshotV2 = serde_json::from_value(file["setup"].clone()).unwrap();
    Document::from_snapshot_v2(snapshot).unwrap()
}

fn uid(n: u32) -> String {
    format!("0192f5a1-7960-7000-8000-{n:012x}")
}

fn edit(doc: &mut Document, op: Value) -> CommandResult<ModulesEdited> {
    let input: ModulesEdit = serde_json::from_value(json!({ "operation": op })).unwrap();
    modules_edit::execute(&mut ExecutionContext::new(doc), input)
}

fn done(doc: &mut Document, op: Value) -> ModulesEdited {
    match edit(doc, op) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

fn entity<'a>(doc: &'a Document, uid: &str) -> &'a Entity {
    doc.get(doc.slot_of(Uuid::parse_str(uid).unwrap()).unwrap())
        .unwrap()
}

fn level(e: &Entity) -> Option<&str> {
    e.own_level()
}

fn define(doc: &mut Document) {
    done(
        doc,
        json!({"kind": "define", "name": "Dubleks", "uids": (1..=11).map(uid).collect::<Vec<_>>(),
               "base": {"x": 0, "y": 0}}),
    );
}

#[test]
fn a_duplex_spans_two_storeys_and_a_floor_repeats_with_its_loose_objects() {
    let mut doc = drawing();
    define(&mut doc);
    let m = doc.arch_modules()[0].clone();
    assert_eq!(m.storeys(), 2);
    assert_eq!(m.instances[0].level.as_deref(), Some("zemin"));
    // In the basement: the upper storey lands on the ground floor (the levels by elevation).
    let placed = done(
        &mut doc,
        json!({"kind": "place", "module": "Dubleks", "at": {"x": 20, "y": 10}, "level": "bodrum"}),
    );
    let inst = placed.module.unwrap().instances[1].clone();
    let of = |key: u32| inst.members.iter().find(|o| o.key == uid(key)).unwrap().uid.clone();
    assert_eq!(level(entity(&doc, &of(1))), Some("bodrum"));
    assert_eq!(level(entity(&doc, &of(3))), Some("zemin"));
    assert_eq!(level(entity(&doc, &of(9))), Some("zemin"));
    // On the top level the upper storey finds no level: refused, nothing written.
    let revision = doc.revision().to_string();
    match edit(
        &mut doc,
        json!({"kind": "place", "module": "Dubleks", "at": {"x": 40, "y": 10}, "level": "kat2"}),
    ) {
        CommandResult::Failed { error } => assert_eq!(error.code, "module_storey_missing"),
        other => panic!("{other:?}"),
    }
    assert_eq!(doc.revision().to_string(), revision);
    // The ground floor on the first with its loose objects: one step.
    let count = doc.entities().count();
    let repeated = done(
        &mut doc,
        json!({"kind": "repeat", "from": "zemin", "loose": true, "to": [{"level": "kat1"}]}),
    );
    assert_eq!(repeated.placed.len(), 1);
    let copy = |n: u32| {
        repeated
            .copied
            .iter()
            .find(|c| c.source == uid(n))
            .unwrap()
            .uid
            .clone()
    };
    assert_eq!(repeated.copied.len(), 8);
    // The basement duplex's upper storey stands on the ground floor: its objects are an instance's not repeated (based
    // in the basement), said apart from the hall's column and the room bound upstairs with its tag.
    let upper: Vec<String> = inst
        .members
        .iter()
        .filter(|o| [3, 4, 6, 8, 9, 10, 11].iter().any(|&k| o.key == uid(k)))
        .map(|o| o.uid.clone())
        .collect();
    let (module, left): (Vec<_>, Vec<_>) = repeated
        .report
        .left
        .iter()
        .map(|l| (l.uid.clone(), l.why))
        .partition(|(_, why)| *why == ModuleLeftWhy::Module);
    let mut module: Vec<String> = module.into_iter().map(|(u, _)| u).collect();
    let mut upper = upper;
    module.sort();
    upper.sort();
    assert_eq!(module, upper);
    assert_eq!(
        left,
        [
            (uid(19), ModuleLeftWhy::Kind),
            (uid(22), ModuleLeftWhy::Outside),
            (uid(23), ModuleLeftWhy::Outside)
        ]
    );
    assert_eq!(doc.entities().count(), count + 11 + 8);
    // The hall's door on its wall's copy, its room bound to the copies and numbered for the first floor, its tag on it.
    match entity(&doc, &copy(14)) {
        Entity::Opening(o) => assert_eq!(o.opening.host, copy(12)),
        other => panic!("{other:?}"),
    }
    match entity(&doc, &copy(15)) {
        Entity::Space(s) => {
            assert_eq!(s.space.number.as_deref(), Some("1-101"));
            assert_eq!(s.space.arch.level.as_deref(), Some("kat1"));
            let sources: Vec<String> = s
                .space
                .sources
                .clone()
                .unwrap_or_default()
                .iter()
                .map(ToString::to_string)
                .collect();
            assert_eq!(sources, [copy(12), copy(13)]);
        }
        other => panic!("{other:?}"),
    }
    match entity(&doc, &copy(16)) {
        Entity::Tag(t) => assert_eq!(t.tag.of, copy(15)),
        other => panic!("{other:?}"),
    }
    // The dimension from the hall's wall to the duplex's: its copy on the repeated duplex's wall.
    let new_duplex = doc.arch_modules()[0].instances.last().unwrap().clone();
    let wall1 = new_duplex.members.iter().find(|o| o.key == uid(1)).unwrap().uid.clone();
    match entity(&doc, &copy(20)) {
        Entity::Dimension(d) => {
            let refs = d.refs.as_ref().unwrap();
            assert_eq!(refs.b.as_ref().unwrap().uid.to_string(), wall1);
        }
        other => panic!("{other:?}"),
    }
    // The original hall stays where it was, on the ground floor.
    assert_eq!(level(entity(&doc, &uid(12))), Some("zemin"));
    // One undo step takes the repeat back whole.
    doc.undo();
    assert_eq!(doc.entities().count(), count);
    assert_eq!(doc.arch_modules()[0].instances.len(), 2);
    doc.redo();
    assert_eq!(doc.entities().count(), count + 19);
    // KCAD 152: written and read back, the storeys and the instances are the same.
    let bytes = kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap();
    let again = Document::from_snapshot_v2(kentos_kcad::decode(&bytes).unwrap()).unwrap();
    assert_eq!(again.arch_modules(), doc.arch_modules());
    assert_eq!(again.arch_modules()[0].storeys(), 2);
}

#[test]
#[ignore]
fn storey_modules_timing() {
    // Ten duplexes with their halls on the ground floor, each 30 m apart, a block of ten levels.
    let mut snapshot: Value = serde_json::from_str::<Value>(CASES).unwrap()["setup"].clone();
    snapshot["settings"]["arch"]["levels"] = json!((0..10)
        .map(|k| json!({"id": format!("k{k}"), "name": format!("{k}. kat"), "elevation": 3.0 * f64::from(k),
                        "height": 3.0}))
        .collect::<Vec<_>>());
    snapshot["settings"]["arch"]["spaceNumberPrefixes"] = json!({"k0": "Z-", "k1": "1-"});
    let ents = snapshot["entities"].as_array().unwrap().clone();
    let mut all = Vec::new();
    let mut uids = Vec::new();
    let mut id = 1u32;
    for n in 0..10u32 {
        let mut map = std::collections::HashMap::new();
        for e in &ents {
            map.insert(uid(e["id"].as_u64().unwrap() as u32), format!("0192f5a1-7961-7000-8000-{:012x}", id));
            uids.push(format!("0192f5a1-7961-7000-8000-{:012x}", id));
            id += 1;
        }
        for e in &ents {
            let mut text = e.to_string();
            for (from, to) in &map {
                text = text.replace(from, to);
            }
            let mut e: Value = serde_json::from_str(&text).unwrap();
            e["id"] = json!(all.len() + 1);
            let shift = 60.0 * f64::from(n);
            shift_x(&mut e, shift);
            for key in ["level"] {
                if let Some(a) = e.get_mut("arch").and_then(|a| a.get_mut(key)) {
                    *a = json!(match a.as_str().unwrap() {
                        "zemin" => "k0",
                        "kat1" => "k1",
                        "bodrum" | "kat2" => "k2",
                        other => other,
                    });
                }
            }
            all.push(e);
        }
    }
    snapshot["entities"] = Value::Array(all);
    snapshot["uids"] = json!(uids);
    let mut doc =
        Document::from_snapshot_v2(serde_json::from_value::<DocumentSnapshotV2>(snapshot).unwrap()).unwrap();
    for n in 0..10usize {
        let members: Vec<String> = (0..11).map(|k| uids[n * 27 + k].clone()).collect();
        done(
            &mut doc,
            json!({"kind": "define", "name": format!("Dubleks {n}"), "uids": members, "base": {"x": 0, "y": 0}}),
        );
    }
    let to: Vec<Value> = (2..9).step_by(2).map(|k| json!({"level": format!("k{k}")})).collect();
    let op = json!({"operation": {"kind": "repeat", "from": "k0", "loose": true, "to": to}});
    let input: ModulesEdit = serde_json::from_value(op).unwrap();
    let t = Instant::now();
    let plan = modules_edit::plan_in(&doc, &input);
    let planned = t.elapsed();
    let CommandResult::Completed { output, .. } = plan else {
        panic!("{plan:?}")
    };
    let t = Instant::now();
    let wrote = modules_edit::execute(&mut ExecutionContext::new(&mut doc), input);
    let written = t.elapsed();
    assert!(matches!(wrote, CommandResult::Completed { .. }));
    println!(
        "Katları tekrarla (10 dubleks, 4 kat, modülsüz {} kopya): plan {:.1} ms, yazma {:.1} ms",
        output.copied.len(),
        planned.as_secs_f64() * 1e3,
        written.as_secs_f64() * 1e3
    );
    assert!(planned.as_secs_f64() < 0.5 && written.as_secs_f64() < 1.0);
}

fn shift_x(v: &mut Value, dx: f64) {
    match v {
        Value::Object(o) => {
            if let (Some(Value::Number(x)), true) = (o.get("x").cloned(), o.contains_key("y")) {
                o.insert("x".into(), json!(x.as_f64().unwrap() + dx));
            }
            for (k, w) in o.iter_mut() {
                if k != "x" {
                    shift_x(w, dx);
                }
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|w| shift_x(w, dx)),
        _ => {}
    }
}
