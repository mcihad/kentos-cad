//! Independent physical expectations; same ordinary closed walls and one command history.
use kentos_commands::*;
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geom::{bulge::bulge_ring_area, wall};
use kentos_native_application::{ExecutionContext, walls_from_lines};
use serde_json::{Value, json};
fn drawing(entities: Value, unit: &str) -> Document {
    let mut setup: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.walls.convert.json"
    ))
    .unwrap();
    setup["setup"]["entities"] = entities;
    setup["setup"]["settings"]["drawingUnit"] = json!(unit);
    Document::from_snapshot(DocumentSnapshotV1::from_json(&setup["setup"].to_string()).unwrap())
        .unwrap()
}
fn source(pts: Vec<[f64; 2]>, holes: Value) -> Value {
    let mut e = json!({"kind":"polygon","id":1,"layerId":"cizim","pts":pts.into_iter().map(|p|json!({"x":p[0],"y":p[1]})).collect::<Vec<_>>(),"attrs":{"oda":"A"},"color":"#123456"});
    if !holes.is_null() {
        e["holes"] = holes;
    }
    e
}
fn input(doc: &Document, justify: &str, keep: bool) -> WallsFromLines {
    serde_json::from_value(json!({"uids":[doc.uid(Slot(1)).unwrap().to_string()],"mode":"contours","justify":justify,"width":0.2,"keepSources":keep})).unwrap()
}
fn completed<T: std::fmt::Debug>(r: CommandResult<T>) -> T {
    match r {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}
fn room_area(w: &WallEntity) -> f64 {
    let pts = w
        .wall
        .pts
        .iter()
        .map(|p| kentos_geometry_core::Vec2::new(p.x, p.y))
        .collect::<Vec<_>>();
    let spec = kentos_formats::walls::spec(&w.wall);
    let line = wall::line_at(&pts, w.wall.bulges.as_deref(), true, wall::left_face(&spec)).unwrap();
    bulge_ring_area(&line.pts, Some(&line.bulges))
}
#[test]
fn l_room_dimensions_ignore_winding_units_and_distant_origin_without_writing_preview() {
    let l = [[0., 0.], [6., 0.], [6., 2.], [2., 2.], [2., 5.], [0., 5.]];
    for unit in ["m", "mm", "cm", "in", "ft"] {
        for (x, y) in [(0., 0.), (500000., 4400000.)] {
            for reverse in [false, true] {
                let mut pts = l
                    .into_iter()
                    .map(|p| [p[0] + x, p[1] + y])
                    .collect::<Vec<_>>();
                if reverse {
                    pts[1..].reverse();
                }
                let doc = drawing(json!([source(pts, Value::Null)]), unit);
                let before = doc.to_snapshot_v2();
                for (reference, area) in [("left", 18.), ("right", 13.76), ("center", 15.84)] {
                    let plan = completed(walls_from_lines::plan_in(
                        &doc,
                        &input(&doc, reference, true),
                    ));
                    let Entity::Wall(w) = &plan.walls[0] else {
                        panic!()
                    };
                    assert!(
                        (room_area(w) - area).abs() < 1e-8,
                        "{unit} {x} {reverse} {reference}: {}",
                        room_area(w)
                    );
                    assert_eq!(w.wall.pts[0], Vec2 { x, y });
                    assert_eq!(w.base.attrs.get("oda").map(String::as_str), Some("A"));
                    assert_eq!(w.base.color.as_deref(), Some("#123456"));
                    assert_eq!(doc.to_snapshot_v2(), before);
                }
            }
        }
    }
}
#[test]
fn courtyard_holes_are_separate_hostable_walls_and_one_atomic_undo() {
    let outer = vec![[0., 0.], [10., 0.], [10., 8.], [0., 8.]];
    let holes = json!([{"pts":[{"x":4,"y":3},{"x":6,"y":3},{"x":6,"y":5},{"x":4,"y":5}]}]);
    for (reference, area) in [("left", 76.), ("right", 67.2), ("center", 71.6)] {
        let mut doc = drawing(json!([source(outer.clone(), holes.clone())]), "m");
        let before = doc.to_snapshot_v2();
        let request = input(&doc, reference, false);
        let plan = completed(walls_from_lines::plan_in(&doc, &request));
        assert_eq!(plan.walls.len(), 2);
        let area_found = plan
            .walls
            .iter()
            .map(|e| {
                let Entity::Wall(w) = e else { panic!() };
                room_area(w)
            })
            .sum::<f64>();
        assert!(
            (area_found - area).abs() < 1e-8,
            "{reference}: {area_found}"
        );
        let done = completed(walls_from_lines::execute(
            &mut ExecutionContext::new(&mut doc),
            request,
        ));
        assert_eq!(done.uids.len(), 2);
        assert_eq!(done.removed.len(), 1);
        assert_eq!(doc.len(), 2);
        assert_eq!(
            doc.journal().last().unwrap().command.as_deref(),
            Some("cad.walls.convert")
        );
        let after = doc.to_snapshot_v2();
        assert!(doc.undo().is_some());
        assert_eq!(doc.to_snapshot_v2().entities, before.entities);
        assert!(doc.redo().is_some());
        assert_eq!(doc.to_snapshot_v2().entities, after.entities);
    }
}
#[test]
fn circles_keep_exact_arcs_and_normalize_reversed_bulges() {
    for reverse in [false, true] {
        let entity = if reverse {
            json!({"kind":"polygon","id":1,"layerId":"cizim","pts":[{"x":3,"y":0},{"x":-3,"y":0}],"bulges":[-1,-1],"attrs":{}})
        } else {
            json!({"kind":"circle","id":1,"layerId":"cizim","c":{"x":0,"y":0},"r":3,"attrs":{}})
        };
        let doc = drawing(json!([entity]), "m");
        for (reference, radius) in [("left", 3_f64), ("right", 2.8), ("center", 2.9)] {
            let plan = completed(walls_from_lines::plan_in(
                &doc,
                &input(&doc, reference, true),
            ));
            let Entity::Wall(w) = &plan.walls[0] else {
                panic!()
            };
            assert_eq!(w.wall.pts.len(), 2);
            assert_eq!(w.wall.bulges, Some(vec![1., 1.]));
            assert!((room_area(w) - std::f64::consts::PI * radius * radius).abs() < 1e-10);
        }
    }
}
#[test]
fn repeated_conversion_mixed_open_source_shared_edge_and_collapsing_room_refuse_without_partial_writes()
 {
    let a = source(vec![[0., 0.], [4., 0.], [4., 3.], [0., 3.]], Value::Null);
    let mut doc = drawing(json!([a.clone()]), "m");
    let request = input(&doc, "left", true);
    completed(walls_from_lines::execute(
        &mut ExecutionContext::new(&mut doc),
        request.clone(),
    ));
    let before = doc.to_snapshot_v2();
    assert!(matches!(
        walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request),
        CommandResult::Failed { .. }
    ));
    assert_eq!(doc.to_snapshot_v2(), before);
    for second in [
        json!({"kind":"line","id":2,"layerId":"cizim","a":{"x":8,"y":0},"b":{"x":10,"y":0},"attrs":{}}),
        json!({"kind":"polygon","id":2,"layerId":"cizim","pts":[{"x":4,"y":0},{"x":8,"y":0},{"x":8,"y":3},{"x":4,"y":3}],"attrs":{}}),
    ] {
        let mut doc = drawing(json!([a.clone(), second]), "m");
        let mut request = input(&doc, "left", false);
        request.uids.push(doc.uid(Slot(2)).unwrap().to_string());
        let before = doc.to_snapshot_v2();
        assert!(matches!(
            walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request),
            CommandResult::Failed { .. }
        ));
        assert_eq!(doc.to_snapshot_v2(), before);
    }
    for width in [1.5, 2., 3., 8.] {
        let mut doc = drawing(json!([a.clone()]), "m");
        let mut request = input(&doc, "right", false);
        request.width = Some(width);
        let before = doc.to_snapshot_v2();
        assert!(
            matches!(
                walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request),
                CommandResult::Failed { .. }
            ),
            "width {width}"
        );
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

#[test]
fn layer_lock_hidden_type_and_readonly_policy_keep_atomicity_and_elevated_source_is_not_flattened()
{
    let a = source(vec![[0., 0.], [4., 0.], [4., 3.], [0., 3.]], Value::Null);
    let mut second = source(
        vec![[10., 0.], [14., 0.], [14., 3.], [10., 3.]],
        Value::Null,
    );
    second["id"] = json!(2);
    second["layerId"] = json!("kilitli");
    let mut doc = drawing(json!([a.clone(), second]), "m");
    let mut request = input(&doc, "left", false);
    request.uids.push(doc.uid(Slot(2)).unwrap().to_string());
    let before = doc.to_snapshot_v2();
    let CommandResult::Failed { error } =
        walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request)
    else {
        panic!()
    };
    assert_eq!(error.code, "layer_locked");
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut hidden = a.clone();
    hidden["layerId"] = json!("gizli");
    let mut doc = drawing(json!([hidden]), "m");
    let request = input(&doc, "left", true);
    let made = completed(walls_from_lines::execute(
        &mut ExecutionContext::new(&mut doc),
        request,
    ));
    assert_eq!(
        doc.get(doc.slot_of(made.uids[0].parse().unwrap()).unwrap())
            .unwrap()
            .base()
            .layer_id,
        "gizli"
    );
    let mut doc = drawing(json!([a.clone()]), "m");
    doc.set_object_policy(kentos_domain::ObjectPolicy::AttributesOnly);
    let request = input(&doc, "left", false);
    let before = doc.to_snapshot_v2();
    assert!(!matches!(
        walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request),
        CommandResult::Completed { .. }
    ));
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut doc = drawing(json!([a.clone()]), "m");
    doc.set_layer_geometry("duvar", Some(LayerGeometry::Point), "Test target type")
        .unwrap(); // explicit incompatible target must fail in plan too
    let mut request = input(&doc, "left", false);
    request.layer_id = Some("duvar".into());
    let before = doc.to_snapshot_v2();
    assert!(matches!(
        walls_from_lines::plan_in(&doc, &request),
        CommandResult::Failed { .. }
    ));
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut elevated = a;
    elevated["zs"] = json!([3, 3, 3, 3]);
    let mut doc = drawing(json!([elevated]), "m");
    let request = input(&doc, "left", false);
    let before = doc.to_snapshot_v2();
    assert!(matches!(
        walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request),
        CommandResult::Failed { .. }
    ));
    assert_eq!(doc.to_snapshot_v2(), before);
}
