//! Independent repeat-conversion geometry/identity expectations; one undo and refusing before writes.
use kentos_commands::{
    BlocksConvert, CommandResult, DocumentSnapshotV1, Entity, EntityId, SubPlace,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, blocks_convert};
use serde_json::{Value, json};

fn fixture() -> Value {
    let mut f: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    f["setup"]["entities"] = json!([
        {"kind":"line","id":1,"layerId":"a","attrs":{},"a":{"x":0.,"y":0.},"b":{"x":2.,"y":0.}},
        {"kind":"circle","id":2,"layerId":"a","attrs":{},"c":{"x":1.,"y":1.},"r":0.25},
        // Reverse the target line independently; a non-collinear circle fixes the collection's frame.
        {"kind":"line","id":3,"layerId":"a","attrs":{},"a":{"x":12.,"y":0.},"b":{"x":10.,"y":0.}},
        {"kind":"circle","id":4,"layerId":"a","attrs":{},"c":{"x":11.,"y":1.},"r":0.25}
    ]);
    f["setup"].clone()
}
fn open(raw: Value) -> Document {
    Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap()
}
fn uid(doc: &Document, id: u32) -> String {
    doc.uid(Slot(id)).unwrap().to_string()
}
fn input(doc: &Document) -> BlocksConvert {
    BlocksConvert {
        name: "Teknik tekrar".into(),
        source: vec![uid(doc, 2), uid(doc, 1)],
        groups: vec![
            vec![uid(doc, 2), uid(doc, 1)],
            vec![uid(doc, 4), uid(doc, 3)],
        ],
        rotations: Some(false),
        ..Default::default()
    }
}
fn add_dimension(doc: &mut Document, source: u32, place: Value, point: Value) -> Slot {
    let e: Entity =
        serde_json::from_value(json!({"kind":"dimension","id":0,"layerId":"a","attrs":{},
        "a":point,"b":{"x":15.,"y":1.},"offset":1.,"height":0.1,
        "refs":{"a":{"uid":uid(doc,source),"place":place}}}))
        .unwrap();
    doc.add(e).unwrap()
}
fn dimension(doc: &Document, s: Slot) -> &kentos_commands::DimensionEntity {
    let Some(Entity::Dimension(d)) = doc.get(s) else {
        panic!("dimension")
    };
    d
}
fn refused(doc: &mut Document, input: BlocksConvert, contains: &str) {
    let before = doc.to_snapshot_v2();
    let history = doc.undo_labels();
    let answer = blocks_convert::execute(&mut ExecutionContext::new(doc), input);
    let CommandResult::Failed { error } = answer else {
        panic!("must refuse {answer:?}")
    };
    assert!(error.message.contains(contains), "{error:?}");
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.undo_labels(), history);
}

#[test]
fn chosen_collections_keep_root_uuid_piece_map_reversed_binding_and_one_history_step() {
    let mut doc = open(fixture());
    let original_ids: Vec<_> = (1..=4).map(|i| uid(&doc, i)).collect();
    let dim = add_dimension(
        &mut doc,
        3,
        json!({"kind":"start"}),
        json!({"x":12.,"y":0.}),
    );
    let before = doc.to_snapshot_v2();
    let history = doc.undo_labels();
    let request = input(&doc);
    let CommandResult::Completed { output: plan, .. } =
        blocks_convert::inspect(&doc, &request, || false)
    else {
        panic!("plan")
    };
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.undo_labels(), history);
    assert_eq!(plan.placements.len(), 2);
    assert_eq!(plan.mapping.len(), 4);
    assert_eq!(
        plan.removed,
        [original_ids[1].clone(), original_ids[3].clone()]
    );
    assert_eq!(plan.block.units, Some(kentos_commands::BlockUnits::M));
    assert_eq!(
        plan.block
            .entities
            .iter()
            .map(|e| e.base().id)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(plan.dimensions.len(), 1);
    assert_eq!(plan.dimensions[0].after.path, [0]);
    assert_eq!(plan.dimensions[0].after.place, SubPlace::End);
    assert_eq!(plan.dimensions[0].deviation, 0.);
    let CommandResult::Completed { output, .. } =
        blocks_convert::execute(&mut ExecutionContext::new(&mut doc), request)
    else {
        panic!("execute")
    };
    assert_eq!(doc.len(), 3);
    assert_eq!(doc.blocks().len(), 1);
    assert_eq!(
        output.inserts,
        [original_ids[0].clone(), original_ids[2].clone()]
    );
    for id in [1, 3] {
        assert_eq!(uid(&doc, id), original_ids[id as usize - 1]);
        assert!(matches!(doc.get(Slot(id)), Some(Entity::Insert(_))));
    }
    for id in [2, 4] {
        assert!(doc.get(Slot(id)).is_none());
    }
    let r = dimension(&doc, dim)
        .refs
        .as_ref()
        .unwrap()
        .a
        .as_ref()
        .unwrap();
    assert_eq!(r.uid, EntityId(doc.uid(Slot(3)).unwrap().into_bytes()));
    assert_eq!(r.path, [0]);
    assert_eq!(r.place, SubPlace::End);
    assert!(r.live());
    assert_eq!(dimension(&doc, dim).a.x, 12.);
    let after = doc.to_snapshot_v2();
    assert_eq!(doc.undo_labels().len(), history.len() + 1);
    assert_eq!(doc.undo().as_deref(), Some(blocks_convert::LABEL));
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.redo().as_deref(), Some(blocks_convert::LABEL));
    assert_eq!(doc.to_snapshot_v2(), after);
    let mut moved = doc.get(Slot(3)).unwrap().clone();
    if let Entity::Insert(i) = &mut moved {
        i.p.x += 5.;
        i.p.y += 3.;
    }
    doc.update(Slot(3), moved);
    assert_eq!(
        dimension(&doc, dim).a,
        kentos_commands::Vec2 { x: 17., y: 3. }
    );
    assert!(
        dimension(&doc, dim)
            .refs
            .as_ref()
            .unwrap()
            .a
            .as_ref()
            .unwrap()
            .live()
    );
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), after);
}

#[test]
fn closed_curved_tapered_path_rebinds_the_reversed_closing_edge_fraction() {
    let mut raw = fixture();
    raw["entities"] = json!([
        {"kind":"polyline","id":1,"layerId":"a","attrs":{},"closed":true,
         "pts":[{"x":0.,"y":0.},{"x":4.,"y":0.},{"x":0.,"y":4.}],"bulges":[0.,0.,0.25],"widths":[[0.1,0.2],[0.3,0.4],[0.5,0.6]]},
        {"kind":"polyline","id":2,"layerId":"a","attrs":{},"closed":true,
         "pts":[{"x":14.,"y":0.},{"x":10.,"y":0.},{"x":10.,"y":4.}],"bulges":[0.,-0.25,0.],"widths":[[0.2,0.1],[0.6,0.5],[0.4,0.3]]}
    ]);
    let mut doc = open(raw);
    let dim = add_dimension(
        &mut doc,
        2,
        json!({"kind":"mid","part":0,"ring":0,"index":1}),
        json!({"x":9.5,"y":2.}),
    );
    let request = BlocksConvert {
        name: "Yaylı".into(),
        source: vec![uid(&doc, 1)],
        groups: vec![vec![uid(&doc, 1)], vec![uid(&doc, 2)]],
        rotations: Some(false),
        ..Default::default()
    };
    let before = doc.to_snapshot_v2();
    let answer = blocks_convert::execute(&mut ExecutionContext::new(&mut doc), request);
    let CommandResult::Completed { output, .. } = answer else {
        panic!("{answer:?}")
    };
    assert_eq!(
        output.dimensions[0].after.place,
        SubPlace::Mid {
            part: 0,
            ring: 0,
            index: 2
        }
    );
    assert_eq!(output.dimensions[0].after.path, [0]);
    assert_eq!(
        dimension(&doc, dim).a,
        kentos_commands::Vec2 { x: 9.5, y: 2. }
    );
    assert!(
        dimension(&doc, dim)
            .refs
            .as_ref()
            .unwrap()
            .a
            .as_ref()
            .unwrap()
            .live()
    );
    let Some(Entity::Polyline(p)) = doc.blocks()[0].entities.first() else {
        panic!()
    };
    assert_eq!(p.widths.as_ref().unwrap()[2], [0.5, 0.6]);
    assert!(p.closed);
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn overlap_changed_geometry_data_and_interleaved_paint_order_refuse_atomically() {
    let mut doc = open(fixture());
    let mut request = input(&doc);
    request.groups[1] = request.groups[0].clone();
    refused(&mut doc, request, "paylaşıyor");
    let mut raw = fixture();
    raw["entities"][3]["r"] = json!(0.3);
    let mut doc = open(raw);
    let r = input(&doc);
    refused(&mut doc, r, "doğrulanamadı");
    let mut raw = fixture();
    for id in [0, 2] {
        raw["entities"][id]["attrs"] = json!({"Door":"A"});
    }
    let mut doc = open(raw);
    let r = input(&doc);
    refused(&mut doc, r, "öznitelik");
    let mut raw = fixture();
    let xs = raw["entities"].as_array_mut().unwrap();
    let a = xs.remove(1);
    xs.insert(2, a);
    let mut doc = open(raw);
    let r = input(&doc);
    refused(&mut doc, r, "çizim sırası");
}

#[test]
fn root_links_expressions_and_layer_schema_are_not_a_narrow_copy_links_count() {
    let mut doc = open(fixture());
    let chosen = uid(&doc, 1);
    // Static text with an embedded UUID stands for an opaque expression/reference: refuse conservatively.
    let e:Entity=serde_json::from_value(json!({"kind":"text","id":0,"layerId":"a","attrs":{},"p":{"x":0.,"y":5.},"text":format!("object({})",chosen.to_uppercase()),"height":0.1,"rotation":0.})).unwrap();
    doc.add(e).unwrap();
    let r = input(&doc);
    refused(&mut doc, r, "bağlı");
    let mut raw = fixture();
    raw["layers"][0]["geometry"] = json!("line");
    let mut doc = open(raw);
    let r = input(&doc);
    refused(&mut doc, r, "geometry");
    let mut raw = fixture();
    raw["settings"]["workspace"] = json!("gis");
    let mut doc = open(raw);
    let r = input(&doc);
    refused(&mut doc, r, "CBS");
}

#[test]
fn cancellation_stale_revision_and_ambiguous_topology_never_write() {
    let mut doc = open(fixture());
    let r = input(&doc);
    let before = doc.to_snapshot_v2();
    let mut calls = 0;
    assert!(matches!(
        blocks_convert::inspect(&doc, &r, || {
            calls += 1;
            calls > 15
        }),
        CommandResult::Cancelled
    ));
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut r = input(&doc);
    r.expected_revision = Some("99".into());
    assert!(matches!(
        blocks_convert::execute(&mut ExecutionContext::new(&mut doc), r),
        CommandResult::Conflict { .. }
    ));
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut raw = fixture();
    raw["entities"] = json!([{"kind":"polyline","id":1,"layerId":"a","attrs":{},"closed":true,
        "pts":[{"x":0.,"y":0.},{"x":2.,"y":0.},{"x":0.,"y":3.},{"x":0.,"y":0.},{"x":2.,"y":0.},{"x":0.,"y":3.}]}]);
    let mut doc = open(raw);
    add_dimension(
        &mut doc,
        1,
        json!({"kind":"vertex","part":0,"ring":0,"index":0}),
        json!({"x":0.,"y":0.}),
    );
    let r = BlocksConvert {
        name: "Belirsiz".into(),
        source: vec![uid(&doc, 1)],
        groups: vec![vec![uid(&doc, 1)]],
        ..Default::default()
    };
    refused(&mut doc, r, "birden fazla");
}

#[test]
fn outdated_dimension_coordinates_are_never_silently_repositioned_by_conversion() {
    let mut doc = open(fixture());
    add_dimension(
        &mut doc,
        3,
        json!({"kind":"start"}),
        json!({"x":11.0,"y":0.0}),
    );
    // The live place is at x=12; an un-settled input file can hold stale displayed coordinates.
    let r = input(&doc);
    refused(&mut doc, r, "kaynak bağı güncel değil");
}

#[test]
fn aggregate_vertex_limit_is_inherited_from_the_same_recognition_gate_without_a_partial_write() {
    let mut raw = fixture();
    let points: Vec<_> = (0..4000).map(|i| json!({"x":i as f64,"y":0.})).collect();
    raw["entities"] = Value::Array(
        (1..=51)
            .map(|id| json!({"kind":"polyline","id":id,"layerId":"a","attrs":{},"pts":points}))
            .collect(),
    );
    let mut doc = open(raw);
    let r = BlocksConvert {
        name: "Büyük kapsam".into(),
        source: vec![uid(&doc, 1)],
        groups: (1..=51).map(|i| vec![uid(&doc, i)]).collect(),
        ..Default::default()
    };
    let before = doc.to_snapshot_v2();
    let result = blocks_convert::execute(&mut ExecutionContext::new(&mut doc), r);
    let CommandResult::Failed { error } = result else {
        panic!("vertex limit")
    };
    assert_eq!(error.code, "recognition_scope_limit");
    assert_eq!(doc.to_snapshot_v2(), before);
    assert!(!doc.can_undo());
}
