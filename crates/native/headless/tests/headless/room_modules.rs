//! Room modules' acceptance (docs/adr/0775): the shared cases' bathroom placed four times on two levels, one mirrored;
//! one detached; the door made 90 cm in the first and the source updated: the three linked instances take it, the
//! detached one does not; every instance's references stay its own; one undo; the drawing written as KCAD and read
//! back keeps its modules; DXF says what it leaves out. And the timings of a larger update (ignored, release).

use std::collections::HashSet;
use std::time::Instant;

use kentos_commands::{CommandResult, DocumentSnapshotV2, Entity, ModulesEdit, OpeningsEdit};
use kentos_domain::{Document, Uuid};
use kentos_headless::export;
use kentos_native_application::{ExecutionContext, modules_edit, openings_edit};
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../../../fixtures/commands/v1/cad.modules.edit.json");

fn drawing() -> Document {
    let file: Value = serde_json::from_str(CASES).unwrap();
    let snapshot: DocumentSnapshotV2 = serde_json::from_value(file["setup"].clone()).unwrap();
    Document::from_snapshot_v2(snapshot).unwrap()
}

fn uid(n: u32) -> String {
    format!("0192f5a1-7750-7000-8000-{n:012x}")
}

fn edit(doc: &mut Document, op: Value) -> kentos_commands::ModulesEdited {
    let input: ModulesEdit = serde_json::from_value(json!({ "operation": op })).unwrap();
    match modules_edit::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

fn entity<'a>(doc: &'a Document, uid: &str) -> &'a Entity {
    doc.get(doc.slot_of(Uuid::parse_str(uid).unwrap()).unwrap())
        .unwrap()
}

fn door_width(doc: &Document, uid: &str) -> f64 {
    match entity(doc, uid) {
        Entity::Opening(o) => o.opening.width,
        other => panic!("{other:?}"),
    }
}

fn bathroom() -> Value {
    json!({"kind": "define", "name": "Banyo", "uids": (1..=9).map(uid).collect::<Vec<_>>(), "base": {"x": 0, "y": 0}})
}

/// The four instances: the first where it was drawn, one moved on the ground floor, one mirrored on the first, one on
/// the first floor detached.
fn four(doc: &mut Document) -> Vec<Vec<String>> {
    edit(doc, bathroom());
    let mut out = vec![(1..=9).map(uid).collect()];
    for op in [
        json!({"kind": "place", "module": "Banyo", "at": {"x": 10, "y": 0}, "level": "zemin"}),
        json!({"kind": "place", "module": "Banyo", "at": {"x": 20, "y": 0}, "mirror": true, "level": "kat1",
               "numbering": {"prefix": "D2-"}}),
        json!({"kind": "place", "module": "Banyo", "at": {"x": 10, "y": 10}, "rotation": std::f64::consts::FRAC_PI_2,
               "level": "kat1"}),
    ] {
        out.push(edit(doc, op).created);
    }
    out
}

#[test]
fn four_bathrooms_three_follow_the_source_one_detached_does_not() {
    let mut doc = drawing();
    let inst = four(&mut doc);
    edit(
        &mut doc,
        json!({"kind": "detach", "module": "Banyo", "instance": "banyo-4"}),
    );
    let door: OpeningsEdit = serde_json::from_value(
        json!({"uids": [uid(5)], "operation": {"kind": "set", "width": 0.9}}),
    )
    .unwrap();
    assert!(matches!(
        openings_edit::execute(&mut ExecutionContext::new(&mut doc), door),
        CommandResult::Completed { .. }
    ));
    let before_update = doc.to_snapshot_v2();
    let done = edit(
        &mut doc,
        json!({"kind": "update", "module": "Banyo", "from": "banyo-1"}),
    );
    assert_eq!(done.report.updated, ["banyo-1", "banyo-2", "banyo-3"]);
    for i in [0, 1, 2] {
        assert!(
            (door_width(&doc, &inst[i][4]) - 0.9).abs() < 1e-12,
            "instance {}",
            i + 1
        );
    }
    assert!(
        (door_width(&doc, &inst[3][4]) - 0.8).abs() < 1e-12,
        "the detached one keeps its door"
    );
    // Every instance's references are its own objects: the door on its wall, the space on its walls, the tag on its
    // space, the dimension on its wall; no id of one instance in another.
    let mut seen = HashSet::new();
    for objs in &inst {
        for o in objs {
            assert!(seen.insert(o.clone()), "an object in two instances");
        }
        let Entity::Opening(o) = entity(&doc, &objs[4]) else {
            panic!()
        };
        assert_eq!(o.opening.host, objs[0]);
        let Entity::Space(s) = entity(&doc, &objs[5]) else {
            panic!()
        };
        assert_eq!(s.space.sources.as_deref(), Some(&objs[0..4]));
        let Entity::Tag(t) = entity(&doc, &objs[7]) else {
            panic!()
        };
        assert_eq!(t.tag.of, objs[5]);
        let Entity::Dimension(d) = entity(&doc, &objs[8]) else {
            panic!()
        };
        let refs = d.refs.as_ref().unwrap();
        assert_eq!(refs.a.as_ref().unwrap().uid.to_text(), objs[0]);
    }
    let Entity::Tag(t) = entity(&doc, &inst[2][7]) else {
        panic!()
    };
    assert_eq!(
        t.tag.text, "D2-B1",
        "the mirrored instance's tag reads its numbered space"
    );
    // One undo gives the update back whole.
    doc.undo();
    let after_undo = doc.to_snapshot_v2();
    assert_eq!(after_undo.entities, before_update.entities);
    assert_eq!(after_undo.settings, before_update.settings);
    doc.redo();
    // KCAD: written and read back, the modules and the instances' objects are the same.
    let bytes = kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap();
    let again = Document::from_snapshot_v2(kentos_kcad::decode(&bytes).unwrap()).unwrap();
    assert_eq!(again.arch_modules(), doc.arch_modules());
    assert!((door_width(&again, &inst[1][4]) - 0.9).abs() < 1e-12);
    // DXF: the three instances' objects go as drawn; the module and its links are said to stay in KCAD.
    let input = export::dxf_input(&doc, &export::by_layer(&doc, doc.entities()));
    let note = input
        .notes
        .iter()
        .find(|n| n.what == "Oda modülü örneği")
        .expect("the DXF report says it");
    assert_eq!(note.count, 3);
    assert!(note.reason.contains("KCAD"));
}

#[test]
fn a_drawing_without_modules_writes_no_modules() {
    let doc = drawing();
    let input = export::dxf_input(&doc, &export::by_layer(&doc, doc.entities()));
    assert!(!input.notes.iter().any(|n| n.what == "Oda modülü örneği"));
}

#[test]
#[ignore]
fn room_modules_timing() {
    let mut doc = drawing();
    edit(&mut doc, bathroom());
    for k in 0..100 {
        let x = 10.0 + 4.0 * f64::from(k % 10);
        let y = 4.0 * f64::from(k / 10);
        edit(
            &mut doc,
            json!({"kind": "place", "module": "Banyo", "at": {"x": x, "y": y}, "level": "zemin"}),
        );
    }
    let door: OpeningsEdit = serde_json::from_value(
        json!({"uids": [uid(5)], "operation": {"kind": "set", "width": 0.9}}),
    )
    .unwrap();
    let _ = openings_edit::execute(&mut ExecutionContext::new(&mut doc), door);
    let input: ModulesEdit = serde_json::from_value(
        json!({"operation": {"kind": "update", "module": "Banyo", "from": "banyo-1"}}),
    )
    .unwrap();
    let t = Instant::now();
    let plan = modules_edit::plan_in(&doc, &input);
    let plan_ms = t.elapsed().as_secs_f64() * 1000.0;
    assert!(matches!(plan, CommandResult::Completed { .. }));
    let t = Instant::now();
    let done = modules_edit::execute(&mut ExecutionContext::new(&mut doc), input);
    let write_ms = t.elapsed().as_secs_f64() * 1000.0;
    let CommandResult::Completed { output, .. } = done else {
        panic!("{done:?}")
    };
    let place: ModulesEdit = serde_json::from_value(
        json!({"operation": {"kind": "place", "module": "Banyo", "at": {"x": 0, "y": 60}}}),
    )
    .unwrap();
    let t = Instant::now();
    let _ = modules_edit::plan_in(&doc, &place);
    let place_ms = t.elapsed().as_secs_f64() * 1000.0;
    println!(
        "101 örnek: güncellemenin planı {plan_ms:.1} ms, yazması {write_ms:.1} ms ({} örnek), yerleştirmenin planı {place_ms:.2} ms",
        output.report.updated.len()
    );
    assert_eq!(output.report.updated.len(), 101);
    assert!(plan_ms < 200.0 && write_ms < 500.0 && place_ms < 20.0);
}
