//! Copy formats without recipe graphs refuse automatic members until explicit, undoable detach.
use kentos_commands::{
    CallOp, DimensionSet, DimensionSetFace, DimensionSetId, DimensionSetOpenings, DimensionSetRow,
    DocumentSnapshotV1, Entity, EntityId, Vec2,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

const SET: DimensionSetId = DimensionSetId([91; 16]);
fn fixture() -> (Document, String, String) {
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Ölçü kopya",
        "settings":{"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":50},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],"activeLayer":"a",
        "entities":[{"kind":"wall","id":1,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":10,"y":0}],"width":0.2},
            {"kind":"line","id":2,"layerId":"a","attrs":{},"a":{"x":0,"y":5},"b":{"x":10,"y":5}}],
        "styles":{"items":[],"categories":[]}});
    let mut doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).unwrap())
            .unwrap();
    doc.add_dimension_set(DimensionSet {
        id: SET,
        wall: EntityId(doc.uid(Slot(1)).unwrap().into_bytes()),
        face: DimensionSetFace::Left,
        rows: vec![DimensionSetRow::Openings, DimensionSetRow::Overall],
        openings: DimensionSetOpenings::Ends,
        layer_id: "a".into(),
        at: Vec2 { x: 0., y: 1. },
        gap_mm: 6.,
        height: 0.125,
        look: Default::default(),
        row_places: vec![],
        members: vec![],
        suppressed: vec![],
        broken: None,
    })
    .unwrap();
    let member = doc.dimension_set(SET).unwrap().members[0].uid.to_string();
    let line = doc.uid(Slot(2)).unwrap().to_string();
    (doc, member, line)
}
fn run(doc: &mut Document, name: &str, op: CallOp, input: Value) -> Value {
    registry::run(&mut ExecutionContext::new(doc), name, op, input).unwrap()
}
fn inputs(member: &str, line: &str) -> Vec<(&'static str, Value)> {
    vec![
        ("cad.entities.copy", json!({"uids":[line,member]})),
        (
            "cad.entities.array",
            json!({"uids":[line,member],"layout":{"kind":"grid","rows":1,"cols":2,"dx":20.,"dy":0.}}),
        ),
        (
            "cad.entities.array",
            json!({"uids":[line,member],"associative":true,"layout":{"kind":"grid","rows":1,"cols":2,"dx":20.,"dy":0.}}),
        ),
        (
            "cad.blocks.define",
            json!({"name":"Ölçülü blok","uids":[line,member],"base":{"x":0.,"y":0.}}),
        ),
        (
            "cad.blocks.define",
            json!({"name":"Ölçülü blok","uids":[line,member],"base":{"x":0.,"y":0.},"replace":true,"layerId":"a"}),
        ),
        (
            "cad.entities.transform",
            json!({"uids":[line,member],"copy":true,"transform":{"kind":"move","dx":20.,"dy":0.}}),
        ),
    ]
}

#[test]
fn mixed_selections_refuse_before_copying_any_object_or_recipe() {
    let (mut doc, member, line) = fixture();
    let before = doc.to_snapshot_v2();
    let allocation = doc.next_slot();
    let revision = doc.revision();
    for (name, input) in inputs(&member, &line) {
        for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
            let result = run(&mut doc, name, op, input.clone());
            assert_eq!(result["status"], "failed", "{name} {op:?}: {result}");
            assert_eq!(
                result["error"]["code"], "dimension_set_owned",
                "{name} {op:?}: {result}"
            );
            assert!(
                result["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("bağımsız")
            );
            assert_eq!(result["error"]["path"], "uids[1]");
            assert_eq!(doc.to_snapshot_v2(), before, "{name} is atomic");
            assert_eq!(doc.next_slot(), allocation);
            assert_eq!(doc.revision(), revision);
        }
    }
}

#[test]
fn explicit_detach_restores_copy_array_and_block_behaviour_and_undo_owns_again() {
    for case in 0..6 {
        let (mut doc, member, line) = fixture();
        let input = inputs(&member, &line).remove(case);
        doc.remove_dimension_set(SET).unwrap();
        let detached = doc.to_snapshot_v2();
        let result = run(&mut doc, input.0, CallOp::Execute, input.1);
        assert_eq!(result["status"], "completed", "{}: {result}", input.0);
        assert!(doc.dimension_set(SET).is_none());
        if input.0 != "cad.entities.copy" {
            doc.undo();
            assert_eq!(doc.to_snapshot_v2(), detached);
        }
        doc.undo();
        assert!(doc.dimension_set(SET).is_some());
        assert!(
            doc.dimension_set_of_item(Uuid::parse_str(&member).unwrap())
                .is_some()
        );
    }
}

#[test]
fn unrelated_selection_copies_without_changing_existing_recipe() {
    let (mut doc, _, line) = fixture();
    let before = doc.dimension_set(SET).unwrap().clone();
    let answer = run(
        &mut doc,
        "cad.entities.copy",
        CallOp::Execute,
        json!({"uids":[line]}),
    );
    assert_eq!(answer["status"], "completed", "{answer}");
    assert_eq!(doc.dimension_set(SET).unwrap(), &before);
    let copied: Vec<Entity> = serde_json::from_value(answer["output"]["objects"].clone()).unwrap();
    assert_eq!(copied.len(), 1);
    assert!(matches!(copied[0], Entity::Line(_)));
}
