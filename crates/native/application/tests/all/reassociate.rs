//! Yeniden bağla over a document whose objects change (docs/adr/0249 §3): what the
//! shared cases cannot set up, since their drawing holds no persistent ids. A
//! bound dimension follows its wall; a deleted vertex breaks its point; the
//! command gives it a place again and it follows once more.

use kentos_commands::{
    CommandResult, DimensionPoint, DimensionsReassociate, DocumentSnapshotV1, Entity, PlaceOn,
    PointBinding, SubPlace, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, dimensions_reassociate};
use serde_json::json;

fn drawing() -> (Document, Slot, Slot) {
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Ölçü","settings":{"srid":5256,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[
        {"kind":"polyline","id":1,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":5}]},
        {"kind":"dimension","id":2,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":7,"y":0},"offset":1,"height":0.25}],
        "styles":{"items":[],"categories":[]}});
    let doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"),
    )
    .expect("opens");
    (doc, Slot(1), Slot(2))
}

fn vertex(index: u32) -> SubPlace {
    SubPlace::Vertex {
        part: 0,
        ring: 0,
        index,
    }
}

fn bind(doc: &mut Document, wall: Slot, dim: Slot, bindings: &[(DimensionPoint, u32)]) -> f64 {
    let uid = |s: Slot| doc.uid(s).expect("known").to_string();
    let input = DimensionsReassociate {
        uid: uid(dim),
        points: bindings
            .iter()
            .map(|&(point, index)| PointBinding {
                point,
                to: Some(PlaceOn {
                    uid: uid(wall),
                    place: vertex(index),
                    path: Vec::new(),
                }),
            })
            .collect(),
        expected_revision: None,
    };
    match dimensions_reassociate::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { output, .. } => output.value,
        other => panic!("not completed: {other:?}"),
    }
}

fn dimension(doc: &Document, slot: Slot) -> kentos_commands::DimensionEntity {
    match doc.get(slot) {
        Some(Entity::Dimension(d)) => d.clone(),
        other => panic!("not a dimension: {other:?}"),
    }
}

fn with_points(doc: &Document, wall: Slot, pts: Vec<Vec2>) -> Entity {
    let mut e = doc.get(wall).expect("known").clone();
    if let Entity::Polyline(p) = &mut e {
        p.pts = pts;
    }
    e
}

#[test]
fn a_bound_dimension_follows_breaks_and_is_bound_again() {
    let (mut doc, wall, dim) = drawing();
    assert_eq!(
        bind(
            &mut doc,
            wall,
            dim,
            &[(DimensionPoint::A, 0), (DimensionPoint::B, 1)]
        ),
        10.0
    );
    // The wall's corner is dragged: the dimension goes with it.
    let dragged = vec![
        Vec2 { x: 0.0, y: 0.0 },
        Vec2 { x: 14.0, y: 0.0 },
        Vec2 { x: 14.0, y: 5.0 },
    ];
    assert!(doc.update(wall, with_points(&doc, wall, dragged)));
    assert_eq!(dimension(&doc, dim).b, Vec2 { x: 14.0, y: 0.0 });
    // The corner is deleted: b breaks and stays where it was.
    let fewer = vec![Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 14.0, y: 5.0 }];
    assert!(doc.update(wall, with_points(&doc, wall, fewer)));
    let d = dimension(&doc, dim);
    assert_eq!(d.b, Vec2 { x: 14.0, y: 0.0 });
    let refs = d.refs.expect("refs");
    assert_eq!(
        refs.b.as_ref().and_then(|r| r.broken.as_deref()),
        Some("vertex_gone")
    );
    // Bound again to the wall's new last corner: live, and follows once more.
    let value = bind(&mut doc, wall, dim, &[(DimensionPoint::B, 1)]);
    assert!((value - 221f64.sqrt()).abs() < 1e-12, "{value}");
    let d = dimension(&doc, dim);
    assert_eq!(d.b, Vec2 { x: 14.0, y: 5.0 });
    assert!(d.refs.as_ref().unwrap().b.as_ref().unwrap().live());
    let moved = vec![Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 14.0, y: 9.0 }];
    assert!(doc.update(wall, with_points(&doc, wall, moved)));
    assert_eq!(dimension(&doc, dim).b, Vec2 { x: 14.0, y: 9.0 });
    // One undo takes the move back, the next the binding: b broken again.
    doc.undo();
    doc.undo();
    let refs = dimension(&doc, dim).refs.unwrap();
    assert_eq!(refs.b.unwrap().broken.as_deref(), Some("vertex_gone"));
}

#[test]
fn a_copy_of_a_bound_dimension_is_free() {
    let (mut doc, wall, dim) = drawing();
    bind(&mut doc, wall, dim, &[(DimensionPoint::A, 0)]);
    let copy = kentos_native_application::geometry::unlinked(doc.get(dim).unwrap().clone());
    let Entity::Dimension(d) = copy else { panic!() };
    assert!(d.refs.is_none());
}

#[test]
fn public_commands_rebind_a_replacement_insert_inside_one_outer_step() {
    use kentos_native_application::{blocks_define, edit};
    let (mut doc, wall, dim) = drawing();
    bind(
        &mut doc,
        wall,
        dim,
        &[(DimensionPoint::A, 0), (DimensionPoint::B, 1)],
    );
    let before = doc.to_snapshot_v2();
    let steps = doc.undo_labels().len();
    let uid = doc.uid(wall).unwrap().to_string();
    doc.transact("Blok ve ölçü bağı", |doc| -> Result<(), ()> {
        let input = serde_json::from_value(json!({"name":"Bağlı parça","base":{"x":0,"y":0},
            "uids":[uid],"units":"m"})).unwrap();
        let CommandResult::Completed { output: block, .. } =
            blocks_define::execute(&mut ExecutionContext::new(doc), input) else { panic!() };
        let input = serde_json::from_value(json!({"operation":"properties","changes":[
            {"kind":"replace","uid":uid,"keepData":true,"geometry":{"kind":"insert",
             "block":block.block,"p":{"x":0,"y":0},"scale":1,"rotation":0}}
        ]})).unwrap();
        assert!(matches!(edit::execute(&mut ExecutionContext::new(doc), input), CommandResult::Completed { .. }));
        let input = serde_json::from_value(json!({"uid":doc.uid(dim).unwrap().to_string(),"points":[
            {"point":"a","to":{"uid":uid,"path":[0],"place":{"kind":"vertex","part":0,"ring":0,"index":0}}},
            {"point":"b","to":{"uid":uid,"path":[0],"place":{"kind":"vertex","part":0,"ring":0,"index":1}}}
        ]})).unwrap();
        let CommandResult::Completed { output, .. } =
            dimensions_reassociate::execute(&mut ExecutionContext::new(doc), input) else { panic!() };
        assert_eq!(output.value, 10.0);
        Ok(())
    }).unwrap();
    assert!(
        dimension(&doc, dim)
            .refs
            .unwrap()
            .each()
            .all(|(_, r)| r.live())
    );
    assert_eq!(dimension(&doc, dim).b, Vec2 { x: 10.0, y: 0.0 });
    let after = doc.to_snapshot_v2();
    assert_eq!(doc.undo_labels().len(), steps + 1);
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), before);
    doc.redo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), after);
}

#[test]
fn public_plan_and_execute_accept_a_closing_arc_inside_a_block() {
    use kentos_native_application::{blocks_define, blocks_insert};
    let (mut doc, wall, dim) = drawing();
    let mut path = doc.get(wall).unwrap().clone();
    if let Entity::Polyline(p) = &mut path {
        p.pts = vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 4.0, y: 0.0 },
            Vec2 { x: 0.0, y: 4.0 },
        ];
        p.bulges = Some(vec![0.0, 0.0, 0.25]);
        p.closed = true;
    }
    doc.update(wall, path);
    let input = serde_json::from_value(json!({"name":"Kapalı yay","base":{"x":0,"y":0},
        "uids":[doc.uid(wall).unwrap().to_string()],"units":"m"}))
    .unwrap();
    let CommandResult::Completed { output: block, .. } =
        blocks_define::execute(&mut ExecutionContext::new(&mut doc), input)
    else {
        panic!()
    };
    let input =
        serde_json::from_value(json!({"block":block.block,"at":{"x":0,"y":0},"layerId":"a"}))
            .unwrap();
    let CommandResult::Completed { output: insert, .. } =
        blocks_insert::execute(&mut ExecutionContext::new(&mut doc), input)
    else {
        panic!()
    };
    let input = serde_json::from_value(json!({"uid":doc.uid(dim).unwrap().to_string(),"points":[
        {"point":"a","to":{"uid":insert.uid,"path":[0],"place":{"kind":"vertex","part":0,"ring":0,"index":0}}},
        {"point":"b","to":{"uid":insert.uid,"path":[0],"place":{"kind":"mid","part":0,"ring":0,"index":2}}}
    ]})).unwrap();
    let before = doc.to_snapshot_v2();
    let CommandResult::Completed { output: plan, .. } =
        dimensions_reassociate::plan(&ExecutionContext::new(&mut doc), &input)
    else {
        panic!()
    };
    assert!((plan.value - 4.25_f64.sqrt()).abs() < 1e-12);
    assert_eq!(doc.to_snapshot_v2(), before);
    let CommandResult::Completed { output, .. } =
        dimensions_reassociate::execute(&mut ExecutionContext::new(&mut doc), input)
    else {
        panic!()
    };
    assert_eq!(output.value, plan.value);
    assert!(
        dimension(&doc, dim)
            .refs
            .unwrap()
            .each()
            .all(|(_, r)| r.live())
    );
    let after = doc.to_snapshot_v2();
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), before);
    doc.redo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), after);
}

/// Öznitelikler writes a dimension's Zemin through `cad.entities.edit`'s
/// update, its geometry from the core's shape (no references): the
/// references stay; a grip that moves a bound point lets that point go.
#[test]
fn an_edit_without_references_keeps_them_and_a_moved_point_goes_free() {
    use kentos_commands::{EditOperation, EntitiesEdit, EntityEdit};
    use kentos_native_application::edit;
    use kentos_native_application::geometry::{edit_geometry, shape};
    let (mut doc, wall, dim) = drawing();
    bind(
        &mut doc,
        wall,
        dim,
        &[(DimensionPoint::A, 0), (DimensionPoint::B, 1)],
    );
    let mut masked = dimension(&doc, dim);
    masked.mask = true;
    let write = |doc: &mut Document, e: Entity| {
        let input = EntitiesEdit {
            operation: EditOperation::Properties,
            changes: vec![EntityEdit::Update {
                uid: doc.uid(dim).unwrap().to_string(),
                geometry: edit_geometry(shape(&e)).unwrap(),
            }],
            expected_revision: None,
        };
        assert!(matches!(
            edit::execute(&mut ExecutionContext::new(doc), input),
            CommandResult::Completed { .. }
        ));
    };
    write(&mut doc, Entity::Dimension(masked));
    let d = dimension(&doc, dim);
    assert!(d.mask);
    assert_eq!(d.refs.as_ref().unwrap().each().count(), 2);
    let mut dragged = d.clone();
    dragged.b = Vec2 { x: 8.0, y: 0.0 };
    write(&mut doc, Entity::Dimension(dragged));
    let refs = dimension(&doc, dim).refs.unwrap();
    assert!(refs.a.is_some() && refs.b.is_none());
}
